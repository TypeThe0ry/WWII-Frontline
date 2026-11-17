# M1-067 运维：预发布部署

## 元数据
- 里程碑：M1
- 工作流：ops
- 规模：M
- 依赖：M1-066, M1-005（manifest 与体积门禁，用于 R2 上传）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §18.1; ARCHITECTURE §18.2; ARCHITECTURE §5.17; ARCHITECTURE §11.6; ARCHITECTURE §18.5

## 背景
**[人工：域名/主机/云账号]** M1 的 20 真人测试局需要外网可访问的预发布环境。ARCHITECTURE §5.17/§18.1：对局主机 Hetzner/OVH 专用或高频云（Docker，≥ 4 物理核、16 GB、1 Gbps）；gateway 小型云主机（2 vCPU/2 GB）；静态资源 Cloudflare R2 + CDN；证书 Let's Encrypt。§18.2：WebTransport 要求服务端有**受信任证书**，绑定对局主机域名（域名待人定）；证书由 `certbot`/`lego` 自动续期，`server` 监听 SIGHUP 重载证书（不重启对局）；证书到期前 14 天告警（生产 M4-033，预发布先做演练）。§11.6：资产 immutable 1 年，manifest/入口 HTML `no-cache` + ETag。§18.5：版本不兼容（`PROTOCOL_VERSION`/`data_hash`）时蓝绿发布。本任务**需要人类提供**：域名与 DNS（H-017）、服务器租用与厂商账号（H-016，选址 H-055）、Cloudflare 账号 R2（H-015）、预算批准（H-020）。缺任一项时任务置 `blocked`，登记 blocked 清单并追加 NEEDS_HUMAN 条目，继续做不依赖的任务。执行者**永不索取明文密钥进聊天**，凭据只通过环境密钥存储/`request_credential` 提供。

## 具体内容
### 要创建/修改的文件
- `deploy/staging/docker-compose.yml` —— 对局主机：`server`（host 网络或固定 UDP 端口范围）；gateway 主机：`gateway` + Caddy/nginx 反代（HTTPS）；证书目录卷挂载。
- `deploy/staging/cert/renew.sh`、`cert/hook-reload.sh` —— `certbot`/`lego` 续期脚本与续期后 `kill -HUP` 指向 server 的 hook；`server` 的 SIGHUP 证书热加载实现（若 M0-017 未实现则在 `crates/server/src/tls.rs` 补，并写单测）。
- `tools/deploy/upload-r2.mjs` —— 按 manifest 上传 `.fpak`（内容哈希命名、`Cache-Control: public, max-age=31536000, immutable`）与 manifest（`no-cache`）；干跑模式；大小预算复核（M1-005 门禁）。
- `deploy/staging/runbook.md` —— 部署/回滚/证书续期演练步骤、端口与防火墙清单、DNS 记录示例（使用占位域名）。
- `.github/workflows/deploy-staging.yml`（可选，手动触发）—— 构建镜像、推送（凭据用 GitHub Secrets，不进仓库）。
- `deploy/staging/healthcheck.sh` —— 外网探针：gateway `/healthz`、WT 握手、WS 握手、`.fpak` Range。
### 要实现的行为
1. 对局主机与 gateway 部署：容器随主机重启自启动（`restart: unless-stopped`）；防火墙仅开放 443/TCP、对局 UDP 端口、SSH（限源）。
2. 证书：Let's Encrypt 签发并自动续期；续期后 `server` 无需重启即可加载新证书（SIGHUP）；证书到期监控（14 天告警，脚本/Prometheus 黑盒均可，正式监控 M2-048）。
3. R2 上传：核对 manifest 哈希，`.fpak` 以内容哈希命名；入口 HTML 与 manifest `no-cache`。
4. 外网浏览器可经 WT 连入，WT 被阻断则 WS 兜底。
5. 版本发布：新版本 `PROTOCOL_VERSION` 变更时 gateway `min_supported_protocol` 更新；旧局运行完为止（蓝绿，§18.5）。
6. 回滚：切换 gateway 使新局启动旧镜像 tag；演练一次。
### 接口与数据
```text
环境变量（示例名，不含值）：GATEWAY_DOMAIN, MATCH_HOST_DOMAIN, R2_BUCKET, R2_ENDPOINT, CERT_DIR, SERVER_UDP_PORT_RANGE
GitHub Secrets / 主机密钥库：GATEWAY_SIGNING_KEY, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, ACME_EMAIL
```

## 不做什么
- 不做生产部署、DDoS 方案、CDN 域名与证书监控告警平台（M4-033、M2-048）。
- 不做 LiveKit 部署（M2-024）、监控栈（M2-048）。
- 不做多机调度/`matchd`；不做自动扩缩容。
- 不购买域名/主机，不接触支付信息（人类，H-015/H-016/H-017）。

## 验收标准
1. 外网浏览器经 WT 连入预发布服务器并进入部署界面：Playwright（外部网络运行）Chromium 脚本成功，遥测传输类型 `wt`；INDEX 指标。
2. UDP 被阻断的网络下自动走 WS 并连入（受限网络测试台，H-022；或本机防火墙规则模拟）。
3. 证书续期演练通过：强制 `certbot renew --force-renewal`（或 `lego renew`）后，`openssl s_client` 查看到新证书序列号，且已建立的对局连接不断开，新连接使用新证书（记录前后序列号）。
4. 证书到期告警演练：把告警阈值临时设为 90 天，脚本在预期时刻触发告警输出（日志/邮件）。
5. R2 上传校验：随机抽 10 个 `.fpak`，`sha256` 与 manifest 一致；响应头含 `immutable`（`curl -I`）；manifest 响应头含 `no-cache`。
6. 回滚演练：切换到上一个镜像 tag，新局在 ≤ 120 s 内以旧版本启动并可加入。
7. 缺人工输入（域名/主机/云账号）时任务状态为 `blocked`，STATUS 顶部 blocked 清单已登记（"所缺条目—日期—谁来解除"），NEEDS_HUMAN 已追加。

## 测试要求
- 单元测试：SIGHUP 证书热加载（`cargo test -p server tls_reload`）；上传脚本干跑与哈希校验。
- 集成/无头测试：外网 Playwright；`healthcheck.sh` 在 cron 或手动运行；headless-client 单客户端 10 min。
- 性能基准：从外网下载 core 包与 R2 `.fpak` Range 延迟记入 `reports/m1-staging.md`。
- 回归：M1-066 本地开发栈保持可用。

## 性能预算
- ARCHITECTURE §18.1：每机 2 局并发的容量估算在 M2 实测；本任务仅记录单局 `tick_ms` 与 RSS 作基线，不新增预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`（缺人工输入则 blocked 并标注）
- [ ] 测试通过；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：runbook 与端口表；偏差记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 价格与流量成本未核实（ARCHITECTURE §5.17/§18.1）；租用前由人类批准 H-020。
- WebTransport 对 HTTP/3 与 UDP 的网络环境较敏感（企业/校园网阻断 UDP），WS 兜底是首发必备路径。
- 任何凭据不得出现在聊天、日志、提交或 compose 文件中；用 GitHub Secrets/主机密钥库/`request_credential`。
