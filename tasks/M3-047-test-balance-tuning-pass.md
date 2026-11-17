# M3-047 数据：M3 首轮数值调参

## 元数据
- 里程碑：M3
- 工作流：data
- 规模：M
- 前置依赖：M3-046
- 素材需求：无；素材状态：n.a.
- 引用文档：GAMEDESIGN §21; GAMEDESIGN §0.2; REFERENCE_MEASUREMENTS §6.1; REFERENCE_MEASUREMENTS §6.3; REFERENCE_MEASUREMENTS §7.2; REFERENCE_MEASUREMENTS §7.3; ARCHITECTURE §17.3

## 背景
M3-046 产出 `tuning_candidates.md`（越界的胜率/武器占比）。本任务依据平衡模拟与已有对标测量，对 `/data` 的**游戏性**初值做首轮调整，并保留变更日志与前后对比。MILESTONES §4.10 要求 Fable 审阅：哪些数值被改、依据、前后对比。
约束：
- 数值唯一真源是 `/data`；客户端/服务器不得硬编码（ARCH §4.4）。
- 取舍规则（REF §6.1）：外形/口径/弹容量/射速等**史实类**以史实为准（`source: historical`）；本任务**不改 `historical` 字段**。偏离史实的 `gameplay` 值必须写 `deviation_reason`。
- `/data` 字段状态：`initial` → `calibrated`（已对标且通过 §7.3 容限）→ `locked`（已有回归断言）。**平衡模拟调参不能把字段升为 `calibrated`**（那要求对标测量）；仅更新值并在字段旁注记 `tuned_by: "M3-047"`。已 `calibrated/locked` 的字段如需改动，必须先写 PLAN_ISSUES 并经 Fable 同意。
- GD §21 的待调参总表是调参对象索引。
- bot ≠ 真人（ARCH §17.3）：只修"明显异常"（一边倒、某武器占比 > 25% 或 < 0.5%），不"为 bot 优化手感"。

## 具体内容
### 要创建/修改的文件
- `data/**/*.json` —— 仅 `gameplay` 数值（伤害、冷却、资源成本、能力半径等）；每处改动对应日志条目。
- `data/tuning/m3-pass1.md` —— 变更日志（新建）：表格列 `#`、文件、字段路径、旧值、新值、单位、依据（balance 报告 CSV 行/指标名）、预期影响、风险、是否触及 `calibrated/locked`（必须为"否"，否则附 PLAN_ISSUES 编号）。
- `data/tuning/m3-pass1-before-after.md` —— 重跑 balance-sim 的前后对比表（胜率、武器占比、局时长、载具击杀比）。
- `reports/balance/m3-after-tuning-*.csv` —— 重跑结果（沿用 M3-046 命令，同种子序列）。
- `docs/PLAN_ISSUES.md` —— 只追加条目（无法在 /data 内解决的问题，如 bot 行为缺陷）。
### 要实现的行为
1. 读取 `tuning_candidates.md`，逐项分类：①数值问题（本任务处理）；②bot 行为问题（登记 PLAN_ISSUES，不改数值掩盖）；③规则问题（登记 PLAN_ISSUES，由 Fable 裁决）。
2. 每次只改一类相关字段（先武器，再载具/能力，再资源），每改一批重跑 20 局冒烟确认方向；最后重跑完整 400 局。
3. 改动幅度原则：单次对单字段的调整 ≤ ±25%（超出需在日志说明理由）【待调参】；史实类字段禁止改动。
4. 每处改动在 `/data` 对应字段旁保留旧值注释（`_prev` 或日志编号），便于回滚；`DATA_HASH` 因此变化，需更新依赖固定哈希的确定性基线（`cargo test` 的 golden hash）。
5. 重跑后：越界项清零，或未清零项逐条登记 PLAN_ISSUES 并写明原因（例如需真人试玩/对标测量）。
6. 日志中把"待对标项"（`【待测 → REF §x】`）标出，供 M3-052 回填后复核。
### 接口与数据
- 变更日志条目示例：`| 3 | data/vehicles/ger_tiger_1.json | gameplay.spawn_cost.fuel | 200 | 180 | F | 载具击杀比 2.9（阈值 ≤ 2.0 诊断） | ...`

## 不做什么
- 改 `historical` 字段或史实复核（M3-051）；对标测量（M3-052）。
- 改规则/代码以达成平衡（登记 PLAN_ISSUES）。
- 把调参结果当作真人手感定稿（只是首轮）。

## 验收标准
1. 重跑 balance-sim（同 400 局矩阵、同种子序列）：阵营胜率 40–60%、无武器占比 > 25% 或 < 0.5% 的项数 = 0；未清零项均有 PLAN_ISSUES 编号（逐项列在日志末尾表）。
2. 100% 的 `/data` 改动都在 `data/tuning/m3-pass1.md` 有一行（脚本 `git diff --stat data/` 的字段改动数 = 日志行数，`tools/data-validate --diff-count` 核对）。
3. 未触及 `historical` 字段、未触及 `calibrated/locked` 字段（`tools/data-validate` 的检查通过，违规数 0）。
4. `data-validate` 通过；`cargo test --workspace` 通过（含更新后的 golden hash）；`DATA_HASH` 变化已记录。
5. 每处改动附前后对比：日志的"旧值/新值"列完整，`before-after.md` 含至少 8 项指标的前后值。
6. 单字段单次改动幅度 > ±25% 的条目数 = 0，或每条有理由说明。

## 测试要求
- 单元：`data-validate` 的 diff 检查；golden hash 更新。
- 集成：balance-sim 全量重跑（400 局），时间 > 30 min 时可在专用主机运行，归档 CSV。
- 性能基准：无。
- 回归：已 `locked` 的数值断言（REF §7.5）全部通过。

## 性能预算
- 无（不新增运行预算）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 调参取舍需 Fable/负责人审阅（MILESTONES §4.6、§4.10）；执行者不得自行"定稿"。
- 过拟合 bot 的风险：若某数值问题仅在 bot 对战中出现且真人体验未知，倾向登记而非改动。
- 与 M3-052（对标测量）并行时，对标回填值优先于平衡调参值（REF §6.1：手感类以对标为准），冲突写 PLAN_ISSUES。
