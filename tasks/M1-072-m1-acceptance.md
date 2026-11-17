# M1-072 M1 里程碑验收与 STATUS 汇总

## 元数据
- 里程碑：M1
- 工作流：test
- 规模：M
- 依赖：M1-050, M1-068, M1-069, M1-064, M1-065, M1-067, M1-070, M1-071, M1-056, M1-059, M1-060, M1-061, M1-058, M1-063, M1-047
- 素材需求：无（灰盒即可）；素材状态：n.a.（汇总全部仍为占位的需求 ID）
- 引用文档：MILESTONES §2; MILESTONES §2.3; MILESTONES §2.4; MILESTONES §2.10; ARCHITECTURE §1.6; ASSET_GAPS §3.4; tasks/README.md §9

## 背景
`m{n}-acceptance` 任务（README §9）**不写功能代码**：逐条核对 MILESTONES §2.3 的退出标准 E1-1 … E1-17，走 §2.4 的演示脚本，汇总 `tasks/STATUS.md` 与 `docs/PLAN_ISSUES.md`，列出仍为占位的需求 ID，为 Fable 的审阅检查点 1（§2.10）准备材料。README §4：里程碑验收任务为 `done` 的条件是退出标准每项都有实测数字，或已逐项登记为 `partial` 原因并经 Fable 同意；**不要为了推进而把 partial 写成 done**。退出标准概览（MILESTONES §2.3）：E1-1 真人测试局（≥ 20 min、≥ 20 名真人每队 10；无法招集则 ≥ 8 真人 + headless 等价客户端，标 `partial`）；E1-2 tick（P99 ≤ 28 ms、稳态 ≤ 18 ms）；E1-3 带宽（P50 ≤ 128 kbps、P99 ≤ 400 kbps、上行 ≤ 32 kbps）；E1-4 AOI（视锥外且 > 96 m 的敌人下发 = 0）；E1-5 延迟补偿（150 ms RTT 命中率差 ≤ 5%）；E1-6 High 性能（P95 ≤ 16.7 ms、draw call ≤ 1 200、三角形 ≤ 4.0 M）；E1-7 Low 性能（≥ 30 fps、draw call ≤ 900/500）；E1-8 材质变体 ≤ 60；E1-9 包体积（core ≤ 8 MB、首图 Medium ≤ 150 MB）；E1-10 部署流程（全程键鼠可达、点击部署到可操作 ≤ 15 s、R_near 预取完成率 ≥ 95%）；E1-11 Garrison + 补给闭环；E1-12 坦克（3 座位、≥ 12 组 AP 判定与手算一致、120 ms RTT 直线校正 ≤ 1 次/10 s）；E1-13 倒地/救起/压制（状态机全转移单测、压制参数被客户端音画消费）；E1-14 素材管线（`.fpak` 往返字节一致、无独立 .glb/.ktx2/.ogg/.opus、缺失报告、无 PENDING 进主分支）；E1-15 基础反作弊（1.3× 速度/超射速被纠正/拒绝并计数）；E1-16 数据与史实（M1 武器与 Sherman historical 100% 有出处或"未核实"，批 M1 回填与复测在容限）；E1-17 占位决定（范围内仍为占位的需求 ID 在 ASSET_GAPS 有"决定"或明确推迟）。

## 具体内容
### 要创建/修改的文件
- `reports/m1-acceptance.md` —— 验收报告：E1-1…E1-17 逐项表（标准｜数字目标｜实测数字｜测量方法/命令/报告路径｜结论 PASS/partial/FAIL/未测）。
- `reports/m1-placeholders.md` —— 仍为占位的需求 ID 清单（来自 M1-064/065 的缺失素材报告），每项对应 ASSET_GAPS 的 GAP 编号与"决定/推迟到 Mx"（NEEDS_HUMAN H-047/H-058）。
- `reports/m1-playtest-log.md` —— 20 真人测试局记录模板与填写：日期、真人数/bot 数、时长、版本、地图、传输类型分布、帧率/ping 摘要、崩溃/断线、玩家反馈要点、核心循环（占点—补给—重生）问题清单。
- `tasks/STATUS.md`（修改）—— M1 全部任务行核对；顶部摘要更新：当前里程碑进入"等待 Fable 审阅检查点 1"；blocked/partial 清单与 PLAN_ISSUES 编号一致。
- `docs/PLAN_ISSUES.md`（追加汇总条目）—— 本里程碑未解决问题的目录（只追加）。
### 要实现的行为（均为核对与记录，无功能代码）
1. 逐项核对 E1-1…E1-17：从 STATUS 与 `reports/*.json` 取实测数字，**不得重新估算或用别的机器数字顶替**；缺项标 `未测` 并说明缺什么（人类/参考机/素材/录像）。
2. 走演示脚本（MILESTONES §2.4）：①预发布域名 HTTPS、启动页与 core 包体积；②服务器浏览器→20 人测试服→US→部署（HQ 点、小队、Officer、预取观察）；③移动/姿态/翻越、Thompson 与 Garand、Officer 放 Garrison（全息）、队友经 Garrison 重生；④Support 放 Supply/Ammo Crate、补弹；⑤Medic 救起、压制效果；⑥3 人乘 Sherman 沿堤道进老城、AP 命中 GER、Panzerfaust 击毁坦克；⑦三据点（L1a/L2a/L3a）拉锯、战术地图；⑧展示 STATUS 的 E1-1…E1-17、`/metrics` 看板、缺失素材报告。每步记录通过/失败与截图/录屏路径（录屏不含参考作品内容）。
3. 20 真人测试局：由人类组织（H-054 招募 20 人、H-056 组织与反馈收集）；执行者准备脚本/表单/日志采集与记录模板，并在测试后汇总数字；若无法招集，≥ 8 真人 + headless 等价客户端，E1-1 标 `partial`。
4. 汇总：STATUS 中 `partial`/`blocked` 清单、PLAN_ISSUES 条目、占位需求 ID 清单；对照 MILESTONES §2.10 的六项必审内容准备材料（测试局记录、5 兵种与 Sherman 假设评估、带宽/tick 偏差、素材到位与占位及 GAP-V01/V02/V03/V06 的 M2 决定、批 M1 测量偏差、M1 任务粒度/返工率统计）。
5. 运行台账校验（LICENSES.md §4.4）：输出 `PENDING` 条数、`REJECTED` 条数、缺失存档文件、manifest 与台账不一致项；存在 PENDING 对应处理产物进入主分支即为违规（E1-14）。
6. 不关闭任何未达标项：未达标写入"遗留问题"，不改验收标准。
### 接口与数据
```text
reports/m1-acceptance.md 表头：| E# | 标准 | 目标 | 实测 | 方法/命令/报告 | 结论 | 遗留(PI-xxx) |
```

