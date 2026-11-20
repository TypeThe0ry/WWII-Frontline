# B6 Carentan M1 WT 丢包长跑（2026-11-20）

PI-109，EPYC 7402 共享工作站（非参考机）。使用 `carentan/m1` Assault、20 个 WebTransport headless、12 个固定服务端 bot、C0 HQ 部署和私有网络命名空间。服务端和 headless 都使用 release。每客户端设为 1810 s，给连接建立和 HQ 出生错峰预留 10 s。是否达到 30 分钟全员重叠以客户端单调钟记录为准。

## 工况与复现

里程碑源码固定为 `c0d4efbabd5a2332f67f86270e4c18002b42a4d1`。里程碑线已有 Sherman，Panzer IV H、Jeep 和 Kübelwagen 使用 [临时夹具](fixtures/b6-carentan-fleet.py)。它从可玩线固定提交提取三辆车的体积、质量、座位等参数，通过里程碑线的履带刚体接口装入。三辆适配车用无弹药的炮座占位，未进行操纵或射击。车队在双方 HQ 各布置一辆坦克和一辆轻车。夹具仅应用于暂存源码，包含每 300 tick 的服务端车辆状态、每 300 tick 附近的客户端解码车辆槽位，以及每客户端的活动时间记录。未引入渲染素材。

地图按 [B4-D1](m1-carentan-wt-b4-d1.md) 的 `chunks-m1` 流程在暂存路径烘焙，解开 380 块 FCOL，并保留 L3a/L3b 合成巡逻点。原始块路径与内容清单的 SHA-256 为 `9ff574ccc87ffbc1cc83023140a8cc19133d666ad09b72777b7af7ae7c12faeb`，与 B6-soakpin 匹配。使用同一 match ID 和 tick/step trace 配置。网关、GPU 与浏览器均未使用。

在里程碑 worktree 创建 `D` 和 `L`（分别在 `/data/scratch` 和 `/dev/shm`），按 B4-D1 准备 `D/data`，再运行以下命令。CPU 掩码按本次工作站拓扑和开跑前负载采样选择。`FRONTLINE_SIM_CPU=32` 仅隔离本次负载线程，CPU 80 为 SMT 同胞，宿主仍共享。

```sh
mkdir -p "$D/src" "$D/soak"
git archive c0d4efbabd5a2332f67f86270e4c18002b42a4d1 | tar -x -C "$D/src"
python3 reports/fixtures/b6-carentan-fleet.py "$D/src"
(cd "$D/src" && CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0 taskset -c 0-23,48-71 \
  cargo build --locked --release -p frontline-server -p frontline-headless-client)
cd "$D/src"
export FRONTLINE_DATA_DIR="$D/data" LOADTEST_LOG_DIR="$L" TMPDIR="$D/soak"
export FRONTLINE_SIM_CPU=32 LOADTEST_SERVER_CPUS=36,39,42 LOADTEST_CLIENT_CPUS=43,45
export FRONTLINE_MATCH_ID=0a9003ec-f0b5-4ac1-a09a-81d05072d401
export FRONTLINE_TICK_TRACE=1 FRONTLINE_STEP_TRACE=1 FRONTLINE_LOG_RATE_LIMIT=10000
export LOADTEST_NETEM='delay 25ms 10ms loss 2% seed 26'
unset GATEWAY_URL FRONTLINE_DEV_NO_GATEWAY FRONTLINE_STEP_TRACE_TICKS
unset FRONTLINE_PIN_SIM_CORE TOKIO_WORKER_THREADS FRONTLINE_BOT_FILL
taskset -c 46 unshare -n bash -euc '
  ip link set lo up
  read -r -a args <<< "$LOADTEST_NETEM"
  tc qdisc add dev lo root netem "${args[@]}"
  ping -n -c 40 -i 0.05 -W 1 127.0.0.1 > "$TMPDIR/ping.txt"
  tc qdisc del dev lo root
  tc qdisc add dev lo root netem "${args[@]}"
  tc -s -j qdisc show dev lo > "$TMPDIR/qdisc-start.json"
  status=0
  scripts/loadtest-m0.sh --transport wt --map carentan/m1 --mode assault --deploy-hq \
    --count 20 --bots 12 --duration 1810 --seed 1 --no-build --sample-interval 10 \
    --out "$TMPDIR/summary.json" || status=$?
  tc -s -j qdisc show dev lo > "$TMPDIR/qdisc-end.json"
  exit "$status"
'
```

