# M3-017 sim：载具呼叫与认领

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 依赖：M3-006, M2-015, M3-016
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §13.9; GAMEDESIGN §14.6; GAMEDESIGN §14.6.1; GAMEDESIGN §11.4; GAMEDESIGN §14.1

## 背景
M2-015 已实现载具刷新点与资源成本/上限的基础；M3-006 提供能力通用流程；M3-012 提供 Tank Commander 认领。本任务把"载具呼叫"实现为第 9 项能力（id=8）：Commander 在面板选择类型 → 3 s 后在 HQ 载具刷新点出现 → 60 s 内被小队认领，否则回收并全额返还资源。成本、上限、冷却、人数需求全来自 `/data/vehicles/*.spawn`（GD §14.6.1），例如：
- Jeep/Kübel：M 20/F 15，全队 ≤ 4，被毁冷却 60 s；CCKW/Blitz：M 40/F 30，≤ 3，90 s；半履带/251：M 70/F 50，≤ 2，120 s。
- 轻型装甲（M8/Puma/Stuart/Luchs）：M 90–100/U 40–50/F 70–80，≤ 2 且轻型合计 ≤ 3，150–180 s，Armor Crew ≥ 2 人。
- 中型（M4A1/Pz IV H）：M 140/U 80/F 120，中型合计 ≤ 3，240 s，解锁门槛 5 min；M4A3(76)W：M 160/U 100/F 140，270 s，10 min。
- 重型（Panther M190/U110/F160，330 s，15 min；Jumbo M200/U120/F180，360 s，20 min；Tiger M240/U140/F200，420 s，25 min）：重型合计 ≤ 1，需 3 人。
- 全队装甲总数 ≤ 6（轻+中+重）、软车辆 ≤ 8；刷新点：每队 HQ 轻 2、中 2、重 1、运输 3（地图数据）；追赶机制：Frontline 中己方持 ≤ 1 线且对方 ≥ 4 线时立即解锁一次重型【待调参】；无人使用且离队 120 s 自动回收返还 50%。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/spawn.rs`（M2-015 已有）—— 扩展 `VehicleSpawnPoint` 占用状态、类型槽（轻/中/重/运输）、冷却表 `death_cooldown[type]`。
- `crates/sim/src/commander/effects/vehicle_call.rs` —— `VehicleCallEffect`（实现 `AbilityEffect`；延迟 3 s）：校验类型解锁时间、上限、冷却、资源、Armor Crew 需求（需存在满足人数的 Armor Crew 方可呼叫重/中/轻装甲？——**GD 规定认领时校验人数；呼叫时只校验资源/上限/解锁/冷却**，见备注）；出现位置选择 HQ 内空闲同类槽。
- `crates/sim/src/vehicle/claim.rs`（M3-012 已创建）—— 60 s 认领超时回收（全额返还）；离队 120 s 回收（返还 50%）；击毁不返还。
- `crates/sim/src/vehicle/upkeep.rs` —— 燃料维持（轻 1、中 2、重 3 /min）与 Fuel=0 的 ×0.6 速度与禁刷（若 M3-015 已建则复用）。
- `crates/sim/tests/vehicle_call.rs`。
### 要实现的行为
1. 呼叫：扣资源（一次性）→ 3 s 后出现；冲突处理：目标槽被占用则拒绝（`SlotBusy`），同时刻两个呼叫选同槽时按 `issue_tick` 先到先得，另一方返还并报错（确定性）。
2. 上限：同类上限 + 类别合计上限 + 全队装甲 ≤ 6 + 软车辆 ≤ 8；超限拒绝，不扣资源。
3. 时间门槛：`match_time_s ≥ unlock_min`；追赶机制按 Frontline 线数条件一次性解锁重型（只在 Frontline 模式）。
4. 认领超时：60 s 无人认领 → 回收并返还全部资源；已认领后不再回收；认领者小队人数不足需求时拒绝认领（M3-012）。
5. 冷却：同类被毁后冷却（60–420 s）内不可再呼叫；击毁不返还资源。
6. 回收：载具无人使用且离队 120 s → 回收返还 50%。
7. 事件：`AbilityExecuted(VehicleCall)`、`VehicleSpawned{type, slot}`、`VehicleRecycled{reason}`；全部可靠事件（C4）。
### 接口与数据
- `VehicleCallRequest{vehicle_type}`（M3-005）；`SpawnSlot{kind, pos, facing, occupant}` 来自地图 `spawns.json`（M2-004/M3-004）。

## 不做什么
- 载具物理/战斗 → M3-013–015；座位 → M3-016；UI 呼叫面板 → M3-033/M3-035；bot 呼叫策略 → M3-026；
- Skirmish 固定刷新（M4）；Assault 后撤 HQ 的额外载具位（M3-020 提供 HQ 切换，本任务读取当前 HQ 列表即可）。

## 验收标准
1. 成本/上限/冷却单测：对 GD §14.6.1 表全部 10 类共 ≥ 30 个断言（成本扣除精确到 1 单位、超限拒绝不扣资源）。
2. 3 s 出现延迟（±1 tick）；60 s 认领超时回收返还 100%（资源恢复精确）；120 s 离队回收返还 50%。
3. 同槽并发呼叫：按 tick 序先到先得，后者拒绝且不扣资源；多次重复运行结果一致（`state_hash`）。
4. 时间门槛：Panther 在 14:59 拒绝、15:00 通过（边界单测）；追赶机制在"己方持 1 线、对方持 4 线"时触发一次且不可重复。
5. 全队装甲总数达 6 后，第 7 辆任何类型装甲被拒；软车辆第 9 辆被拒。
6. 燃料维持与 Fuel=0 速度 ×0.6、拒绝新呼叫（单测）。
7. 呼叫单次处理 ≤ 0.03 ms（`criterion`）。

## 测试要求
- 单元测试：表驱动全部载具类型；认领/回收时序；Frontline 追赶机制。
- 集成/无头：`headless-client` 作 Commander 呼叫 Jeep、Sherman，作 TC 认领，随后驾驶 100 m；非法呼叫（无 Commander、资源不足）100% 被拒。
- 性能基准：`crates/sim/benches/vehicle_call.rs`（可选，若 ≤ 0.03 ms 可只做单测断言）。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 5（规则 0.7 ms）与阶段 3（载具 2.0 ms）：呼叫触发时 ≤ 0.03 ms，常态 0。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §14.6 说"Armor Crew 需求：认领者所在小队当前人数 ≥ 需求"，而 §13.9 说 Commander 呼叫后 60 s 内无人认领回收；本任务因此在**认领**而非呼叫时校验人数，若 Commander 在没有 Armor Crew 时呼叫坦克，资源会在 60 s 后返还（请 Fable 确认意图，写 PLAN_ISSUES）。
- 呼叫"M8/Puma 成本区间 90–100"取值沿用 M3-002 的数据，不在代码中判断。
- GD 14.6.1 的"Skirmish 固定刷新"与本任务无关。
