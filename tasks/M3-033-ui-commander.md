# M3-033 UI：指挥官界面

## 元数据
- 里程碑：M3
- 工作流：ui
- 规模：L
- 前置依赖：M2-044, M3-006, M3-007, M3-008, M3-009
- 素材需求：无（图标由执行者按 UI_SPEC §2.5 以 SVG 原创绘制）；素材状态：n.a.
- 引用文档：UI_SPEC §4.11; UI_SPEC §4.7; UI_SPEC §8; GAMEDESIGN §13.1; GAMEDESIGN §13.2; GAMEDESIGN §13.10; GAMEDESIGN §14.6

## 背景
Commander 的核心玩法是"读图 + 点选"（UI_SPEC §4.11）。界面不是独立屏，而是战术地图（§4.7，图层 L-Cmd）的指挥层叠加：左侧资源账本、底部 8 个能力槽、地图上直接放置目标。呼出键 `K`，默认"半屏模式"（地图 70% 透明，世界仍可见），支持"受击自动收起"。
sim/协议侧已就绪：M3-006（席位/资源池/指令校验）、M3-007/008/009（8 种能力）。能力数值来自 `data/commander/*.json`，UI 不硬编码（GD §13.2 初值：Supply Drop M100+F20/240 s/15 s；Ammo Drop U120/180 s；Bombing U350+F120/600 s/45 s，条带 400×70 m；Strafing U150+F60/360 s，300×14 m；Precision U200+F80/420 s，R 22 m；Recon F60/240 s，300×120 m；Rally M120/300 s，R 40 m；Airhead M200/360 s，R 25 m）。
GD §13.1：取消在飞机到达前返还 50%；友军 25 m 内显示"危险近距"但仍允许；落点距敌方据点圆心须 ≥ `R_sp + 80 m`（空投）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/commander/CommanderOverlay.tsx` —— 根组件，状态机 Closed/Overview/Targeting/Confirming/Dispatching（UI §4.11.3）。
- `client/src/ui/commander/Ledger.tsx` —— Manpower/Munitions/Fuel 当前值、每分钟增量、上限（1 Hz，`EVT`）。
- `client/src/ui/commander/AbilityTray.tsx` —— 8 个能力槽：图标、名称、消耗、冷却环、不可用原因（"缺 Fuel 20"）；热键 `1–8`。
- `client/src/ui/commander/PlacementPreview.ts` —— 地图 Canvas 上的预览：圆形（Precision 22 m、Rally 40 m、Airhead 25 m、空投散布 15/10 m）、条带（Bombing/Strafing/Recon，拖拽定方向，`Q E`/滚轮旋转）、友军 25 m 内高亮 `warn` 并显示人数。
- `client/src/ui/commander/VehicleDispatch.tsx` —— 载具调度列表（类型、成本 M/U/F、全队上限与冷却、刷新点选择、认领倒计时 60 s）。
- `client/src/ui/commander/OrdersPanel.tsx` —— 全队命令（Attack/Defend/Move/Observe）放置与撤销。
- `client/src/ui/commander/BriefingStrip.tsx` —— 战线推进/敌情标记数简报。
- `client/src/net/commanderActions.ts` —— 发送 `CommanderOrder`（M3-005）并处理 accept/reject。
- `client/src/i18n/*/cmdr.json` —— `ui.cmdr.*` 全部文案（en + zh-CN，M3-053 再做完整性检查）。
### 要实现的行为
1. `K` 打开/关闭；被击倒/死亡自动禁用并提示；断线时关闭，Officer 端显示"指挥官离线"。
2. 选择能力 → 进入 Targeting → 左键放置（条带类按住拖拽设方向，释放即确认）→ 发送指令 → Confirming → 服务器 accept/reject。右键/`Esc` 取消。
3. 非法落点即时在预览中给出原因（`ui.cmdr.err.*`）：冷却中、资源不足、落点距敌据点过近（空投）、空中支援全局锁 10 s、在空飞机已达 3；服务器拒绝的原因码同样映射文案。
4. 取消在飞能力：槽上显示取消按钮，提示"返还 50%"。
5. 每个能力显示预警时长（敌方警告于 T−15 s）；友军误伤警告不阻止。
6. 载具呼叫：选类型 → 选 HQ 刷新点 → 确认；刷新后 60 s 未认领自动回收提示。
7. 半屏模式、受击自动收起（设置项，默认关）。
### 接口与数据
- 数据：`data/commander/*.json`、`data/vehicles/*.json`（成本/上限/冷却）；事件：`EVT.resources`、`EVT.ability_cooldowns`。
- 信号：每个槽独立订阅（UI_SPEC §8.2），禁止订阅整个状态树。

## 不做什么
- 能力规则、指令校验（sim：M3-006~009、M3-017）；飞机表现（M3-030）。
- Spotter/OP 标记（M3-034）；载具座位 HUD（M3-035）。
- LLM 指挥官的展示面板（M3-027，如需）。

## 验收标准
1. 全部 8 项能力 + 载具呼叫可从 UI 下达并被服务器接受（headless Playwright + 本地 server 脚本测试 9 个用例，全部 `accepted`）。
2. 操作 ≤ 3 次点击：每个能力从"热键/点击槽"到"服务器确认"的点击数 ≤ 3（Playwright 点击计数断言；条带类拖拽记 1 次）。
3. 非法输入给出原因：冷却中、资源不足、距敌据点过近、全局锁、非 Commander 共 5 类，各一个用例，文案 key 存在于 en 与 zh-CN。
4. 友军 25 m 内预览出现 `warn` 且显示人数；不阻止下达。
5. 性能：指挥官界面打开时 UI CPU 均值 ≤ 1.5 ms/帧、P95 ≤ 3 ms（UI_SPEC §8.1，Playwright tracing）；Canvas 地图单次重绘 ≤ 2 ms（200 单位 + 100 标记）。
6. DOM 节点（指挥官叠层）≤ 350（HUD 预算口径，UI_SPEC §8.1）。
7. 无硬编码数字：grep `client/src/ui/commander` 中不得出现能力成本/冷却的字面量（CI lint 规则或测试）。

## 测试要求
- 单元（vitest）：状态机转换、预览几何（条带端点、友军计数）、原因码映射。
- 集成：Playwright 与本地 `server` + `headless-client`，Commander 席位占用场景。
- 性能：UI tracing 场景 "HUD 战斗态 + 指挥官叠层"（UI_SPEC §8.5）。
- 回归：M3-048 视觉基线（指挥官面板 1 机位可选）。

## 性能预算
- UI_SPEC §8.1：UI 总 CPU ≤ 1.5 ms 均值 / ≤ 3 ms P95；每秒 DOM 写入 ≤ 400；ARCH §13.5.1 UI 0.8 ms 不回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；可拆 `M3-033b`（载具调度 + 命令面板）。
- 能力图标需原创 SVG（UI_SPEC §2.5），D-MOD 对 UI 图标的边界待人确认（NEEDS_HUMAN H-001 决议）。
- UI_SPEC §4.11 线框中的预览半径（如 60 m）是示意，实际以 `/data` 为准。
- 载具调度的"产能队列"在 GD §14.6 未定义（刷新为 3 s 出现），UI 只显示待命/生成中/待认领，不做队列。
