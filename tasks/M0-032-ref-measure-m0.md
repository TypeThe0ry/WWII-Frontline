# M0-032 对标测量：M0 批回填

## 元数据
- 里程碑：M0
- 工作流：data
- 规模：M
- 依赖：M0-012, M0-011
- 素材需求：无（需人工录像，REFERENCE_MEASUREMENTS §9.2）；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §9.1; REFERENCE_MEASUREMENTS §7; REFERENCE_MEASUREMENTS §8; REFERENCE_MEASUREMENTS §3; REFERENCE_MEASUREMENTS §4

## 背景
`[人工：录像]`。把"手感"数值（射速、后坐、散布、开镜、移动速度、姿态切换、致死发数）与参考作品**黑盒录像分析**对齐：流程见 REFERENCE_MEASUREMENTS §7.1（采集 → 逐帧/音频/像素分析 → 记录 → 映射到 `/data` → 我方同法复测 → 误差对比 → 自动化回归）。合规边界（§1.2/§1.3）：**只做黑盒观测**；录像与原始数据**不入库**（`docs/measurements/raw/`、`assets/incoming/ref-captures/` 被忽略）；不提取素材/不反编译/不解析协议；记录里只存数字与文字。
**批 M0 必测项（REFERENCE_MEASUREMENTS §9.1）**：步枪射击 W-01、W-02、W-03、W-04、W-05、W-06（若首发步枪为栓动/逐发装填）、W-07、W-08、W-09、W-11、W-12、W-13（栓动）、W-15–W-18（后坐）、W-19–W-21（扩散）、W-22–W-25（初速与下坠）、W-26、W-27–W-29（晃动与屏息）、W-30、W-37；基础移动 M-01–M-06、M-12–M-14、M-17、M-19、M-24–M-26；生存基础 H-01（步枪致死发数，近中远）、H-02、H-03。M0-032 分两部分：**A 部分（工具链与模板，不依赖录像，必须完成）**与 **B 部分（录像分析与回填，依赖人类录像）**。

## 具体内容
### 要创建/修改的文件（A 部分）
- `tools/ref-analysis/`（Python 3 + OpenCV + NumPy + SciPy；`pyproject.toml`）：`extract_frames.py`（ffmpeg 抽帧/裁剪封装）、`audio_events.py`（M-AUD：波形峰值检测 → 射击事件时间戳 → rpm，REFERENCE_MEASUREMENTS §3.3）、`frame_events.py`（M-FRM：基于帧号的事件间隔，§3.2）、`calibrate.py`（M-CAL：像素标定求速度/距离，§3.4）、`stats.py`（中位数/极差/不确定度 = max(极差/2, 1 帧)、置信度 A–D 判定，§4.0）、`record.py`（生成 `docs/measurements/{ID}.md` 与追加 `summary.csv` 行）、`tests/`（对合成数据的单元测试）。
- `docs/measurements/_TEMPLATE.md`（REFERENCE_MEASUREMENTS §8.1 的记录模板原样复制）、`docs/measurements/summary.csv`（仅表头，列定义见 §8.2，全部 25 列）、`docs/measurements/series/.gitkeep`、`docs/measurements/README.md`（目录用法与"录像私有存储"说明）。
- `.gitignore` 复核：`docs/measurements/raw/`、`assets/incoming/ref-captures/` 已忽略（M0-001 已加，此处核对）。
- 我方复测支持（B 部分用）：`crates/sim/tests/ref_regression.rs`——读取 `/data` 里的 `ref_median` 与容限字段（**不硬编码参考数字**）做断言（REFERENCE_MEASUREMENTS §7.5）；`crates/sim/tests/own_remeasure.rs`——在 `sim` 中用与录像法同定义的方式测量我方射速/下坠/速度/致死发数并输出 JSON。
### 要实现的行为（B 部分，录像到位后）
1. 按 §2 的采集规范（≥ 120 fps、固定 FOV、关闭动态模糊、每项重复 ≥ 5 次）读取私有录像（路径通过环境变量 `REF_CAPTURE_DIR` 提供，不入仓库）；对批 M0 每项产出 `docs/measurements/{ID}.md`（含元数据、重复明细、统计、置信度）与 `summary.csv` 一行。
2. 映射到 `/data`（§7.2）：对应字段旁增加 `source: ref_measurement`、`ref_id`、`ref_version`、`ref_median`；时序量化到 tick 时用累加器验证长连射平均（§7.2 第 3 条）；史实与对标冲突按 §6.1 取舍并写 `deviation_reason`。
3. 我方同法复测（§7.4）：用 M0-020… 的客户端或 `own_remeasure` 导出日志，并**同时**对我方录像用同一分析脚本复测；误差按 §7.3 容限判定（时序 ±5% 或 ±1 帧@60 fps 取大；速度 ±3%；距离 ±5% 或 ±1 m；角度 ±5% 或 ±0.1°；统计量中位数 ±10%；**致死发数必须精确相等**）。
4. 通过项的 `_meta.status` 升为 `measured`（或 schema 规定的 `calibrated`），并在 `ref_regression.rs` 增加对应断言（状态机 `initial → calibrated → locked`，§7.5）。
5. 无法黑盒测得的项标 `infeasible`（置信度 D，改用史实/估计）。
### 接口与数据
- `summary.csv` 25 列、记录文件模板、`/data` 映射字段以 REFERENCE_MEASUREMENTS §8 与 M0-004 schema 为准；schema 如需增加可选 `ref_*` 字段，先改 schema 并记 PLAN_ISSUES。

