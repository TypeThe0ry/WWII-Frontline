# M0-007 sim：World/step/确定性底座

## 元数据
- 里程碑：M0
- 工作流：sim
- 规模：L
- 前置依赖：M0-005, M0-004
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §4.3.2; ARCHITECTURE §8.1; ARCHITECTURE §6.1; ARCHITECTURE §17.1; ARCHITECTURE §5.2

## 背景
`sim` 是"一份代码两处运行"（服务器原生 + 客户端 WASM 预测，P2）的确定性模拟核心。本任务只建**底座**：`World`、`step`、稳定的 `EntityId`、确定性 RNG、`state_hash`、只读 `WorldView`、`GameData` 接入；移动/武器/死亡分别在 M0-011/012/013。约束全部来自 ARCHITECTURE §4.3.2：以 `bevy_ecs`（仅该 crate，不用 Bevy App）组织数据；**禁止** `tokio`、任何 I/O、`std::time::{Instant,SystemTime}`、`rand::thread_rng`、`std::collections::HashMap/HashSet`（用 `indexmap` 或 `BTreeMap`）、`rayon`/线程、`wasm-bindgen`；全部 `f32`，数学函数走 `libm`；`bevy_ecs` 必须单线程 executor（§5.2 风险④）。
确定性规则（§8.1，强制）：①`step` 的输入按 `PlayerId` 升序；②ECS 查询结果须按 `EntityId` 显式排序后再迭代；③随机数通过 `Rng::for_(tick, entity, purpose)` 派生，`Pcg64Mcg`，种子 = `hash(match_seed, tick, entity_id, purpose)`；④不用平台 `sin/cos`；⑤禁 rayon。`EntityId = u16`，"稳定槽位 + 代数在高 4 位"（12 bit 槽位 + 4 bit 代数）。`tick: u32` 环绕比较。

## 具体内容
### 要创建/修改的文件
- `crates/sim/Cargo.toml` —— 依赖：`protocol`、`bevy_ecs`（`default-features=false`，关闭 multi-threaded，锁小版本）、`glam`、`libm`、`indexmap`、`rand_core`、`rand_pcg`、`serde`；`[features] client-predict = []`；M0-010 起再加 `rapier3d`。
- `crates/sim/src/lib.rs` —— 模块导出。
- `ids.rs`（`PlayerId=u8`、`EntityId` 编解码、`Tick` 环绕比较 `lt/le/diff`）、`rng.rs`（`Rng::for_(match_seed,tick,entity,purpose)->Pcg64Mcg`；混合函数固定为文档化的 splitmix64 式 64 位混合，`Purpose` 枚举：`Spread, Respawn, Bot, Misc…`）、`hash.rs`（自实现 FNV-1a 64 或 xxh3 常量版，**不得**用 `std::hash::DefaultHasher`；`state_hash` 通过 `HashWriter` 逐字段写入 f32 位模式）。
- `world.rs` —— `World::{new(cfg:SimConfig, collision:&CollisionData), step(inputs:&[(PlayerId,InputCmd)])->StepOutput, spawn_player(SpawnReq)->Result<EntityId,SpawnError>, remove_player(PlayerId), view()->&dyn WorldView, tick(), state_hash()->u64}`；`SimConfig{tick_hz:u8(30),match_seed:u64,map:MapId,mode:GameMode,data:&'static GameData}`；`GameMode{Frontline,Assault,Skirmish,Sandbox}`（M0 使用 `Sandbox`，对应 `--mode sandbox`；新增变体在 PLAN_ISSUES 记"细化"）；`CollisionData` 此处仅为空壳（`CollisionData::empty()`），M0-010 实装。
- `components.rs` —— `Player{id}`、`Transform{pos:Vec3,yaw:f32,pitch:f32}`、`LastInput{cmd:InputCmd}`、`Team`；`schedule.rs` —— 单线程 `Schedule`，系统顺序用 `.chain()` 固定：`ingest_inputs → (后续任务的系统插槽) → bookkeeping`。
- `view.rs` —— `trait WorldView`（ARCHITECTURE §8.1：`players()/entity_state(id)->protocol::EntityState/los()/height_at()/…`；M0 先实现 `players`、`entity_state`，其余返回占位并 `todo` 由 M0-010/012 补）。
- `step.rs` —— `StepOutput{events:Vec<SimEvent>, rule_changes:Vec<RuleChange>}`、`SimEvent` 枚举骨架。
- `tests/determinism_basic.rs`。
### 要实现的行为
1. `step` 先断言输入 `PlayerId` 严格升序（debug 断言 + release 下排序保护），`tick += 1`，把每个玩家的 `InputCmd` 的 yaw/pitch 写入 `Transform`（移动留给 M0-011）。
2. 所有遍历按 `EntityId` 排序后进行；提供 `sorted_players()` 辅助，禁止裸 `Query::iter()` 参与会影响状态的计算（用 clippy 自定义 lint 或 code review 清单，写入模块文档）。
3. `state_hash` 覆盖：`tick`、全部实体按 `EntityId` 序的 `Transform` 位模式、`LastInput`、RNG 流位置；与 `tick_hz` 之外的运行环境无关。
4. 测试用 feature `test-util`：额外系统每 tick 对每个实体抽 1 次 `Rng::for_(.., Purpose::Misc)` 的随机数并抖动位置，用来证明 RNG 与顺序确定。
5. `GameData`（由 M0-004 的 `build.rs` 产出）接入 `SimConfig`；`data.hash` 暴露为 `World::data_hash()`。
### 接口与数据
- 以 ARCHITECTURE §8.1 的签名为准，不得与之冲突；`StepOutput/SimEvent` 的变体由后续任务追加。

