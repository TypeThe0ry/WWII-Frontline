# M3-005 protocol：M3 消息（能力、炮兵、标记、载具座位）

## 元数据
- 里程碑：M3
- 工作流：net
- 规模：M
- 前置依赖：M2-007
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.8; ARCHITECTURE §6.9; ARCHITECTURE §8.2; ARCHITECTURE §6.6.1; ARCHITECTURE §19.1; GAMEDESIGN §13; GAMEDESIGN §15; GAMEDESIGN §17.1; GAMEDESIGN §17.2

## 背景
M1-010 定义了 C0/C4/C2.events 全消息集与载具实体编码，M2-007 增加了 Build/Resource/Ping/Vote/Marker 扩展。M3 的 sim 规则（M3-006–M3-020）需要新的线上消息：指挥官指令细化、能力执行/拒绝/取消、炮兵指令与弹着事件、Spotter 标记、载具座位与武器位同步、载具模块状态。
`protocol` crate 约束（ARCHITECTURE §4.3.1）：`no_std + alloc`、无 I/O、禁止依赖 sim/tokio/wasm-bindgen；`PROTOCOL_VERSION` 在握手时严格相等；字段只在末尾追加，破坏性修改必须升版本（§8.2）。通道划分（§6.6.1）：丢了会造成状态永久错误的走 C4 可靠；只影响一次表现的走 C2.events 冗余；指令走 C0。
敌方信息规则（R3）：被标记/侦察暴露的敌人**只下发地图图标**（1 Hz，位置量化到 5 m，带有效期），本任务的 `Marker` 编码必须强制这一点，不得携带精确状态。

## 具体内容
### 要创建/修改的文件
- `crates/protocol/src/msg/c0.rs` —— `CommanderOrder` 细化：`ability:u8`（0..8，8=载具呼叫，id 与 `/data/abilities` 对齐，见 M3-003）、`pos:q(16×2)`、`dir:q(8)`（条带方向，0..255 → 2π）、`target:id16`（Rally 的小队或载具类型 id）、`seq:u16`；新增 `CancelAbility{ability_instance:u16}`、`VehicleCallRequest{vehicle_type:u8}`。
- `crates/protocol/src/msg/artillery.rs` —— C0：`FireMission{gun:id16, pos:q(16×2)}`、`FireMissionCorrection{gun:id16, dx:i8_step, dz:i8_step}`（步长 25 m，值域 ±4 步 = 100 m，GD §15.3）、`ArtyAim{gun:id16, azimuth:u16(0.1°), elevation:u16(0.1°), charge:u2, shell:u1}`、`ArtyFire{gun:id16}`、`ArtyLoadOp{gun:id16, op:u8}`；C2.events：`ArtyShot{gun, muzzle:q, azimuth, elevation, charge}`、`ArtyImpact{pos:q(3×14), kind:u8, eta_ms:u16}`（落点前 4 s 预警用）。
- `crates/protocol/src/msg/c4.rs` —— `AbilityExecuted`（扩展 ARCHITECTURE §6.8.3：`ability, team, pos, eta:u16(ms), instance:u16, state:u8{queued,incoming,done,cancelled}`）；`AbilityRejected{seq, reason:u8{not_commander,cooldown,resource,airlock,aircap,invalid_pos,friendly_near,dead}}`（C0 回复）；`AirWarning{pos:q, radius_q:u8, eta_ms}`（敌方警告环，只含模糊圆心）；`CommanderState{cooldown_bits, resources}`（可并入 WorldState）。
- `crates/protocol/src/msg/mark.rs` —— C2.events `Marker` 扩展：`kind:u8{enemy_inf,enemy_veh,enemy_art,enemy_atgun,enemy_garrison,...}`、`pos:q(≈5 m 量化)`、`ttl:u8`、`source:u8{spotter,officer,commander,recon}`；`MarkRequest{target:id16, hold_ms:u16}`（C0，Spotter 按住 1.2 s/Officer 1.8 s 后发送）；`OpPlaceRequest{pos:q}`。
- `crates/protocol/src/entity.rs` —— `VehicleExtra` 追加：`seat_occupancy:bits(8)`、`active_weapon:u3`、`module_flags:bits(8)`（Engine/Track/Turret/Gun/Ammo 受损态）、`hatch:u1`；`SeatChange{seat:u3}`（C0，经 `InputCmd.buttons` bit13 触发时携带 aux）。
- `crates/protocol/tests/` —— 往返、proptest、fuzz 靶；`PROTOCOL_VERSION` +1 并写变更说明（`docs/ARCHITECTURE.md` 的"变更说明"登记，不改章节编号）。
### 要实现的行为
1. 所有新消息实现 `Codec`（`encode/decode`），长度/范围检查在解码侧完成，非法枚举值返回 `CodecError` 而非 panic（ARCHITECTURE §19.1）。
2. `Marker` 编码结构上无法容纳精确 y/朝向/血量：位置只有 5 m 步长的 x、z（2 个 `q(10b)`）。
3. 新增字段仅追加在既有结构末尾；对 M2 客户端不再兼容（版本号不同直接 `Reject`）。
4. 为 `sim` 与 `server` 提供 `AbilityId`、`ArtyOp`、`SeatId` 等枚举与常量，禁止在其它 crate 重复定义。
### 接口与数据
- 沿用 `Codec`/`BitWriter`/`BitReader`（ARCHITECTURE §8.2）；量化 helper 复用 `q_pos`、`q_yaw8`。
- 载具 delta 记录上限 24 B（§6.9.1）：新增 `seat_occupancy/active_weapon/module_flags` 共 ≤ 2 B，必须通过 field_mask 仅在变化时发送，使常态记录仍 ≤ 24 B。

