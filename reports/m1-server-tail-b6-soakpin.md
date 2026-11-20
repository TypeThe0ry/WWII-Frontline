# B6-soakpin：Carentan WT 服务端尾延迟复测（2026-11-20）

## 工况

PI-109：共享 EPYC 7402 工作站，**非参考机**。同机私有 `unshare -n` loopback，无 netem。Release server/headless，`carentan/m1`、Assault、20 WT headless + 12 固定 bot、HQ 部署、seed 1、1800 s、默认预热、10 s scrape、沿用 B5-E1 的 match ID、两类 tick trace 和日志限速。未连接网关，未使用 GPU。地图按 [B4-D1](m1-carentan-wt-b4-d1.md) 在暂存副本中烘焙，解包 380 块 FCOL 并用 L3a/L3b 作 headless 巡逻点，未改正式地图。原始 `.bin` 的路径和内容摘要（按相对 `carentan` 路径排序的 `sha256sum` 清单再取 SHA-256）：`9ff574ccc87ffbc1cc83023140a8cc19133d666ad09b72777b7af7ae7c12faeb`。

`FRONTLINE_SIM_CPU`（亦可用 `--pin-sim-core`；保留旧 `FRONTLINE_PIN_SIM_CORE`）给 Linux `sim` 线程设置单个逻辑 CPU。其他线程继承启动掩码。内核拒绝时服务器记录 `sim_cpu_pin_failed` 并保留继承掩码；隔离压测则核对 `/proc`，不能验证就退出。这里用 29 专供本次 sim，避开 SMT 同胞 77；server 主线程、`log-output` 和两个 `net` worker 均用 36、38、39，headless 用 41、44，脚本/sampler 用 46。核不是宿主独占；不改 governor/IRQ、nice 或 `SCHED_FIFO`，不移动其他进程。`LOADTEST_LOG_DIR` 要求可写 tmpfs；两份高频原始日志在 tmpfs 下，`TMPDIR` 下的探针、报告和链接仍可按原路径读取和审计。

## 复现

`D` 是按 B4-D1 暂存地图后的工作目录，`L` 是可写的 tmpfs 目录（例如用 `mktemp -d /dev/shm/soakpin.XXXXXX` 创建）。须先按当前主机拓扑和负载重选 CPU；在本次工作站上的命令如下。每次独立启动服务器、客户端与 netns；串行执行以免互相争用。

```sh
CARGO_BUILD_JOBS=8 cargo build --locked --release -p frontline-server -p frontline-headless-client
lscpu -e=CPU,CORE,SOCKET,NODE
mpstat -P 29,77,36,38,39,41,44,46 1 5
ps -C sunshine -C Xorg -L -o pid,tid,psr,pcpu,comm
cpupower -c 29,77 frequency-info -p
export FRONTLINE_SIM_CPU=29 LOADTEST_SERVER_CPUS=36,38,39 LOADTEST_CLIENT_CPUS=41,44
export FRONTLINE_DATA_DIR="$D/data" LOADTEST_LOG_DIR="$L"
export FRONTLINE_MATCH_ID=0a9003ec-f0b5-4ac1-a09a-81d05072d401
export FRONTLINE_TICK_TRACE=1 FRONTLINE_STEP_TRACE=1 FRONTLINE_LOG_RATE_LIMIT=10000
unset GATEWAY_URL FRONTLINE_DEV_NO_GATEWAY FRONTLINE_STEP_TRACE_TICKS LOADTEST_NETEM
unset FRONTLINE_PIN_SIM_CORE TOKIO_WORKER_THREADS
mkdir -p "$D/run1" "$D/run2"
for run in run1 run2; do
  TMPDIR="$D/$run" taskset -c 46 unshare -n bash -c '
    ip link set lo up && exec scripts/loadtest-m0.sh --transport wt --map carentan/m1 \
      --mode assault --deploy-hq --count 20 --bots 12 --duration 1800 --seed 1 \
      --sample-interval 10 --no-build --out "$TMPDIR/summary.json"'
  printf "%s exit=%s\n" "$run" "$?"
done
```

`affinity.txt` 同时记录实际和预期掩码；`stat -f -L -c %T` 核对日志链接目标为 `tmpfs`。脚本保留每轮两个原始目录，提取报告后需分别清理。

两轮使用同一套 release 二进制（源码基线 `27d5a1d` 加 `203b4d9`）：

| 文件 | SHA-256 |
| --- | --- |
| `frontline-server` | `d1a7ecafa7b3d84c10aae08c0f6d65f6ab265be8f1abdde3a34a4ae07bc068b1` |
| `headless-client` | `30890efb154523888c0c948193319d7c2b09df57dcfcefebe95028ca1231219c` |
| `scripts/loadtest-m0.sh`（运行版） | `53106ef7fce9ac35d9867c99cdc08622bd731c3892fd061e7721cc64c44d4566` |

