# M2-022 gateway：战绩、设置与可选 OAuth

## 元数据
- 里程碑：M2
- 工作流：server
- 规模：M
- 前置依赖：M1-028
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §19.2; ARCHITECTURE §2; ARCHITECTURE §19.3; ARCHITECTURE §19.4; ARCHITECTURE §4.3.5; GAMEDESIGN §18.2

## 背景
A8（ARCH §2）：首发匿名游客 + **可选 OAuth（Discord/Google）**，无付费、无库存，只存昵称、设置、基础战绩。M1-028 已有 axum gateway：服务器列表、游客 token（JWT，TTL 24 h，EdDSA/HS256，存 `localStorage`）、join ticket（EdDSA，TTL 60 s，单次，含 `player_id/match_id/nick/exp/jti/is_admin`）、SQLite、每 IP 登录/ticket 10/min（账号 6/min，ARCH §19.3）。
**OAuth（ARCH §19.2）**：Authorization Code + **PKCE**（S256），回调到 gateway；**不保存第三方 access token**（仅一次取 profile）。**数据最小化（§19.4）**：存昵称、设置、基础战绩、可选 OAuth 的 `provider_id`（**哈希**）；**不存邮箱**（除非 OAuth 必须，待人定）；IP 仅短期（≤ 30 天）。游客的进度保存在 gateway（以匿名 ID）与本地 token，丢 token 即丢进度（诚实提示，GD §18.2）。严格 CSP、无第三方脚本（§19.5）。
战绩来源：对局结束后 `server` 经 M1-029 的内部 API 上报 `MatchSummary`（M2-017：Match XP、总分、K/D/A、Role XP 增量、胜负）。账号等级 `XP_acc(n)=400×(n−1)^1.5`、兵种等级 `125n(n−1)`（只存累计 XP，等级由读取时计算）。

## 具体内容
### 要创建/修改的文件
- `crates/gateway/src/auth/oauth.rs` —— `GET /auth/oauth/{discord|google}/start`（生成 `state`、`code_verifier`，存短期服务端会话 ≤ 10 min）、`GET /auth/oauth/{provider}/callback`（校验 state、换 code、取 profile、立即丢弃 token）
- `crates/gateway/src/auth/upgrade.rs` —— `POST /auth/upgrade`：游客（带游客 token）绑定 OAuth 账号，合并战绩/设置，旧游客 id 作废
- `crates/gateway/src/profile.rs` —— `GET/PUT /me/settings`（JSON ≤ 16 KB，带 `schema_version`）、`GET /me/stats`
- `crates/gateway/src/internal/stats.rs` —— `POST /internal/match_result`（仅 server 服务账户，签名校验）
- `crates/gateway/migrations/00xx_accounts_stats.sql` —— `accounts(id, kind guest|oauth, provider, provider_id_hash, nick, created_at, last_seen)`、`settings(account_id, json, updated_at)`、`stats(account_id, matches, wins, kills, deaths, assists, score, account_xp, role_xp_json, updated_at)`、`match_results(match_id, account_id, summary_json, created_at)`
- `crates/gateway/tests/oauth_mock.rs` —— 本地 mock IdP（wiremock 级别，若新增依赖须先 PLAN_ISSUES + `cargo deny`）
### 要实现的行为
1. 登录/换票限速沿用 M1-028（IP 10/min、账号 6/min）；OAuth start/callback 同限速。
2. `state` 不匹配/过期/重复使用 → 400；`code_verifier` 仅在服务端会话里，失败即销毁；回调后写入 `accounts`，`provider_id_hash = SHA-256(provider || ":" || sub || server_pepper)`（pepper 来自环境变量，不入库）。
3. 游客升级：以事务合并，`stats` 数值相加、`role_xp` 逐兵种相加、`settings` 以账号侧优先；失败回滚；合并后签发新会话 token。
4. 战绩写入幂等：以 `(match_id, account_id)` 唯一，重复上报忽略；只接受已知 server 服务账户。
5. 昵称：NFKC 规范化，≤ 24 字符，拒绝控制/双向覆盖字符（ARCH §19.1）；OAuth 昵称仅作初值，用户可改。
6. 无 OAuth 配置（缺 client id/secret）时 OAuth 路由返回 501 且游客路径完全可用；启动日志明确提示。
### 接口与数据
- 环境变量：`OAUTH_DISCORD_CLIENT_ID/SECRET`、`OAUTH_GOOGLE_CLIENT_ID/SECRET`、`OAUTH_REDIRECT_BASE`、`ID_HASH_PEPPER`；错误响应不泄露内部细节。

## 不做什么
- 管理员 API/举报/封禁 → M2-023；GDPR 导出/删除、隐私政策/Cookie 声明 → M4（NEEDS_HUMAN H-074、H-082）。
- 排行榜、好友、成就、库存（A8 无）；账号等级的 UI 展示 → M2-042。
- 邮箱收集、第三方 API 调用（仅 profile 一次）。

## 验收标准
1. OAuth 流程端到端通过：用 mock IdP 的 Discord 与 Google 两条路径各跑通 start → callback → 会话 token → `/me/stats`；无真实应用时该项记 `partial`（真实 IdP 项 `未测`）。
2. 审计"不保存第三方 access token"：`cargo test -p gateway no_third_party_tokens` 扫描 DB 全表与日志输出，断言不含 mock IdP 发出的 token 字符串；代码中无存储 token 的字段（`rg access_token crates/gateway` 仅出现在内存临时变量）。
3. PKCE：缺 `code_verifier` 或 verifier 不匹配时 IdP mock 拒绝换码，gateway 返回 400；`state` 重放被拒；各 1 个测试。
4. 游客升级：升级前后 `stats` 总和守恒（kills/deaths/score/xp 逐字段），事务中途失败回滚（注入故障测试）。
5. `/internal/match_result` 重放同一 `match_id` 不重复累计；未签名请求 401。
6. 设置 PUT 超过 16 KB 返回 413；昵称 25 字符被拒；`provider_id_hash` 长度 = 64 hex。
7. 限速：同 IP 第 11 次/min OAuth start 返回 429；账号第 7 次/min 登录返回 429。
8. 端到端 P95 延迟（本地）：`/me/stats` ≤ 20 ms（`wrk` 或测试内计时，100 并发）。

## 测试要求
- 单元测试：PKCE 生成/校验、state 存储过期、昵称规范化、哈希函数、合并算法。
- 集成：`cargo test -p gateway` 含 mock IdP；`sqlx` 迁移测试（空库 → 最新）。
- 性能基准：无硬阈值，记录。
- 回归：M1-028 的 ticket 过期/重放单测保持通过。

## 性能预算
无（gateway 无新增预算）；不得使 ARCH §18.1 的 gateway 2 vCPU/2 GB 假设回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工项：OAuth 应用注册（Discord Developer Portal / Google Cloud Console）、回调 URL、隐私政策链接——NEEDS_HUMAN H-063；应用名称依赖 H-064。缺失则本任务 `partial`，并在 STATUS blocked 清单登记。
- 游客 token 存 `localStorage` 的 XSS 风险依赖严格 CSP（ARCH §19.2/§19.5），CSP 头在本任务的静态响应中验证。
- 若需新增 HTTP 客户端依赖（`reqwest` 已在 server 清单，gateway 清单未列）→ 先写 PLAN_ISSUES。
