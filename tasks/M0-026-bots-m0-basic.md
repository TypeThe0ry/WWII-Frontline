# M0-026 bots：M0 基础 bot

## 元数据
- 里程碑：M0
- 工作流：bots
- 规模：L
- 前置依赖：M0-018, M0-012
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §16.1; ARCHITECTURE §16.3; ARCHITECTURE §16.5; GAMEDESIGN §20.1; GAMEDESIGN §20.3; GAMEDESIGN §20.6

## 背景
为 M0 的"10 名 bot 对战"提供行为。原则（ARCHITECTURE §16.1）：**bot 通过与真人完全一致的路径产出 `InputCmd`**；服务器像对真人一样校验与模拟——bot 只是"没有网络连接的客户端"，**不得直接修改 `World`**，也不得"全知"（GAMEDESIGN §20.3 B-05：只能基于视觉（视锥、LoS、可见度）与听觉；B-22：遵守同样的伤害/后坐/弹药规则，无作弊）。M0 版是简化的：没有 navmesh（M1-036/037），用**高度场网格 A\***；没有行为树/效用 AI（M1-038），用手写的小型状态机；没有指挥层（M2-028）。
分片与预算（§16.5）：士兵 bot 每 3 tick 思考一次，按 `bot_id % 3` 分片；单次 ≈ 30–60 µs（含感知）；M0 的 10 个 bot 合计 ≤ 1 ms/tick（INDEX 验收），整体 bot 预算 ≤ 3.0 ms/tick（§6.10 阶段 6）。难度档用 GAMEDESIGN §20.6 的 **Regular**（反应 0.35–0.60 s，瞄准误差 ×1.3，命中保持 4 s）作 M0 固定档【待调参】。感知：视锥 120°、距离衰减的识别概率（§16.3）。
`trait BotBrain { fn think(&mut self, view: &dyn WorldView, ctx: &BotCtx) -> InputCmd }`（ARCHITECTURE §4.3.6）。`crates/bots` 只依赖 `sim`（只读 `WorldView`）与 `protocol`（`InputCmd`），**不得依赖 `server`**（§4.5）；服务器侧的 bot 宿主在 `crates/server/src/bot_host.rs`。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/{lib.rs,brain.rs,ctx.rs,grid_nav.rs,perception.rs,aim.rs,soldier.rs,difficulty.rs}`；`crates/bots/benches/think.rs`、`crates/bots/tests/*.rs`。
- `crates/server/src/bot_host.rs`：创建 `--bots N` 个 bot（队伍均分），为每个 bot 分配 `PlayerId`、昵称前缀 `[BOT]`（GAMEDESIGN §20.2），每 tick 把 `think` 产出的 `InputCmd` **走 M0-018 的 `validate_input` 单一入口** 后送入 sim 输入；`bot_think_ms` 直方图指标；`server_bots` gauge。
- `WorldView` 补充（如缺）：`players()` 含 team/alive/pos/yaw、`height_at`、`slope_deg_at`、`los`（M0-010）；若 M0-007 的 trait 缺方法，在 sim 内补并记 PLAN_ISSUES。
### 要实现的行为
1. **导航**：把 256 m 地图划分为 4 m 格（64×64）；格可走 = 坡度 ≤ 30°（略保守于 sim 的 35°）且区块已载入；A\*（8 邻域，八向启发式），路径重算每 bot ≤ 2 Hz；路径平滑到航点序列；到点判定半径 2 m。路径查询总预算 ≤ 1.0 ms/tick。
2. **状态机**：`Patrol`（在 `spawns.json.patrol_points` 间随机选点，确定性 RNG `Purpose::Bot`）→ `Engage`（发现敌人：停步/小幅侧移，面向目标，射击）→ `Reload`（弹匣空或 ≤ 1 且无目标）→ `Dead`（等待重生，不输出移动）。`Engage` 失去目标后按"命中保持"4 s 追至最后已知位置再回 `Patrol`。
3. **感知**：每次 `think`：对每个敌人，距离 ≤ 视距（500 m 上限）、在 120° 视锥内、`los` 为真，则以 `p = clamp(1 − dist/viewdist, 0.1, 1)` 的概率在本次思考中"识别"（用 `Rng::for_(tick, bot, Bot)`，确定性）；识别后进入反应延迟计时（0.35–0.60 s 均匀）才允许开火。**不得读取视野外敌人状态。**
4. **瞄准与射击**：瞄准点 = 目标躯干中心 + 与距离相关的误差（`aim_error ×1.3`，小型高斯）；视角转动速度限制 ≤ 400°/s（远低于服务器 8000°/s 校验）；距离 ≤ 80 m 盲射，更远 ADS；Garand 半自动按边沿点射（间隔 ≥ 数据节奏），Kar98k 栓动循环期间不扣扳机；弹匣空自动 reload。
5. **卡住处理**（B-21）：5 s 位移 < 1 m → 重新选点/跳跃。
6. **输出**：每 tick 输出一条 `InputCmd`（思考 tick 更新决策，其余 tick 复用上次决策但刷新 `seq/tick` 与视角平滑）；`buttons` 只含合法位；不会产生 sprint 与 fire 同时。
7. **宿主**：`BotHost::tick(view, now)` 并按分片思考；bot 阵亡后由 sim 的重生系统（M0-013）复活，bot 无需自己处理出生。
### 接口与数据
- `BotCtx{bot_id, team, tick, difficulty:Difficulty, rng_seed}`；`Difficulty{reaction_ms:(u16,u16), aim_error_mult:f32, memory_s:f32}`（初值来自 GAMEDESIGN §20.6，M0 写在 `/data`？——难度表放 `data/rules/bots.json` 并加 schema，记 PLAN_ISSUES"细化"；代码不硬编码）。

## 不做什么
- 不做 navmesh/off-mesh/避障（M1-036/037）；不做行为树与效用 AI、兵种选择、掩体使用（M1-038）；不做小队长/指挥官 bot（M1-039、M2-028）。
- 不做载具、手雷、Medic、Engineer 等行为（M1+/M2+）；不做 LLM 指挥官（M3-027）。
- 不做 bot 的语音/聊天/投票。

## 验收标准
1. **稳定性**：10 个 bot 在服务器内对战 5 min（`cargo test -p server --test bots_soak -- --ignored`，进程内，30 Hz 不限速或实时均可），**0 panic**；期间击杀数 ≥ 10、重生数 ≥ 击杀数 − 在途数。
2. **思考耗时**：10 bot 每 tick 的总 `think` 耗时 P99 ≤ 1 ms（`bot_think_ms` 直方图；`cargo bench -p bots think_10` 中位数记录），路径查询 P99 ≤ 1 ms/tick。
3. **同路径校验**：单测 `bot_input_goes_through_validation`：构造一个故意违规的 bot（输出 `move` 模长 2.0、保留位置 1），经 `BotHost` 后被 `validate_input` 钳制/拒绝，`input_rejected_total` 增加；并断言 `BotHost` 代码不直接引用 `World` 的可变接口（编译期：`bots` crate 不依赖 `server`，`cargo tree -p bots` 无 `server`；`WorldView` 只读）。
4. **无全知**：构造敌人在视锥外/被地形遮挡的场景，bot 不转向、不开火（`perception_no_wallhack` 5 个用例）；敌人进入视锥后到首发开火间隔 ≥ 0.35 s（反应延迟下限，100 次统计最小值）。
5. **导航**：A\* 单测——平地直线、绕过 > 35° 陡坡区、目标不可达返回 `None`；10 bot 各自 60 s 内到达 ≥ 3 个不同巡逻点。
6. **确定性**：相同种子两次 `bots_soak` 的每 tick bot 输入哈希相同（5000 tick）。
7. 命名：bot 昵称均以 `[BOT]` 开头（GAMEDESIGN §20.2）；`server_bots` 指标等于配置数。

## 测试要求
- 单元测试：A\*、感知（视锥/LoS/概率）、瞄准误差分布、状态机转换、卡住检测。
- 集成/无头测试：`bots_soak`（验收 1）；M0-027 的 10 headless + 10 bot 联测复核。
- 性能基准：`cargo bench -p bots think_10`、`path_query`。
- 回归：golden bot 输入哈希。

## 性能预算
Bot 思考 ≤ 3.0 ms/tick（≤ 60 bot，ARCHITECTURE §6.10 阶段 6）；M0 的 10 bot 验收线 ≤ 1 ms/tick；路径查询 ≤ 1.0 ms/tick（§16.2/§16.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 本任务产出被 M0-027（10 headless + 10 bot 联测）直接使用，但 INDEX 里 M0-027 的依赖只有 M0-019；按编号顺序 M0-026 先于 M0-027 完成，不影响执行，仅依赖记录不完整。
- 视锥 120° 来自 ARCHITECTURE §16.3，识别概率曲线为【待调参】的简化；不要为"更强"而放宽感知限制。
- 难度表放 `/data` 是对"数值只在 `/data`"（README §5.4）的遵守；若 M0-004 的 schema 没有 bots 表，先加 schema 再用。