两轮开跑前各用 `mpstat` 采样 5 次，CPU 29/77 平均空闲分别为 98.2%/92.6% 和 99.8%/100%。CPU 29、77 未出现在两轮启动前的 Sunshine/Xorg 核位快照。`cpupower` 确认两核均用 `schedutil`（策略范围 1.50–2.80 GHz）。第二轮开跑前非 sim 核 38 持续忙（5 次采样空闲均为 0%）；该核属于三 CPU 网络/日志池，没有启用宿主独占隔离。

## 自检

- `CARGO_BUILD_JOBS=8 taskset -c 0-23,48-71 scripts/local-ci.sh lint rust determinism` 退出 0；无 fail/skip 分组。配置测试覆盖非负单个 CPU、负值、列表、范围及溢出；Linux 线程测试验证设置后仅调用线程的掩码改变。native 与 WASM 在 3 个种子共 54,000 tick 状态哈希逐 tick 相同、位置误差 0 m。未更新历史金样。
- 重基到 `0912177` 后 `CARGO_BUILD_JOBS=8 scripts/local-ci.sh lint determinism` 与 `CARGO_BUILD_JOBS=8 cargo test --locked -p frontline-server --lib` 均退出 0；确定性仍逐 tick 一致，服务端 143 通过、0 失败、4 原本忽略。长跑数字仍对应上文的实测二进制。
- `shellcheck scripts/loadtest-m0.sh`、`bash -n scripts/loadtest-m0.sh` 通过。另用 release CLI 实测拒绝 `FRONTLINE_SIM_CPU=4,5`，压测脚本拒绝 sim/网络掩码冲突和非 tmpfs 的日志目录（均退出 2）。
- 首次 20 s 冒烟因运行时脚本编辑导致 Bash 语法错误、无汇总；不计作长期样本。代码定稿后的 20 s 同配置复跑退出 0，20/20 加入，0 次超时、0 断线、0 解码错误。第一轮长测启动后核验两份日志的文件系统均为 tmpfs。

## 两轮 1,800 s 结果

| 指标 | B5-E1 | B6 第 1 轮 | B6 第 2 轮 |
| --- | ---: | ---: | ---: |
| 压测退出码 | 1 | **0** | **0** |
| tick P50 / P99 / max（ms） | 4.189 / 8.644 / 126.668 | **4.568 / 8.767 / 24.434** | **4.162 / 8.059 / 81.440** |
| 超时数 / 跳过定时槽 | 29 / 52 | 0 / 1 | 9 / 11 |
| 最长连续超时（门槛 ≤1） | 2 | **0** | **1** |
| 超时归因：runq / CPU / 其它 | 29 / 0 / 0 | 0 / 0 / 0 | 8 / 1 / 0 |
| 下行 P50 / P99（kbps） | 21.992 / 26.888 | 22.264 / 26.856 | 21.616 / 26.872 |
| 上行 P50 / P99（kbps） | 20.576 / 21.424 | 20.576 / 21.424 | 20.560 / 21.400 |
| AOI 快照配对 | 719,320 / 719,320 | 719,987 / 719,987 | 719,853 / 719,853 |
| R3 敌方记录 / 越界 | 133,596 / 0 | 182,566 / 0 | 259,476 / 0 |
| 结束时宿主 1 / 5 / 15 min loadavg | 177.92 / 172.14 / 161.86 | 15.83 / 16.77 / 18.77 | 46.91 / 32.85 / 25.07 |

- 第 1 轮：10:07:06–10:37:30 SGT（含采样、启动与归并），54,223 tick；客户端各在线 1800.031–1800.042 s。20/20 加入、HQ 部署、收 C3（35,999 包），加入失败、断线、解码错误、缺基线均为 0。日志完整性检查通过。[逐轮 JSON](loadtest-1119/carentan-wt-30min-b6-soakpin-run1.json)。
- 第 2 轮：10:37:30–11:07:55 SGT，54,216 tick；客户端各在线 1800.031–1800.034 s。20/20 加入、HQ 部署、收 C3（35,993 包），加入失败、断线、解码错误、缺基线均为 0。日志完整性检查通过。[逐轮 JSON](loadtest-1119/carentan-wt-30min-b6-soakpin-run2.json)。
- 第 2 轮最慢 tick 49158：81.440 ms 墙钟、3.460 ms on-CPU、77.612 ms runq。唯一 CPU 主导的超时 tick 25976：38.987 ms 墙钟、36.172 ms on-CPU、2.836 ms runq；移动阶段墙钟 27.070 ms。未重新采内核等待栈，不能据此断言消除了所有文件系统或调度尾延迟。

**结论：** 两次均通过本批 Carentan WT 无损连续超时门槛，P50/P99 也通过 E1-2 数值门槛。相较 B5-E1 同时改变了亲和性、日志介质和宿主负载，不能从这两轮拆分单项收益。E1-2 与 M1-068 仍为 `partial`：Carentan 1800 s netem、参考机、宿主独占核、满编载具和真人未测。若共享宿主再出现连超，下一步优先在授权的独立主机或管理员划分的 cgroup v2 隔离 cpuset 上复测（整个物理核及 SMT 同胞），再考虑需要人工批准维护窗口的 `isolcpus` 方案；本批未改宿主配置或重启。
