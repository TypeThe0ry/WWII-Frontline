# M1-028 gateway：服务器列表、游客鉴权与 ticket

## 元数据
- 里程碑：M1
- 工作流：server
- 规模：L
- 前置依赖：M1-010
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §4.3.5; ARCHITECTURE §19.2; ARCHITECTURE §19.3; ARCHITECTURE §3.3; ARCHITECTURE §18.3; GAMEDESIGN §2.3

## 背景
`crates/gateway`（axum）负责：服务器列表、游客/（OAuth 在 M2+）鉴权、**join ticket** 签发、对局服务器的内部注册/心跳 API、战绩入库。**禁止**依赖 `sim`/`server`/`bots`，不持有任何实时对局状态（ARCHITECTURE §4.3.5）；仅依赖 `protocol` 的共享 DTO（`ServerInfo`、`PROTOCOL_VERSION`）。
鉴权规则（ARCHITECTURE §19.2）：
- 游客：首访生成随机 `guest_id`（128 bit）；gateway 签发 `session token`（JWT，**EdDSA** 或 HS256，TTL 24 h，存 `localStorage`；风险：XSS 可窃取，故严格 CSP、无第三方脚本）。
- **join ticket**：签发给特定 `(player_id, match_id)`，**EdDSA 签名，TTL 60 s，单次使用**（对局服务器记录已用 `jti`）；内容：`player_id`、`match_id`、`nick`、`exp`、`jti`、`is_admin`；对局服务器只持公钥。
- 速率限制（§19.3）：gateway 登录/ticket **每 IP 10/min，每账号 6/min**。
- 对局生命周期（§3.3）：对局服务器启动后 `POST /internal/matches/register`（含公钥指纹、端口、`data_hash`）→ 心跳；**心跳 5 s 丢失 → gateway 标记失效**；局终上报战绩。首发不做对局迁移。
- 服务器列表字段（GD §2.3）：名称、地图、模式、当前阶段、剩余时间、人数（真人/bot/上限）、延迟、地区、规则标签、版本；刷新节流 ≥ 2 s（客户端侧）。
- 存储：SQLite（sqlx）；只存昵称、设置、基础战绩（A8）。

## 具体内容
### 要创建/修改的文件
- `crates/gateway/Cargo.toml`、`src/main.rs`（axum + tokio）、`src/config.rs`（环境变量：`GATEWAY_BIND`、`TICKET_SIGNING_KEY_FILE`、`INTERNAL_API_TOKEN`、`DATABASE_URL`）
- `crates/gateway/src/routes/guest.rs` —— `POST /v1/guest`（可选带旧 token 续期）→ `{guest_id, token, nick_suggestion}`
- `crates/gateway/src/routes/servers.rs` —— `GET /v1/servers` → `Vec<ServerInfo>`（过滤失效服务器）
- `crates/gateway/src/routes/ticket.rs` —— `POST /v1/servers/{match_id}/ticket`（Bearer session token + nick）→ `{ticket, addr, wt_port, ws_port, expires_in_s}`
- `crates/gateway/src/routes/internal.rs` —— `POST /internal/matches/register`、`POST /internal/matches/{id}/heartbeat`、`POST /internal/matches/{id}/result`（Bearer `INTERNAL_API_TOKEN`；生产还应限内网/IP 白名单）
- `crates/gateway/src/auth/{token.rs,ticket.rs}` —— JWT 与 ticket 签发/校验（`ed25519-dalek` 或 `jsonwebtoken`，二选一，登记 LICENSES §5）
- `crates/gateway/src/ratelimit.rs` —— 每 IP / 每账号滑动窗口（内存实现；单实例）
- `crates/gateway/src/db/{mod.rs,migrations/0001_init.sql}` —— `guests`、`matches`、`results`
- `crates/protocol/src/dto.rs` —— `ServerInfo`、`TicketClaims`（**仅结构与规范化序列化，不含密码学**）、`MatchRegister`
- `crates/gateway/tests/*.rs`；`crates/gateway/README`（不新建 .md 之外的文档；接口表写在 `routes/mod.rs` 注释）
### 要实现的行为
1. 游客：昵称 3–20 字符（NFKC、禁控制字符）；`guest_id` 随机；token 带 `sub`、`iat`、`exp`（24 h）；无 PII。
2. 服务器列表：返回仅 `state=live` 且 `last_heartbeat ≤ 5 s` 的服务器；`ServerInfo` 字段如上；`protocol_version` 不匹配的标 `compatible=false`。
3. ticket：签发时校验 token、服务器存在且未满（`players < cap`）、限速；`jti` 为 128-bit 随机；`exp = now+60 s`；返回连接信息。
4. 内部 API：`register` 返回 `match_id` 与服务端应使用的 ticket 公钥指纹确认；`heartbeat` 更新人数/阶段/剩余时间；超过 5 s 无心跳 → 后台任务置 `stale`；`result` 写入战绩表。
5. 限速：每 IP 10/min（登录/ticket 合计）、每账号 6/min（ticket）；超限 429 + `Retry-After`。
6. 日志与指标（§18.3）：请求延迟/错误率直方图、登录失败率；错误响应不泄露内部细节。
7. 安全：CORS 仅允许配置的站点来源；响应头 `Cache-Control: no-store`（鉴权类）；不记录 token/ticket 原文。
### 接口与数据
```rust
pub struct TicketClaims { pub player_id:u8, pub match_id:Uuid, pub nick:String, pub exp:u64, pub jti:[u8;16], pub is_admin:bool }
pub struct ServerInfo { pub match_id:Uuid, pub name:String, pub map:String, pub mode:String, pub phase:String, pub time_left_s:u32,
  pub humans:u8, pub bots:u8, pub cap:u8, pub region:String, pub tags:Vec<String>, pub version:String, pub compatible:bool }
```
`TicketClaims` 的规范化二进制编码在 `protocol::dto` 中定义，供 gateway 签名与 server（M1-029）验签共用，避免 server 依赖 gateway（ARCHITECTURE §4.5）。

