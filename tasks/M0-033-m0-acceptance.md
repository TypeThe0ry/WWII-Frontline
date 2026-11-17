# M0-033 M0 里程碑验收与 STATUS 汇总

## 元数据
- 里程碑：M0
- 工作流：test
- 规模：M
- 前置依赖：M0-030, M0-029, M0-027, M0-028, M0-025, M0-026, M0-031, M0-032
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MILESTONES §1; ARCHITECTURE §20.1; tasks/README §4; tasks/README §9

## 背景
里程碑验收任务**不写功能代码**（README §9）：逐条核对 `docs/MILESTONES.md` §1.3 的退出标准 E0-1…E0-12，走 §1.4 的演示脚本，对 ARCHITECTURE §20.1 的五项首要风险逐项给出"通过/不通过 + 证据"，汇总 STATUS 与 PLAN_ISSUES，为 Fable 的审阅检查点 0 准备材料（MILESTONES §1.10）。
判定规则（MILESTONES §1.3）：**E0-1 ~ E0-6 任一不通过 → M0 状态 `partial`**，必须写 PLAN_ISSUES 并由 Fable 裁决是否改架构；E0-7 ~ E0-12 不通过可带条件通过（如 E0-12 缺录像），但必须登记为 M1 的前置待办。`m{n}-acceptance` 为 `done` 的条件：退出标准每项都有实测数字，或已逐项登记为 `partial` 原因并经 Fable 同意（README §4）。**不得为推进而把 partial 写成 done。**
退出标准清单：E0-1 10 headless + 10 bot 30 min，`tick_ms` P99 ≤ 28 ms 且无连续 2 tick > 33 ms；E0-2 下行 P50 ≤ 128 kbps、上行 ≤ 32 kbps；E0-3 KCC 单次 ≤ 20 µs、100 个 ≤ 3 ms/tick；E0-4 native/native、wasm/wasm 逐 tick `state_hash` 一致，native vs wasm RMS 漂移 < 2 cm/s（信息性）；E0-5 RTX 3060 1080p ≥ 60 fps、Iris Xe/WebGL2/720p ≥ 30 fps；E0-6 Babylon `WebGPUEngine`+`ComputeShader` 冒烟与 Hi-Z/froxel/TAAU 可行性结论；E0-7 Chromium WT 与 WS 均通过、WT 禁用 ≤ 5 s 回退；E0-8 100 ms 延迟 + 2% 丢包下静止校正 ≤ 1 次/10 s、本地开火与枪口焰同帧；E0-9 `sim-wasm` ≤ 1.2 MB（br ≤ 400 KB）；E0-10 CI 全绿、依赖违规 PR 被拒；E0-11 /data 校验通过、两把枪 historical 字段 100% 有出处或"未核实"；E0-12 批 M0 每项有记录、复测在容限内（缺录像则 blocked）。

## 具体内容
### 要创建/修改的文件
- `reports/m0/acceptance.md`（验收报告：E0-1…E0-12 表、五项风险结论、演示脚本记录、任务粒度数据、未决问题清单）。
- `tasks/STATUS.md`：M0 各行补全实测结果（引用各任务已写入的数字）、顶部摘要（当前里程碑保持 M0，等待 Fable 审阅；blocked/partial 清单与表一致）。
- `docs/PLAN_ISSUES.md`：整理本里程碑新增条目（只追加状态说明，不改他人条目）；`docs/NEEDS_HUMAN.md`：仅在有新的人类输入需求时**追加**条目。
### 要实现的行为
1. **核对表**：对 E0-1…E0-12，每项一行：`标准 | 数字阈值 | 实测值 | 来源任务与报告路径 | 结论(达标/未达标/未测) | 备注`。实测值必须来自已提交的报告/STATUS，**不得新编数字**；缺参考机/录像的项写"未测"并说明原因与解除方。
2. **演示脚本**（MILESTONES §1.4 全部 7 步）逐步执行并记录结果与证据路径（截图/日志/命令）：①启动 `frontline-server --map m0 --mode sandbox`，Chrome 叠层显示 FPS/GPU ms/RTT/传输类型；②10 bot 交火，本地玩家走/冲刺/蹲/跳，开火 Kar98k 与 Garand，观察弹道下坠与命中标记，被击杀后重生；③`tc netem` 注入 100 ms + 2% 丢包，叠层显示校正次数；④禁用 WebTransport 回退 WebSocket，叠层显示 WS；⑤强制 WebGL2 重载；⑥启动 10 个 headless-client 并展示 `/metrics`；⑦对照 STATUS 的 E0-1…E0-12。无法执行的步骤（缺环境/硬件）如实标记。
3. **ARCHITECTURE §20.1 五项风险**逐项结论：（1）`wtransport` datagram 稳定性与浏览器互通（M0-017/M0-028）；（2）Rapier `enhanced-determinism` KCC ≤ 20 µs（M0-011）；（3）WASM 预测与原生漂移 < 2 cm/s（M0-029）；（4）Babylon `WebGPUEngine` + `ComputeShader` 下 Hi-Z/froxel/TAAU 可行性（M0-020）；（5）单块地形 + 植被帧率（M0-024/M0-030）。每项：**通过 / 不通过 / 带条件** + 证据 + 是否建议改架构（对应 §5.3 降级、WT 兜底策略、compute 路线等）。
4. **全局检查**：`cargo test --workspace`、`pnpm -C client test`、`cargo clippy -D warnings`、`eslint`、`tsc --noEmit`、CI 全部作业绿；`node tools/ci/check-deps.mjs`、`check-sim-purity.mjs`；`/data` 校验；`LICENSES.md` 台账校验（无 `PENDING` 素材进入主分支，M0 无真素材）。
5. **Fable 检查点材料**（MILESTONES §1.10）：①五项风险结论；②实测 `tick_ms`、编码耗时、带宽、KCC µs 是否支持 ARCHITECTURE §6.10 预算表，若不支持给出重新分配建议；③任务粒度校准：每个 M0 任务消耗的会话数与超出"一次会话"的任务列表；④对标测量工具链可用性与"M1 开工前是否必须补录像"的建议；⑤M0 发现的新依赖（INDEX 缺口）列表。
6. 在 STATUS 汇总 PLAN_ISSUES 编号、`partial`/`blocked` 任务及其解除条件；`m0-acceptance` 自身状态按判定规则设置。
### 接口与数据
- 无代码接口；报告用 Markdown + 表格，数字带单位与来源链接（相对路径）。

