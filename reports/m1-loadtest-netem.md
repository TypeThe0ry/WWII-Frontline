# M1-068b：20 客户端联测（netem 与 30 分钟 WT，工作站数字）

机器：2×AMD EPYC 7402（96 线程）、Linux 7.0.0-14，release 构建，服务器固定在 CPU 14,42、headless 客户端固定在 CPU 11,43（用 `LOADTEST_SERVER_CPUS / LOADTEST_CLIENT_CPUS` 选了当时最空的核；JSON 里 `host.note` 的“CPUs 0-1/2-3”是脚本里的固定文字，实际以此为准）。工作站同时在跑别的会话的任务（平均负载 30–90），本机不是上次 4 核 VM。20 个 headless 客户端 + 12 个 bot。**netem 在私有网络命名空间里的 lo 上设置**（`scripts/loadtest-netem.sh`，需 root；宿主 loopback 和其它本地服务不受影响）。单向延迟每包生效，RTT = 2×延迟。原始 JSON 在 `reports/loadtest-1119/`。

| 运行 | netem | 时长 | tick P50 / P99 / max ms | 超时（连续） | 下行 P50 / P99 kbps | 上行 P50 kbps | RTT P50 / P99 ms | 校正（snap） | 解码错/断线/加入失败 | 判定 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ws 无损（本机重跑） | delay 0 | 300 s | 2.231 / 5.607 / 27.9 | 0 | 17.8 / 26.7 | 20.5 | 0.3 / 3.6 | 13 388（24） | 0 / 0 / 0 | ALL OK |
| ws 丢包+抖动 A | delay 25±10 ms，loss 2 % | 300 s | 2.170 / 5.003 / 15.9 | 0 | 17.3 / 26.4 | 20.5 | 52.5 / 159.7 | 16 547（419） | 0 / 0 / 0 | ALL OK |
| ws 丢包+抖动 B | delay 40±15 ms，loss 5 % | 300 s | 2.445 / 5.637 / 29.3 | 0 | 17.4 / 27.4 | 20.5 | 97.1 / 370.5 | 15 698（1 350） | 0 / 0 / 0 | ALL OK |
| ws 150 ms RTT | delay 75 ms | 300 s | 1.776 / 4.769 / 8.2 | 0 | 14.2 / 22.3 | 20.5 | 150.3 / 153.5 | 18 373（2 530） | 0 / 0 / 0 | ALL OK |
| **wt 30 分钟** | delay 0 | 1800 s | 2.461 / 5.804 / **201.3** | 3（连续 1） | 21.4 / 29.5 | 20.5 | 0.30 / 3.2 | — | 0 / 0 / 0 | ALL OK（工具判定） |

- **服务器 RSS**：ws 300 s 各组 -13 % 左右（预热后回落）；WT 30 分钟 17.77 → 18.05 MB（后 592 s 窗口 +1.56 %），无泄漏迹象。
- **E1-2**（P99 ≤ 28 ms）：所有运行 P99 4.8–5.8 ms，PASS；max 在 ws 各组 ≤29 ms，**WT 30 分钟出现一次 201 ms 的单刻尖峰**（3 次超时，最长连续 1），该次未采集 trace，原因待定。
- **E1-3**（下行 P50 ≤ 128、P99 ≤ 400 kbps，上行 ≤ 32 kbps）：所有运行下行 P50 14–21 kbps、P99 22–30 kbps，上行 P50 20.5 kbps，PASS（含丢包/抖动）。
- **E1-5 命中率对照（服务器事件 hits/shots）**：无损 111/621 = 17.9 %，150 ms RTT 130/577 = 22.5 %，差 +4.6 个百分点（≤ 5）。**这不是有效的延迟补偿检验**：命中率是整个房间（12 个服务器端 bot + 20 个合成输入客户端）的服务器事件比，不含“客户端看到命中”的判定，每组仅约 600 发，二项分布标准误约 1.6 个百分点，差值在噪声内；真正的 E1-5 需要让带 150 ms RTT 的客户端按射线瞄准、与同一目标的零延迟对照，这个场景（`m1_loadtest20.rs`）**没有做**。
- 未做：AOI 审计（audit.rs）、`m1_loadtest20.rs` 场景、WT 的 netem 组合（只做了无损 30 分钟）。

