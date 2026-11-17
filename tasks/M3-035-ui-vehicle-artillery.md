# M3-035 UI：载具与火炮操作界面

## 元数据
- 里程碑：M3
- 工作流：ui
- 规模：L
- 前置依赖：M1-060, M3-018, M3-017
- 素材需求：无（SVG 原创）；素材状态：n.a.
- 引用文档：UI_SPEC §4.14; UI_SPEC §4.6; GAMEDESIGN §14.1; GAMEDESIGN §14.7; GAMEDESIGN §15.3; GAMEDESIGN §15.4; GAMEDESIGN §15.5; GAMEDESIGN §14.6

## 背景
载具是"多人岗位"（UI_SPEC §4.14）：信息核心是"谁在哪个座位、我的武器还剩多少、车哪里坏了"。火炮是 HQ 附近的预设阵地（R7，开局即存在，无建造）。本任务做两个操作界面的数据与主体，细节图（模块损伤线稿、Assault HUD）由 M3-036 做。
关键规则：
- 座位（GD §14.1）：驾驶/炮手/车长/乘客；停车或 < 5 km/h 时 F1–F4 立即换座，移动中换座 2 s；离车按住 0.8 s；车长缺位装填 ×1.3 慢；炮手 `R` 切换弹种需重新装填 1.0 s（GD §14.7）。
- 火炮（GD §15.3/15.4）：炮手 + 装填手；方位转速 4°/s（微调 0.1°/s），仰角 3°/s、范围 5°–85°；装药 1/2/3 的 `Rmax` = 800/1800/3200 m，`v = √(Rmax·g)`；低角 `θ = ½·asin(R/Rmax_c)`，最小射程 `Rmax·sin 10°`（139/313/556 m）；循环 6.0 s（M2A1，2 人）/10 s（1 人），leFH 18：7.0/11.5 s；炮弹库存初始 24 HE + 8 烟，上限 48 + 16；观察者修正向左/右、增减 25/50/100 m。
- AT 炮（GD §15.7）：AP/HE 切换 1.0 s，3.0/4.2 s 装填（US），无机枪。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/vehicle/SeatList.tsx` —— 座位竖列（占用者名、空位、换座过渡 1–2 s）；`1–5` 换座，`F` 进入最近空座，按住 `F` 弹出座位选择。
- `client/src/ui/vehicle/GunHud.tsx` —— 弹种（AP/HE/Smoke）、弹量、装填环（30 Hz）、同轴/车体机枪弹量、炮塔相对车体指示（炮塔环）、速度/档位。
- `client/src/ui/vehicle/EnterPrompt.tsx` —— 进入/离车提示、速度 > 20 km/h 离车伤害警示（30 HP）。
- `client/src/ui/vehicle/ClaimPanel.tsx` —— Armor Crew 认领载具提示：HQ 刷新点列表（类型、距离、认领倒计时 60 s、Armor Crew 人数要求）。
- `client/src/ui/artillery/GunnerHud.tsx` —— 方位环（0–360°，0.1° 精度）、仰角尺（5–85°）、装药/弹种选择、装填进度、库存（HE/烟）、射程圈与预计飞行时间、友军危险区预警（目标附近友军数，开火二次确认）。
- `client/src/ui/artillery/FireTable.tsx` —— "射表卡"：按所选装药列出 R–θ–TOF（读取 `data/artillery/*.json`，与 GD §15.4 表一致）；输入距离给推荐仰角（含 Δh 迭代由服务器返回，UI 展示服务器值）。
- `client/src/net/vehicleActions.ts`、`artilleryActions.ts` —— 指令发送与回执。
- `client/src/i18n/*/veh.json` —— `ui.veh.*`、`ui.veh.arty.*`。
### 要实现的行为
1. 载具内 HUD 状态机 OnFoot/Entering/Seated/Switching/Exiting/Bailout（UI_SPEC §4.14.3）；座位被占时回原座并提示。
2. 弹种切换：`R` 发送切换，UI 显示"重新装填 1.0 s"，装填环显示进度。
3. 火炮：观察者目标到达（`FireMission`）→ HUD 显示方位/距离/推荐装药与仰角，点击"采用"自动对准（按转速逐步转，不瞬移）；超射程显示 `warn` + 箭头；无目标显示"等待观察手目标"。
4. 观察者修正到达后更新目标并高亮变化量；收到服务器落点回报（观察者）显示偏移量由 M3-034 负责，炮手侧只显示修正后的目标值。
5. 开火前若目标 60 m 内有友军，显示危险警告并要求二次按键确认。
6. 单人操作（仅炮手）显示"装填周期延长"提示（10 s / 11.5 s）。
### 接口与数据
- 订阅：`vehicle.seats`、`vehicle.ammo`、`vehicle.reload`（30 Hz 仅装填环）、`artillery.state`、`artillery.target`。
- 数值只从 `/data` 读取：转速、仰角范围、`Rmax`、库存、装填时间。

## 不做什么
- 模块损伤线稿与"弃车"倒计时、车内/车外 HUD 切换、Assault HUD（M3-036）。
- 车辆指挥官调度（M3-033）；Fire Mission 发布（M3-034）。
- 声音（M3-037/038）；火炮物理与弹道（M3-018/019）。

## 验收标准
1. 炮位射表输入与服务器落点一致：关闭散布（测试开关）时，对 GD §15.4 表中 14 组 `(装药, R)`，UI 显示的推荐仰角与表值偏差 ≤ 0.1°；用该仰角开火，服务器落点与目标距离偏差 ≤ 2 m（含重力解析解）。
2. 换座响应：停车状态 `1–5` 换座，UI 在收到服务器确认的同一帧更新（≤ 1 帧延迟）；移动中换座 2 s 过渡显示。
3. 装填环：30 Hz 更新，与服务器装填进度误差 ≤ 1 tick（33 ms）。
4. HUD 更新 ≤ 0.8 ms/帧（ARCH §13.5.1 UI 预算；Playwright tracing，载具内 + 火炮各一个场景）。
5. 友军危险区：目标 60 m 内放 1 名友军，开火需要二次确认（用例）；无友军时单击开火。
6. 超射程/低于最小射程：显示 `warn`，开火按钮禁用（装药 1 对 R=100 m 与 R=900 m 各一用例）。
7. 战斗态 DOM ≤ 350 节点；每秒 DOM 写入 ≤ 400（UI_SPEC §8.1）。

## 测试要求
- 单元（vitest）：低角/高角换算函数 vs GD §15.4 表（14 行）、座位状态机、库存显示。
- 集成：Playwright + 本地 server：Armor Crew 认领 → 进入 → 开炮；炮组 2 人 + 观察者。
- 性能：UI tracing；100 单位事件流叠加。
- 回归：无。

## 性能预算
- ARCH §13.5.1 UI 0.8 ms；UI_SPEC §8.1（均值 1.5 ms / P95 3 ms）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：可拆 `M3-035b`（火炮炮手界面）。
- INDEX 写"车辆刷新点选择"，Commander 侧的调度选择在 M3-033，本任务只做 Armor Crew 的认领界面。
- UI_SPEC §4.14.4 提到"炮管过热/冷却"，GD §15.3 明确"无过热"，以 GD 为准（不做过热显示），在 PLAN_ISSUES 记一条。
- "炮手 + 装填手"的座位集合与 UI_SPEC "Support/Officer 配属的火炮"措辞不同，权限矩阵以 M3-019 的 sim 为准。
