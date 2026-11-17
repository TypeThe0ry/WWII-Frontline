# M2-025 语音：服务器订阅集管理

## 元数据
- 里程碑：M2
- 工作流：voice
- 规模：L
- 依赖：M2-024, M1-026
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §14.2; GAMEDESIGN §16.1; ARCHITECTURE §7.4; ARCHITECTURE §18.3; ARCHITECTURE §14.1

## 背景
每个真人玩家的客户端以 `autoSubscribe=false` 连接 LiveKit，**订阅哪些轨道由对局服务器决定**（ARCH §14.2）。规则（GD §16.1 与 ARCH §14.1 频道表）：
- 频道：`squad`（同小队 ≤ 6，无空间化，PTT/VOX）、`command`（Commander + 各 Officer 小队长，GD：Tank Commander 与 Recon 队长仅可听）、`crew`（同载具乘员 ≤ 5）、`prox`（任意玩家距离 ≤ 50 m，**含敌方**，空间化）、`all-dead`（可选）。敌方的 `squad/command/crew` **永不订阅**。同时收听上限 **8 路**。
- 每 **0.5 s（2 Hz）** 对每个真人 `i`：`candidates = squad ∪ (command if i 是队长/Commander) ∪ crew ∪ {j: dist(i,j) ≤ 50 m}`；优先级 `crew(1) > squad(2) > command(3) > prox 按距离升序(4)`；取前 8；**迟滞**：prox 进入 50 m、退出 60 m；集合变动持续 ≥ 1 个周期才下发；只发差分 `(add, remove)`。
- 死亡玩家：停止发布 `prox`（把其 prox 轨从他人订阅集排除，`squad` 保留）；KIA 者可听 Squad 不可说；倒地者仅走 Proximity（−12 dB、低通，客户端处理）。
- 每人每秒平均变化 ≈ 0.2 次，100 人 ≈ 20 次/s，批量合并为每 0.5 s 至多 1 次/玩家的 API 调用（≤ 200 次/s 上限；LiveKit 具体限速未核实）。
- 目标（MILESTONES E2-5、T8）：订阅更新延迟 P95 < 300 ms；100 玩家下 LiveKit API 错误率 < 1%；LiveKit 不可用时游戏不受影响。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/comms/voice_sets.rs` —— **纯函数** `compute_subscriptions(view:&dyn WorldView, prev:&VoiceState) -> VoiceDiff`（频道成员、优先级、迟滞、上限 8；规则在 sim，便于单测与 balance-sim 复用，ARCH §4.3.4）
- `crates/server/src/voice/manager.rs` —— 2 Hz 定时任务（tokio 任务，不占 sim 线程）：从 sim 取快照视图 → 调 `compute_subscriptions` → 批量调用 LiveKit Server API
- `crates/server/src/voice/livekit_api.rs` —— `reqwest` 客户端：`UpdateSubscriptions`、`ListParticipants`、`RemoveParticipant`；重试（指数退避，上限 5 s）；熔断（连续失败 N 次后暂停并记指标，不影响 tick）
- `crates/server/src/voice/tracks.rs` —— `player_id → mic track sid` 映射（来源：LiveKit webhook `track_published` 或 `ListParticipants` 轮询，二选一，在任务内定并写入备注）
- `crates/server/src/metrics.rs` —— `livekit_subscription_updates_total`、`livekit_api_errors_total`、`voice_sub_latency_ms`（直方图）
### 要实现的行为
1. 候选/优先级/迟滞/上限完全按上述规则实现；输出稳定排序（同距离按 `player_id`）。
2. 差分去抖：同一变更需连续 2 个周期（1 s）存在才下发；加入 prox 立即需要的情况（敌人走近说话）也遵守迟滞但 `crew/squad` 变更（换队、上下车）**立即**下发。
3. 批处理：一个玩家一次调用携带全部 add/remove；全局并发限制（默认 16）；队列积压 > 200 时丢弃最旧的 prox 变更（保 squad/crew）。
4. 死亡/换队/离队/重连钩子：玩家状态变化事件驱动立即重算。
5. 降级：LiveKit 不可用 → 管理器静默降级，记 `livekit_api_errors_total`，游戏照常；恢复后全量重同步一次。
6. 观测：每次下发记录从"sim 检测到变更"到"API 返回 200"的耗时；客户端到达侧延迟由测试脚本测。
### 接口与数据
- `VoiceDiff{ player:PlayerId, add:SmallVec<[PlayerId;8]>, remove:SmallVec<[PlayerId;8]> }`；`VoiceState` 每玩家当前订阅集合（上限 8）。

## 不做什么
- token/部署（M2-024）；客户端轨道接收、空间化与 PTT（M2-026）；语音相关 UI（M2-040）。
- 转写、录制、LiveKit Cloud。
- 敌方 prox 之外的任何敌方语音；Commander 席位功能性验收（M3；规则先实现并单测）。

## 验收标准
1. 订阅更新延迟 P95 < 300 ms（T8，**实测**）：100 个测试参与者（含 20 个真实 `livekit-client` 订阅者做到达侧测量），1 200 次状态变化，输出 `reports/m2-voice-latency.json`；无 LiveKit 容量时记 `未测`/`partial`。
2. 100 玩家 30 min 合成走位下 `livekit_api_errors_total / livekit_subscription_updates_total < 1%`。
3. `compute_subscriptions` 单测：prox 49.9 m 进入、60.1 m 退出迟滞；优先级（crew > squad > command > prox）；上限 8 的截断；敌方 squad/command/crew 不进集合；死亡者不出现在他人 prox；全部 ≥ 15 个用例通过。
4. 订阅 API 调用速率峰值 ≤ 200 次/s（100 玩家，指标 `rate()`）。
5. LiveKit 容器停止后：游戏 `tick_ms` P99 不变（对照组差 ≤ 1 ms），30 s 后重启，订阅在 5 s 内重新同步。
6. 管理器 CPU（server 进程）≤ 2% 单核；不在 sim 线程执行任何网络 I/O（代码审查 + `rg` 检查 `reqwest` 不出现在 `sim` 线程路径）。

## 测试要求
- 单元测试：`cargo test -p sim voice_sets`（纯函数，表驱动 + proptest：集合大小 ≤ 8、无自订阅、无敌方私有频道）。
- 集成：`cargo test -p server voice_manager`（mock LiveKit HTTP 服务）+ 对真实容器的集成（M2-024 的 compose）。
- 性能基准：`voice_sets` 100 玩家一次重算 ≤ 0.3 ms（`cargo bench -p sim voice_sets`，运行在 server 的异步线程，不计入 sim tick）。
- 回归：M1 server 不启用语音时行为不变（feature flag `voice` 关闭）。

## 性能预算
语音订阅计算不得占用 sim 线程（ARCH §6.10 线程模型：`bot-llm`/IO 在异步任务）；订阅更新 P95 < 300 ms（MILESTONES E2-5）；API 错误率 < 1%（ARCH §18.3 告警线）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 100 人容量与延迟的真实数字需要 LiveKit 主机（NEEDS_HUMAN H-018、H-067 批准压测机）；缺失时延迟项 `未测` → 任务 `partial`，PLAN_ISSUES 登记 T8 风险。
- LiveKit Server API 的精确方法名/限速以所钉版本文档为准（规划阶段未核实）。
- 若 P95 > 300 ms：优先合并批量、缩短去抖，不得改 8 路/50 m 规则（改动交 Fable）。
