# M1-040 client/game：游戏状态与事件总线类型

## 元数据
- 里程碑：M1
- 工作流：net
- 规模：L
- 依赖：M0-022（预测/对账/插值/事件总线骨架）, M1-010（M1 全消息集）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §8.4; ARCHITECTURE §7.2; ARCHITECTURE §7.3; ARCHITECTURE §4.3.9; ARCHITECTURE §6.8.3; ARCHITECTURE §6.8.4

## 背景
M0 的 `src/game` 只有预测/对账/插值与最小事件总线。M1 的 UI（部署界面、HUD、战术地图、倒地界面）与音频（枪声、命中）都只通过"只读 `GameState` signals + 事件总线"消费游戏状态（ARCHITECTURE §4.3.9：`render/audio/ui` 只读 `game` 并订阅事件，`game` 不 import 它们）。本任务把 M1-010 的 C0/C4/C2.events 消息解码结果落到客户端状态机与类型化事件上，是 M1-054..061、M1-063 的共同上游。
关键约束：事件总线为自建 `EventBus<EventMap>`，类型安全、同步分发、**零分配热路径（事件对象池）**（§8.4）；订阅者必须 O(1) 返回。C4 可靠事件需按 `event_id` 去重（C2.events 为冗余发送，§6.5.1）。

## 具体内容
### 要创建/修改的文件
- `client/src/game/events.ts` —— `EventMap` 完整类型（在 §8.4 目录基础上补 M1 事件：`deploy.state`、`objective.changed`、`kill.feed`、`revive`、`roster.changed`、`squad.changed`、`spawnpoint.changed`、`match.phase`、`notice`、`hit.confirm`、`damage.taken`、`suppress`、`marker.changed`）。
- `client/src/game/state/*.ts` —— signals 状态：`roster`、`squads`、`deploy`（部署状态机）、`objectives`、`spawnPoints`、`killFeed`（环形 ≤ 8 条）、`self`（来自 `self_state`：弹药/生命/体力/压制/姿态/冷却位）、`matchPhase`。
- `client/src/game/apply.ts` —— `applyReliableEvent(ev)`（C4）与 `applyUnreliableEvent(ev)`（C2.events）：去重（`event_id` 滑窗 256）、更新 signals、发总线事件。
- `client/src/game/deploy-fsm.ts` —— 部署状态机：`Idle → Choosing → Requested → Counting(spawn_tick) → Spawned → Dead/Downed → Choosing`；与 GD §2.2 状态名对应，服务器 `DeployResult` 为权威。
- `client/src/game/pool.ts` —— 事件对象池。
- `client/test/game/*.test.ts`（vitest）。
### 要实现的行为
1. 处理 C0：`DeployResult`（含拒绝 reason→i18n key）、`SquadOp` 回执、`Chat`（仅存入环形缓冲，UI 在 M2）。
2. 处理 C4：`Kill`、`Death`（含 `respawn_after`、`can_be_revived`）、`Revive`、`ScoreDelta`、`ObjectiveChange`、`SpawnPointState`、`PlayerRoster`（增量合并）、`Notice`（code + args → i18n key）、`MatchPhase`。
3. 处理 C2.events：`Shot`→`fire.remote`；`Impact`→`impact`；`HitConfirm`（仅射手）→`hit.confirm`；`DamageDir`→`damage.taken`；`Explosion`；`Suppress`→`suppress`。
4. 部署状态机与 `self_state.deploy_state(3b)` 对齐；断线重连后以 `Welcome` + 全量 roster 重建状态。
5. `GameState` 以 signals 暴露只读视图；写接口仅 `apply.ts` 内部可见（类型上用 `ReadonlySignal`）。
6. 事件对象池：热路径事件（`fire.*`、`impact`、`hit.confirm`）不产生每次分配（用 `performance.now` 对比或 allocation 计数测试）。
### 接口与数据
```ts
export interface EventMap { 'kill.feed': {killer:number; victim:number; weapon:number; headshot:boolean; dist:number};
  'deploy.state': {phase: DeployPhase; spawnTick?: number; reason?: string}; /* …见 §8.4 目录 + 上文补充 */ }
export interface GameState { readonly roster: ReadonlySignal<Map<PlayerId,RosterEntry>>; readonly deploy: ReadonlySignal<DeployState>; /* … */ }
export function applyReliableEvent(ev: C4Event): void;
```

## 不做什么
- 不做任何 UI 渲染（M1-054..061）、音频播放（M1-063）。
- 不做预测/对账本身（M0-022）与载具预测（M1-041）。
- 不做聊天 UI/语音状态（M2-040）、记分板数据汇总（M2-038）。
- 不新增协议消息（M1-010 已定义）。

## 验收标准
1. 部署状态机全部合法转移与非法转移均有 vitest 用例，覆盖率 ≥ 95%（`pnpm -C client vitest run --coverage game/deploy-fsm`）。
2. 同一 `event_id` 重复投递 3 次，signals 只更新 1 次、总线只发 1 次（测试 `dedup_c4`）。
3. `PlayerRoster` 增量合并：随机乱序的 1000 条增量与顺序应用的最终 roster 一致（property test）。
4. 热路径事件零分配：10 000 次 `fire.remote` 发射的 JS 堆增量 < 64 KB（测试用 `performance.measureUserAgentSpecificMemory` 或 Node `process.memoryUsage` 近似）。
5. `Kill` 事件到 `killFeed` 状态更新耗时 P95 < 0.05 ms（vitest bench）。
6. `game` 目录下无对 `render/ui/audio` 的 import（eslint `no-restricted-imports` 规则 CI 通过）。
7. 重连场景：断线→`Welcome`→全量 roster，状态与断线前一致（`reconnect_rebuild`）。

## 测试要求
- 单元测试：各 C4/C2.events 到状态/事件映射；去重滑窗边界（256 条回绕）；拒绝 reason 映射。
- 集成/无头测试：用 M0 的快照录制回放 + 合成 C4 序列驱动整套状态机（vitest）。
- 性能基准：vitest bench 记录 `applyReliableEvent` 吞吐，基线存 `reports/m1-game-state-bench.json`。
- 回归：M0-022 的预测/对账测试保持通过。

## 性能预算
- ARCHITECTURE §13.5.1：主线程 CPU 总预算 16.67 ms（High），其中 game 状态更新不得使 §13.5.1 表中"net+game"行回归；本任务单帧事件处理目标 ≤ 0.3 ms（100 事件/帧的极端值）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：EventMap 新增条目已回写 ARCHITECTURE §8.4 的"变更说明"（PLAN_ISSUES 登记）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：若 EventMap 与全部 C4 处理超一次会话，把"部署状态机+roster"保留在本任务，其余追加 `M1-040b`（README §5.6）。
- 服务器权威：客户端不显示预测的伤害/击杀（ARCHITECTURE §7.2 注），`kill.feed` 仅来自 C4 `Kill`。
- 与 M1-010 的字段宽度若有差异，以 protocol 实现为准并写 PLAN_ISSUES。
