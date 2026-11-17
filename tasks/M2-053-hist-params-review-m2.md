# M2-053 史实参数复核：M2 武器与载具（R8）

## 元数据
- 里程碑：M2
- 工作流：data
- 规模：M
- 前置依赖：M2-006
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §6.1; REFERENCE_MEASUREMENTS §6.2; REFERENCE_MEASUREMENTS §6.3; GAMEDESIGN §6.2; GAMEDESIGN §14.3; GAMEDESIGN §6.1; GAMEDESIGN §0.4

## 背景
总控裁定 R8：GAMEDESIGN 中的史实参数出自记忆、未联网复核，必须有任务"按 REFERENCE_MEASUREMENTS §6 的史料来源逐项复核 `/data` 武器与载具参数"，复核前相关数值状态为 `initial`。M1-070 复核了 M1 武器与 Sherman；本任务复核 **M2-006 新增**的全部条目（以 `/data` 目录相对 M1 的差异为准，预计包括：BAR M1918A2、M1919A4/A6、MG 42、（MG 34/StG 44 若已加入）、M1897 霰弹枪、M1 Carbine/M1903A4（若已加入）、Bazooka M1A1、Panzerschreck、Panzerfaust 60、AP/AT 地雷、Satchel/Geballte Ladung，载具 Panzer IV Ausf. H、Willys Jeep、Kübelwagen、GMC CCKW、Opel Blitz）。
取舍规则（REFERENCE_MEASUREMENTS §6.1）：外形、口径、弹种名称、弹匣/弹夹容量、重量、全长、枪管长以**史实**为准；理论射速、枪口初速、有效射程以史实为第一来源作为 `/data` 初值，允许有限度偏离且必须写 `deviation_reason`；手感类时序以对标测量为准；伤害/致死发数/冷却/资源以对标+平衡；载具最高速度/战斗全重/火炮口径/弹种取史实为初值（GAMEDESIGN §14.3：游戏最高速 = 史实路速 × 0.75）。
权威来源（§6.2，核实日期 2026-11-17，**多数仅确认检索结果存在、未核对页内数字**）：TM 9-1275（M1 Garand）、FM 23-5、TM 9-1205（Browning 机枪）、TM 9-2200、HyperWar TM 索引、TM-E 30-451（德军手册，可能不完整/不准确）、德军 HDv/Merkblatt（未在线核实）、Hogg 系列（需馆藏，未核实）、博物馆馆藏（未检索具体条目）。数据字段（§6.3）：`historical`（`caliber`、`cartridge`、`mag_capacity`、`cyclic_rpm`、`muzzle_velocity_mps`、`mass_kg`、`length_mm`、`source_refs[]`）、`ref_measurement`、`gameplay`（含 `deviation_reason`、`tuning_status`）。

## 具体内容
### 要创建/修改的文件
- `data/weapons/*.json`、`data/vehicles/*.json`（M2-006 新增条目）—— 为每个 `historical` 字段填写 `source_refs[]`（来源 ID + 页码/节号 + 访问日期）；核实不了的字段标记 `"verified": false` 并在 `source_refs` 写 `"未核实"` 与原因；核实通过后 `tuning_status` 由 `initial` 改为 `reviewed`（仅限 `historical` 字段组；`gameplay` 仍待对标，保持 `initial`）。
- `data/sources/sources.json` —— 来源注册表（ID、标题、类型、URL（仅取自 REFERENCE_MEASUREMENTS §6.2 已列出的链接，或执行者本次实际访问成功的链接）、访问日期、可靠性备注、版权状态）；**不复制原文段落，只记录数值事实**。
- `tools/data-validate/src/check_sources.rs`（扩展 M0-004）—— 新增 `--check-sources`：M2 范围内每个 `historical.*` 字段必须有 `source_refs`（含页码）或显式 `unverified`；引用的来源 ID 必须存在于 `sources.json`。
- `data/review/m2_review_log.json` —— 复核记录：每个字段 {旧值, 复核值, 来源, 是否改动, 改动原因}；偏离史实的 `gameplay` 值必须有 `deviation_reason`。
### 要实现的行为
1. 逐项核对 GAMEDESIGN §6.2/§14.3 的数值（例如 BAR 慢约 350/快约 550 rpm、M1919A4 约 500 rpm、MG 42 约 1 200 rpm、MG 34 约 850 rpm、StG 44 7.92×33 mm Kurz 30 发、Panzerfaust 60 初速约 45 m/s、Panzerschreck 88 mm、Panzer IV H 的 7.5 cm KwK 40 L/48 与装填/弹药量等）与 `/data` 取值是否一致，并与来源比对。
2. 来源冲突按 §6.1 规则处理；冲突无法裁决时在 `m2_review_log.json` 标 `needs_human`（对应 H-062 抽样复核/H-031 来源合法性）。
3. 无法联网或来源页无法打开：该字段标 `unverified`，**不得编造页码、链接或数值**。
4. 不改 `gameplay` 数值（对标测量归 M2-054），除非史实复核导致"外形/口径/弹容量"类硬事实错误（此时修正并记录）。
5. 重新计算 `DATA_HASH`；通知依赖方（sim 嵌入数据，CI `data-validate` 通过）。
### 接口与数据
```json
"historical": { "cyclic_rpm": 500, "mag_capacity": 250, "source_refs": [ { "src": "TM9-1205", "loc": "§…/p.…", "accessed": "2026-11-17", "note": "" } ], "verified": true }
```

