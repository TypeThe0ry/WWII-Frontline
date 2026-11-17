# M2-055 M2 里程碑验收与 STATUS 汇总

## 元数据
- 里程碑：M2
- 工作流：test
- 规模：M
- 前置依赖：M2-049, M2-050, M2-051, M2-052, M2-047, M2-053, M2-054, M2-042, M2-043, M2-041, M2-040, M2-038, M2-039, M2-037, M2-044, M2-045, M2-026, M2-023, M2-016, M2-018, M2-012, M2-011, M2-021, M2-029, M2-048, M2-024, M2-025
- 素材需求：无（灰盒即可）；素材状态：n.a.（核对 M2 批占位清单）
- 引用文档：MILESTONES §3; MILESTONES §3.3; MILESTONES §3.4; MILESTONES §3.10; ARCHITECTURE §1.6; README（tasks）§4; README（tasks）§9

## 背景
本任务**不写功能代码**（tasks/README.md §9）：逐条核对 `docs/MILESTONES.md` §3.3 的 15 条 M2 退出标准，走 §3.4 演示脚本，汇总 STATUS 与 PLAN_ISSUES，为 Fable 的审阅检查点 2（MILESTONES §3.10）准备材料。`m{n}-acceptance` 为 `done` 的条件：每条退出标准都有实测数字，或已逐项登记为 `partial` 原因并经 Fable 同意（README §4）。
M2 退出标准（MILESTONES §3.3）与对应来源任务：
- E2-1 100 客户端 WT 压测：tick_ms P99 ≤ 28 ms、稳态 ≤ 18 ms、无连续 overrun、末 10 min RSS 增长 < 2%（M2-049）。
- E2-2 WS 兜底：tick P99 ≤ 40 ms、快照 15 Hz、连接保持（M2-049）。
- E2-3 带宽：下行 P50 ≤ 128 / P99 ≤ 400 kbps；5% 丢包典型 ≤ 68 kbps；上行 ≤ 32 kbps（M2-049、M2-020）。
- E2-4 bot CPU：60 bot ≤ 3.0 ms/tick、100 bot ≤ 5 ms（M2-029）。
- E2-5 语音：订阅更新 P95 < 300 ms、LiveKit API 错误率 < 1%、LiveKit 不可用时游戏不受影响（M2-024/025/026，T8）。
- E2-6 渲染 pass（High）：Hi-Z ≤ 0.5、GTAO ≤ 0.9、froxel ≤ 0.8、TAAU ≤ 0.8、粒子 1 600 ≤ 0.7 ms（M2-030…034、M2-052）。
- E2-7 帧时间：四机位 P95 ≤ 16.7 ms；JS 堆 + WASM ≤ 1.5 GB（Medium ≤ 1.2 GB）；老城 draw call 因剔除下降 ≥ 30%（M2-052）。
- E2-8 包体积：core ≤ 8 MB、Carentan Medium ≤ 150 MB（M2-047 manifest 门禁）。
- E2-9 完整对局：至少 1 局完整 Frontline（开局到胜负）在 bot + 真人混合下无崩溃；战线拉锯与资源结算与 GD §3.3、§11 一致（M2-008/009）。
- E2-10 建造与资源：建造清单全部可放置/被摧毁；位置限制单测 100%；资源整分钟结算精确（M2-009/010/011）。
- E2-11 balance-sim：缩比对局单核 ≥ 10× 实时、200 局缩比对局夜间 ≤ 4 h、CSV 字段齐全、同 seed 可复现（M2-051）。
- E2-12 视觉回归：12 机位 × 2 路径基线入库、差异阈值 0.1%（M2-050）。
- E2-13 监控：§18.3 全部指标有看板；人为制造 tick 超限触发告警（M2-048）。
- E2-14 史实与测量：M2 新增武器/载具 historical 100% 有出处或"未核实"；批 M2 对标回填并复测（M2-053/054）。
- E2-15 带宽调节器：netem 弱网下 `PACKET_BUDGET` 缩放生效，2 Hz 下限不被饿死（M2-020/049）。

