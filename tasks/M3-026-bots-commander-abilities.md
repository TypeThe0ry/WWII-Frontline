# M3-026 bots：规则指挥官使用全部能力

## 元数据
- 里程碑：M3
- 工作流：bots
- 规模：L
- 前置依赖：M2-028, M3-008, M3-009, M3-007, M3-017, M3-018
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §20.4; GAMEDESIGN §13; GAMEDESIGN §14.6; GAMEDESIGN §15; ARCHITECTURE §16.4; ARCHITECTURE §16.1

## 背景
M2-028 的 `RuleCommander` 已有资源分配、小队任务、载具呼叫、优先据点，并产出与真人同构的指令（单局 ≥ 10 条合法指令；非法指令被校验层拒绝，0 漏过）。M3 的能力集扩大到 8 项 + 载具呼叫 + 炮兵 Fire Mission。本任务让 `RuleCommander` 使用**全部**能力，并保持同一校验路径（M3-006）。
GD §20.4（C-01–C-06）要求：
- C-01 每 15 s 评估战况（当前活动线、人数优势、资源）；优先级：占点 > 建造 Garrison > 支援火力 > 载具。
- C-02 资源规则：保留 ≥ 100 Manpower 作为 Airhead 预算；Fuel ≥ 120 才呼叫重型/中型载具；Munitions ≥ 350 且前线存在聚集目标时才用 Bombing；其余同理。
- C-03 能力使用时机：Supply Drop 在前线 Engineer 补给不足时；Ammo Drop 在前线小队持续战斗 ≥ 120 s；Recon Flight 在进攻前 60 s；Rally 在进攻小队接敌前 5 s；Airhead 在敌方切入后方或己方重生点被封锁时。
- C-04 在活动线放置 Attack/Defend 标记，Garrison Request 在前线适当位置。
- C-05 载具呼叫：根据队内是否有 Armor Crew 与 bot 数量决定。
- C-06 所有指令频率限制与真人相同（冷却、资源、全局锁）。
能力数值见 `/data/abilities`（M3-003）：Bombing 350/120、Strafing 150/60、Precision 200/80、Recon Fuel 60、Rally M120、Airhead M200、Supply Drop M100+F20、Ammo Drop U120。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/commander/rule.rs`（M2-028 已有）—— 扩展 `RuleCommander::decide()`：增加能力选择器列表。
- `crates/bots/src/commander/ability_utility.rs` —— 每项能力一个 `AbilityScorer`：`score(situation) -> Option<(pos, dir, urgency)>`，含落点/方向选择：
  - Bombing：已知敌人聚集簇（本队可见/被标记，不得全知）中心，条带沿簇主轴；要求簇内敌人数 ≥ 6 且友军距离 > 25 m。
  - Strafing：沿敌步兵进攻/集结路线；Precision：已被标记的高价值目标（载具/火炮/AT 炮/Garrison）。
  - Recon：下一活动据点方向，进攻前 60 s；Rally：接敌前 5 s 的进攻小队；Airhead：后方被切入或重生点封锁。
  - Supply/Ammo Drop：前线补给/弹药状态；位置满足合法性（M3-006 规则）。
- `crates/bots/src/commander/vehicle_policy.rs` —— 载具呼叫策略：判定有无 Armor Crew（真人或 bot）、解锁时间门槛、上限；呼叫优先级 运输 > 轻型 > 中型 > 重型（受 Fuel ≥ 120 约束）。
- `crates/bots/src/commander/artillery_orders.rs` —— 炮兵目标：采纳 Officer 的 Fire Request（GD §16.3）或自选 Fire Mission（已标记目标）；每炮同时 1 个任务。
- `crates/bots/src/commander/situation.rs` —— 构建 `Situation`（仅本队可见信息，供 M3-027 LLM 复用）。
- `crates/bots/src/commander/validator_hook.rs` —— 所有指令经 M3-006 `validate_order`；被拒指令计数 `rule_cmd_rejected_total`，且不重复提交同一非法指令（10 s 抑制）。
- `crates/bots/tests/commander_abilities.rs`、`crates/balance-sim/src/scenarios/commander_full.rs`。
### 要实现的行为
1. 每 15 s 评估；每次最多下达 2 个能力指令（遵守全局空中锁/上限）；失败后退避。
2. 全部 8 种能力在一局内至少各使用 1 次（受战况条件约束；`commander_full` 场景构造触发条件，如强制敌人聚集与己方重生点被封锁）。
3. 保留预算：Manpower ≥ 100 保留给 Airhead（C-02）；资源不足的能力不尝试。
4. 非法指令：规则层应尽量不产生；若产生，被校验层 100% 拒绝，`illegal_cmd_leaked_total` = 0。
5. 同一份 `Situation` 构建代码供 LLM 指挥官使用（M3-027）：包含 `phase/time_left/strongpoints/front_graph/resources/ability_cooldowns/squads/recent_events`，不含敌方未被发现的位置。
### 接口与数据
- `trait CommanderBrain { fn decide(&mut self, situation:&Situation, budget:&mut CostBudget) -> Result<Vec<CommanderAction>, BrainError> }`（ARCHITECTURE §16.4）；`RuleCommander` 实现之（`CostBudget` 对规则指挥官为 no-op）。
- `CommanderAction::{UseAbility{ability,pos,dir_deg,target}, AssignSquadObjective{squad,objective}, RequestSupply{..}, SetPriority{..}, CallVehicle{type}, FireMission{gun,pos}}`，经 `bots` 内转换为与真人同构的 `CommanderOrder`。

## 不做什么
- LLM 指挥官 → M3-027；Assault 模式阶段策略 → M3-028；能力 sim 逻辑 → M3-006–M3-018；
- Commander UI；bot 士兵层。

## 验收标准
1. **一局内全部 8 种能力至少各被使用 1 次**：`balance-sim commander_full` 5 局（Carentan Frontline，90 min 内），每局统计各能力 `ability_used_total{ability}` ≥ 1（报告 `reports/m3-commander-bot.json`；若少数能力因战况不触发，需 5 局累计各 ≥ 1 且每局至少 6 种）。
2. **0 条非法指令漏过校验**：全部局 `illegal_cmd_leaked_total` = 0；被拒指令占比 ≤ 15%（`rule_cmd_rejected_total / issued_total`）。
3. 资源保留：Airhead 预算规则单测——Manpower < 100+cost 时其它 Manpower 能力不下达。
4. 载具呼叫：无 Armor Crew 时不呼叫坦克（单测）；Fuel < 120 时不呼叫中/重型（单测）。
5. 空中支援间隔 ≥ 10 s、同时在空飞机 ≤ 3（审计 0 违规）。
6. 决策开销：每次 `decide()` ≤ 1.0 ms（`criterion`，100 bot 场景），每 15 s 一次，均摊可忽略（ARCHITECTURE §16.5）。
7. `Situation` 信息审计：随机 100 个快照中，含敌方未暴露单位的比例 = 0。

## 测试要求
- 单元测试：各 `AbilityScorer` 的触发条件与落点选择（构造情景）；资源保留；退避。
- 集成/无头：`balance-sim commander_full` ≥ 5 局；服务器 + `headless-client` 观察 bot Commander 的指令序列。
- 性能基准：`crates/bots/benches/commander_decide.rs`。
- 回归：bot 行为轨迹哈希（信息性）。

## 性能预算
ARCHITECTURE §16.5 总 bot 预算 ≈ 3.0 ms/tick；指挥官决策 ≤ 1.0 ms/次、15 s 一次。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L，接近上限；若超限，把载具/炮兵策略拆为 `M3-026b`。
- 8 种能力"一局内各至少一次"取决于战况，Frontline 90 分钟理论可行；Bombing 需 Munitions ≥ 350（起始储备 80，需累积），验收允许 5 局累计，登记于 STATUS。
- INDEX 把 Fire Mission 目标选择归本任务，M3-025 只负责炮组响应；两者接口为 `FireMission` 协议消息。
