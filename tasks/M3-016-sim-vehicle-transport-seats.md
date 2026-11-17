# M3-016 sim：运输载具座位与乘员射击

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 前置依赖：M2-015, M3-002
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §14.3; GAMEDESIGN §14.7; GAMEDESIGN §14.9; GAMEDESIGN §14.1; GAMEDESIGN §12.2; ARCHITECTURE §6.5.2

## 背景
M2-015 已实现 Jeep/Kübel、CCKW/Blitz 卡车、载具刷新与载员乘坐的基础。本任务补齐 **半履带运输车**（M3 Half-track / Sd.Kfz. 251）与**座位规则全集**：座位类型与权限、乘客可射击扇区、补给卡车车载补给、封闭车厢限制、离车/换座。GD §14.3 数据：
- M3 Half-track：驾驶、机枪手（车长位）、乘客 ×8（共 10 座）；车长位 M2HB 300 发（枪盾）；乘客可射击；车载补给 300 Supplies；最高速 72×0.75 km/h。
- Sd.Kfz. 251/1：驾驶、机枪手、乘客 ×8；车长位 MG 34/42 800 发（枪盾）；乘客可射击；53 km/h。
- GMC CCKW / Opel Blitz：驾驶、副驾、后厢 ×10（12 座）；**封闭车厢乘客不可射击**，副驾可持手枪射击；补给版车载 600 Supplies。
- Jeep/Kübel：MG 版副驾为机枪手（M1919A4/MG 34 800 发）。
座位规则（GD §14.1）：停车或速度 < 5 km/h 时 F1–F4 立即切换，移动中切换需 2 s；离车按住 0.8 s；车速 > 20 km/h 离车受 30 HP 伤害；运输车辆任意玩家可上；乘客（开放车厢）可用手枪/冲锋枪在 **90° 扇区**射击。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/seats.rs`（M1-022/M2-015 已有）—— 座位表驱动：`SeatDef{id, kind: Driver|Gunner|Commander|Passenger|MgGunner, can_fire, fire_arc_deg, weapon, enclosed}`；`enter/exit/swap_seat`，延迟与伤害规则。
- `crates/sim/src/vehicle/passenger_fire.rs` —— 乘客射击：仅允许 `can_fire` 的座位；武器限手枪/冲锋枪类（`kind ∈ {pistol, smg}`）；射击方向须在座位扇区 ±45° 内（90° 扇区），超出则开火被服务器拒绝并钳制；封闭车厢乘客完全禁射。
- `crates/sim/src/vehicle/cargo.rs` —— 补给卡车/半履带车载 Supplies：卡车 600、半履带 300；Support 在 3 m 内装卸 1 箱需 2 s；车被毁则车载补给丢失；Engineer/Support 在 8 m 内可取用（GD §12.2）。
- `crates/sim/src/vehicle/mounted_mg.rs` —— 车顶/枪盾机枪：半履带 M2HB 300 发、251 MG 800 发；弹药独立；机枪手位座椅视角限位。
- `crates/sim/tests/transport_seats.rs`。
### 要实现的行为
1. 座位占据：先到先得；已占用拒绝；Armor Crew 认领规则不适用运输车（任意玩家可上）。
2. 换座：`speed < 5 km/h` 立即；否则 2 s 读条，被打断则取消；换座期间视作无座（不可开火）。
3. 离车：按住 0.8 s；`speed > 20 km/h` 时受 30 HP 伤害；载具击毁（Burning）时乘员可逃生并受 20 HP 燃烧伤害（GD §14.5.2）。
4. 乘客射击矩阵：{开放/封闭} × {Jeep、半履带、卡车副驾、卡车后厢} × {手枪、SMG、步枪、MG} 的 `can_fire` 结果表（详见验收 2）。
5. 乘员作为独立实体可被内部/外部命中（GD §14.1：HP 100），开放车厢乘客可被枪弹直接命中（M3-013 软车辆规则）。
6. 车上玩家的 `PlayerAuthState` 位置跟随车辆（不单独跑 KCC），预测子集需包含载具座位状态（`client-predict`）。
### 接口与数据
- 座位定义来自 `/data/vehicles/*.json` 的 `seats[]`（M3-002），本任务补充字段 `fire_arc_deg`、`enclosed`、`weapon_ref`（schema 追加并 PLAN_ISSUES 记"细化"）。
- 事件：`SeatChanged{player, vehicle, seat}`、`VehicleCrewExit`（同步 `VehicleExtra.seat_occupancy`，M3-005）。

## 不做什么
- 呼叫/认领/成本上限 → M3-017；车载语音频道 → M3-040；乘客渲染/第一人称座位视角 → M3-029/M3-036；
- 步兵"搭乘外挂"（GD §14.9 明确不做）；Armor Crew 认领规则 → M3-012。

## 验收标准
1. 座位权限与可射击矩阵单测：覆盖 6 种运输/侦察车 × 全部座位 × {pistol, smg, rifle, mg}，≥ 60 个断言全部通过；封闭卡车后厢 `can_fire=false` 的断言数 = 全部武器类型。
2. 90° 扇区边界：扇区中线 ±45° 内开火通过、±46° 被钳制/拒绝（单测）。
3. 换座：静止立即、`speed ≥ 5 km/h` 需 2 s ±1 tick；离车 0.8 s ±1 tick；`> 20 km/h` 离车伤害 = 30 HP；`≤ 20 km/h` = 0。
4. 补给：卡车载 600、半履带载 300 Supplies；装/卸 1 箱 2 s ±1 tick；击毁后车载补给清零。
5. M2HB（半履带）弹药 300 发与 251 的 MG 800 发独立计数；乘客弹药不影响车载 MG。
6. 12 名乘客的 CCKW 满载时座位占用位图编码在 `VehicleExtra` 不超过 2 B（单测）。
7. 单车座位逻辑每 tick ≤ 5 µs（`criterion`）。

## 测试要求
- 单元测试：上述矩阵；换座打断；乘员死亡后座位空出可补位；燃烧逃生伤害。
- 集成/无头：headless 6 名玩家乘坐半履带行驶 200 m，换座/下车事件序列与预期一致。
- 性能基准：`crates/sim/benches/seats.rs`。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 3（载具）：座位/乘客逻辑与物理合计受 2.0 ms 约束；本任务贡献 ≤ 0.1 ms（40 辆）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §14.1 说乘客扇区 90° 但未规定"以座位朝向还是车辆朝向为中线"，按座位朝向（左/右舷座位朝外）实现并写 PLAN_ISSUES。
- 半履带与卡车的数据由 M3-002/M2-006 提供，若 `seats[]` 缺 `enclosed` 字段，在本任务补充。
- 满载（12 人）车辆的快照字段位宽需与 M3-005/M3-022 协调。