## 具体内容
### 要创建/修改的文件
- `tasks/STATUS.md` —— 更新 M2 全部任务行（状态、实测、遗留 PI、提交哈希）、顶部摘要（当前里程碑保持 M2，直至 Fable 推进）；追加"M2 验收汇总"小节（E2-1…E2-15 逐条：数字、来源任务、状态 `done/partial/未测`、证据路径）。
- `reports/m2-acceptance/summary.md`、`reports/m2-acceptance/criteria.json` —— 退出标准核对表（机器可读）与证据索引（压测报告 `reports/m2-load.json`、性能报告 `reports/perf/m2_perf.json`、视觉回归报告、balance 摘要、监控告警演练记录、ref 批 M2 覆盖表）。
- `reports/m2-acceptance/demo_run.md` —— 演示脚本执行记录（含日期、提交哈希、机器、录屏/截图存放路径（私有，不入库））。
- `docs/PLAN_ISSUES.md`（只追加条目）—— 汇总执行期间所有未决问题，按严重度分类；**不改他人条目**。
### 要实现的行为
1. 逐条核对 E2-1…E2-15：读取 STATUS 中对应任务的实测数字，核对是否达标；无数字则回到对应任务补测（不在本任务内实现功能）；达标/未达标/未测三态。
2. 走演示脚本 MILESTONES §3.4 的 9 步：①预发布服开 100 人局，服务器浏览器显示人数/模式/地图；②5 条战线拉锯、相邻锁定、战术地图 Frontline 叠层、资源 HUD；③Engineer 建造铁丝/沙袋/MG 巢/资源节点，Officer 放置 Outpost，展示 Supplies 与资源的区别（R6）；④小队语音与近距语音（两台设备），LiveKit 断开后游戏继续；⑤载具：Jeep 运兵、Panzer IV 对 Sherman、模块损伤与维修站修理；⑥GTAO/体积雾/TAAU 开关对比与遮挡剔除 draw call 对比叠层、沼泽与运河；⑦运行 `loadtest-100` 并展示看板与 WS 兜底结果；⑧结算界面、记分板、投票/举报流程；⑨balance-sim 200 局 CSV 与视觉回归报告。每步记录通过/失败与偏差。
3. 完整对局验证（E2-9）：至少 1 局 bot+真人混合的完整 Frontline（可用 `time_limit` 缩短的版本需注明，且另有一局 `full_control` 或时间到的自然结束），记录开局到胜负的日志与 `state_hash`，无崩溃。
4. 素材核对：列出 M2 范围内仍为占位的需求 ID，并核对 ASSET_GAPS 是否有对应决定或"推迟到 Mx"（ASSET_GAPS §3.4 第 3 条）。
5. 汇总 `docs/PLAN_ISSUES.md`：分类（阻断/重要/细化）；对 INDEX/文档冲突（如聊天限速、Ping 轮盘、PTT 键位、balance-sim 速度目标）逐项确认已按总控裁定落实（以 tasks/README.md §5.1 的优先级规则为准）；裁定之外新发现的冲突确认已登记。
6. 产出"Fable 审阅检查点 2"材料清单：未达标项与建议、范围冻结/砍项候选、M3 前置人类事项（H-068、H-069、H-070 等）。
### 接口与数据
```json
{ "criterion": "E2-1", "target": "tick_ms P99 <= 28", "measured": "…", "status": "done|partial|untested", "evidence": "reports/m2-load.json", "task": "M2-049", "pi": ["PI-0xx"] }
```

## 不做什么
- 不实现或修复功能；发现缺陷只写 PLAN_ISSUES 并回到对应任务（或经 Fable 追加续作任务，README §5.6）。
- 不改 INDEX/README/MILESTONES/ARCHITECTURE；不把 `partial` 写成 `done`。
- 不决定里程碑推进（Fable 审阅后才推进"当前里程碑"）。

## 验收标准
1. `criteria.json` 含 15 条退出标准，每条均有"实测值或明确的未测原因 + 证据路径"（脚本校验 15/15 非空）。
2. 达标判定：E2-1…E2-15 中每条标记 `done`（有数字且达标）或 `partial/untested`（附原因、PI 编号与"缺什么"）；`done` 的条目不得含"待测"字样（脚本扫描）。
3. 演示脚本 9 步全部执行并记录结果（9/9 有记录）；失败步骤有 PLAN_ISSUES 编号。
4. 压测报告 `reports/m2-load.json` 的数字被引用到 STATUS（tick_ms P99、下行 P50/P99、RSS 增长），且与原始报告一致（抽查 3 个数字）。
5. STATUS 一致性：M2-001…M2-055 每行状态与"实测结果/遗留问题/完成提交"列非空（`done`/`partial` 行）；顶部摘要（当前里程碑/下一个任务/blocked 清单）与表一致（脚本 `tools/status-check.ts`，0 处不一致）。
6. 素材占位清单：M2 范围内每个仍为占位的需求 ID 在 ASSET_GAPS 有决定或在报告里标"推迟到 Mx"（逐项列出，0 个遗漏）。
7. PLAN_ISSUES 汇总：本里程碑所有条目有状态与建议；与本任务引用的冲突项（至少 4 项）均已登记。
8. 材料交付：Fable 审阅检查点 2 材料清单已写入 `reports/m2-acceptance/summary.md`。

## 测试要求
- 单元测试：`tools/status-check.ts`（构造不一致 STATUS 必须报错）；`criteria.json` schema 校验。
- 集成/无头测试：重新跑一遍 `loadtest-100` 冒烟（5 min）与 `cargo test --workspace`、`vitest`、`e2e-visual`，确认主干绿色。
- 性能基准：引用 M2-049/052 报告，不重复采集（除非报告日期晚于任一重大合并，则需复核）。
- 回归：全量 CI（lint/test-rust/test-ts/wasm-build/determinism/data-validate/manifest-budget/e2e-visual）全绿。

## 性能预算
- 不新增预算；汇总 M2 全部预算数字（ARCH §6.9.5、§6.10、§13.5、§16.5）的达标情况。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（全量 CI 绿）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖列表已补 M2-021、M2-029、M2-048、M2-024、M2-025；地图 M2-001…005 等经 M2-049/M2-026 等传递满足，验收时仍须核对全部 M2 任务状态，若任一为 `blocked/partial` 需在汇总中显式列出。
- 需要人类/硬件的项（参考机、压测机、录像、LiveKit 主机、OAuth 注册）缺失时对应条目记 `partial/未测`，本任务至多 `partial`；对应 NEEDS_HUMAN：H-060、H-061、H-063、H-066、H-067。
- 演示脚本第 1 步需预发布服（M1-067）；缺失则在本地 compose 环境演示并注明。
- 检查点完成后由人类触发 Fable 审阅（H-068），不得自行推进里程碑。