## 不做什么
- 消息的 sim 语义（校验、结算）→ M3-006 及后续；服务器分发/AOI 优先级 → M3-022；客户端 UI → M3-033–035。
- 不实现语音相关消息（M3-040 使用既有 `VoiceState`）。

## 验收标准
1. `cargo test -p protocol` 全绿；新增消息各至少 1 个往返样例 + proptest 往返 10^5 样本，量化误差 ≤ 既有界（位置 1/32 m，敌方标记 ≤ 5 m 步长，方位 0.1°）。
2. `cargo fuzz run decode_c0 -- -max_total_time=600`、`decode_c2events`、`decode_c4` 无 panic、无超限分配。
3. 载具 delta 记录在"全字段变化"时 ≤ 24 B，常态（仅位姿变化）≤ 20 B（单测断言）。
4. `Marker` 解码结果不含高于 5 m 精度的位置（属性测试：编码前后位置差 ∈ [0, 5 m]）。
5. 新增事件对 ARCHITECTURE §6.9.3 密集场景的带宽影响复核：按"每秒 ≤ 6 个能力/炮兵事件 × 10 B"估算，增量 ≤ 0.5 kbps，结果写入 STATUS（并在 §6.9.3 复核备注中引用）。
6. `PROTOCOL_VERSION` 递增；握手版本不等被 `Reject`（server 单测或 headless 烟雾）。

## 测试要求
- 单元测试：所有新消息往返；非法枚举/越界值解码返回错误（每个消息至少 1 例）；空包、截断包。
- 集成/无头：`headless-client` 编译通过并能解码新事件（不要求行为）。
- 性能基准：`cargo bench -p protocol` 快照编码 25 µs/客户端的既有基线不得回归 > 10%。
- 回归：协议黄金样例文件（`crates/protocol/tests/golden/`）更新并入库。

## 性能预算
- 不得使 ARCHITECTURE §6.9.5 带宽结论回归：下行均值 ≤ 256 kbps、峰值 ≤ 512 kbps；压测验收线 P50 ≤ 128 kbps、P99 ≤ 400 kbps。
- 快照编码 ≤ 25 µs/客户端（§6.10 阶段 9）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 上述字段名是"草案细化"，位宽取值若与 M1-010/M2-007 已落地的实际定义冲突，以代码为准并在 PLAN_ISSUES 记"细化"。
- 能力 id 与 `/data/abilities`（M3-003）必须同步，否则 `data_hash` 校验失败。
- 炮兵 `ArtyAim` 的方位/仰角精度 0.1°（GD §15.3）→ 16 bit 足够；服务器再次校验转速上限（4°/s、3°/s）属 M3-018。
