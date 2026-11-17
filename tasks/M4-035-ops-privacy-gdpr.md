# M4-035 运维：隐私与合规（数据导出/删除）

## 元数据
- 里程碑：M4
- 工作流：ops
- 规模：M
- 依赖：M2-022（战绩/设置/OAuth）, M4-033
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §19.4; ARCHITECTURE §19.2; ARCHITECTURE §18.4; NEEDS_HUMAN H-074, H-082, H-083, H-085

## 背景
ARCHITECTURE §19.4 的数据最小化设计：
- 语音**不录制、不存档**（SFU 仅转发；LiveKit 关闭 egress/录制；服务器日志无音频）；举报无语音证据。
- 聊天：内存保留至局终；仅举报关联的前后 **20 条** 保留 **14 天**，之后删除。
- 个人数据：昵称、设置、基础战绩、可选 OAuth 的 `provider_id`（**哈希**）；不存邮箱（除非 OAuth 必须，待人定）。IP 仅短期（**≤ 30 天**）用于封禁/滥用，之后哈希或删除。审计日志（封禁/管理员操作）保留 1 年（§18.4）。
- 合规：面向欧洲区需 GDPR 流程（**数据导出/删除端点**）；隐私政策与 Cookie/本地存储说明【待人定与法务审阅，本文不构成法律意见】。
- 客户端遥测：匿名、默认关闭或首次明确同意（M4-036 实现）。
- `localStorage` 存游客 session token（JWT，TTL 24 h，§19.2）：需在隐私说明里披露；ePrivacy/Cookie 同意是否需要**未核实**（H-082）。
法务审阅与最终政策文本是**人类职责**（H-082/H-074/H-085）；执行者出模板草稿与技术实现。

## 具体内容
### 要创建/修改的文件
- `crates/gateway/src/privacy/{export.rs,delete.rs,retention.rs}` —— 导出/删除端点与保留期清理作业。
- `crates/gateway/migrations/*_privacy.sql` —— 表结构支持（`ip_log`、`report_chat_snapshot` 的 `expires_at`、软删除标记）。
- `data/privacy/retention.json` —— 保留期配置：`report_chat_days:14`、`ip_days:30`、`audit_days:365`（`initial`，与 H-074 一致）。
- `client/src/ui/privacy/{PrivacyPage,DataRequest,LocalStorageNotice}.tsx`、`client/src/i18n/{en,zh}/privacy.json` —— 设置内"隐私"分类（UI_SPEC §4.16 隐私项）：导出/删除按钮、本地存储说明、遥测开关入口（M4-036）。
- `legal/templates/{privacy-policy.md,terms-of-service.md,local-storage-notice.md}` —— **草稿模板**，全文标注"草稿，待法务审阅"，不含虚构的公司名/地址（使用占位符）；列出收集的数据、目的、保留期、第三方处理方（Cloudflare、服务器厂商、OAuth 提供方、Anthropic API（若启用 H-073））。
- `tests/e2e/privacy_flow.spec.ts`。
### 要实现的行为
1. **导出端点**：`POST /api/me/export`（需鉴权）→ 异步生成 JSON（昵称、设置、战绩、OAuth provider 标识哈希、近期会话列表）→ 一次性下载链接（TTL 24 h，单次）。游客用 `guest_id` 凭 session token 导出。
2. **删除端点**：`POST /api/me/delete`（二次确认令牌）→ 删除昵称/设置/战绩/OAuth 关联/封禁以外的个人数据；**封禁与审计记录因滥用防范保留的最小字段（哈希）须在政策模板中披露**；完成后使 session token 失效。
3. **保留期作业**：每日 cron：删除到期的举报聊天快照（14 天）、到期 IP（30 天，或改存盐哈希）；写运行记录（条数、时间）。
4. 请求受理：数据主体请求另可通过项目邮箱（H-013/H-074），端点只是技术路径；响应时限以律师意见为准（**未核实**）。
5. 语音/聊天核验：自动化测试断言 LiveKit 配置无 egress、gateway/服务器日志样本无聊天全文与音频。
6. 本地存储披露：设置页列出 `localStorage` 键用途（session token、设置、教程进度、筛选），提供"清除本地数据"按钮。
### 接口与数据
- `GET /api/me/export/{job_id}` → 下载；`DELETE /api/me`；保留期配置 schema 校验；所有操作写审计日志（不含个人内容）。

## 不做什么
- 不起草成"最终法律文本"（人工/律师，H-082/H-091）；不做年龄分级/年龄门（H-083，仅预留声明页位置）。
- 不做遥测上报与同意（M4-036）。
- 不做 Cookie 横幅逻辑，除非 H-082 判定需要（届时另立任务/PLAN_ISSUES）。
- 不做数据跨境传输评估（人工）。

## 验收标准
1. **数据导出/删除端点端到端通过**：e2e 创建游客与 OAuth 模拟用户 → 产生战绩/设置 → 导出（JSON schema 校验通过，字段齐全）→ 删除 → 再次查询返回 404/空；导出与删除各 100% 成功（各 10 次）。
2. **保留期清理作业运行记录**：把保留期缩短（测试配置 1 min）并造数据，作业运行后过期数据条数 = 0，未过期保持不变；日志记录删除条数；生产配置值仍为 14/30/365 天（配置校验单测）。
3. 删除后无残留：对已删除用户做全库字段搜索（用户标识/昵称），残留行数 = 0（封禁哈希除外，列出）。
4. 语音与聊天：LiveKit 配置 `egress` 关闭（配置断言）；日志采样 10 000 行无聊天全文/音频字段（grep 断言）。
5. 导出文件不含他人个人数据（构造含多人战绩的对局，断言 0 泄漏）。
6. 隐私设置页 en/zh 完整；清除本地数据按钮清除全部列出的键（e2e）。
7. 政策模板含"草稿，待法务审阅"标记，且不含虚构公司/地址（脚本检查占位符 `{{…}}` 存在）。

## 测试要求
- 单元测试：保留期计算、导出序列化、删除级联、令牌 TTL/单次使用。
- 集成/无头测试：`privacy_flow.spec.ts`；数据库迁移测试。
- 性能基准：导出 ≤ 5 s（10 000 条战绩）；清理作业 ≤ 60 s（10^6 行）。
- 回归：迁移与作业进入 CI（`cargo test -p gateway privacy`）。

## 性能预算
- gateway 请求延迟/错误率（§18.3）不得回归；清理作业不得影响对局服务器（独立进程/低优先级）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **人工：法务审阅**（H-082 隐私政策/ToS、H-074 保留期与响应流程、H-085 运营主体、H-083 年龄）。H-082 无默认：无政策不得向公众开放注册/聊天/语音；执行者在 STATUS 标注"技术就绪、待法务"。
- 本任务不构成法律意见；模板仅为起草材料。
- 保留期数值取自 ARCHITECTURE §19.4，H-074 默认"同意设计中的保留期"。
