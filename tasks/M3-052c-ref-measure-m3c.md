# M3-052c 对标测量：M3 批回填（3/4：指挥官能力 C-01–C-15 与音频时序 AU-09–AU-12）

## 元数据
- 里程碑：M3
- 工作流：data
- 规模：M
- 前置依赖：M3-052b
- 素材需求：无；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §9.1; REFERENCE_MEASUREMENTS §4.8; REFERENCE_MEASUREMENTS §4.12; REFERENCE_MEASUREMENTS §7.2; REFERENCE_MEASUREMENTS §7.3; REFERENCE_MEASUREMENTS §7.4; REFERENCE_MEASUREMENTS §7.5; REFERENCE_MEASUREMENTS §8

## 背景
**[人工：录像]** 本任务依赖人类录制参考作品录像（NEEDS_HUMAN H-071，批 M3；前置 H-026 的合规确认、H-025 录制设备、H-028 私有存储、H-029 版本记录）。执行者负责**分析**与回填，不录制。合规边界（REF §1.2、BRIEF §1）：只黑盒观测；录像与原始数据**不入库**（`docs/measurements/raw/` 与 `assets/incoming/ref-captures/` 被 .gitignore）；不提取素材、不解析协议；对外不出现参考作品名称。
批 M3 必测项总计 67 项（REF §9.1），按测量类别拆为 M3-052 / 052b / 052c / 052d 四个任务（README §5.6 续作规则，每次会话做一组；分配：052 = V-01–V-14 + R-12，共 15 项；052b = V-15–V-27，共 13 项；052c = C-01–C-15 + AU-09–AU-12，共 19 项；052d = A-01–A-13 + W-13/W-27–W-29/W-36 + U-02/U-13，共 20 项）。
**本任务只负责：C-01–C-15（延迟/持续/冷却/范围/成本/预警/空袭密度/扫射/精确打击/空投/侦察/鼓舞/通告）与 AU-09–AU-12（音频时序），共 19 项。**
流程（REF §7.1）：采集 → 逐帧/音频/像素分析（工具栈 REF §3.1：ffmpeg、Python+OpenCV/NumPy/SciPy、Audacity、mpv）→ 记录 `docs/measurements/{ID}.md` + `summary.csv`（模板 REF §8）→ 映射 `/data` 字段（REF §7.2：`source: ref_measurement`、`ref_id`、`ref_version`、`ref_median`）→ 我方复测（REF §7.4）→ 误差对比（REF §7.3 容限）→ 调整 `/data` → 回归断言（REF §7.5）。
容限（REF §7.3）：时序 ±5% 或 ±1 帧@60 fps 取较大者；速度 ±3%；距离/半径 ±5% 或 ±1 m；角度 ±5% 或 ±0.1°；致死发数必须精确相等；统计量中位数 ±10%。

## 具体内容
### 要创建/修改的文件
- `docs/measurements/{ID}.md` —— 每项一份记录（模板 REF §8.1：元数据、原始数据逐次重复、统计、公开资料、我方复测、映射的 /data 字段、备注）。
- `docs/measurements/summary.csv` —— 追加行（列定义 REF §8.2）。
- `tools/ref-analysis/`（M0-032 已建；本任务补脚本：音频时序脚本（波形事件检测）与空袭/扫射落点密度统计脚本）。
- `data/**/*.json` —— 回填 `ref_measurement` 字段与 `gameplay` 映射值（仅手感类/时序类；史实类以 REF §6.1 为准）。
- `crates/sim/tests/ref_assertions_m3c.rs` —— 已 `calibrated` 的字段回归断言（读取 `/data` 的 `ref_median`/容限，不硬编码）。
- `docs/PLAN_ISSUES.md` —— 仅追加（测量不确定度大于容限、与史实冲突需裁决等）。
### 要实现的行为
1. 检查录像到位清单：对照 H-071 与 REF §4 的场景需求；缺失的项标 `blocked` 并在 STATUS 列出所缺条目（不得编造数据）。
2. 每项至少 5 次重复（REF §2.5）；记录置信度（A/B/C）；不确定度 > 容限则按 REF §7.3 放宽并降低置信度。
3. 映射到 `/data`（路径以 `/data` schema 为准；REF §4 的 `/data` 字段列为拟议路径）；一项多字段时换算规则写入 schema 说明而非代码（REF §7.2.2）。
4. 我方复测：在练习场用同一方法（≥ 120 fps 录制，REF §7.4）复测，记录误差与是否通过；通过则字段 `tuning_status: calibrated`。
5. 通过且已写断言的字段设为 `locked`。
6. 与 M3-047 的平衡调参冲突时：手感类以对标为准，写 PLAN_ISSUES 并通知 Fable。
### 接口与数据
- `/data` 字段旁：`source: ref_measurement`、`ref_id: "V-05"`、`ref_version: "<构建号>"`、`ref_median: <值>`、`tuning_status`。

## 不做什么
- 不测本批 M3 中属于其它三个任务（M3-052/052b/052c/052d 之外的组）的项。
- 录制录像（人类）；任何素材/音频提取；协议或客户端拆解。
- 版本漂移处理与抽样复测（M4-044）。
- 史实复核（M3-051）。

## 验收标准
1. 本任务负责的 19 项（见背景）每项在 `summary.csv` 有一行，状态为 `measured` 或 `blocked(缺录像：原因)`；已处理项数/总项数写入 STATUS（目标总数 19）。
2. 每个 `measured` 项：重复次数 ≥ 5，记录文件含原始数据、统计（中位数、P95）、置信度；缺任一项即不算完成。
3. 复测：每个映射到 `/data` 的项都有"我方复测"小节，误差在 REF §7.3 容限内的比例写入 STATUS（目标 100%；不达标项登记 PLAN_ISSUES 并保持 `initial`）。
4. 致死发数类（若涉及狙击 W-36/H 类）复测精确相等。
5. 回归断言：`cargo test -p sim ref_assertions_m3c` 通过；断言读取 `ref_median`，源码无参考数字字面量（grep 检查）。
6. 仓库中没有录像/原始数据文件：`git ls-files | grep -Ei '\.(mp4|mkv|mov|wav)$'` 为空；`docs/measurements/raw/` 被忽略。
7. 缺录像时本任务标 `blocked`，STATUS 顶部 blocked 清单登记"所缺条目 — 日期 — 谁来解除（H-071）"。

## 测试要求
- 单元：分析脚本（标定/统计）对合成样例的精度测试。
- 集成：对一项（如 V-11 炮塔转速）端到端：样例视频 → 记录 → /data → 我方复测。
- 性能基准：无。
- 回归：REF §7.5 断言入 CI。

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
- 批 M3 共 67 项，已拆为 M3-052/052b/052c/052d 四个任务（每组 19 项即本任务），依次执行；后一任务依赖前一任务的分析脚本与记录模板。
- 在录制合规确认（H-026：EULA 是否允许）前不得录制；默认 `blocked`。
- 参考作品版本变化会使数据 `stale`（REF §7.6）；每项记录构建号。
- 本任务产出文件位于 `docs/measurements/`，这是任务产物目录，不属于对既有 docs 章节的修改。
