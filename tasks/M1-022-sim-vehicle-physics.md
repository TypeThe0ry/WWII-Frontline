# M1-022 sim：载具物理（悬挂、履带、座位）

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：L
- 前置依赖：M0-010, M1-009
- 素材需求：无（灰盒即可；V-VEH-US-003）；素材状态：placeholder
- 引用文档：GAMEDESIGN §14.1; GAMEDESIGN §14.4; GAMEDESIGN §14.5; GAMEDESIGN §14.7; GAMEDESIGN §7.6; GAMEDESIGN §14.9; ARCHITECTURE §5.3; ARCHITECTURE §6.10; ARCHITECTURE §8.5.4

## 背景
载具用**自写 raycast 悬挂 + 履带模型**，不使用通用刚体车辆（ARCHITECTURE §5.3）。M1 只有 M4A1 Sherman（M1-009 的 `us_m4a1_75`），但代码要通用（M2/M3 增加其它载具）。预算（ARCHITECTURE §6.10 阶段 3）：**≤ 40 辆，每辆 4–8 悬挂射线 + 履带力，合计 2.0 ms/tick**。
物理与操作要点：
- 数据（M1-009）：每侧 6 悬挂射线、`rest_length 0.6 m`、`stiffness 38000`、`damping 3800`；履带 `max_torque 22000`、`friction_forward 1.1`、`friction_lateral 2.2`（均【待调参】）；质量 30.3 t；游戏最高速 = 史实路速 × 0.75（Sherman 38 → **28.5 km/h**）；加速到最高速 6–12 s；原地转向 30°/s（中型）；爬坡上限 30°（履带）；炮塔转速/俯仰由 M1-023。
- 驾驶输入（GD §14.7）：W/S 油门/倒车、A/D 转向，**差速转向**；Shift 提速至游戏最高速；手刹。输入来自 `InputCmd`（`move_x/move_z`、`buttons`：sprint 作提速、`vehicle_seat` bit13）。
- 座位（GD §14.1）：Driver、Gunner、Commander（M1 三座）；停车或速度 < 5 km/h 时立即切换，移动中切换需 2 s；离车按住 0.8 s，车速 > 20 km/h 离车受 30 HP；载具"认领"：Armor Crew 小队任一成员靠近按交互即占位，整个小队可上车（M1 简化为任意本队 Crewman 可上）。
- 碰撞：与建筑/树木/障碍，车速 > 25 km/h 时受 `0.5×v²` 点 `hull_hp` 伤害（v 单位 m/s）；碾压步兵（GD §7.6）：车速 ≥ 15 km/h KIA、5–15 km/h 70 HP、< 5 km/h 无伤（伤害结算由 M1-016 环境伤害消费本任务提供的碰撞事件）。
- 乘员：每个座位乘员为独立实体（HP 100）；M1 乘员死亡座位空出，他人可补位（实现状态，伤害由 M1-023 触发）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/mod.rs` —— `Vehicle` 组件、`step_vehicles(world)`
- `crates/sim/src/vehicle/suspension.rs` —— 射线悬挂（弹簧-阻尼）、接地检测
- `crates/sim/src/vehicle/tracks.rs` —— 履带驱动力、差速转向、侧向摩擦（`friction_lateral`）、刹车
- `crates/sim/src/vehicle/seats.rs` —— 座位占用、进出、换座、离车伤害
- `crates/sim/src/vehicle/collision.rs` —— 车体与世界/玩家碰撞事件、碾压、碰撞伤害
- `crates/sim/src/vehicle/input.rs` —— `InputCmd` → 驾驶控制映射
- `crates/sim/src/vehicle/snapshot.rs` —— `VehicleView`、`VehicleExtra` 导出（供 M1-010/M1-024）
- `crates/sim/benches/vehicle_step.rs`；`crates/sim/tests/vehicle_*.rs`
- `crates/headless-client/src/scenarios/drive_straight.rs`（驾驶脚本，供 M1-041/M1-069 复用）
### 要实现的行为
1. 车体 = Rapier 动态刚体（质量、惯量取 `/data`），每 tick 对每条悬挂射线做地形/静态几何射线查询，计算弹簧力 `F = k·compression − c·v_rel`（只在接地时），累加到车体；**不使用 Rapier 车辆控制器**。
2. 履带力：左右履带各有驱动扭矩/速度环；转向 = 左右差速；前向/侧向摩擦用椭圆摩擦模型；最高速限制在游戏值（28.5 km/h）；倒车上限 = 40% 最高速。
3. 地形适应：车体姿态由悬挂自然产生；坡度 > 30° 驱动力衰减至 0；翻覆检测（倾角 > 70° 持续 3 s 判损毁，M1 仅记事件）。
4. 座位：`enter(player, seat)`、`exit(player)`、`switch_seat(player, seat)`（含 2 s/即时规则）；玩家进入车辆时从世界移除步行实体（保留身份）；驾驶员输入驱动车辆，炮手/车长输入交给 M1-023。
5. 碰撞事件：`VehicleCollision{vehicle, other, impulse, speed_mps}` → 车体伤害 `0.5·v²`（v>25 km/h）；与步兵碰撞输出 `CrushEvent{victim, speed_kph}`。
6. 载具实体状态量化与 `VehicleExtra`（quat smallest-three、速度 4 B 辅助外推）导出，delta ≤ 24 B（M1-010）。
7. 载具预测接口：`PredictSim::step_vehicle_driver()`（client-predict feature）供 M1-041 使用；服务器/预测共用同一函数。
8. 确定性：所有射线/力计算顺序固定（按 `EntityId` 升序），用 `libm`；`state_hash` 包含载具位姿。
### 接口与数据
```rust
pub struct VehicleCtrl { pub throttle: f32, pub steer: f32, pub boost: bool, pub brake: bool }
pub fn step_vehicles(w:&mut World, dt:f32) -> SmallVec<[VehicleEvent; 8]>;
pub fn seat_enter(w:&mut World, p:PlayerId, v:EntityId, seat:SeatId) -> Result<(), SeatError>;
```

## 不做什么
- 主炮/机枪/装甲判定/模块损伤/弹药：M1-023；
- 客户端载具预测与对账：M1-041；渲染与履带视觉：M1-048；
- 轮式载具（Jeep/卡车）与半履带：M2/M3；
- 载具刷新/认领超时/资源成本：M2（M1 用 `m1_override`）；
- 载具 bot 驾驶：M3。

## 验收标准
1. 路速与 `/data`（史实 × 0.75）误差 < 3%：平地满油门提速到稳定速度 ≈ 28.5 km/h（8–10 s 窗口平均），`|v−28.5|/28.5 < 3%`（`cargo test vehicle_top_speed`）。
2. 加速时间：0→90% 最高速 6–12 s；原地转向 30°/s ±10%；爬坡：25° 可爬、32° 坡上停住下滑（单测）。
3. 悬挂：静止在平地 ≥ 5 s 后车体高度稳定且 6 s 内振幅 < 2 cm；从 0.5 m 落下 2 s 内稳定（单测）。
4. 座位：速度 < 5 km/h 立即换座、10 km/h 需 2 s（±1 tick）；离车 0.8 s；车速 21 km/h 离车受 30 HP（单测）。
5. 碰撞：以 30 km/h 撞墙受 `0.5×(8.33)²≈34.7` 点 hull_hp 伤害（±5%）；24 km/h 无伤；碾压 16 km/h 步兵 KIA、10 km/h 70 HP、4 km/h 无伤（单测）。
6. **40 辆合成载具**同时驾驶/静止混合，一 tick 的 `step_vehicles` ≤ 2 ms（`cargo bench -p sim vehicle_step`，参考机，对应 ARCHITECTURE §6.10 阶段 3）。
7. 确定性：10 辆载具随机输入 1 万 tick，`state_hash` 两次一致（native/native 阻断；wasm/wasm 同）。
8. 载具 `VehicleExtra` 编码 delta ≤ 24 B（与 M1-010 联调的 `cargo test vehicle_delta_size`）。

## 测试要求
- 单元测试：弹簧阻尼数值、履带力方向、差速、摩擦椭圆、座位状态机、事件阈值。
- 集成/无头：`drive_straight` 场景在 server 内驾驶 60 s，路径偏差 < 1 m、无抖动（STATUS 记录）。
- 性能基准：criterion `vehicle_step`（含 6 射线/侧、40 辆）。
- 回归：确定性哈希基线更新（M0-029）。

## 性能预算
- 引用 ARCHITECTURE §6.10 阶段 3：≤ 40 辆合计 2.0 ms/tick；每辆 4–8 悬挂射线 + 履带力。超预算则写 PLAN_ISSUES（不要自行降精度）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L 且调参难（履带摩擦/悬挂刚度）；先达成"可驾驶、稳定、确定性"，手感调参（对标 V-01–V-05）在 M1-071。
- "车体用 Rapier 刚体 + 自写力"是对 ARCHITECTURE §5.3 的实现细化（文档只说"不使用通用刚体车辆"）；若 `enhanced-determinism` 对刚体积分有性能问题，写 PLAN_ISSUES。
- 灰盒碰撞体：车体盒 + 悬挂挂点（ASSET_GAPS §3.2），尺寸取 M1-009 的 `dimensions_m`。
