# M2-027 bots：Frontline 目标与战术行为

## 元数据
- 里程碑：M2
- 工作流：bots
- 规模：L
- 前置依赖：M1-039, M2-008, M2-011, M2-010, M2-015, M2-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §20.3; ARCHITECTURE §16.3; GAMEDESIGN §20.2; GAMEDESIGN §20.6; ARCHITECTURE §16.1; GAMEDESIGN §10.3

## 背景
M1-037/038/039 已有：Recast 寻路、行为树 + 效用框架、小队长 bot、Officer bot 放 Garrison、Support bot 补给、真人替换 bot。M2 要让 bot 在完整 Frontline（5 线、相邻锁定、活动线 1–2 条）下**有目的地打仗**，并使用 Outpost、建造、补给与救治（M2 新增系统：M2-008/009/010/011）。约束（ARCH §4.3.6/§16.1）：bot 只经 `WorldView` 只读，产出与真人**同构**的 `InputCmd` 与客户端命令（`BuildPlace`、放置 Outpost/Garrison 等 C0 等价），受同一套校验；**无透视**（视锥 120°、LoS、听觉事件，B-05）；难度参数见 GD §20.6（Recruit/Regular/Veteran：反应 0.55–0.85 / 0.35–0.60 / 0.25–0.45 s，瞄准误差 ×2.0/×1.3/×1.0，视距 150/220/300 m）。
效用 AI（ARCH §16.3）：`U(goal)=Π(consideration_i^w_i)` 或加权和；considerations：距离曲线、威胁、小队指令、资源（弹药/血量）、任务价值（据点优先级）、拥挤度；行为树节点 ≤ 80/bot，运行 10–30 µs。
本任务需覆盖 GD §20.3 的行为条目（M3 才有的 B-13 Sniper/Spotter、B-14 载具驾驶、B-15 火炮、B-16 运输不在 M2）：B-02 占点（前往活动线据点，在占领区内站位保持分散间距 ≥ 4 m，净人数优势时停留，被压制/敌众我寡时撤退）；B-03 防守（占 `cover_points`，朝向来敌）；B-07 小队长（选择目标据点、下 Order、≥ 2 队员在身边时放 Outpost、前线供给充足时放 Garrison）；B-08 Medic；B-09 Support（把 Supply Crate 搬到 Engineer 工地/Garrison 放置点，响应 Need Ammo/Supply）；B-10 Engineer（按资源站点/策略点建节点、障碍、铁丝，修载具，布雷并遵守 GD §12.4 的限额与位置限制，响应 Need Engineer；**不建 Garrison、AT 炮由 AT bot 建**）；B-11 Anti-Tank（接近装甲侧后方射击并转移，建/操作 AT 炮）；B-12 MG/AR（选择有射界的位置趴下部署、覆盖通道）；B-17 使用重生网络（最近可用、非封锁点，非必要不选 HQ）；B-20 倒地 30 s 无救起则放弃；B-21 卡住检测（5 s 位移 < 1 m → 重规划/跳跃/换目标）；B-22 遵守规则（无作弊）。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/goals/frontline.rs` —— `ObjectiveGoal`：据点进攻/防守效用，读 `WorldView::objectives()`（`locked`、`progress`、活动线）
- `crates/bots/src/goals/{outpost.rs,build.rs,supply.rs,medic.rs,at.rs,mg.rs}` —— 各兵种目标与行为树子树
- `crates/bots/src/perception/cover.rs` —— `cover_points`（来自地图数据，M2-004 补充字段；缺失时由 navmesh 邻近障碍启发式）
- `crates/bots/src/respawn.rs` —— 出生点选择效用
- `crates/bots/tests/scenarios/*.rs` —— 场景测试（每个 B-xx 一个）
- `crates/bots/tests/sim_battle.rs` —— 简易整局对战（20 种子 × 20 min，50 bot vs 50 bot，不依赖 balance-sim）
### 要实现的行为
1. 目标价值：活动线据点价值最高；锁定线价值 0；己方 Sector 内无敌的据点防守价值低；净人数劣势时降低进攻效用；拥挤度惩罚避免全员挤一点。
2. 占点站位：在半径内按角度均匀取点，间距 ≥ 4 m；"净人数优势时停留"：本队 `n_A−n_B ≥ 1` 且无被压制。
3. Officer：条件（≥ 2 队员 15 m 内、25 m 无敌、间距规则）全满足才放 Outpost；否则等待/移动；放 Garrison 前核对 15 m 内 Supplies ≥ 成本。
4. Engineer：按优先级队列执行：①资源节点（空闲 `resource_site`）②活动线据点后方沙袋/铁丝 ③修理 ④地雷；每项前调用 `validate_placement`（sim 提供只读函数）避免被拒重试风暴；请求限频 ≥ 2 s。
5. Support：搬运箱到 Engineer 请求点；Ammo Crate 放战斗点；每次至多 1 个未完成运输任务。
6. AT/MG：AT 发现装甲 → 侧后方路径（选择与目标方向夹角 ≥ 60° 的点）；MG 选射界 ≥ 60° 的支撑面。
7. 所有输出受现有校验与速率限制约束；bot 不聊天、不投票（B-19）。
### 接口与数据
- 沿用 `BotBrain::think(&mut self, &dyn WorldView, &BotCtx) -> BotOutput{input:InputCmd, cmds:SmallVec<ClientCommand>}`；效用曲线参数放 `data/bots/utility.json`（【待调参】，不硬编码）。

## 不做什么
- 规则式指挥官 → M2-028；100 bot 规模优化（感知缓存、思考分片）→ M2-029；载具驾驶/火炮/Sniper/Spotter → M3。
- 战术强度调参 → 平衡模拟（M2-051）与后续对标；LLM → M3。
- 改规则或校验（bot 受规则约束，不改规则）。

## 验收标准
1. GD §20.3 逐条场景通过：为 B-02、B-03、B-07、B-08、B-09、B-10、B-11、B-12、B-17、B-20、B-21、B-22 各写 1 个场景测试（共 ≥ 12），全部 `cargo test -p bots scenarios` 通过；B-13–B-16 在测试清单中明确标 `deferred(M3)`。
2. 占点站位：占领区内 8 个 bot 两两距离 ≥ 4 m（P95）；锁定线据点上 bot 数 = 0（持续 60 s 窗口，允许路过）。
3. Outpost 放置：条件满足时 ≥ 90% 的小队长在 60 s 内放置；条件不满足（队员 < 2）时放置次数 = 0（`Outpost` 规则校验拒绝数 ≤ 3/局，证明不在重试风暴）。
4. Engineer：20 min 内每队至少建造 ≥ 1 个资源节点与 ≥ 4 个其他工事；被 `validate_placement` 拒绝的重复尝试率 < 20%；无 Garrison 放置。
5. bot 进攻成功率在平衡模拟中非 0 且非 100%：`sim_battle` 20 种子 × 20 min，统计"至少发生 1 次据点易手的对局占比"，要求 ∈ (0, 100%)，且每方各自至少 1 局占点成功；结果写入 STATUS（若 0% 或 100% 记 `partial` 并登记调参建议）。
6. 单 bot 思考耗时 ≤ 60 µs（`cargo bench -p bots think_one`，Regular 难度，含感知），行为树节点 ≤ 80。
7. 无透视审计：bot 的 `think` 调用不读取视锥外敌人（测试：把视锥外敌人设置为"毒药"对象，访问即 panic 的 mock `WorldView`）。

## 测试要求
- 单元测试：效用曲线、站位算法、Outpost 条件前置判断。
- 集成：`sim_battle`（含 headless 服务器 5 min 烟测，bot 全满 100）。
- 性能基准：`think_one`、`think_60`（60 bot 平均每 tick ≤ 1.0 ms 的感知+决策，ARCH §16.5）。
- 回归：M1-038/039 场景测试保持通过。

## 性能预算
ARCH §16.5：士兵 bot 单次思考 ≈ 30–60 µs；60 bot 合计 ≈ 3.0 ms/tick（路径查询 ≤ 1.0 ms、战术/射击 ≈ 1.0 ms、思考 ≈ 1.0 ms）；本任务不得使 M1 基线回归 > 10%，规模优化见 M2-029。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L，行为条目多；若超一次会话，按"目标/站位/Outpost → 兵种行为"拆续作 M2-027b。
- 树篱/洪泛地形下 navmesh 卡死风险（MILESTONES §3.8）：B-21 的"无进展计数"必须作为指标暴露给 M2-049。
- 依赖已含 M2-010（B-10 依赖其 `validate_placement`）、M2-015、M2-019。
- `cover_points` 在 M2-004/M2-005 数据里没有明确列出，缺失时用启发式并 PLAN_ISSUES 登记。
