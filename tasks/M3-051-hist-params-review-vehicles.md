# M3-051 史实参数复核：全部载具与剩余武器（R8）

## 元数据
- 里程碑：M3
- 工作流：data
- 规模：L
- 前置依赖：M3-002, M3-001
- 素材需求：无；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §6.1; REFERENCE_MEASUREMENTS §6.2; REFERENCE_MEASUREMENTS §6.3; GAMEDESIGN §0.4; GAMEDESIGN §6.2; GAMEDESIGN §14.2; GAMEDESIGN §14.3; GAMEDESIGN §14.4

## 背景
总控裁定 R8：GAMEDESIGN 中的史实参数出自记忆、未联网复核，必须逐项按 REF §6.2 的权威来源复核；复核前相关数值状态为 `initial`。M0-031、M1-070、M2-053 已复核 Kar98k/M1 Garand、M1 武器与 Sherman、M2 武器与载具。本任务复核**全部载具与剩余武器**：
- 载具（GD §14.3）：M8、M5A1、M4A1(75)、M4A3(76)W、M4A3E2 Jumbo、CCKW、M3 半履带、Jeep；Puma、Luchs、Pz IV H、Panther G、Tiger I、Opel Blitz、Sd.Kfz.251、Kübelwagen。复核字段：乘员、主炮口径/型号、装甲板厚/倾角（GD §14.3.3 表）、质量、最高路速、炮塔转速/俯仰（GD §14.5.4）、弹药量、炮弹初速（GD §14.2 表）。
- 剩余武器（GD §6.2 表）：G43、FG42、MG34、M1903A4、Kar98k+ZF41、M3 Grease Gun、Bazooka、Panzerschreck、Panzerfaust 60、载具机枪等（以 M2 已复核之外的 `/data/weapons/*.json` 为准，逐文件列清单）。
权威来源（REF §6.2，核实日期 2026-11-17）：TM 9-1275（M1）、FM 23-5、TM 9-1205（M1919）、TM 9-2200、HyperWar TM 索引、TM-E 30-451（德军手册，情报性质可能不准）、HDv/Merkblatt（**未核实**，需检索核验合法性）、Hogg 等出版物（需馆藏，**未核实**）、博物馆馆藏（未检索，**未核实**）。公共领域声明：仅提取数值事实、不复制整段文字、不上传原件；把转录文本放入仓库须人类确认（H-031）。
取舍规则（REF §6.1）：外形/口径/弹容量/重量以史实为准；射速/初速/有效射程以史实为第一来源；手感类时序与伤害/冷却/资源以对标测量 + 平衡为准；载具游戏节奏（加速/转向/装填）以对标为准，偏离史实必须写 `deviation_reason`。

## 具体内容
### 要创建/修改的文件
- `data/vehicles/*.json`、`data/weapons/*.json` —— 只改 `historical` 组与其状态/出处字段；`historical.source_refs[]`（来源 ID + 页码/章节）、`_source`（未核实时写 `"未核实"`）、`review_status: "reviewed"|"unverified"|"conflict"`。
- `data/sources/sources.json` —— 来源登记表（来源 ID、标题、版次、URL（仅真实访问过的）、访问日期、可靠性备注）；沿用 M0-031 已建立者。
- `data/review/m3-historical-review.md` —— 逐项复核表：`文件, 字段, 原值, 复核值, 来源 ID+页码, 结论(一致/修正/未核实/冲突), 备注`。
- `tools/data-validate/`（改）—— 增加 `--check-historical-coverage`：统计 `historical` 字段的出处覆盖率与状态。
- `docs/PLAN_ISSUES.md` —— 仅追加冲突项条目（史实与 GD 表冲突、来源间冲突）。
### 要实现的行为
1. 先列清单：脚本生成所有载具与武器的 `historical` 字段列表（目标 ≥ 16 辆载具 × 约 8 字段 + 武器若干）。
2. 对每个字段在 REF §6.2 来源中查证：能打开原文并读到数值 → 记录来源 ID + 页码；打开不了或页面内无数值 → 标 `"未核实"`；**严禁编造出处/页码/链接**。
3. 冲突处理：史实值与 GD 表不一致时，若属于外形/口径/容量/射速/初速/质量/装甲板厚类，更新 `historical` 为复核值，并检查依赖它的 `gameplay` 字段（只提示，不自动改）；写 PLAN_ISSUES，注明受影响任务（M3-047 调参、M3-013 判定）。
4. 游戏标称值（`P100`、`drop`、结构伤害）不是史实数据，不改，仅在日志注明"游戏标称"。
5. 状态：`initial` → `reviewed`（仅当出处完整）；有"未核实"字段的条目保持 `initial` 并在日志列出。
6. 装甲倾角（GD §14.3.3）为"游戏实现用取整值"，复核时与史实对照，差异 > 2° 记入日志，由 M3-013 复核判定是否影响。
### 接口与数据
- `historical` 字段结构沿用 REF §6.3：`caliber, cartridge, mag_capacity, cyclic_rpm, muzzle_velocity_mps, mass_kg, length_mm, source_refs[]`；载具：`crew, main_gun, armor_mm[...], mass_kg, max_speed_kph, source_refs[]`。

## 不做什么
- 游戏性数值的调参（M3-047）与对标测量（M3-052）。
- 全局最终清扫（M4-043）；M0/M1/M2 已复核条目（只做覆盖率核对）。
- 复制/上传来源原文；购买出版物（H-031 人类决定）。

## 验收标准
1. 全部载具与剩余武器的 `historical` 字段：出处覆盖率 100%，即每个字段要么有 `source_refs[]`（含来源 ID 与页码/章节），要么显式标 `"未核实"`（`tools/data-validate --check-historical-coverage` 输出未标注字段数 = 0）。
2. 冲突项全部登记 PLAN_ISSUES（日志中"冲突"行数 = PLAN_ISSUES 新增条目引用数）。
3. `data-validate` 通过；`DATA_HASH` 变化已记录；`cargo test --workspace` 通过（含 golden hash 更新）。
4. 日志表行数 = 复核字段数；统计：一致/修正/未核实/冲突各自数量写入 STATUS。
5. 抽样自检：随机抽 10 个标"已核实"的字段，重新打开来源核对数字（记录于日志末尾），10/10 一致；不一致者回改。
6. 没有出现无法追溯的 URL：`sources.json` 中每个 URL 在日志里均有"访问日期"字段。

## 测试要求
- 单元：覆盖率检查工具（缺出处样例必报错）。
- 集成：`data-validate` + 全工作区测试。
- 性能基准：无。
- 回归：已 `locked` 字段不得被改（若复核值与其不同，仅写 PLAN_ISSUES，REF §7.5）。

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
- 规模 L；可拆 `M3-051b`（武器）。德军条令/Hogg/博物馆来源在 REF §6.2 均为"未核实"，大量字段可能只能标"未核实"，这是合法结果（E3-14："100% 有出处或'未核实'"）。
- 抽样复核与来源合法性确认需人类参与（H-062 抽样、H-031 来源边界），缺失时记 partial。
- 若联网受限（环境网络白名单），无法打开来源 → 全部标"未核实"并在 STATUS 说明，不得猜测。
