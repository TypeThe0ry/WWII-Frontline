# B5-E1　Carentan 服务器尾延迟与网关签票（2026-11-19）

## Tick 热路径

D1（#151 / PI-127D1）的 `FL_AOI_TRACE` 在快照阶段逐条 `eprintln!`。结构化日志也在调用线程持锁写 stdout 并逐行 flush。D1 后段在 13 个慢 tick 的现场栈里捕获了 ext4/jbd2 文件写入等待。当前实现把两种输出投递到 8,192 条有界队列，由 `log-output` 线程格式化 AOI 行并批量写入，每 50 ms flush。队列满时调用方丢弃并计数，正常关停时汇总记录丢弃和写入错误。负载报告把该记录及 AOI 缺配对纳入硬门禁。

源代码审计还核对了 tick 指标、SQLite 和其余文件路径。tick 指标更新在内存中，HTTP scrape 由运行时处理。地图和 bot 导航在启动时加载，报告和结果补投在 sim 线程 join 后写入，SQLite 只在网关使用。tick 诊断仍同步读 Linux `/proc/thread-self/schedstat`，这是内核计数器读取。`FRONTLINE_TICK_TRACE=0` 可关闭这项诊断。

## D1 的 56 个残差主导 tick

先用 tick 墙钟减去 CPU 时间与 runq 等待。step 内残差的下界是 `max(0, residual - (tick_wall - step_wall))`，上界是 `min(residual, step_wall)`。由此确认 53 个 tick 的多数残差在 `World::step` 外，其中包括全部 23 个残差主导的超时。2 个在 step 内，1 个（7844）在此精度下无法区分。例外 tick 11504 的最大单段是 step，但仅 6.150 ms，小于 13.463 ms 残差的一半。整个 tick 的调度计数和子阶段墙钟不能进一步确定等待机制。

| D1 tick | tick 墙钟 ms | CPU / runq ms | step 墙钟 ms | 最大 step 子系统 |
| --- | ---: | ---: | ---: | --- |
| 1535 | 27.388 | 4.456 / 3.440 | 22.237 | movement 22.079 ms |
| 1660 | 25.590 | 3.355 / 0.350 | 24.299 | movement 24.145 ms |
| 7844 | 12.395 | 2.829 / 2.770 | 4.168 | movement 4.036 ms |
| 11504 | 20.944 | 3.517 / 3.965 | 6.150 | movement 6.012 ms |
| 11923 | 323.875 | 3.950 / 0.486 | 2.089 | movement 1.957 ms，快照阶段 321.099 ms |

子阶段 trace 现在记录 World tick、移动调用合计、最慢移动调用及实体 ID，并分别计时输入准备、ECS 剩余部分、延迟补偿、残骸清理、小队同步、视图刷新和输出收集。汇总按 match ID 和 tick 关联全部残差样本。步兵移动用地形覆盖检查代替仅供判空的高度插值，X/Z 不变时复用覆盖结果，并把输入冻结判定提到玩家循环外。`FRONTLINE_STEP_TRACE_TICKS` 可按原 tick 编号强制输出。在线包到达和主机调度仍会改变实际轨迹。

两次另绑核短跑各 405 s（服务器 `25,30,42`、客户端 `7,9`），用 `FRONTLINE_STEP_TRACE_TICKS=1535,1660,7844,11504,11923` 强制记录 5 个编号。首轮 WT 客户端 15 在连接时收到 peer close 261，19/20 加入。次轮 20/20 加入与 HQ 部署，0 解码错、0 断线，负载脚本退出 0。次轮计时如下。

| tick（server / World 相同） | tick 墙钟 ms | step ms | movement ms | 最慢移动调用 ms（实体 ID） |
| --- | ---: | ---: | ---: | ---: |
| 1535 | 3.190 | 1.789 | 1.632 | 0.153（15） |
| 1660 | 1.836 | 1.523 | 1.394 | 0.104（6） |
| 7844 | 3.407 | 2.371 | 2.325 | 0.167（14） |
| 11504 | 3.310 | 2.289 | 2.185 | 0.330（12） |
| 11923 | 3.434 | 2.076 | 2.024 | 0.188（12） |

两轮均取得同编号 trace，1535 与 1660 的原 movement 尖峰均未重现。没有用短跑差值估计移动优化本身的加速。

## WT 复跑

共享 EPYC 7402 工作站（PI-109，RTX 3060 参考机未测）。与 D1 相同的私有 loopback 网络命名空间、无 netem、`carentan/m1` Assault、380 个原始 FCOL、20 个 WT headless、12 个固定 bot、HQ 部署、种子 1、相同 match ID、每 10 s scrape、两类 trace 开启。服务器线程 CPU `24,29,41`，客户端 CPU `6,10`，已逐线程核对。本轮基于 #151 合入提交并含 #150 室内碰撞，D1 的运行二进制是 #150 前灰盒。核未独占，结果包含宿主负载和版本差异。

地图暂存与烘焙命令沿用 [D1](m1-carentan-wt-b4-d1.md)。以下 `D` 指向该夹具根目录，`D/data` 放解包后的地图。

