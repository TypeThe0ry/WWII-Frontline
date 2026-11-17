# M1-007 数据：M1 五兵种、loadout 与基础规则

## 元数据
- 里程碑：M1
- 工作流：data
- 规模：M
- 依赖：M0-004（INDEX 原文）；**隐含依赖 M1-008**（loadout 引用的武器/手雷 ID 须存在，见备注）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §5; GAMEDESIGN §4.4; GAMEDESIGN §3.2; GAMEDESIGN §10; GAMEDESIGN §12; ARCHITECTURE §8.5.2; ARCHITECTURE §8.5.3; ARCHITECTURE §4.4

## 背景
M1 的 5 个兵种 = **Officer、Rifleman、Medic、Support、Crewman**（US/GER 各一套，MILESTONES §2.1）。数值唯一真源是 `/data`（ARCHITECTURE §4.4），`sim` 的 `build.rs` 编译期嵌入并计算 `DATA_HASH`（M0-004）。所有游戏性数值状态 `initial`（`_meta.status`），经 M1-070/M1-071 复核后才升 `reviewed/measured`。命名遵 GD §0.3：兵种 `rifleman`；loadout `{faction}_{role}_{n}`（如 `us_rifleman_1`）；武器 `us_m1_garand`（**不用** ARCH §8.5.3 示例里的 `us_rifleman_default`/`m1_garand`，示例仅演示结构）。
M1 只收 **L1–L3 解锁**的 loadout（INDEX 范围）。GER 无坦克，故在 GER Rifleman 增加 `m1_only` 的 Panzerfaust 60 loadout 作反装甲手段（GD §5.1 之外的 M1 特例，完整 Anti-Tank 兵种在 M2）。

## 具体内容
### 要创建/修改的文件
- `data/roles/{officer,rifleman,medic,support,crewman}.json`（字段按 ARCHITECTURE §8.5.2 与 M0-004 已落地 schema）
- `data/loadouts/`：`us_officer_1`、`ger_officer_1`、`us_rifleman_1`、`us_rifleman_2`（L3）、`ger_rifleman_1`、`ger_rifleman_3`（L3）、`ger_rifleman_pf60`（L1，`m1_only`）、`us_medic_1`、`ger_medic_1`、`us_support_1`、`ger_support_1`、`us_crew_1`、`ger_crew_1`（共 13 个 JSON；内容取自 GD §5.5、§5.6、§5.9、§5.10、§5.15 的表）
- `data/gadgets/{bandage,aid_kit,binoculars,garrison_kit,ammo_crate_kit,supply_crate_carry,repair_toolbox}.json` + `data/schemas/gadgets.schema.json`（loadout 引用的非武器工具；GD §5.9 医疗包：3 m 半径、8 HP/s、10 s、3 次、放置 1 s；绷带见 GD §7.2）
- `data/rules/capture.json`、`spawn.json`、`supplies.json` + `data/schemas/rules.schema.json`
- `tools/data-validate/` 增加引用完整性规则：loadout→weapon/gadget、role→loadout、`m1_only` 标记
### 要实现的行为
1. **roles**：`max_per_squad`/`unlock.squad_size_min` 取 GD §4.4：Officer 1 / ≥2（且仅队长）；Rifleman 6 / 1；Medic 2 / 2；Support 1 / 3；Crewman 仅 Armor Crew（全队上限 8【待调参】，M1 两队各 1 辆坦克则每队 ≤ 3 人）。`movement` 取 GD §9.1：walk 2.0、run 4.4、sprint 6.4、crouch 2.4/1.5、prone 0.9/1.3（m/s）、`stamina_max 100`；`health 100`。
2. **loadouts**：每条含 primary/secondary/gadget/melee/ammo_mags、`unlock_level`、`unlock_condition`、`tags`（`m1_only`）。弹药记法按 GD §5.3（"N 弹匣含枪内一个"）：如 `us_rifleman_1` = Garand 8×10、Mk2×2、M1 刺刀。
3. **rules/capture.json**：`k_cap_pct_per_s 0.20`、`eff` 分段（≤4 人满权重、4–10 人 0.5、上限 10 人净优势）、`regress_pct_per_s 0.15`、`height_tolerance_m 6`、`default_radius_m 50`（据点实际半径取地图数据：L1a 45 / L2a 55 / L3a 50，MAPS §3.6）、载具内不计入（GD §3.2）。
4. **rules/spawn.json**：波次 15 s；`base_wait_s`：HQ 5、Garrison 8；出生保护 3 s（移动/射击即取消，仅 HQ/据点内有效）；Garrison：24 槽、每 5 s 恢复 1、1200 HP、建造 20 s、成本 己方·中立 50 / 敌方 100 Supplies、间距限制（己方 ≥150 m、敌方 Garrison ≥250 m、据点圆心 ≥ R_sp+100 m、HQ ≥200 m）、封锁半径 30 m、Overrun 20 m/25 s（GD §10.2、§10.6）。
5. **rules/supplies.json**：Supply Crate 100/箱、HP 150、地面上限 40；HQ Depot 45 s 一箱/库存 8；据点补给点 90 s/库存 4；Ammo Crate 600 点、单价（手枪匣 1/冲锋枪匣 3/步枪夹 4/手雷 6/绷带 2）、2 个/命、放置冷却 30 s、寿命 150 s（GD §12.2、§12.3）。
6. 每个文件 `_meta.status = "initial"`，`_meta.tunable` 列出【待调参】字段，`_meta.calibration_ref` 指向 REFERENCE_MEASUREMENTS 章节（无则写"待定"）。
### 接口与数据
```json
{ "id":"us_rifleman_1", "role":"rifleman", "faction":"us", "unlock_level":1,
  "primary":"us_m1_garand", "secondary":null,
  "gadget_1":{"id":"us_mk2","count":2}, "gadget_2":{"id":"bandage","count":2},
  "melee":"us_m1_bayonet", "ammo_mags":10, "tags":[], "_meta":{"status":"initial"} }
```

