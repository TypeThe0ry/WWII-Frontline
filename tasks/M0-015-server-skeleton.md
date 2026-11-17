# M0-015 server：进程骨架与 30 Hz 主循环

## 元数据
- 里程碑：M0
- 工作流：server
- 规模：M
- 前置依赖：M0-007, M0-005
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.1; ARCHITECTURE §6.10; ARCHITECTURE §18.3; ARCHITECTURE §18.4; ARCHITECTURE §4.3.4; ARCHITECTURE §5.4

## 背景
对局服务器 `frontline-server`：一个进程 = 一局（ARCHITECTURE §5.4）。主循环跑在**专用 OS 线程**（非 tokio worker），30 Hz，使用**绝对时间表**（`deadline += 33.333 ms`，不累积漂移）；网络在 tokio 线程池；两者通过**有界 channel**（输入容量 1 024；每客户端输出容量 4 包，满则丢最旧，§6.10 线程模型）。超时保护：某 tick 超过 33 ms，下一 tick **不追赶**，记录 `tick_overrun`；连续 3 次超限触发降频保护（出包降到 15 Hz，实际降频逻辑在 M0-019）。监控指标（§18.3）：`server_tick_ms` 直方图（P99 > 28 ms 警告、> 33 ms 严重）、`server_tick_overrun_total`、`server_players{team}`、`server_bots`。日志：`tracing` 结构化 JSON → stdout（§18.4，字段 `match_id/tick/player_id/event/level`，默认 INFO，每秒限速）。
`sim` 此时只有空世界（M0-007）；本任务让进程能起、能稳定走 tick、能被观测。传输在 M0-016/017，会话与输入在 M0-018，AOI/快照在 M0-019。

## 具体内容
### 要创建/修改的文件
- `crates/server/Cargo.toml`：`tokio`（多线程 runtime）、`tracing`、`tracing-subscriber`（json）、`prometheus`、`clap`（derive）、`crossbeam-channel`、`sim`、`protocol`、`bots`（仅链接，M0-026 使用）。
- `crates/server/src/{main.rs,config.rs,logging.rs,metrics.rs,sim_thread.rs,clock.rs,shutdown.rs}`、`crates/server/src/net/mod.rs`（仅 trait 与空实现占位，M0-016 填充）。
### 要实现的行为
1. **配置**：CLI + 环境变量（`FRONTLINE_` 前缀）：`--map m0`、`--mode sandbox`、`--match-id <uuid>`、`--bots <n>`、`--ws-port`、`--wt-port`、`--metrics-port`、`--log-level`、`--duration <s>`（压测用，到时优雅退出）。非法组合启动即退出码非 0 并给出人类可读错误。`--map/--mode` 此时只做解析与校验（地图数据装载在 M0-018）。
2. **sim 线程**：`std::thread::Builder::name("sim")`，循环 `deadline += tick_period`；等待用"睡眠到 `deadline − 1 ms`，再自旋到 `deadline`"的混合方式（可用 `spin_sleep` crate 或自写，记录选择与理由）；`Clock` trait 抽象时间源，便于单测用假时钟。每 tick 流程骨架严格按 ARCHITECTURE §6.1 的 1–7 步写成带 `// TODO(M0-0xx)` 以外的**空函数**占位：`drain_inputs → apply_inputs → sim.step → record_lag_comp(空) → bots.think(空) → aoi_and_snapshot(空) → send(空)`；`TODO` 必须带后续任务 ID（README §5.8）。
3. **超限处理**：tick 耗时 > 33.3 ms → `tick_overrun_total += 1`，不补步；连续超限计数器 `tick_overrun_consecutive`（gauge）与 `tick_overrun_consecutive_max`（gauge，局内最大值，供 E0-1 "无连续 2 tick 超 33 ms"判定）。
4. **指标**：`server_tick_ms` 直方图（桶 0.5,1,2,4,8,12,16,20,24,28,33.3,50,100 ms）、`server_tick_interval_error_ms`（实际唤醒时刻相对绝对时间表的偏差直方图，桶 0.05…5 ms，用于抖动验收）、上述 overrun 指标、`server_players{team}`、`server_bots`、`server_uptime_seconds`。`/metrics`（Prometheus 文本格式）由 tokio 上的极小 HTTP 响应器提供（不新增 HTTP 框架依赖；若确需 `hyper` 写 PLAN_ISSUES）。
5. **日志**：JSON，字段如上；每秒限速（同类事件每秒 ≤ N 条，N 取配置）；启动时打印构建信息（`git sha`、`PROTOCOL_VERSION`、`DATA_HASH`）。
6. **优雅关闭**：SIGINT/SIGTERM → 停止 sim 线程 → 退出码 0；`--duration` 到时同样处理，并在退出前把指标摘要 JSON 写到 `--report <path>`（含 tick_ms P50/P99/max、抖动 P99、overrun 计数）。
### 接口与数据
- `trait Clock { fn now(&self)->Instant; fn sleep_until(&self,Instant) }`；`SimThreadHandle{inputs_tx, commands_tx}`；输入 channel 容量 1 024（`crossbeam` 有界），满时丢弃新输入并计数 `input_queue_full_total`。

