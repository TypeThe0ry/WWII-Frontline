# M4-044 对标测量：复测与版本漂移处理

## 元数据
- 里程碑：M4
- 工作流：data
- 规模：M
- 前置依赖：M3-052, M4-043, M3-052b, M3-052c, M3-052d
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §7.4; REFERENCE_MEASUREMENTS §7.6; REFERENCE_MEASUREMENTS §9.1; REFERENCE_MEASUREMENTS §7.3; REFERENCE_MEASUREMENTS §7.5; REFERENCE_MEASUREMENTS §1.2

## 背景
对标测量批（M0–M3）已覆盖全部必测项；M4 不新增批次，而是做 **locked 项抽样复测与版本漂移处理**（REF §7.6、§9.1 "持续"行；MILESTONES §5.7）。
流程要点：
- 调参状态机（REF §7.5）：`initial` → `calibrated`（已对标并通过 §7.3 容限）→ `locked`（已有回归断言）；`tuning_status` 与记录状态 `pending/measured/stale/infeasible`（REF §8.3）。
- **版本漂移**（REF §7.6）：参考作品更新后，相关项标记 `stale`；是否跟随由人负责人/设计决策（**默认不盲目跟随**，仅当我方设计目标需要时重测）；我方 `/data` 改动不自动触发重测，但改动 PR 须注明是否影响已 `locked` 项。
- **同法复测**（REF §7.4）：我方游戏提供测量模式/调试场景（固定练习场、网格叠加、可调时钟、导出事件日志）；但为了与对标可比，**必须同时用录像分析法复测**，两者互相校验；复测用与 REF §2 相同设置（≥ 120 fps 录制、关闭动态模糊、固定 FOV）、相同场景编号与重复次数；结果记在记录文件的"我方复测"小节，列与参考列并排，输出误差百分比与是否通过。
- 误差容限（REF §7.3）：时序 ±5% 或 ±1 帧@60 fps（取较大者）；速度 ±3%；距离/半径/范围 ±5% 或 ±1 m；角度 ±5% 或 ±0.1°；统计量中位数 ±10% 且 95 分位 ±15%，**致死发数必须精确相等**；曲线型三个关键点各 ±10%。
- 合规硬约束（REF §1.2）：只做黑盒观测——录自己的游戏画面/音频、公开视频、官方日志与 wiki 公开规则；**禁止**反编译/拆包/内存读取/抓包/协议解析/提取素材/使用数据挖掘表；原始录像与测量原始数据**不入库**（`docs/measurements/raw/`、`assets/incoming/ref-captures/` 在 `.gitignore`）；对外材料不得出现参考作品名称。
**本任务需要人类提供参考作品新版本的抽样录像**（人工，H-011/H-025/H-029 等）。

