# M0-017 server：WebTransport 传输

## 元数据
- 里程碑：M0
- 工作流：server
- 规模：L
- 依赖：M0-016
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §5.5; ARCHITECTURE §6.6.1; ARCHITECTURE §18.2; ARCHITECTURE §5.4; ARCHITECTURE §20.1

## 背景
WebTransport（HTTP/3，`wtransport` crate）是主传输：不可靠 datagram 走输入/快照，可靠 stream 走事件/RPC（ARCHITECTURE §5.5）。这是 ARCHITECTURE §20.1-1 的首要风险："`wtransport` datagram 在 100 连接 × 20 pkt/s 下的稳定性与 Chrome/Firefox/Safari 互通"。`wtransport` 是较小的社区 crate（§5.4 风险①）；失败预案：`quinn` + 自写 WebTransport 握手，或临时仅 WS（M0-016 已就绪，不阻塞其它任务）。
通道映射（§6.6.1）：C0 control = 客户端开启的**第一个双向 stream**（持久，长度前缀帧）；C1 input = C→S datagram；C2 snapshot、C3 team = S→C datagram；C4 reliable-events = 服务器开启的**单向 uni stream #1**（持久）；C5 bulk = 按需 uni stream。datagram 载荷格式：`[channel u8][payload]`（无 `len`，由 datagram 边界给出）；stream 上的帧格式：`[len u16][payload]`。T7：消息 MTU 目标 ≤ 1200 B 载荷，但**实际可用上限须运行时查询 `max_datagram_size`**，超限消息拆包或丢弃并计数（不依赖 IP 分片）。TLS：WebTransport 要求受信任证书；开发期自签名证书走 `serverCertificateHashes`（ECDSA P-256，有效期 ≤ 14 天，浏览器支持情况未核实，§5.5/§18.2），生产用 Let's Encrypt（域名待 H-017）。

## 具体内容
### 要创建/修改的文件
- `crates/server/Cargo.toml`：加 `wtransport`（精确版本；先读其当前文档确认 API）。
- `crates/server/src/net/wt.rs`（`WtListener`：绑定 `--wt-port`，`--cert/--key` 或 `--dev-cert`；接受会话 → 等待客户端第一个 bidi stream → 读取 Hello（握手逻辑复用 M0-016 的 `handshake.rs`）→ 打开 C4 uni stream）、`crates/server/src/net/wt_conn.rs`（`impl ClientTransport for WtConn`）、`datagram.rs`（`[channel u8]` 编解码，`max_datagram_size` 适配）。
- `tools/dev/gen-dev-cert.sh`（或 Rust bin `crates/server/src/bin/gen_dev_cert.rs`）：生成 ECDSA P-256 自签名证书，有效期 ≤ 13 天，输出 `dev-cert.pem/key.pem` 与 `serverCertificateHashes` 用的 SHA-256 指纹（hex 与 base64）。
- `tools/dev/wt-smoke/index.html`（20 行以内：读取 URL 里的指纹，`new WebTransport(url,{serverCertificateHashes})`，发 Hello、收 Welcome、发 10 个 datagram 并显示回包计数）。
- `crates/server/tests/wt_loopback.rs`、`wt_stress.rs`（使用 `wtransport` 客户端）。
### 要实现的行为
1. 与 `ClientTransport` trait 对接：对上层，WT 与 WS 连接**不可区分**；`send(C2, …)` 走 datagram，超 `max_datagram_size` 的包丢弃并 `net_datagram_oversize_total += 1`；`send(C0/C4,…)` 走对应 stream；`recv` 把 C0 帧、C1 datagram 汇入同一 `Inbound` 流。
2. 握手与限制与 M0-016 一致（5 s 超时、每 IP ≤ 4、版本/`data_hash` 校验）；会话结束清理 stream。
3. datagram 乱序/丢失属正常：服务器不对 C1 做重传，依赖输入冗余（M0-018 处理）。
4. 暴露 `transport_kind` 标签（`wt`/`ws`）到指标与日志，供 M0-027 分组统计。
5. 开发证书流程写入 `DEVELOPING.md`（追加一节，不改其它内容）：如何生成、如何把指纹传给客户端（`?certhash=` 查询参数，M0-021 使用）。
### 接口与数据
- `WtConn::max_datagram_size()` 暴露给 M0-019 以限制 `PACKET_BUDGET = min(1150, max_datagram_size − 1)`。

