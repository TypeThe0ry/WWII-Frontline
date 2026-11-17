# M4-039 测试：三图三模式平衡模拟

## 元数据
- 里程碑：M4
- 工作流：test
- 规模：M
- 前置依赖：M3-047, M4-016
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.3; GAMEDESIGN §21; GAMEDESIGN §3.4; GAMEDESIGN §3.5; GAMEDESIGN §20.6; REFERENCE_MEASUREMENTS §7.3

## 背景
`balance-sim`（ARCHITECTURE §17.3）：无渲染、无网络，以缩比对局（默认 20v20、30 min，单核 ≥ 10× 实时；全尺寸 100 人局抽样 ≥ 5 局）跑 bot 对战 N 局（并行进程），输出 CSV：胜率（按地图/阵营）、局时长、武器击杀占比、各兵种 KDA、资源曲线、据点争夺周期。**注意：bot 行为 ≠ 真人，只用于发现数值异常（一边倒、不可用武器），不替代真人试玩**；改动数值的 PR 必须附带前后对比；目标用 REFERENCE_MEASUREMENTS 的黑盒测量校准"节奏指标"（TTK、冲锋时间等）作为回归基线。
MILESTONES E4-5：**三图×可用模式各 200 局缩比对局**：Frontline/Skirmish 阵营胜率 **40–60%**；Assault 攻方胜率 **35–65%（须说明理由）**；越界项已调参或登记。
可用模式矩阵（GAMEDESIGN §3.7 与 MAPS）：Carentan：Frontline、Skirmish（及 Assault 测试配置，M3-004）；Omaha：Assault（Frontline 变体与 Skirmish 子区为可选）；Foy：Frontline、Assault（美军攻，MAPS §5.6）、Skirmish（子区）。具体以 `data/maps/*/zones.json.modes` 为准（脚本读取，不硬编码）。
调参清单：所有越界项给出建议改动（`/data` 字段、方向、幅度），遵守"游戏性数值只在 `/data`；初值 `initial`，经校准后改状态；客户端不硬编码"（README §5.4）；任何偏离史实的数值须写 `deviation_reason`（REF §6.1）。

## 具体内容
### 要创建/修改的文件
- `crates/balance-sim/src/matrix.rs` —— 地图×模式矩阵编排（读 zones.modes）、并行（独立进程，不进 `sim`）。
- `crates/balance-sim/src/report.rs` —— 汇总 CSV + Markdown 报告 + 置信区间（Wilson）。
- `reports/balance/m4-{map}-{mode}.csv`、`reports/balance/m4-summary.md`、`reports/balance/m4-tuning-list.md`。
- 如需调参：`data/**/*.json` 的最小改动（每改一处在报告列出前后对比；`tuning_status` 保持 `initial/calibrated` 的状态机，R8/REF §7.5）。
- 夜间 job：`balance`（沿用 M3-050）。
### 要实现的行为
1. 对每个（地图, 模式）组合跑 200 局；固定种子序列（`seed = base + i`），记录版本哈希（`DATA_HASH`、git commit）。
2. Assault：攻守阵营固定（Omaha US 攻、Foy US 攻、Carentan US 攻（攻守互换由 M4-031 的轮换规则在多局间实现，balance-sim 另跑一次互换））；Skirmish 子区按轮换；Frontline 阵营互换以消除出生侧偏差（每 100 局互换）。
3. 输出指标：胜率与 95% 置信区间、平均局时长、各兵种击杀占比、武器击杀占比（识别"不可用/主导武器"，阈值 > 25% 单武器击杀占比提示）、据点易手次数、资源曲线、TTK 分布。
4. 对越界项（胜率不在目标区间）做**调参建议**，在不破坏对标/史实约束下首选调整：重生/Garrison 数量、Assault 加时、据点半径、载具刷新（数值在 `/data`，不动代码）；最多迭代 3 轮，每轮重跑受影响组合（200 局）。
5. 无法在 3 轮内收敛的项登记 PLAN_ISSUES，并说明原因（如 Omaha 海滩地形结构性偏向防守）。
6. 异常检测：单局时长 < 5 min 或 > 120 min 的局数占比 ≤ 5%；无卡死局（沿用 M4-016 指标）。
### 接口与数据
- `balance-sim --map <id> --mode <mode> --games 200 --seed-base N --out reports/balance/…`；CSV 列：`game_id,seed,map,mode,attacker,winner,reason,duration_s,captures,kills_by_weapon…`。

## 不做什么
- 不替代真人试玩（只发现数值异常）；不改 sim 代码规则（只改 `/data`）；不做武器史实复核（M4-043）。
- 不做 Foy/Omaha 之外的新地图。
- 不做 LLM 指挥官对照实验（可选，若 M3-027 启用则分开跑）。

## 验收标准
1. **各图胜率 40–60%**：Frontline 与 Skirmish 每个（地图, 模式）组合的某一阵营胜率及其 95% 置信区间下界/上界与 40–60% 区间的关系：点估计 ∈ [40%, 60%]。
2. **Assault 攻方 35–65%**（须说明）：Omaha、Foy、Carentan 各自攻方点估计 ∈ [35%, 65%]；报告含说明段落（地形/起始资源/加时机制如何导致结果）。
3. 每个组合 200 局，失败（崩溃/卡死）局数 ≤ 2%；超过则该组合重跑并登记。
4. 越界项：每项要么调参后重测达标（附前后对比表），要么登记 PLAN_ISSUES（编号写入 STATUS）。
5. 单武器击杀占比 > 25% 或 < 0.5%（可用武器）的条目全部列出并有处理结论。
6. 可复现：同 `seed-base` + 同版本两次运行 `winner` 序列逐局相同（`state_hash` 一致，抽样 20 局）。

## 测试要求
- 单元测试：`cargo test -p balance-sim`：Wilson 区间、矩阵读取 zones.modes、阵营互换逻辑。
- 集成/无头测试：先 10 局冒烟，再全量；并行度按 CPU 核数（记录机器）。
- 性能基准：每局墙钟耗时与实时倍率（缩比对局目标单核 ≥ 10×，记录；全尺寸抽样仅记录）。
- 回归：把本轮 CSV 汇总值作为基线 `reports/balance/baseline-m4.json`，后续数值 PR 对比。

## 性能预算
- 夜间 `balance` 作业 200 局缩比对局 ≤ 4 h（总控裁定）：单个组合 200 局应在并行度下按此预算评估；超出则拆分作业，不改验收局数。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- bot ≠ 真人：Omaha 滩头对 bot 与真人的难度差异大；结果仅供数值方向参考，Go/No-Go 需结合 M4-045 真人局反馈。
- 若 Foy/Omaha 地图数据为占位，结论仅限"流程可跑"，须在真数据接入后重测（`partial`）。
- 数值调参来源：内部试玩 + 本任务；**对标测量**项（`locked`）不得随意改动，需先经 M4-044 的流程（REF §7.6）。