`tc` 在私有 `lo` 的出向路径对每个包实施单向延迟和随机丢包。ICMP 探测完成后重建 qdisc，避免把探测计入长跑丢包统计。服务器和客户端日志写入 tmpfs，RSS/Prometheus 抓取约每 10 秒一次。headless 带宽按应用负载和每消息 58 B 开销估算，单位为十进制 kbps。`tc` 队列统计是私有 `lo` 上所有流量的合计，含 WT 两个方向和本地 metrics。

R3 审计用客户端已解码的 C2 玩家记录与服务端同一接收者、同一 tick 的权威位姿配对，含解码成功的乱序快照。判定沿用 PI-071 关断界限。载具槽位仅用于核实车辆出现在解码结果中。丢失或无法解码的包、客户端独立载具视锥审计、遮挡审计均未测。

## 实测结果

本轮通过 E1-2、E1-3、E1-4 的工作站数值门槛。测试窗口为 2026-11-20 12:28:01–12:58:34 SGT，含启动、末次抓取及归并。`loadtest-m0.sh` 退出 0，共 54,621 tick。[JSON 实测记录](loadtest-1119/carentan-wt-30min-b6-netem.json)

| 项目 | 结果 |
| --- | --- |
| 人数与时长 | 20/20 WT 加入并完成 HQ 部署，各在线 1810.031–1810.033 s。HQ 部署后全员同时活动 1808.767 s。178 次抓取中 175 次为 US 10 / GER 10 / bot 12，另 3 次落在加入和退出阶段。20/20 收到 C3，共 35,487 包。加入失败、断线、解码错误和缺基线均为 0。 |
| 载具 | 四辆车从 tick 0 部署，182 次服务端采样均为 4 在场、4 有血量、0 驾驶员，末次 tick 54,600。20 个客户端采样中解码到共 973 条载具记录，槽位 128/129/130/131 各为 74/66/392/441 条。 |
| netem | qdisc 通过 2,542,179 包、丢弃 52,118 包，实测丢包率 2.009%，结束时队列为空。开跑前独立 ICMP 探测收回 39/40，RTT min/avg/max 为 31.002/48.295/69.349 ms。WT 客户端 RTT P50/P99 为 50.808/146.825 ms，采用 headless 自身采样口径。 |
| E1-2 tick | P50/P99/max **3.809/8.027/84.295 ms**，超时 **7** 次，跳过 **11** 个定时槽，最长连续 **1**。严格按 `>33.000 ms` 再查日志也是 7 次，最长连续 1。7 次超时均由 runq 等待主导，最慢 tick 的 CPU/runq 为 3.298/81.699 ms。 |
| E1-3 下行 | 合并样本 P50/P99 **20.352/25.336 kbps**，最差客户端 P50/P99 分别为 **21.872/27.544 kbps**。 |
| E1-3 上行 | 合并样本 P50/P99 **20.456/21.376 kbps**，最差客户端 P50/P99/max 为 **20.536/21.456/21.648 kbps**。 |
| E1-4 AOI | **709,320/709,320** 个已解码快照同 tick 配对，审计 **397,531** 条敌方玩家记录，其中 3,937 条超出 105.6 m 近界。客户端审计与服务端计数均为 **0 越界**。 |

运行时 `/proc` 核验 sim 为 32，网络/日志为 36,39,42，headless 为 43,45。CPU 32/80 启动前 3 s 平均空闲率为 44.48%/87.63%。验证脚本使用另一组 CPU，宿主资源仍共享。服务器与客户端日志实测均落在 tmpfs，日志完整性检查通过。

## 本地自检与剩余验收

- `FRONTLINE_DETERMINISM_STRICT=1 CARGO_BUILD_JOBS=8 taskset -c 0-23,48-71 scripts/local-ci.sh determinism lint` 退出 0。native/WASM 三种子各 18,000 tick 哈希逐 tick 一致，位置误差 0 m。重基至 `1f889bf` 后 `scripts/local-ci.sh lint` 通过。
- `CARGO_BUILD_JOBS=8 cargo test --locked -p frontline-server --test aoi_audit` 通过 2 项。夹具复生成文件逐一对比一致。20 s 同设定冒烟及 1810 s 长跑均退出 0。
- E1-2/3/4 与 M1-068 保持 `partial`。参考机、独占 CPU、正式载具数据、有人驾驶与交战、外网部署及真人未测。E1-4 的独立同 tick 审计覆盖玩家记录。载具接收另有槽位样本。