## 不做什么
- 其余 9 个兵种与 L4+ loadout：M2/M3。
- 武器/手雷/Panzerfaust 本身的数值：M1-008；Sherman：M1-009。
- 兵种等级/XP 结算：M2；规则在 `sim` 内的实现：M1-016..021。
- 史实复核与对标回填：M1-070、M1-071。

## 验收标准
1. `tools/data-validate` 对全部新增 JSON 通过（0 错误）；CI `data-validate` 作业 < 2 min（ARCHITECTURE §17.5）。
2. 引用完整性：13 个 loadout 的 primary/secondary/gadget/melee 引用 ID 100% 存在（`data-validate` 报告引用数 N 与缺失数 0）。
3. 5 个 role 的 `max_per_squad`/`squad_size_min` 与 GD §4.4 表逐项一致（单测 `roles_match_gd_4_4` 读取 JSON 断言 5×2 项）。
4. loadout 弹药/手雷数与 GD §5.5/5.6/5.9/5.10/5.15 一致（脚本逐条比对，抽检不少于 13 条主武器弹匣数）。
5. 缺字段/错单位的样例（`data/schemas/__fixtures__/bad_*.json` ≥ 5 个）必报错（沿用 M0-004 测试风格）。
6. `m1_only` 标记仅出现在 `ger_rifleman_pf60`；`sim` 构建后 `DATA_HASH` 稳定（同内容两次构建相同）。
7. 所有数值带单位后缀（`_m/_ms/_mps/_s`）；无单位数值字段数为 0（validate 的单位规则）。

## 测试要求
- 单元测试：schema 校验正反例；引用完整性；名额一致性（对照 GD §4.4 表的内嵌期望常量）。
- 集成：`cargo build -p sim` 能通过 `build.rs` 嵌入并解析新 JSON。
- 性能基准：无。
- 回归：`DATA_HASH` 值写入 STATUS（后续任务以此判断数据变更）。

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
- **依赖问题**：INDEX 只列 M0-004，但 loadout 引用的武器/手雷在 M1-008（编号更大）。执行方式：先写 roles/rules/gadgets，loadout 的引用完整性检查在 M1-008 完成后再做；本任务在此之前最多标 `partial`（遗留：引用完整性）。建议总控把 M1-008 加入依赖。
- `data/gadgets`、`data/rules` 不在 M0-004 的 schema 清单内，属"细化"，需在 PLAN_ISSUES 登记（ARCHITECTURE §8.5 未定义这两类文件）。
- ID 命名以 GAMEDESIGN §0.3 为准，ARCHITECTURE §8.5.3 示例的 `*_default` 仅作结构示例（PLAN_ISSUES 登记文档不一致）。
- 数值全部 `initial`；校准来源 REFERENCE_MEASUREMENTS（占领速度 `k_cap`、出生计时等标【待测】）。