## 不做什么
- 不做 WS/WT 传输、握手（M0-016/017）；不做会话、输入校验、地图装载（M0-018）；不做 AOI/快照（M0-019）；不做 bot（M0-026）。
- 不做 gateway 注册、Prometheus 之外的监控栈（M1-029、M2-048）。
- 不做 SIGHUP 证书热更新（M4）。

## 验收标准
1. 空载 10 min（`frontline-server --duration 600 --report r.json`）：`server_tick_interval_error_ms` P99 < 1 ms（Linux，参考机；写入机型/内核）；`tick_overrun_total == 0`。
2. `server_tick_ms` 在 `/metrics` 中可被采集：`curl localhost:<port>/metrics | grep server_tick_ms_bucket` 输出非空；直方图桶数 ≥ 10。
3. `Clock` 假时钟单测：模拟一个 50 ms 的 tick 后，下一 tick 的截止时刻仍为理想时间表（无追赶、无漂移），`tick_overrun_total` 恰好 +1；连续 3 个超限后 `tick_overrun_consecutive_max == 3`。
4. 100 个玩家实体的空 `World` 下 `sim.step` 的 `server_tick_ms` P99 < 1 ms（证明骨架本身开销可忽略；实体由测试直接 spawn）。
5. 日志为合法 JSON 行（`jq -e .` 逐行通过），含 `match_id`、`tick`、`level` 字段；每秒限速测试：一秒内 10 000 次同类日志实际输出 ≤ 配置上限。
6. SIGTERM 在 100 ms 内导致进程退出码 0（脚本测试）。

## 测试要求
- 单元测试：时间表算术、overrun 计数、配置解析（含非法组合）、日志限速器。
- 集成/无头测试：`cargo test -p server --test skeleton_smoke`（起进程/线程 5 s，抓取 `/metrics`，校验指标存在）。
- 性能基准：验收 1 的 10 min 运行；结果交 M0-030。
- 回归：无。

## 性能预算
tick 总预算稳态 ≤ 18 ms、P99 ≤ 28 ms、绝不连续 2 tick 超 33 ms（ARCHITECTURE §6.10）；本任务的骨架开销应 < 1 ms（验收 4）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 抖动 < 1 ms 取决于宿主内核与 CPU 调度；云 CI/容器内可能达不到，需在参考机或物理机测，否则该项记"未测/partial"（README §5.5）。
- 绑核（`sim thread` 绑定 CPU）属部署项（M1-067），此处只留 `--pin-sim-core <n>` 的可选参数入口。
- E0-1 需要的"连续超限"指标（`tick_overrun_consecutive_max`）INDEX 未明写，本任务补上，属细化。
