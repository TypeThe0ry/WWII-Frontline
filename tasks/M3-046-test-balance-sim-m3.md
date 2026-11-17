# M3-046 测试：M3 平衡模拟

## 元数据
- 里程碑：M3
- 工作流：test
- 规模：L
- 前置依赖：M2-051, M3-026, M3-028, M3-023
- 素材需求：无；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.3; ARCHITECTURE §4.3.8; ARCHITECTURE §16.5; GAMEDESIGN §21; GAMEDESIGN §3.4; MILESTONES §4.3 (E3-8)

## 背景
`crates/balance-sim`（ARCH §4.3.8）：无渲染、无网络的整局 bot 对战（总控裁定：用**缩比对局**——默认 20v20、30 min、简化物理/粗粒度导航，单核 ≥ 10× 实时；全尺寸 100 人局只做抽样 ≥ 5 局），输出 CSV：胜率（按地图/阵营）、局时长、武器击杀占比、各兵种 KDA、资源曲线、据点争夺周期。M2-051 已做 Frontline 版本。M3 加入载具、全部指挥官能力、炮兵、Spotter/Sniper、Assault 模式。
注意（ARCH §17.3）：bot 行为 ≠ 真人，**只用于发现数值异常（一边倒、不可用武器），不替代真人试玩**；调参必须附带前后对比（见 M3-047）。
判据（E3-8，已按裁定区分模式）：缩比对局 200 局 × 两种模式（Frontline、Assault，共 400 局）：**Frontline 阵营胜率 40–60%，Assault 攻方胜率 35–65%**；无单武器击杀占比 > 25% 或 < 0.5%；越界项写入调参清单。载具击杀比（载具击杀 ÷ 载具损失）与兵种 KDA 作为诊断输出，不设硬阈值。
CI 关联（ARCH §17.5）：`balance` 作业 200 局缩比对局 ≤ 4 h（nightly/手动，总控裁定替代原 < 40 min），由 M3-050 接入。

## 具体内容
### 要创建/修改的文件
- `crates/balance-sim/src/modes.rs`（改）—— 支持 `--mode frontline|assault`、`--map carentan`、Assault 使用 M3-004 测试配置；阵营交替（攻守互换）。
- `crates/balance-sim/src/runner.rs`（改）—— 多进程并行（`--jobs N`）、固定种子序列 `seed_i = base_seed + i`、可重现。
- `crates/balance-sim/src/metrics.rs`（改）—— 新增指标：武器击杀占比（按武器 ID，含载具武器、能力击杀单独成类 `ability:*`）、兵种 KDA、载具击杀比、能力使用次数与击杀贡献、炮兵命中率、Assault 阶段完成率与平均时长、局时长分布。
- `crates/balance-sim/src/report.rs`（改）—— 输出 `reports/balance/m3-{date}-{mode}.csv` 与 `summary.md`，并自动生成 `tuning_candidates.md`（越界项：指标、当前值、阈值、涉及 `/data` 字段路径）。
- `crates/balance-sim/tests/determinism.rs` —— 同种子两次运行结果逐指标相同。
### 要实现的行为
1. 400 局缩比对局运行（200 × 2 模式；另加全尺寸 100 人抽样 ≥ 5 局，仅记录）；默认难度 Regular（GD §20.6）；每局时间上限 = 模式时限（Frontline 90 min 上限，Assault 初始 20 min + 加时）；
2. 胜率统计：Frontline 按阵营（US/GER）；Assault 按攻方/守方，并同时给出按阵营的结果。
3. 武器击杀占比：分母为全部击杀，分子按武器；载具主炮、机枪、爆炸物、能力（空袭/炮击）分别成类。
4. 自动判定：胜率不在 [40%, 60%]、任一武器占比 > 25% 或 < 0.5%（仅统计"本模式内应使用的武器"，白名单见 `/data` loadouts）→ 写入 `tuning_candidates.md`。
5. 输出每局原始行（`games.csv`：seed、模式、胜方、时长、阶段数、各阵营击杀/损失），便于复核。
### 接口与数据
```
balance-sim --mode frontline --map carentan --games 200 --jobs 8 --seed 1000 --out reports/balance/
```
- CSV 列定义写在 `crates/balance-sim/README.md` 的"列定义"节（可新建，限本 crate）。

## 不做什么
- 调参本身（M3-047）；真人试玩平衡。
- Omaha/Foy/Skirmish（M4-039）。
- 优化 bot 行为（只发现问题，记录 PLAN_ISSUES）。

## 验收标准
1. 完成 400 局：`reports/balance/m3-*` 下 CSV 行数 = 400 局记录 + 汇总；总 panic/卡死局数 = 0。
2. 确定性：同一 `--seed` 重跑前 20 局，所有汇总指标逐位相同（`cargo test -p balance-sim determinism` 通过）。
3. 越界项自动产出：对人为注入的偏差样例（例如临时把某武器伤害 ×3），`tuning_candidates.md` 必报出该武器（测试用例）。
4. 报告含：两种模式阵营/攻守胜率、局时长 P50/P95、武器击杀占比表（全部武器）、兵种 KDA、载具击杀比、各能力使用次数 ≥ 1 次的局占比、Assault 阶段完成率。
5. 吞吐：记录 `games/min` 与总耗时；`balance` 作业目标（缩比对局单核 ≥ 10× 实时；200 局夜间 ≤ 4 h）——如达不到，写 PLAN_ISSUES 并说明所需并行度（不放宽 200 局）。
6. Frontline 阵营胜率在 [40%, 60%]、Assault 攻方胜率在 [35%, 65%] 内，或越界项已列入 `tuning_candidates.md`（此条是"结果须被正确记录"，不要求平衡已达成）。

## 测试要求
- 单元：指标聚合、越界判定、白名单过滤、CSV 序列化。
- 集成：`balance-sim --games 4`（冒烟）在 PR CI 内 < 3 min。
- 性能基准：记录参考机型号、`--jobs`、总耗时。
- 回归：与 M2-051 的 Frontline 基线对比，列出差异（载具/能力引入造成）。

## 性能预算
- 200 局缩比对局 ≤ 4 h（nightly，总控裁定）；bot 思考预算按 ARCH §16.5（≈ 3.0 ms/tick）不得回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；可拆 `M3-046b`（Assault 部分）。
- 吞吐口径已裁定：全尺寸 100 人局（约 45 simulated min）400 局约 18 000 局·分钟，在 40 min 内不可达（需 ≥ 225× 实时），故改为缩比对局口径（见背景）；若缩比口径仍达不到，写 PLAN_ISSUES。
- 胜率口径已裁定：Frontline 阵营胜率 40–60%；Assault 攻方胜率 35–65%（与 M4-039 一致）。
- 载具/能力导致的击杀占比归类规则须在报告里写明，避免与"武器击杀占比"阈值混淆。