## 不做什么
- 不测批 M1/M2/M3 的项（M1-071、M2-054、M3-052）；不做版本漂移复测（M4-044）。
- 不录制/提取/存储参考作品的图像、音频、素材；不分析其网络协议或内存（REFERENCE_MEASUREMENTS §1.2）。
- 不决定对标/史实冲突的裁决规则（NEEDS_HUMAN H-030）。

## 验收标准
1. **A 部分**：`pytest tools/ref-analysis/tests` 全绿；`record.py` 对合成数据生成的记录文件与 `summary.csv` 行通过列数/列名校验（25 列，表头与 §8.2 逐列一致）；合成射速音频（已知 600 rpm 的人工脉冲序列）经 `audio_events.py` 得到的 rpm 误差 ≤ ±1%；合成匀速运动视频（已知 4.4 m/s）经 `calibrate.py` 得速度误差 ≤ ±3%。
2. **B 部分（需录像）**：批 M0 **每一项**在 `summary.csv` 有一行，`status ∈ {measured, infeasible}`，且对应 `docs/measurements/{ID}.md` 存在；缺任一项则 B 部分未完成。
3. 回填后我方复测：批 M0 所有 `measured` 项 `pass == yes`（误差在 §7.3 容限内）；致死发数 H-01/H-02/H-03 精确相等。未通过项逐条写入 PLAN_ISSUES（含误差与原因）。
4. `ref_regression.rs` 对已 `locked` 项有断言且 `cargo test -p sim` 通过；断言读 `/data` 的 `ref_median`/容限，源码中无参考数值字面量（grep 抽查）。
5. 合规抽查：`git ls-files | grep -Ei "\.(mp4|mkv|avi|mov|wav|flac)$"` 为空；`docs/measurements/` 内无图像文件；记录文件不含参考作品的商标图形。
6. 录像缺失时：A 部分完成并提交，STATUS 将本任务标 `blocked`，在"blocked 清单"写"所缺：批 M0 录像（H-011/H-025/H-026）— 日期 — 人类解除"，并追加 NEEDS_HUMAN 条目（README §6）。

## 测试要求
- 单元测试：`stats.py`（置信度判定边界 n=5/10、极差 5%/20%）、音频峰值检测抗噪、帧号→时间换算、CSV 追加幂等。
- 集成/无头测试：`own_remeasure` 导出的 JSON 与 `summary.csv` 的 `own_median` 对接；`cargo test -p sim ref_regression`。
- 性能基准：无。
- 回归：已 `locked` 项的断言；参考版本更新后标 `stale`（§7.6，M4-044 处理）。

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
- 人类事项：合法获取并确认 EULA 允许录制与内部分析（NEEDS_HUMAN H-011）、录制设备（H-025）、M0 批录像会话至少 2–3 个（H-026）、协助者（H-027）、私有存储（H-028）、版本记录（H-029）、裁决规则（H-030）、工具安装与传输方式（H-033）、分析人员与抽样复核（H-034）。
- 参考作品的游戏内名称不得出现在任何对外材料；记录文件中统一称"参考作品"。
- 工具安装（ffmpeg、OpenCV 等）需要执行环境允许；若云端容器无法访问私有录像，传输方式由人类决定（不得上传到不受控的第三方，REFERENCE_MEASUREMENTS §9.2 #10）。
- 本任务规模标 M，但 B 部分的分析量取决于录像质量；超出一次会话时按 README §5.6 追加 `M0-032b`。
