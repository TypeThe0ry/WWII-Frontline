# M0-031 史实参数复核：Kar98k 与 M1 Garand（R8）

## 元数据
- 里程碑：M0
- 工作流：data
- 规模：S
- 依赖：M0-008
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §6; GAMEDESIGN §0.4; GAMEDESIGN §6.2; BRIEF R8

## 背景
总控裁定 R8：GAMEDESIGN 的史实参数出自作者记忆、**未联网复核**（GAMEDESIGN §0.4），必须有任务按 REFERENCE_MEASUREMENTS §6 的史料来源逐项复核 `/data` 的武器与载具参数；**复核完成前，相关数值状态一律为 `initial`**。本任务是 R8 的第一批：只复核 M0 的两把枪（`us_m1_garand`、`ger_kar98k`）的 `historical.*` 字段。
取舍规则（REFERENCE_MEASUREMENTS §6.1）：外形、口径、弹种名、弹容量、全重、全长 → **史实为准**；理论射速、枪口初速、有效射程 → 史实为第一来源，作为 `/data` 初值，游戏性有限度偏离须写 `deviation_reason`；手感类时序（开镜、切枪、换弹动画）→ 对标测量，不在本任务。权威来源（§6.2，核实日期 2026-11-17，**执行者须打开原文核对页码**）：TM 9-1275《U.S. Rifle, Cal. .30, M1》（1942/1947 版转写，biggerhammer.net 与 GPO 扫描）、FM 23-5（1951，战后，仅交叉参考）、TM 9-1005-222-12（1969，战后）、TM-E 30-451《Handbook on German Military Forces》（1945，Lone Sentry 转录，战时情报可能不准确）、HDv/Merkblatt 德军条令（**未在线核实到可靠链接**）、Hogg 系列出版物（需馆藏）、博物馆馆藏（Royal Armouries/Smithsonian/IWM，未检索具体条目）。**纪律**：只提取数值事实，不复制整段文字、不上传原件；版权不明的转录文本不入库（NEEDS_HUMAN H-031 决定边界）。

## 具体内容
### 要创建/修改的文件
- `data/weapons/us_m1_garand.json`、`data/weapons/ger_kar98k.json`：把 `historical._source` 由字符串改为结构化数组（schema 同步在 `data/schemas/weapon.schema.json` 中允许数组；M0-004 若已定为字符串，本任务改 schema 并记 PLAN_ISSUES）：`[{field, value, unit, source_id, title, locator(页码/章节), url, accessed:"YYYY-MM-DD", note}]`。
- `tools/data-validate/rules/historical.mjs`：规则 `hist-source-required`：当 `_meta.status ∈ {reviewed, measured}` 时，`historical` 下每个数值字段必须有 `_source` 条目，或在 `_meta.unverified[]` 中以"未核实"登记；`status=initial` 不强制。
- `docs/PLAN_ISSUES.md`：追加条目（仅追加）：与 GAMEDESIGN §6.2/ARCHITECTURE §8.5.1 示例冲突的数值差异（如初速 853 vs 850）、需要人类裁决的来源争议。
### 要实现的行为
1. 逐字段复核（Garand / Kar98k 各）：`caliber`/`cartridge`、`magazine_capacity`、`magazine_type`、`muzzle_velocity_mps`、`rate_of_fire_rpm_practical`、`effective_range_m`（若写入）、`mass_kg`、`length_mm`（若 schema 含）。每个字段：找到 ≥ 1 个来源并记录页码/章节；两个来源冲突时按 REFERENCE_MEASUREMENTS §5.3 冲突优先级处理并在 `note` 说明；数值差异 > 5% 时更新 `/data` 值并保留旧值于 `note`。
2. 本任务**只改 `historical` 块**；`gameplay` 数值（伤害、射速上限、装填时间）不变，除非复核发现数据**单位/弹种**写错（此时改并在 PLAN_ISSUES 说明）。
3. 若某 `gameplay` 值偏离史实（如游戏射速 cap 120 rpm vs 史实实用 40 rpm），在对应字段旁补 `deviation_reason`（如"半自动节奏上限，GAMEDESIGN §6.3"），不改数值。
4. 无法联网核实的字段：保持原值，登记 `_meta.unverified[]` 并写"未核实"；**绝不编造出处或页码**。
5. 状态：所有 `historical` 字段均有出处或"未核实"登记后，`_meta.status` 由 `initial` 改为 `reviewed`；`gameplay` 仍待调参（`calibration_ref` 不变）。`/data` 校验通过。
### 接口与数据
- 无代码接口；`DATA_HASH` 会变化（预期）。`source_id` 取 REFERENCE_MEASUREMENTS §6.2 的来源名缩写（如 `TM9-1275`、`TME30-451`）。

## 不做什么
- 不复核其它武器与载具（M1-070、M2-053、M3-051、M4-043）；不改游戏性数值（M0-032 回填、M2/M3 调参）。
- 不下载/存档来源原件进仓库（只存数值事实与定位信息）。
- 不决定"对标与史实冲突"的裁决规则（NEEDS_HUMAN H-030，人类裁定）。

## 验收标准
1. 两个文件的 `historical` 下**每个数值字段 100%** 满足：有 ≥ 1 条 `_source`（含 `locator`）或被列入 `_meta.unverified[]`；`node tools/data-validate/index.mjs` 退出码 0（规则 `hist-source-required` 开启）。
2. 复核记录汇总表（字段 × 旧值 × 新值 × 来源 × 状态）写入 STATUS 或 `reports/m0/hist-review-m0.md`，行数 = 两把枪的 historical 字段总数，且"已核实/未核实"计数相加等于字段总数。
3. 至少 1 个字段由**非 GAMEDESIGN/记忆**的来源（TM/FM/博物馆馆藏页）支持；若全部无法核实（如无网络），任务状态为 `partial`，`_meta.status` 保持 `initial`，原因写 PLAN_ISSUES。
4. 任何数值改动均同步更新依赖测试（M0-008 的 STK 衍生检查、M0-012 的弹道测试）并全部通过：`cargo test -p sim` 全绿。
5. 无来源原文被复制入仓库：`git diff` 中不含超过 20 个连续英文单词的原文句子（人工抽查 + 简单脚本）。

## 测试要求
- 单元测试：`hist-source-required` 规则的正反例（`tools/data-validate/test/`）。
- 集成/无头测试：`cargo test -p sim`、`data-validate` CI。
- 性能基准：无。
- 回归：M0-012 的弹道/射速单元测试如因数值更新而变，更新期望值时必须引用本任务来源。

## 性能预算
无。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- REFERENCE_MEASUREMENTS §6.2 的链接均为检索结果，访问可用性与页内数字以执行当时为准；德军条令（HDv/Merkblatt）**未核实到可靠链接**，Kar98k 的部分字段可能只能引用 Hogg/博物馆资料或标"未核实"。
- GAMEDESIGN §0.4 已声明：执行者据可靠来源修订 `/data` 无需回评审，但须在 `_source` 记录出处；本任务据此执行。
- 相关人类事项：来源合法性与是否可存转录文本（NEEDS_HUMAN H-031）；史实容忍度见 H-046（M1 前）。
