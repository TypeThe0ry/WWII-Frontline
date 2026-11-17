# M4-033 运维：生产部署

## 元数据
- 里程碑：M4
- 工作流：ops
- 规模：M
- 依赖：M1-067（预发布部署）, M4-031, M2-048（监控与日志）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §18.1; ARCHITECTURE §18.2; ARCHITECTURE §5.17; ARCHITECTURE §3.2; ARCHITECTURE §19.2; NEEDS_HUMAN H-015, H-016, H-017, H-018, H-087, H-093

## 背景
把预发布（M1-067）提升为生产：对局主机、gateway、LiveKit、R2 + CDN、证书与监控。拓扑与起步配置（ARCHITECTURE §18.1）：
| 组件 | 起步 | 备注 |
|---|---|---|
| 对局主机 ×1–2 | 专用/高频云，≥ 4 物理核、16 GB、1 Gbps | 每机 2 局并发（M2 实测） |
| gateway ×1 | 2 vCPU / 2 GB | 无状态；SQLite→Postgres |
| LiveKit ×1 | 4 vCPU / 8 GB（与对局同机房） | 容量待 M2 压测确认 |
| R2 + CDN | Cloudflare | 静态资源 |
| 监控 | 1 小机 | Prometheus + Grafana + Loki |
容量估算：2 台 × 2 局 = 4 局 × 100 = 400 并发上限；起步档位由 H-087 决定（默认精简档：1 台对局 + gateway/LiveKit 合并）。
TLS（§18.2）：WebTransport 要求受信任证书（Let's Encrypt），绑定对局主机域名（待人定）；由 `certbot`/`lego` 自动续期，`server` 监听 SIGHUP 重载证书（不重启对局）；`serverCertificateHashes` 仅本地开发；**到期前 14 天告警**。静态资源：内容哈希命名、不可变、R2 + CDN；`.fpak` 为唯一素材下发格式（R11）。
**本任务强依赖人类提供生产主机/域名/CDN/账号**（H-015/016/017/018）；缺失则 `blocked`，执行者不得自行租用或购买。

## 具体内容
### 要创建/修改的文件
- `deploy/prod/` —— `docker-compose.yml`（或 systemd 单元）、`env.example`（无密钥）、`caddy|nginx` 配置、`livekit.yaml`、`prometheus/`、`grafana/dashboards/`、`loki/`。
- `deploy/prod/scripts/{provision.sh,deploy.sh,rollback.sh,cert-reload.sh,smoke.sh}`。
- `docs/` **不改**；运行手册在 M4-034；本任务产出 `deploy/prod/README.md`（部署步骤，非规划文档）。
- `.github/workflows/deploy-prod.yml` —— 手动触发、需审批、使用 OIDC/受限 token；上传 `.fpak` 到 R2（内容哈希命名）。
- `tools/ops/cert_expiry_exporter.*` —— 证书到期指标（`cert_expiry_days`）。
### 要实现的行为
1. 基础设施即脚本：从空主机到运行所需的全部步骤可重复（幂等 `provision.sh`），敏感值只来自环境/密钥存储（`request_credential`/CI secrets），不进仓库、不进日志。
2. 部署：对局镜像（Docker，带 tag）、gateway、matchd（M4-031）、LiveKit、监控；客户端静态资源上传 R2 并按 manifest 切换入口 `index.html`。
3. TLS：证书申请/续期、SIGHUP 重载演练；`cert_expiry_days` 指标与 14 天告警规则（规则本体在 M4-034，此处提供指标源）。
4. 外网冒烟：从外网用 Chromium（WebTransport）与强制 WS 各完成"主菜单→浏览器→加入→进入对局→发射/移动→退出"；语音（LiveKit）可加入房间并互听（2 个客户端）。
5. 成本/容量记录：把实际主机型号、带宽、月费（按实时报价，**不写入文档的价格需标"未核实"**）记入 `deploy/prod/CAPACITY.md`，供 M4-037 引用。
6. 安全基线：防火墙仅开放 443/TCP、443/UDP、LiveKit 所需端口（以官方文档为准，未核实）；SSH 密钥登录；内网接口（`/internal/*`）不对外。
### 接口与数据
- 环境变量清单 `deploy/prod/env.example`：`GATEWAY_URL`、`MATCH_PUBKEY`、`LIVEKIT_URL`、`R2_*`（占位）、`PUBLIC_DOMAIN`；运行时配置通过 `/data` 与环境注入，客户端不硬编码域名。

## 不做什么
- 不租用/购买主机、域名、CDN（人工，H-015/H-016/H-017/H-018）。
- 不做告警规则/运行手册/回滚演练（M4-034）；不做隐私合规（M4-035）。
- 不做多区域/高可用（首发单区域，ARCHITECTURE §20 第 12 项）。
- 不做 DDoS 供应商集成（H-087 决定后另立）。

## 验收标准
1. **外网全流程可玩**：从公网网络（非内网）的 Chromium 与 WS 兜底各完成一次全流程冒烟（`smoke.sh` 退出码 0），并记录 RTT；缺任一项则 `partial`。
2. **证书续期演练**：在预发布/生产用 `certbot --dry-run`（或 `lego` 等价）成功，并演练 SIGHUP 重载：重载期间已在局玩家连接保持（断线数 = 0，测 30 s）。
3. `cert_expiry_days` 指标可在 Prometheus 查询；人为把阈值设为 90 天时触发测试告警（演练证据存档，规则在 M4-034）。
4. 静态资源：入口与 `.fpak` 全部内容哈希命名，响应头 `Cache-Control: public, max-age=31536000, immutable`（`curl -I` 抽查 20 个）；无独立 .glb/.ktx2/.ogg 对外暴露。
5. 部署可重复：在全新主机上 `provision.sh && deploy.sh` 二次运行结果一致（幂等；第二次变更数 = 0）。
6. 安全基线：外部端口扫描（`nmap` 对公网 IP）仅暴露预期端口；`/internal/*` 外网访问返回 403/404。

## 测试要求
- 单元测试：脚本 `shellcheck` 通过；配置模板渲染测试。
- 集成/无头测试：`smoke.sh`（含 `headless-client` 10 人、WT 与 WS 各一组）；LiveKit 房间双客户端互听探测。
- 性能基准：主机 `tick_ms` 基线与 M2-049 对比（`/metrics`），偏差 ≤ 10%。
- 回归：部署脚本变更须附 `smoke.sh` 通过记录。

## 性能预算
- 沿用服务器预算（§6.10：稳态 ≤ 18 ms，P99 ≤ 28 ms）与带宽预算（§5.6）；生产主机不得使 `tick_ms` P99 比预发布差 > 10%。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`shellcheck`、`cargo test`、`smoke.sh`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **阻塞条件**：缺少生产主机/域名/Cloudflare/LiveKit 选择（H-015/H-016/H-017/H-018）时，任务置 `blocked`，STATUS 登记"所缺条目 — 日期 — 谁来解除"，并在 `docs/NEEDS_HUMAN.md` 追加条目；可先在预发布环境验证脚本（标 `partial`）。
- 费用/价格/LiveKit 端口细节均**未核实**（ARCHITECTURE §14/§18），实施时按官方文档与实时报价。
- 依赖 H-087（规模与 DDoS 供应商批准）与 H-093（告警接收/备份恢复演练）。