## 具体内容
### 要创建/修改的文件
- `docs/measurements/summary.csv` —— 更新 `locked` 项的复测列、`version`、`status`（`measured/stale`）；（此文件属 REF §8.2 约定的记录，不是规划文档；只追加/更新行）。
- `docs/measurements/{ID}.md` —— 被抽样项的"我方复测"小节（REF §8.1 模板）。
- `tools/ref-analysis/` —— 沿用脚本；新增 `recheck_sampler.py`（按分层随机抽样 `locked` 项，种子固定）与 `compare_report.py`（参考 vs 我方 vs 容限）。
- `reports/ref/m4-recheck.md` —— 抽样表、结果、漂移说明、行动项。
- `crates/sim/tests/ref_assertions.rs`、`crates/headless-client/scenarios/ref_*` —— 对 `locked` 项断言（REF §7.5）；断言读取 `/data` 里的 `ref_median` 与容限，**不得硬编码参考数字**。
### 要实现的行为
1. **抽样**：从全部 `locked` 项按类别分层抽样（武器时序、移动速度、生存/致死发数、载具、建造/补给、指挥官、火炮、对局、UI 时序），总体 ≥ 15% 且每类 ≥ 1 项；固定种子并写入报告。
2. **参考侧**：人类提供新版本（或同版本）抽样录像 → 执行者按 REF §3 方法分析 → 与记录里的旧版 `ref_median` 比较：差异在容限内 → `measured` 不变；超出 → 判定"版本漂移"并标 `stale`，登记行动（跟随 / 不跟随 / 重测，**决定权在人**）。
3. **我方侧**：在我方练习场/headless 场景用同法复测，写"我方复测"，与参考并排；误差超容限 → 调 `/data`（`deviation_reason`）或登记。
4. **漂移处理**：对 `stale` 项，保持当前 `gameplay` 值不变（默认不跟随），记录 `ref_version_drift` 备注；若人决定跟随，则按 §7.1 流程更新（需重新 `calibrated → locked`，回归断言同步）。
5. **回归**：运行全部 `locked` 项断言（`cargo test -p sim ref_assertions`），确保 M4 中所有 `/data` 改动未破坏 `locked` 项。
6. **合规检查**：确认仓库无录像/原始数据/参考作品名称（`git ls-files` 检查与禁用词扫描）。
### 接口与数据
- `summary.csv` 状态列沿用 REF §8.2/§8.3；新增列 `recheck_date`、`recheck_result`（`pass|drift|fail|skipped`）；若列定义需变更，按 README §5.1"细化"写 PLAN_ISSUES，不重排 ID。

## 不做什么
- 不新增测量批次（M0–M3 已覆盖）；不测新武器/新机制。
- 不使用任何禁止手段（拆包/抓包等）；不上传录像至不受控第三方。
- 不决定是否跟随参考作品的数值（人负责）；不改对标相关的史实数据（M4-043）。

## 验收标准
1. **抽样项误差在容限**：被抽样的 `locked` 项我方复测与参考的误差全部落在 REF §7.3 容限内（致死发数精确相等）；通过率 = 100%，不通过项必须有处理行动（调参并重测达标，或登记 PLAN_ISSUES 与 `deviation_reason`）。
2. **漂移项更新或登记**：参考版本较旧记录发生数值变化的项（若有）全部标 `stale` 并有"跟随/不跟随/重测"结论（人确认）；无结论项数 = 0。
3. 抽样规模：`locked` 项总体抽样 ≥ 15%，且每个类别 ≥ 1 项；报告列抽样种子与清单。
4. 回归：`cargo test -p sim ref_assertions` 全部通过（0 失败），断言不含硬编码参考数字（grep 检查）。
5. 合规：`git ls-files` 中无 `*.mp4|*.mkv|*.mov` 与 `docs/measurements/raw/` 内容；禁用词扫描 0 命中。
6. 报告 `reports/ref/m4-recheck.md` 含：参考版本号、日期、录制设置、协助者/人员、置信度变化。

## 测试要求
- 单元测试：`compare_report.py` 对容限判定（时序/速度/距离/角度/统计/曲线）的单测，含致死发数精确相等规则；抽样器分层与种子可复现。
- 集成/无头测试：`headless-client` 的 `ref_*` 场景；`cargo test -p sim ref_assertions`。
- 性能基准：无。
- 回归：本任务本身产出回归断言的最新状态；M4-046 引用。

## 性能预算
无。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **人工：参考作品新版本抽样录像**；依赖 H-011（合法获取与 EULA）、H-025（录制设备）、H-027（协助者）、H-028（私有存储与访问控制）、H-029（版本与日期记录）、H-034（抽样复核）。缺录像 → 参考侧项标 `skipped`，任务 `partial`/`blocked`，不得凭印象填数。
- 若参考作品 EULA 禁止录制/分析（H-011），只能用公开视频与官方日志，抽样范围会缩小；写 PLAN_ISSUES。
- 我方复测的"测量模式"依赖 M1-071/M2-054 已建立的调试场景；缺失则先补最小实现（≤ S 规模）并说明。
