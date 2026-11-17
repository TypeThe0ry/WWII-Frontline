# M2-008 sim：Frontline 模式规则

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：L
- 前置依赖：M1-018, M1-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §3.3; GAMEDESIGN §3.2; GAMEDESIGN §2.2; GAMEDESIGN §3.6; GAMEDESIGN §10.5; ARCHITECTURE §4.3.2; ARCHITECTURE §6.10; ARCHITECTURE §8.1

## 背景
M1-018 实现了占领速度公式（GD §3.2）、3 个活动据点、热身/进行/结束状态机与 M1 简化胜负；M1-019 实现 HQ 出生与 Garrison。M2 需要完整 **Frontline（5 条战线拉锯）**：开局随机选点、中线开局、活动线推进与相邻锁定、扇区（Sector）所有权、胜负与超时。本任务是 M2 关键路径起点（MILESTONES §3.9：M2-008 → M2-009 → M2-010 → M2-011 → M2-027 …）。所有规则只在 `crates/sim`（ARCH §4.3.4：`server` 内禁止实现游戏规则）；确定性约束：禁 `HashMap`/`thread_rng`/墙钟；随机用 `Pcg64Mcg`，种子 `hash(match_seed, tick, entity_id, purpose)`。
规则要点（GD §3.3）：
- 5 线 L1–L5，L1 在 A（US）侧最深；每线 2–3 候选据点；开局 `seed = hash(match_id)` 每线均匀随机选 1 个，共 5 个激活据点；Sector 取该候选配套多边形。
- 中线开局：L1、L2 属 A（p=+100），L4、L5 属 B（p=−100），L3 中立（p=0，`last_owner=none`）。
- `front_A` = 最大 k 使 L1..Lk 全属 A（L1 不属 A 则为 0）；`front_B` = 最小 k 使 Lk..L5 全属 B（L5 不属 B 则为 6）。若存在中立线（`front_B − front_A = 2`）则 `Active={中立线}`，否则 `Active={L_front_A, L_front_B}`。非活动线上的 `p` 更新被忽略（锁点），也不计占领分。
- 一方持有 L1..L5 全部 → 立即获胜（`full_control`）；90 min 到时比较持有线数（`time_limit_lead`）；线数相同依次比较 ① 活动线上 `p` 之和的符号 ② 全局累计总分（GD §18.1，M2-017 前以 0 兜底）③ 平局。**Frontline 无加时**（GD §3.3.4 末句；INDEX 范围里的"加时"属 Assault，M3）。
- 防刷分：同据点 90 s 内反复易手，后续易手不计 Offense 分（此处只输出 `flip_within_90s` 标志，计分在 M2-017）。
- 占领公式沿用 M1-018：`eff(d)=min(|d|,4)+max(0,min(|d|,10)−4)×0.5`、`k_cap=0.20 %/s`、回弹 0.15 %/s；装甲乘员/运兵载具乘客/驾驶员不计入。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/rules/frontline.rs` —— `FrontlineState`、`select_candidates(seed)`、`recompute_active()`、`on_captured()`、`check_victory()`
- `crates/sim/src/rules/sector.rs` —— Sector 所有权、`SectorOwnerChanged` 处理（GD §3.3.6 表）
- `crates/sim/src/rules/mode.rs` —— `GameMode::Frontline` 分派（保留 Assault/Skirmish 空分支）
- `crates/sim/src/data/map_rules.rs` —— 解析 `data/maps/carentan/zones.json`、`spawns.json`（由 `build.rs` 嵌入，或经 `CollisionData` 旁路加载，沿用 M1-018 的方式）
- `crates/sim/tests/frontline_*.rs`
### 要实现的行为
1. `World::new` 在 `MATCH_INIT` 用种子选 5 个激活据点（每线 `candidates[i]`），写入 `RuleState`；同种子同结果。
2. 每 tick（规则阶段，ARCH §6.10 阶段 5）：仅对 `Active` 线上的据点运行 M1-018 的占领更新；其余线锁定并在 `ObjectiveView.locked=true` 暴露。
3. 占领完成 → `recompute_active()` → 发 `SimEvent::StrongpointCaptured{point,line,new_owner,tick}`、`RuleChange::FrontAdvance{front_a,front_b,active}`。
4. Sector 易手按 GD §3.3.6：X 的 Strongpoint 重生点失效、Y 的启用（`p=±100` 且圆心 80 m 内无敌，GD §10.5）；X 在 S 内的 Garrison 置 `Condemned`，30 s 后销毁且不返还 Supplies（若 M1-019 已有则复用）；Outpost 距 Y 据点圆心 < 120 m 立即销毁；资源节点销毁与战利品钩子 `on_sector_flip`（实现 → M2-009）；工事不易主（→ M2-010）；Airhead/载具不变。
5. 胜负：`check_victory()` 每次易手与每分钟各检一次；`MatchPhase` 转 `ended`，`victory_reason ∈ {full_control, time_limit_lead, tiebreak}`（GD §18.4）。
6. 计时器 90 min = 162 000 tick（30 Hz）；时间用 tick 计，不用墙钟。
### 接口与数据
- `pub fn active_lines(&self) -> &[LineId]`（长度 1–2 且相邻，供 bots/UI/M2-044 使用）；`ObjectiveView{ id, line, owner, progress:i8(-100..100), locked, contested }` 经 `WorldView::objectives()` 暴露；`WorldState`（ARCH §6.8.2）需要的 5 个据点状态与 `front_line_pos` 由 `rule_changes` 提供。

## 不做什么
- 资源产出与节点 → M2-009；建造 → M2-010；Outpost/维修站/AT 炮 → M2-011。
- 计分与经验 → M2-017；自动平衡/投票 → M2-018；战术地图叠层 → M2-044。
- Assault 的阶段加时 `+8 min` 与方向性规则、Skirmish 单点规则 → M3 / M4。
- 地图数据本身 → M2-004。

## 验收标准
1. 脚本化 20 min 模拟（`cargo test -p sim frontline_scripted_20min`）：给定固定输入序列，据点状态序列（tick, point, owner）与测试内期望表逐项一致，0 差异。
2. 活动线不变量（proptest 10 000 随机易手序列）：`|Active| ∈ {1,2}` 且两线相邻；开局 `Active={L3}`；`front_B − front_A ≥ 1` 恒成立。
3. 相邻锁定单测：开局向 L2/L4 输入占领人数，`p` 变化 = 0；L3 被 A 占领后 `Active={L3,L4}`，L4 可更新而 L2 不可。
4. 全占决胜：构造 `front_A=5` 在同一 tick 触发 `ended` 且 `victory_reason=full_control`；90 min 超时按线数、平线按 `p` 和、再平则平局，三条路径各 1 个单测。
5. 同一 `match_seed` 两次运行 `state_hash` 逐 tick 相同；不同 seed 的候选选择分布：1 000 个种子下每线各候选被选概率在 1/3 ± 5% 内（仅 3 候选的线）。
6. `p` 与 M1-018 的公式一致：净优势 10 人从 −100 到 +100 耗时 143 s ± 1 s（单测）。
7. 规则阶段 CPU：`cargo bench -p sim rules_tick` 本任务新增部分中位数 ≤ 0.2 ms（100 玩家、5 据点）。

## 测试要求
- 单元测试：`frontline_active_lines`、`frontline_victory_paths`、`frontline_sector_flip_effects`、`frontline_locked_points_ignored`、`frontline_seed_select`。
- 集成/无头：`headless-client` 5v5 + bot 跑 10 min 无 panic，据点易手被快照 `WorldState` 正确下发（M2-020 前可只验 C4 事件）。
- 性能基准：`cargo bench -p sim rules_tick`（criterion），基线放 `crates/sim/benches/baseline.json`。
- 回归：确定性哈希快照更新。

## 性能预算
ARCH §6.10 阶段 5「规则 0.7 ms」整体预算（据点+资源+补给+重生队列+指挥官能力）：本任务新增 ≤ 0.2 ms，不得使阶段 5 超 0.7 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 行已删去"加时"（GD §3.3.4：Frontline 无加时），按 GD 实现。
- 若 M2-004 的 Sector 多边形组合有缺口，`recompute` 不应 panic；以日志 + 指标 `sector_gap_total` 暴露。
- 占点参数 `k_cap` 待 REF §capture 校准（M2-054），读自 `/data`，不得硬编码。
