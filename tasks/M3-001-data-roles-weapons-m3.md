# M3-001 数据：M3 全兵种补全与剩余武器

## 元数据
- 里程碑：M3
- 工作流：data
- 规模：M
- 依赖：M2-006
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §5.4; GAMEDESIGN §5.14; GAMEDESIGN §5.16; GAMEDESIGN §5.17; GAMEDESIGN §6.2; GAMEDESIGN §6.3; GAMEDESIGN §6.5; GAMEDESIGN §4.4; ARCHITECTURE §8.5

## 背景
M0–M2 已落地 Officer/Rifleman/Medic/Support/Crewman（M1-007）与 Assault/AR/MG/AT/Engineer（M2-006）的 roles 与 loadout，以及对应武器。M3 的目标是 **14 兵种 × 2 阵营全部可选**（MILESTONES E3-1），本任务补齐四个新兵种 Commander（GD §5.4）、Tank Commander（§5.14）、Spotter（§5.16）、Sniper（§5.17）以及此前 loadout 引用到、但 `/data/weapons/` 尚不存在的全部武器。
数值第一来源是史料；游戏性数值只写 GD §6.3/§6.5 的初值，状态一律 `initial`（R8），史实复核由 M3-051 完成。`/data` 是数值唯一真源（ARCHITECTURE §4.4），客户端不硬编码数值。
关键事实：GD 的 loadout 表实际只到 L8（Officer/MG/AT/Engineer 的 `_3`），XP 公式（GD §5.2）才到 L10；INDEX 写"L1–L10 全部"指"覆盖到表内出现的全部等级"，**不得自行发明 L9/L10 loadout**，缺口写 `docs/PLAN_ISSUES.md`。

## 具体内容
### 要创建/修改的文件
- `data/roles/{commander,tank_commander,spotter,sniper}.json` —— 字段按 ARCHITECTURE §8.5.2；名额按 GD §4.4：Commander team_cap 1；Tank Commander team_cap 4（=车组数上限）；Crewman 8（已有，复核）；Spotter/Sniper team_cap 2、仅 Recon Team。
- `data/loadouts/{us,ger}_{commander_1,commander_2,tc_1,tc_2,spotter_1,spotter_2}.json` 以及 `us_sniper_{1,2}`、`ger_sniper_{1,2,3}` —— 严格对应 GD §5.4/§5.14/§5.16/§5.17 表（共 4+4+4+5=17 个新 loadout）；解锁等级取表值（如 ger_sniper_2 为 L3，ger_sniper_3 为 L6）。
- `data/weapons/*.json` —— 对照 GD §6.2/§6.3 全表**做差集**，补建缺失项。预期缺失（以仓库实际为准）：`us_m1c_garand`、`us_m1903a4`、`us_m3_grease`、`ger_g43`、`ger_g43_zf4`、`ger_fg42`、`ger_mg34`、`ger_kar98k_zf39`、`ger_kar98k_zf41`、`vehicle_*`（M2HB/MG34/MG42/M1919，若 M3-002 车辆引用则由 M3-002 建，避免重复）。
- `data/roles/*.json` 既有文件中 `abilities[]` 补入新能力 id（`place_garrison` 仅 commander/officer/spotter，R1；`place_op` 仅 spotter；`mark_target_10x` spotter、`mark_target_4x` officer；`breath_hold` sniper）。
### 要实现的行为
1. 每个武器 JSON 含 `historical`（口径/供弹/理论射速/初速/全重，取 GD §6.2，`_source` 写"GD §6.2 初值，待 M3-051 复核"）与 `gameplay`（GD §6.3 的 dmg、F0/F1/min、eff、装填秒数、ADS 时间、pen_power；GD §6.5 的后坐表）。
2. 狙击/带镜武器写 `sight.zoom`：M1903A4 M73 2.5×、M1C M81 2.2×、Zf39 4×、ZF41 1.5×、G43 ZF4 4×（GD §6.3）；栓动写 `cycle_time_s`（M1903A4 1.6 s、Kar98k 1.5 s、Zf41 1.55 s）。
3. 所有 `【待调参】` 字段登记进该文件 `_meta.tunable`，`calibration_ref` 指向 REFERENCE_MEASUREMENTS 对应节（可 Grep 确认；查不到写"待定"）。
4. loadout 弹药表示"N 弹匣含枪内一个"（GD §5.3），换算成 `ammo_mags` 时在注释字段 `_note` 写明算法。
5. 名额表：写一个 `tools/data-validate` 可调用的检查（或新增校验规则）——对 GD §4.4 表逐行断言 `team_cap/squad_cap/min_squad_size`。
### 接口与数据
- 只新增 JSON 与校验规则，不改 schema 结构；若 schema 缺字段（如 `sight.zoom`、`abilities[]` 枚举）则在 `data/schemas/` 追加并在 PLAN_ISSUES 记"细化"。

## 不做什么
- 载具与载具武器本体 → M3-002。
- 指挥官能力与火炮数据 → M3-003。
- 兵种特有规则代码（望远镜、OP 出生、名额逻辑）→ M3-010/011/012。
- 史实参数逐项复核 → M3-051；游戏性调参 → M3-047。
- 不发明 GD 未定义的 loadout 或 L9/L10 解锁内容。

## 验收标准
1. `tools/data-validate` 对全 `/data` 通过，退出码 0；14 兵种 × 2 阵营每个组合至少有 1 个 loadout（脚本断言，输出矩阵）。
2. loadout 总数与 GD §5.4–§5.17 表行数逐项一致（脚本对比 ID 列表，差异 0）。
3. 所有 loadout 引用的武器/手雷/工具 ID 在 `/data` 中均存在（引用完整性，错误数 0）。
4. GD §4.4 名额表 14 行全部通过断言。
5. 每个新武器 JSON 的 dmg/F0/F1/min/eff/pen_power/ADS 与 GD §6.3 对应行逐字段一致（脚本抽检全部新增武器）。
6. 每个新增文件状态标记为 `initial`，且 `_meta.tunable` 非空；`DATA_HASH` 对同内容两次构建一致。
7. Garrison 放置能力只出现在 commander/officer/spotter 三个 role（R1，grep 断言 Engineer 无 `place_garrison`）。

## 测试要求
- 单元测试：`tools/data-validate` 的名额断言与引用完整性用例各至少含 1 个故意错误的样例（缺武器 ID、超名额）必报错。
- 集成：`cargo build -p sim` 的 `build.rs` 嵌入新数据并重新计算 `DATA_HASH`，`cargo test -p sim data::` 通过。
- 性能基准：无。
- 回归：更新 `DATA_HASH` 基线快照（如存在）。

## 性能预算
无（纯数据）。不得使 §4.3.2 `sim-wasm` 体积预算（≤ 1.2 MB / br 400 KB）回归：构建后检查 wasm 体积并写入 STATUS。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §5.5/§5.12 表中部分武器（如 StG 44、FG 42）可能已在 M2-006 建过；先做差集再补，不重复建。
- Commander loadout 的"Garrison 套件""无线电（指挥面板）"是 tool 标记而非武器，写入 `gadget`/`tools[]` 字段。
- INDEX 写"L1–L10"与 GD 表（最高 L8）不符，见背景；需 Fable 确认是否要补 L9/L10。
