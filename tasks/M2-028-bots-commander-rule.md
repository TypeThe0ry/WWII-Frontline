# M2-028 bots：规则式指挥官

## 元数据
- 里程碑：M2
- 工作流：bots
- 规模：L
- 前置依赖：M2-027, M2-009, M2-010, M2-015, M2-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §20.4; ARCHITECTURE §16.1; ARCHITECTURE §16.4; GAMEDESIGN §11; GAMEDESIGN §16.3; GAMEDESIGN §14.6

## 背景
每队 1 个 Commander bot（若无真人 Commander，GD §20.1）。M2 里程碑**没有 Commander 席位与能力**（MILESTONES §3.2 范围外，M3），所以 `RuleCommander` 在 M2 的有效动作是：资源分配（载具刷新）、给小队下达任务（Squad/Command Order）、发布 Garrison/Supply Request 标记、设定据点优先级；能力类动作（Supply Drop/Ammo Drop/Bombing/Strafing/Precision/Recon/Rally/Airhead）在 `CommanderAction` 枚举里存在，但 M2 校验层一律返回 `NotAvailable`（M3 打开）。它同时是 LLM 指挥官（M3）的**回退实现**：ARCH §16.4 的接口：

```rust
pub trait CommanderBrain {
    fn decide(&mut self, situation: &Situation, budget: &mut CostBudget) -> Result<Vec<CommanderAction>, BrainError>;
}
```

