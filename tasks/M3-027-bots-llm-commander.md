# M3-027 bots：LLM 指挥官（可选 feature）

## 元数据
- 里程碑：M3
- 工作流：bots
- 规模：L
- 依赖：M3-026
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §16.4; ARCHITECTURE §16.1; ARCHITECTURE §6.10; ARCHITECTURE §19.1; GAMEDESIGN §20.1; GAMEDESIGN §20.4; BRIEF R4

## 背景
可选功能：每队 1 个 LLM 指挥官（默认**关闭**，`feature = "llm"`）。R4：决策间隔 **60 s**，每局成本上限 **0.5 美元**，价格**未核实、不得写死**，实现前按官方价格重算并写入配置。输入是结构化战况 JSON，输出是**受限动作枚举**，所有动作经与真人 Commander 同等的校验（资源/冷却/权限，M3-006），LLM 输出绝不直接执行。`RuleCommander`（M3-026）常驻热备，任何异常立即接管。
[人工：API key 与官方价格复核] —— 需人类提供 Anthropic API key（服务器环境变量/密钥存储，不进日志，对应 NEEDS_HUMAN H-019）与启用/预算决定（H-073）；价格由人类/执行者在实现时从官方来源核实并填入配置。缺失时本任务以**模拟 LLM 客户端（mock）**完成全部可测内容，真实调用项标 `未测`，状态至多 `partial`/`blocked`（README §6）。
接口（ARCHITECTURE §16.4）：
```rust
pub trait CommanderBrain { fn decide(&mut self, s:&Situation, b:&mut CostBudget) -> Result<Vec<CommanderAction>, BrainError>; }
pub struct LlmCommander { client: LlmClient, model: String, interval_s: u32 /*60*/, fallback: RuleCommander }
```
关键约束：超时 8 s；成本检查——调用前按最坏估算，调用后按实际用量记账；达 90% 间隔拉长到 90 s，达 100% 立即回退；输入 ≤ 2 000 token；输出动作数 ≤ 6；默认模型 `claude-haiku-5-5`（可配置）；提示词不含玩家聊天（防提示注入）；`llm_fallback_total` 指标。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/commander/llm/mod.rs`（`#[cfg(feature="llm")]`）—— `LlmCommander`：异步（tokio 任务）、60 s 间隔、超时 8 s 沿用上一决策并启动回退。
- `crates/bots/src/commander/llm/client.rs` —— `LlmClient` trait + HTTP 实现（`reqwest`，仅 `llm` 特性）+ `MockLlmClient`（脚本化：正常/超时/非法 JSON/超限/越权动作）。
- `crates/bots/src/commander/llm/schema.rs` —— 动作 JSON Schema（≤ 6 条）：`assign_squad_objective`、`use_ability{ability,pos,dir}`、`request_supply{kind,at}`、`set_priority{objective,level}`、（可选）`call_vehicle`、`fire_mission`；严格解析，未知字段/越界值丢弃并记录。
- `crates/bots/src/commander/llm/budget.rs` —— `CostBudget{price_in, price_out, limit_usd: 0.5, spent, calls}`：价格从配置文件读取（`config/llm_prices.toml`，**仓库内不预填价格**，缺省为"未配置→LLM 禁用"）；调用前最坏估算（`max_in_tokens×price_in + max_out_tokens×price_out`）若超余额则拒绝调用；调用后按 API 返回的实际 token 记账；90% 降频、100% 回退。
- `crates/bots/src/commander/llm/prompt.rs` —— 战况序列化（复用 M3-026 的 `Situation`，仅本队可见信息，≤ 2 000 token 截断策略）；系统提示固定；不含聊天文本。
- `crates/bots/src/commander/llm/metrics.rs` —— `latency_ms`、`tokens_in/out`、`cost_usd`、`actions_accepted/rejected`、`llm_fallback_total{reason}`。
- `crates/server/src/config.rs` —— 开关 `--llm-commander=off|team_a|team_b|both`（默认 off）；API key 经环境变量，日志脱敏。
- `crates/bots/tests/llm_commander.rs`。
### 要实现的行为
1. **默认关闭**：未启用 feature/配置时不发起任何网络请求；价格未配置也不启用。
2. 每 60 s 一次决策；异步不占 sim 线程（ARCHITECTURE §6.10 阶段 6：LLM 调用异步）。
3. 回退触发（均增加 `llm_fallback_total{reason}`）：超时（> 8 s）、非法 JSON、schema 校验失败、成本超限、API 不可用/HTTP 错误、所有动作被校验层拒绝；回退后 `RuleCommander` 立即接管，后续是否重试 LLM 按 60 s 周期并带指数退避（上限 300 s）。
4. 动作经 `validate_order`，被拒动作计数；LLM **0 次绕过校验**。
5. 成本记账与实际 token 一致：`spent = Σ(tokens_in×price_in + tokens_out×price_out)`，误差 0（浮点容差 1e-9）。
6. 安全：API key 仅服务器环境变量；日志/指标不含 key；提示词不含玩家聊天/昵称。
### 接口与数据
- `config/llm_prices.toml`（示例结构，数值留空由人类填写）：`model="..."`、`price_in_per_mtok=<待填>`、`price_out_per_mtok=<待填>`、`verified_on="<日期>"`、`source="<官方链接>"`。

