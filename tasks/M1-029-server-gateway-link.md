# M1-029 server：注册、心跳与 ticket 验证

## 元数据
- 里程碑：M1
- 工作流：server
- 规模：S
- 依赖：M1-028, M0-018
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §3.3; ARCHITECTURE §19.2; ARCHITECTURE §4.3.4; ARCHITECTURE §19.1

## 背景
对局服务器（`crates/server`）与 gateway（M1-028）的对接侧。一局的生命周期（ARCHITECTURE §3.3）：server 启动 → 加载 `/data` 与地图碰撞/导航 → 向 gateway 注册（`POST /internal/matches/register`，含公钥指纹、端口、`data_hash`）→ 玩家带 join ticket 连入 → 局终上报战绩并退出。心跳 **5 s 丢失 → gateway 标记失效**；首发不做对局迁移/断线续局。
join ticket（ARCHITECTURE §19.2）：EdDSA 签名、TTL 60 s、**单次使用**（server 记录已用 `jti`）；内容含 `player_id`、`match_id`、`nick`、`exp`、`jti`、`is_admin`；server 只持**公钥**（`TICKET_PUBLIC_KEY_FILE`）。`Hello.ticket:bytes(≤256)`（ARCHITECTURE §6.8.1）；握手拒绝原因 `Reject.reason`：ticket 无效/过期/重放、版本、`data_hash` 不匹配、满员、封禁。
依赖方向：`server` 允许依赖 `reqwest`（gateway/LiveKit 调用）；**不得依赖 `gateway` crate**，共享类型来自 `protocol::dto`（`TicketClaims`、`MatchRegister`，M1-028 定义）。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/gateway_link/client.rs` —— `GatewayClient`：`register()`、`heartbeat()`、`submit_result()`（reqwest，超时 2 s，指数退避重试）
- `crates/server/src/gateway_link/heartbeat.rs` —— 后台 tokio 任务，每 **1 s** 上报（人数/阶段/剩余时间），确保 5 s 窗口内 ≥ 4 次机会
- `crates/server/src/gateway_link/ticket_verify.rs` —— 验签（ed25519-dalek）、`exp` 检查（容忍时钟偏差 ±2 s）、`match_id` 匹配、`jti` 去重缓存（`TTL = 120 s`，容量上限 4096，过期清理）
- `crates/server/src/gateway_link/result.rs` —— 局终战绩 DTO（击杀/死亡/得分/据点计数；M1 基础）
- `crates/server/src/config.rs`（扩展）—— `GATEWAY_URL`、`INTERNAL_API_TOKEN`、`TICKET_PUBLIC_KEY_FILE`、`--dev-no-gateway`（开发模式：接受固定开发密钥签的 ticket 或在本机回环免 ticket，**默认关闭，发布构建禁用**）
- `crates/server/tests/gateway_link_*.rs`（含假 gateway：`wiremock`/自写 axum）
### 要实现的行为
1. 启动序列：加载数据 → 绑定端口 → `register()`（失败则重试 3 次，仍失败退出码非 0，日志明确原因）→ 开始接受连接。
2. 心跳：1 s 间隔；连续 3 次失败 warn、持续失败不终止对局（gateway 会将其标 stale）；对局结束发送 `result` 后再退出。
3. Ticket 验证（在握手 `Hello` 处理中）：解析 → 验签 → 检查 `exp`、`match_id` → `jti` 去重 → 通过则以 `player_id/nick/is_admin` 建会话；失败回 `Reject{reason}` 并计数 `ticket_rejected_total{reason}`。
4. 重放防护：同一 `jti` 第二次连接被拒（即使在 60 s 内）；断线重连（GD §2.8，90 s 内凭会话 token）走独立的会话重连令牌，不复用 ticket（细化，写 PLAN_ISSUES）。
5. 公钥轮换：支持多公钥（当前 + 旧），按 `kid` 选择；M1 先接口与单测。
6. 局终：上报 `result` 失败时落盘到 `spool/`（JSON），下次启动重试；M1 简化为日志告警。
7. 安全：不记录 ticket 原文；错误信息不泄露公钥路径；`Hello` 缓冲 ≤ 256 B 上限检查（§19.1）。
### 接口与数据
```rust
pub struct TicketVerifier { /* keys, jti_cache */ }
impl TicketVerifier { pub fn verify(&mut self, raw:&[u8], now:u64, match_id:Uuid) -> Result<TicketClaims, RejectReason>; }
pub struct GatewayClient { /* base_url, token */ }
impl GatewayClient { pub async fn register(&self, r:&MatchRegister) -> Result<MatchId>; pub async fn heartbeat(&self, h:&Heartbeat) -> Result<()>; }
```

## 不做什么
- gateway 的路由/签发/限速：M1-028；
- 战绩展示与入库查询：M2；
- 对局迁移、`matchd` 守护、进程拉起：M2/M1-066（ops）；
- LiveKit token：M2；
- OAuth 与管理员 ticket 流程（`is_admin` 仅解析与记录）。

## 验收标准
1. **重放 ticket 被拒**：同一 ticket 连接两次，第 2 次得到 `Reject{ticket_replayed}`；过期 1 s 得到 `Reject{ticket_expired}`；篡改签名得到 `ticket_invalid`（`cargo test ticket_verify_*`，3 例 + 边界 59 s/61 s）。
2. **5 s 心跳丢失 gateway 标记失效**：集成测试（假时钟或真实 6 s 等待）：停止 server 心跳 5.1 s 后 `GET /v1/servers` 不再列出该服务器（与 M1-028 联测，记录通过）。
3. 心跳节奏：连续运行 60 s 假 gateway 收到 60±2 次心跳，间隔抖动 P99 < 200 ms（测试日志）。
4. 注册失败：假 gateway 返回 500，server 重试 3 次后以非 0 码退出，日志含失败原因；注册成功后 `/metrics` 出现 `gateway_registered=1`。
5. `jti` 缓存：4096 容量满时淘汰最旧项且不 panic；过期项 120 s 后清除（单测，注入时钟）；内存占用 < 1 MB。
6. 局终上报：对局结束事件触发 `result` POST 且 payload 含局内玩家数与据点数（假 gateway 断言）；失败时写入 `spool/`。
7. 依赖检查：`cargo metadata` 确认 `server` 不依赖 `gateway`（CI 依赖图脚本通过）。
8. 开发开关 `--dev-no-gateway` 在 release profile 下编译为错误或被忽略（`cfg` 单测断言）。

## 测试要求
- 单元测试：验签、时钟容差、`jti` 缓存、公钥选择、配置解析。
- 集成/无头：假 gateway + headless-client 握手全流程；与真实 gateway（M1-028）联测一次。
- 性能基准：验签 ≤ 0.1 ms/次（criterion）。
- 回归：`Reject.reason` 码表与 protocol 一致。

## 性能预算
- 无新增 tick 预算（握手路径在网络线程）；不得使 §6.10 阶段 1 回归。心跳/请求在独立 tokio 任务。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 心跳间隔 1 s 是细化（文档只规定"5 s 丢失标记失效"）；若 gateway 负载有顾虑可改 2 s，仍满足 5 s 窗口。
- 重连凭证（会话 token）与 ticket 的关系文档未明确（GD §2.8 vs ARCHITECTURE §19.2），本任务设计为独立重连令牌并登记 PLAN_ISSUES。
- 与 M0 `Hello` 处理的衔接：M0 可能接受任意 ticket（占位），本任务替换为真验证并保留 `--dev-no-gateway`。
