# M3-015 sim：中/重型坦克

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 前置依赖：M3-013
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §14.3; GAMEDESIGN §14.5; GAMEDESIGN §14.5.4; GAMEDESIGN §14.6; ARCHITECTURE §5.3

## 背景
M1-009/M1-022/M1-023 与 M2-015 已提供 M4A1 Sherman 与 Panzer IV H 的物理与战斗。本任务接入四辆中/重型履带坦克：US **M4A3(76)W**（76 mm M1A1：AP 24/HE 18/WP 6，装填 3.5 s，史实 42 km/h，33.6 t）、**M4A3E2 Jumbo**（75 mm M3：AP 28/HE 24/WP 6，装填 3.2 s，35 km/h，38 t）、GER **Panther Ausf. G**（7.5 cm KwK 42 L/70：AP 24/HE 20，装填 3.5 s，46 km/h，44.8 t）、**Tiger I**（8.8 cm KwK 36 L/56：AP 20/HE 14，装填 4.5 s，38 km/h，57 t）。
游戏最高速 = 史实 × 0.75：M4A3(76)W 31.5、Jumbo 26.25、Panther 34.5、Tiger 28.5 km/h。物理（GD §14.5.4）：原地转向速度 30°/s（中型）/22°/s（重型）；加速到最高速 6–12 s；爬坡上限 30°（履带）；炮塔转速 M4A3(76)W 24°/s、Jumbo 20°/s、Panther 20°/s、Tiger 14°/s；俯仰 Sherman −10/+25、Panther −8/+18、Tiger −8/+15（【待调参】）。模块 HP：Engine 300（重型 400），其它按 GD §14.5.1。M3-002 已落数据，本任务写 sim 行为。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/tracked.rs`（M1-022 已有）—— 质量/转向/加速度曲线参数化：重型 vs 中型转向速度、爬坡上限、地形阻力；`heavy` 标志。
- `crates/sim/src/vehicle/turret.rs`（M1-023 已有）—— 炮塔转速/俯仰限位按车型；稳定器无（GD §14.5.4）。
- `crates/sim/src/vehicle/reload.rs` —— 装填计时：弹种切换需 1.0 s 重新装填（GD §14.7）、车长缺位 ×1.3、Main Gun 模块 ≤ 50% 时 ×1.4。
- `crates/sim/src/vehicle/module_damage.rs`（M1-023 已有）—— 重型 Engine 400/中型 300 的 ≤ 50% 与 0 效果；起火（Engine 被击穿毁 40% 概率，持续 15 s，20 HP/s）；殉爆概率按车辆数据（M4A3(76)W/Jumbo 20%、Panther/Tiger 35%）；击毁后 Burning 8 s 再爆炸（半径 8 m Dmax 200）、残骸保留 60 s。
- `crates/sim/src/vehicle/fuel.rs` —— 燃料维持：中型 2/min、重型 3/min，Fuel=0 时全部载具速度 ×0.6（GD §11.4）。
- `crates/sim/tests/vehicles_heavy.rs`、`crates/sim/benches/vehicle_step_heavy.rs`。
### 要实现的行为
1. 车辆加载：由 `/data/vehicles` 构造 `VehicleController`，`heavy = mass_kg ≥ 38000`（或数据字段 `class: heavy`）。
2. 驾驶手感参数：原地转向、起步加速曲线（6–12 s 到最高速：Tiger 12 s、Sherman 6–8 s 用数据而非公式硬编码）。
3. 主炮与弹种：见背景表；Tiger 装填 4.5 s；弹药不足时拒绝开火并发 `OutOfAmmo` 提示。
4. 乘员/座位：3 座（驾驶/炮手/车长），车体 MG（驾驶员，固定向前）与同轴 MG（GD §14.3）；弹药独立。
5. 重型解锁/上限的逻辑在 M3-017；本任务只暴露 `VehicleClass`。
6. 碰撞：车速 > 25 km/h 撞固定障碍受 `0.5·v²` hull 伤害；碾压伤害规则沿用 M2-015（GD §7.6）。
### 接口与数据
- 与 M3-014 共用 `VehicleController`，`class ∈ {light, medium, heavy, transport, softskin}` 来自数据。

## 不做什么
- 装甲/穿深判定 → M3-013；轻型 → M3-014；呼叫/认领 → M3-017；AI → M3-023；渲染 → M3-029。
- 不做稳定器、车长舱盖外的车载工具（灭火器已含在模块损伤里，每车 2 次、每次 6 s 内扑灭，GD §14.5.2）。

## 验收标准
1. 四车最高速与数据（史实×0.75）误差 < 3%：31.5/26.25/34.5/28.5 km/h（平地 90 s 测量）。
2. 原地转向速度：中型 30°/s、重型 22°/s，误差 < 5%；Tiger 炮塔 14°/s、Panther 20°/s、Jumbo 20°/s、M4A3(76)W 24°/s，误差 < 3%。
3. 爬坡：30° 可爬、31° 不可（单测）；重型起步加速到最高速时间在 `/data` 值 ±10% 内。
4. 主炮装填时间误差 ≤ 1 tick：M4A3(76)W 3.5、Jumbo 3.2、Panther 3.5、Tiger 4.5 s；车长缺位 ×1.3、炮管受损 ×1.4（单测）。
5. 殉爆检定：10 000 次击穿命中弹药架采样，Panther 殉爆率 35% ±1.5 个百分点，M4A3(76)W 20% ±1.5（固定种子）。
6. Burning/爆炸/残骸时序：8 s 后爆炸、残骸 60 s 移除（±1 tick）。
7. 40 辆混合载具 `step` 合计 ≤ 2.0 ms/tick（ARCHITECTURE §6.10 阶段 3；`cargo bench -p sim vehicle_step_heavy`，参考机）。

## 测试要求
- 单元测试：转向、速度限幅、装填链、模块效果、燃料维持、殉爆概率统计。
- 集成/无头：headless 驾驶 Tiger 与 Panther 1 km，权威/预测位置误差 > 0.05 m 触发回滚的比率 < 1%（信息性）。
- 性能基准：`crates/sim/benches/vehicle_step_heavy.rs`。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 3：≤ 40 辆合计 2.0 ms/tick；重型单车 `step` ≤ 50 µs。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 验收写"装填时间误差 ≤ 1 tick"，本任务的 4 车数值取自 GD §14.3；M3-051 复核史实后可能改值，测试读 `/data` 而非常量。
- "重型原地转向/爬坡参数单测"以数据中 `turn_rate_dps` 与 `max_slope_deg` 为准。
- 殉爆概率检定须用确定性 RNG；若测试不稳定，用 2 000 次、容差放宽至 ±3 点并在 STATUS 注明。