## 不做什么
- 规则指挥官本体 → M2-028/M3-026；LLM 对话/语音/聊天回复；多轮记忆；微调；
- 价格/模型的固定（文档不固定价格，R4）；真实 key 的获取 → 人类（H-019/H-073）；
- 默认启用；公开测试的隐私披露（H-082，人类）。

## 验收标准
1. **注入非法 JSON/超时/超限均回退且 `llm_fallback_total` 增加**：`MockLlmClient` 4 种故障各注入 5 次（共 20 次），每次均在同一决策周期内由 `RuleCommander` 接管，`llm_fallback_total{reason}` 分别 +5（`cargo test -p bots --features llm llm_fallback`）。
2. **成本记账与实际 token 一致**：Mock 返回的 token 数序列（20 次）与 `CostBudget.spent` 逐次核对，误差 ≤ 1e-9 美元；真实 API 下（若有 key）实测 5 次，记账与响应 `usage` 字段一致（无 key 则该项记 `未测`）。
3. 成本上限：设置 `limit=0.5` 与模拟价格，当累计达 90% 时间隔变为 90 s，达 100% 立即回退并不再调用（单测）。
4. 间隔：10 次调用之间间隔 = 60 s ±1 tick（`tokio::time::pause` 虚拟时间）；超时 8 s ±0.1 s 触发回退。
5. LLM 输出 0 次绕过校验：向 Mock 注入 50 条越权动作（非 Commander 权限、资源不足、冷却中、非法坐标），校验层 100% 拒绝，`illegal_cmd_leaked_total` = 0。
6. 默认关闭：不带 feature 与带 feature 但 off 的构建，抓包/mock 计数确认 HTTP 请求数 = 0；价格未配置时 `LlmCommander::new` 返回 `Disabled`。
7. 输入 ≤ 2 000 token（以近似计数器测量，100 个战况快照，超限 0 个）；输出动作数 > 6 时截断为 6。
8. 日志审计：grep 构建日志/指标输出，API key 字符串出现次数 = 0。

## 测试要求
- 单元测试：schema 解析与拒绝、预算最坏估算、90%/100% 阈值、退避、提示词无聊天字段。
- 集成/无头：服务器 + `headless-client` 跑 10 分钟局，Mock 周期性注入故障，`balance-sim` 中 LLM 与 Rule 切换不崩溃；真实 API 端到端演示（需 key）录入 STATUS（MILESTONES 演示脚本 6）。
- 性能基准：`LlmCommander::decide` 异步入口不阻塞 sim 线程：tick 抖动 P99 增量 ≤ 0.2 ms（`tick_ms` 直方图对比开/关）。
- 回归：协议/行为无变化；feature 关闭构建体积/依赖不变（`cargo tree` 检查无 `reqwest` 引入默认构建）。

## 性能预算
ARCHITECTURE §6.10 阶段 6（Bot 思考 3.0 ms）：LLM 调用异步，sim 线程仅做动作校验（≤ 0.05 ms）；每局成本 ≤ 0.5 美元（价格走配置，E3-9）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- NEEDS_HUMAN：H-019（Anthropic API key 与支出上限）、H-073（是否启用与预算；ARCHITECTURE §16.4 旧文如仍写 45 s 以 R4 的 60 s 为准，已在 RISKS R-51 登记）。价格复核前**不得**把任何价格写进仓库默认配置。
- 成本最坏估算会使"理论上 120 次调用"在价格较高时提前触顶；这是预期行为（回退）。
- 隐私：战况 JSON 外发第三方，不含个人数据；仍需隐私政策披露（人类，H-082）。
- 规模 L；若超限把 HTTP 客户端真实实现拆为 `M3-027b`，Mock 与校验先行。