## 不做什么
- OAuth（Discord/Google）、管理员 API、封禁、LiveKit token：M2+；
- 对局服务器侧的注册/心跳客户端与 ticket 验签：M1-029；
- 战绩展示/账号设置同步：M2；
- 匹配/排队与多服务器调度（gateway 编排模块，不新增 crate）：M4-031；
- 客户端服务器浏览器 UI：M1-055。

## 验收标准
1. ticket 过期/重放单测：`exp` 过期 1 s 验签失败；同一 `jti` 在验签库（测试用 `JtiCache`）二次使用失败；篡改 1 字节失败；TTL = 60 s ±1 s（`cargo test ticket_*`）。
2. 限速：同一 IP 第 11 次 `/v1/guest`+ticket 请求 1 min 内返回 429；同一账号第 7 次 ticket 返回 429；窗口过后恢复（单测，注入时钟）。
3. 服务器列表：注册 3 台服务器，1 台停心跳 6 s，`GET /v1/servers` 返回 2 台；`compatible` 对版本不一致标 false（集成测试）。
4. 游客 token：TTL 24 h ±1 s；过期 token 调用 ticket 返回 401（单测）。
5. 内部 API 无 token 返回 401；错误 token 401；随机 `match_id` 404（单测）。
6. 数据库迁移可从零建库；`cargo sqlx prepare` 或编译期检查通过；并发 200 个 ticket 请求无死锁，P95 ≤ 50 ms（本机 `oha`/脚本，记录）。
7. 依赖方向：`cargo metadata` 检查 gateway 不依赖 `sim/server/bots`（CI 依赖图脚本，M0-002）；`cargo deny` 通过。
8. 日志审计：脚本 grep 确认日志中无 `token`/`ticket` 原文（测试夹具生成后断言）。

## 测试要求
- 单元测试：签名/验签、TTL、限速窗口、昵称校验、`ServerInfo` 序列化。
- 集成：`axum` 内存服务（`tower::ServiceExt`）覆盖所有路由；SQLite 内存库。
- 性能基准：ticket 签发 ≤ 0.2 ms/次（criterion）。
- 回归：HTTP 路由表快照（`routes/mod.rs` 注释生成的列表）。

## 性能预算
- 无硬预算；gateway 请求延迟/错误率按 ARCHITECTURE §18.3 暴露（5xx > 1% 告警线）。ticket 签发 ≤ 0.2 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 行已注明本任务含 protocol DTO（`ServerInfo`、`TicketClaims`、`MatchRegister`），落在 `protocol/src/dto.rs`；M1-010 不含这些 DTO。
- JWT 算法在 ARCHITECTURE §19.2 为"HS256/EdDSA"二选一；游客 token 用 HS256 即可（gateway 自验），ticket 必须 EdDSA（server 只持公钥）。
- 内部 API 的传输安全（内网/mTLS）属部署（M1-066/M1-067）；开发环境用共享 token。
- 域名/TLS/账号等运维条目依赖人类（H-015、H-016、H-017）；本任务不被其阻塞（本地可跑）。
