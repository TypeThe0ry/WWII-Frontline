# M3-020 sim：Assault 模式规则

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：L
- 依赖：M2-008, M3-004
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §3.4; GAMEDESIGN §3.7; GAMEDESIGN §3.2; GAMEDESIGN §3.3.6; GAMEDESIGN §10.5; GAMEDESIGN §2.2; GAMEDESIGN §11.2

## 背景
M2-008 已实现 Frontline（5 条战线拉锯、占领速度公式、相邻锁定、Sector 推进、胜负/超时/加时）。Assault 是**非对称**逐段推进模式（GD §3.4）：进攻方（Attacker）从 L1 起逐段占领，防守方（Defender）持有 L2–L5；**仅"当前目标点"（next uncaptured line）活动**；阵营在每局开始前由地图配置（Carentan：US 攻；Omaha：US 攻；Foy：GER 攻），每局结束后若连续两局同图则互换。M3 用 Carentan 的 `assault_stages` 测试配置（M3-004）验证规则，正式地图是 Omaha（M4-004）。
规则要点：
- **每段目标**：进攻方占领当前线 Strongpoint（`p` 从 −100 推到 +100，进攻方净人数差为正，规则 GD §3.2）→ 该线归攻方，目标推进下一线；**加时 +8 min**（`capture_bonus`，总剩余上限 60 min）；防守方在该 Sector 的 Garrison 按 GD §3.3.6 销毁；防守方获得后撤 Rally 点（地图数据 `fallback_spawn`）作为额外 HQ 重生点，当前线之前的据点重生点失效。
- **计时**：初始 20 min（`initial_time`）；计时归 0 且攻方未占完 5 线 → 防守方胜；攻方占完 5 线 → 攻方胜。每阶段时间用尽时给 60 s 争夺窗口：若 `p > 0` 且仍在占领区内，则持续到该占领完成或回落到 0 才结束（最多再延长 90 s）【待调参】。
- **限制**：防守方不能在已被攻占的 Sector 建 Garrison；攻方不能在未占领 Sector 建 Garrison（Outpost/OP/Airhead 可）；攻方资源节点仅建在已攻占 Sector；进攻方起始资源 +25%；防守方每队 Garrison 上限 +2【待调参】；无平局。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/mode/assault.rs` —— `AssaultMode`：`stages[]`（来自 `zones.json` 的 `assault_stages`）、`current_stage`、`attacker/defender`、`timer`、争夺窗口；实现与 `FrontlineMode` 同一 `GameModeRules` trait（M2-008 定义）。
- `crates/sim/src/mode/mod.rs` —— `GameMode::Assault` 接入 `World::new`；模式切换由 `SimConfig.mode`（ARCHITECTURE §8.1）。
- `crates/sim/src/spawn/assault_spawns.rs` —— 攻守出生点规则：攻方 HQ 区（L1）；防守方 HQ + `fallback_spawn` 随阶段推进激活/旧据点重生点失效；Strongpoint 重生点规则复用 GD §10.5。
- `crates/sim/src/mode/assault_restrictions.rs` —— Garrison/Outpost/资源节点/OP/Airhead 放置限制与偏置（资源 +25%、守方 Garrison +2）。
- `crates/sim/tests/assault_mode.rs`、`crates/sim/tests/data/carentan_assault_script.json`（脚本化输入序列）。
### 要实现的行为
1. 阶段推进：`current_stage` 的 primary 全被攻占 → 推进下一阶段、`timer += 8 min`（上限 60 min 总剩余）、事件 `ObjectiveChange` 与 `MatchPhase` 更新。
2. 占领：仅当前阶段据点活动（其它据点锁定），占领速度公式沿用 GD §3.2（M2-008 实现）；攻方净人数差为正才推进。
3. 超时与争夺窗口：阶段时限或全局计时用尽进入 60 s 争夺窗口；`p > 0` 且攻方仍在区内可延长至最多 90 s；`p` 回落 0 即结束。
4. Sector 易手：按 GD §3.3.6 处理防守方 Garrison（`Condemned` 30 s 后销毁）、资源节点、Outpost 位置检查；防守方获得 `fallback_spawn`。
5. 胜负：攻方占完全部阶段（5 线）→ 攻方胜；全局计时归 0（含窗口）→ 守方胜；`MatchPhase{ended}` 事件。
6. 互换阵营：每局结束若连续两局同图则互换（由对局生命周期 M2-xxx/GD §19.7 调用 `swap_sides()`，本任务提供接口与配置读取）。
7. 资源偏置：攻方起始储备 ×1.25（M2-009 接口）。
### 接口与数据
- `AssaultConfig{initial_time_s:1200, capture_bonus_s:480, max_total_s:3600, contest_window_s:60, contest_max_extra_s:90, attacker_resource_mult:1.25, defender_garrison_bonus:2}` 来自 `/data/maps/*/map.json` 的 `modes.assault`（MAPS §6.2），默认值写入 `/data` 而非代码。

## 不做什么
- Skirmish 模式（M4）；Omaha 的登陆艇出生（M4-011）；Assault HUD → M3-036；Assault bot 行为 → M3-028；
- 平衡调参 → M3-047；Assault 正式关卡 → M4-004。

## 验收标准
1. 用 Carentan 测试配置**脚本化跑通 4 阶段**：`cargo test -p sim assault_mode::scripted_4_stages` 通过；阶段序列、加时次数（3 次）、最终胜者与脚本期望一致。
2. 阶段超时单测：阶段计时归 0 且 `p > 0` → 进入 60 s 窗口并最多再延长 90 s；`p` 回落 0 则立即结束（3 个分支各 1 个断言）。
3. 加时规则：每次占领 +480 s，总剩余上限 3 600 s（单测：连续 8 次不超过上限）。
4. 防守方 Garrison 在易手后 `Condemned` 30 s 销毁（±1 tick）；Outpost 距新守线中心 < 120 m 立即销毁；攻方在未占 Sector 建 Garrison 被拒（单测）。
5. 攻方起始资源 = 基准 ×1.25（Manpower 250/Munitions 100/Fuel 50，误差 0）；守方 Garrison 上限 +2。
6. 同一输入脚本两次运行 `state_hash` 逐 tick 相同；每 tick 规则开销 ≤ 0.1 ms（`criterion`）。
7. 无平局：构造超时与同时到达场景，`MatchPhase` 总有唯一胜者（单测）。

## 测试要求
- 单元测试：阶段推进、窗口、加时、放置限制、资源偏置、阵营互换接口。
- 集成/无头：`headless-client` + 脚本化 bot 攻方完成 4 阶段（真实网络、WS/WT 任一），事件序列与期望一致。
- 性能基准：`crates/sim/benches/assault_rules.rs`。
- 回归：确定性哈希更新（含模式状态）。

## 性能预算
ARCHITECTURE §6.10 阶段 5 规则 0.7 ms/tick：Assault 规则 ≤ 0.1 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；争夺窗口与加时的交互 GD 描述较简略，歧义写 PLAN_ISSUES（例如"阶段时间"与"全局计时"的关系：本任务取"全局计时 20 min + 每段 +8 min"，阶段不单设时限，`time_limit_s` 字段仅 M3-004 配置里保留不用）。
- GD §3.4 写"同样 5 条线 L1–L5"，而 M3-004 的测试配置只有 4 个阶段（INDEX/MILESTONES 称"4 阶段"）；以 `assault_stages` 条数为准，不硬编码 5。
- 依赖 M1-019 的 Garrison/Outpost 规则；若 Outpost（M2-010/011）接口不全，用 trait 桩。
