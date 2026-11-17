# M2-042 UI：结算界面

## 元数据
- 里程碑：M2
- 工作流：ui
- 规模：M
- 依赖：M2-038, M2-017
- 素材需求：I-MUS-002（结算短曲，可选，缺则静音）；素材状态：placeholder
- 引用文档：UI_SPEC §4.17; GAMEDESIGN §18.4; GAMEDESIGN §18.2; GAMEDESIGN §19.3; GAMEDESIGN §3.3.4

## 背景
UI_SPEC §4.17：对局结束后先给结论（胜负大字 + 一句原因），再给个人，再给全队，最后给动作；避免第一屏信息过载。元素：胜负标题与原因（`full_control`/`time_limit_lead`/`tiebreak`，GAMEDESIGN §3.3.4 / §18.4）、战线终态图（5 条战线）、个人统计（击杀/阵亡/助攻/战斗-进攻-防守-支援分/治疗救起数/建造数/补给数/命中率可选/最远击杀）、小队贡献与 MVP、全队排行榜（Top 5 各类别）、本局时间线（据点易主，Canvas 小图）、下一局地图投票（3 个候选，GAMEDESIGN §19.3 Map Vote）、操作按钮（同服下一局/返回服务器浏览器/主菜单/举报玩家）、可选好评/差评问卷。状态机：Summary ⇄ Detail、Summary → Voting → Summary、→ Leaving。
空/错误：数据延迟先显示胜负与已有数据，其余骨架，超时 10 s 显示"统计暂不可用"但仍可离开；匿名游客提示"登录以保存战绩"（可关闭）；断线重连落在结算则直接显示结算。数据来源：M2-017 的局终统计（GAMEDESIGN §18.4 含 Match XP 构成与账号等级进度；M2 内账号进度来自 gateway M2-022，未就绪则隐藏该块）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/result/ResultScreen.tsx` —— 结算根组件与状态机；复用 M2-038 的 `ScoreRow`（Top 榜）。
- `client/src/ui/result/FrontlineSummary.tsx` —— 战线终态图 + 时间线 Canvas2D 小图（`Path2D` 缓存，UI_SPEC §8.3）。
- `client/src/ui/result/PersonalStats.tsx`、`SquadMvp.tsx`、`TeamLeaders.tsx` —— 个人/小队/排行块。
- `client/src/ui/result/NextMapVote.tsx` —— 下一局投票（候选 3 个 + 倒计时，投票消息走 M2-043 同一协议；服务器权威计票）。
- `client/src/game/match_end_client.ts` —— 接收 `MatchEnded{reason,winner,stats…}` 与后续 `MatchStatsFull`（M2-017 定义）；10 s 超时处理；重连落在结算的恢复。
- `client/src/ui/result/result_audio.ts` —— 结算短曲触发（I-MUS-002，缺失静音 + 一次性日志，ASSET_GAPS §3.3）。
- `client/src/i18n/{en,zh-CN}/result.json` —— `ui.result.*`。
### 要实现的行为
1. 收到 `MatchEnded` 立即渲染胜负与原因，其余块以骨架占位，逐块填充；10 s 未收全显示"统计暂不可用"，"离开"按钮始终可用。
2. 所有数值直接显示服务器数据，不在客户端重算（防不一致）；时间格式 `mm:ss`。
3. 同服下一局：发送 `RequestRematch`；返回浏览器/主菜单：确认后断开并清理对局状态（Pointer Lock 释放、对局资源回收）。
4. 游客提示可关闭，关闭状态存 `localStorage`（try/catch）。
5. 举报入口：打开 M2-043 举报表单并预选玩家。
### 接口与数据
```ts
export interface MatchEndView { reason: 'full_control' | 'time_limit_lead' | 'tiebreak'; winner: 'A' | 'B' | 'draw'; durationS: number; frontlineTimeline: { tS: number; line: 1|2|3|4|5; owner: 'A'|'B' }[]; me: PersonalStats; squads: SquadSummary[]; leaders: Record<'kills'|'support'|'build'|'heal', ScoreRowData[]> }
```
字段取自 M2-017 的 protocol 定义，不得自造。

## 不做什么
- 不做 Assault 段位示意与 Assault 结算变体（M3）；不做资源曲线"高级视图"。
- 不做账号等级/经验的持久化（M2-022 gateway）；仅展示 gateway 返回值。
- 不做好评问卷后端。
- 不做记分板（M2-038）与投票/举报面板本体（M2-043）。

## 验收标准
1. 数据一致性：headless 完整局（`balance-sim` 或 headless-client 驱动的短局，≥ 5 min）结束后，结算界面显示的 个人 K/D/A、四类分数、总分、小队 MVP、Top 5 榜与服务器局终统计 JSON 逐字段相等（`test-result-consistency`，差异 = 0）。
2. 超时降级：服务器不下发 `MatchStatsFull`，10 s 后显示"统计暂不可用"，"同服下一局/返回浏览器/主菜单"按钮可用（Playwright，计时误差 ≤ 0.5 s）。
3. 退出/再来一局流程：三种动作各 1 个 e2e 用例通过；返回主菜单后重新进服务器浏览器无残留对局状态（内存泄漏检查：返回后 JS 堆回落到进局前 ≤ +10%）。
4. 断线重连落在结算：模拟断线后重连，直接显示结算（Playwright）。
5. 结算界面 DOM ≤ 1 500；时间线 Canvas 单次重绘 ≤ 2 ms。
6. I-MUS-002 缺失时无报错，每会话仅 1 条 `[asset-missing]` 日志。
7. i18n 键完整性（en/zh-CN）。

## 测试要求
- 单元测试：`MatchEndView` 解析与缺字段容错；超时状态机；时间线绘制数据转换。
- 集成/无头测试：headless 短局（可设 `time_limit` 为 3 min 加速）→ 结算数据对比；Playwright 三个出口。
- 性能基准：UI 采集 `?uiperf=1`。
- 回归：结算后进入新局 deploy 流程不受影响。

## 性能预算
- UI_SPEC §8.1：菜单类界面 DOM ≤ 1 500；UI 内存 ≤ 60 MB；Canvas 单次重绘 ≤ 2 ms（§8.1 地图类）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（I-MUS-002）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- UI_SPEC 线框图示例含 "Omaha Beach (Assault)" 下一局；M2 仅 Carentan Frontline，候选地图由服务器轮换配置决定，缺失时隐藏投票块。
- 结算曲素材采购属 H-059（M2 采购批）；缺失不阻塞。
- 多语言长文本（德/英单位名）布局需在 en/zh-CN 两语言下截图检查溢出。