---

# 续（2026-11-19）：30 分钟 WT + netem，以及 201 ms 尖峰的归因

新增运行（20 客户端 + 12 bot，WebTransport，1800 s，服务器固定在较空的核上，`FRONTLINE_TICK_TRACE=1`）：

| 运行 | netem | tick P50 / P99 / max ms | 超时（连续） | 下行 P50 / P99 kbps | 上行 P50 | RTT P50 / P99 ms | RSS 增长 | AOI 审计 | 判定 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **wt 30 分钟 + netem** | delay 25±10 ms，loss 2 % | 2.87 / 5.30 / **34.5** | 1（连续 1） | 19.8 / 26.3 | 20.5 | 50.8 / 145.7 | +0.59 % | 166 万条记录，泄漏 0 | ALL OK |
| wt 30 分钟（无损，trace） | 无 | 2.38 / 5.28 / **153.8** | 1（连续 1） | — | 20.5 | — | +0.8 % | 160 万条记录，泄漏 0 | ALL OK |

（netem 组 `tick_interval_error` P99 0.001 ms、最大 4.5 ms；两组都没有断线、解码错误或加入失败。）

## 尖峰归因
服务器新增了开关 `FRONTLINE_TICK_TRACE=1`：每个超时 tick 的日志带上本线程的 on-CPU 时间和**运行队列等待时间**（`/proc/thread-self/schedstat`）以及六个阶段的墙钟耗时（输入 / step / step 后 / bot / AOI+快照 / 发送）。

- 上次的 201 ms 尖峰（无 trace）：工作站平均负载约 30–90。
- 这次无损 trace 组抓到一次 **153.8 ms** 的 tick（tick 38110）：**on-CPU 仅 2.35 ms，运行队列等待 151.3 ms**，阶段拆分里 151.2 ms 落在第一个阶段（输入处理，线程刚醒来就被调度出去），step 只有 2.15 ms。也就是说**线程是可运行状态但一直排不上 CPU**，不是服务器在算。当时机器平均负载 136（整个运行期间中位数 122、最大 154，96 线程），我们固定的 4 个核（以及它们的超线程兄弟核）全程 90–99 % 忙：`taskset` 只限制我们不去别的核，并不能把别人挡在这几个核之外。
- netem 组的最大 34.5 ms（tick 11643）：on-CPU 32.3 ms、运行队列等待 0.0 ms，是一次真实的计算尖峰（约为平均 2.9 ms 的 11 倍，54 000 个 tick 里一次），当时负载较低（约 17–27）；我们的计时没有带阶段拆分（那一版还没加），所以不知道是哪个阶段，没有复现。
- 结论：154 ms 尖峰归因于共享工作站的 CPU 调度等待。更早的 201 ms 尖峰没有 trace，确切原因未验证。较安静的核上曾出现 34.5 ms 的计算尖峰，当时尚无子阶段计时。要消除调度噪声需要独占核（cgroup cpuset/isolcpus）或空闲机器，本机做不到。

---

# 续二（2026-11-19）：阶段计时常开 + 三组 30 分钟并行复测

`FRONTLINE_TICK_TRACE` 现在**默认开启**（`=0` 关闭）：每刻在执行前后读取 `/proc/thread-self/schedstat`，并对六个阶段计时；超时 tick 记 `tick_overrun`，耗时超过周期 1/3 但未超时的记 `tick_slow`，都带 `cpu_ms`、`runq_wait_ms`、`phase_*_ms`。三组 1800 s 同时跑（wt+netem、wt 无损、ws 无损；原始日志 `reports/loadtest-1119/phase-*.json`）：

| 运行 | tick P50 / P99 / max ms | 超时 | 判定 |
| --- | --- | --- | --- |
| wt + netem（25±10 ms，2 %） | 2.19 / 5.58 / 26.3 | 0 | ALL OK |
| wt 无损 | 2.46 / 5.82 / 25.4 | 0 | ALL OK |
| ws 无损 | 2.28 / 5.23 / 42.9 | 2 | ALL OK |

