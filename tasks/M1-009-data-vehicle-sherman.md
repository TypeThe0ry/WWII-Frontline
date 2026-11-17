# M1-009 数据：M4A1 Sherman（M1 的 1 辆坦克）

## 元数据
- 里程碑：M1
- 工作流：data
- 规模：S
- 前置依赖：M0-004
- 素材需求：无（灰盒即可；真素材见 V-VEH-US-003，由 M1-064 接入）；素材状态：placeholder
- 引用文档：GAMEDESIGN §14.1; GAMEDESIGN §14.2; GAMEDESIGN §14.3; GAMEDESIGN §14.4; GAMEDESIGN §14.5; GAMEDESIGN §14.6; ARCHITECTURE §8.5.4; ARCHITECTURE §5.3

## 背景
M1 唯一的坦克是 US **M4A1 Sherman (75)**，ID `us_m4a1_75`（GD §14.3.1）。本任务产出 `data/vehicles/us_m4a1_75.json`（+ 依赖的炮弹与车载武器数据），供 M1-022（物理）、M1-023（武器/装甲判定）使用。GER 无坦克，GER 以 Panzerfaust 60（M1-008）对抗。结构按 ARCHITECTURE §8.5.4（historical / gameplay 分块，所有数值带单位），状态 `initial`；史实复核在 M1-070（R8），穿深等为"游戏标称值"【待调参】。

## 具体内容
### 要创建/修改的文件
- `data/vehicles/us_m4a1_75.json`
- `data/ammo_shells/us_75mm_m61_apcbc.json`、`us_75mm_m48_he.json`、`us_75mm_wp_smoke.json`（或并入车辆文件的 `weapons[]`，二选一，须与 M0-004 schema 一致；独立文件便于复用于 M3 的 M4A3E2）
- `data/schemas/vehicles.schema.json`（若 M0-004 的草案缺字段则补全：`seats[]`、`armor_zones[]`、`modules[]`、`weapons[]`、`physics`、`spawn`）
- `tools/data-validate` 增加车辆规则：座位 ID 唯一、`armor_zones` 覆盖 9 区（车体前上/前下/侧/后/顶、炮塔前/侧/后/顶）
### 要实现的行为（字段与初值）
1. **historical**（GD §14.3.1）：mass 30.3 t、乘员 5、主炮 75 mm M3、史实路速 38 km/h，`_source` 占位 `"待 M1-070 复核"`。
2. **座位**（游戏 3 座，GD §14.1/§14.3）：`driver`（可操作车体 M1919，400 发）、`gunner`（主炮 + 同轴 M1919 1000 发，`turret:true`）、`commander`（车顶 M2HB 200 发，舱盖开/关，`turret:true`）；缺位时装填 ×1.3 变慢（装填手/无线电员由 AI 抽象）。
3. **主炮**：75 mm M3，弹药 AP 30 / HE 24 / WP 6，装填 3.0 s；切换弹种需重新装填 1.0 s（GD §14.7）。
4. **弹种**（GD §14.2 表）：M61 APCBC 初速 619 m/s、`P100` 82 mm、`drop` 3.5%/100 m、结构伤害 400；M48 HE 爆炸 Dmax 250/R 7 m，HE 穿深 = 口径 20%（15 mm）；WP 烟雾云团 R 12 m、40 s。
5. **装甲**（GD §14.3.3，板厚 mm / 倾角°）：车体前上 51/56、前下 51/45、侧 38/0、后 38/0、顶 19/90；炮塔前（含防盾）76/30、侧 51/25、后 51/25、顶 25/90。
6. **模块**（GD §14.5.1）：Engine 300、Track L/R 各 250、Turret Drive 200、Main Gun 250、Ammo Rack 150（殉爆概率 60%，干式弹药架，GD §14.5.3）、乘员 100 HP。
7. **`hull_hp`**：初值 **1600**（GD §14.8 隐含"1600 HP 中型坦克"）；注意 ARCH §8.5.4 示例写 1000，仅为结构示例，以 GD 为准并在 PLAN_ISSUES 登记不一致。
8. **物理/悬挂**（GD §14.4/§14.5.4 + ARCH §8.5.4 结构）：游戏最高速 = 38 × 0.75 = **28.5 km/h**；炮塔转速 24°/s；俯仰 −10°/+25°；原地转向 30°/s（中型）；加速到最高速 6–12 s；爬坡上限 30°；悬挂每侧 6 射线（ARCH 示例初值，【待调参】）。
9. **刷新/成本**（GD §14.6.1）：M 140 / U 80 / F 120、被毁冷却 240 s、需 Armor Crew ≥ 2 人、时间门槛 5 min；M1 测试服用 `spawn.m1_override: {cost:{manpower:0,munitions:0,fuel:0}, cooldown_s:60, time_gate_s:0}`（M1 无资源系统，战略资源在 M2）。
10. 灰盒尺寸字段 `dimensions_m {length, width, height}`（取史实 5.84×2.62×2.74，`_source` 待复核）与挂点名（`socket_gun_muzzle`、`socket_turret`、`socket_hatch`），供 M1-048 占位使用。
### 接口与数据
字段骨架同 ARCHITECTURE §8.5.4；`armor_zones[]`: `{id, thickness_mm, slope_deg, kind:'plate'|'mantlet'}`；`modules[]`: `{id, hp, effects:{at_50pct:…, at_0:…}}`；`weapons[]`: `{id, shell_ids:[…], ammo:{ap:30,he:24,wp:6}, reload_s:3.0, seat:'gunner'}`。

## 不做什么
- 悬挂/履带物理实现：M1-022；装甲判定与弹道：M1-023。
- 其余载具（Jeep、Panther 等）与 GER 载具：M2/M3。
- 战略资源池与载具刷新规则：M2（M1 用 `m1_override`）。
- 车载武器 M1919/M2HB 的完整武器 JSON：作为载具武器条目内联最小字段（射速/伤害取 GD §6.3 的 vehicle_m2hb 与 us M1919 行），完整武器化不在本任务。

## 验收标准
1. `data-validate` 通过（0 错误）；全字段带单位后缀。
2. 与 GD §14.3 逐项一致：乘员/座位数 3、主炮弹药 30/24/6、装填 3.0 s、装甲 9 区共 18 个数值、5 个模块 HP、殉爆 60%（`diff-gd-vehicle` 脚本 0 差异）。
3. 派生值：`max_speed_kph == 28.5`（= 38×0.75，浮点容差 0.01）；单测断言。
4. `armor_zones` 覆盖 9 区且 `thickness_mm>0`、`slope_deg ∈ [0,90]`；座位 ID 唯一。
5. `_meta.status=="initial"`；`historical._source` 存在。
6. `cargo build -p sim` 通过且 `DATA_HASH` 稳定；`sim-wasm` 体积回归 < 5 KB（STATUS 记录）。

## 测试要求
- 单元测试：schema 正反例（缺 armor 区、倾角 > 90、座位重名）；`diff-gd-vehicle`。
- 集成：`cargo test -p sim data::vehicles`。
- 性能基准：无。
- 回归：`DATA_HASH`。

## 性能预算
- 无。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 文档不一致需登记：ARCH §8.5.4 示例 `hull_health 1000` 与 GD §14.8 的 1600；GD 以文字出现、ARCH 为示例，按 GD 执行。
- 悬挂/履带参数（stiffness、damping、friction）无史料来源，纯调参量，M1-022 实测后回填；对标测量 V-01–V-05 等见 M1-071。
- 灰盒占位着美军橄榄绿，节点 `PH_V-VEH-US-003__sherman`（ASSET_GAPS §3.1）。
