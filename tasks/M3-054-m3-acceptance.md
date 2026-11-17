# M3-054 M3 里程碑验收与 STATUS 汇总

## 元数据
- 里程碑：M3
- 工作流：test
- 规模：M
- 前置依赖：M3-045, M3-047, M3-048, M3-049, M3-050, M3-051, M3-052, M3-053, M3-041, M3-042, M3-043, M3-044, M3-027, M3-025, M3-024, M3-040, M3-032, M3-012, M3-007
- 素材需求：无；素材状态：n.a.
- 引用文档：MILESTONES §4; MILESTONES §4.3; MILESTONES §4.4; MILESTONES §4.10; ARCHITECTURE §1.6; tasks/README.md §9

## 背景
本任务**不写功能代码**（README §9）：逐条核对 `docs/MILESTONES.md` §4.3 的退出标准 E3-1 … E3-15，走 §4.4 演示脚本，汇总 STATUS 与 PLAN_ISSUES，为 Fable 的检查点 3 准备材料。`done` 的条件（README §4）：每项退出标准有实测数字，或已逐项登记为 `partial` 原因并经 Fable 同意；不得把 partial 写成 done。
退出标准（MILESTONES §4.3）与取证来源：
| # | 标准 | 取证任务 |
|---|---|---|
| E3-1 | 14 兵种 × 2 阵营 loadout 可选并通过 data-validate；全部载具可刷新/驾驶/被击毁 | M3-001/002/012/029/032 |
| E3-2 | 8 能力 + 载具呼叫可由真人 Commander 下达；非法指令 100% 被拒 | M3-006/007/008/009/017/033 |
| E3-3 | ≥ 40 组装甲判定与手算一致 | M3-013 |
| E3-4 | 榴弹炮 1 000 发散布与 /data 一致；预设阵地全流程可玩 | M3-018/019/035 |
| E3-5 | Assault 4 阶段脚本化跑通；bot 攻方阶段完成率非 0 且非 100% | M3-020/028 |
| E3-6 | 100 客户端 × 30 min（WT 与 WS）：tick_ms P99 ≤ 28 ms；密集下行 P99 ≤ 400 kbps；无泄漏 | M3-045 |
| E3-7 | 12 载具同屏 draw call ≤ 1 200、三角形 ≤ 4.0 M、High P95 ≤ 16.7 ms | M3-029/049 |
| E3-8 | 200 局 × 两模式：胜率 40–60%；武器占比 0.5%–25% | M3-046/047 |
| E3-9 | LLM 指挥官：默认关闭；60 s；成本记账一致、≤ 0.5 美元/局；回退 `llm_fallback_total`；0 次绕过校验 | M3-027 |
| E3-10 | 150 ms RTT 载具命中率与 0 延迟差 ≤ 5% | M3-022 |
| E3-11 | 车内 3 人互通不泄漏；Command 仅 Commander + Officer；8 路上限 | M3-040 |
| E3-12 | en/zh-CN 键 100% 对齐；3 500 字覆盖；最长文本不溢出 | M3-053 |
| E3-13 | loadtest/balance/e2e-visual 连续 3 晚成功 | M3-050 |
| E3-14 | 全部载具与武器 historical 100% 有出处或"未核实"；批 M3 对标回填并复测 | M3-051/052 |
| E3-15 | core ≤ 8 MB；Carentan Medium 包 ≤ 150 MB | M3-041~044（manifest） |