## 不做什么
- 不新增功能、不修 Bug（发现 Bug 写 PLAN_ISSUES 并建议由哪个任务修复）；不改 INDEX/MILESTONES/ARCHITECTURE。
- 不替 Fable 做架构裁决；不推进 `当前里程碑` 到 M1（由 Fable 在检查点后推进）。
- 不补做别的任务的缺失验收（缺则记 `未测`，退回对应任务）。

## 验收标准
1. `reports/m0/acceptance.md` 存在，E0-1…E0-12 **十二行齐全**，每行"实测值"来自可追溯报告，"未测"项各有原因与解除方；无空白格。
2. §20.1 五项风险各有一句"通过/不通过/带条件 + 证据路径"；若建议改架构，给出受影响文档章节。
3. 演示脚本 7 步各有"通过/失败/无法执行"与证据（日志片段、命令输出或截图路径）。
4. STATUS 中 M0-001…M0-033 每行状态与"实测结果"列非空（`todo` 不得残留）；顶部摘要的 blocked/partial 清单与表格一致（脚本核对：`grep` 状态列 = 清单项）。
5. E0-1 ~ E0-6 中任一不达标/未测 → `m0-acceptance` 与里程碑状态标 `partial` 并列出 PLAN_ISSUES 编号；全部达标 → 可标 `done`，但仍需等待 Fable 审阅（STATUS"下一个任务"写"等待 Fable 审阅检查点 0"）。
6. 全局检查命令全部执行并把退出码/摘要写入报告（含 CI 运行链接或日志摘要）。
7. 报告中没有任何参考作品的名称/截图/素材；对标测量段落只写"参考作品"与数字。

## 测试要求
- 单元测试：无新增。
- 集成/无头测试：重跑 `scripts/loadtest-m0.sh` 的 5 min 冒烟与 `tools/determinism` 以确认报告所引数字未因后续提交失效（若关键路径代码在测量后变化过）。
- 性能基准：不重测，引用基线；若基线日期早于最后一次影响性能的提交，注明"可能过期"。
- 回归：确认 golden 哈希与基线文件均已更新入库。

## 性能预算
汇总核对：ARCHITECTURE §6.10（tick ≤ 18 ms 稳态、P99 ≤ 28 ms）、§6.9.5（下行 P50 ≤ 128 kbps）、§13.5（CPU/GPU 帧预算）、§4.3.3（WASM ≤ 1.2 MB）；本任务不放宽任何数字，未达标即登记。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 本任务最容易出现的问题是"为了闭环而写 done"。凡缺参考硬件（NEEDS_HUMAN H-021）、缺录像（H-011/H-025/H-026）、缺受信任证书（H-017）的项，必须保持 `未测`/`partial`，并让人类知道缺什么。
- 触发 Fable 审阅的人类事项见 NEEDS_HUMAN H-038（M0 结束审阅）；M1 开工前还需 H-039（基于 M0 数据决定最低配置）。
- 缺失素材需求 ID 汇总（回报用）：M0 全部灰盒——V-CHR-US-001、V-CHR-GE-001、V-WPN-US-001、V-WPN-GE-001、V-FX-001、V-FX-006、V-NAT-002、V-NAT-004、V-NAT-007（CC0 可选）；音频全部缺失（M0 无音频）。