## 不做什么
- 不写功能代码、不调参、不改文档结论（发现问题只写 PLAN_ISSUES）。
- 不推进"当前里程碑"到 M2（由 Fable 审阅后推进，README §7.2）。
- 不组织真人测试局本身（人类，H-054/H-056）；不替人类做素材决定（H-047/H-058）。
- 不重新设计任务或拆分 M2（M2 任务粒度重评估由 Fable 在检查点做）。

## 验收标准
1. `reports/m1-acceptance.md` 覆盖 E1-1…E1-17 共 17 行，每行含实测数字或"未测+原因"，无空白格（脚本 `tools/ci/check-acceptance.mjs` 检查 17 行且无空字段）。
2. 演示脚本 8 步每步有记录（通过/失败/部分）；失败步骤有 PLAN_ISSUES 编号。
3. STATUS 一致性：M1 的 72 个任务行均不为 `todo`/`in-progress`（`done`/`partial`/`blocked` 之一）；顶部摘要的 blocked/partial 清单与表内状态一致（脚本比对，不一致数 = 0）。
4. `reports/m1-placeholders.md` 列出范围内全部仍为占位的需求 ID，且每项有 GAP"决定"或"推迟到 Mx"（E1-17；缺决定的项标红并报给 Fable）。
5. LICENSES 台账校验输出已贴入报告：PENDING 对应处理产物进入主分支数 = 0。
6. E1-1 状态为以下之一：`PASS`（≥ 20 min、≥ 20 真人每队 10）、`partial`（≥ 8 真人 + headless 等价，已注明）、`未测`（无测试局）；测试局记录含真人数/bot 数/时长/崩溃与断线次数。
7. 为 MILESTONES §2.10 六项必审内容各准备 ≤ 1 页材料（`reports/m1-review-pack.md`）。

## 测试要求
- 单元测试：`check-acceptance.mjs` 与 STATUS 一致性脚本的用例（缺行、空字段、状态不一致）。
- 集成/无头测试：重跑 `test-loadtest-20`、`test-perf-m1` 与 CI 全绿快照（只引用已有结果，若过期 > 7 天且代码有变更则重跑）。
- 性能基准：无新增；引用 M1-068/M1-069 报告。
- 回归：全部 CI 作业（`lint`、`test-rust`、`test-ts`、`wasm-build`、`determinism`、`data-validate`、`manifest-budget`、`e2e-visual`）在验收提交上通过。

## 性能预算
- 无新增；汇总引用 ARCHITECTURE §6.9/§6.10（服务器）、§13.5（客户端）、§11.4（体积）的已测数字。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已汇总（缺项标"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材清单已输出；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：PLAN_ISSUES 汇总条目已追加
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新（状态"等待 Fable 审阅检查点 1"）；提交信息含任务 ID

## 备注/已知风险
- 触发 Fable 审阅由人类执行（NEEDS_HUMAN H-057）；本任务完成后回报"等待 Fable 审阅"并结束会话，不开始 M2。
- 招集 20 真人为最大不确定项（MILESTONES §2.8）；允许 8 真人 + headless 等价客户端并标 `partial`，是否补测由 Fable 决定。
- 若上游任务因素材/录像/域名账号 `blocked`，本任务对应退出标准标 `未测` 而非 `PASS`；不得用占位或他机数字代替。