`Situation`（≤ 2 000 token 的结构化战况，**仅含该队可见信息**，不得泄露敌方未被发现的位置）：`phase`、`time_left`、`strongpoints[{id,owner,contested,my_strength_nearby,enemy_est}]`、`front_graph`、`resources{manpower,munitions,fuel}`、`ability_cooldowns`、`squads[{id,role_mix,pos_region,state,objective}]`、`recent_events`。输出动作 ≤ 6 条/决策，经与真人 Commander **同等**的校验层（资源、冷却、权限、标记配额）。
GD §20.4 要求：C-01 每 15 s 评估战况（活动线、人数优势、资源），优先级 占点 > 建造 Garrison > 支援火力 > 载具；C-02 资源规则：保留 ≥ 100 Manpower 作 Airhead 预算，Fuel ≥ 120 才呼叫重/中型载具，Munitions ≥ 350 且前线有聚集目标才用 Bombing，其余同理；C-03 能力时机（M3 才有能力，规则先写进策略表）；C-04 在活动线放 Attack/Defend 标记，Garrison Request 在前线适当位置；C-05 载具呼叫取决于队内是否有 Armor Crew 与 bot 数量；C-06 所有指令频率与真人相同（冷却、资源、全局锁）。**注意**：ARCH §16.1 写"规则式 5 s 级"，GD C-01 写"每 15 s 评估"——两者节奏不一致，按 GD 的 15 s 评估 + 5 s 执行队列实现，登记 PLAN_ISSUES。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/commander/mod.rs` —— `CommanderBrain` trait、`CommanderAction` 枚举、`Situation` 构造器（只读 `WorldView`，按"该队可见"过滤）
- `crates/bots/src/commander/rule.rs` —— `RuleCommander`：优先级评估、资源规则、动作生成
- `crates/bots/src/commander/validate.rs` —— 调用 sim 暴露的命令校验（`sim::command::validate(team, &CommanderAction)`），**不在 bots 内复制规则**；若 sim 尚无该接口则在 `crates/sim/src/command.rs` 新增，真人 Commander（M3）与 bot 共用
- `crates/bots/src/commander/exec.rs` —— 把合法动作转成与真人同构的命令（Squad Order/Command Order/Garrison Request/Supply Request/载具刷新请求→M2-015 的 `Requester::RuleCommander`）
- `data/bots/commander.json` —— 阈值（reserve_manpower、fuel 门槛、评估间隔 15 s、最多 6 动作等；【待调参】）
- `crates/bots/tests/commander_*.rs`
### 要实现的行为
1. 每 15 s（450 tick）评估：确定当前活动线与"最需要的据点"（进攻/防守）；为每个步兵小队分配目标（`assign_squad_objective`），均衡兵力（每据点至少 1 个小队）。
2. 标记：活动线据点 `Attack`/`Defend` Command Order（≤ 12，GD §16.3）；前线 Garrison Request（≤ 3）；Engineer 缺补给时 Supply Request（≤ 3）；自动撤销过期标记。
3. 资源：若 Fuel ≥ 门槛且 Manpower ≥ 成本 + 保留量，则请求 Jeep/运输车（M2 可用）与 Pz IV/Sherman（需 Armor Crew 在队，C-05），遵守载具上限与冷却（M2-015 校验）；Fuel=0 不请求。
4. 非法动作：校验层拒绝并记 `commander_rejected_total{reason}`；`RuleCommander` 自身不发明动作（只从合法候选集合选取），但**对抗性测试**会直接注入非法动作验证校验层。
5. 与人类共存：真人 Commander 就位（M3）时 bot 立即退位（接口 `yield_to_human()`）。
6. 决策日志：每次决策记录 `situation_hash`、动作、被拒原因（便于 M2-051 平衡分析）。
### 接口与数据
- `CommanderAction{ AssignSquadObjective{squad,point}, SetPriority{point,level}, PlaceMark{kind,pos}, RequestVehicle{kind}, UseAbility{...}/* M3 */ }`；`BrainError{ Timeout, Invalid, Budget }`（为 LLM 回退预留）。

## 不做什么
- LLM 指挥官、成本预算器实际计费 → M3（`CostBudget` 此处为空实现）；任何能力类动作的执行 → M3。
- 小队长/士兵的战术行为 → M2-027；100 bot 性能 → M2-029；平衡模拟 → M2-051。
- Commander UI/面板 → M3。

## 验收标准
1. 单局（20 min，50v50 全 bot，固定种子）内每队指挥官产生 ≥ 10 条**合法**指令（`commander_accepted_total ≥ 10`，含标记/小队任务/载具请求，各队分别统计）。
2. 非法指令被校验层拒绝（0 漏过）：对抗性测试向校验层注入 ≥ 1 000 个随机/边界非法动作（资源不足、冷却中、超配额、M3 能力、对方队伍的小队、不存在的据点、非 Commander 权限），`accepted_illegal = 0`（`cargo test -p bots commander_adversarial`，固定种子，proptest）。
3. 资源规则：Fuel < 120 时不请求中型载具；Manpower 低于 成本+保留量 不请求；Fuel=0 时不请求任何载具（单测）。
4. 信息隔离：`Situation` 构造不包含敌方未被发现的位置（用"毒药"WorldView 测试：访问敌方隐藏实体即失败）。
5. 评估间隔：相邻两次评估间隔 = 450 ± 1 tick；每次决策动作数 ≤ 6。
6. 决策耗时：`decide()` 单次 ≤ 1 ms（`cargo bench -p bots rule_commander`，100 玩家 Situation），不在 sim 线程每 tick 执行（按 15 s 周期）。
7. 同一种子运行两次，指挥官动作序列逐条相同（确定性）。

## 测试要求
- 单元测试：优先级排序、资源规则表驱动、配额拦截、过期撤销。
- 集成/无头：headless 局中 RuleCommander 驱动 bot 小队，截取 `commander_*` 指标；对真实 server 的 C0 命令路径做 1 局烟测。
- 性能基准：`rule_commander`。
- 回归：M2-027 的场景测试保持通过。

## 性能预算
ARCH §16.1：规则式指挥官不占逐 tick 预算（15 s 级）；ARCH §6.10 阶段 6「Bot 思考 3.0 ms」不得因指挥官增加 > 0.05 ms 的摊销。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- M2 无 Commander 能力，"≥ 10 条合法指令"只能靠标记/任务/载具请求；Squad Order/Command Order 的 sim 侧校验由 M2-019 提供，载具请求由 M2-015 提供，本任务依赖它们（依赖已补 M2-010/M2-015/M2-019）。
- C-02 的"保留 ≥ 100 Manpower 作 Airhead 预算"在 M2 无 Airhead，默认仍按 GD 保留，参数在 `commander.json`，M2-051 观察是否造成资源闲置。
- 评估间隔 15 s（GD C-01）与 ARCH §16.1 的 5 s 级不一致 → PLAN_ISSUES。
