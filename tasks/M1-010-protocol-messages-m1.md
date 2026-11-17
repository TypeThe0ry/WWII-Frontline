# M1-010 protocol：M1 全消息集

## 元数据
- 里程碑：M1
- 工作流：net
- 规模：L
- 依赖：M0-006
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.8; ARCHITECTURE §8.2; ARCHITECTURE §6.5; ARCHITECTURE §6.9; ARCHITECTURE §19.1; ARCHITECTURE §17.1

## 背景
M0 的 `crates/protocol` 已有 BitWriter/BitReader、量化、`Hello/Welcome/Reject/Ping/Pong/InputBundle/InputCmd/FireCmd`、`SnapshotEncoder/Decoder`（基线环 32 tick、field_mask、cargo-fuzz 靶）。本任务补齐 M1 所需**全部消息**（ARCHITECTURE §6.8）：
- **C0（可靠）**：`DeployRequest/DeployResult`、`SquadOp`、`Chat`、`MarkerPlace`、`Report`（+ `Settings`、`Disconnect` 若 M0 未含）。`CommanderOrder`/`VoiceState` 留 M2/M3。
- **C4（可靠有序）**：`Kill`、`Death`、`Revive`、`ScoreDelta`、`ObjectiveChange`、`SpawnPointState`、`PlayerRoster`、`Notice`、`MatchPhase`（`ResourceDelta`/`AbilityExecuted` 留 M2）。
- **C2.events（不可靠冗余）**：`Shot`、`Impact`、`HitConfirm`、`DamageDir`、`Explosion`、`Suppress`（`Marker` 敌方标记按 R3：位置量化 5 m、≤1 Hz）。
- **实体**：载具实体与 `VehicleExtra`（`quat32` smallest-three、`turret_yaw`、`gun_pitch`、`track_flags`、`seats`）；**团队通道**编码（`TeamState`：每人 `x:11b,z:11b,role:4b,state:3b,facing:3b` = 32 bit，1 Hz；己方小队成员走高精度快照路径）；`WorldState`（1 Hz）M1 子集。
关键数字：载具 delta 记录 ≤ **24 B**（ARCHITECTURE §6.9.1）；位置量化步长 1/32 m；`PROTOCOL_VERSION` 递增且附变更说明；解码路径禁 `unsafe`，所有长度/范围检查，失败即断开（§19.1）；字符串上限：昵称 ≤ 24、聊天 ≤ 200（§19.1）。

## 具体内容
### 要创建/修改的文件
- `crates/protocol/src/c0.rs`（`ClientMsgC0`/`ServerMsgC0` 枚举与 Codec）
- `crates/protocol/src/c4.rs`（`ServerEventC4` 枚举，字段表见 ARCHITECTURE §6.8.3）
- `crates/protocol/src/events.rs`（C2.events 编解码；`event_id:u16` 去重语义）
- `crates/protocol/src/vehicle.rs`（`VehicleExtra` 编解码与 delta 字段掩码）
- `crates/protocol/src/team.rs`（`TeamState`、`WorldState`）
- `crates/protocol/src/ids.rs`（`msg_type`、`event_type:5b`、`EntityKind::{Player,Vehicle,Projectile,Deployable}`）
- `crates/protocol/src/snapshot.rs`（扩展：`entities` 支持载具记录；`events` 段支持新事件）
- `crates/protocol/tests/`（proptest 往返）、`fuzz/fuzz_targets/{c0,c4,events,snapshot_vehicle}.rs`
- `crates/protocol/CHANGELOG.md`（PROTOCOL_VERSION 变更说明；**文件内追加，不改 docs**）
### 要实现的行为
1. 每个消息实现 `Codec`（`encode(&self,&mut BitWriter)`、`decode(&mut BitReader)`），字段位宽取 ARCHITECTURE §6.8；字符串用"长度前缀 + UTF-8"，上限校验，非法 UTF-8/越界返回 `CodecError`。
2. `DeployRequest{spawn_point:id16, role:u8, loadout:u8, squad:u8}`、`DeployResult{ok,spawn_tick:u32,reason:u8}`；`SquadOp{op:{create,join,leave,kick,promote,rename,lock,unlock,invite}, squad, target, name_key}`（`lock/unlock/invite` 为 M1 补充，GD §4.3，须在 CHANGELOG 注明）。
3. `Kill{victim,killer(0xFF=环境),weapon,headshot,dist:q(12b,0..1024 m),assist1,assist2}`；`Death{victim,respawn_after:u8,can_be_revived}`；`Revive{medic,target}`；`ScoreDelta{player,kind,value:i16}`；`ObjectiveChange{objective,owner,reason}`；`SpawnPointState{id,kind∈{garrison,outpost,airhead,op,vehicle},team,pos,state}`；`PlayerRoster`（增量，`flags` 含倒地/Commander/队长）；`Notice{code:u16,args:[i32;3]}`（i18n key + 参数）；`MatchPhase{phase∈{warmup,live,overtime,ended},ends_at}`。
4. 事件：`Shot{shooter,weapon,muzzle:q(3×14),dir:q(yaw10,pitch8)}`；`Impact{pos,normal:oct12,material}`；`HitConfirm{target,zone,damage_q}`；`DamageDir{angle,dmg_q}`；`Explosion{pos,kind,radius_q}`；`Suppress{source_dir,intensity}`。
5. 载具：位置/朝向/炮塔/履带/座位/损伤字段掩码；全量记录与 delta 记录；炮塔 yaw 与炮管 pitch 共 24 bit；`seats` 位图（3 座）。
6. 团队通道：对同阵营每人 32 bit 编码；超出 49 人时按 player_id 序截断并计数。
7. 版本：`PROTOCOL_VERSION` 在 M0 基础上 +1；`CHANGELOG.md` 逐条列出新增消息与位宽；握手严格相等放行（ARCHITECTURE §8.2）。
### 接口与数据
```rust
pub enum ServerEventC4 { Kill(Kill), Death(Death), Revive(Revive), ScoreDelta(ScoreDelta),
  ObjectiveChange(ObjectiveChange), SpawnPointState(SpawnPointState), PlayerRoster(PlayerRoster),
  Notice(Notice), MatchPhase(MatchPhase) }
pub struct VehicleExtra { pub quat: [u8;4]/*smallest-three 32b*/, pub turret_yaw: u16, pub gun_pitch: u8,
  pub track_flags: u8, pub seats: u8, pub damage_flags: u8 }
```

