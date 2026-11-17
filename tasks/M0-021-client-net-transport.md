# M0-021 client/net：传输与时钟同步

## 元数据
- 里程碑：M0
- 工作流：net
- 规模：M
- 依赖：M0-014, M0-017
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.2; ARCHITECTURE §6.6; ARCHITECTURE §4.3.9; ARCHITECTURE §8.4; ARCHITECTURE §5.5

## 背景
客户端网络层：WebTransport 优先、WebSocket 回退的**传输抽象**，握手，以及时钟同步（`estimated_server_tick`）。`net` 目录只能依赖 `sim-wasm` 与事件总线，**不得依赖 `render/ui/audio`**（ARCHITECTURE §4.3.9）。
时钟（§6.2）：`Welcome` 下发 `server_tick/tick_rate/match_seed/data_hash/client_tick_offset`；客户端每 1 s 发 `Ping{client_time}`，收 `Pong{client_time, server_tick, server_time}` 得 RTT，对 RTT 做 EWMA（α = 0.1）；输入目标 tick = `server_tick_estimate + ceil(RTT/2 / 33.3 ms) + 1`；服务器在快照头回传 `input_slack`，客户端据此做 ±1 tick 微调，限幅 ±10%/s。通道（§6.6.1）：C0 = 第一个 bidi stream，C1 = datagram 上行，C2/C3 = datagram 下行，C4 = 服务器 uni stream；WS 兜底：`[channel u8][len u16][payload]` 单连接复用（帧函数见 M0-016 的 `protocol::framing`，客户端通过 `sim-wasm` 暴露的等价函数或在 TS 中按同一格式实现并由黄金字节测试保证一致）。事件总线（§8.4）自建 `EventBus<EventMap>`：类型安全、同步分发、热路径零分配（对象池）；本任务创建其最小实现与 `net.connected/net.disconnected` 事件，M0-022 继续扩展事件目录。

## 具体内容
### 要创建/修改的文件
- `client/src/game/event-bus.ts`（`EventBus<EventMap>`；`emit/on/off`；事件对象池）及测试。
- `client/src/net/{transport.ts,wt-transport.ts,ws-transport.ts,connect.ts,handshake.ts,clock.ts,codec.ts,index.ts}`；`client/src/net/*.test.ts`。
- `client/src/net/config.ts`：解析 URL 参数 `?server=host:port`、`?transport=wt|ws|auto`、`?certhash=<base64 sha-256>`（开发证书，M0-017）。
### 要实现的行为
1. **Transport 接口**：`interface Transport { kind:'wt'|'ws'; connect():Promise<void>; sendC0(bytes); sendC1(bytes); onC0(cb); onC2(cb); onC3(cb); onC4(cb); close(); readonly maxDatagramSize:number }`。WT 实现：`new WebTransport(url, {serverCertificateHashes})`（仅在 `certhash` 提供时使用）；datagram 读写用 `datagrams.readable/writable`；C0 bidi stream 的 `[len u16][payload]` 帧；C4 uni stream 读取。WS 实现：`WebSocket` `binaryType='arraybuffer'`，所有通道经 `[channel][len][payload]` 复用；WS 模式下对 C1 上行用"最新者优先"（`bufferedAmount > 8 KB` 丢旧）。
2. **选择与回退**：`auto` = 若 `typeof WebTransport === 'undefined'` 或 `transport.ready` 在 **3 s** 内未完成/被拒 → 回退 WS；**从开始连接到 WS 握手完成 ≤ 5 s**（E0-7）。`?transport=ws` 强制 WS（演示脚本第 4 步）。`net.connected{transport, rtt}` 事件在握手成功后触发；断线 `net.disconnected{reason}`；M0 不做自动重连（M1-040）。
3. **握手**：发 `Hello{protocolVersion, dataHash(来自 sim-wasm), ticket:空, clientBuild, caps}`（M0-014 提供的 `encodeHello`）；收 `Welcome` 或 `Reject`（Reject 以明确错误抛出并显示 reason）；`Welcome.data_hash` 与本地 `dataHash()` 不等即中止。
4. **时钟同步**（`ClockSync`）：连接后 500 ms 内连发 5 次 Ping（间隔 100 ms）快速收敛，之后 1 Hz；`rtt_ewma`（α=0.1）；`estimated_server_tick(now)` = 最近 Pong 的 `server_tick` + `(now − t_pong_recv)/33.333 ms` + `rtt/2/33.333 ms` 的连续估计，对外取整；`input_lead_ticks = ceil(rtt/2/33.333)+1`；`applySlack(input_slack)`：向目标微调，每秒最多 ±10%。暴露 `clock.now()`、`clock.rtt()`、`clock.jitter()`（RTT 标准差）。
5. **编解码**：全部消息经 `sim-wasm`（M0-014）的类型化函数；`net` 不手写 bit 操作。
6. **统计**：`NetStats{bytesIn,bytesOut,pktsIn,pktsOut,rttMs,jitterMs,transport}` 供叠层（M0-020 的占位槽位）。
### 接口与数据
- `connect(opts): Promise<Session{transport, clock, welcome}>`；`Session.onSnapshot(cb:(bytes:Uint8Array)=>void)`（C2 原始字节，解码在 `game`）；`Session.sendInputBundle(bytes)`。

