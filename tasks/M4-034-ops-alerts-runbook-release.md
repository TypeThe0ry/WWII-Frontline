# M4-034 运维：告警、运行手册与发布回滚

## 元数据
- 里程碑：M4
- 工作流：ops
- 规模：M
- 前置依赖：M2-048, M4-033
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §18.3; ARCHITECTURE §18.5; ARCHITECTURE §18.4; ARCHITECTURE §18.2; NEEDS_HUMAN H-093, H-065

## 背景
把 ARCHITECTURE §18.3 的监控指标与告警阈值落成可执行的告警规则，并建立运行手册与发布/回滚流程。告警阈值（逐条实现）：
- `server_tick_ms` P99 > 28 ms 持续 1 min → 警告；> 33 ms → 严重。
- `server_tick_overrun_total` 速率 > 0.5/min。
- `net_bytes_out_per_client` P99 > 400 kbps；`net_rtt_ms` P95 > 150 ms。
- `suspicion_score`/复核队列长度；`deploy_requests_total{result}` 拒绝率异常。
- `livekit_api_errors_total` 错误率 > 1%。
- `llm_cost_usd_total` 单局 > 0.45（若启用 LLM）。
- 进程 RSS 单调增长；gateway 5xx > 1%。
- 证书到期前 14 天（§18.2）。
发布与回滚（§18.5）：客户端构建产物内容哈希，入口 `index.html` 引用 manifest；`PROTOCOL_VERSION`/`data_hash` 变更时服务器与客户端同步发布，新局用新版本，旧局用完为止（"蓝绿"）；服务器 Docker 镜像打 tag；`gateway` 维护 `min_supported_protocol`；**回滚 = 切换 gateway 使新局启动旧镜像**；`/data` 变更 = 版本变更（`data_hash`）。
日志（§18.4）：结构化 JSON → Loki；不记录语音内容、聊天全文；审计日志保留 1 年。

## 具体内容
### 要创建/修改的文件
- `deploy/prod/prometheus/alerts.yml` —— 全部告警规则（每条含 `severity`、`runbook_url`、`for`）。
- `deploy/prod/alertmanager.yml` —— 路由（接收人待 H-093；默认写到本地文件/Webhook 占位）。
- `ops/runbooks/*.md` —— 事故运行手册（见下）。**这是运维产物，不是规划 docs**。
- `deploy/prod/scripts/{release.sh,rollback.sh,blue_green.sh}`；`gateway` 增加 `min_supported_protocol` 与镜像 tag 选择配置（`data/gateway/release.json`）。
- `tools/ops/alert_drill.sh` —— 逐条触发演练。
- `reports/ops/m4-drills.md` —— 演练记录模板与结果。
### 要实现的行为
1. 规则落地：每条告警都有 `expr`、`for`、`severity`、`summary`、`runbook` 链接；`promtool check rules` 通过。
2. 运行手册（每份 ≤ 1 页，含症状→诊断命令→缓解→升级）：tick 超限/高 P99、连接洪水/DDoS（提示由人类 H-087 决定供应商）、证书过期、LiveKit 故障（降级为无语音）、gateway 5xx、内存泄漏、数据泄露/凭据泄露事件、回滚、对局崩溃、复核队列积压、LLM 成本超限。
3. 发布：`release.sh` 构建镜像与客户端 → 上传 R2（不可变）→ 更新 gateway 的目标版本；旧局不被打断，新局用新版本；不兼容（`PROTOCOL_VERSION`/`data_hash` 变）时旧客户端在握手被 Reject 并提示刷新。
4. 回滚：`rollback.sh` 将 gateway 目标版本切回上一 tag；新局启动旧镜像；已在局的玩家不受影响；校验 `data_hash` 与客户端兼容。
5. 演练：在预发布/生产对每条告警制造触发条件（可用 `stress`/故障注入/调低阈值）并记录触发时间与通知到达；回滚演练计时。
6. 备份/恢复：gateway 数据库备份脚本与一次恢复演练（H-093）。
### 接口与数据
- `release.json`：`{"current":"v1.2.3","previous":"v1.2.2","min_supported_protocol":N,"images":{"server":"sha256:…","gateway":"…"}}`。

## 不做什么
- 不选择/购买 DDoS 防护或寻呼服务（H-087/H-093 人工）。
- 不做生产环境搭建（M4-033）、隐私合规（M4-035）、对外公告（M4-037）。
- 不做自动扩缩容。
- 不做多区域故障转移。

## 验收标准
1. **回滚演练 ≤ 10 min**：从"决定回滚"到"新局使用旧镜像且冒烟通过"，计时 3 次，P95 ≤ 600 s，记录在 `reports/ops/m4-drills.md`。
2. **告警规则均有触发演练记录**：§18.3 表内每条告警（≥ 12 条）在演练中被触发并到达接收端，记录触发/到达时间戳；缺失条数 = 0（对无法演练者须说明并标 `partial`）。
3. `promtool check rules` 与 `amtool check-config` 退出码 0；每条规则含 `runbook` 注解（脚本断言 100%）。
4. 蓝绿：发布期间在局玩家连接不断开（断线数 = 0，10 人 headless 在局时发布并观察 5 min）；新局版本为新镜像。
5. 版本不兼容路径：旧客户端连新服务器在握手被 Reject 且显示"请刷新"（e2e）；`min_supported_protocol` 生效。
6. 备份恢复演练成功：恢复后玩家战绩/设置条目数一致（计数比对）。
7. 运行手册 ≥ 10 份，每份含"症状/诊断/缓解/升级"四节（脚本检查标题）。

## 测试要求
- 单元测试：`promtool test rules`（对每条规则提供合成时间序列单测）。
- 集成/无头测试：`alert_drill.sh`；`headless-client` 10 人在局发布/回滚。
- 性能基准：无。
- 回归：规则与 `release.json` 变更须通过 `promtool` 与 `rollback.sh --dry-run`。

## 性能预算
- 监控开销：指标采集对对局进程 CPU 增量 ≤ 1%（对比开关前后 `tick_ms`）；日志限速 1 秒防洪（§18.4）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`promtool`、`shellcheck`、`cargo test`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 告警接收人、值班与密钥轮换演练需人类提供（H-093、H-065）；缺失则告警只落本地并标 `partial`。
- LLM 成本告警仅在 M3-027 启用时有意义；未启用则规则存在但标记 `disabled`。
- 蓝绿"旧局用完为止"意味着旧镜像需保留 ≥ 最长局时长（90 min Frontline）；运维脚本需检查在局进程再回收。
