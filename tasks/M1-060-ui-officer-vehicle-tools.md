# M1-060 UI：Officer/Support 放置与载具进出

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：M
- 依赖：M1-057, M1-019（sim Garrison）, M1-020（sim Supplies/补给箱）, M1-041（载具座位同步）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.12; UI_SPEC §4.13; UI_SPEC §4.14; GAMEDESIGN §10.2; GAMEDESIGN §12.1; GAMEDESIGN §12.2; GAMEDESIGN §12.3; GAMEDESIGN §14.7

## 背景
M1 的闭环：Officer 放置 Garrison → 队友经 Garrison 重生 → Support 放置 Supply Crate/Ammo Crate → 队友补弹（MILESTONES E1-11）；3 名玩家乘 Sherman（驾驶/炮手/车长）。UI_SPEC §4.12：放置采用**虚影预览 + 合法性色编码**（复用 §4.13 的预览组件 `PlacementGhost`）；文案 `ui.officer.err.*`。Garrison 规则（GD §10.2，R1：放置者 Officer/Spotter/Commander）：Supplies 消耗己方/中立区 50、敌方区 100【待调参】；间距——距己方 Garrison/Outpost ≥ 150 m、距敌方 Garrison ≥ 250 m、距活动据点圆心 ≥ `R_sp + 100 m`、距 HQ ≥ 200 m；坡度 ≤ 25°；非水体；周围 50 m 内无可见敌人；放置者 15 m 内箱/车须有足够 Supplies；数量上限全队 6/每 Sector 2。Supply Crate：100 Supplies/箱，仅 Support 可搬运（移动 ×0.75），放下 1.5 s；Ammo Crate：600 弹药点，Support 每命 2 个，放置冷却 30 s，使用 4 m 内按住交互 1.5 s，寿命 150 s。**合法性颜色**：INDEX 写"绿/红"，UI_SPEC §4.12/§4.13 规定"蓝=合法 / 斜纹+warn=不合法 / 灰=资源不足"（形状冗余、色盲友好）；**以 UI_SPEC 为准**并登记差异。载具（UI_SPEC §4.14）：进入提示（`F` 进入最近空座、按住 `F` 弹出座位选择）、座位列表（驾驶/炮手/车长，占用者名）、`1–5`（车内）切座（停车/速度 < 5 km/h 立即，移动中 2 s，GD §14.7/§14.1）、长按 `F` 离车（0.8 s，GD §14.1）、被击毁 Bailout。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/hud/PlacementGhostLabel.tsx` —— 世界内虚影标签：合法性、原因、成本（Supplies）、剩余次数；颜色+形状冗余。
- `client/src/game/placement.ts` —— 放置模式状态机（`Idle/PlacingGhost/Invalid/Committing`）：从 loadout 工具槽进入（Garrison 套件、Supply Crate、Ammo Crate），每帧射线求落点（读 `game` 只读状态与 sim-wasm 的合法性预判 `placement_check`），左键放置→C0 请求，右键取消；服务器拒绝时回 `PlacingGhost` 并显示 `ui.officer.err.*`。
- `client/src/render/placement/GhostMesh.ts` —— 虚影网格（灰盒：Garrison=帐篷盒 + 圈，Crate=箱盒）+ 半径环；接口由 UI 提供合法性，渲染只负责着色（蓝/斜纹 warn/灰）。
- `client/src/ui/hud/SquadToolPanel.tsx` —— 小队管理面板（`U`）：成员、Officer 转交、锁定（M1 最小：列表+锁定+踢出+移交）。
- `client/src/ui/hud/VehicleSeatHud.tsx` —— M1 最小载具 HUD：座位竖列（驾驶/炮手/车长+占用者）、速度、同轴/主炮弹种与余量占位、"按 F 离车"提示；`VehicleEnterPrompt`。
- `client/src/game/seat-actions.ts` —— 进入/换座/离车 RPC（对接 M1-041 的 `SeatState`）。
- `crates/sim-wasm/src/lib.rs`（小改）—— `placement_check(kind, pos) -> {legal, reason}` 暴露（规则在 sim M1-019/020；若已存在则仅胶水）。
- 测试：`client/test/ui/placement.test.ts`、`vehicle-hud.test.ts`、Playwright `placement.spec.ts`。
### 要实现的行为
1. Garrison 放置虚影：实时合法性与原因（离敌军过近/地面不平/附近已有重生点/超过数量上限/禁建区/Supplies 不足），原因 i18n key 与服务器拒绝原因共用。
2. 箱子：Support 放置 Supply Crate/Ammo Crate 虚影；Ammo Crate 冷却显示；搬运时 HUD 显示"携带补给箱"。
3. 放置资源显示：Garrison 附近可用 Supplies 余量、成本 50/100、数量上限 `n/6`。
4. 载具：靠近显示进入提示和座位占用预览；`F` 进入最近空座；按住 `F` 弹座位选择；车内 `1–5` 换座，移动中显示 2 s 过渡；长按 `F` 0.8 s 离车；被击毁显示 Bailout 提示。
5. 换座 ≤ 1 次按键（停车时 `1`/`2`/`3` 直接切换）。
6. 错误：非 Officer/小队人数不足/阶段不允许 → 工具不可选，tooltip 说明条件。
### 接口与数据
```ts
export type PlacementKind = 'garrison'|'supply_crate'|'ammo_crate';
export interface PlacementResult { legal: boolean; reasonKey?: string; supplyCost?: number; remaining?: number }
export const placement: { mode: Signal<'idle'|'placing'|'invalid'|'committing'>; kind: Signal<PlacementKind|null>; result: Signal<PlacementResult> };
```

## 不做什么
- 不做 Outpost/OP/Airhead 放置（M2/M3，M3-034 Spotter/OP 工具）、完整建造轮盘与 Engineer 建造菜单（M2-039）。
- 不做放置的规则本身与服务器权威校验（M1-019/M1-020/M1-026）。
- 不做完整载具 HUD（弹种切换、模块损伤示意、火炮界面，M3-035/M3-036）。
- 不做命令轮盘/Order 下发 UI（M2-037）。

## 验收标准
1. 非法位置显示"不合法"态并给出原因：构造 8 类非法场景（距敌 < 50 m、坡度 > 25°、水体、距己方 Garrison < 150 m、Supplies 不足、数量上限、禁建区、距 HQ < 200 m），每类 UI 均显示对应 `reasonKey`（vitest，使用 sim-wasm `placement_check`）。
2. 合法性颜色形状冗余：合法=蓝、不合法=斜纹+warn、资源不足=灰，在色盲模拟（灰度）截图下三态仍可区分（Playwright 截图，人工确认标"需人工"）。
3. 换座 ≤ 1 次按键：停车时按 `2` 直接换座，往返 ≤ 1 RTT+1 tick 确认（脚本）；移动中 2 s ±1 tick（fake timers）。
4. 放置请求在服务器拒绝时 UI 回到 `PlacingGhost` 并显示服务器给出的原因 key（stub）。
5. 放置虚影更新 ≤ 0.3 ms/帧（`?uiperf=1` + 渲染叠层）。
6. 演示闭环（MILESTONES 演示脚本 3–4）可由 2 名脚本玩家完成：Officer 放 Garrison→队友经 Garrison 出生；Support 放 Ammo Crate→队友补弹（headless E2E，需 M1-019/020/026 已完成，否则 `partial`）。

## 测试要求
- 单元测试：放置状态机；合法性原因映射；座位操作（停车/移动）；按键冲突（`F` 长按/短按判定）。
- 集成/无头测试：headless E2E 闭环；Playwright 放置/上车。
- 性能基准：放置模式每帧耗时写入 `reports/m1-placement.json`。
- 回归：M1-057 HUD 交互提示（`F`）一致。

## 性能预算
- UI_SPEC §8.1：UI 总 CPU ≤ 1.5 ms 均值；虚影渲染计入透明/其它 pass，≤ 0.1 ms（灰盒）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID 已列出（虚影灰盒无素材需求）
- [ ] 文档同步：合法性颜色与 INDEX"绿/红"差异登记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Garrison 放置者以 R1 为准（Officer/Spotter/Commander，非 Engineer）；UI_SPEC §4.12 正文已同步 R1。
- 放置合法性规则必须只在 sim 一份，UI 通过 `placement_check` 取，不得复制公式。
