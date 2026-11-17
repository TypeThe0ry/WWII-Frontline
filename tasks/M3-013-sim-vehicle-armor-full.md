# M3-013 sim：全弹种穿深与装甲判定

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：L
- 前置依赖：M1-023, M3-002
- 素材需求：无（灰盒即可；装甲区用灰盒碰撞体的区域 ID 映射）；素材状态：n.a.
- 引用文档：GAMEDESIGN §14.2; GAMEDESIGN §14.4; GAMEDESIGN §14.5; GAMEDESIGN §6.9.4; ARCHITECTURE §8.1

## 背景
M1-023 实现了 M4A1 Sherman 的基础主炮弹（AP/HE）、穿深与装甲判定（12 组验收）以及基础模块损伤；M2-014 实现了 HEAT 对装甲的判定。M3 要把**全部弹种 × 全部载具**接入同一套判定：AP/APCBC、HE、HEAT、烟雾、倾角/跳弹/过匹配、侧裙与防盾弱点、车体顶/底部、小口径武器对装甲，并用 M3-002 的数据驱动装甲区。判定流程严格按 GD §14.4.1：
1. 线段与外形碰撞体相交 → 命中点、法线 `n`、速度方向 `v̂`；2. 区域识别取板厚 `t` 与基准倾角；3. 入射角 `φ = acos(−v̂·n)`（以几何法线为准）；4. 弹种修正：AP `φ_eff = max(0, φ−5°)`，HEAT `φ_eff = φ`；5. 跳弹：AP `φ_eff ≥ 68°`、HEAT `≥ 75°`；**过匹配**：口径 ≥ 3×板厚则不跳弹且 `φ_eff = min(φ_eff, 45°)`；6. `t_eff = t / cos φ_eff`（上限 `3t`）；7. `P(d) = P100×(1 − drop×(d−100)/100)`（d<100 m 取 P100，最小 0.3×P100，HEAT drop=0）；8. `P(d)×U(0.95,1.05) ≥ t_eff` 击穿；9. 击穿后 `P_rem = P(d)−t_eff` 做内部检测（§14.4.4）；10. 未击穿造成 3% 结构冲击伤害。
弹种范围（已裁定）：AP（含 APCBC/APC）、HE、HEAT、Smoke（GD §14.2）；**APCR 已从 INDEX 与本任务中删除**（GD 无数据，首发不做，不发明数值）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/armor.rs`（M1-023 已有）—— 泛化 `resolve_shell_hit(shell: &ShellDef, hit: &HitInfo, vehicle: &VehicleDef, rng) -> ShellResult`；`ShellResult{outcome: Pen|Bounce|Ricochet|Overmatch, p_eff, t_eff, internal_hits[]}`。
- `crates/sim/src/vehicle/shells.rs` —— `ShellDef` 加载 `/data/weapons` 主炮弹种（GD §14.2 表的 11 门炮 + 3 种火箭）；HE 穿深 = 口径 ×20%；Smoke 云团 R 12 m 持续 40 s（仅 75/76 mm 与 105 mm；炮兵烟雾见 M3-018）。
- `crates/sim/src/vehicle/interior.rs` —— 击穿后内部检测：`internal_len = 0.6 + P_rem/40`（上限 3.0 m），沿弹道对内部碰撞体：乘员伤害 `dmg_pen×0.25`（≥ 100 KIA；同舱其它乘员 `×0.08`）、模块 `×1.0`、弹药架触发殉爆检定（概率来自车辆数据，殉爆爆炸半径 12 m Dmax 350）；`hull_hp −= dmg_pen×0.5 + 模块伤害合计×0.2`。
- `crates/sim/src/vehicle/small_arms.rs` —— GD §14.4.6：小口径对 `t ≥ 12 mm` 不击穿；`t < 12 mm` 且 ≤ 200 m 时 `.30-06/7.92×57` 可击穿（P=8 mm@100 m，drop 25%/100 m，击穿后伤害 ×0.15）；.50 BMG P=20 mm。
- `crates/sim/src/vehicle/external_modules.rs` —— 履带/车轮 20 mm/垂直、炮管 50 mm/垂直（GD §14.4.5）。
- `crates/sim/tests/armor_matrix.rs`、`crates/sim/benches/armor_resolve.rs`。
### 要实现的行为
1. 全部弹种×车辆×部位×距离走同一函数，数据驱动；不允许 per-vehicle 硬编码分支。
2. 侧裙（Schürzen）：Pz IV H 侧裙区 HEAT 穿深 ×0.6、AP 无效果（GD §14.3.3 注）；Panther/Tiger 无侧裙（GD 未给，不加）。
3. 顶/底：顶部用垂直入射并按 HE 穿深（0.2×口径）；Bombing 600 HP 直击视为击穿（与 M3-008 对接）；底部走地雷流程（M2-014）。
4. 炮塔防盾弱点本版**不做**（GD §14.3.3 末注）。
5. 判定随机数走 `Rng::for_(tick, entity, purpose)`，保证确定性与预测一致。
6. 输出结果事件：`HitConfirm`/`Explosion`/`VehicleModuleDamaged`（协议见 M3-005）。
### 接口与数据
- `HitInfo{point, normal, dir, distance_m, zone_id, shell}`；`zone_id` 由碰撞体顶点着色映射到 `armor_zones[]`（灰盒碰撞体由 M3-029 提供，本任务用程序化盒体 + 区域标签）。

## 不做什么
- 车辆物理/驾驶 → M3-014/015；座位交互 → M3-016；服务器载具回溯 → M3-022；
- 模块损伤视觉 → M3-032；乘员内部碰撞体的几何模型细化 → 灰盒用 AABB。

## 验收标准
1. `cargo test -p sim armor_matrix`：≥ 40 组（弹种×车辆×部位×距离）判定与手算一致（手算表入库 `crates/sim/tests/data/armor_hand.csv`；含 GD §14.4.2 三个示例：M4A1 对 Panther 前上不穿、Panther 对 Sherman 车体前穿、Bazooka 对 Tiger 侧穿/前上不穿、Panzerfaust 对 Jumbo 前上穿）。
2. Tiger I 车体前上（100 mm/9°）对 75 mm M3（P100 82、300 m 约 76 mm）水平命中：100% 未击穿（10 000 次采样，U(0.95,1.05) 内无击穿）。
3. 跳弹阈值：AP `φ_eff` 67.9° 不跳弹、68.1° 跳弹；HEAT 74.9/75.1°；过匹配（75 mm 打 9.5 mm 侧板）不跳弹（边界单测）。
4. 穿深曲线：`P(d)` 在 100/300/600 m 与公式一致（误差 < 0.5%）；最小 0.3×P100 钳制生效。
5. 击穿后内部损伤：`internal_len` 上限 3.0 m、乘员 `0.25×`、模块 `1.0×`、`hull_hp` 公式（单测数值断言）。
6. 小口径：`.30-06` 对 M8 侧板（9.5 mm）≤ 200 m 击穿、对 Sherman 前不击穿；.50 BMG 对 Half-track 击穿。
7. `armor_resolve` 单次判定中位数 ≤ 5 µs（参考机，`criterion`）；同 seed 判定序列 `state_hash` 两次一致。

## 测试要求
- 单元测试：上述 1–6 + 随机性边界（U(0.95,1.05) 固定种子）。
- 集成/无头：服务器端一次"坦克对射"场景 100 次，判定分布与单测手算期望吻合（击穿率误差 ≤ 3 个百分点）。
- 性能基准：`crates/sim/benches/armor_resolve.rs`，基线入库。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 3（载具 ≤ 40 辆 2.0 ms）与阶段 4（武器与弹道 2.5 ms）；判定仅在命中事件发生，常态 0。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；若超限把小口径与外部模块拆为 `M3-013b`。
- GD §14.4.2 示例中 Jumbo 对 Panther 的"临界 ±5% 随机"依赖 RNG，测试须固定种子并只断言概率区间。
- 装甲/穿深全部【待调参】，对标测量 V-01–V-27 在 M3-052，数值只改 `/data`。
