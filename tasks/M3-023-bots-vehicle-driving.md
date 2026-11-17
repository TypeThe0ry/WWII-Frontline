# M3-023 bots：载具驾驶与乘员 bot

## 元数据
- 里程碑：M3
- 工作流：bots
- 规模：L
- 前置依赖：M2-029, M3-016, M3-004
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §16.2; ARCHITECTURE §16.3; ARCHITECTURE §16.5; GAMEDESIGN §20.3; GAMEDESIGN §14.7; GAMEDESIGN §20.5

## 背景
M1-037 已有基于 Recast navmesh 的步兵寻路；M2-029 完成 100 bot 规模优化（60 bot ≤ 3.0 ms/tick）。ARCHITECTURE §16.2：**载具驾驶 bot 沿道路图（独立路点图）而非 navmesh，M3**。GD §20.3 B-14：驾驶员 bot 沿路网行驶、躲避障碍、朝目标据点推进、遇敌 AT 时后撤；炮手 bot 目标选择（先 AT 威胁、再装甲、再步兵）、遵守弹种规则；车长 bot 开/关舱视野、标记。B-16：驾驶运输车把 bot 小队载往前线重生点或据点。B-21：反堵塞（5 s 位移 < 1 m）自动重新规划。
bot 必须与真人受同一规则：只能产出 `InputCmd`（含载具输入、座位切换、`aux`），不得直接改 `World`（ARCHITECTURE §4.3.6）；感知不得全知（视锥 120°、LoS、听觉事件，§16.3）。
输入数据：M3-004/M2-004 的 `vehicle_routes.json`（路点图 `nodes/edges`，宽度、路面、`vehicle_ok`、`no_go`）；载具数据与控制器（M3-014/015/016）。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/nav/road_graph.rs` —— 加载 `vehicle_routes.json`，构建加权图；A*（边权 = 长度/速度系数，坡度/泥地惩罚）；`nearest_node`、`path_to_objective(vehicle_class, from, objective)`；重算上限 2 Hz/车。
- `crates/bots/src/vehicle/driver.rs` —— 驾驶员 bot：路径跟随（纯追踪 + 速度规划，按车宽/转弯半径调速）；`InputCmd` 的油门/转向；前方障碍检测（友军/建筑/地雷已知点）；倒车脱困；遇敌 AT 威胁后撤到最近掩护节点。
- `crates/bots/src/vehicle/gunner.rs` —— 炮手 bot：目标优先级（AT 威胁 > 装甲 > 步兵），弹种选择（装甲目标 AP，步兵/软目标 HE），射击前稳定等待（炮塔稳定度），按反应时间/瞄准误差难度参数（GD §20.6）；弹药管理。
- `crates/bots/src/vehicle/commander.rs` —— 车长 bot：开/关舱（受压制/炮击时关舱）、观察与目标指派给炮手、标记（需 Spotter 权限不具备，仅指挥炮手）、M2HB 顶枪射击。
- `crates/bots/src/vehicle/crew.rs` —— `VehicleCrewCoordinator`：多 bot 乘员协同（座位分配、同时就座、驾驶员等待乘员 ≤ 10 s 后出发）；乘员缺位退化（单人驾驶/无人炮手时只机动）。
- `crates/bots/src/vehicle/transport.rs` —— B-16：运输车 bot 装载小队，送往前线重生点/据点附近下车点。
- `crates/bots/src/vehicle/stuck.rs` —— 反堵塞：5 s 位移 < 1 m → 倒车/换道/重新规划，三次失败放弃该路径并上报。
- `crates/bots/tests/vehicle_bots.rs`、`crates/balance-sim/src/scenarios/tank_run.rs`。
### 要实现的行为
1. 驾驶：沿路点图向当前活动据点推进；在据点前 80–150 m 停车占领"炮击位"（可配置）；遇敌 AT（≥ 1 次被 AT 火箭击中或视野内 AT 威胁）后撤 150 m 并换位。
2. 避障：与道路外障碍（树篱 `vehicle_blocking`、坑洞）；与友军载具保持 ≥ 6 m；不碾压友军（碾压伤害 ×1.0，GD §6.6）。
3. 乘员：驾驶员/炮手/车长三类由部署时的兵种选择（Crewman 或 Tank Commander，M3-012）驱动；无真人时 Armor Crew 小队 bot 按 `bots-squad-roles-fill`（M1-039）规则补人。
4. 难度参数：反应、瞄准误差、命中保持等沿用 GD §20.6；炮手视觉距离 300 m（Veteran）。
5. 规模约束：载具 bot 思考 10 Hz（每 3 tick 分片），与步兵 bot 总预算合计见 §16.5。
### 接口与数据
- `trait VehicleBrain { fn think(&mut self, view:&dyn WorldView, ctx:&BotCtx) -> InputCmd }` 实现于三类乘员；`BotCtx` 增加 `route_graph`、`vehicle_state`（只读）。

## 不做什么
- 炮组/车长高层（选目标指挥）与炮兵 bot → M3-025；指挥官呼叫载具策略 → M3-026；
- 步兵与载具协同（跟坦克推进）的高层策略（Backlog）；渲染/声音。

## 验收标准
1. **bot 坦克沿路线到据点成功率 ≥ 90%（10 次）**：`cargo run -p balance-sim -- tank_run --runs 10 --map carentan`，每次从 HQ 载具位出发到 L2 据点圆内，≥ 9 次成功（报告 `reports/m3-tank-run.json`）；每次用时 ≤ 路径长度/(0.6×最高速) 的 150%。
2. 卡住自恢复：构造被障碍卡住场景 20 次，≥ 18 次在 15 s 内自行脱困（`stuck_recovered_total`）。
3. 炮手：对 200 m 处 Pz IV H（灰盒）AP 首发命中率与同难度人类脚本期望的差 ≤ 10 个百分点；目标优先级序列单测 3 组场景正确。
4. 运输车 bot 把 6 名步兵 bot 送达 300 m 外下车点，成功率 ≥ 90%（10 次）。
5. 性能：10 辆载具 bot 的 `think` 合计 ≤ 0.5 ms/tick；与 60 步兵 bot 同场时总 bot 预算仍 ≤ 3.0 ms/tick（ARCHITECTURE §16.5/§6.10 阶段 6；`criterion` + 压测 profile）。
6. 合规：bot 产出的 `InputCmd` 100% 通过服务器校验（`anticheat_reject_total` = 0，10 分钟 bot 对局）。

## 测试要求
- 单元测试：路点 A*（权重、连通性）、纯追踪控制、堵塞检测、目标优先级、舱盖策略。
- 集成/无头：`balance-sim` tank_run；`headless-client` + 服务器 bot 载具 3 分钟。
- 性能基准：`crates/bots/benches/vehicle_think.rs`。
- 回归：bot 行为快照（固定种子下的轨迹哈希，信息性）。

## 性能预算
ARCHITECTURE §16.5：士兵 bot 1.0 + 路径 1.0 + 战术 1.0 ≈ 3.0 ms/tick（§6.10 阶段 6）；载具 bot 贡献 ≤ 0.5 ms（计入而非额外）；路径查询总 ≤ 1.0 ms/tick。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；若超限把运输 bot（B-16）拆为 `M3-023b`。
- 依赖已含 M3-004（`vehicle_routes.json` 补全）。
- 道路图与 navmesh 的坐标系统一（本地米坐标，MAPS §6），路点对齐误差 ≤ 1 m。
- 数值（反应/瞄准误差）与 GD §20.6 同为【待调参】。
