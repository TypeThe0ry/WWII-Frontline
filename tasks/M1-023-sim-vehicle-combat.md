# M1-023 sim：载具武器与装甲判定（含 Panzerfaust 60 发射与 HEAT 判定）

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：L
- 前置依赖：M1-022, M1-012, M1-014
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §14.2; GAMEDESIGN §14.3; GAMEDESIGN §14.4; GAMEDESIGN §14.5; GAMEDESIGN §14.7; GAMEDESIGN §6.9.4; ARCHITECTURE §6.7.3

## 背景
实现 Sherman 的主炮（AP/HE/WP）、机枪、炮塔/炮管运动、装甲判定与模块损伤，是 E1-12（"AP 命中穿透判定与 §14.4 手算一致 ≥ 12 组"）的主体。数据由 M1-009 提供。
**装甲判定流程（GD §14.4.1）**：
1. 弹丸线段与载具外形碰撞体相交 → 命中点、法线 `n`、方向 `v̂`；
2. 命中区域 `armor_zone` → 板厚 `t`、倾角基准；**实际入射角以命中面几何法线为准**；
3. `φ = acos(−v̂·n)`；AP/APCBC：`φ_eff = max(0, φ−5°)`（normalization）；HEAT：`φ_eff = φ`；
4. **跳弹**：AP `φ_eff ≥ 68°`、HEAT `≥ 75°` → 跳弹（镜面反射，衰减 60% 动能，无穿透/结构伤害；对外露模块 3% 冲击）；**过匹配**：口径 ≥ 3×板厚 → 不跳弹，`φ_eff = min(φ_eff, 45°)`；
5. `t_eff = t / cos(φ_eff)`，上限 `3t`；
6. 穿深 `P(d) = P100 × (1 − drop×(d−100)/100)`（d<100 m 时 `P=P100`，最小 0.3×P100；HEAT drop=0）；M61 APCBC：`P100=82 mm`、`drop 3.5%/100 m`、结构伤害 400；M48 HE：Dmax 250/R 7 m，HE 对装甲穿深 = 口径 20%（15 mm）；
7. 击穿判定：`P(d)×U(0.95,1.05) ≥ t_eff`（`U` 由 `Rng::for_(tick, shooter, ARMOR)`）；
8. 击穿后：`P_rem = P − t_eff`；`dmg_pen = 结构伤害×U(0.9,1.1)`；`internal_len = 0.6 + P_rem/40`（上限 3.0 m）；沿路径内部碰撞体：乘员伤害 `dmg_pen×0.25`（≥100 KIA）、同舱室他人 `×0.08`、模块 `×1.0`、弹药架按殉爆概率检定（Sherman 干式 **60%**）；`hull_hp −= dmg_pen×0.5 + 模块伤害合计×0.2`；
9. 未击穿：`hull_hp −= 3%×结构伤害`；外露模块（履带 20 mm、炮管 50 mm）按 §14.4.5。
**模块（GD §14.5）**：Engine 300、Track L/R 250、Turret Drive 200、Main Gun 250、Ammo Rack 150；HP ≤ 50% 与 = 0 的效果（速度 ×0.6、炮塔 ×0.5、装填 ×1.4/散布 ×1.5 等）；殉爆：车辆立即毁、乘员全灭，爆炸 R 12 m、Dmax 350；击毁 → Burning 8 s，乘员可逃生（20 HP 燃烧伤害），8 s 后爆炸 R 8 m、Dmax 200，残骸 60 s。
**炮塔/炮管**：Sherman 转速 24°/s，俯仰 −10°/+25°；车长缺位装填 ×1.3；切换弹种需重新装填 1.0 s；主炮装填 3.0 s。
**小口径对装甲（§14.4.6）**：`t ≥ 12 mm` 不击穿；`t < 12 mm`（本场景只有未来的轻型车）略。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/armor.rs` —— `armor_hit(proj, vehicle, hit) -> ArmorResult{Pen, NoPen, Ricochet}`，纯函数（便于手算对照）
- `crates/sim/src/vehicle/internal.rs` —— 内部路径检测与模块/乘员伤害
- `crates/sim/src/vehicle/modules.rs` —— 模块状态与效果、殉爆、起火/Burning
- `crates/sim/src/vehicle/turret.rs` —— 炮塔/炮管运动（速率限制、俯仰限位）
- `crates/sim/src/vehicle/weapons.rs` —— 主炮（装填/弹种/弹药）、同轴/车体 M1919、车顶 M2HB（复用 M1-013 的自动武器）
- `crates/sim/src/vehicle/heat.rs` —— HEAT 判定与 **Panzerfaust 60 弹丸**（见备注）
- `crates/sim/tests/armor_cases.rs`、`tests/fixtures/armor_cases.json`
- `crates/sim/benches/armor_hit.rs`
### 要实现的行为
1. 主炮开火：枪口位置/方向来自炮塔朝向；弹丸走 M1-012 的段扫描；弹种数据来自 M1-009（AP 30/HE 24/WP 6）；命中时调用 `armor_hit`。
2. `armor_hit` 严格按上述 9 步实现，所有阈值读 `/data`；随机因子可注入固定值（测试用）。
3. 模块损伤与效果表（HP ≤ 50% 与 0）；乘员伤害（独立 HP 100）；殉爆检定；Burning 状态机。
4. 炮塔转动：输入来自炮手 `InputCmd.yaw/pitch` 增量，受 24°/s 与俯仰限位约束；Turret Drive 损伤 ×0.5；卡死时 3°/s。
5. 同轴机枪随炮塔、车体机枪随车体；车顶 M2HB（舱盖开时可用）；弹药由数据给出（M1919 1000/400，M2HB 200）。
6. HE 命中步兵/软目标的爆炸：Dmax 250 / R 7 m，走 M1-015 的 `explosion_damage`；WP 烟雾云团 R 12 m、40 s（复用烟雾系统）。
7. 事件：`VehicleDamaged{module,hp}`、`VehicleKnockedOut`、`VehicleAmmoExplosion`、`CrewKilled`；`Explosion` 事件 kind 区分。
8. **Panzerfaust 60 与 HEAT**：GER 步兵 loadout（M1-007 `ger_rifleman_pf60`）发射一次性火箭：初速 45 m/s、抛物线无推力、有效 60 m（最大 80）、HEAT 穿深 200 mm 不衰减、结构伤害 900、爆炸 Dmax 150/R 5 m，后焰 5 m 锥 30 HP；`φ_eff = φ`、无 normalization、跳弹 ≥ 75°。**INDEX 未明确该武器的归属任务**（见备注）。
### 接口与数据
```rust
pub enum ArmorResult { Pen { p_rem: f32, t_eff: f32 }, NoPen { t_eff: f32 }, Ricochet }
pub fn armor_hit(shell:&ShellDef, dist_m:f32, n:Vec3, v_hat:Vec3, zone:&ArmorZone, rng_u:f32) -> ArmorResult;
```

## 不做什么
- 载具物理/座位：M1-022；
- 弹药补给/维修（Repair Station、Crewman 维修）：M2/M3；
- 其它炮种/车型（Panther、Tiger…）：M3；
- 反坦克炮（AT Gun）：M2；炸药包/地雷：M2；
- 客户端炮口焰/命中特效/车内视角：M1-047/M1-048；
- 载具伤害 UI（模块状态）：M1-060。

## 验收标准
1. **≥ 12 组**（板厚×倾角×距离）AP 判定与手算一致：`tests/fixtures/armor_cases.json` 含 12 组（板厚 {19,38,51,76} mm × 水平入射倾角 {0°,30°,56°} × 距离 {100,300,600} m 中选取，必须覆盖：击穿、未击穿、跳弹、过匹配各 ≥ 1），期望值按 §14.4.1 手算并在文件内逐步骤给出（`φ_eff, t_eff, P(d), 结论`）；以随机因子=1.0 运行 12/12 一致；对临界样例（|P−t_eff|/t_eff < 5%）另测 ±5% 区间两端结果相反。
2. 穿深衰减：M61 在 100/300/600 m 的 P = 82.0/76.3/67.7 mm（±0.1）。
3. 跳弹阈值：AP `φ_eff` 67.9° 不跳、68° 跳；HEAT 74.9° 不跳、75° 跳；过匹配（75 mm 打 9.5 mm 板）不跳（单测）。
4. 炮塔转速与 `/data` 一致：24°/s ±2%；俯仰限位 −10°/+25° 外输入被钳制；Turret Drive HP ≤ 50% 后 12°/s ±5%（单测）。
5. 主炮装填 3.0 s ±1 tick；换弹种需重新装填 1.0 s；车长缺位 ×1.3（3.9 s）（单测）。
6. 模块效果：Engine 降到 150 HP 后最高速 ×0.6；Track L 毁后只能向左转圈速度 ×0.3；Ammo Rack 被击穿检定 60% 殉爆（固定 seed 1000 次统计 58–62%）。
7. Panzerfaust：60 m 处命中 Sherman 侧甲（38 mm/0°）击穿；对前上 51/56° 在 60 m 命中同样击穿（t_eff=51/cos(56°)≈91 < 200）；80 m 外下坠明显（落点偏差 > 1.5 m 的单测）。
8. 性能：`cargo bench -p sim armor_hit` 中位数 ≤ 2 µs/次；含内部路径检测 ≤ 10 µs；确定性哈希一致。

## 测试要求
- 单元测试：`armor_hit` 全分支、内部路径长度、模块效果、殉爆概率、炮塔限位、HEAT 弹道。
- 集成/无头：headless：US 坦克炮击木靶/固定装甲靶；GER 步兵以 Panzerfaust 击毁 Sherman（事件 `VehicleKnockedOut`）。
- 性能基准：criterion `armor_hit`。
- 回归：`armor_cases.json` 入库作为合同；`DATA_HASH` 与确定性哈希更新。

## 性能预算
- 并入 ARCHITECTURE §6.10 阶段 3/4（载具 2.0 ms、武器与弹道 2.5 ms）；装甲判定不得使阶段 4 超 2.5 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **已裁定归属**：Panzerfaust 60 的发射与 HEAT 判定归本任务（INDEX 行与标题已注明；MILESTONES §2.4 演示脚本第 6 步需要）。规模风险高，如超出一次会话按 README §5.6 拆续作 M1-023b。
- GD §14.4.2 的手算示例使用 `φ=30°` 而未做 5° normalization（与 §14.4.1 步骤 4 不一致）；本任务以 §14.4.1 步骤为准，示例仅参考，PLAN_ISSUES 登记。
- 规模 L 且含 4 块逻辑（装甲、模块、炮塔、HEAT）；若超出按 §5.6 拆 `M1-023b`（模块/殉爆/Burning）。
- 穿深、结构伤害等为"游戏标称值"【待调参】，对标 V-11–V-16、V-19–V-21 在 M1-071 回填。
