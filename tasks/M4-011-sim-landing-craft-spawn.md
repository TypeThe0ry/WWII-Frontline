# M4-011 sim：登陆艇出生与滩头规则

## 元数据
- 里程碑：M4
- 工作流：sim
- 规模：M
- 前置依赖：M4-004, M3-020, M1-019
- 素材需求：V-VEH-*（灰盒可）；素材状态：placeholder
- 引用文档：GAMEDESIGN §10.1; GAMEDESIGN §2.6; GAMEDESIGN §10.6; GAMEDESIGN §3.4; MAPS §4.6; MAPS §4.8; MAPS §6.2; ARCHITECTURE §4.3.2; ARCHITECTURE §8.1; ARCHITECTURE §6.8

## 背景
Omaha 的 US HQ 是离岸"Landing Craft Anchorage"（不可进入的水上出生区），玩家通过登陆艇上岸；`spawns.json` 已有 `kind:"boat"`（M4-004，MAPS §6.2、§4.8）。本任务在 `crates/sim` 实现该出生点类型与登陆艇生命周期，使其与 Assault 阶段、重生波次、出生保护统一。
已有规则（GAMEDESIGN）：
- 重生类型表 §10.1：HQ 波次 15 s、Strongpoint 波次 15 s、Garrison 个人 8 s、Outpost 12 s、Airhead 5 s、OP 15 s；本任务新增 `Boat` 类型，**数值【待调参】**（初值：登陆艇等同 HQ 波次 15 s，容量/航行时间见下）。
- 出生保护 §2.6：3 s 内伤害 ×0.0，移动/射击即取消；仅非前线重生点（HQ、Strongpoint 内部、Airhead 内部）有效，Garrison/Outpost/OP 不给保护。登陆艇到岸点属"前线点"，**默认不给保护**，但需要"滩头出生保护"例外：登陆艇载员在跳板放下后 3 s 内保护，移动/射击取消（GAMEDESIGN 无此条，**作为新规则**，数值待调，写 PLAN_ISSUES）。
- 防"重生杀"与出生安全 §10.8（需阅读）；Assault 防守方 Garrison 规则 §3.4。
- 确定性：不得使用墙钟/HashMap/thread_rng（ARCHITECTURE §4.3.2）；随机用 `Rng::for_(tick, entity, purpose)`。
- 协议：`SpawnPointState.kind` 目前枚举 `{garrison,outpost,airhead,op,vehicle}`（ARCHITECTURE §6.8），需新增 `boat`；这是 protocol 的**增量变更**，须 bump `PROTOCOL_VERSION` 并更新 §6.8 变更说明（PLAN_ISSUES 登记）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/spawn/boat.rs` —— `BoatSpawn` 组件/资源、登陆艇生命周期状态机。
- `crates/sim/src/spawn/mod.rs` —— 注册 `SpawnKind::Boat`，队列与波次逻辑接入。
- `data/spawns/boat.json` 或并入 `/data/rules/spawn.json` —— 航行时间、容量、跳板时间、保护时长、冷却（带单位与 `initial` 状态）。
- `crates/protocol/src/messages.rs` —— `SpawnPointState.kind` 增加 `boat`；更新 `PROTOCOL_VERSION`。
- `crates/sim/tests/boat_spawn.rs`。
### 要实现的行为
1. 生命周期：`Anchorage(待命) → Sailing(航行) → Beached(抵滩) → Unloading(跳板放下) → Departing(离开)`；每艘艇容量 `craft_capacity`（初值 16 人【待调参】）；一波满员或到达发艇时刻即出发。
2. 出生选择：玩家部署时选"登陆艇波次"，被分配到下一艘有空位的艇；进入 `Sailing` 后不可更换重生点；航行中玩家处于"载员"状态（不可被伤害、不可操作，可看地图/聊天）。
3. 抵滩：到达 `spawns.json.boat_spawn_points[i].pos_beach` 后 `Beached`，经 `ramp_time_s`（初值 3 s）进入 `Unloading`，载员在跳板点周围 8 m 内出生（GAMEDESIGN §2.6 出生位置规则），出生保护 `protect_s`（初值 3 s，移动/射击取消）。
4. 艇可被摧毁吗：初版**不可被伤害**（无实体伤害模型），只能被"抵滩点封锁"（15 m 内存活敌人 ≥ 1 时，该艇推迟卸载最多 10 s 并改选备用抵滩点）；该规则数值【待调参】。
5. Assault 联动：攻方 HQ 区为 `Anchorage`；占领 L3 并启用 Garrison 后，登陆艇出生点按 `stage` 失效（MAPS §4.6 "海滩上不设 Garrison"）；由 `spawns.json` 的 `active_stages` 控制。
6. 确定性 + 回放：同 seed + 同输入，艇抵滩 tick 逐次相同（`state_hash`）。
### 接口与数据
- `pub enum SpawnKind { Hq, Strongpoint, Garrison, Outpost, Airhead, Op, Boat }`；`pub struct BoatWave { craft_id, state, eta_tick, seats_taken, beach_point }`。
- 可观察事件 `SimEvent::BoatState{craft_id,state,eta_ticks}` 供 server 分发（C4/团队通道，沿用既有事件，不新建通道）。

## 不做什么
- 不做登陆艇的渲染/动画（用灰盒，渲染在 world/render 后续，素材接入在 M4-029）。
- 不做 UI 出生界面（M4-023）。
- 不做 bot 使用登陆艇的行为（M4-016）。
- 不做载具类登陆艇（可驾驶 LCVP）——Backlog。

## 验收标准
1. 登陆艇到岸时刻与 `data` 配置一致（±1 s，即 ±30 ticks）：单测在固定 seed 下断言 `Beached` tick = `depart_tick + sail_ticks`。
2. 出生保护单测：保护期内伤害 ×0.0；移动或射击立即取消；`protect_s` 到期自动取消；非艇出生点行为不变（回归测试）。
3. 容量边界：`craft_capacity` 人满后下一名玩家分配到下一艘艇；单测覆盖 0/1/满/溢出。
4. 封锁规则：抵滩点 15 m 内存在存活敌人时推迟卸载 ≤ 10 s 并改选备用点；单测断言。
5. 确定性：同 seed 同输入跑 1 万 tick，`state_hash` 逐 tick 相同（`cargo test -p sim boat_determinism`）。
6. `cargo test -p protocol`：`SpawnPointState` 往返测试含 `boat`；`PROTOCOL_VERSION` 已递增，旧版客户端握手被 Reject（沿用 M0-005 的版本测试）。

## 测试要求
- 单元测试：生命周期状态机每个转移；容量；封锁；`active_stages` 失效。
- 集成/无头测试：`headless-client` 加入 Omaha（灰盒）→ 选择登陆艇出生 → 上岸后行走 30 s；断言出生位置位于跳板点 8 m 内。
- 性能基准：`cargo bench -p sim spawn_tick` 单 tick 额外开销 ≤ 0.05 ms（规则阶段预算 0.7 ms，ARCHITECTURE §6.10 阶段 5）。
- 回归：确定性哈希基线更新；`DATA_HASH` 更新。

## 性能预算
- 规则阶段（§6.10 阶段 5）总预算 0.7 ms/tick，本任务增量 ≤ 0.05 ms；不得使既有预算回归（CI `cargo bench` 对比）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 新增规则（艇出生保护例外、封锁推迟）与新增 `SpawnKind::Boat` 在 GAMEDESIGN/ARCHITECTURE 中无正文定义：按 README §5.1 写 PLAN_ISSUES 并由 Fable 回写文档；不得擅自改章节编号。
- 数值全部为【待调参】，校准来源是内部试玩与 M4-039 的平衡模拟，无 REFERENCE_MEASUREMENTS 对应项。
- 登陆艇容量与航行时间影响 Assault 节奏（US 攻方胜率 35–65% 目标），在 M4-039 重点观察。
