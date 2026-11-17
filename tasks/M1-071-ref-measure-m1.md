# M1-071 对标测量：M1 批回填与我方复测

## 元数据
- 里程碑：M1
- 工作流：data
- 规模：M
- 依赖：M0-032（`tools/ref-analysis` 与 `docs/measurements/` 模板）, M1-011（移动）, M1-017（压制）, M1-019（Garrison/重生）, M1-023（坦克）, M1-015（手雷）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §9.1; REFERENCE_MEASUREMENTS §7.4; REFERENCE_MEASUREMENTS §7.3; REFERENCE_MEASUREMENTS §7.2; REFERENCE_MEASUREMENTS §7.5

## 背景
**[人工：录像]** 参考作品只能黑盒观测（公开视频、自录视频、官方日志、公开 wiki 规则）；禁止拆包/反编译/内存读取/协议解析/素材提取（BRIEF §1）。录像与原始数据**不入库**（`docs/measurements/raw/` 与 `assets/incoming/ref-captures/` 在 `.gitignore`）；仓库只存数值事实与记录文件。批 M1 的测量需求（REF §9.1）：剩余移动 M-07–M-11、M-15、M-16、M-18、M-20–M-23、M-27–M-29；5 个兵种所需武器的其余 W 项与近战 W-31–W-33、机枪 W-35；生存 H-04–H-17；压制 U-01–U-11；手雷 G-01–G-08、G-12；1 辆坦克 V-01–V-05、V-08、V-11–V-16、V-19–V-21；建造/补给 B-01–B-03、B-07、B-08、B-10–B-13、B-17；UI 时序 UI-01–UI-08、UI-12；对局 R-01、R-03–R-05、R-08–R-10；音频 AU-01、AU-02、AU-04（**精确 ID 以 REF §9.1 与 §4 为准**）。映射规则（§7.2）：每个 `/data` 游戏性字段旁注 `source: ref_measurement`、`ref_id`、`ref_version`、`ref_median`；服务器 30 Hz 将时序量化为 tick，小间隔用累加器避免系统性偏差。误差容限（§7.3）：时序 ±5% 或 ±1 帧@60 fps（取较大者）；速度 ±3%；距离/半径 ±5% 或 ±1 m；角度 ±5% 或 ±0.1°；统计量中位数 ±10% 且致死发数精确相等；曲线型三关键点各 ±10%。我方复测（§7.4）：固定练习场/测量模式，用与对标相同设置（≥ 120 fps 录制、固定 FOV、相同场景编号与次数），结果记在记录文件的"我方复测"节并输出误差与是否通过。调参状态机：`initial → calibrated → locked`（回归断言，§7.5）。

## 具体内容
### 要创建/修改的文件
- `docs/measurements/{ID}.md`（每个已测项一份，模板 REF §8.1）与 `docs/measurements/summary.csv`（REF §8.2 列）—— 逐项记录：元数据、原始数据摘要（仅数值）、统计、公开资料数值事实、我方复测、映射的 `/data` 字段。
- `tools/ref-analysis/`（M0-032 的工具；本任务补批 M1 分析脚本）—— 帧计时、音频波形法（M-AUD）、像素标定（M-CAL）、位移/速度提取；输入为本地私有录像路径（不入库）。
- `data/**` —— 回填被测字段：`source: ref_measurement`、`ref_id`、`ref_version`、`ref_median`、`tuning_status: calibrated`（通过容限者），偏离者写 `deviation_reason`。
- `crates/sim/tests/ref_regress_m1.rs` 与 `crates/headless-client/src/scenarios/ref_retest_m1.rs` —— 我方复测场景与回归断言（读 `/data` 的 `ref_median` 与容限字段，不硬编码参考数字；通过项升 `locked`）。
- `client/test/ref-timing/*.spec.ts` —— UI/表现层时序的 Playwright 固定机位 + `performance.now()` 差值断言（UI-01–UI-08、UI-12）。
- `docs/PLAN_ISSUES.md`（追加）—— 初值与测量值偏差大的项、与史实冲突需裁决项（REF §6.1/H-030）。
### 要实现的行为
1. 逐批 M1 项：录像到位→分析→记录→回填 `/data`→我方复测→通过则 `calibrated`（回归断言后 `locked`）。
2. 冲突处理：史实 vs 对标冲突按 REF §6.1 规则（外形/口径/弹容量/射速以史实）；手感类以对标；有偏离必须写 `deviation_reason`。
3. 录像缺失项：记录文件状态 `blocked(no-footage)`，summary.csv 对应行 `待测`，不使用其它来源数字顶替。
4. 合规纪律：不导出/保存/入库参考作品的音频或画面；音频仅测行为阈值（REF §4.12）；记录与对外文本不出现参考作品名称/商标/截图（用"参考作品/HLL"于内部文档允许，游戏内与对外不得）。
5. 数据变更 = 版本变更（`DATA_HASH`）；在 STATUS 记录新旧哈希。
### 接口与数据
```json
"fire": { "rpm": 700, "_ref": { "source": "ref_measurement", "ref_id": "W-01", "ref_version": "待填构建号", "ref_median": 0, "tolerance": "±5%|±1f", "tuning_status": "calibrated" } }
```
（`ref_median` 为占位，实际值来自记录，禁止编造。）

