# M3-006 sim：指挥官席位、资源池与指令校验

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：L
- 前置依赖：M3-003, M3-005, M2-009
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §13.1; GAMEDESIGN §13.10; GAMEDESIGN §5.4; GAMEDESIGN §16.1; GAMEDESIGN §11.4; ARCHITECTURE §4.3.2; ARCHITECTURE §8.1

## 背景
M2-009 已实现 Manpower/Munitions/Fuel 的整分钟结算与资源池（R6：与 Supplies 分离）。M3 要把"指挥官"变成一等公民：唯一席位、权限、**能力通用流程**（资源扣除→冷却→全局空中锁→延迟→生效→取消返还），以及 Command 频道权限。具体能力效果由 M3-007（空投）、M3-008（空袭）、M3-009（侦察/Rally/Airhead）、M3-017（载具呼叫）、M3-018（炮兵 Fire Mission 授权）实现；本任务只提供框架与校验，并保证"非 Commander/资源不足/冷却中的指令 100% 被服务器拒绝"（MILESTONES E3-2）。
规则只能在 `crates/sim`，不得写进 `server`（ARCHITECTURE §4.3.4）。bot 指挥官产出与真人同构指令，同样走本校验层（GD §20.4 C-06）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/commander/mod.rs` —— `CommanderSeat`：每队 1 席；`claim/release/handover`（局中让出或被投票罢免，GD §13.10/§19.3）；每名玩家一局只能当一次 Commander；Commander 兵种等级由累计指挥分决定。
- `crates/sim/src/commander/orders.rs` —— `validate_order(world, team, player, &CommanderOrder) -> Result<AbilityTicket, OrderReject>`；`OrderReject` 与 `protocol::AbilityRejected.reason` 一一对应。
- `crates/sim/src/commander/ability.rs` —— `AbilityRuntime`：每队 `cooldowns[ability]`、`air_lock_until`、`airborne_count`、`pending: Vec<AbilityInstance>`；`tick()` 推进延迟并在到期调用 `AbilityEffect::on_fire`（trait，供 M3-007/008/009/017 实现）。
- `crates/sim/src/comms/command_channel.rs` —— Command 频道成员/说话权限表（只做规则，不涉 LiveKit）。
- `crates/sim/tests/commander_orders.rs`。
### 要实现的行为
1. **席位**：仅存活且未倒地的 Commander 可下新指令；Commander 死亡/倒地时进行中的能力继续，冷却继续计时（GD §13.1）。无 Commander 的队伍：bot 接管（接口预留 `CommanderSource::{Human,Bot,None}`），`None` 时全部能力拒绝。
2. **资源**：下达时一次性扣除（`/data/abilities/*.json` 的 cost）；资源不足拒绝；取消（飞机到达前）返还 `cancel_refund = 50%`；溢出规则见 GD §11.4，不得扣成负数。
3. **冷却**：每能力独立；下达即开始计时，持续到冷却结束。
4. **全局空中支援锁**：Bombing/Strafing/Precision/Recon/Supply Drop/Ammo Drop/Airhead 任意两次之间 10 s；同时在空飞机 ≤ 3（Bombing 编队按 3 架计）；违反返回 `airlock`/`aircap`。
5. **位置合法性**：落点在地图内；Supply/Ammo Drop 须在本方或战区 Sector、距敌方据点圆心 ≥ `R_sp+80 m`（GD §13.3）；友军 25 m 内的空袭目标**允许**但在 ticket 中置 `friendly_danger=true`（供 UI 警告，ff_mult 爆炸 0.7）。
6. **Command 频道**：Commander + Officer 可说话；Tank Commander 与 Recon 队长只可听（GD §16.1）；输出 `can_speak(player)`、`listeners(team)`。
7. **事件**：生成 `SimEvent::AbilityExecuted{state: queued/incoming/done/cancelled}` 与 `ResourceDelta`；敌方警告（`AirWarning`）在 T−15 s 发出（Bombing/Strafing/Precision，GD §13.1）。
8. 所有计时以 tick 为单位（30 Hz），秒→tick 取 `round`，统一函数 `secs_to_ticks`。
### 接口与数据
```rust
pub trait AbilityEffect { fn on_queue(&mut self, w:&mut World, t:&AbilityTicket); fn on_fire(&mut self, w:&mut World, t:&AbilityTicket) -> Vec<SimEvent>; fn on_cancel(&mut self, w:&mut World, t:&AbilityTicket); }
pub struct AbilityTicket { pub id: u16, pub ability: AbilityId, pub team: Team, pub issuer: PlayerId, pub pos: Vec2, pub dir_deg: f32, pub target: Option<EntityId>, pub fire_at: Tick, pub friendly_danger: bool }
```

## 不做什么
- 具体能力效果 → M3-007/008/009；载具呼叫 → M3-017；Fire Mission 授权细节 → M3-018；
- 规则式 Commander bot → M2-028/M3-026；UI → M3-033；LiveKit 订阅 → M3-040。

## 验收标准
1. 单测矩阵：非 Commander、已死亡/倒地、无 Commander 三类发令者 × 8 项能力，拒绝率 = 100%（共 ≥ 24 个断言）。
2. 资源不足、冷却中、全局锁内、飞机上限 4 架时的指令 100% 被拒且**不扣资源**（断言资源不变）。
3. 取消返还精确为 `floor(cost × 0.5)`；取消后冷却策略与 GD 一致（取消不重置已走冷却，文档未规定处记 PLAN_ISSUES）。
4. 同 seed + 同输入两次运行 10 000 tick 的 `state_hash` 逐 tick 相同（含能力排队）。
5. Command 频道权限表与 GD §16.1 逐行一致（5 个角色 × 说/听）。
6. 能力流程每 tick 额外耗时 ≤ 0.05 ms（`cargo bench -p sim ability_tick`，100 玩家场景；计入 ARCHITECTURE §6.10 阶段 5 的 0.7 ms 规则预算）。

## 测试要求
- 单元测试：上述矩阵；`secs_to_ticks` 边界；取消返还；Commander 死亡后进行中能力继续。
- 集成/无头：`headless-client` 注入非法指令（伪造 issuer、越界 pos、超速率），服务器 100% 拒绝并计数（与 M3-045 复用脚本）。
- 性能基准：`criterion` 基线 `crates/sim/benches/ability_tick.rs`。
- 回归：确定性哈希快照更新。

## 性能预算
ARCHITECTURE §6.10 阶段 5「规则（据点、资源、补给、重生队列、指挥官能力）」合计 0.7 ms/tick；本任务贡献 ≤ 0.05 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L，接近一次会话上限：先做校验与流程骨架，Command 频道权限最后做；若超限按 README §5.6 追加 `M3-006b`。
- "取消是否重置冷却""Commander 倒地时能否取消"GD 未明文，按保守处理并写 PLAN_ISSUES。
- 本任务不依赖 M2-019（标记权限），但 Fire Mission/Rally Marker 的权限表属 M2-019 范围，勿重复实现。