## 具体内容
### 要创建/修改的文件
- `reports/m3-acceptance.md` —— 验收报告：逐条 E3-1…E3-15 的结果表（数字、命令、报告路径、状态 done/partial/blocked/未测）。
- `reports/m3-demo-log.md` —— 演示脚本 7 步的执行记录（日期、环境、参与者、结论、截图/日志路径；录像不入库）。
- `reports/m3-llm-cost.md` —— LLM 指挥官成本与回退演示记录：调用次数、tokens、成本记账与厂商账单对照（价格走配置，按**当时官方价格**重算；官方价格未核实则标"未核实"）。
- `reports/m3-fable-checkpoint.md` —— 给 Fable 的检查点 3 材料（MILESTONES §4.10 六项）。
- `tasks/STATUS.md` —— 更新 M3 全表与顶部摘要（执行者本职）。
- `docs/PLAN_ISSUES.md` —— 仅追加：未达标项、数字冲突、被推迟项。
### 要实现的行为
1. 运行并核对：`cargo test --workspace`、`pnpm -C client test`、`data-validate`、`manifest-budget`；CI 绿；记录提交哈希。
2. 对每条退出标准从取证任务的 STATUS 摘取**实测数字**；缺数字则重跑对应命令，不得抄"通过"。
3. 演示脚本（MILESTONES §4.4）7 步：①Commander 全部 8 能力 + 载具呼叫；②榴弹炮/Spotter/OP/AT 炮；③载具对抗（Tiger/Panther vs Sherman/Jumbo，M8/Puma 侦察，半履带运兵，击毁残骸）；④狙击/屏息/暴露规则/TC 认领；⑤Assault 测试配置 4 阶段（bot + 真人混合）；⑥开 LLM 指挥官（演示 key）并断网触发回退；⑦zh-CN 全流程 + 平衡报告 + nightly 历史。真人参与项（⑤、⑦ 的真人部分）标"需人工"。
4. 素材核查：列出 M3 范围（SHOP §4.4）仍为占位的需求 ID 清单，确认每项在 ASSET_GAPS 有 GAP 决定或"推迟到 Mx"（ASSET_GAPS §3.4-3）。
5. 汇总所有 `partial/blocked` 任务的原因与解除条件；汇总 PLAN_ISSUES 中 M3 相关条目。
6. 在 STATUS 顶部更新："当前里程碑"保持 M3，下一个任务写"等待 Fable 审阅检查点 3（H-076）"。
### 接口与数据
- 报告表头：`标准 | 目标 | 实测 | 命令/来源 | 状态 | 备注`。

## 不做什么
- 新功能、修缺陷、调参（发现问题写 PLAN_ISSUES；需要时由 Fable 追加任务）。
- 推进到 M4（须等 Fable 审阅）。
- 改 MILESTONES/ARCHITECTURE 等 docs（只写报告与 PLAN_ISSUES）。

## 验收标准
1. `reports/m3-acceptance.md` 覆盖 E3-1…E3-15 共 15 行，每行有数字或 `partial/blocked/未测` 的明确原因（行数核对脚本：缺行数 = 0）。
2. 每条标"达标"的行都附命令或报告路径，可复现；抽查 3 行重跑一致（记录于报告末尾）。
3. 演示脚本 7 步均有执行记录；未执行者写明原因（缺人/缺素材/缺 key）。
4. LLM 指挥官记录：回退演示至少触发 1 次 `llm_fallback_total` 增加（截图/日志）；单局成本 ≤ 0.5 美元（按当时价格重算），或标"未测/价格未核实"。
5. `tasks/STATUS.md` 中 M3 全部任务 done/partial/blocked 状态与 `reports/m3-acceptance.md` 一致；顶部三项摘要与表一致。
6. `docs/PLAN_ISSUES.md` 新增条目均有任务 ID 与建议方案；未登记的 `partial` 原因数 = 0。
7. 素材核查表包含 SHOP §4.4 中全部 M3 需求 ID 及其状态（real/placeholder/missing）。

## 测试要求
- 单元：无新增。
- 集成：重跑关键命令（上方第 1 点），输出写入报告。
- 性能基准：引用 M3-045/049 的结果，不重做。
- 回归：确认 M0–M2 退出标准未回退（抽查 `m2-acceptance` 的关键数字：loadtest-100、perf-m2）。

## 性能预算
- 无新增；报告中复述 ARCH §6.9/§6.10/§13.5 预算与实测对照。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 受人工输入影响的标准（E3-9 演示 key/价格 H-073、E3-14 录像 H-071、E3-13 需 3 个日历夜、E3-11 需 2+ 人听测、E3-7 需参考机 H-021）很可能只能 `partial`；这是允许的，但须逐项登记并经 Fable 同意。
- INDEX 的依赖列未含 M3-001~M3-023 等早期任务（经由传递依赖保证）；验收时仍需逐项核对其 STATUS。
- 触发 Fable 审阅属人类动作：H-076。
