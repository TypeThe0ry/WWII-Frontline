# M0-027 headless-client v0 与 10 客户端联测

## 元数据
- 里程碑：M0
- 工作流：test
- 规模：M
- 依赖：M0-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.2; ARCHITECTURE §4.3.7; ARCHITECTURE §6.9; ARCHITECTURE §6.10; MILESTONES §1.3

## 背景
`headless-client` 是无渲染客户端：握手、发送合成输入、解码快照、统计字节分布，用于压测与集成测试（ARCHITECTURE §4.3.7）。M0 版（v0）只做 10 个客户端 + 服务器内 10 个 bot 的 30 min 联测，对应 **E0-1/E0-2**：`tick_ms` P99 ≤ 28 ms 且无连续 2 tick 超 33 ms；每客户端下行 **P50 ≤ 128 kbps**、上行 **≤ 32 kbps**（ARCHITECTURE §6.9.5、§6.10）。WT 与 WS 两条路径都要跑；WS 路径阈值放宽（§17.2：P99 ≤ 40 ms tick、快照 15 Hz）。依赖：`sim`、`protocol`、`tokio`、`wtransport`/`tokio-tungstenite`；**禁止依赖 `server`**（§4.5）。M0 不做延迟/丢包注入（`tc netem` 属 M1-068）、不做客户端预测（`--predict` 留给 M1）。

## 具体内容
### 要创建/修改的文件
- `crates/headless-client/src/{main.rs,cli.rs,client.rs,transport/{mod.rs,ws.rs,wt.rs},synth_input.rs,stats.rs,report.rs}`。
- `scripts/loadtest-m0.sh`（启动服务器 `--bots 10`、启动 N 个 headless、每 10 s 抓取 `/metrics`、结束时汇总）、`scripts/summarize-loadtest.mjs`（读取抓取的直方图，计算 tick_ms P50/P99/max 与连续超限最大值）。
- `reports/m0/loadtest-wt.json`、`reports/m0/loadtest-ws.json`（运行产物，入库摘要而非原始抓取）。
### 要实现的行为
1. CLI：`headless-client --url <host:port> --transport wt|ws --count 10 --duration 1800 --seed 1 --report out.json [--cert-hash <b64>]`；每个客户端一个 tokio 任务。
2. 握手：`Hello`（`PROTOCOL_VERSION`、`DATA_HASH` 取 `sim::GameData`）→ `Welcome`；Reject 即失败并计数。
3. 合成输入（30 Hz）：路点行走（路点来自 `spawns.json` 的 `patrol_points`，按种子选择）+ 随机开火（每 2–6 s 一次点射）+ 偶尔跳/蹲；`InputBundle` 带最近 3 条、`ack_snapshot_tick`（收到快照后更新）、`pred_hash`（固定使用权威位置量化 CRC8——不预测）；`tick` 字段用与客户端相同的时钟公式（`server_tick` 估计 + `ceil(RTT/2/33.3 ms)+1`），使输入落在服务器缓冲窗内。
4. 接收：C2 快照经 `SnapshotDecoder` 解码，统计：每秒下行字节、包数、解码错误数、`MissingBaseline` 次数、关键帧占比；Ping/Pong 得 RTT 分布；每秒上行字节。
5. **带宽口径**：报告同时给出"应用负载 kbps"与"线路估计 kbps（+58 B/包，ARCHITECTURE §6.9.1）"，P50/P99/最大；上行同理。判定使用线路估计口径。
6. 报告 JSON：`{transport, count, duration_s, per_client:[{id, down_kbps:{p50,p99,max}, up_kbps:{…}, rtt_ms:{…}, decode_errors, missing_baseline, disconnects}], server:{tick_ms:{p50,p99,max}, overrun_total, overrun_consecutive_max, aoi_encode_ms:{p99}, bytes_out_per_client:{p50,p99}}}`；`server` 段由 `scripts/summarize-loadtest.mjs` 合并。
7. 连接中断处理：被服务器踢出/断线时记录并退出该任务，不影响其它客户端；总体 `disconnects` 必须为 0 才算通过。
### 接口与数据
- 与浏览器客户端走同一线上格式（M0-016 `framing`、M0-017 datagram 格式）；`headless-client` 不 import 任何 `server` 代码，测试用服务器二进制由脚本启动。

## 不做什么
- 不做 20/100 客户端与 netem（M1-068、M2-049）；不做对账误差统计（需预测，M1+）。
- 不做 `balance-sim`、不做浏览器自动化（M0-028/030）。
- 不做 bot 行为改进（M0-026 已定）。

## 验收标准
1. **WT 30 min**：`scripts/loadtest-m0.sh --transport wt --duration 1800`：10 headless + 10 bot，`tick_ms` **P99 ≤ 28 ms**，`overrun_consecutive_max ≤ 1`（即无连续 2 tick 超 33 ms）；每客户端下行 **P50 ≤ 128 kbps**（线路估计口径）、上行 P50 ≤ 32 kbps；`disconnects == 0`、`decode_errors == 0`。数字与 `reports/m0/loadtest-wt.json` 路径写入 STATUS。
2. **WS 10 min**（放宽线，§17.2）：`tick_ms` P99 ≤ 40 ms；下行 P50 ≤ 128 kbps（快照 15 Hz 下应更低）；`disconnects == 0`。
3. 参考典型值对照：报告中并列 ARCHITECTURE §6.9.2 的典型推导 ≈ 54 kbps，若实测 > 2× 推导值，在 PLAN_ISSUES 写原因分析（不一定失败）。
4. 服务器内存稳定：运行最后 10 min RSS 增长 < 2%（`/proc/<pid>/status` 采样，ARCHITECTURE §17.2 泄漏检测）。
5. 报告 schema 测试：`cargo test -p headless-client report_schema`；合成输入确定性：同 `--seed` 两次运行的输入序列哈希相同。
6. `cargo tree -p headless-client` 不含 `server`；`check-deps.mjs` 通过。

## 测试要求
- 单元测试：路点行走、输入序列化、带宽窗口统计（P50/P99 的计算与边界）、报告生成。
- 集成/无头测试：验收 1/2（长跑）；CI 内保留一个 60 s、3 客户端冒烟（`loadtest-smoke`），不进 PR 必跑集合。
- 性能基准：记录 headless 自身 CPU（避免压测机成为瓶颈：单进程 CPU < 50% 单核）。
- 回归：把验收 1 的指标作为 M0-030/033 与 M1-068 的对照基线。

## 性能预算
`tick_ms` P99 ≤ 28 ms，无连续 2 tick 超 33 ms（ARCHITECTURE §6.10）；每客户端下行 P50 ≤ 128 kbps、P99 ≤ 400 kbps（§6.9.5 压测线；M0 判 P50）；上行 ≤ 32 kbps。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **依赖缺口**：验收场景含 10 个服务器内 bot，需要 M0-026；INDEX 只列 M0-019。按编号顺序 M0-026 先完成，不影响执行，但如果有人乱序，务必先做 M0-026。
- 压测机本身的 CPU/网络栈会影响抖动；服务器与 headless 最好分机或绑核，否则 P99 数字"不得标 done"，改 `partial` 并说明（README §5.5）。
- 30 min 压测需要稳定的 loopback 环境；CI 不跑，由执行者在本机/参考机执行并存报告摘要。
