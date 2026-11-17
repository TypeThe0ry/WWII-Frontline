# M2-021 server：100 玩家 CPU 优化

## 元数据
- 里程碑：M2
- 工作流：server
- 规模：L
- 依赖：M2-020, M2-008
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.10; ARCHITECTURE §6.1; ARCHITECTURE §5.3; ARCHITECTURE §17.2; ARCHITECTURE §18.3

## 背景
单局 100 人（真人 + bot）、模拟 30 Hz（33.3 ms/tick）。ARCH §6.10 预算：**稳态 ≤ 18 ms/tick（54%）**、**P99 ≤ 28 ms**、**绝不连续 2 tick 超 33 ms**；基准硬件 ≥ 3.5 GHz 核，模拟线程 1 核 + 编码/网络线程池（最多 3 核）。阶段预算（墙钟 ms）：1 输入摄取/校验 0.8；2 角色移动（100 个 Rapier KCC，≈ 20 µs/个）3.0；3 载具（≤ 40 辆）2.0；4 武器与弹道（≤ 400 活动弹体段）2.5；5 规则（据点/资源/补给/重生/能力）0.7；6 Bot 思考（≤ 60 bot）3.0；7 历史缓冲写入 0.1；8 AOI 与优先级（~67 个出包客户端 × ~60 候选）2.0；9 快照编码 0.6（CPU 总量 1.7，3 线程并行）；10 序列化与发送 1.5；11 日志/指标 0.2；合计 ≈ 16.4。线程模型：`sim thread`（1，绑核，阶段 1–8）；`net-io`（tokio 多线程 2–4 worker）；`encode pool`（rayon，限 3 线程，**仅服务器**；`sim` 保持单线程）；`bot-llm` 异步。输入经有界 `crossbeam` channel（容量 1 024）→ sim；输出每客户端容量 4 包，满则丢最旧。tick 超 33 ms 时下一 tick **不追赶**，记 `tick_overrun`，连续 3 次触发降频保护（出包降到 15 Hz，ARCH §6.6.3）。
M1 已有 20 人基线（M1-068）。M2 由 M2-008 等规则加重，需要实测与优化。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/tick/stages.rs` —— 11 个阶段的计时器与指标 `server_tick_stage_ms{stage}`（直方图，`/metrics`）
- `crates/server/src/tick/pool.rs` —— 编码并行池（rayon `ThreadPoolBuilder::num_threads(3)`，仅 server）；每客户端编码任务切块
- `crates/server/src/net/aoi.rs` —— 局部排序（只对候选 ~60 做部分排序，用 `select_nth_unstable` / 堆；邻域格增量维护，避免全表扫描）
- `crates/server/src/tick/overrun.rs` —— 不追赶、连续 3 次降频、指标 `server_tick_overrun_total`
- `crates/server/benches/tick_100.rs` —— 100 玩家合成场景基准；`reports/m2-perf100.json`（输出）
- 必要时对 `crates/sim` 热点（KCC、射线检测、空间网格）做局部优化（不得改变规则结果；`state_hash` 基线必须一致）
### 要实现的行为
1. 阶段计时：每 tick 记录 11 个阶段的墙钟，汇总 `server_tick_ms`（直方图，ARCH §18.3）；`/metrics` 暴露 P50/P99。
2. 基准场景 A：100 个 `headless-client`（合成输入）无 bot；场景 B：40 headless + 60 bot（ARCH §6.10 的 bot 常态上限）；各 30 min。
3. Profiling：`cargo flamegraph`（或 `perf`）找前 5 个热点，逐个优化并记录前后数字于 `reports/m2-perf100.md`（报告放 reports，非 docs）。
4. 编码并行：67 个出包客户端的快照编码在 3 线程池并行，sim 线程只做 AOI 选择与任务派发；结果经有界 channel 回收。
5. 超限保护：`tick_overrun` 不追赶；连续 3 次降频到 15 Hz 并在 2 s 后尝试恢复。
6. 内存：`RSS` 末 10 min 增长 < 2%（泄漏检测，ARCH §17.2）。
### 接口与数据
- 指标名按 ARCH §18.3：`server_tick_ms`、`server_tick_overrun_total`、`aoi_starved_total`、`snapshot_budget_utilization`；新增 `server_tick_stage_ms{stage}`。

## 不做什么
- 带宽调节 → M2-020；bot 规模优化（感知缓存/思考分片）→ M2-029；正式 30 min 双传输压测与报告 → M2-049；监控看板/告警 → M2-048。
- 改规则语义或预算数字（超预算写 PLAN_ISSUES，由 Fable 调 `PACKET_BUDGET`/AOI 参数）。
- LLM 指挥官（M3）。

## 验收标准
1. 场景 A 与 B，100 人 30 min：`tick_ms` 稳态（P50）≤ 18 ms、P99 ≤ 28 ms；`server_tick_overrun_total` 无连续 2 tick 超 33 ms（`/metrics` 与日志扫描）。
2. 各阶段墙钟 P50 不超过 ARCH §6.10 表预算 ×1.25（11 个阶段逐项记入 STATUS）；超出者在 PLAN_ISSUES 登记。
3. 末 10 min RSS 增长 < 2%。
4. 优化前后基准：`cargo bench -p server tick_100`，输出 `reports/m2-perf100.json`，至少提升最热的 3 个阶段或证明已在预算内（记录前后中位数）。
5. 优化不改变规则：同一 seed 与输入，优化前后 600 tick 的 `state_hash` 序列逐 tick 一致。
6. 降频保护单测：注入 3 次连续 40 ms tick 后出包频率变 15 Hz；恢复条件满足后回 20 Hz。
7. 参考机：记录 CPU 型号、核数、内核版本、`rustc -V`；无参考机（未达 ≥ 3.5 GHz）时标 `partial`，数字注明机器。

## 测试要求
- 单元测试：阶段计时器、不追赶逻辑、降频状态机。
- 集成/无头：`loadtest-100` 脚本（M1-068 的扩展，参数 `--clients 100 --bots 0|60 --duration 30m`）。
- 性能基准：`cargo bench -p server tick_100`；`cargo flamegraph` 结果存 `reports/`。
- 回归：M1 的 `test-loadtest-20` 指标不回归。

## 性能预算
ARCH §6.10：稳态 ≤ 18 ms、P99 ≤ 28 ms、阶段表如上、合计墙钟 ≈ 16.4 ms；容量估计（4 物理核 2 局）待本任务实测核实。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；按"测量 → 前三热点优化"为止，不追求超出预算的优化；做不完拆续作 M2-021b。
- INDEX 依赖未含 M2-009/M2-010/M2-015 等规则加重项；本任务应在这些完成后测量才有代表性（编号保证其先于本任务），Bot CPU 若尚未优化（M2-029）则场景 B 的 bot 阶段允许 ≤ 预算 ×1.5 并写明。
- 若 Rapier KCC > 3 ms（阶段 2），按 ARCH §5.3 降级方案处理并登记 PLAN_ISSUES。
- 100 人真人规模的网络压力需要人批准租机（NEEDS_HUMAN H-067）；本地单机测不含真实网络栈延迟。
