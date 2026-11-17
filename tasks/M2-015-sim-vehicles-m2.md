# M2-015 sim：M2 载具（轻型车、卡车、Panzer IV）与刷新

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：L
- 前置依赖：M1-023, M2-006, M2-009
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §14.3; GAMEDESIGN §14.6; GAMEDESIGN §14.9; GAMEDESIGN §14.1; GAMEDESIGN §7.6; GAMEDESIGN §11.4; ARCHITECTURE §6.10

## 背景
M1-022/M1-023 实现了履带车辆物理（raycast 悬挂 + 履带）与战斗；M1 只有 Sherman。M2 新增：Jeep/Kübelwagen（轮式，MG 版）、CCKW/Blitz 卡车（运输版/补给版）、Panzer IV H（履带，复用 M1），并建立**刷新系统**与资源成本。
数值（GD §14.3/§14.6，游戏最高速 = 史实路速 ×0.75）：Jeep 100→75 km/h、Kübel 80→60、CCKW 72→54、Blitz 80→60、Pz IV H 38→28.5。座位：Jeep/Kübel 驾驶 + 副驾（MG 版为机枪手）+ 后座 ×2 = 4；卡车 驾驶 + 副驾 + 后厢 ×10 = 12（封闭车厢乘客不可射击，副驾可持手枪射击）；Pz IV 3 座（驾驶/炮手/车长）。补给版卡车车载 600 Supplies（Engineer/Support 8 m 内取用，装卸 1 箱 2 s，卡车被毁补给丢失）。
成本/上限/冷却（GD §14.6.1）：Jeep/Kübel M20/F15、全队 ≤ 4、被毁冷却 60 s；CCKW/Blitz M40/F30、≤ 3、90 s；Pz IV H M140/U80/F120、中型合计 ≤ 3、240 s、解锁时间门槛 5 min、Armor Crew ≥ 2 人。全队装甲总数 ≤ 6，软车辆 ≤ 8。刷新点：每队 HQ 载具位 轻 2、中 2、重 1、运输 3（地图数据，M2-004）；选定类型后 3 s 出现；认领超时 60 s 回收；无人使用且离队 120 s 自动回收返还 50%。Fuel 维持见 M2-009（中型 2/min）。
交互（GD §14.1/§14.9/§7.6）：离车按住 0.8 s，车速 > 20 km/h 离车受 30 HP；座位切换停车或 < 5 km/h 立即，行驶中 2 s；载具碾压：车速 ≥ 15 km/h 碰撞 KIA、5–15 km/h 70 HP、< 5 km/h 无伤；撞建筑/树/障碍车速 > 25 km/h 时 `hull_hp −= 0.5·v²`（v m/s）【待调参】；软车辆击穿按软目标处理（伤害 = 武器伤害 ×0.4 作用于 `hull_hp`，乘客按人体命中盒）；乘员互相不受同车枪弹/炮弹伤害。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/wheeled.rs` —— 轮式悬挂/转向/驱动（复用 M1-022 的射线悬挂接口，增加阿克曼转向、4 轮独立射线）
- `crates/sim/src/vehicle/seats.rs` —— 座位类型、乘客射击扇区（开放车 90°）、进出/切座规则
- `crates/sim/src/vehicle/spawn.rs` —— `VehicleSpawner`：刷新位、请求→资源扣除→出现→认领→回收；上限/冷却/门槛/Armor Crew 需求
- `crates/sim/src/vehicle/crush.rs` —— 碾压与碰撞伤害
- `crates/sim/src/vehicle/cargo.rs` —— 卡车车载 Supplies 与装卸（对接 M1-020）
- `data/vehicles/*.json` 增补（M2-006 已建）：`spawn.{slot_class, claim_timeout_s:60, recycle_after_s:120, refund_frac:0.5}`、`time_gate_s`
### 要实现的行为
1. `request_spawn(team, kind, requester) -> Result<VehicleId,SpawnError>`：校验上限（类别合计与全队装甲 ≤ 6/软 ≤ 8）、冷却、时间门槛（5 min = 9 000 tick）、HQ 载具位空闲、`Fuel>0`，调用 M2-009 `try_spend`；3 s 后出现。**授权**：M2 无 Commander 席位（M3），请求者仅限 RuleCommander bot（M2-028）；人类入口见备注。
2. 认领：Armor Crew 小队成员靠近交互；运输车任意玩家可上；中型坦克认领需 Armor Crew 当前人数 ≥ 2；已认领后乘员离队不影响载具。
3. 回收：认领超时 60 s、无人使用且离队 120 s 回收，返还 50% 资源；被毁不返还，冷却开始。
4. 轮式车辆物理：路速与 /data（史实 ×0.75）误差 < 3%；爬坡上限 25°（轮式）/30°（履带）；洪泛区不可通行（用 M2-004 的 `vehicle_routes`/材质标记）。
5. 碾压/碰撞/离车伤害按上文规则；`SimEvent::RunOver`（击杀归属给驾驶员）。
6. 补给卡车：车载 600 Supplies，交给 M1-020 的 Supplies 抽取接口；卡车被毁清零。
### 接口与数据
- `VehicleSpawnReq{ team, kind:VehicleId, requester:Requester }`；`enum Requester{ RuleCommander(Team), CrewClaim(PlayerId) }`；`SpawnError{ NoSlot, Cap, Cooldown, TimeGate, Resources, LowFuel, CrewShort }`。

## 不做什么
- 模块损伤/维修/起火 → M2-016；Sherman 外的其余载具（M8/M5A1/Puma/Luchs/Panther/Tiger/半履带）→ M3；载具呼叫（Commander 面板）→ M3。
- 载具渲染与动画 → M2-047/渲染任务；载具预测（客户端）→ M1-041（Pz IV/轮式的预测联调在 M2-049 压测前验证）。
- bot 驾驶 → M3（ARCH §16.2）。

## 验收标准
1. 载具成本扣资源与上限单测：Jeep 第 5 辆被拒；全队装甲第 7 辆被拒；Pz IV 在 8 999 tick 被拒、9 000 tick 通过；成本 M20/F15 精确扣除；资源不足不扣。
2. 冷却：被毁后 Jeep 60 s（1 800 tick）内同槽位不可再刷，±1 tick。
3. 回收：认领超时 60 s 与离队 120 s 回收返还 50%（M140 → 70）；被毁不返还。
4. 碾压规则单测：14.9 km/h 70 HP、15.0 km/h KIA、4.9 km/h 0 伤；车速 24 km/h 撞墙无伤、30 km/h 撞墙 hull_hp −= 0.5×(8.33)²≈34.7；离车 21 km/h 受 30 HP、19 km/h 无伤。
5. 路速：5 种载具路速与 `史实×0.75` 误差 < 3%（`cargo test -p sim vehicle_speed_m2`）；40 辆合成载具（含轮式）每 tick ≤ 2.0 ms（`cargo bench -p sim vehicles_40`，ARCH §6.10 阶段 3）。
6. 座位：开放车乘客射击扇区 90° 外的开火被拒；封闭卡车后厢乘客开火被拒；切座 < 5 km/h 立即、否则 60 tick。
7. 补给卡车 600 Supplies：Engineer 8 m 内可抽取，9 m 不可；卡车被毁后车载为 0。

## 测试要求
- 单元测试：spawner 状态机表驱动；proptest：任意请求序列不突破上限且资源不为负。
- 集成/无头：headless 5v5：RuleCommander 刷 Jeep/Pz IV，bot 乘员认领。
- 性能基准：`vehicles_40`（含 20 轮式）。
- 回归：M1 Sherman 路速/弹道测试不变。

## 性能预算
ARCH §6.10 阶段 3「载具 ≤ 40 辆 2.0 ms」；载具实体网络 delta ≈ 24 B/辆（ARCH §6.9.1，轮式可省履带字段）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **M2 临时规则（已裁定，GD §14 载具刷新小节与 MILESTONES M2 范围已注明）**：M2 无 Commander 席位，由"规则指挥官（服务器自动，`Requester::RuleCommander`）"按固定节奏刷新载具到 HQ 载具位，Crewman 在 HQ 载具位旁按交互认领（`Requester::CrewClaim` 走同一资源/上限校验）；M3 由 Commander 面板取代。
- 轮式物理是新模型（M1-022 仅履带），规模风险；超时则拆续作 M2-015b（轮式）。
- hull_hp 取自 /data（M1-009 为准），GD 仅在维修示例里提到"1600 HP 中型坦克"。
