# M1-008 数据：M1 武器与投掷物

## 元数据
- 里程碑：M1
- 工作流：data
- 规模：M
- 依赖：M0-008
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.2; GAMEDESIGN §6.3; GAMEDESIGN §6.4; GAMEDESIGN §6.5; GAMEDESIGN §6.8; GAMEDESIGN §6.9; ARCHITECTURE §8.5.1

## 背景
M0-008 已落地 `us_m1_garand`、`ger_kar98k` 两个 JSON（historical + gameplay，状态 `initial`）及 weapons schema。本任务补齐 M1 五兵种 loadout（M1-007）用到的其余武器与投掷物，**沿用 M0-008 的 schema 结构（historical / gameplay / _meta），不另起结构**。史实参数（GD §6.2）出自记忆，**未联网复核**（GD §0.4、BRIEF R8）→ 一律 `initial`，`historical._source` 先写占位 `"待 M1-070 按 REF §6 复核"`；复核在 M1-070。游戏性数值全部【待调参】，部分【待测 → REFERENCE_MEASUREMENTS §fire-rate / §recoil】。

## 具体内容
### 要创建/修改的文件
`data/weapons/` 新增：
- 步枪/卡宾：`us_m1_carbine`
- 冲锋枪：`us_thompson_m1a1`、`us_m3_grease`、`ger_mp40`
- 手枪：`us_m1911a1`、`ger_p38`
- 投掷物：`us_mk2`（破片）、`ger_m24`（破片）、`us_m18`（烟雾）、`ger_nb39`（烟雾）
- 火箭：`ger_panzerfaust_60`
- 近战：`us_m1_bayonet`、`ger_s8498_bayonet`、`us_m3_knife`、`ger_combat_knife`、`melee_rifle_butt`、`melee_pistol_butt`（GD §6.8 表；若 M0-004 schema 无 melee class，则扩展 schema 并登记 PLAN_ISSUES）
- `data/ammo/`（若 M0-004 未建则新建）：`us_30_carbine`、`us_45_acp`、`ger_9x19` 的 `k_drag_per_m`、`mass_g`、`pen_power`、`suppression_caliber_weight`（GD §6.4 阻力表、§6.7、§8.1）
### 要实现的行为（数值初值，取自 GD 表）
1. **us_m1_carbine**：.30 Carbine，15 发，初速 ≈ 580 m/s，半自动 cap 200 rpm，dmg 48，F0/F1/min = 50/250/0.55，战术装 2.0 s/空仓 2.4 s，ADS 0.24 s，pen 90；后坐 vkick 0.9°、hσ 0.20°、回落 14°/s、盲射 3.0°、ADS 0.20°、bloom 0.30°/发、衰 4.0°/s；k_drag 0.00150。
2. **us_thompson_m1a1**：.45 ACP，30 发，≈ 280 m/s，自动 700 rpm，dmg 40，20/90/0.55，2.3/2.7 s，ADS 0.22，pen 55；vkick 0.7、hσ 0.35、10°/s、盲射 3.8、ADS 0.35、bloom 0.35/5.0；k_drag 0.00170。
3. **us_m3_grease**：450 rpm，dmg 42，2.5/3.0 s；vkick 0.6、hσ 0.30（其余同 Thompson 行）。
4. **ger_mp40**：9×19，32 发，≈ 380 m/s，500 rpm，dmg 36，20/80/0.5，2.4/2.9 s，ADS 0.20，pen 60；vkick 0.65、hσ 0.30、10°/s、盲射 3.8、ADS 0.30、bloom 0.30/5.0；k_drag 0.00200。
5. **手枪**：M1911A1（7 发，cap 300 rpm，dmg 45，15/60/0.5，1.6/2.0 s，vkick 2.0）；P38（8 发，dmg 34，同衰减，vkick 1.8）。
6. **手雷**（GD §6.9.1）：Mk 2 引信 4.0 s、投速 18 m/s、Dmax 220/R 8.0 m、致命半径 ≈ 3.2 m、可烹饪、反弹 0.4/摩擦 0.6；M24 引信 4.5 s、20 m/s、200/8.5 m、≈ 3.8 m。**烟雾**（§6.9.2）：M18 引信 2.0 s、云团最大 7.0 m（3 s 展开）、持续 45 s（末 8 s 衰减）；Nb.Hgr.39 6.0 m、30 s。
7. **ger_panzerfaust_60**（§6.9.4）：初速 45 m/s、有效 60 m（最大 80）、抛物线无推力、HEAT 穿深 200 mm【待调参】、对建筑 1000、直击爆炸 Dmax 150/R 5 m、后焰 5 m 锥 30 HP、一次性（无装填）。
8. 近战（§6.8）：刺刀伤害 90、前摇 0.35 s、冷却 1.2 s、距离 1.7 m；枪托 40/0.45/0.9/1.3；匕首 70/0.25/0.8/1.2；手枪托 20/0.3/0.6/1.0；背后（夹角 > 120°）KIA；`ff_mult` 近战 0。
9. 每个文件：`_meta.status="initial"`、`_meta.tunable`、`historical._source` 占位、`faction`、`class`、`name_key`（`weapon.{id}.name`）。
### 接口与数据
沿用 ARCHITECTURE §8.5.1 示例结构；`falloff` 数组由 GD 的 F0/F1/min 展开为 `[{dist_m:0,mult:1.0},{dist_m:F0,mult:1.0},{dist_m:F1,mult:min}]`；射速字段 `rate_of_fire_rpm_cap`（半自动 cap）与 `rpm_cyclic`（自动）分开；`penetration.pen_power` 为整数"穿透点"（GD §6.7）。

