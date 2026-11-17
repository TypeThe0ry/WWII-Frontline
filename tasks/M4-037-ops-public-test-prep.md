# M4-037 运维：公开测试准备

## 元数据
- 里程碑：M4
- 工作流：ops
- 规模：M
- 依赖：M4-033, M4-034, M4-035
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §18.1; MILESTONES §6; MILESTONES §5.10; ARCHITECTURE §20; NEEDS_HUMAN H-087, H-090, H-094, H-095

## 背景
为 Go/No-Go（MILESTONES §5.10）准备对外材料与运营方案：**容量与成本方案、落地页与状态页、反馈入口、已知问题清单、发布公告草稿**。约束：
- 对外沟通**不得出现参考作品名称/商标/截图**（BRIEF §1；MILESTONES §6.1）；对外承诺必须与 ARCHITECTURE §20 的诚实限制一致（无硬光追、内存上限、Firefox/Safari 尽力支持、反作弊上限、单区域/崩溃不迁移、LLM 依赖外部 API 等）。
- 容量方案基于**压测数据**（M2-049 的 100 客户端压测、M4-038 的 60 min 三图浸泡）：每局 100 人，单局 ≈ 1.0–1.5 核等效，4 物理核装 2 局（估计，以实测为准）；流量：单局约 5.4 Mbps 典型 + 语音 ≈ 5.8 Mbps ⇒ 每局 ≈ 11 Mbps（§18.1）；**价格未核实，不写入固定金额，使用"按实时报价重算"并记录日期**。三档规模：精简 / 标准 / 极致（A6、H-087）。
- 公开测试时间与渠道由人决定（H-090）；建议固定时段集中人流（RISKS R-41）。
- 状态页/落地页内容仅静态页面，不收集个人数据（遵守 M4-035/M4-036）；捐赠入口仅外链，捐赠不换权益（H-086）。

## 具体内容
### 要创建/修改的文件
- `ops/public-test/CAPACITY_PLAN.md` —— 容量与成本方案（含公式、实测输入、三档规模、降级策略：限人数/限流/只开 1 局）。**运维产物，非规划 docs**。
- `ops/public-test/KNOWN_ISSUES.md` —— 已知问题清单（来自 PLAN_ISSUES、STATUS 的 partial 与诚实限制 §20，按严重级分组，对外友好措辞草稿）。
- `ops/public-test/ANNOUNCEMENT_DRAFT.md` —— 发布公告草稿（中英）；`ops/public-test/FAQ_DRAFT.md`（含"为什么浏览器游戏无法做到 PC 级反作弊"，H-095 参考）。
- `site/landing/`（静态落地页：Vite/纯 HTML，无第三方脚本）、`site/status/`（静态状态页：读取 gateway `/status.json`，无个人数据）。
- `crates/gateway/src/status.rs` —— `GET /status.json`（版本、在线局数、人数区间、维护提示；缓存 10 s；不泄露个人信息）。
- `tools/ops/ip-hygiene-scan.*` —— 对外文本扫描（禁用词表、`X-like` 描述、截图文件元数据检查），词表放 `data/ip_blocklist.txt`（不含在公开文档中）。
- `ops/public-test/GO_NO_GO_INPUTS.md` —— 给 M4-046 的清单素材索引。
### 要实现的行为
1. 容量方案：以 M2-049/M4-038 的实测输入（CPU、带宽、内存）推导每局资源与并发上限；给出"人数/局数 → 机器数 → 月度成本区间（占位，价格未核实）"的表；列出触发降级的阈值（对应 §18.3 告警）。
2. 状态页：静态 + `/status.json`；故障时显示维护横幅；不使用第三方分析/字体 CDN（CSP 兼容，M4-032）。
3. 反馈入口：落地页/状态页链接到项目邮箱与社区渠道（人类提供 H-095，缺则占位 `{{CONTACT}}`）。
4. 已知问题清单：自动汇总 STATUS/PLAN_ISSUES 中所有 `partial/blocked/未测` 项为对外措辞草稿，保留内部编号映射。
5. 对外文案自查：对全部草稿运行 `ip-hygiene-scan`，0 命中；并包含 ARCHITECTURE §20 的诚实限制要点（WebGPU/WebGL2、浏览器支持、反作弊局限、单区域）。
6. 与 M4-035 对齐：落地页链接隐私政策/ToS（法务定稿前显示"草稿"，不得对公众开放注册，见 H-082）。
### 接口与数据
- `GET /status.json`：`{"version":"…","matches":[{"map":"…","mode":"…","players_bucket":"60-80"}],"notice":null|"…"}`；人数以区间桶返回，避免精确泄露。

## 不做什么
- 不决定公开测试日期/渠道/规模（H-090/H-087，人类）；不购买域名/服务（M4-033 人工）。
- 不做市场营销素材（预告片、截图）——对外截图须避开参考作品并由人审（H-094）。
- 不做社区/客服运营。
- 不做法律最终文本（H-091）。

## 验收标准
1. **容量方案与 100 人压测数据对应**：`CAPACITY_PLAN.md` 引用 M2-049 与 M4-038 的实测数字（tick_ms P99、下行 kbps P50/P99、RSS、CPU 核数），表内每个输入带来源路径；缺任一输入则标 `未测` 并 partial。
2. **公告通过 BRIEF §1 知识产权自查**：`ip-hygiene-scan` 对 `ops/public-test/**` 与 `site/**` 命中数 = 0；人工复核清单（H-094）逐项勾选或标 `待人`。
3. 状态页：`/status.json` P95 响应 ≤ 100 ms；不含个人数据字段（schema 断言）；状态页零第三方请求（Playwright 断言所有请求同源）。
4. 诚实限制：公告/FAQ 含 §20 要点至少 8 项（脚本检查关键词表：WebGPU 回退、Firefox/Safari 尽力、浏览器反作弊上限、单区域、崩溃不续局、内存要求、语音不录制、LLM 可选/外部依赖），缺项数 = 0。
5. 已知问题清单覆盖 STATUS 中全部 `partial/blocked` 条目（脚本对比，漏列数 = 0）。
6. 落地页 Lighthouse（或等价）性能 ≥ 90（桌面，记录），页面总体积 ≤ 500 KB（不含主游戏包）。

## 测试要求
- 单元测试：`status.rs` 序列化与桶化；`ip-hygiene-scan` 词表匹配（含大小写/空格/连字符变体）。
- 集成/无头测试：Playwright 同源断言；`/status.json` 在对局启停时变化。
- 性能基准：`/status.json` 压测 100 rps 无错误。
- 回归：词表扫描纳入 CI（对 `ops/public-test/**`、`site/**`）。

## 性能预算
- gateway `/status.json` 不得影响 §18.3 gateway 延迟指标；页面体积上限见上。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 价格/带宽费用**未核实**：只给公式与区间占位，不得写入未经查证的具体金额。
- 依赖 H-087（规模/DDoS）、H-090（时间与渠道）、H-094（对外沟通卫生审查）、H-095（反馈/申诉渠道）、H-085（运营主体/Impressum）；缺失则文案用占位符并 `partial`。
- 在 M4-045 封闭测试后须更新已知问题清单；本任务只产出首版。
