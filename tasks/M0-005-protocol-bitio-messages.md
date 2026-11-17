# M0-005 protocol：位流、量化与 M0 消息集

## 元数据
- 里程碑：M0
- 工作流：net
- 规模：L
- 依赖：M0-001
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §4.3.1; ARCHITECTURE §6.3; ARCHITECTURE §6.8; ARCHITECTURE §8.2; ARCHITECTURE §6.9; ARCHITECTURE §19.1

## 背景
`protocol` crate 是线上格式的唯一定义，服务器、`headless-client`、`sim-wasm`（客户端）共用（ARCHITECTURE §4.3.1）。它必须 `#![no_std]`（至少 `alloc`）、无 I/O、禁用 `unsafe`（§19.1 解码路径）、不依赖 `sim/tokio/wasm-bindgen/wtransport`。本任务做底座：位流读写、量化函数、版本号、`Codec` trait，以及 M0 需要的消息：`Hello/Welcome/Reject/Ping/Pong/Disconnect`（C0）与 `InputBundle/InputCmd/FireCmd`（C1）。快照编码在 M0-006，M1 消息集在 M1-010。
关键数字（ARCHITECTURE §6.3/§6.5.2/§6.9.5）：`InputCmd` = `seq u16`、`tick u32`、`move_x/move_z i8(-127..127)`、`yaw u16`（0..65535→0..2π）、`pitch i16`（→±90°）、`buttons u16`（bit0 fire、bit1 aim、bit2 reload、bit3 jump、bit4 crouch、bit5 prone、bit6 sprint、bit7 use、bit8/9 lean、bit10 melee、bit11 gadget、bit12 swap、bit13 vehicle_seat、bit14/15 保留）、`weapon_slot u8`、`aux u8`、`view_tick u16`（相对差值）。位置量化：x/z 各 16 bit、步长 1/32 m（覆盖 2048 m），y 13 bit（0..256 m，步长 1/32 m）。线上 yaw 10 bit（0.35°）/ 8 bit、pitch 6 bit（快照用，M0-006 使用）。上行预算：一个 `InputBundle`（3 条）约 25–28 B（§6.9.5 推导）。

## 具体内容
### 要创建/修改的文件
- `crates/protocol/src/lib.rs` —— `PROTOCOL_VERSION: u16 = 1`、模块导出、`#![no_std]`、`#![forbid(unsafe_code)]`。
- `crates/protocol/src/bitio.rs` —— `BitWriter`（`write_bits(v: u64, n: u8)`、`write_bool`、`write_i(v, n)` 补码、`write_bytes_len_prefixed`、`finish() -> Vec<u8>`）、`BitReader`（对应读函数，越界返回 `CodecError::Eof`，**绝不 panic**）。
- `crates/protocol/src/quant.rs` —— `quant_pos_xz(f32)->u16`、`dequant_pos_xz`、`quant_pos_y`（13 bit）、`quant_yaw(f32, bits)`、`quant_pitch`、`quant_unit(f32, bits)`；全部"夹紧后四舍五入"，NaN/Inf 输入映射为 0 并返回错误标志。
- `crates/protocol/src/codec.rs` —— `trait Codec { fn encode(&self,&mut BitWriter)->Result<(),CodecError>; fn decode(&mut BitReader)->Result<Self,CodecError> }`、`CodecError{Eof,Overflow,InvalidTag,TooLong,...}`（`thiserror` 的 `no_std` 用法或手写 `Display`）。
- `crates/protocol/src/channel.rs` —— `Channel{C0..C5}` 枚举与 `msg_type: u8` 常量。
- `crates/protocol/src/msg/c0.rs` —— `Hello{protocol_version:u16,data_hash:u64,ticket:Vec<u8>(≤256),client_build:u32,caps:u8}`、`Welcome{player_id:u8,server_tick:u32,tick_hz:u8,match_seed:u64,data_hash:u64,map_id:u8,mode:u8,team:u8,client_tick_offset:i16}`（LiveKit 字段 M2 追加，只能追加在末尾）、`Reject{reason:u8}`（枚举：版本/满员/封禁/ticket 无效/数据哈希不匹配）、`Ping{client_time:u32}`、`Pong{client_time:u32,server_tick:u32,server_time_ms:u32}`、`Disconnect{reason:u8}`；`ClientMsgC0`/`ServerMsgC0` 枚举（ARCHITECTURE §8.2，M0 只含上述变体）。
- `crates/protocol/src/msg/input.rs` —— `InputCmd`、`InputBundle{ack_snapshot_tick:u16(差值),pred_hash:u8,cmds:ArrayVec<InputCmd,3>,fire:ArrayVec<FireCmd,2>}`、`FireCmd{seq:u16,view_tick:u16,weapon:u8,dir_hint(yaw16,pitch16),origin_hint(3×14 bit),spread_seed:u16}`。后两条 `InputCmd` 相对前一条做差分编码（`tick` 必为 +1，`seq` +1 时各用 1 bit 标志位省略）。
- `crates/protocol/tests/{roundtrip.rs,limits.rs}`、`crates/protocol/benches/bitio.rs`（criterion，可选）。
### 要实现的行为
1. 所有消息 `encode → decode` 往返相等（浮点量化字段按量化后值比较）。
2. 解码对任何输入字节序列**不 panic、不无界分配**：`ticket > 256`、数组长度越界、枚举非法值均返回 `CodecError`。
3. `InputBundle` 的 `n ∈ 1..=3`（2 bit），编码字节数随差分而变；`FireCmd` 随 `InputBundle` 末尾附带最近至多 2 条（ARCHITECTURE §6.8.2）。
4. 每个消息有稳定的 `msg_type` 数值；`PROTOCOL_VERSION` 变更规则写在 `lib.rs` 顶部注释（字段只在末尾追加，破坏性修改升版本，§8.2）。
### 接口与数据
- `encode_c0(&ClientMsgC0)->Vec<u8>`/`decode_c0_server(&[u8])->Result<ServerMsgC0,_>` 等对称函数；`encode_input_bundle(&InputBundle)->Vec<u8>`。供 M0-014 在 `sim-wasm` 里包装。