## 不做什么
- 不做客户端 TS 侧传输（M0-021）；不做浏览器互通矩阵（M0-028）；不做 Let's Encrypt/SIGHUP 证书热更新（M4-033、M4-034）。
- 不做 QUIC 层拥塞窗口驱动的带宽调节（M2-020）；不做 `quinn` 自写握手（仅在失败预案触发时）。
- 不改 M0-016 的握手逻辑与 `ClientTransport` 语义（需要改则同步更新 M0-016 的测试）。

## 验收标准
1. `cargo test -p server --test wt_loopback`：`wtransport` 客户端连入，完成 Hello/Welcome，收发 100 个 datagram 往返（echo 由测试专用 C1→C2 回显实现）**100/100 到达**（loopback）；C0 与 C4 stream 各传 1000 条消息保序无丢。
2. `wt_stress`（`#[ignore]`，nightly/手动）：100 个 `wtransport` 客户端 × 每客户端 20 datagram/s 上行 + 20 datagram/s 下行，持续 120 s：loopback 下行/上行到达率均 ≥ 99.9%；0 次会话意外关闭；服务器 CPU 与 `server_tick_ms` P99 记录（对应 ARCHITECTURE §20.1-1；结果写入 STATUS，不达标写 PLAN_ISSUES）。
3. **Chrome 本机连入**（需人工，标"需人工/参考机"）：`tools/dev/wt-smoke/index.html` 在桌面 Chrome 中使用开发证书指纹连入，Welcome 显示，10 个 datagram 收到 10 个回包；截图/控制台输出路径写入 STATUS。
4. 同一 `Message` 层与 WS 通用：同一份握手测试（M0-016 的 `ws_handshake` 用例参数化为 `[ws, wt]`）两种传输均通过。
5. 超大 datagram：构造 > `max_datagram_size` 的 C2 包，被丢弃且计数 +1，不影响后续包。
6. `gen-dev-cert` 生成的证书有效期 ≤ 14 天（`openssl x509 -noout -dates` 校验）且指纹与 `serverCertificateHashes` 形式匹配。

## 测试要求
- 单元测试：datagram 编解码、`max_datagram_size` 适配、stream 帧拆包（半包/粘包）。
- 集成/无头测试：验收 1、2、4、5；M0-027 的 headless-client 将再用 WT 跑 30 min。
- 性能基准：stress 期间的服务器 CPU（记录）。
- 回归：M0-016 的 WS 测试全部保持通过。

## 性能预算
不得使 M0-015 的 tick 抖动回归；100 连接 × 20 pkt/s = 2 000 pkt/s 的 QUIC/AEAD 开销计入 ARCHITECTURE §6.10 阶段 10（≈ 1.5 ms），stress 期间 `server_tick_ms` P99 ≤ 28 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 这是 E0 风险 #1 的承载：若 `wtransport` 无法稳定通过验收 2，**不要硬撑**：写 PLAN_ISSUES，M0 以 WS 完成其余任务，WT 状态 `partial`，由 Fable 裁决是否换 `quinn` 自写握手。
- 域名/受信任证书见 NEEDS_HUMAN H-017；`serverCertificateHashes` 在 Safari/Firefox 的支持**未核实**（ARCHITECTURE §5.5），M0-028 登记。
- 验收 3 需要人在桌面 Chrome 上操作；无人值守环境下该项记 `未测`，任务至多 `partial`。
