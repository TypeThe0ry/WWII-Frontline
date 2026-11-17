# M0-011 sim：Rapier 角色控制器与基础移动

## 元数据
- 里程碑：M0
- 工作流：sim
- 规模：L
- 前置依赖：M0-010
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §5.3; ARCHITECTURE §6.10; ARCHITECTURE §8.1; ARCHITECTURE §20.1; GAMEDESIGN §9.1; GAMEDESIGN §9.2; GAMEDESIGN §0.5

## 背景
这是 E0-3 的承载任务：验证 Rapier `enhanced-determinism` 的 KCC 单次耗时 ≤ 20 µs（ARCHITECTURE §20.1-2），100 个 KCC 一 tick ≤ 3 ms（§6.10 阶段 2）。若超标，启用 §5.3 降级方案——"简单胶囊 vs 高度场 + 静态 AABB 的自写 KCC"——并写 PLAN_ISSUES。同时提供**服务器与客户端预测共用**的 `step_player`（ARCHITECTURE §8.1 的 `PredictSim::step_player`），保证预测与权威同源（P2）。
M0 范围（INDEX）："走/冲刺/蹲/跳"。速度来自 GAMEDESIGN §9.1（站立走 2.0、跑 4.4、冲刺 6.4、蹲姿跑 2.4/走 1.5 m/s，均【待调参/待测】）；碰撞胶囊半径 0.35 m，高度站 1.8 m/蹲 1.2 m（GAMEDESIGN §0.5）；站→蹲/蹲→站各 0.25 s（§9.2）；跳跃起跳高度 0.6 m（§9.3，初速 = √(2·g·0.6) ≈ 3.43 m/s，g = 9.81）。**趴姿、体力、翻越、探身属 M1-011**，M0 冲刺无体力限制、跳跃无体力消耗。数值必须来自 `/data`，不得写进代码（ARCHITECTURE §4.4）。

## 具体内容
### 要创建/修改的文件
- `data/roles/rifleman.json`（`id:"rifleman"`，`movement{walk_mps:2.0,run_mps:4.4,sprint_mps:6.4,crouch_mps:2.4,prone_mps:0.9,stamina_max:100}`，`crouch_walk_mps:1.5`，`jump_height_m:0.6`，`stance_transition_ms:250`，`capsule{radius_m:0.35,height_stand_m:1.8,height_crouch_m:1.2}`，`health:100`，`faction_overrides.us/ger.loadout_ids:[]`（M0-012 填充），`_meta.status:"initial"`、`tunable` 列表）。如 M0-004 schema 缺这些字段，按其流程追加可选字段。
- `crates/sim/src/movement/{mod.rs,kcc.rs,stance.rs,params.rs,state.rs}`：`MoveParams`（由 `RoleDef` 构造）、`PlayerAuthState{pos,vel,yaw,pitch,stance,grounded,stance_timer,…}`、`step_player(state:&PlayerAuthState, cmd:&InputCmd, params:&MoveParams, col:&CollisionData)->PlayerAuthState`（**纯函数**，不触碰 `World`）。
- `crates/sim/src/predict.rs`（`#[cfg(feature="client-predict")]`）：`PredictSim::{new,reset_to,step_player,state}`（ARCHITECTURE §8.1）。
- `crates/sim/benches/kcc.rs`、`crates/sim/tests/movement_*.rs`。
### 要实现的行为
1. 输入解码：`move_x/move_z ∈ -127..127` 归一化到长度 ≤ 1（`hypot` 用 `libm`）；`yaw/pitch` 来自 `InputCmd`；朝向矩阵用 `libm::sinf/cosf`。
2. 速度选择：`buttons` 的 sprint（bit6）→ 冲刺（仅向前分量 > 0 时生效）；crouch（bit4）→ 蹲姿速度；默认跑；走（Shift 对应 `aux` 的 walk 位，见 `aux` 约定写入模块文档）；加速度模型 M0 取"即时到目标速度 + 地面摩擦"（平滑加速待 REF M-24–M-26 测量后改，写 PLAN_ISSUES 提示）。
3. KCC：Rapier `KinematicCharacterController`（`snap_to_ground`、`max_slope_climb_angle = 35°`、`min_slope_slide_angle`、autostep 最大 0.3 m【待调参】）对 M0-010 的高度场碰撞体移动胶囊；站/蹲切换时胶囊高度按 `stance_transition_ms` 线性过渡，M0 无顶部障碍故可恒站起。
4. 跳跃：`jump`（bit3）在 `grounded` 时给垂直初速；重力 9.81 m/s²（GAMEDESIGN §0.5）；空中保留水平速度，空中控制系数【待调参】取 0.3，写入数据。
5. `World::step` 对每个玩家调用 `step_player`，把结果写回组件，所有 100 个玩家顺序固定（EntityId 升序）。
6. 越界保护：玩家位置超出已载入碰撞区块时不得继续前进（`step_player` 返回原位置并置 `blocked=true`，ARCHITECTURE §9.3 失败处理）。
### 接口与数据
- `PlayerAuthState` 的序列化位布局与 `self_state`/`Predictor.reset(authState)` 的字节表示在本任务定义（`to_bytes/from_bytes`，小端），供 M0-014/M0-022 使用；字段变更须同步 M0-014。

