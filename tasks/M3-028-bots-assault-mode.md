# M3-028 bots：Assault 模式 bot 行为

## 元数据
- 里程碑：M3
- 工作流：bots
- 规模：M
- 前置依赖：M3-026, M3-020
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §3.4; GAMEDESIGN §20.3; GAMEDESIGN §20.4; ARCHITECTURE §16.1; ARCHITECTURE §16.3

## 背景
M3-020 在 `crates/sim` 实现了 Assault 规则（GD §3.4）；M3-026 让规则指挥官会用全部 8 种能力。本任务让 bot 在 Assault 模式下"打得像在进攻/防守"，使 M3-046 平衡模拟能产出有意义的数据。Assault 要点（GD §3.4）：
- 非对称：攻方（Carentan 测试配置为 US）从 L1 起逐段推进，仅"当前目标点"活动；占领成功 +8 min（`capture_bonus`，剩余总时间上限 60 min），初始 20 min。
- 阶段时间用尽时给 60 s 争夺窗口（攻方 `p > 0` 且在区内则延长，最多再 90 s）【待调参】。
- 防守方不能在已被攻占的 Sector 建 Garrison；攻方不能在未占领 Sector 建 Garrison（Outpost/OP/Airhead 可）。攻方起始资源 +25%；防守方 Garrison 上限 +2【待调参】。
- 防守方获得后撤点（`fallback_spawn`）作为额外 HQ 重生点。
bot 通过与真人同构的 `InputCmd`/指挥意图工作，不得直接改 `World`（ARCH §4.3.6）；无透视（GD §20.3 B-05）。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/assault/mod.rs` —— `AssaultDoctrine`：按阵营角色（Attacker/Defender）选择战略姿态。
- `crates/bots/src/assault/attacker.rs` —— 攻方：集结点选择、推进序列、占领期间站位。
- `crates/bots/src/assault/defender.rs` —— 防守：纵深防御点（取地图 `cover_points`）、反击触发、后撤重组。
- `crates/bots/src/commander_rule.rs`（改）—— 规则指挥官读取 `MatchPhase::Assault{stage, attacker}`，调整优先级与资源保留（C-01/C-02）。
- `crates/bots/src/squad_leader.rs`（改）—— 小队长目标选择接入"当前目标线"。
- `crates/bots/tests/assault_behavior.rs` —— 脚本化阶段测试。
- `crates/balance-sim/src/scenarios/assault_carentan_test.rs`（改/增）—— 加载 M3-004 的 `assault_stages` 测试配置；若 M2-051 的 CLI 无模式参数，补最小参数 `--mode assault`。
### 要实现的行为
1. **攻方集结**：阶段开始/刚占下上一线后，≥ 70% 的攻方 bot 小队在目标线 150 m 外的集结区汇合（≤ 40 s）再推进【阈值待调参】；小队长按 B-07 下达 Move/Attack。
2. **推进与占点**：进入 Strongpoint 半径内站位、间距 ≥ 4 m（B-02）；净人数优势时停留，被压制/敌众我寡时撤退重组。
3. **Assault 专用指挥官策略**：Recon Flight 于强攻前 60 s（C-03）；Bombing/Precision 优先打击当前线据点内聚集；攻方保留 Airhead 预算以便在目标线侧翼落点；防守方指挥官在失守前线后用 Rally 反击。
4. **防守**：占据当前线与下一线的 `cover_points`；失守后向后撤点（`fallback_spawn`）重组并在下一线布防；不在已失守 Sector 放 Garrison。
5. **时间压力**：剩余时间 < 5 min 且未占下当前线时，攻方 bot 提高冒险阈值（降低撤退条件）；争夺窗口内攻方全员压上。
6. **阶段间重组**：阶段切换后 ≤ 20 s 内小队重新分配目标（不留在旧线）。
7. **反卡死**：沿用 B-21（5 s 位移 < 1 m 重规划）。
### 接口与数据
```rust
pub enum AssaultRole { Attacker, Defender }
pub struct AssaultCtx { pub role: AssaultRole, pub stage: u8, pub time_left_s: u32, pub fallback_spawn: Option<Vec3> }
impl AssaultDoctrine { pub fn objective_for_squad(&self, ctx: &AssaultCtx, squad: &SquadView) -> Objective; }
```
`/data` 中的数值（`capture_bonus`、阶段时限）只读取，不硬编码。

## 不做什么
- 不做 LLM 指挥官的 Assault 提示词适配（M3-027 的动作枚举已够用；如需新增动作写 PLAN_ISSUES）。
- 不做 Omaha 登陆艇/滩头行为（M4-016）。
- 不调数值平衡（M3-047）；不实现 Assault 规则本身（M3-020）。

## 验收标准
1. `cargo test -p bots assault_` 全通过，覆盖攻/守、阶段切换、时间压力、后撤重组 ≥ 8 个用例。
2. 脚本化场景：Carentan 测试配置，攻方 bot 在 20 局固定种子中，阶段完成率（完成的阶段数 ÷ 总阶段数）既不为 0 也不为 100%（`balance-sim --mode assault --games 20 --seeds 1..20` 输出 `stage_completion_rate`）。
3. 20 局中"> 30 s 无进展"计数 = 0（沿用 M4-016 的同口径指标 `stall_events`）。
4. 阶段切换后 95% 小队在 20 s 内更新目标（`objective_switch_latency_s` P95 ≤ 20）。
5. 无作弊审计：bot 决策函数输入类型只含 `WorldView` 可见信息；`cargo test -p bots no_omniscience` 通过。
6. 规则校验：bot 产生的指令 0 条被服务器规则拒绝为"非法"（20 局日志统计 `rejected_orders == 0`，Garrison 限制尤其检查）。

## 测试要求
- 单元测试：目标选择函数在 5 种局面（开局/占领中/争夺窗口/失守/临近超时）的输出；防守方不选已失守 Sector 放 Garrison。
- 集成：balance-sim 20 局（上面 2、3、6）；单局 headless 回放确认 bot 沿路线推进（日志）。
- 性能基准：bot 思考合计不得超过 ARCH §16.5 的 3.0 ms/tick（60 bot）；`cargo bench -p bots think_60` 对比 M2 基线回归 ≤ 10%。
- 回归：balance-sim 报告存 `reports/balance/m3-028-assault-smoke.csv`。

## 性能预算
- 不得使 ARCH §16.5 / §6.10 阶段 6（Bot 思考 3.0 ms/tick）预算回归；以上述 bench 检测。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- bot 平衡结果 ≠ 真人（ARCH §17.3），"阶段完成率非 0 非 100%"只是烟雾测试，不代表平衡。
- Assault 的正式地图是 Omaha（M4）；此处仅验证 Carentan 测试配置，M4-016 会再适配。
- 若 M3-004 的测试配置阶段数不是 4，以实际配置为准并更新本文件。
