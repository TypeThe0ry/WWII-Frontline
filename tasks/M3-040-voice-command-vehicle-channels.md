# M3-040 语音：指挥与车组频道

## 元数据
- 里程碑：M3
- 工作流：voice
- 规模：M
- 前置依赖：M2-025, M3-006, M3-016
- 素材需求：无；素材状态：n.a.
- 引用文档：ARCHITECTURE §14.1; ARCHITECTURE §14.2; ARCHITECTURE §14.3; ARCHITECTURE §7.4; GAMEDESIGN §16.1; GAMEDESIGN §14.1

## 背景
M2-025 建立了 LiveKit 房间、gateway token、服务器订阅集合计算（squad + prox）。本任务加入 `command` 与 `crew` 频道（ARCH §14.1 表）。数字与规则：
- 订阅集合每 0.5 s（2 Hz）计算：`candidates(i) = squad(i) ∪ (command 若 i 为队长/Commander) ∪ crew(i) ∪ prox(≤ 50 m)`；优先级 crew(1) > squad(2) > command(3) > prox 按距离升序(4)；取前 **8**；prox 迟滞：进入 50 m、退出 60 m；集合变动持续 ≥ 1 个周期才下发；只发 `(add, remove)` 差分；每玩家每 0.5 s 至多 1 次 API 调用（ARCH §14.2）。
- Command 频道（GD §16.1）：成员（可听可说）= Commander + Officer（Infantry 小队长且兵种为 Officer）+ Spotter（侦察组长）+ Tank Commander【待调参】。
- Vehicle/crew 频道：同一载具内所有座位（含乘客）；出车后自动退出；不空间化、轻混响。
- 阵营：敌方的 squad/command/crew 永不订阅；prox 含敌我（ARCH §14.2）。
- 倒地者仅走 Proximity（受限）；KIA 者可听 Squad 不可说（GD §16.1）。
已裁定：以 GD §16.1 为准（Commander + Officer + Spotter（侦察组长）+ Tank Commander 可听可说），ARCH §14.1 已同步修订。
规则只在 `crates/sim`（ARCH §4.3.4）：频道成员判定放纯函数，server 只负责调用 LiveKit API 应用差分。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/voice_rules.rs` —— 纯函数 `fn channel_members(world: &WorldView, player: PlayerId) -> ChannelSets`（`squad/command_speak/command_listen/crew/prox`），无 I/O、无时钟。沿用 M2-025 已有位置（若已在 sim，则扩展；若在 server，则将判定迁入 sim 并保留适配）。
- `crates/server/src/voice/subscriptions.rs`（改）—— 合并 command/crew 候选，应用优先级与 8 路上限、迟滞与差分；`speak` 权限（canPublish 的频道语义）由客户端 PTT 键与服务器订阅共同保证。
- `crates/server/src/voice/livekit_api.rs`（改）—— 批量 `UpdateSubscriptions`（合并为每玩家 ≤ 1 次/0.5 s）。
- `client/src/net/voice/channels.ts`（改）—— 频道状态 `channelStatus`；`N` 键 = 指挥 PTT，`V`/`B` 沿用；载具内按 `V` 即车组；上下车自动切换。
- `client/src/ui/voice/ChannelIndicator.tsx`（改）—— 显示可用频道（不可用则灰置并说明）。
- `crates/server/tests/voice_channels.rs` —— 使用 mock LiveKit 的集成测试。
### 要实现的行为
1. 上车 → 下一个 0.5 s 周期把同车乘员加入 `crew` 并升到最高优先级；下车 → 自动移除（≥ 1 个周期稳定后下发）。
2. Command：Commander、所有 Officer、Spotter（侦察组长）、Tank Commander 互相可听可说；其他兵种无该频道。
3. 8 路上限：候选 > 8 时，按优先级与距离截断；被截断的 prox 不下发。
4. 死亡/倒地：死亡者撤销 prox 发布；KIA 保留 squad 听权。
5. 敌我隔离：单测断言任意 `(i, j)` 若 team 不同，则 `j ∈ squad/command/crew(i)` 恒为假。
6. 客户端：无麦克风/LiveKit 不可用时只禁用语音（ARCH §14.5），文字聊天不受影响。
### 接口与数据
```rust
pub struct ChannelSets { pub squad: Vec<PlayerId>, pub command: Vec<PlayerId>, pub crew: Vec<PlayerId>, pub prox: Vec<(PlayerId, f32)> }
pub fn subscribe_set(sets: &ChannelSets, cap: usize /*8*/) -> Vec<PlayerId>;
```

## 不做什么
- LiveKit 部署、token、基础订阅（M2-025）；游戏事件人声与无线电滤波（M3-039）。
- 语音转写（R10，首发不做）。
- 举报/管理员静音（M4 安全任务）。

## 验收标准
1. 车内 3 人互通且不泄漏：mock LiveKit 集成测试：3 人在载具内，互相 `crew` 订阅为真；车外第 4 人（含同队、敌队各 1）订阅集合中不含这 3 人的 `crew` 轨道（0 泄漏）。
2. Command 频道成员 = Commander + Officer + Spotter（侦察组长）+ Tank Commander：10 个玩家（Commander、2 Officer、1 TC、1 Spotter 组长、5 普通兵种）：说话与收听权限均为前 5 人，其余 0。
3. 8 路上限：构造 12 候选（2 crew、3 squad、2 command、5 prox），`subscribe_set` 长度 = 8 且包含全部 crew/squad/command（共 7）+ 1 个最近 prox。
4. 迟滞与稳定：prox 在 49/51/59/61 m 反复穿越用例，进入阈值 50 m、退出 60 m；集合变动不足 1 个周期（0.5 s）不下发（时序单测）。
5. API 频率：100 玩家合成场景下，对 LiveKit 的订阅更新调用 ≤ 200 次/s（ARCH §14.2），mock 计数。
6. 服务器 tick 影响：`voice` 部分每 0.5 s 计算耗时 ≤ 1.0 ms（criterion，100 玩家）。

## 测试要求
- 单元：`channel_members`（属性测试：对随机编制，敌我隔离恒成立）；`subscribe_set` 优先级。
- 集成：`cargo test -p server --test voice_channels`（mock LiveKit）；`headless-client` 无语音，故用 server 层断言。
- 性能：`cargo bench -p sim voice_rules`。
- 回归：M2-025 的 squad/prox 测试不变。

## 性能预算
- ARCH §6.10 阶段 5（规则 0.7 ms）不回归；语音计算 ≤ 1.0 ms/0.5 s（折合 < 0.07 ms/tick）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 真实 LiveKit 服务器上的端到端听测需人工 2+ 人参与（E3-11），记 `未测`/partial 直至完成。
- LiveKit 限速参数未核实（ARCH §14.2）；真实 SFU 压测不在本任务。
- 车组频道允许乘客（含运输车）：GD §16.1 写"同一载具内所有座位（含乘客）"，补给卡车 12 座同样适用。
