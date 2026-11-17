# M3-025 bots：炮组与车长 bot

## 元数据
- 里程碑：M3
- 工作流：bots
- 规模：M
- 依赖：M3-023, M3-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §20.3; GAMEDESIGN §15; GAMEDESIGN §15.3; GAMEDESIGN §15.4; GAMEDESIGN §14.1; GAMEDESIGN §16.3

## 背景
GD §20.3 B-15：炮手/装填手 bot 接受 Fire Mission，按 §15.4 计算方位/仰角并开火，按观察者修正调整。B-14：车长 bot 开/关舱视野、标记；B-11：Anti-Tank bot 发现装甲后接近至有效射程的侧后方射击并转移、**操作 AT 炮**。本任务在 M3-023（载具乘员 bot 基础）与 M3-019（炮组操作规则与 AT 炮）之上完成：**炮兵 bot**、**车长高层指挥**（选目标、给炮手指派、与步兵协同的最小集合）、**AT 炮 bot 操作**。
规则约束：bot 只能输出 `InputCmd` 与协议指令（`FireMission` 接收端、`ArtyAim/ArtyFire/ArtyLoadOp`，M3-005），受同一服务器校验；不能全知：Fire Mission 目标来自 Commander/Officer 标记（M2-019）或观察点（Spotter bot，M3-024）；弹着修正只读"观察者可见的落点偏差"（`ArtyImpact` 事件 + 观察者 LoS）。
炮兵参数（GD §15.3/§15.4）：方位转速 4°/s（微调 0.1°/s）、仰角转速 3°/s、范围 5°–85°；三档装药 Rmax 800/1800/3200 m；低角 `θ = ½·asin(R/Rmax_c)`；循环 M2A1 6.0 s（2 人）/10 s（1 人）。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/artillery/gunner.rs` —— 炮手 bot：收到 `FireMission`（含目标 pos）→ 调用 `sim` 的 `solve_elevation`（`WorldView` 暴露的只读函数）算方位/仰角/装药（默认低角、最小装药满足射程）→ 以允许的转速转炮（逐 tick 输出 `ArtyAim`）→ 对准容差内 `ArtyFire`。
- `crates/bots/src/artillery/loader.rs` —— 装填手 bot：按炮手选的装药/弹种执行 `ArtyLoadOp`（3.5 s 装填动作）；库存不足时请求 Supplies（发 Supply Request 标记，GD §16.3）。
- `crates/bots/src/artillery/observer_link.rs` —— 观察者修正：接收 Spotter bot（M3-024）/Officer 的 `FireMissionCorrection`（25/50/100 m 步长）并更新目标；炮手 bot 在收到落点偏差 > 15 m 时应用修正后再射。
- `crates/bots/src/artillery/atgun_bot.rs` —— AT 炮 bot：Anti-Tank bot 在装甲目标出现的路径上选址建造 AT 炮（200 Supplies，M2-011 规则，仅 `AT` 兵种）；炮手 bot 瞄准目标（提前量按目标速度）、AP/HE 选择（装甲 AP、步兵 HE）、装填、转架（越界时请求 Engineer，不可得则放弃）。
- `crates/bots/src/vehicle/tank_commander.rs` —— 车长 bot 高层：选目标（威胁评分：距离 × 类型权重 × 是否瞄准己方）、向炮手下达优先目标、开/关舱、与步兵小队队长协同（接收 Squad Order 攻击/防守并映射为据点接近线）；舱盖状态受压制/炮击触发。
- `crates/bots/tests/artillery_bots.rs`、`crates/balance-sim/src/scenarios/fire_mission.rs`。
### 要实现的行为
1. **Fire Mission 响应**：从收到任务到首发开火 ≤ 30 s（GD §20.3 B-15 与 INDEX 验收），包含就座、转炮、装填；预先就座时间不计入。
2. 所有射击命令合法：方位/仰角在限位内，`R ∈ [R_min, Rmax_c]`，弹药库存 > 0；非法 0 次（由服务器校验层统计）。
3. 修正：接收观察者修正后重算，3 发内收敛到目标 R < 20 m（统计，与散布 σ 有关）。
4. 人手不足：只剩炮手时单人循环（10 s）；无人时炮空闲，bot 步兵可被调度补位（Support/Rifleman 空闲 bot 就近操作，权限由 M3-019）。
5. 车长：对炮手下达目标 ≥ 每 5 s 评估一次；重型装甲优先用 AP；遇敌 AT 时指挥驾驶员后撤（与 M3-023 对接）。
6. 避免友军误伤：Fire Mission 目标 25 m 内有已知友军时，炮手 bot 拒绝开火并报告（`FireMissionDeclined`）。
### 接口与数据
- `ArtilleryBrain`/`TankCommanderBrain` 实现 `BotBrain`；与 `/data/structures/*.json`（M3-003）射表一致；`WorldView::artillery_sites(team)` 由 M3-018 提供。

## 不做什么
- 载具驾驶/炮手基础 → M3-023；Spotter bot → M3-024；Commander 的炮兵目标选择 → M3-026；
- 炮兵 UI → M3-035；渲染/声音。

## 验收标准
1. **炮兵 bot 在观测点请求后 ≤ 30 s 开火**：`balance-sim fire_mission` 20 次（Carentan 预设阵地、目标距离 600–2 400 m），响应时间中位数 ≤ 20 s、P95 ≤ 30 s（报告 `reports/m3-artillery-bot.json`）。
2. **射击命令全部合法**：整局审计 `illegal_arty_cmd_total` = 0（服务器校验计数）。
3. 修正收敛：3 发内落点距目标 ≤ 20 m 的比例 ≥ 70%（装药 2，R = 1 500 m，20 次，有 Spotter 修正）。
4. 友军误伤守卫：目标 25 m 内放友军，开火次数 = 0（5 个场景）。
5. AT 炮 bot：对 3 辆来袭坦克（灰盒 Pz IV）击穿命中 ≥ 1 发/坦克的比例 ≥ 60%（10 次，100–300 m）。
6. 车长 bot：目标切换延迟 ≤ 5 s；重型目标选 AP 的比例 ≥ 95%（统计）。
7. 性能：炮组 + 车长 bot 合计 ≤ 0.2 ms/tick（`criterion`）。

## 测试要求
- 单元测试：射表调用封装、修正算法、目标评分、友军守卫、弹药请求。
- 集成/无头：`balance-sim fire_mission`；服务器 + headless 观察炮击序列。
- 性能基准：`crates/bots/benches/artillery_think.rs`。
- 回归：bot 行为轨迹哈希（信息性）。

## 性能预算
ARCHITECTURE §16.5 bot 总预算 ≈ 3.0 ms/tick；炮组/车长 bot ≤ 0.2 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 依赖含 M3-023 与 M3-019，未显式含 M3-024（Spotter bot）与 M3-018；验收 3 的修正来源依赖 M3-024，若其未完成则用脚本化观察者替代并标 `partial`。
- 火炮 bot 的 Fire Mission 来源是 Commander bot（M3-026）或 Officer bot 的 Fire Request；若 M3-026 未完成，用 `balance-sim` 脚本下发 `FireMission` 测试。
- 弹着"观察者误差报告"依赖 M3-018 的观察者 LoS 接口。
