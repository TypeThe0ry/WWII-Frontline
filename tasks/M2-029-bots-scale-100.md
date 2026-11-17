# M2-029 bots：100 bot 规模优化

## 元数据
- 里程碑：M2
- 工作流：bots
- 规模：M
- 依赖：M2-028
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §16.5; ARCHITECTURE §16.1; ARCHITECTURE §16.2; ARCHITECTURE §16.3; ARCHITECTURE §6.10; GAMEDESIGN §20.5; GAMEDESIGN §20.6

## 背景
M2 退出标准 E2-4（MILESTONES §3.3）要求：60 bot ≤ 3.0 ms/tick，100 bot（压测）≤ 5 ms/tick。M2-027/028 已实现 Frontline 目标行为与规则式指挥官，但只验证了功能。本任务只做 `crates/bots` 内的性能与规模化，使 100 bot 全 bot 对局（balance-sim、压测）可行。
ARCHITECTURE §16.5 的预算分解：士兵 bot 每 3 tick 思考一次、按 `bot_id % 3` 分片，单次约 30–60 µs（含感知）→ 60 bot 约 1.0 ms/tick；路径查询 ≤ 1.0 ms/tick（每 bot 重算 ≤ 2 Hz，ARCH §16.2）；战术/射击决策约 1.0 ms/tick；合计约 3.0 ms/tick，对应 ARCH §6.10 阶段 6 的 3.0 ms。bot 感知必须与真人一致（视锥 120°、距离衰减识别概率、听觉事件，ARCH §16.3），优化不得引入"全知"。
行为树每 bot 节点数 ≤ 80（ARCH §16.3）。难度档来自 GAMEDESIGN §20.6（Recruit/Regular/Veteran：反应 0.55–0.85 / 0.35–0.60 / 0.25–0.45 s，瞄准误差 ×2.0/×1.3/×1.0，视距 150/220/300 m）。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/perception_cache.rs` —— 每 tick 一次构建的共享感知缓存：按 64 m 网格的玩家/bot 空间索引（用 `indexmap`/排序 Vec，禁用 `HashMap`），供视觉候选筛选；同一 tick 内多个 bot 复用，避免 O(N²) 全表扫描。
- `crates/bots/src/scheduler.rs` —— 思考分片：`(tick + bot_id) % 3 == 0` 的 bot 本 tick 思考；高层（小队长 2–5 s、指挥官 5 s 级）错峰；每 tick 总思考预算超限时把剩余 bot 顺延到下一 tick（记录 `bots_deferred_total`）。
- `crates/bots/src/path_budget.rs` —— 路径查询预算器：全局队列、每 tick 总耗时 ≤ 1.0 ms（超出则延后）、每 bot 最多 2 Hz 重算、目标未变且路径仍有效时复用。
- `crates/bots/src/difficulty.rs` —— 难度参数表（数据来自 `/data/bots/difficulty.json`，初值 GAMEDESIGN §20.6，状态 `initial`）；反应延迟、瞄准误差倍率、记忆时长、视距、压制敏感度。
- `crates/bots/benches/bots_tick.rs` —— criterion：`bots_tick_60`、`bots_tick_100`（固定种子，全 bot 对局中段快照世界）。
- `data/bots/difficulty.json`、`data/schemas/bots_difficulty.schema.json` —— 数据与 schema（M0-004 的校验工具覆盖）。
### 要实现的行为
1. 感知缓存：每 tick 为每个 bot 只做"候选敌人 ≤ 32 个"的 LoS 检测；LoS 射线查询批处理并按距离优先；识别概率按距离衰减，结果与优化前统计一致（见验收 4）。
2. 思考分片与错峰：同一 tick 内任何 bot 的思考成本不得超过 120 µs（单次保护，超时则中断并沿用上一决策）。
3. 路径预算：队列按"距离上次重算时间"排序；`path_budget_ms_per_tick` 默认 1.0（配置项）；每 tick 输出实际耗时指标。
4. 难度可在服务器配置按队伍/按 bot 设置；默认 Regular。
5. 提供 `BotsMetrics`（tick 总耗时、感知/寻路/战术分段、延期数、卡住计数）供 server 的 `/metrics`（M0-015 框架）输出，命名 `bots_tick_ms`、`bots_path_ms`、`bots_perception_ms`。
### 接口与数据
```rust
pub struct BotScheduler { pub fn plan(&mut self, tick: u32, bots: &[BotId]) -> ThinkPlan }
pub struct PathBudget { pub ms_per_tick: f32, pub max_hz_per_bot: f32 }
pub struct PerceptionCache { pub fn rebuild(&mut self, view: &dyn WorldView); pub fn candidates(&self, bot: BotId, out: &mut SmallVec<[EntityId; 32]>) }
```
所有接口只依赖 `sim` 的 `WorldView` 与 `protocol` 类型（ARCH §4.5：bots → protocol/sim 允许，不得依赖 server/client）。

## 不做什么
- 不做载具驾驶/乘员 bot（M3-023）、Spotter/Sniper bot（M3-024）。
- 不做 LLM 指挥官（M3）；不改 RuleCommander 的决策逻辑（M2-028）。
- 不在 `server` 内实现任何 bot 或游戏规则；server 侧只接入指标（server 的 100 玩家整体优化见 M2-021）。

## 验收标准
1. `cargo bench -p bots bots_tick_60`：单 tick 中位数 ≤ 3.0 ms（参考机，ARCH §16.5/§6.10）。
2. `cargo bench -p bots bots_tick_100`：单 tick 中位数 ≤ 5.0 ms；P99 ≤ 8 ms。
3. 单个 bot 单次思考 ≤ 120 µs 的保护有单测（构造超时场景，断言沿用上一决策且指标 +1）。
4. 优化前后同种子 `balance-sim`/集成对局 5 分钟：各 bot 的"发现敌人→开火"延迟分布中位数偏差 ≤ 10%，且不存在对视线外目标开火（0 次，LoS 审计单测）。
5. 路径预算：构造 100 bot 同时换目标，单 tick 路径耗时 ≤ 1.0 ms（`bots_path_ms` 最大值），所有 bot 在 ≤ 2 s 内拿到路径。
6. 难度表加载单测：三档参数与 GAMEDESIGN §20.6 一致；`data-validate` 对缺字段 JSON 报错。
7. 卡住检测（B-21：5 s 位移 < 1 m）在 100 bot、10 min 模拟中触发率 ≤ 2 次/bot/10 min。

## 测试要求
- 单元测试：调度器分片覆盖（每个 bot 在 3 tick 内恰好被调度 1 次）；路径预算排队与复用；感知缓存与暴力全表扫描结果一致（属性测试，随机世界 1 000 例）。
- 集成/无头测试：`crates/bots/tests/scale_100.rs` 以 `sim` 世界跑 100 bot × 3 000 tick，无 panic、无非法输入（输入校验与真人一致）。
- 性能基准：`crates/bots/benches/bots_tick.rs`；基线 JSON 存 `reports/bench/bots_tick_baseline.json`。
- 回归：确定性——同 seed 两次运行 `state_hash` 逐 tick 相同。

## 性能预算
- ARCHITECTURE §16.5：60 bot ≈ 3.0 ms/tick（感知+思考约 1.0、路径 ≤ 1.0、战术/射击约 1.0）；本任务把 100 bot 控制在 ≤ 5 ms/tick（E2-4）。
- 不得使 ARCH §6.10 阶段 6 的 3.0 ms 预算在 60 bot 常态下回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test -p bots`、`cargo bench -p bots`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 参考机若不可用，benchmark 标 `未测`，任务至多 `partial`（README §5.5）。
- 难度档数值全部【待调参】；真正的"服务器内 100 bot + 100 玩家"整体 tick 在 M2-021/M2-049 验收，本任务只保证 bots 部分。
- 树篱/全图场景的卡死监控依赖 M2-005 的 off-mesh 标注；若发现导航缺口，写 PLAN_ISSUES，不在本任务改地图数据。
