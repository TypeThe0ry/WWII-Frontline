# M0-006 protocol：快照编码与基线增量

## 元数据
- 里程碑：M0
- 工作流：net
- 规模：L
- 依赖：M0-005
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.5; ARCHITECTURE §8.2; ARCHITECTURE §6.9.1; ARCHITECTURE §19.1; ARCHITECTURE §17.1

## 背景
下行带宽预算（均值 ≤ 256 kbps，典型推导 ≈ 54 kbps，ARCHITECTURE §6.9）完全依赖"位级量化 + 基线增量"。本任务实现快照的**编码/解码核心**，不含选择哪些实体（那是 M0-019 的 AOI 优先级）。服务器每个客户端、每个实体保留"该客户端已确认的基线"，客户端通过 `InputBundle.ack_snapshot_tick` 确认；基线环 32 tick（≈ 1 s）；ack 落后 > 32 tick 或新实体 → 全量关键帧（§6.5.2）。
包结构（§6.5.1，一包 ≤ 1200 B）：`header: type(4b) | server_tick(32b 全量 或 delta16(16b)) | ack_input_seq(16b) | input_slack(4b) | baseline_tick_delta(5b) | flags(4b)`；`self_state`（§6.5.3，约 14 B：`pos_corr`/ok 4 bit、`ammo_in_mag`、`ammo_reserve`、`health 8b`、`stamina 6b`、`suppression 4b`、`stance 3b`、`deploy_state 3b`、`cooldown_bits 8b`）；`events: n(6b)+[event_id(16b)+type(5b)+payload]`；`entities: n(7b)+[entity_id(8b)|has_change_mask(1b)|field_mask(≤8b)|fields…]`；`trailer: CRC16`（仅 WS 兜底可省）。字段掩码覆盖 `pos/rot/vel/anim_state/weapon/health_q/flags`；位置差分用"2 bit 大小类（<8/<64/<512/其余 步长 1/32 m）+ 6/9/12/16 bit 带符号数值"；旋转 yaw 8 bit（远）/10 bit（近）、pitch 6 bit；速度不下发（M0 无载具）。字节数目标：近景 delta 记录均值约 10 B、远景 9 B、关键帧 13 B（§6.9.1）。

## 具体内容
### 要创建/修改的文件
- `crates/protocol/src/snapshot/{mod.rs,header.rs,entity.rs,self_state.rs,events.rs,baseline.rs,encoder.rs,decoder.rs,crc.rs}`。
- `crates/protocol/fuzz/`（`cargo-fuzz` 工程，靶 `snapshot_decode`、`input_bundle_decode`）；`crates/protocol/tests/{snapshot_prop.rs,snapshot_size.rs}`。
### 要实现的行为
1. `EntityState{id,kind(Player),pos:QPos,yaw:u8,pitch:u8,vel:Option<QVel>,anim:u8,stance:u8,weapon:u8,flags:u8,health_q:u8,vehicle:Option<VehicleExtra>}`（ARCHITECTURE §8.2；`vehicle` 在 M0 恒为 `None`，字段保留）。
2. `SnapshotEncoder::begin(client, tick, budget) -> SnapshotBuilder`；`SnapshotBuilder::{add_self, add_entity(&EntityState, near: bool) -> Result<bool>, add_events, finish() -> Vec<u8>}`：`add_entity` 在放不下时返回 `Ok(false)`（预算 `budget` 默认 1150 B 载荷），**不得**写出半条记录。
3. 基线管理：`SnapshotEncoder::on_ack(client, ack_tick)` 把该包内实体提升为新基线；`baseline[client][entity]` 环长 32；ack 落后 > 32 tick → 之后对该客户端所有实体发全量关键帧；`remove_client`；新进入实体首包必须全量。
4. `SnapshotDecoder::decode(&[u8]) -> Result<DecodedSnapshot>`：按包内 `baseline_tick_delta` 取自身基线；基线缺失时返回 `CodecError::MissingBaseline`（调用方请求关键帧，即下个 ack 缺失自动触发）；实体 `entity_remove` 标记；解码结果为 SoA（`ids: Vec<u8>`、`pos: Vec<f32>` 等，便于 M0-014 零拷贝暴露）。
5. 事件段仅提供"写入给定事件列表"的能力（`Shot/Impact/HitConfirm/DamageDir` 的 payload 位布局按 §6.8.4）；冗余重发与 ack 的策略在服务器（M0-019）。
6. CRC16（CCITT）trailer 由 `flags` 的一位控制；WS 路径可关闭。
7. 解码上限：`entities ≤ 127`（7 bit）、`events ≤ 63`；包长 > 1200 B 或位流耗尽即拒绝；`unsafe` 禁用。
### 接口与数据
- 公开 API 如 ARCHITECTURE §8.2 的 `SnapshotEncoder/SnapshotBuilder/SnapshotDecoder` 草案；`DecodedSnapshot{tick,ack_input_seq,input_slack,self_state,entities(SoA),events,removed_ids}`。
- 字段位宽的具体取值写进模块文档注释，并在 PLAN_ISSUES 记"细化"（README §5.1 例外）。

