# M4-046 M4 里程碑验收与 STATUS 汇总

## 元数据
- 里程碑：M4
- 工作流：test
- 规模：M
- 依赖：M4-045, M4-039, M4-044, M4-029, M4-028, M4-024, M4-025, M4-026, M4-027, M4-023, M4-019, M4-018, M4-021, M4-036, M4-032, M4-013
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MILESTONES §5; MILESTONES §5.3; MILESTONES §5.4; MILESTONES §5.10; ARCHITECTURE §1.6; tasks/README.md §4; tasks/README.md §9

## 背景
本任务**不写功能代码**（README §9）：逐条核对 MILESTONES §5.3 的 16 项退出标准 E4-1 … E4-16，走 §5.4 的演示脚本，汇总 `tasks/STATUS.md` 与 `docs/PLAN_ISSUES.md`，为 Fable 的审阅检查点 4（公开测试 Go/No-Go，§5.10）准备材料。
退出标准清单（数字照 MILESTONES §5.3）：
- E4-1 画质档性能 P95：Low Iris Xe 720p ≥ 30 fps；Medium 1080p 输出 45–60 fps；High RTX 3060 1080p ≥ 60 fps；Ultra RTX 4070+ 1440p ≥ 60 fps（缺硬件标 partial）。
- E4-2 内存：JS+WASM Low ≤ 0.9 / Medium ≤ 1.2 / High、Ultra ≤ 1.5 GB；显存 Low ≤ 1.5 / Med ≤ 2.5 / High ≤ 4 / Ultra ≤ 6 GB。
- E4-3 包体积：core ≤ 8 MB；每图 Medium ≤ 150 / High ≤ 350 / Ultra ≤ 600 MB。
- E4-4 浸泡：100 headless × 60 min × 三图（WT/WS）：tick_ms P99 ≤ 28 ms、无连续 overrun、末 10 min RSS 增长 < 2%、连接保持率 ≥ 99%。
- E4-5 平衡：三图×可用模式各 200 局：Frontline/Skirmish 40–60%；Assault 攻方 35–65%。
- E4-6 变体门禁：≤ 60（Low ≤ 40），预热覆盖率 100%。E4-7 动态分辨率：帧时间回到目标 ≤ 3 s，不自动降档。
- E4-8 反作弊统计：注入自瞄入前 1%，正常 bot 误报 < 1%。E4-9 安全：fuzz ≥ 1 h、CSP 无 unsafe-inline、依赖审计无高危未处理。
- E4-10 运维：告警均有演练、回滚 ≤ 10 min、证书续期演练、无人工连续 8 局。E4-11 合规：导出/删除端到端、遥测默认关闭抓包验证。
- E4-12 浏览器矩阵：Chromium/Edge 全功能；Firefox/Safari 分级登记。E4-13 封闭测试：≥ 3 局 × ≥ 60 人，缺陷分级入 PLAN_ISSUES。
- E4-14 可访问性与布局：文本对比度 ≥ 4.5:1、UI §6.5 矩阵通过、全部交互键盘可达。E4-15 史实与测量：无 `initial` historical 字段（或有 PLAN_ISSUES）、locked 抽样复测通过。E4-16 视觉回归：三图×12 机位×3 时段×四档子集基线入库，CI 阈值生效。
演示脚本（MILESTONES §5.4）8 步：浏览器按模式/地图过滤 + 基准向导推荐；Omaha Assault 登陆艇→滩头→崖顶；Foy 冬季 Frontline 与德军 Assault；Skirmish（Carentan 子区）；四档画质切换与强制 WebGL2 的 Low；运维（Grafana、告警/回滚演练、matchd 自动开局、复核队列）；隐私（导出/删除、遥测同意）；展示 STATUS 的 E4 实测数字与 Go/No-Go 清单。
Go/No-Go 额外必审（§5.10）：性能、稳定性、安全合规、反作弊（诚实对外说明浏览器局限）、内容（三图三模式可玩性与平衡）、素材与许可证（LICENSES.md 无 PENDING 入主分支、署名页含 OSM ODbL、对外材料无参考作品名称/截图）、成本（容量与月度成本，公开测试规模上限与降级策略）、结论（Go/有条件 Go/No-Go）。

