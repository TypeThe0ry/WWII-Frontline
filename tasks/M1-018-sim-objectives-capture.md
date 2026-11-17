# M1-018 sim：据点占领与对局状态机

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：M
- 前置依赖：M1-007, M0-007
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §3.2; GAMEDESIGN §3.6; GAMEDESIGN §2.1; GAMEDESIGN §2.2; GAMEDESIGN §19.7; MAPS §3.8; ARCHITECTURE §8.1

## 背景
M1 有 **3 个活动据点：L1a（Ingouf Farm）、L2a（Market-Garden Fields）、L3a（Notre-Dame）**（MAPS §3.8），半径 45/55/50 m（MAPS §3.6，【待调参】）。本任务实现通用占领公式与对局状态机，并做 **M1 简化胜负**（完整 5 线 Frontline 的活动线/锁点/推进规则在 M2）。
占领公式（GD §3.2）：每 tick 统计半径 R_sp 内的有效人数 `n_A`、`n_B`——计入条件：存活、非倒地、非载具内（装甲载具乘员、运兵载具乘客、驾驶员均不计）、水平距离 ≤ R_sp 且高度差 ≤ 6 m。`d = n_A − n_B`，`d=0` 僵持；`eff(d) = min(|d|,4)×1.0 + max(0, min(|d|,10)−4)×0.5`；`rate = k_cap × eff(d) × sign(d)`（`k_cap = 0.20 %/s`【待调参 / 待测 → REFERENCE_MEASUREMENTS §capture】）；`p += rate×Δt`，`p ∈ [−100,+100]`；`p=±100` 即所有权变更；无人时 `|p|<100` 以 `0.15 %/s` 向 `last_owner` 回弹（从未被占的中立点不回弹）。示例：净优势 10 人 → eff 7、rate 1.4 %/s、−100→+100 需 ≈ 143 s；净优势 1 → ≈ 1000 s。
对局状态机（GD §2.2/§19.7）：`Idle → Warmup(60 s；玩家部署、不可占点；重生计时 ×0.5；≥2 名真人或 60 s 到期开局) → Live → Ended(结算 30 s)`；`MatchPhase{warmup,live,overtime,ended, ends_at}` 对应 M1-010 的 C4 事件。
**M1 简化规则（INDEX "M1 简化胜负"，本任务细化，需 PLAN_ISSUES 登记）**：3 点一字排开（US 侧 L1a、中 L2a、GER 侧 L3a）；开局 L1a 属 US（p=+100）、L3a 属 GER（p=−100）、L2a 中立；**三点同时可争夺，无锁点**；一方持有全部 3 点 → 立即获胜（`full_control`）；时限默认 20 min（`/data` 配置，MILESTONES E1-1 要求 ≥ 20 min 对局），到时比较持点数，再比较各点 `p` 之和，仍相同为平局。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/rules/objectives.rs` —— `Objective{id, pos, radius_m, p, owner, last_owner, locked}`、`step_objectives(world)`
- `crates/sim/src/rules/capture.rs` —— `eff()`、`capture_rate()`、有效人数统计（`WorldView` 的玩家空间查询）
- `crates/sim/src/rules/match_state.rs` —— `MatchPhase` 状态机、计时、胜负判定（`VictoryReason::{FullControl, TimeLimitLead, Draw}`）
- `crates/sim/src/rules/mod.rs` —— `RuleChange::{ObjectiveOwner, MatchPhase}` 输出到 `StepOutput.rule_changes`（ARCHITECTURE §8.1）
- `data/rules/capture.json`（M1-007 已建；本任务只读）、`data/rules/match_m1.json`（warmup_s 60、time_limit_s 1200、win 规则）
- `crates/sim/tests/capture_*.rs`、`match_state_*.rs`
### 要实现的行为
1. 从地图 `zones.json`（M1-034 的切片文件）读取据点；M1 单元测试内用内联夹具，不依赖地图管线。
2. 有效人数：每 tick 对每个据点遍历半径内玩家（空间网格加速），按条件过滤；O(据点×半径内玩家)。
3. 占领推进：公式如上；到 ±100 的一刻发 `StrongpointCaptured{objective, new_owner, reason}`（对应 C4 `ObjectiveChange`）并更新 `last_owner`。
4. 回弹：两队均无人且 `|p|<100` 且点有 `last_owner` 时以 0.15 %/s 回归；`last_owner` 的符号方向。
5. 状态机：`Warmup` 阶段占点逻辑暂停（`locked=true`）；开局事件 `MatchPhase{live, ends_at}`；胜负判定每 tick 检查；`Ended` 后冻结移动输入 3 s、30 s 后进入 `SCOREBOARD`（UI 层）。
6. 反刷分（预留）：同一据点 90 s 内反复易手只发事件标 `repeat=true`（计分在 M2）。
7. 输出 `WorldView::objectives()`（`ObjectiveView{id,pos,radius,p,owner,contested}`）供 AOI/UI/bots。
### 接口与数据
```rust
pub fn eff(d_abs: u32) -> f32;                      // min(d,4) + 0.5*max(0,min(d,10)-4)
pub fn step_objectives(w:&mut World, dt:f32) -> SmallVec<[RuleChange;4]>;
pub enum MatchPhaseKind { Idle, Warmup, Live, Overtime, Ended }
```

## 不做什么
- 5 线 Frontline 的活动线/锁点/Sector/战线切换处理（GD §3.3）：M2；Assault、Skirmish：M3/M4；
- 资源产出（Manpower/Munitions/Fuel）：M2；
- 得分/结算页/投票：M2；
- 据点重生点启用/封锁（据点内重生点）：M1-019 使用本任务的 `owner/p`；
- 客户端地图与 HUD 进度条：M1-057/M1-058。

## 验收标准
1. 5v5 占领速度与公式误差 < 1%：净优势 5 人 → eff=4.5、rate=0.90 %/s；从 p=0 推到 +100 的 tick 数与 `100/0.9×30 = 3333` ±1% 一致（`cargo test capture_5v5`）。
2. 净优势 10 人从 −100 到 +100 ≈ 143 s（±1%）；净优势 1 人 ≈ 1000 s（±1%）；净优势 12 人速度 = 10 人（上限）（单测）。
3. 僵持：`n_A==n_B` 时 p 不变（200 tick）；一队全员倒地/在装甲载具内时不计入（单测 3 例）。
4. 高度差 6 m 边界：5.9 m 计入、6.1 m 不计入（单测）。
5. 回弹：p=60、`last_owner=A`、无人 ⇒ 每秒 +0.15（±2%）；p=60、`last_owner=none` ⇒ 不变。
6. 状态机：热身 60 s 或 ≥2 名真人提前开局，热身期间 p 不变；全占 3 点立即 `Ended(FullControl)`；时限到按持点数判定，平点比 p 之和（4 例单测，时限可用测试配置缩短）。
7. 事件：`StrongpointCaptured` 在 |p|=100 的**同一 tick** 发出且仅一次（单测）；确定性哈希两次运行一致。
8. 性能：3 据点 × 100 玩家的占领阶段 ≤ 0.05 ms/tick（`cargo bench -p sim objectives`）。

## 测试要求
- 单元测试：`eff` 全取值、人数过滤、回弹、状态机转移、胜负优先级。
- 集成：`headless-client` 5v5 占领 L2a，实测占点时间写入 STATUS。
- 性能基准：criterion `objectives`。
- 回归：确定性哈希基线更新。

## 性能预算
- 并入 ARCHITECTURE §6.10 阶段 5（规则 ≤ 0.7 ms/tick）；本任务 ≤ 0.05 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- M1 的"3 点无锁、全占即胜"是对 GD §3.3（5 线+锁点）的**简化**，不在文档原文中，属细化，需写 PLAN_ISSUES；M2 引入活动线逻辑时必须兼容本任务的 `locked` 字段。
- `k_cap` 待对标测量（REF §capture，H-053 的 R-01）；`/data` 值为 `initial`。
- 据点所属 Sector 与 Garrison 成本（己方/敌方 50/100）在 M1 简化为"按据点所有者判断"，由 M1-019 读取 `owner`。
