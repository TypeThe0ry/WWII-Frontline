# M1-026 server：部署/小队处理与可靠事件

## 元数据
- 里程碑：M1
- 工作流：server
- 规模：L
- 前置依赖：M1-021, M1-019, M1-010
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §7.3; ARCHITECTURE §6.8.1; ARCHITECTURE §6.8.3; ARCHITECTURE §6.6.1; ARCHITECTURE §19.1; ARCHITECTURE §19.3; ARCHITECTURE §15.1

## 背景
把 sim 的规则（M1-019/021）与协议（M1-010）接到 C0/C4 通道：处理部署与小队请求、广播可靠事件。游戏规则一律在 `sim`，**`server` 只做路由、校验入口、限速与分发**（ARCHITECTURE §4.3.4）。
- **C0 control**（双向 bidi stream #0，可靠有序）：`DeployRequest/DeployResult`、`SquadOp`、`Chat`、`MarkerPlace`、`Report`。部署流程（§7.3）：校验 → 通过则创建实体、`spawn_tick = now + 倒计时`、回 `DeployResult(ok, spawn_tick)`；拒绝回 `DeployResult(ok=false, reason)`；到 `spawn_tick` 创建角色并发初始关键帧快照与 `PlayerRoster` 增量。
- **C4 reliable-events**（uni stream #1，可靠有序）：`Kill/Death/Revive/ScoreDelta/ObjectiveChange/SpawnPointState/PlayerRoster/Notice/MatchPhase`。丢了会造成状态永久错误的事件（击杀、重生确认、弹药补给、据点占领）必须走 C4；只影响一次表现的走 C2.events。
- **WebSocket 兜底**：同一 `Message` 枚举（`[channel:u8][len:u16][payload]`）；C4 在 TCP 上天然可靠。
- 速率限制（§15.1/§19.3）：聊天每人每 8 s 最多 5 条（GD §16.2，已裁定）、重复内容检测；部署 1/s；举报 1/min、10/局；超限消息**丢弃 + 惩罚计数**。聊天：≤ 200 字符，NFKC 规范化、拒绝控制字符与双向覆盖字符（§19.1）；UI 层纯文本渲染。
- 连接：每 IP 并发 ≤ 4；握手超时 5 s；缓冲 ≤ 64 KB（M0 已有）。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/c0/mod.rs` —— 消息分发 `handle_c0(session, msg)`
- `crates/server/src/c0/deploy.rs` —— 调用 `sim::validate_deploy`、预占/创建出生、回 `DeployResult`
- `crates/server/src/c0/squad.rs` —— 调用 `sim::apply_squad_op`、广播 `PlayerRoster`
- `crates/server/src/c0/chat.rs` —— 频道（all/team/squad）路由、字符过滤、重复检测
- `crates/server/src/c0/marker.rs` —— `MarkerPlace`（M1：仅小队队长的 Move/Attack/Defend/Hold，权限由 sim 校验）
- `crates/server/src/c0/report.rs` —— 入库为内存队列 + 日志（M2 做复核队列）
- `crates/server/src/c4/broadcast.rs` —— `ReliableEventBus`：按队伍/小队/个人定向广播；顺序保证；背压（每连接缓冲 64 KB，超限断开并记录）
- `crates/server/src/ratelimit.rs` —— 令牌桶（按玩家+消息类型），指标 `c0_rate_limited_total{type}`
- `crates/server/src/text.rs` —— NFKC、字符白名单/黑名单、长度、重复检测
- `crates/server/tests/c0_*.rs`、`c4_*.rs`；`crates/headless-client/src/scenarios/c4_loss.rs`
### 要实现的行为
1. 每条 C0 消息先过 `ratelimit`，再过格式校验，再交 `sim`；`sim` 返回的 `SimEvent` 转为 C4/C2 事件。
2. `DeployRequest` 流程：成功后立即 `DeployResult{ok, spawn_tick}`；失败返回 `reason`（`DeployError` → u8 码表固定在 protocol）；`deploy_requests_total{result}` 计数。
3. 倒计时期间的实体：`spawn_tick` 之前不在世界；预加载提示由客户端据 `spawn_point` 做（ARCHITECTURE §9.4）。
4. `SquadOp`：成功后广播 `PlayerRoster` 增量给全队；队长变更发 `Notice{code,args}`。
5. `Chat`：按频道路由（all 仅本局、team、squad）；过滤失败/超长 → 丢弃并回 `Notice`；M1 无敏感词过滤。
6. C4：每个 `SimEvent`（Kill/Death/Revive/ObjectiveChange/SpawnPointState/MatchPhase…）转为 `ServerEventC4` 并按受众发送；`PlayerRoster` 在新玩家加入时发全量，之后增量。
7. 断线重连（GD §2.8，90 s 内）：重连时重发全量 `PlayerRoster`、当前 `MatchPhase`、`ObjectiveChange` 状态快照（C5 bulk 暂用 C4）。
8. 指标：`deploy_requests_total{result}`、`c4_events_total{type}`、`c0_rate_limited_total{type}`、`chat_rejected_total{reason}`。
### 接口与数据
```rust
pub struct Audience { pub team: Option<TeamId>, pub squad: Option<SquadId>, pub player: Option<PlayerId> }
impl ReliableEventBus { pub fn publish(&self, aud: Audience, ev: ServerEventC4); }
pub fn sanitize_chat(raw:&str) -> Result<String, ChatReject>;
```

## 不做什么
- 部署/小队规则本身：M1-019/M1-021（在 sim）；
- 协议定义：M1-010；
- 敏感词过滤（词表待人定，H-系列）、管理员禁言/封禁：M2；
- 举报复核队列与统计：M2；
- 语音频道与 LiveKit 订阅管理：M2；
- 客户端消费 C4/C0：M1-040/M1-056。

## 验收标准
1. 速率超限消息被丢弃并计数：聊天 8 s 内第 6 条丢弃、部署 2 次/秒第 2 次丢弃、举报 1/min 超限（单测 + `/metrics` 断言 `c0_rate_limited_total` 递增）。
2. **C4 事件丢包 5% 下仍全部送达**（headless，WT 路径 netem 5% 丢包 10 min，每秒 10 个 C4 事件；接收端序号校验无缺失、无乱序；WS 路径同样）。
3. 聊天过滤：201 字符被拒、含 U+202E（RLO）/控制字符被拒、NFKC 全角转半角后入库（单测 ≥ 10 例）。
4. 部署：合法请求回 `DeployResult(ok=true, spawn_tick)` 且 `spawn_tick` 与 `sim` 的 `ready_time` 一致（±1 tick）；≥ 10 种非法请求（满员/名额/封锁点等）回 `ok=false` 且 `reason` 码与 sim 错误一一对应（表驱动测试）。
5. 小队：建队/加入/踢人/移交后，全队收到正确的 `PlayerRoster` 增量，与 sim 状态一致（20 客户端场景比对，不一致数 = 0）。
6. 背压：慢客户端缓冲 > 64 KB 时被断开并计数，不影响其他客户端 tick（`server_tick_ms` P99 ≤ 28 ms 保持）。
7. 重连 90 s 内恢复：重连后 `PlayerRoster/MatchPhase/ObjectiveChange` 全量到达且角色状态保持（headless 场景）。
8. 服务器规则未泄漏：代码审查脚本 `scripts/check-no-rules-in-server.sh` 搜索 `server/` 中对规则常量的硬编码（`15.0`、`150`、`50` 等魔数白名单外）为 0（细化检查，登记 PLAN_ISSUES 如需豁免）。

## 测试要求
- 单元测试：令牌桶边界、文本过滤、受众路由、事件映射表（`SimEvent → C4`）全覆盖。
- 集成/无头：`c4_loss`（5% 丢包）、20 客户端部署/小队混合脚本、重连场景。
- 性能基准：C0 处理 ≤ 20 µs/条；C4 广播 20 客户端 ≤ 0.1 ms/事件批。
- 回归：事件映射表快照。

## 性能预算
- 并入 ARCHITECTURE §6.10 阶段 1（输入摄取 0.8 ms）与阶段 10（序列化发送 1.5 ms）；可靠事件均值 ≈ 300 B/s/客户端（§6.9.2）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- ARCHITECTURE §7.3 流程图含"预扣 Manpower"；M1 无战略资源，该步为空操作（M2 接入）。
- "C4 5% 丢包全部送达"在 WT 上依赖 QUIC 可靠流，实质测试服务器对 uni stream 背压与重连处理，不是协议层可靠性。
- 规模 L；若超出，将聊天/标记/举报拆为 `M1-026b`。
