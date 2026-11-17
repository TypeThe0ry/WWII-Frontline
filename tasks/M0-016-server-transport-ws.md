# M0-016 server：WebSocket 传输与握手

## 元数据
- 里程碑：M0
- 工作流：server
- 规模：M
- 依赖：M0-015
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.6.2; ARCHITECTURE §19.1; ARCHITECTURE §6.8.1; ARCHITECTURE §6.2

## 背景
WebSocket 是 WebTransport 之外的**一等兜底路径**（ARCHITECTURE §5.5、T6：企业网/部分网络会阻断 UDP/HTTP3）。同一 `Message` 枚举，二进制帧封装 `[channel: u8][len: u16][payload]`，所有通道复用单一 TCP 连接（§6.6.2）。无 datagram：C1/C2/C3 退化为"最新者优先"队列——服务器发送前检查缓冲，`buffered_amount > 8 KB` 则**丢弃旧快照**，仅保留最新一包；WS 模式下快照上限改为 15 Hz（降频逻辑在 M0-019）。安全约束（§19.1）：所有网络输入做长度/范围检查，失败即断开；每连接内存上限（缓冲 64 KB）、未完成握手超时 **5 s**、每 IP 并发连接上限 4；解码路径禁 `unsafe`。握手字段（§6.8.1）：`Hello{protocol_version, data_hash, ticket≤256, client_build, caps}` → `Welcome{player_id, server_tick, tick_hz, match_seed, map_id, mode, team, …}` 或 `Reject{reason}`。
本任务做出**传输无关的会话抽象**，供 M0-017（WebTransport）与 `headless-client` 复用：服务器内部只面对 `ClientTransport` trait，不关心底层是 WS 还是 WT。

## 具体内容
### 要创建/修改的文件
- `crates/protocol/src/framing.rs` —— 纯函数：`encode_ws_frame(channel, payload)->Vec<u8>`、`decode_ws_frame(&[u8])->Result<(Channel,&[u8])>`（长度校验、`len` 与实际一致、`payload ≤ 65535`）；`no_std`，含 proptest。
- `crates/server/src/net/{transport.rs,ws.rs,conn.rs,handshake.rs,limits.rs}`：`trait ClientTransport { fn send(&self, ch: Channel, bytes: Bytes, reliability: Rel) ; async fn recv(&mut self)->Option<(Channel, Bytes)>; fn close(...); fn stats() -> TransportStats }`；`WsListener`（`tokio-tungstenite`，监听 `--ws-port`，M0 明文 `ws://`，`wss` 由前置 TLS 终结，见备注）。
- `crates/server/tests/ws_handshake.rs`、`ws_soak.rs`、`ws_drop_oldest.rs`。
### 要实现的行为
1. **握手**：连接建立后 5 s 内必须收到 C0 `Hello`，否则关闭并计数 `net_handshake_timeout_total`；校验 `protocol_version == PROTOCOL_VERSION` 且 `data_hash == sim.data_hash`（不等 → `Reject(version/data_hash)` 后关闭）；通过则分配 `PlayerId`（0..127 的最小空闲号）并回 `Welcome`（`server_tick` 为当前 tick，`client_tick_offset` 初值 0）。M0 不校验 `ticket`（`--dev-no-ticket` 默认开；M1-029 实装）。
2. **限制**：每 IP ≤ 4 并发；总连接 ≤ 128；入站帧 `len` > 4096（C0/C1 的合理上限，常量写 `limits.rs`）→ 断开并计数；未知 `channel`/非法 `msg_type` → 断开；每连接收发缓冲 ≤ 64 KB。
3. **通道语义**：C0/C4 可靠有序，永不丢（缓冲超 64 KB 视为慢客户端断开）；C2/C3 "最新者优先"：每客户端出站队列容量 4，满则丢最旧，且当底层写缓冲 > 8 KB 时只保留最新一包，丢弃计数 `net_snapshots_dropped_total`。C1 入站直接投递到 sim 输入通道（M0-018 接入；此处暂投递到 `InboundQueue` 并计数）。
4. **Ping/Pong**：收到 C0 `Ping{client_time}` 立即回 `Pong{client_time, server_tick, server_time_ms}`（server_tick 来自 sim 线程发布的原子变量）。
5. **指标**：`net_connections`（gauge）、`net_rejects_total{reason}`、`net_bytes_in_total/out_total`、`net_frames_malformed_total`。
6. 断开清理：连接关闭时发布 `ClientGone` 事件给 sim 命令通道（M0-018 处理实体移除）。
### 接口与数据
- `Channel` 与 `Rel{Reliable,Unreliable}` 来自 `protocol`；`ClientTransport` 的 `send` 不阻塞 sim 线程（经有界 channel 交给 net 任务）。

## 不做什么
- 不做 WebTransport（M0-017）；不做会话进出世界与输入缓冲（M0-018）；不做快照/AOI（M0-019）。
- 不做 TLS（`wss`）、gateway ticket 校验、速率限制的完整表（M1-029、M2、M4-032）。
- 不做带宽调节器（M2-020）。

## 验收标准
1. 集成测试 `ws_soak`：10 个 WS 客户端（`tokio-tungstenite` 客户端）握手成功并保持 10 min，每秒发 Ping，**0 次意外断开**，Pong 的 `client_time` 回显正确；CI 用 60 s 版本，10 min 版本为 `#[ignore]` 手动验收并记录日志。
2. 握手异常用例全部通过：版本不符 → `Reject`；`data_hash` 不符 → `Reject`；5 s 内不发 Hello → 连接被关闭（实测 5.0–5.5 s）；`ticket` 257 字节 → 断开；同一 IP 第 5 个连接 → 拒绝。
3. 丢旧快照单测 `ws_drop_oldest`：向慢消费者连续压入 100 个 C2 包，最终客户端只收到最后一包及至多 3 个更早的包，且包序单调；C0 包 100% 送达且保序。
4. 畸形帧测试：10^4 个随机字节帧与全部"合法帧的前缀截断"，服务器 0 次 panic，且对应连接被关闭（计数 `net_frames_malformed_total` 增加）。
5. 握手到 `Welcome` 的本机往返 P50 < 5 ms（记录）；10 连接并发握手总耗时 < 500 ms。

## 测试要求
- 单元测试：`framing`（往返、长度越界）、`PlayerId` 分配与回收、每 IP 限制计数。
- 集成/无头测试：见验收 1–4；`headless-client`（M0-027）后续复用同一帧格式。
- 性能基准：无；记录单连接 CPU 占用（空闲）。
- 回归：golden 帧字节。

## 性能预算
网络线程不占用 sim 线程；每连接缓冲 ≤ 64 KB（ARCHITECTURE §19.1）；WS 路径快照上限 15 Hz（§6.6.2，由 M0-019 落实）。不得使 M0-015 的空载 tick 抖动指标回归（重跑 10 min 比较，P99 < 1 ms 保持）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `ws://` 明文只适合 localhost 开发（页面也是 http://localhost）；浏览器从 https 页面连接需要 `wss`，TLS 终结与证书见 NEEDS_HUMAN H-017，M0 不实现。
- `framing.rs` 放在 `protocol` 而非 `server`，是为了让 `headless-client` 与（经 M0-014）浏览器共用同一实现；INDEX 把它归在本任务，属于文件位置的细化。
- WS 在 2% 丢包下的 P95 延迟可能 > 300 ms（ARCHITECTURE §6.6.2，物理限制），不是本任务的缺陷。