## 不做什么
- 不做 AOI 候选筛选、优先级累加器、出包调度（M0-019/M1-024）。
- 不做团队通道与 `WorldState`、载具记录（M1-010）；不做 C4 事件。
- 不做 WASM 绑定（M0-014）。

## 验收标准
1. 属性测试 `decode(encode(b, Some(a)), Some(a)) == b`：随机实体集（1..60 个实体、随机字段变化、关键帧/增量混合）10^5 次全部成立（`cargo test -p protocol --release snapshot_prop`）；全量关键帧路径 `decode(encode(b, None), None) == b` 同样成立。
2. `cargo +nightly fuzz run snapshot_decode -- -max_total_time=600` 运行 10 min：0 crash、0 panic、0 OOM（输出日志路径写入 STATUS）；`input_bundle_decode` 靶同样 ≥ 5 min。
3. 字节数：用"实体沿 4 m/s 直线行走、20 Hz、基线 ack 间隔 ≤ 2 tick"的合成轨迹，近景（≤ 96 m）玩家 delta 记录平均 ≤ 10 B；关键帧记录平均 ≤ 13 B；`self_state` ≤ 14 B（`snapshot_size` 测试断言并输出实测均值）。
4. 40 个实体的典型快照（近景 4 + 中景 10 + 远景 26，delta）总长 ≤ 600 B；预算 1150 B 时 `add_entity` 在放不下时返回 `Ok(false)` 且已写出内容仍可解码。
5. 基线环：ack 落后 33 tick 后，下一包全部实体为关键帧（测试断言记录类型）；丢弃 30% 包（随机）下仍能解码并最终收敛，0 次 `MissingBaseline` 之后持续 > 1 s。

## 测试要求
- 单元测试：位大小类边界（7/8、63/64、511/512 步）、y 13 bit 溢出夹紧、yaw 回绕、`field_mask` 全 0 / 全 1。
- 集成/无头测试：本任务内用"内存中的客户端/服务器对"回放 60 s 合成轨迹，记录字节分布供 M0-019/M0-027 对照。
- 性能基准：`cargo bench -p protocol snapshot_encode_client`（60 实体）、`snapshot_decode`；ARCHITECTURE §6.10 估计编码约 25 µs/客户端——实测写入 STATUS（M0-030 汇总）。
- 回归：golden bytes 固定样例。

## 性能预算
编码 ≈ 25 µs/客户端、67 个出包客户端并行 3 线程墙钟 ≈ 0.6 ms/tick（ARCHITECTURE §6.10 阶段 9）；单快照 ≤ 1200 B（§6.5.1）；近景 delta ≈ 10 B（§6.9.1）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `cargo-fuzz` 需要 nightly 工具链与 libFuzzer；CI 不跑 10 min 模糊（只跑 60 s 冒烟），完整 10 min 作为本任务的本地验收并存日志。
- 位置差分的"大小类"位宽取值是细化项；若实测均值超 10 B，优先调整大小类阈值而非加字段。
- 实体 ID 8 bit 与 `EntityId: u16` 的差异见 M0-005 备注。