## 不做什么
- 不做趴姿、体力、翻越/攀爬、探身、姿态对散布修正、水/雪地减速（M1-011）。
- 不做玩家之间碰撞（ARCHITECTURE §6.4：玩家互不阻挡）；不做载具（M1-022）。
- 不做射击（M0-012）；不做服务器输入缓冲/校验（M0-018）。

## 验收标准
1. `cargo bench -p sim kcc_step`（criterion，release，`enhanced-determinism`，参考机）单次 KCC 中位数 ≤ 20 µs；机型与 CPU 写入 STATUS。若 > 20 µs：启用 ARCHITECTURE §5.3 降级方案并写 PLAN_ISSUES，此项记为 `partial` 直至降级方案达标。
2. `cargo bench -p sim kcc_100_players_tick`：100 个玩家在 M0 地形随机走动，一次 `World::step` 的移动阶段中位数 ≤ 3 ms（ARCHITECTURE §6.10 阶段 2）。
3. 速度精度：测试 `speed_matches_data` 在平地直线运动 3 s，测得稳态速度与 `/data` 的 walk/run/sprint/crouch 值相对误差 < 2%（四项全测）。
4. 跳跃：峰值高度与 `jump_height_m`（0.6 m）误差 < 5%；落地后 `grounded=true`。
5. 坡度：在 M0-009 的陡坡（> 35°）上以 run 速度推进 5 s 不得爬上；≤ 30° 坡可上坡且速度 ≥ 水平速度的 70%。
6. 确定性：`step_player` 对相同输入序列（10 000 步，含随机跳/蹲/冲刺）两次运行位置位模式逐步相同；`PredictSim`（feature `client-predict`）与 `World` 单玩家结果逐步位模式相同。

## 测试要求
- 单元测试：输入归一化、姿态过渡计时（250 ms = 7.5 tick 的取整规则写明并测试）、边界（区块外、NaN 输入被钳制为 0）。
- 集成/无头测试：`movement_world_vs_predict` 对拍；`movement_replay_10k`。
- 性能基准：`kcc_step`、`kcc_100_players_tick`（基线 JSON 存 `reports/baselines/sim-m0.json`，M0-030 引用）。
- 回归：golden 轨迹哈希。

## 性能预算
单次 KCC ≤ 20 µs；100 个 KCC ≤ 3 ms/tick（ARCHITECTURE §6.10 阶段 2，§20.1-2）。超标触发 §5.3 降级方案。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 这是 M0 最大的技术风险之一（MILESTONES §1.8）；"参考机"不可用时本机数字只能标"本机，非参考机"，该项记 `partial`（README §5.5）。
- 速度/姿态数值均 `initial`，待 REF M-01–M-06、M-12–M-14、M-17、M-19、M-24–M-26（M0-032）回填。
- `aux` 字段用途（走/开火模式切换）在 ARCHITECTURE §6.3 只写"开火模式切换/标记"，walk 位的约定属于细化，写入 PLAN_ISSUES。