## 归因（按 cpu_ms 与 runq_wait_ms 分）
- **大于 20 ms 的 tick 共 8 次，7 次是调度等待**：例如 wt+netem tick 46974/46975（cpu 11.0/11.6，runq 10.4/13.8）；wt 无损 tick 3146（cpu 3.2，runq 22.5，阶段全落在“快照”）；ws tick 35607（cpu 1.6，**runq 41.3**，阶段全落在“发送”，线程刚醒就被换下）。耗时落在哪个阶段只取决于被抢占时线程正好在哪，不代表那阶段慢。
- **1 次是真实计算尖峰**：ws tick 28499，cpu 37.5 ms、runq 0.5 ms，`phase_step` 36.8 ms，其余阶段都 <1 ms；前一刻 28498 也偏慢（cpu 12.0）。即问题在 sim step。
- 若干慢 tick 的编号在并行运行中相同，包括 4929、11358、14904–14905、15398、21665 和 42147。事后找回原始日志，WS 与 WT 无损的 tick 4929 日志时间戳均为 `1795045038.694`，三组服务的启动时间跨度为 11 ms，且三个服务器 match UUID 不同。客户端 seed 都是 1。相同 tick 号不足以证明世界状态相同或存在脚本触发，尤其是几轮共享同一工作站。旧日志没有 step 子阶段计时。
- 其余 `tick_slow` 日志（11–17 ms）大多是 cpu 6–10 ms 加 runq 3–8 ms，即正常步进 + 正常调度抖动。

## 对 E1-2 的含义
P99 4.8–5.8 ms、三组最大 26–43 ms；P99 门槛 28 ms 有 5 倍余量。最大值超过 28 ms 的只有 ws 一组两次，其中 41 ms 一次是纯调度、一次 37.5 ms 是真实 step 开销（超出约 33.33 ms 的周期，未造成连续超时）。

## M1-041d 第 4 部分：带驾驶客户端的短联测（2026-11-19）

在 EPYC 7402 工作站同机运行 release 构建，M0 sandbox，20 个 WS headless（客户端 0 驾驶，其余徒步）+ 12 个服务器 bot。服务器固定在 CPU 0–1，客户端固定在 CPU 2–3。`tc netem` 只作用在私有 netns 的 `lo`，25±10 ms 是配置的单向延迟，2 % 是配置的包丢失率；WS/TCP 会重传，不能把这个百分比当作应用快照丢失率。工作站数字不替代 RTX 3060 参考机（PI-109）。

```sh
cargo build --locked --release -p frontline-server -p frontline-headless-client
scripts/loadtest-netem.sh --netem 'delay 25ms 10ms loss 2%' -- \
  --no-build --transport ws --count 20 --bots 12 --drive-vehicle --duration 300 \
  --out /data/scratch/batch/A/ws-drive-25ms-loss2-300s.json
```

服务器只在夹具模式下配置 Crewman 和 M4A1。客户端靠 `SelfState` 确认入座，在座位 0 沿同一方向前进、倒车折返，末尾制动并长按离座。载具统计在 `per_client[0].vehicle`，不混入步兵纠正：用同 tick 的量化权威位置计算误差，≥0.05 m 回滚并重放未确认且晚于该 tick 的命令；≥1.0 m 另记硬纠正。找不到预测 tick 的重新锚定单列，不给它填假误差。首次锚定不算纠正；没有有限纠正时 `error_m` 为 `null`。驾驶员乘车期间及离座后短暂跳过基于步兵预测位置的客户端 AOI 审计，其他客户端的审计逻辑不变。

实测汇总：[ws-drive-netem-300s.json](loadtest-1119/ws-drive-netem-300s.json)。采集使用基线 `0f1c59e` 加本任务代码，早于上游 bot 行为改动 #129；下列数值不代表更新后的 bot 场景。