## 不做什么
- 不做预测、对账、插值（M0-022）；不做 gateway 鉴权/join ticket（M1-029/M1-055）。
- 不做自动重连、带宽调节器、WS 降级提示 UI（M1-040、M2-020）。
- 不做浏览器互通矩阵（M0-028）。

## 验收标准
1. vitest（假定时器 + 假传输）`clock_sync`：模拟 RTT = 80 ms（单向 40 ms）、抖动 ±10 ms 均匀分布，连续 60 s，**连接后 5 s 起** `|estimated_server_tick − 真实 server_tick| ≤ 1 tick` 的采样占比 100%（采样间隔 100 ms）。
2. vitest `fallback`：`WebTransport` 未定义 → 不尝试 WT，直接 WS；`WebTransport.ready` 永不 resolve → 3 s 后回退，总耗时 ≤ 5 s；`ready` reject → 立即回退；`?transport=ws` 强制 WS。四种用例全绿。
3. 真实联调（需运行 `frontline-server`，本机 loopback，人工/脚本）：Chrome 经 WT 连入显示 `transport=wt`；加 `?transport=ws` 或禁用 WT 后显示 `transport=ws`；Welcome 字段正确（`tick_hz=30`，`match_seed` 非零）。结果写入 STATUS。
4. `data_hash` 不一致或 `protocol_version` 不一致时报出 `Reject` 原因而非静默失败（2 个用例）。
5. RTT EWMA：输入序列 `[100, 100, 200, 100]` ms 的期望值与手算（α=0.1）逐项一致（精确到 0.01 ms）。
6. `EventBus` vitest：emit 同步调用、订阅者异常不破坏其他订阅者、取消订阅后不再触发；热路径 emit 100 万次堆分配增量 ≈ 0（`performance.memory`/`--expose-gc` 粗测，阈值 < 1 MB）。

## 测试要求
- 单元测试：`ClockSync`、`connect` 回退状态机、帧编解码（含黄金字节与 `protocol::framing` 一致）、`applySlack` 限幅。
- 集成/无头测试：验收 3（与服务器联调）；浏览器矩阵留给 M0-028。
- 性能基准：每帧 `net` 处理 ≤ 0.2 ms（记录；预算见 §13.5.1 快照解码 0.6 ms 的一部分）。
- 回归：无。

## 性能预算
输入 + `game` 更新合计 ≤ 1.5 ms、快照解码 + 插值 ≤ 0.6 ms（ARCHITECTURE §13.5.1）；本任务的接收回调只做入队，不在回调内解码。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 的"80 ms 人工延迟"语义不明：本任务取 **RTT = 80 ms**（单向 40 ms）；若要在真机上用 `tc netem` 复测，注意在 loopback 上 netem 对双向都生效。
- 事件总线的实现放在本任务（最早使用者），INDEX 在 M1-040 才说"事件总线类型"；M1-040 只需追加事件类型，不应重写实现。
- 浏览器对 `serverCertificateHashes` 的支持未核实（ARCHITECTURE §5.5），Safari/Firefox 若不支持，WT 路径需要受信任证书（H-017）。