```sh
cargo build --locked --release -p frontline-server -p frontline-headless-client
mkdir -p "$D/soak"
LOADTEST_SERVER_CPUS=24,29,41 LOADTEST_CLIENT_CPUS=6,10 TMPDIR="$D/soak" \
  FRONTLINE_DATA_DIR="$D/data" FRONTLINE_MATCH_ID=0a9003ec-f0b5-4ac1-a09a-81d05072d401 \
  FRONTLINE_TICK_TRACE=1 FRONTLINE_STEP_TRACE=1 FRONTLINE_LOG_RATE_LIMIT=10000 \
  unshare -n bash -c 'ip link set lo up; unset GATEWAY_URL FRONTLINE_DEV_NO_GATEWAY; \
    exec scripts/loadtest-m0.sh --transport wt --map carentan/m1 --mode assault --deploy-hq \
    --count 20 --bots 12 --duration 1800 --seed 1 --sample-interval 10 --no-build --out "$TMPDIR/m1-wt.json"'
```

| 指标 | D1 | B5-E1 |
| --- | ---: | ---: |
| tick P50 / P99 / max（ms） | 3.519 / 9.810 / 323.875 | 4.189 / 8.644 / 126.668 |
| 超时数 / 跳过定时槽 | 94 / 216 | 29 / 52 |
| 最长连续超时 / 连续两次的对数 | 2 / 3 | 2 / 1 |
| runq / 残差 / 混合主导 | 321 / 56 / 5 | 226 / 0 / 0 |
| 下行 P50 / P99（kbps） | 24.904 / 34.144 | 21.992 / 26.888 |
| 上行 P50 / P99（kbps） | 20.448 / 21.280 | 20.576 / 21.424 |
| AOI 快照配对 | 717,121 / 717,121 | 719,320 / 719,320 |
| R3 敌方记录 / 越界 | 2,682,547 / 0 | 133,596 / 0 |

20/20 完成 HQ 部署并收到 C3（35,966 包），各在线约 1800.031–1800.033 s，0 加入失败、0 断线、0 解码错、0 缺基线。日志完整性检查通过。两个版本的 AOI 敌方记录数和公开场景轨迹不同。负载脚本退出 1，仅最长连续超时（门槛 1）失败。唯一一对相邻超时为 tick 9063/9064，runq 分别为 35.574/36.558 ms。最慢 tick 23169 的 movement 墙钟为 124.453 ms，整个 tick CPU 为 5.041 ms，runq 为 121.631 ms。约每 4 ms 的后续内核采样捕获到 88 个 ext4/jbd2 栈，均在 `log-output` 线程。共享核上的下一步是隔离 sim 核并对齐同版本重复采集 runq 等待。

完整 [WT 汇总](loadtest-1119/carentan-wt-30min-b5-e1.json)。原始日志、二进制摘要、绑核和等待栈记录位于 `/data/scratch/batch/b5/E1/`。

## 网关

SQLite 原已使用单连接池、WAL 和 `synchronous=NORMAL`。原路径每票持有连接执行 BEGIN、游标 UPDATE、游客 UPSERT、COMMIT。昵称相同的常见路径现在在一条带游客条件的 UPDATE 中推进游标，等语句提交后才签票。昵称改动或填写缺失游客继续在同一事务里完成。集成测试覆盖并发游标回绕、昵称持久化及游客写入失败时回滚。

D1 历史 P95 为 53.269 ms（原夹具）和 56.865–61.166 ms（采集指标）。本批使用相同夹具交替复跑七对。每次新建磁盘 SQLite，事先建立 200 条 TCP 连接，然后并发签票。网关绑 CPU `33–35`，Python 客户端绑 CPU `36`。28 轮均签发 200/200 张唯一票。指标采集在请求批次结束后执行。

| 夹具 | 重跑 D1 P95 中位数（范围）ms | B5-E1 P95 中位数（范围）ms | P95 ≤ 50 ms |
| --- | ---: | ---: | ---: |
| 原版 | 60.787（57.964–65.431） | 51.639（39.823–58.562） | 0/7 → 3/7 |
| 带指标采集 | 62.734（52.007–84.039） | 53.495（43.630–118.043） | 0/7 → 3/7 |

带指标夹具每轮平均 pool 等待的中位数从 30.794 降至 23.540 ms，数据库操作均值的中位数从 0.178 降至 0.092 ms。优化后带指标的七轮各有 200/200 个服务端处理器落在 ≤50 ms 桶，客户端端到端仍有超标。保留 **partial**。同机共享负载下的剩余延迟需要通过请求派发、服务端处理和客户端收包时间的同次对照区分。逐轮 [P95 汇总](loadtest-1119/gateway-ticket-b5-e1.json)。原始 JSON、Prometheus scrape、环境和复跑命令保留于 `/data/scratch/batch/b5/E1-gateway/`。

补充调度实验在优化版二进制上各交替复跑 16 对。临时设置 `TOKIO_WORKER_THREADS=1`，原版夹具 P95 中位数从 46.558 降到 40.204 ms（13/16→16/16 轮 ≤50 ms），带指标夹具从 48.730 降到 40.455 ms（9/16→14/16）。后者两轮仍超标且该两轮全部服务端 handler ≤50 ms。额外的 FIFO 准入闸没有提高达标率，裸连接实验的收益不稳定。运行默认值维持不变。实验命令及请求分段原始数据在 `/data/scratch/batch/b5/E1-gateway/admission/`。

## 未验收项

沿用 PI-109 的工作站标注。独占核、参考机、Carentan 1800 s netem、满编载具和真人场未测。M1-064 正式素材及 M1-067 预发布账号仍阻塞。M1-041d 保护路径无改动。
