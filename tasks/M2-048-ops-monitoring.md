# M2-048 运维：监控与日志

## 元数据
- 里程碑：M2
- 工作流：ops
- 规模：M
- 依赖：M1-067, M2-021
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §18.3; ARCHITECTURE §18.4; ARCHITECTURE §18.1; ARCHITECTURE §6.10; ARCHITECTURE §19.4; ARCHITECTURE §16.4

## 背景
ARCHITECTURE §18.1：监控 = 1 台小机（Prometheus + Grafana + Loki）。§18.3 监控指标表（本任务必须全部有看板与对应告警）：
`server_tick_ms`（直方图；P99 > 28 ms 持续 1 min 警告，> 33 ms 严重）、`server_tick_overrun_total`（速率 > 0.5/min）、`server_players{team}`、`server_bots`、`net_bytes_out_per_client`（P50/P99；P99 > 400 kbps 告警）、`net_packet_loss_ratio`、`net_rtt_ms`（P95 RTT > 150 ms）、`aoi_starved_total`、`snapshot_budget_utilization`、`lagcomp_clamped_total`、`lagcomp_wall_reject_total`（突增）、`suspicion_score`、`reports_total`（复核队列长度）、`deploy_requests_total{result}`（拒绝率异常）、`livekit_subscription_updates_total`、`livekit_api_errors_total`（错误率 > 1%）、`llm_calls_total`、`llm_cost_usd_total`、`llm_fallback_total`（单局成本 > 0.45；LLM 指挥官 M3 才启用，指标先注册）、进程 CPU/RSS/FD（RSS 单调增长告警）、gateway 请求延迟/错误率/登录失败率（5xx > 1%）、客户端遥测（匿名、可选：帧时间分布、画质档、传输类型 wt/ws、着色器编译耗时）。
§18.4 日志：结构化 JSON（`tracing` + `tracing-subscriber`）→ stdout → Loki；字段 `match_id`、`tick`、`player_id`、`event`、`level`；默认 INFO，`DEBUG` 可按 `match_id` 动态开关；每秒限速防洪；**不记录语音内容、聊天全文（仅举报关联片段保留 14 天）；IP 仅在 gateway 访问日志与封禁表中保留**；封禁/踢出/管理员操作审计保留 1 年。M2-021 提供 tick 分段耗时指标；M1-067 提供预发布部署。

## 具体内容
### 要创建/修改的文件
- `ops/monitoring/docker-compose.yml`（与 M1-066 的 compose 目录约定一致；若 M1-066 选了其它目录，跟随之）—— Prometheus、Grafana、Loki（+ promtail/Alloy 收集器）、Alertmanager。
- `ops/monitoring/prometheus/prometheus.yml`、`rules/*.yml` —— 抓取配置（server、gateway、node 指标）与告警规则（上表全部阈值）。
- `ops/monitoring/grafana/dashboards/{match-server,network,gateway,anticheat,voice,llm,process}.json`、`provisioning/` —— 看板即代码。
- `ops/monitoring/loki/config.yml`、`promtail.yml` —— 日志管道、标签（`match_id`、`level`、`event`）、保留期（30 天；审计另存 1 年）。
- `crates/server/src/metrics.rs`、`crates/gateway/src/metrics.rs`（修改）—— 补齐 §18.3 缺失的指标（名称严格按表）；`crates/server/src/logging.rs` —— 日志限速（令牌桶 ≤ 50 条/s/级别 INFO，ERROR 不限）与按 `match_id` 动态 DEBUG（管理端点，需鉴权）。
- `client/src/telemetry/frame_stats.ts`（可选，默认关）—— 匿名遥测上报（帧时间分布、画质档、传输类型、着色器编译耗时；经 gateway 的 `POST /v1/telemetry`，用户可在设置关闭，ARCH §19.5 隐私）。
- `ops/monitoring/README.md` 不创建；运行说明写入 STATUS 与 PLAN_ISSUES（如需）。
### 要实现的行为
1. 全部 §18.3 指标有抓取点与看板面板；缺失的指标在 server/gateway 补注册（即使 M2 值恒为 0，如 `llm_*`）。
2. 告警：按表阈值配置，Alertmanager 路由到一个接收端（webhook/邮件，凭据不入库）；告警含 `match_id` 与 runbook 链接占位。
3. 日志：JSON 字段齐全；敏感内容过滤（语音/聊天全文/IP 不入对局日志）；限速生效。
4. 人为触发：提供 `tools/ops/inject-tick-overrun.sh`（或 server 的测试开关 `FRONTLINE_TEST_TICK_DELAY_MS`，仅测试构建）使 tick_ms P99 > 28 ms 持续 1 min，验证告警触发。
### 接口与数据
- 指标命名与类型严格取自 ARCH §18.3；标签基数受控（`player_id` 不作为指标标签，仅出现在日志）。
- Loki 标签：`job`、`match_id`、`level`、`event`；其余字段留在日志体中。