| 项 | 实测 |
| --- | --- |
| 驾驶闭环 | 300.031 s 在线，入座/离座各 1 次；驾驶槽 128 / 座位 0，离座后 `(0, 0)`。2,951 份驾驶快照、8,827 条驱动指令，权威最大水平位移 13.564 m |
| 载具对账 | 2,950 次同 tick 比较，59 次有限纠正（硬纠正 0），缺 tick 重锚 0；纠正幅度 P50 / P99 / max = 0.0614 / 0.0979 / 0.0979 m |
| 服务器 tick | P50 / P99 / max = 2.145 / 7.148 / 410.372 ms，超时 15 次、最长连续 1 次，跳过定时槽 80 次；最大尖峰 trace：on-CPU 2.919 ms，运行队列等待 408.058 ms |
| 网络 | RTT P50 / P99 = 52.819 / 173.601 ms（所有客户端）；下行 P50 / P99 = 18.680 / 27.760 kbps，上行 P50 = 20.384 kbps；20/20 加入，解码错/缺基线/断线均 0 |
| AOI | 服务器泄漏计数 0；独立客户端审计检查 302,361 条敌方记录，其中 51,798 条超出近距边界，报告 13 条异常（徒步客户端 2、6、8）。本轮未开启逐条审计 trace，无法确认是否是真实泄漏 |

**判定：驾驶闭环通过；全场联测因 AOI 独立审计未通过（partial）。** 未放宽或关闭该硬判据。原始服务器和 headless 日志/JSON 保留在工作站 `/data/scratch/batch/A/loadtest-js6Dt4/`。

合入 #129 后用同样的 netem 配置补跑了 12 s 冒烟（debug，2 WS + 12 bot）：汇总通过，入/出各 1 次，最大水平位移 9.283 m，96 次比较中 5 次纠正（最大 0.0748 m），硬纠正/缺 tick 重锚均 0，tick P99 12.741 ms。仅是回归检查，不代替 release 的 300 s 场景或 AOI 复查。

**未测**：浏览器实车和画面平滑、WT 行车、120 ms RTT 直线对账门槛（E1-12），以及真实网络下的瞄准延迟补偿对照（E1-5）。

## B2-A：M1-068 驾驶联测 AOI 归因与复测（2026-11-19）

在共享 EPYC 7402 工作站上顺序执行两组 release WS，M0 sandbox，20 个 headless 客户端（1 个驾驶员）+ 12 个 bot。服务器固定到 CPU 0–1，客户端固定到 CPU 2–3。netem 只在私有命名空间对 `lo` 注入 25±10 ms 单向延迟与 2% 包丢失。工作站数字不替代 RTX 3060 参考机验收（PI-109）。

```sh
cargo build --locked --release -p frontline-server -p frontline-headless-client
scripts/loadtest-m0.sh --no-build --transport ws --count 20 --bots 12 --drive-vehicle --duration 300 \
  --out /data/scratch/batch/b2/A/ws-drive-aoi-lossless-300s.json
scripts/loadtest-netem.sh --netem 'delay 25ms 10ms loss 2%' -- \
  --no-build --transport ws --count 20 --bots 12 --drive-vehicle --duration 300 \
  --out /data/scratch/batch/b2/A/ws-drive-aoi-netem-300s.json
```

| 实测项 | [无损](loadtest-1119/ws-drive-aoi-lossless-300s.json) | [netem](loadtest-1119/ws-drive-aoi-netem-300s.json) |
| --- | ---: | ---: |
| 同 tick 配对 / 解码快照数 | 89,828 / 89,828 | 89,637 / 89,637 |
| 独立审核的敌方记录 / 其中 >105.6 m | 368,962 / 61,945 | 350,168 / 70,524 |
| 独立 R3 违规 / 服务器泄漏计数 | 0 / 0 | 0 / 0 |
| 旧预测审计告警 | 0 | 6 |
| tick P50 / P99 / max（ms） | 2.786 / 9.109 / 163.964 | 2.499 / 6.305 / 261.554 |
| tick 超时数 / 最长连续 / 跳过定时槽 | 8 / 1 / 17 | 10 / 1 / 36 |
| RTT P50 / P99（ms） | 0.270 / 4.075 | 52.544 / 163.551 |
| 加入成功 / 断线 / 解码错 / 缺基线 / 乱序 | 20 / 0 / 0 / 0 / 0 | 20 / 0 / 0 / 0 / 0 |
| 入座 / 离座 / 驾驶权威最大位移 | 1 / 1 / 12.189 m | 1 / 1 / 13.439 m |
| 驾驶同 tick 比较 / 有限纠正 / 硬纠正 / 缺 tick 重锚 | 2,972 / 0 / 0 / 0 | 2,966 / 51 / 0 / 0 |