## 不做什么
- 消息的服务器处理逻辑：M1-026；AOI 与发送策略：M1-024；延迟补偿使用 `view_tick`：M1-025。
- 客户端解码接入与 `sim-wasm` 暴露：M1-040。
- `CommanderOrder`、`VoiceState`、`ResourceDelta`、`AbilityExecuted`：M2/M3。
- 聊天内容过滤（NFKC、控制字符）属服务器 M1-026，协议层只做长度/UTF-8 校验。

## 验收标准
1. proptest 往返：每种消息各 ≥ 10^5 样本（总计 ≥ 10^6）`decode(encode(m)) == m`；`cargo test -p protocol --release` 通过。
2. 量化误差：位置 ≤ 1/64 m、yaw ≤ 0.35°（沿用 M0 阈值；炮塔 yaw 误差 ≤ 0.35°），属性测试断言。
3. 载具 delta 记录均值 ≤ 24 B、最大 ≤ 28 B（`cargo test vehicle_delta_size`，随机运动轨迹 10^4 步统计）。
4. `cargo-fuzz` 对 `c0/c4/events/snapshot_vehicle` 各 10 min 无 panic、无 OOM（CI nightly 或本地记录；`fuzz` 日志路径写 STATUS）。
5. 越界输入：字符串长度 201/25、非法 UTF-8、`n` 超限、截断包各自返回 `CodecError` 而非 panic（单测 ≥ 20 例）。
6. `PROTOCOL_VERSION` 相对 M0 递增 1，`CHANGELOG.md` 含每个新增消息条目（脚本检查消息名全覆盖）。
7. 团队通道：49 人编码一次 ≤ 200 B（含头，ARCHITECTURE §6.9.2 的 ≈ 200 B/次）。
8. `crates/protocol` 无 `unsafe`（`#![forbid(unsafe_code)]`）、`no_std + alloc` 构建通过（`cargo build -p protocol --no-default-features`）。

## 测试要求
- 单元测试：每消息往返、边界值（最大/最小/零）、非法枚举值、版本不匹配。
- 集成：`sim-wasm` 构建仍通过（体积检查 ≤ 1.2 MB，M0-014）。
- 性能基准：`cargo bench -p protocol`：载具 delta 编码 ≤ 2 µs/条、`SnapshotBuilder` 含 60 实体 ≤ 25 µs（对齐 ARCHITECTURE §6.10 阶段 9 的 ~25 µs/客户端）。
- 回归：更新 M0 的快照金样（golden bytes）并记录哈希。

## 性能预算
- 引用 ARCHITECTURE §6.9.1 实体字节数（玩家近景 10 B、载具 24 B、事件 10 B）与 §6.10 阶段 9（编码 ≈ 25 µs/客户端）；不得使 M0 的 `InputBundle(3 条) ≤ 30 B` 回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L 且消息多，建议先 C0/C4（文本量大但简单），再事件与载具；若超出可按 §5.6 追加 `M1-010b`（载具与团队通道）。
- `SquadOp` 的 lock/unlock/invite 与 `PlayerRoster.flags` 位分配为"细化"，在 PLAN_ISSUES 登记（ARCHITECTURE §6.8.1 的 op 枚举未含 lock/invite）。
- `Chat` 双向消息 `channel ∈ {all,team,squad,command}`，M1 只用前三者；`command` 频道随 Commander（M3）启用。
