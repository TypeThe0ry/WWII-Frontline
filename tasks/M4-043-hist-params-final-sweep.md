# M4-043 史实参数最终清扫（R8）

## 元数据
- 里程碑：M4
- 工作流：data
- 规模：S
- 前置依赖：M3-051, M3-047
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §6; GAMEDESIGN §0.4; REFERENCE_MEASUREMENTS §6.1; REFERENCE_MEASUREMENTS §6.3; ARCHITECTURE §8.5

## 背景
R8：GAMEDESIGN 的史实参数出自记忆、未联网复核，必须由专门任务按 REFERENCE_MEASUREMENTS §6 的史料来源逐项复核；**复核完成前相关数值在 `/data` 中状态一律为 `initial`**（GAMEDESIGN §0.4）。M1-070/M2-053/M3-051 已分批复核；本任务做**最终清扫**：统计全部 `/data` 条目的 `historical` 字段状态，剩余 `initial` / "未核实" 逐项处理或登记。
取舍规则（REF §6.1）：外形/口径/弹种/弹匣容量/重量/全长/枪管长 → 以史实为准；理论射速、初速、有效射程 → 史实为第一来源，游戏性偏离须 `deviation_reason`；手感类时序 → 对标测量（`source: ref_measurement`）；伤害/致死发数/冷却/资源 → 对标 + 平衡；载具最高速度/战斗全重/火炮口径/弹种 → 史实初值，游戏节奏以对标为准。
数据字段（REF §6.3）：每把武器 `historical{caliber,cartridge,mag_capacity,cyclic_rpm,muzzle_velocity_mps,mass_kg,length_mm,source_refs[]}`、`ref_measurement{…}`、`gameplay{…,deviation_reason,tuning_status: initial|calibrated|locked}`。
来源（REF §6.2，核实日期 2026-11-17，**页内数字需执行者打开原文核对并记录页码**）：TM 9-1275（M1 Garand）、FM 23-5（1951，仅交叉参考）、TM 9-1205（Browning MG）、TM 9-2200、HyperWar TM 索引、TM-E 30-451（德军手册，战时情报可能不准确）、德军条令/Merkblatt（**未在线核实到可靠链接**）、Hogg 系列（出版物，仅读数字事实，不复制文字）、博物馆馆藏（未检索具体条目）。
纪律：只提取数值事实，不复制整段文字，不上传原件；任何原文复制进仓库前须人类确认（REF §6.2 注）。

## 具体内容
### 要创建/修改的文件
- `tools/data-validate/src/historical_audit.rs`（或 TS/Python 按 M0-004 工具栈）—— 遍历 `/data/weapons|vehicles|…`，统计每个 `historical` 字段的 `status` 与 `source_refs`。
- `reports/data/m4-historical-audit.md` + `reports/data/m4-historical-audit.json` —— 状态统计（总字段数、`reviewed`、`initial`、`未核实`）与逐项处理清单。
- `data/**/*.json` —— 对剩余项：补 `source_refs`（含页码）、更新 `status`、必要时修正数值并写 `deviation_reason`。
- `docs/PLAN_ISSUES.md` —— 对无法核实的项追加条目（只追加）。
- `tools/data-validate` 规则：新增 CI 校验"无 `initial` historical 字段，或有 PLAN_ISSUES 编号字段 `issue_ref`"。
### 要实现的行为
1. 审计：输出全表统计与 diff（对比 M1-070/M2-053/M3-051 的结论）。
2. 逐项处理：对每个 `initial` 项，优先用 REF §6.2 的在线可读来源核对；核对得到则改 `reviewed` 并写来源与页码；核对不到则保持 `initial` + `issue_ref`。
3. 冲突处理：史实与已调参数值冲突属"外形/口径/弹容量/射速"→ 以史实为准；属手感类 → 保留对标并写 `deviation_reason`；争议项登记（H-030 人负责裁决）。
4. 变更影响：任何数值变更都触发 `DATA_HASH` 变化；对 `locked` 项的变更须注明并跑回归断言（REF §7.5）；如影响平衡，通知 M4-039（写 STATUS 备注）。
5. 抽样复核：对 ≥ 10% 的 `reviewed` 项交人类抽样（H-034），记录通过率。
6. 禁止编造：无法联网/无来源时不得填数；标 `未核实` 并登记。
### 接口与数据
- 状态枚举：`historical.status ∈ {initial, reviewed, disputed}`；`issue_ref: "PI-###"`；`source_refs[]: {id, page, note}`。若 schema 尚无这些字段，按 README §5.1 的"细化"规则补 schema 并登记。

## 不做什么
- 不做对标测量（M4-044）；不做新数值调参（M4-039/M3-047）。
- 不复制原文段落；不引入未登记的来源。
- 不新增武器/载具（范围外）。

## 验收标准
1. **`/data` 中无状态为 `initial` 的 historical 字段，或每个遗留项均有 PLAN_ISSUES 编号**：审计脚本退出码 0；统计报告列出 `initial` 剩余数及全部带 `issue_ref`（缺 `issue_ref` 的 `initial` 数 = 0）。
2. 审计报告含总字段数、`reviewed`/`initial`/`disputed` 计数与百分比；`reviewed` 的 `source_refs` 含页码比例 = 100%。
3. 对 `locked` 项的数值变更数 = 0，或每项变更有回归断言更新与理由（逐项列出）。
4. `data-validate` CI 作业通过；`DATA_HASH` 在变更前后各记录一次。
5. 抽样复核：≥ 10% `reviewed` 项交人类复核，记录抽样数与通过率（缺人类则 `未测`）。
6. 争议项（`disputed`）数量与说明列表，不超过全部项的 5%（超过须说明）。

## 测试要求
- 单元测试：审计工具对合成数据（含 initial/无页码/缺 issue_ref）的判定。
- 集成/无头测试：`cargo test -p sim`（数据变更后全部回归断言通过，尤其 `locked` 项断言，REF §7.5）。
- 性能基准：无。
- 回归：`DATA_HASH` 变化需随之更新基线与 `headless-client` 握手版本（`data_hash` 变更视为版本变更，§18.5）。

## 性能预算
无。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `data-validate`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖 H-031（来源合法性确认）、H-030（对标与史实冲突裁决规则）、H-062（史实参数复核抽样）；缺少人类确认则 `reviewed` 的置信度上限为 B（H-034）。
- 德军条令一手资料"未在线核实到可靠链接"，可能长期留 `initial`+`issue_ref`；不得以 Wikipedia 之类二手站点替代而不标注。
- 战后版手册（FM 23-5 1951、TM 9-1005-222-12 1969）只作交叉参考，不作战时数据唯一来源。