## 不做什么
- 武器行为实现（状态机、散布、后坐）：M1-012、M1-013；投掷物行为：M1-015。
- 其余武器（BAR、MG42、StG44、G43、狙击、Bazooka、Panzerschreck…）：M2/M3。
- 史实复核与 `_source` 填写：M1-070；对标测量回填：M1-071。
- 载具武器（M2HB、MG34 同轴）：M1-009 只引用，数据归后续任务；M1 Sherman 同轴/车体机枪见 M1-009 备注。

## 验收标准
1. `tools/data-validate` 全部通过（0 错误），含新武器 ≥ 18 个文件；`data-validate` 作业 < 2 min。
2. 每个新文件 `_meta.status=="initial"` 且 `historical._source` 非空（脚本统计 100%）。
3. 逐项对表：dmg、F0/F1/min、rpm、装填秒数、pen_power、初速、弹匣容量与 GD §6.2/§6.3 表一致（脚本 `diff-gd-tables` 对 11 个枪械/投掷物逐字段比对，0 差异；差异必须在备注解释）。
4. 全部数值字段有单位后缀；无单位字段数 0。
5. 衰减曲线单调不增且 `min` ∈ (0,1]；散布/后坐字段非负（schema 范围规则）。
6. 引用完整性：M1-007 的 13 个 loadout 引用的武器/手雷/近战 ID 100% 在 `data/weapons` 中存在（运行 M1-007 的引用检查，缺失数 0）。
7. `sim` 构建（`cargo build -p sim`）通过，`DATA_HASH` 稳定并写入 STATUS。

## 测试要求
- 单元测试：schema 正反例（缺 `falloff`、`mult>1`、单位错）；`diff-gd-tables`；melee 类字段必填。
- 集成：`cargo test -p sim data::` 加载全部武器无 panic。
- 性能基准：无。
- 回归：`DATA_HASH` 与武器数量快照。

## 性能预算
- 无（嵌入后 `GameData` 体积计入 `sim-wasm` ≤ 1.2 MB 预算，M0-014；新增 JSON 嵌入后体积变化记录于 STATUS）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 范围未列近战武器与弹药表，但 M1-013 的近战判定与 M1-012 的阻力参数需要它们；本任务一并创建（如认为过大，按 §5.6 拆 `M1-008b`）。
- 史实数值来自记忆（GD §0.4），不得在 `_source` 编造出处；复核前保持 `initial`。
- 武器 ID 以 GAMEDESIGN §0.3 为准（`us_` / `ger_` 前缀）；M0-008 若用了无前缀 ID（ARCH §8.5.1 示例 `m1_garand`），在 PLAN_ISSUES 登记并以已落地者为准统一。
