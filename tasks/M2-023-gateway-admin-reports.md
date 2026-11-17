# M2-023 gateway：管理 API 与举报

## 元数据
- 里程碑：M2
- 工作流：server
- 规模：M
- 依赖：M2-022, M1-027
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §15.3; GAMEDESIGN §19.6; ARCHITECTURE §19.3; ARCHITECTURE §19.2; ARCHITECTURE §18.4; ARCHITECTURE §19.4

## 背景
M1-027 实现了基础服务器校验与违规计数/日志；M1 已有 `Report` C0 消息（`target:u8, reason:u8, note:str ≤ 120`）。M2 需要把举报落库并提供管理员工具。要求汇总：
- **举报（GD §19.6、ARCH §19.3）**：原因 = 作弊/辱骂/捣乱/TK；每玩家 1 次/min、≤ 10 次/局；服务器保留 5 min 环形缓冲，**仅被举报时落盘**最近 60 s 回放记录；与举报关联的前后 20 条聊天保留 14 天，其余聊天内存保留至局终/24 h 供管理员查看（ARCH §19.4、GD §16.2）；语音不录制（举报无语音证据）。
- **管理员（GD §19.6）**：踢人、封禁（1 h / 24 h / 7 d / 永久）、强制换队、静音、换图、广播、查看聊天日志、查看举报与可疑统计；**所有封禁决定记录审计日志**。ARCH §19.2：管理 API = **独立 token + IP 白名单 + 审计**；ARCH §15.3：封禁对象为账号/游客 token 指纹 + IP 段并带期限；"首发不自动封禁，进入人工复核队列"；§18.4：封禁/踢出/管理员操作审计保留 **1 年**；IP 仅短期（≤ 30 天，§19.4）。join ticket 含 `is_admin`；M1-028 已有 ticket 签发，本任务让其检查封禁表。
- 对局服务器与 gateway 通过 M1-029 的内部链路通信；管理动作在 gateway 落库后经内部 API 下发到对应 `server`（server 执行：断开/静音/换队，规则仍在 sim，M2-018 的 `ServerAction`）。

## 具体内容
### 要创建/修改的文件
- `crates/gateway/src/admin/mod.rs` —— 路由 `/admin/*`、中间件（token + IP 白名单 + 审计）
- `crates/gateway/src/admin/{reports,bans,actions,audit}.rs`
- `crates/gateway/src/internal/report.rs` —— `POST /internal/report`（server 上报：reporter、target、reason、note、`match_id`、replay_ref、chat_excerpt_ref）
- `crates/gateway/migrations/00xx_admin.sql` —— `reports(id, match_id, reporter_id, target_id, reason, note, replay_ref, status, created_at)`、`bans(id, subject_kind, subject_hash, ip_cidr, reason, expires_at, created_by, created_at)`、`audit_log(id, ts, actor, action, target, params_json, result, ip)`、`admin_ip_allowlist` 配置表
- `crates/server/src/report.rs` —— 5 min 环形缓冲（输入/事件摘要）、被举报时落盘 `replays/{match_id}_{report_id}.bin`、聊天片段截取；速率限制（1/min，10/局）
- `crates/gateway/src/ticket.rs`（修改）—— 签发 ticket 前查封禁表；被封返回 403 + 到期时间
### 要实现的行为
1. 管理员认证：`Authorization: Bearer <admin_token>`；token 以哈希存储，常量时间比较；来源 IP 必须在白名单（仅信任配置的反向代理的 `X-Forwarded-For`，其余使用连接 IP）；失败 403 并记审计。
2. 端点：`GET /admin/reports?status=`、`POST /admin/reports/{id}/resolve`、`POST /admin/ban`（`duration ∈ {1h,24h,7d,perm}`）、`DELETE /admin/ban/{id}`、`POST /admin/kick|mute|force_team|broadcast|map_change`（转发给指定 server）、`GET /admin/chat/{match_id}`（24 h 内）、`GET /admin/players/{id}`（战绩、违规计数、`suspicion` 摘要）。
3. 审计：每个 `/admin/*` 调用（含被拒）写 `audit_log`；写失败则整个操作失败（先写审计再执行）。
4. 封禁检查：按账号 id / 游客 token 指纹 / IP CIDR；到期自动失效；检查在签 ticket 与登录时都做。
5. 数据保留：后台任务清理 > 14 天的聊天片段与 > 30 天的 IP；审计 1 年。
### 接口与数据
- 管理员 token 通过环境变量 `ADMIN_TOKEN_HASH`（SHA-256 hex）配置；白名单 `ADMIN_IP_ALLOWLIST=cidr,cidr`；响应不回显 token。

## 不做什么
- 回放查看器/复盘 UI、统计异常评分（ARCH §15.3 的 z-score 模型）→ M3/M4；本任务只存 `replay_ref` 与只读摘要。
- 玩家侧举报按钮与结算页入口 → M2-043/M2-042；服务端处罚阶梯（TK/AFK）→ M2-018。
- 隐私政策/数据主体请求流程 → M4（H-074、H-082）。

## 验收标准
1. 管理操作均有审计行：对 8 个管理端点各调用 1 次（成功 + 被拒各 1），`audit_log` 行数 = 调用数（`cargo test -p gateway admin_audit`）；审计写入失败时操作不生效（注入故障测试）。
2. 非白名单 IP 拒绝：白名单外 IP 请求返回 403（100%）；伪造 `X-Forwarded-For` 在未配置可信代理时无效（测试）；错误 token 返回 401/403 且不泄漏差异（响应时间差异在 5 ms 内抽测 1 000 次）。
3. 封禁生效：被封账号/游客指纹/IP 段在换 ticket 时返回 403；1 h 封禁 1 h + 1 s 后可登录（用可注入时钟）；永久封禁无到期。
4. 举报限速：同玩家第 2 次 < 60 s 被拒、第 11 次/局被拒；落盘回放 ≤ 60 s 窗口且文件大小 ≤ 300 KB（30 Hz × 11 B × 60 s ≈ 20 KB，上限留余量）。
5. 保留策略：模拟时钟快进 15 天，聊天片段被清；快进 31 天，IP 被清；审计行保留。
6. 管理操作转发：kick/mute 在 headless 局中 1 s 内生效（C4 通知 + 连接断开/静音状态）。
7. 管理端点 P95 ≤ 50 ms（本地，SQLite，1 000 条报告）。

## 测试要求
- 单元测试：CIDR 匹配（IPv4/IPv6）、token 哈希比较、封禁时间计算、保留任务。
- 集成：gateway + server + headless 联测（举报→入库→管理员 kick）。
- 性能基准：无硬阈值，记录。
- 回归：M1-028/029 的 ticket、心跳测试保持通过。

## 性能预算
无（不涉及 tick 预算）；`server` 侧环形缓冲内存 ≤ 100 玩家 × 5 min × ~330 B/s ≈ 10 MB（估算，实测写入 STATUS）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工项：管理员白名单 IP、备份管理员与应急联系人（NEEDS_HUMAN H-065）；社区守则与处罚阶梯（H-075）。无这些输入时用占位配置，管理员 token 不得写入仓库。
- 聊天保留 GD（24 h）与 ARCH（举报关联 14 天）粒度不同，按 ARCH §19.4 + GD §16.2 同时实现，已在需求中说明；不一致处登记 PLAN_ISSUES。
- 保存的"约 20 KB/60 s"回放估算来自 ARCH §15.3 的 200 KB/10 min 数字线性折算，仅为估算。