## 具体内容
### 要创建/修改的文件
- `reports/m4/acceptance.md` —— E4-1…E4-16 逐项表：标准、数字、实测值、证据路径（报告/命令/STATUS 行）、状态（done/partial/未测）、负责任务。
- `reports/m4/demo-script-run.md` —— 演示脚本逐步执行记录（含时间戳、截图路径、问题）。
- `reports/m4/go-no-go.md` —— Go/No-Go 清单草案：每项有证据与状态；列出阻塞项与回退任务建议。
- `reports/m4/plan-issues-digest.md` —— PLAN_ISSUES 全量摘要（未决/已决/待 Fable 回写文档项）。
- `tasks/STATUS.md` —— M4 各行状态与顶部摘要（当前里程碑、blocked/partial 清单）更新。
- `tools/accept/m4_check.py` —— 机械核对脚本：读取 STATUS 与报告，输出 E4 表与缺漏。
### 要实现的行为
1. 运行 `m4_check.py`：对每个 E4 项找出对应报告并读取实测数字；缺证据的项标 `缺证据`。
2. 逐步走演示脚本（在可用环境；缺参考机则在软件渲染/占位素材下尽可能走通并注明）。
3. 汇总 STATUS：M4-001…045 每个任务状态；`partial/blocked/未测` 全部列出并映射到 Go/No-Go 清单的"条件"。
4. 汇总 PLAN_ISSUES：未决条目分级；需 Fable 裁决/回写文档的条目（如 ARCHITECTURE §13.5 预算偏差、navmesh 参数不一致、Foy Assault 攻方差异、`SpawnKind::Boat`、per-area Garrison 间距）单列。
5. 起草 Go/No-Go 清单（§5.10 八项），每项附证据路径；给出"建议结论"，**最终结论由 Fable/人类决定**（H-089、H-096）。
6. 核对合规项：LICENSES.md 无 PENDING 入主分支（脚本）、Credits 含 OSM ODbL 署名（R9）、对外材料禁用词扫描 0 命中（M4-037 工具）。
7. 不得把 `partial` 写成 `done`（README §4）；`m4-acceptance` 为 `done` 的条件：每个 E4 项有实测数字，或已逐项登记 `partial` 原因并经 Fable 同意。
### 接口与数据
- E4 表列：`id,criterion,target,measured,evidence,status,owner_task,notes`；Go/No-Go 清单列：`item,evidence,status,condition,owner`。

## 不做什么
- 不写功能代码，不修复缺陷（登记 PLAN_ISSUES 并指派）；不改 INDEX/docs 的结论与章节编号。
- 不替人类/Fable 做 Go/No-Go 决定；不触发公开测试发布（H-090/H-096）。
- 不重新测量已在其它任务完成的指标（引用其实测报告，除非证据缺失/过期）。

## 验收标准
1. **退出标准全部有实测数字**：E4-1…E4-16 共 16 项在 `acceptance.md` 中各有"实测值 + 证据路径"，或已逐项登记 `partial` 原因并写明 Fable 同意的待办（`缺证据` 项数 = 0）。
2. **Go/No-Go 清单每项有证据**：§5.10 的 8 项（性能、稳定性、安全合规、反作弊、内容、素材与许可证、成本、结论）每项均有证据路径与状态；无空白项。
3. 演示脚本 8 步全部走完并记录（通过/问题）；失败步骤数 = 0，或有 PLAN_ISSUES 编号与豁免说明。
4. STATUS 与报告一致：脚本核对 STATUS 中 M4 每个任务的状态与其报告结论一致，不一致数 = 0。
5. PLAN_ISSUES 摘要完成：未决条目全部分级；"阻塞/严重"缺陷在清单中有"修复任务或豁免"，缺归属数 = 0。
6. 合规核对：`LICENSES.md` 中 PENDING 但已进入主分支的条目数 = 0；Credits 含 OSM ODbL 署名；对外材料扫描命中 = 0。

## 测试要求
- 单元测试：`m4_check.py` 对合成 STATUS/报告集的解析（缺证据、状态不一致、partial 误标）。
- 集成/无头测试：演示脚本流程（可自动化部分用 Playwright + `headless-client`；人工部分记录）。
- 性能基准：汇总（不新测）。
- 回归：本任务产出作为 M5/公开测试补丁序列的基线。

## 性能预算
无新增；汇总 E4-1/E4-2/E4-3/E4-4/E4-7 等既有预算的实测结果（ARCHITECTURE §13、§11.4、§17.2）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖人类事项汇总：H-089（触发 M4 的 Fable 审阅）、H-081（并排相似度法律审查）、H-088（LICENSES 清零）、H-091/H-094（对外文案复核）、H-096（Go/No-Go）；这些均为人类输入，缺则对应清单项为"待人"，不得代填。
- 若硬件缺失导致 E4-1 部分 `partial`，在清单中写明"对外标注最低/推荐配置"的建议（§5.10 第 1 项）。
- INDEX 中本任务依赖未列出 M4-030/031/033/034/035/037 等（经由 M4-045/M4-036/M4-032 等传递）；验收时仍须检查全部 M4 任务的状态。