netem 的 6 条旧告警均为徒步客户端收包时使用预测位置导致。只替换成同 tick 权威位置，6 条全部落入合法近距范围（94.34–103.53 m）。只替换朝向，6 条仍触发告警（预测距离 109.85–115.34 m）。例如接收者槽 2 在 tick 3316 收到敌方槽 7，最近预测位置测得 113.93 m / 168.26°，同 tick 权威位置测得 95.76 m / 166.88°。两组均未观察到快照乱序。上批 13 条缺逐条日志，无法逐条复核。

新审计按接收者槽位和 C2 tick 配对解码记录与当刻服务器观察者位姿，独立计算 R3 离开阈值，只留位置量化误差。两组均通过硬门禁。最大 tick 尖峰分别有 159.876 / 260.592 ms 落在启用 trace 的快照阶段，on-CPU 分别为 5.194 / 2.412 ms，运行队列等待 0.052 / 0.000 ms。阻塞来源尚未定位。完整 30 min M1 驾驶场景和端到端瞄准对照仍未测。原始日志在 `/data/scratch/batch/b2/A/loadtest-Fsi5LW/` 和 `/data/scratch/batch/b2/A/loadtest-FQKlBW/`。

## B2-G 工作站 WT 30 分钟 soak 与历史慢 tick 复跑（2026-11-19）

### 配置

- 2× EPYC 7402 共享工作站，Linux 7.0.0-14-generic。同机 loopback，M0 sandbox，release，20 个 headless + 初始 12 bot，客户端 seed 1，预热 5 s，无 netem。WT 开启 `FRONTLINE_BOT_FILL=16`，即每队填到 16 个席位。测量基线是 `b317907` 加本节的 step probes。PI-109 的 RTX 3060 参考环境未测。
- `systemd-run --user --scope -p AllowedCPUs=29,33,34` 的等价用户 scope 探针仍显示 `Cpus_allowed_list: 0-95`，父级仅委派 `cpu memory pids`，未委派 `cpuset`。使用 `taskset` 固定服务器至 CPU 29、33、34 和 headless 至 15、17，并逐线程核对了 `/proc/<pid>/task/*/status`。三颗服务器物理核按主线程及其 SMT sibling 的启动前空闲率选取。各实验与本地 CI 使用互不重叠的物理核，宿主其余任务仍可调度到这些核。
- `FRONTLINE_STEP_TRACE=1` 增加 `tick_step_trace`，按 tick 与 `tick_slow`/`tick_overrun` 联结。计时分为步兵移动（含碰撞）、武器、弹丸、载具、火箭、炮弹及投掷物（含各自处理的爆炸）。`other_ms` 是 step 总耗时减去这七个系统，包含 ECS 调度和世界数据整理。此前只记录整个 step。本次的子阶段仍是墙钟计时，需要对照整 tick 的 `cpu_ms` / `runq_wait_ms`，调度等待可能落在任意子阶段。没有单独的载具或爆炸压力场景。

```sh
cargo build --locked --release -p frontline-server -p frontline-headless-client
LOADTEST_SERVER_CPUS=29,33,34 LOADTEST_CLIENT_CPUS=15,17 \
  FRONTLINE_MAP=m0 FRONTLINE_MODE=sandbox \
  FRONTLINE_MATCH_ID=0a9003ec-f0b5-4ac1-a09a-81d05072d400 FRONTLINE_BOT_FILL=16 \
  FRONTLINE_TICK_TRACE=1 FRONTLINE_STEP_TRACE=1 FRONTLINE_LOG_RATE_LIMIT=200 \
  FRONTLINE_STEP_TRACE_TICKS=4929,11358,14904,14905,15398,21665,28498,28499,42147 \
  taskset -c 12-23,60-71 scripts/loadtest-m0.sh --transport wt --count 20 --bots 12 \
    --seed 1 --duration 1800 --no-build --out /data/scratch/batch/b2/G/wt-summary.json
```

### WT 实测

