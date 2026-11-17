# M3-034 UI：Spotter 与 OP 工具

## 元数据
- 里程碑：M3
- 工作流：ui
- 规模：M
- 前置依赖：M1-060, M3-010
- 素材需求：无（SVG 原创图标）；素材状态：n.a.
- 引用文档：UI_SPEC §4.12; UI_SPEC §4.13; UI_SPEC §4.7.3; GAMEDESIGN §17.1; GAMEDESIGN §17.2; GAMEDESIGN §10.7; GAMEDESIGN §15.3

## 背景
M3-010 在 sim 中实现了 Spotter 标记、OP 放置与暴露规则（R3：被标记/侦察暴露的敌人只下发地图图标，1 Hz、位置量化到 5 m）。M1-060 已有 Officer 放置工具（Garrison/Outpost 的"虚影预览 + 合法性色编码"，UI_SPEC §4.12/§4.13 的预览组件）。本任务复用该组件做 OP 放置，并实现望远镜精确标记交互与标记生命周期显示。
规则摘要：
- Spotter 望远镜 10×（FOV 6°），按住标记键 **1.2 s**（Officer 4× 望远镜、1.8 s）；最大距离 800 m（Officer 500 m）；须 LOS（GD §17.1）。
- 可标记：敌步兵（显示 8 s）、载具（15 s，视线内实时更新，丢失后显示最后位置）、火炮/AT 炮/Garrison/Outpost/OP/节点/工事（60 s）。对被标记者不提示。
- OP（GD §10.7）：仅 Spotter 放置；周围 40 m 无敌；距敌方据点圆心 ≥ `R_sp + 40 m`；距友方 Garrison ≥ 60 m；蹲/趴姿放置；放置 10 s；须在地图 `cover_zone`；寿命 15 min；被毁冷却 150 s；使用者仅该 Recon Team。
- 炮兵观察者（Officer/Spotter/Commander）可发布 Fire Mission 标记并发送修正：向左/右、增加/减少 25/50/100 m（GD §15.3-7）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/recon/BinocularsOverlay.tsx` —— 望远镜视图：十字、测距、倍率/FOV 6°；按住标记键显示 1.2 s 进度环，晃动/离开目标则中断。
- `client/src/ui/recon/MarkLifecycle.tsx` —— 标记世界图标（类型、距离）与地图图标的计时淡出；状态：新建/刷新/最后已知/过期。
- `client/src/ui/recon/OpPlacementTool.tsx` —— OP 虚影 + 逐条合法性（40 m 无敌、距敌据点、距友方 Garrison、姿态、cover_zone），不合法时给出原因 `ui.spotter.err.*`；放置 10 s 进度条（受伤/移动中断）。
- `client/src/ui/recon/FireMissionMarker.tsx` —— 观察者在地图/望远镜中放置 Fire Mission 标记与"修正"面板（8 个偏移按钮，显示落点相对标记的偏移量）。
- `client/src/net/markActions.ts` —— 发送标记/放置/修正指令并处理 accept/reject。
- `client/src/i18n/*/spotter.json` —— `ui.spotter.*`、`ui.map.mark.*` 文案。
### 要实现的行为
1. 标记键按住 1.2 s（Officer 1.8 s，来自 `/data` 兵种装备字段，UI 不硬编码）；完成后发送 `MarkTarget`；UI 立即显示"待确认"图标（半透明），服务器确认后转实图标；被拒显示原因（无 LOS、超距、目标不可标记）。
2. 标记生命周期：按类型倒计时显示淡出；同目标重复标记 = 刷新计时（不叠加）；Spot Assist 得分由记分 UI 处理。
3. 标记延迟与服务器一致：UI 显示的"剩余秒数"以服务器事件 tick 计算，而非本地计时起点。
4. OP 放置：非 Spotter 不显示工具；条件不满足时工具灰置并 tooltip 说明；放置合法性的客户端预判只做提示，权威在服务器。
5. Fire Mission：发布后炮手 HUD（M3-035）收到目标；修正面板每次点击发送一条修正，最多显示最近 5 次。
6. 敌方信息守恒（R3）：标记图标只用服务器下发的 1 Hz 5 m 量化位置，不用客户端本地精确敌人位置。
### 接口与数据
- 输入事件：`mark.created/refreshed/expired`、`op.state`；输出：`MarkTarget{target_id|pos, kind}`、`PlaceOp{pos, stance}`、`FireMission{pos}`、`FireCorrection{dx, dz}`。

## 不做什么
- 标记/OP/暴露规则本身（sim：M3-010）；Spotter bot（M3-024）。
- 炮手瞄准界面（M3-035）；Commander 能力放置（M3-033）。
- 近距离语音/音频（不涉及）。

## 验收标准
1. 标记 UI 的"待确认→确认"延迟 = 服务器 RTT + ≤ 1 帧：本地服务器 + `netem 80 ms` 下，图标由半透明转实的时刻与服务器 `mark.created` 到达时刻之差 ≤ 50 ms。
2. 标记倒计时与服务器一致：步兵 8 s、载具 15 s、火炮 60 s，显示淡出完成时刻与事件 `expired` 的差 ≤ 100 ms。
3. OP 工具合法性：5 种不合法条件（有敌 40 m、近敌据点、近友方 Garrison、姿态错误、非 `cover_zone`）各一个用例，均显示对应原因文案（en + zh-CN key 都存在）。
4. 被标记敌人只显示地图图标，世界中不显示敌方精确模型（Playwright 断言：未满足 R3 视锥条件的敌人实体不在场景中）。
5. 望远镜 10× 视图：FOV 6° ±0.1°（vitest 读取相机 FOV）；标记键释放 < 1.2 s 不发送指令。
6. UI CPU 均值 ≤ 1.5 ms/帧，战斗态 DOM ≤ 350 节点（UI_SPEC §8.1）。

## 测试要求
- 单元：计时器与服务器 tick 对齐、合法性预判函数、修正偏移量计算。
- 集成：Playwright + 本地 server，两个客户端（Spotter、Commander）验证标记可见范围（全队）与 Officer 标记（小队 + Commander）。
- 性能：UI tracing，含 50 个标记的地图重绘 ≤ 2 ms。
- 回归：无。

## 性能预算
- UI_SPEC §8.1 总预算；ARCH §13.5.1 UI 0.8 ms 不回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 对本任务只列了"标记/OP/生命周期"；Fire Mission 观察者侧（发布与修正）归入本任务，炮手侧归 M3-035。如 M3-018 未提供对应消息，写 PLAN_ISSUES。
- Garrison 放置者按 R1 为 Officer/Spotter/Commander，沿用 M1-060 的工具，本任务不改动。
- UI_SPEC §4.12 的 Outpost 放置不属本任务。