## 不做什么
- 不做快照/基线编码（M0-006）；不做 C4 可靠事件与 C0 的部署/小队等消息（M1-010）。
- 不做 WS/WT 的帧封装与传输（M0-016/017）。
- 不做 CRC16 trailer（属于快照包，M0-006）。

## 验收标准
1. `proptest` 往返测试 `cargo test -p protocol --release -- --ignored roundtrip_1m`：对 `InputCmd`、`InputBundle`、`Hello/Welcome/Ping/Pong` 随机生成共 10^6 样本全部往返相等（默认 CI 跑 `PROPTEST_CASES=10000` 的快速版）。
2. 位置量化最大误差 ≤ 1/64 m（x、z、y 三轴，随机 10^6 样本实测最大值写入 STATUS）；yaw 线上 10 bit 量化最大误差 ≤ 0.35°，pitch 误差 ≤ 0.35°。
3. 含 3 条 `InputCmd`（典型走动输入）的 `InputBundle` 编码 ≤ 30 B（含 `pred_hash`/`ack`，不含 `FireCmd`），单元测试用固定样例断言，并在 STATUS 记录实测字节数。
4. 解码健壮性：对 10^5 个随机字节串及每个合法消息的所有前缀截断，`decode` 返回 `Err` 或合法值，**0 次 panic**（`cargo test -p protocol fuzz_smoke`）；`ticket.len() == 257` 的 `Hello` 被拒绝。
5. `crates/protocol` 源码中 `grep -R "unsafe" crates/protocol/src` 为空；`cargo build -p protocol --target wasm32-unknown-unknown` 通过；`cargo tree -p protocol` 无 `std`-only 的禁用依赖（`tokio`/`wasm-bindgen`/`sim`）。

## 测试要求
- 单元测试：`BitWriter/BitReader` 边界（跨字节、64 位、0 位）、补码符号扩展、量化夹紧与 NaN 处理；每个消息的最小/最大样例。
- 集成/无头测试：无（传输在 M0-016）。
- 性能基准：可选 criterion `bitio_write_read`，供 M0-030 引用。
- 回归：将若干固定字节样例（golden bytes）存 `crates/protocol/tests/golden/`，防止无意改线上格式。

## 性能预算
上行：`InputBundle` 一条消息 ≤ 30 B，30 Hz 上行含传输开销 ≈ 22 kbps ≤ 32 kbps（ARCHITECTURE §6.9.5）。编码/解码单条消息 < 1 µs 量级（仅记录，不设硬门槛）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `ArrayVec` 之类需要 `no_std` 兼容 crate（如 `arrayvec`，关闭 `std`）；引入前确认许可证（H-037）。
- `Welcome` 比 ARCHITECTURE §6.8.1 草案多了 `data_hash` 与 `client_tick_offset`（§6.2 文字要求下发，但 §6.8.1 表未列）：属"细化"，在 PLAN_ISSUES 记一条。
- 实体 ID 在快照里 8 bit，而 `EntityId` 为 `u16`（§8.1）：M0 玩家 ≤ 20，无冲突；M2 前需裁决（不在本任务处理）。