- 20/20 加入。20 名客户端的在线时间为 1800.031–1800.033 s。预热后、标称结束前的 179 次 `/metrics` 采样均为 11 US + 9 GER 客户端、12 bot。客户端结束后的最后一次采样为 0 客户端和 12 bot。
- 服务器 54,222 tick。tick P50 / P99 / max 为 **2.099 / 5.894 / 75.676 ms**。超时 2 次，最长连续 1 次，跳过 3 个定时槽。工作站本轮达到 E1-2 的 P50、P99 和连续超时门槛。
- 单客户端汇总下行 P50 / P99 为 21.216 / 30.296 kbps，上行 P50 为 20.552 kbps。加入失败、断线、解码错和缺基线分别为 0、0、0、0。主测构建的客户端 AOI 审计检查 1,612,038 条敌方记录，报告 0 条异常，服务器泄漏计数 0。末窗 593 s 内 RSS 变化 +1.501 %。联测工具判定 ALL OK。
- 服务器 CPU 29、33、34 启动前 5 s 的空闲率分别为 66.12 %、70.39 %、66.94 %。运行期按 10 s 采样的平均空闲率分别为 67.37 %、61.78 %、65.45 %。

| >20 ms 的 tick | 日志事件 | 总耗时 ms | on-CPU ms | runq wait ms | 墙钟耗时最长的服务器阶段 |
| ---: | --- | ---: | ---: | ---: | --- |
| 1551 | `tick_slow` | 23.426 | 2.662 | 20.728 | step，其中移动 22.442 ms，阶段总计 22.575 ms |
| 6312 | `tick_overrun` | 47.728 | 2.224 | 45.466 | 快照，阶段总计 45.381 ms |
| 33283 | `tick_overrun` | 75.676 | 2.512 | 73.276 | 发送，阶段总计 73.354 ms |

这 3 个 tick 均由运行队列等待主导。最长的 tick 33283 等待 73.276 ms，实际在 CPU 上运行 2.512 ms。

### 历史慢 tick 复跑

两轮 WS 都跑了 975 s。各用 20 headless、12 个静态 bot、客户端 seed 1，并复用了日志里旧 WS 的服务器 UUID `7c3dff01-ff3c-4dce-8d1f-ebdaac92f041`。v4 使用主测构建，服务器固定在 CPU 27、35、40，客户端固定在 16、18。v3 从 `02d5b27` 归档并加上相同探针，在服务器 CPU 28、32、37 和客户端 19、21 上运行。v3 归档没有 `.git`，启动日志中的 `git_sha` 来自外层 worktree。原始场次缺少逐 tick 已应用输入和世界状态，且实时网络接入顺序和输入到达时刻会变化，因此本次属于同场景复跑，逐 tick 确定性重放仍未完成。

| 运行 | 4929 全 tick / step / 移动（ms） | 28499 全 tick / step / 移动（ms） |
| --- | ---: | ---: |
| 旧 WS v3 | 20.616 / 18.961 / 未采集 | 39.561 / 36.818 / 未采集 |
| 新 WS v3 | 1.868 / 1.597 / 1.497 | 4.965 / 3.262 / 3.127 |
| 新 WS v4 | 1.741 / 1.234 / 1.127 | 4.425 / 3.369 / 3.313 |
| 新 WT v4 + bot-fill | 3.040 / 2.091 / 1.788 | 1.811 / 1.366 / 1.244 |

两轮 WS 的 P99 分别为 5.405 ms（v3）和 5.530 ms（v4）。>20 ms 的 tick 分别有 6 个和 3 个，on-CPU 均低于 3.7 ms，runq wait 为 18.335–43.441 ms。WS 与 WT 都没有重现 4929 或 28499 的旧尖峰。复跑在这两个 tick 中测到步兵移动占 step 的大部分时间，旧 WS 的 28499（on-CPU 37.536 ms、runq wait 0.465 ms）的具体子系统仍未定位。

原始日志、按 10 s 抓取的 metrics、客户端和服务器 JSON 及汇总保留在工作站 `/data/scratch/batch/b2/G/`。`evidence.json` 联结了慢 tick 的两种 trace，核对了最大值、超时计数和监测窗口。独占核、参考机以及启用 bot-fill 后的 30 分钟 WT netem 工况未测。
