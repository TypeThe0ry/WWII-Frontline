# M2-039 UI：建造界面

## 元数据
- 里程碑：M2
- 工作流：ui
- 规模：M
- 依赖：M1-060, M2-010
- 素材需求：无（灰盒即可）；素材状态：n.a.（建筑虚影用灰盒几何，尺寸取 `/data`）
- 引用文档：UI_SPEC §4.13; UI_SPEC §4.8.1; GAMEDESIGN §12.4; GAMEDESIGN §11.3; GAMEDESIGN §12.1

## 背景
UI_SPEC §4.13：建造是"放置（虚影）→ 施工（进度环）→ 完成"的持续过程；放置合法性与进度放在目标物旁（世界内标签），附近可用 **Supplies**（建造货币，补给箱承载）在底部居中显示；**Supplies 与指挥官资源池 Manpower/Munitions/Fuel 是两套独立系统（R6），界面不得混用**。状态机：Idle → Choosing（`Y` 按住呼出建造轮盘）→ Placing（虚影，滚轮/`Q E` 旋转，左键放置）→ Constructing（按住 `F`/左键施工，多人加速）→ Built；长按 `F` 拆除（Engineer 返还 50% Supplies）。虚影合法性色：蓝=合法 / 斜纹+warn=不合法 / 灰=资源不足。
规则以 sim 为准（M2-010）：GAMEDESIGN §12.4 建造清单（铁丝网 20 Supplies/4 s/150 HP/上限 24；Tank Trap 40/8 s/1 000 HP/16；沙袋墙 25/5 s/400 HP/24；散兵坑 10/10 s/20；MG 巢 100/12 s/800 HP/6；Repair Station 120/15 s/600 HP/3；Resource Node 250/25 s/1 500 HP/各 3；AT 炮阵地 200/20 s/600 HP/3 由 Anti-Tank 建造）；位置限制：只能建在己方 Sector 或战区，据点圆心 `R_sp+15 m` 内仅沙袋/散兵坑，距敌方存活玩家 ≥ 15 m（资源节点 ≥ 40 m），不得封死 `nav_critical`，全队总建筑 ≤ 120。**Garrison 由 Officer/Spotter/Commander 放置（R1，非 Engineer）；Outpost 由 Officer 放置**；M1-060 已有 Officer/Support 的放置工具，本任务把建造交互泛化并接入 Engineer 与 Anti-Tank 蓝图。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/build/BuildController.ts` —— 建造状态机（UI_SPEC §4.13.2）；与 `radial_controller`（M2-037 的 `registerRadial('build', …)`）协作：本任务注册建造轮盘扇区。
- `client/src/ui/build/BuildWheelSpec.ts` —— 按兵种权限生成扇区：防御工事（Engineer）/ 补给箱（Support）/ 障碍（Engineer）/ AT 炮阵地（仅 Anti-Tank）/ Garrison（仅 Officer、Spotter、Commander）/ Outpost（仅 Officer）；灰置条件：Supplies 不足、全队上限、权限。
- `client/src/render/build/ghost_hologram.ts` —— 虚影渲染（全息材质，斜纹/灰/蓝三态；`PH_` 灰盒几何，尺寸来自 `/data/build/*.json`）。
- `client/src/ui/build/BuildLabels.tsx` —— 世界内标签：合法性原因文案、进度环与剩余时间、施工人数；底部 Supplies "x / y"（来源：附近补给箱/车载）。
- `client/src/game/build_client.ts` —— 发送 `BuildPlace{kind,pos,yaw}`、`BuildHold{id,on}`、`BuildDemolish{id}`（M2-007 消息），接收 `BuildState` 事件与拒绝原因枚举。
- `client/src/i18n/{en,zh-CN}/build.json` —— `ui.build.*`（含全部拒绝原因，如 too_close_enemy、outside_sector、near_objective、over_cap、no_supplies、blocks_nav_critical）。
### 要实现的行为
1. 虚影位置由射线投到地形/静态体；本地预判合法性（距离、坡度、与 `nav_critical` 的粗检查）仅用于着色；**最终合法性以服务器响应为准**（UI 不得因本地预判而阻止服务器已允许的放置）。
2. 拒绝原因：服务器返回枚举，界面把原因置于虚影上方一行（UI_SPEC §4.13.4）。
3. 上限/余量实时：全队上限（如沙袋 24）与附近 Supplies 来自 `SNAP`/`EVT`，变化时 ≤ 250 ms 内更新轮盘灰置。
4. 施工：按住 `F`/左键施工，多人加速由服务器给出的进度与人数显示（4 Hz，UI_SPEC §4.13.1）；进度环仅动画 `transform/opacity`。
5. 拆除：长按 `F`（受"长按时长"辅助系数影响，UI_SPEC §7.5）；返还 50% Supplies 的提示（仅 Engineer）。
6. 完成/被摧毁事件播报走 M2-043 的通知通道（接口 `notify(kind,i18nKey,params)`，若未就绪用 console 占位并留 TODO-PI）。
### 接口与数据
```ts
export type BuildRejectReason = 'too_close_enemy' | 'outside_sector' | 'near_objective' | 'over_cap' | 'no_supplies' | 'blocks_nav_critical' | 'bad_slope' | 'no_permission';
export interface BuildBlueprintView { kind: string; costSupplies: number; buildTimeS: number; capTeam: number; usedTeam: number; allowedRoles: RoleId[] }
```
蓝图数据来自 `/data/build/*.json`（M2-010 已定义），客户端不硬编码数值（ARCH §4.4）。

## 不做什么
- 不做建造规则、合法性判定与 HP 增长（M2-010 sim）；Outpost/Repair/AT 炮的 sim 规则（M2-011）。
- 不做 Officer 放置 Garrison 的原有 UI 的重写（M1-060 已有，仅接入新轮盘注册表）。
- 不做炮兵 Shell Stock 与榴弹炮操作 UI（M3）。

## 验收标准
1. 12 种 GAMEDESIGN §12.4 蓝图（铁丝网、Tank Trap、沙袋、散兵坑、MG 巢、Repair Station、Resource Node×3、AT 炮阵地、Garrison、Outpost）各有 1 个 Playwright/vitest 用例：给定服务器拒绝原因，UI 显示对应 i18n 文案（12/12 通过）。
2. 上限/余量一致性：headless 联调中，放置第 25 个沙袋被服务器拒绝，轮盘该扇区在 ≤ 250 ms 内灰置并显示"已达上限"；Supplies 减少事件到达后底部数值 ≤ 250 ms 更新。
3. 权限：Rifleman 无建造轮盘（不呼出）；Engineer 轮盘无 Garrison 扇区；Officer 有 Garrison 与 Outpost 扇区；Anti-Tank 仅有 AT 炮阵地（单测 6 兵种断言，R1/R7）。
4. 虚影三态着色（蓝/斜纹/灰）与合法性枚举一一对应（截图基线 3 张）。
5. 建造 UI 活动时 UI 脚本耗时 P95 ≤ 1.5 ms/帧；世界内标签 ≤ 8 个同屏（超出按距离隐藏）。
6. i18n 键完整性（en/zh-CN）通过；所有拒绝原因枚举均有文案（遍历枚举单测）。

## 测试要求
- 单元测试：状态机转换；轮盘权限矩阵；拒绝原因到文案映射；Supplies 显示格式化。
- 集成/无头测试：headless-client + 测试服：放置/施工/拆除全流程往返；Playwright 截图。
- 性能基准：UI 性能采集（`?uiperf=1`）。
- 回归：M1-060 的 Officer/Support 放置与载具进出用例继续通过。

## 性能预算
- UI_SPEC §8.1：HUD 节点 ≤ 350、UI CPU ≤ 1.5 ms 均值；虚影渲染 ≤ 0.1 ms（单对象）；不得使 ARCH §13.5.3 draw call 预算回归（虚影 ≤ 3 draw call）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 工兵动画 V-ANM-007（建造/挖掘/修理）缺失时不播放动作，仅显示进度环（占位）。
- 若 M2-010 的拒绝原因枚举与上表不同，以 protocol 为准并更新本文件备注。
- 与 M2-037 共用 `registerRadial` 注册表；若 M2-037 未完成，本任务不得自建第二套轮盘（先联调顺序：M2-037 → M2-039，编号已保证）。