## 不做什么
- 不做生产部署、证书与域名（M4-033）、告警运行手册与回滚演练（M4-034）。
- 不做 PagerDuty/值班（H-093 人类事项）。
- 不做 LLM 指挥官本体（M3）；仅预注册 `llm_*` 指标。
- 不做游戏内管理员面板。

## 验收标准
1. 看板覆盖：脚本 `tools/ops/check-dashboards.sh` 对 §18.3 表的 14 行指标族逐项检查，Grafana 看板 JSON 中存在引用该指标的面板，覆盖率 = 100%（缺一即失败）。
2. 告警触发：运行 `inject-tick-overrun` 使 `server_tick_ms` P99 > 28 ms 持续 ≥ 1 min，Alertmanager 在 ≤ 3 min 内发出"警告"告警（记录触发时间戳）；> 33 ms 触发"严重"。
3. 告警规则有效性：`promtool check rules` 通过；对每条规则至少 1 条 `promtool test rules` 单测（阈值边界）。
4. 日志合规：对 10 min 压测日志抽样 10 000 行，0 行包含聊天全文/语音内容/IP 字段（脚本 grep + schema 校验）；JSON 字段 `match_id/tick/player_id/event/level` 齐全率 ≥ 99%（对对局事件日志）。
5. 日志限速：突发 5 000 条 INFO/s，输出 ≤ 60 条/s（含限速摘要行），ERROR 全部保留（单测）。
6. 指标开销：开启全部指标后服务器 tick CPU 增加 ≤ 0.2 ms（ARCH §6.10 阶段 11 预算 0.2 ms，对比基准）。
7. 动态 DEBUG：对指定 `match_id` 开启/关闭 DEBUG 生效 ≤ 5 s，且需鉴权（无 token 返回 401，单测）。

## 测试要求
- 单元测试：日志限速令牌桶；指标注册去重；标签基数上限。
- 集成/无头测试：`docker compose up` 全栈 + headless-client 20 个连接，抓取所有目标 UP；告警注入演练一次并记录。
- 性能基准：指标开销对比（criterion 或 tick 采样）。
- 回归：M1-067 预发布部署流程不受影响。

## 性能预算
- ARCHITECTURE §6.10 阶段 11 日志/指标 0.2 ms/tick；监控机资源 ≤ 2 vCPU/4 GB（估计，未核实）；日志体积 ≤ 5 GB/天（估算，压测实测写入 STATUS）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`、`promtool test rules`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 监控主机（Hetzner/OVH 小机）与告警接收渠道属人类事项（H-016、H-093）；缺失时只在本地 compose 验证，状态 `partial`。
- 客户端遥测必须匿名、可选、可关闭（ARCH §19.5）；默认关闭直到隐私政策就绪（H-082），不得上传设备指纹。
- 日志不入库的原则：测试夹具不得含真实 IP/聊天内容。