## 不做什么
- 不做移动/碰撞（M0-010/011）、武器与弹道（M0-012）、生命与重生（M0-013）。
- 不做 `LagCompHistory`（M1-025）、`PredictSim`（M0-011 起，feature `client-predict`）。
- 不做 WASM 绑定（M0-014）；不做网络序列化（`protocol`）。

## 验收标准
1. 测试 `determinism_basic`：相同 `match_seed` + 相同 10 个玩家的输入序列，运行两次各 10 000 tick，**逐 tick** `state_hash` 完全相同（10 000 / 10 000 相等；两次运行在不同线程、不同堆分配顺序下执行）。
2. 改变 `match_seed` 或任意一个输入位，`state_hash` 在 ≤ 2 tick 内出现差异（用 100 次随机扰动验证 100/100）。
3. `node tools/ci/check-sim-purity.mjs`、`node tools/ci/check-deps.mjs` 通过；`cargo tree -p sim` 不含 `tokio/rayon/wasm-bindgen/getrandom(std)`。
4. `cargo build -p sim --target wasm32-unknown-unknown` 与 `--features client-predict` 均通过。
5. 输入未按 `PlayerId` 升序时：debug 构建 panic 带明确信息；release 构建结果与已排序输入完全一致（单测各一例）。
6. `EntityId` 槽位复用：移除再生成同槽位后代数 +1，旧 id 查询返回 `None`（单测）。

## 测试要求
- 单元测试：`Rng::for_` 同参同流、异参异流；`Tick` 环绕比较（`u32::MAX` 附近）；`HashWriter` 对 `-0.0/NaN` 的位模式处理有明确约定并测试。
- 集成/无头测试：无（服务器接入在 M0-015/018）。
- 性能基准：`cargo bench -p sim world_step_empty`（10 玩家，仅记录，M0-030 汇总）。
- 回归：把 10 000 tick 的最终 `state_hash` 写为 golden 常量（RNG/哈希算法变更须显式更新，附原因）。

## 性能预算
本任务无新增预算；`World::step` 在 100 玩家空输入下应 ≪ ARCHITECTURE §6.10 阶段 1–2 预算（记录数值，不设门槛）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `bevy_ecs` 内部会用哈希结构管理实体/组件，本任务无法也不必禁止；确定性靠"显式排序后迭代"保证，不依赖其存储顺序。若发现 `bevy_ecs` 版本在 `wasm32` 下需要额外 feature，写 PLAN_ISSUES。
- 版本锁定：`bevy_ecs` 版本迭代快（ARCHITECTURE §5.2 风险②），Cargo.toml 用精确版本。
- R5：不要求 WASM 与原生逐比特一致；本任务的哈希比较只在同一目标内（native/native）阻断。跨平台比较在 M0-029。
