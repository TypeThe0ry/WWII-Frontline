# M2-043 UI：投票、举报、换队与系统提示

## 元数据
- 里程碑：M2
- 工作流：ui
- 规模：M
- 前置依赖：M1-057, M2-018
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.18; UI_SPEC §4.19; UI_SPEC §5.5; GAMEDESIGN §19.3; GAMEDESIGN §19.1; GAMEDESIGN §19.2; GAMEDESIGN §19.4

## 背景
UI_SPEC §4.18：Esc 菜单是"战术间歇面板"，**不暂停多人对局**（Esc 会强制退出 Pointer Lock，`pointerlockchange` 触发打开菜单，而不是报错）。条目：继续（重新锁定指针，落在用户手势上）、部署界面（重新部署需确认）、换兵种（需死亡/在 HQ 才生效）、换队（受冷却/平衡规则限制，显示冷却）、小队管理入口（`U`）、设置、举报玩家（原因：作弊/辱骂/捣乱/AFK/其他；可附近 N 秒上下文）、投票（踢人/换图/跳过：发起、进行中、赞成/反对计票倒计时；`F1`=赞成 `F2`=反对仅在投票横幅可见时）、离开对局（二次确认）。若对局已结束，Esc 菜单只保留"离开"。
UI_SPEC §4.19 统一通知通道：Toast（右上，堆叠 ≤ 3，3–6 s，错误 8 s）、对局事件横幅（上部中，指南针下，3 s，同时 ≤ 1 个）、命令通知、教程提示（底部中，每条一次，记录于 `localStorage`，可重置/关闭）、阻断对话框（断线/被踢）；相同内容 3 s 内合并为 "×N"；必有 `severity` 与形符号（圆 i/勾/三角/八边形）；`aria-live="polite"`（错误 `assertive`）。数据结构草案 `Notice`（§4.19.2）。
规则（M2-018 sim-server-rules，GAMEDESIGN §19）：Kick 投票——任意真人发起，局内 ≥ 3 min，每人每 5 min 最多 1 次，45 s，同队票 ≥ 8 且赞成 > 反对 × 2；Remove Commander——Officer ≥ 2 联署，在任 ≥ 10 min，同队 ≥ 40% 赞成；Map Vote 局终自动 3 候选；TK 原谅/惩罚（15 s 选择，§19.2）；AFK 警告（90 s 起提示，120 s 起 30 s 倒计时，§19.4）；自动平衡邀请（"移到人数少的队可得 +20% XP"，§19.1）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/pause/PauseMenu.tsx` —— Esc 菜单与状态机（UI_SPEC §4.18.3）；`pointerlockchange` 联动；对局结束后精简。
- `client/src/ui/pause/TeamRoleChange.tsx` —— 换队/换兵种入口（调用部署界面 M1-056 的兵种页与阵营选择 M1-055，显示冷却）。
- `client/src/ui/report/ReportForm.tsx`、`client/src/game/report_client.ts` —— 举报表单（"最近 5 分钟交互过的玩家"列表、原因枚举、限频提示）；提交走 gateway 的举报接口（M2-023 的 `POST /v1/reports`；未就绪时提交返回失败并保留表单，不丢数据）。
- `client/src/ui/vote/VoteBanner.tsx`、`VotePanel.tsx`、`client/src/game/vote_client.ts` —— 投票横幅/面板、`F1/F2` 键、计票与倒计时（服务器权威）；发起 Kick/Remove Commander/Map Vote 的入口与资格提示（新进玩家/游客禁用并说明）。
- `client/src/ui/notice/NoticeCenter.tsx`、`notice_queue.ts` —— 通知中心：优先级队列、合并、插队、`aria-live`；公开 `notify(n: Notice)`（M2-039/M2-040/M1-0xx 使用）。
- `client/src/ui/notice/TutorialHints.ts` —— 教程提示框架：触发条件注册、一次性记录（`localStorage` try/catch，失败按会话计）、设置中"重置教程/全部关闭"开关（设置页接线在 M2-041，内容引导任务 M4-027）。
- `client/src/ui/dialogs/BlockingDialog.tsx` —— 断线/被踢/确认对话框。
- `client/src/i18n/{en,zh-CN}/{pause,report,vote,toast,banner,dialog}.json`。
### 要实现的行为
1. 通知队列：同一时刻 ≤ 1 个横幅、≤ 3 个 toast；超出排队；高优先级插队；3 s 内同内容合并为 `×N`。
2. 投票：横幅可见时 `F1/F2` 投票，面板显示赞成/反对计票与剩余秒数（1 Hz）；结果以服务器 `VoteResult` 为准；限频/资格不足时显示原因。
3. 举报：提交成功 toast；失败保留表单并提示；防刷限频（服务器返回 `retry_after`）。
4. TK 原谅/惩罚弹窗（15 s，GAMEDESIGN §19.2）与自动平衡邀请、AFK 倒计时统一走通知中心（专属样式，高优先级）。
5. 安全框：所有横幅/toast 不得进入准星区域安全框（UI_SPEC §1.3，屏幕中心 ±15% 宽高禁区）。
### 接口与数据
```ts
export interface Notice { id: string; kind: 'toast'|'banner'|'order'|'tutorial'|'dialog'; severity: 'info'|'ok'|'warn'|'error'; i18nKey: string; params?: Record<string, string|number>; ttlMs?: number; dedupeKey?: string; sound?: string }
export function notify(n: Omit<Notice, 'id'>): string;
```

## 不做什么
- 不做完整新手引导内容（M4-027）；不做设置页（M2-041）；不做记分板（M2-038）。
- 不做服务器侧投票/TK/AFK/平衡规则（M2-018）与举报入库/封禁（M2-023）。
- 不做指挥官罢免之外的 Commander 席位 UI（M3-033）。

## 验收标准
1. 投票一致性：headless 联调中，5 种投票场景（Kick 通过/Kick 未通过/Remove Commander/Map Vote/限频拒绝）客户端显示结果与服务器 `VoteResult` 100% 一致（5/5）；计票显示滞后 ≤ 1.2 s（1 Hz 刷新 + 网络）。
2. 通知队列：单测覆盖——同刻 ≤ 1 横幅/≤ 3 toast；`×N` 合并窗口 3 s；高优先级插队；错误 toast 8 s（假定时器）。
3. 安全框：Playwright 在 1920×1080 与 21:9 下触发各类通知，其包围盒与准星安全框交集面积 = 0。
4. Esc 流程：按 Esc 解锁指针并打开菜单；"继续"在用户手势内重新锁定，失败时显示"点击继续"遮罩；对局结束后菜单仅"离开"（Playwright 4 个用例）。
5. 举报：提交失败（模拟 5xx）时表单内容保留且显示错误；成功后 toast；限频返回时显示倒计时。
6. `aria-live`：toast 的 DOM 属性为 `polite`、错误为 `assertive`（单测）；每类通知含形符号与 `severity`。
7. 教程提示：同一 ID 只触发一次（`localStorage` 禁用时按会话一次）；"全部关闭"后 0 次触发。
8. i18n 键完整性。

## 测试要求
- 单元测试：队列/合并/插队；投票状态；资格判断；教程一次性；Notice 校验。
- 集成/无头测试：headless 投票/举报/换队往返；Playwright Pointer Lock 流程。
- 性能基准：`?uiperf=1`；通知高频事件（100 事件/s）下 UI CPU ≤ 1.5 ms 均值。
- 回归：HUD 战斗态节点数 ≤ 350。

## 性能预算
- UI_SPEC §8.1：UI CPU ≤ 1.5 ms 均值/3 ms P95；每秒 DOM 写入 ≤ 400；`will-change` 层 ≤ 6。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 举报接口依赖 M2-023（编号在本任务之前不保证，INDEX 中本任务不依赖它）：先对接口契约编程，后端缺失时用 mock，联调在 M2-055 验收前补齐。
- 提示音（`sound`）缺失静音。
- 举报"附上近 N 秒上下文"仅附聊天片段（隐私：保留 14 天，ARCH §18.4），不得附语音。