## 不做什么
- 不做 M1 武器/Sherman 复核（M1-070）；不做 M3 新增条目复核（M3 的 hist 任务）。
- 不做对标测量与 `gameplay`/`ref_measurement` 调参（M2-054）。
- 不把任何手册扫描件/文本复制进仓库（H-031：默认只存数值事实）。
- 不改 docs（若发现 GAMEDESIGN 表有错，写 PLAN_ISSUES，由 Fable 回写）。

## 验收标准
1. 覆盖率：`tools/data-validate --check-sources` 对 M2 新增的全部武器/载具条目，`historical.*` 字段 100% 有 `source_refs`（含页码/节号）或显式 `unverified`（脚本退出码 0；输出覆盖率 = 100%）。
2. 来源完整性：`source_refs` 引用的来源 ID 100% 存在于 `data/sources/sources.json`；`sources.json` 的 URL 全部可追溯到 REFERENCE_MEASUREMENTS §6.2 或本次实际访问记录（人工抽检 10%，H-062）。
3. 复核日志：`data/review/m2_review_log.json` 中每个被检字段有记录；统计 `已核实/未核实/已修正` 三类数量并写入 STATUS（数量以实测为准）。
4. 偏离说明：所有 `gameplay` 与 `historical` 不一致的字段都有非空 `deviation_reason`（脚本检查，0 例缺失）。
5. `/data` 校验与 `DATA_HASH`：`data-validate` 通过；`cargo test -p sim` 通过（数值变更未破坏单测，若破坏则更新测试并记录原因）。
6. 载具游戏最高速 = 史实路速 × 0.75 的公式一致性检查通过（脚本逐车对比，误差 < 1%）。

## 测试要求
- 单元测试：`check_sources` 的正反例（缺来源、非法来源 ID、`unverified` 合法）；`DATA_HASH` 稳定性。
- 集成/无头测试：CI `data-validate` 作业；`cargo test --workspace` 绿。
- 性能基准：无。
- 回归：M1-070 复核结果不得被覆盖（只对 M2 新增条目改动）；`tuning_status` 状态迁移单测（`initial → reviewed`）。

## 性能预算
- 无（数据任务）；`data-validate` 作业 < 2 min（ARCH §17.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`、`data-validate`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 来源可访问性是主要风险：§6.2 多数条目"仅确认检索结果存在"；德军条令与 Hogg 需馆藏/购买（H-031），无法访问的字段只能 `unverified`，任务可 `done`（验收只要求"有出处或标未核实"），但 STATUS 要写未核实字段数量。
- 人类抽样复核：H-062（史实参数复核抽样，BRIEF R8）。
- 德军手册 TM-E 30-451 为战时情报手册，可能不准确，不得作为唯一来源。
- 若 M2-006 的 `/data` 目录结构与本文路径不同，以实际目录为准并在备注记录。