## 不做什么
- 不做 M2/M3 批测量（M2-054、M3 对应任务）。
- 不做史实参数复核（M1-070）；不做平衡模拟调参（M2+ `balance-sim`）。
- 不上传/提交录像、原始音频或截图；不把参考作品内部数据（拆包/抓包所得）当来源——**只有黑盒观测**。
- 不在无录像时编造测量值。

## 验收标准
1. 批 M1 每个测量项在 `docs/measurements/summary.csv` 有一行：状态为 `已测` 或 `blocked(no-footage)`；已测项都有对应 `{ID}.md` 与元数据（构建号、日期、重复次数 ≥ 5，REF §2.5）。已测项数 / 批 M1 总项数写入 STATUS。
2. 已回填的 `/data` 字段 100% 带 `_ref`（ref_id、ref_version、ref_median）；`data-validate` 通过。
3. 我方复测误差在 REF §7.3 容限内：时序 ±5% 或 ±1 帧@60 fps；速度 ±3%；致死发数精确相等；其余按表（`ref_retest_m1` 与 Playwright 断言输出每项 PASS/FAIL，通过率写入 STATUS；INDEX 指标）。
4. 通过项已有回归断言（`cargo test -p sim ref_regress_m1` 全绿），状态 `locked`；未通过项在 PLAN_ISSUES 有条目与处置建议。
5. 合规检查：`git ls-files` 中无录像/原始音频/截图（`docs/measurements/raw/**`、`assets/incoming/ref-captures/**` 不在仓库）；`git grep` 在 `client/` 与对外文案中无参考作品名称/商标（CI 检查已有则复用）。
6. 录像缺失则任务 `blocked`，STATUS 顶部 blocked 清单登记"所缺录像批—日期—人类（H-053）"，NEEDS_HUMAN 已追加；其余可做项（如我方复测脚手架）继续。

## 测试要求
- 单元测试：分析脚本的帧→时间换算、像素标定、统计（中位数/95 分位）；容限判定函数。
- 集成/无头测试：`ref_retest_m1`（headless 测量模式）；Playwright 时序断言。
- 性能基准：无。
- 回归：已 `locked` 项进入 CI；改动 `/data` 的 PR 必须说明是否影响 `locked` 项（REF §7.5）。

## 性能预算
- 无（数据任务）；时序量化为 30 Hz tick 的误差须在 §7.3 容限内。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`（缺录像项标 `未测`，状态至多 `partial`/`blocked`）
- [ ] 测试通过（`cargo test`、Playwright、`data-validate`）；`clippy`/`eslint`/`tsc --noEmit` 干净
- [ ] 性能预算：无
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：冲突项写 PLAN_ISSUES；`summary.csv` 更新
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 需要人类：合法获取参考作品并确认 EULA 允许录制分析（H-011）、录制设备（H-025）、批 M1 录像会话（H-053，约 4–6 个会话）、协助者（H-027）、私有存储（H-028）、版本日期记录（H-029）、冲突裁决规则（H-030）、分析人员抽样复核（H-034）。
- 部分项需要协助者/多人场景（R-01 占点人数规律、H-09 救起、U-01–U-11 压制），录像难度高；未到位的项保持 `initial` 并在 M1 验收如实标注。
- 参考作品版本漂移（REF §7.6）：记录每次录像的构建号；更新后标 `stale`，不盲目跟随。
