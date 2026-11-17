# M4-032 安全：加固与审计

## 元数据
- 里程碑：M4
- 工作流：server
- 规模：M
- 前置依赖：M4-030, M2-023
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §19; ARCHITECTURE §17.1; ARCHITECTURE §19.1; ARCHITECTURE §19.3; ARCHITECTURE §19.5; ARCHITECTURE §15.1

## 背景
公开测试前的安全审计与加固，范围对应 ARCHITECTURE §19：
- **输入不可信（§19.1）**：所有网络输入（二进制消息、字符串、数组长度）解码时做长度/范围检查，失败即断开；解码器经 `cargo-fuzz`；**解码路径禁用 `unsafe`**；昵称 ≤ 24、聊天 ≤ 200，NFKC 规范化、拒绝控制字符/双向覆盖字符；UI 层禁止 `innerHTML` 注入用户内容；每连接缓冲 64 KB、握手超时 5 s、每 IP 并发连接 ≤ 4、每秒新连接限速。
- **Token（§19.2）**：join ticket EdDSA、TTL 60 s、单次使用（`jti`）；游客 JWT TTL 24 h；管理 API 独立 token + IP 白名单 + 审计。
- **速率限制（§19.3）**：gateway 登录/ticket 每 IP 10/min、每账号 6/min；聊天每人每 8 s 最多 5 条（GD §16.2）；举报 1/min、10/局；连接每 IP 4 并发。
- **供应链/前端（§19.5）**：`cargo-deny`、`npm audit`/`osv-scanner`、锁文件进库；第三方脚本零；CSP：`default-src 'self'; connect-src 'self' https://*.game-domain wss://… ; script-src 'self' 'wasm-unsafe-eval'`；`frame-ancestors 'none'`；入口脚本 SRI；不依赖 `SharedArrayBuffer`。
- 目标（MILESTONES E4-9）：解码 fuzz 累计 ≥ 1 h 无 panic；CSP 无 `unsafe-inline`；依赖审计无高危未处理。

## 具体内容
### 要创建/修改的文件
- `crates/protocol/fuzz/fuzz_targets/*.rs` —— 覆盖全部消息解码器（含 M3/M4 新增：能力、炮兵、登陆艇状态、复核等）。
- `crates/server/tests/limits.rs` —— 速率/连接限制回归测试；`crates/gateway/tests/ratelimit.rs`。
- `client/index.html`、`client/vite.config.ts`（CSP 与 SRI 插件）、`deploy/nginx|caddy` 头部配置（CSP/HSTS/`X-Content-Type-Options`/`Referrer-Policy`）。
- `client/tools/security/{no-innerhtml.ts,csp-check.ts}` —— ESLint 规则 + CSP 校验脚本。
- `reports/security/m4-audit.md` —— 审计报告（发现、严重级、修复/豁免）。
- `.github/workflows/security.yml` —— `cargo-deny`、`npm audit --omit=dev`、`osv-scanner`、fuzz smoke（10 min）。
### 要实现的行为
1. 长跑 fuzz：每个解码靶 ≥ 10 min，总累计 ≥ 1 h（CPU 时间），语料入库（小而精）；任何 panic/超时/OOM 为阻断缺陷。
2. 字符串过滤审计：昵称/聊天/小队名走同一规范化函数；对双向控制字符、零宽、超长、混合脚本样例做表驱动测试。
3. CSP：生产构建的 HTTP 头与 `<meta>` 均无 `unsafe-inline`/`unsafe-eval`（`wasm-unsafe-eval` 允许）；内联脚本改为外链或带 SRI/nonce；`frame-ancestors 'none'`。
4. 速率/连接限制：逐项对照 §19.3 表做黑盒压测（gateway ticket、聊天、举报、新连接/IP）；超限返回确定性错误码，且不放大（无反射/放大响应）。
5. 依赖审计：`cargo deny check`（licenses+advisories）与 `npm audit` 无未处理高危；每个豁免写入 `deny.toml`/`audit-exceptions.md` 并注明理由与复查日期。
6. UI 注入检查：全仓 grep + ESLint 规则禁止 `innerHTML`/`dangerouslySetInnerHTML` 用于用户内容；Playwright 注入测试（昵称含 `<img onerror>`）无执行。
7. 密钥/日志：扫描日志与仓库确认无 API key/JWT（`gitleaks` 类扫描，需过依赖政策）。
### 接口与数据
- 无新协议字段；CSP/头部以配置文件为准；审计报告模板含 `id, area, severity, status, evidence`。

## 不做什么
- 不做渗透测试服务采购/外部审计（人工，H-087/运维范围）。
- 不实现 DDoS 防护（主机/云厂商层，ARCHITECTURE §19.3 注）。
- 不做客户端反篡改/混淆（只增加成本，不作为安全措施，§15.4）。
- 不改变游戏规则。

## 验收标准
1. **fuzz 累计 ≥ 1 h 无 panic**：`cargo fuzz run <target> -- -max_total_time=...` 汇总各靶时长 ≥ 3600 s，crash/timeout/oom 数 = 0（日志存档 `reports/security/fuzz-summary.json`）。
2. **CSP 无 unsafe-inline**：`pnpm -C client security:csp` 对生产构建输出头部检查，`unsafe-inline`/`unsafe-eval` 出现次数 = 0；`frame-ancestors 'none'` 存在；入口脚本带 SRI。
3. **依赖审计无高危未处理**：`cargo deny check` 退出码 0；`npm audit --omit=dev --audit-level=high` 退出码 0（或豁免条目均有复查日期）；`osv-scanner` 高危 = 0。
4. 速率限制：对 §19.3 每一行做压测，实际限制与规定值偏差 ≤ ±1（如 10/min 不得放行第 12 个）；全部断言通过（`cargo test -p gateway ratelimit`）。
5. 字符串过滤：表驱动 ≥ 50 条样例 100% 通过；`innerHTML` 违规数 = 0（ESLint）。
6. 解码路径 `unsafe` 数 = 0（`cargo geiger`/grep 断言，范围 `crates/protocol/src/decode*`）。
7. 审计报告中严重/高危发现数 = 0 或均有修复提交。

## 测试要求
- 单元测试：规范化函数、限流器（令牌桶）、ticket 重放拒绝、CSP 解析。
- 集成/无头测试：`headless-client` 恶意流量场景（超长数组长度、畸形帧、握手慢速攻击 slowloris、重复 `jti`）；服务器不 panic、连接被断开、`/metrics` 计数增加。
- 性能基准：限流器开销 ≤ 5 µs/消息（`cargo bench`）。
- 回归：fuzz 语料与 CSP 断言入 CI（nightly 跑长 fuzz，PR 跑 smoke）。

## 性能预算
- 输入摄取阶段仍 ≤ 0.8 ms/tick（§6.10 阶段 1）；限流/校验增量 ≤ 0.05 ms/tick。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- CSP 的 `connect-src` 域名依赖 H-017（域名）；未确定前用占位域名并在 STATUS 标注 `partial` 项。
- WebTransport 与 CSP `connect-src` 的交互在不同浏览器差异**未核实**，需在 M4-042 的浏览器矩阵验证。
- 引入 `cargo-geiger`/`gitleaks`/`osv-scanner` 等新工具需先过依赖政策（H-037）并写 PLAN_ISSUES；它们只在 CI 使用，不进运行期。
