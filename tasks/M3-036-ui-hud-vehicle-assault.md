# M3-036 UI：载具 HUD 细节与 Assault HUD

## 元数据
- 里程碑：M3
- 工作流：ui
- 规模：M
- 前置依赖：M1-057, M3-020, M3-013
- 素材需求：无（SVG 原创）；素材状态：n.a.
- 引用文档：UI_SPEC §4.6; UI_SPEC §4.14; GAMEDESIGN §3.4; GAMEDESIGN §14.5; GAMEDESIGN §14.8; GAMEDESIGN §14.1

## 背景
M1-057 建了基础 HUD（UI_SPEC §4.6 的信息元素总表）。M3-035 提供载具座位/弹药/装填与火炮 HUD 的主体。本任务补两块：
1. **载具 HUD 细节**：车内/车外 HUD 切换；模块损伤示意（车体线稿，模块着色；履带 L/R、发动机、炮塔转向、炮管、弹药架、乘员——正常/受损/失效，用 `warn`/`danger` + 形状角标，不只靠颜色）；耐久分段（不显示精确数值）；离车/被击毁提示（"弃车"倒计时、乘员弹出）；维修提示（Engineer 靠近显示可修模块，长按 `F` 修理，GD §14.8：Engineer 8 s / Crewman 12 s 回到 50%）。
2. **Assault HUD**（GD §3.4）：阶段条（5 线中当前目标线）、剩余时间（初始 20 min，占领 +8 min，加时显示）、争夺窗口（60 s，最多再 90 s）提示、攻/守角色与目标、后撤重生点生效提示、"攻方不能在未占领 Sector 建 Garrison"类限制提示。
模块表（GD §14.5.1）：Engine 300（轻 200/重 400）、Track 250/Tires 每组 100、Turret Drive 200、Main Gun 250、Ammo Rack 150；≤ 50% 与 = 0 的效果见表。HUD 不显示精确 HP（UI_SPEC §4.6.1 品类特点）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/vehicle/DamageSchematic.tsx` —— 顶视车体线稿 SVG（按车型：坦克/半履带/轮式 3 套模板），6 个模块区着色；状态由 `module_state: 0 正常|1 受损(≤50%)|2 失效` 驱动。
- `client/src/ui/vehicle/VehicleHudSwitch.tsx` —— 车内（座位竖列左下、弹药右下、损伤图中左）/车外（友方载具信息：型号、乘员名单、乘员数）两套布局切换。
- `client/src/ui/vehicle/BailoutPrompt.tsx` —— 击毁倒计时/弃车提示（与 M3-032 的 8 s 燃烧同步）。
- `client/src/ui/vehicle/RepairPrompt.tsx` —— 靠近己方受损载具时显示可修模块与按住进度环；修理中断条件（受伤 ≥ 15 HP 或离开 3 m）提示。
- `client/src/ui/hud/AssaultHud.tsx` —— 阶段条、倒计时、加时、争夺窗口、攻/守标签、阶段推进横幅（走 `ui.banner.*`）。
- `client/src/ui/hud/hudLayout.ts`（改）—— 注册新元素与"准星安全区"。
- `client/src/i18n/*/hud_veh_assault.json` —— `ui.hud.*`、`ui.veh.dmg.*`、`ui.hud.assault.*`。
### 要实现的行为
1. 模块状态由 `vehicle.module` 事件驱动（≤ 事件频率，不轮询）；每个模块用独立信号，只重渲染变化的 SVG 节点。
2. 色盲安全：`warn`/`danger` 同时带形状角标（三角/叉），在色盲变体 token 下可区分（UI_SPEC §2.2.3/§7.1）。
3. 车内/车外切换：上车/下车的下一帧完成；车外只在 ≤ 30 m 的己方载具显示信息。
4. Assault HUD：阶段推进时显示横幅；计时在最后 60 s 变 `warn`；争夺窗口显示"延长中 Ns/90 s"。
5. 阶段条与 `MatchPhase` 同步：服务器 `MatchPhase::Assault{stage, time_left, overtime}` 变化后下一帧更新。
6. 所有新增元素不得占用屏幕中心**准星安全区**（中心 ±8% 屏宽×±8% 屏高），且在 `?uiperf=1` 下统计节点数。
### 接口与数据
- 订阅：`vehicle.module`、`vehicle.hull_tier`、`match.phase`、`match.assault`。
- 常量全部来自 `/data`（`capture_bonus`、`initial_time`、争夺窗口时长）。

## 不做什么
- 座位列表/弹种/装填/火炮 HUD（M3-035）；击毁特效（M3-032）。
- 指挥官界面、结算页的 Assault 统计（M2/M4 结算任务）。
- Assault 规则（M3-020）。

## 验收标准
1. 阶段切换提示与服务器 `MatchPhase` 同步：本地服务器推进阶段后，HUD 阶段条在 ≤ 1 帧（16.7 ms）+ 事件到达时间内更新（Playwright 时间戳差 ≤ 50 ms）。
2. 无遮挡准星：Playwright 在 1920×1080 与 2560×1080（21:9）下，断言中心安全区内无新增 HUD 元素的 bounding box 相交（0 相交）。
3. 模块损伤 6 个模块 × 3 状态 = 18 个渲染用例截图，且色盲（deuteranopia 变体）下每个失效状态仍带形状角标（DOM 断言存在 `data-shape`）。
4. 战斗态 DOM ≤ 350 节点（含新增元素）；UI CPU 均值 ≤ 1.5 ms、P95 ≤ 3 ms（UI_SPEC §8.1）。
5. 倒计时精度：Assault 剩余时间显示与服务器值偏差 ≤ 1 s（用例模拟 80 ms RTT）。
6. 弃车倒计时与燃烧 8 s 同步：显示归零与 `vehicle.exploded` 事件差 ≤ 100 ms。

## 测试要求
- 单元：模块状态映射、安全区检测函数、计时器与服务器 tick 对齐。
- 集成：Playwright + 本地 server：Assault 测试配置完整推进 1 个阶段；载具被击中模块损伤序列。
- 性能：UI tracing 场景（含 Assault HUD 与车内 HUD 同屏）。
- 回归：M3-048 可选 1 个 HUD 机位。

## 性能预算
- UI_SPEC §8.1；ARCH §13.5.1 UI 0.8 ms 不回归；每秒 DOM 写入 ≤ 400。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 与 M3-035 在载具 HUD 上有交叠：两者共用 `client/src/ui/vehicle/` 目录与同一布局注册表，注意合并冲突；M3-035 先于本任务。
- 线稿模板仅 3 套（坦克/半履带/轮式），不做每车型独立绘制（避免素材需求）。
- 安全区尺寸 ±8% 为本任务提议值（UI_SPEC §1.3 的逻辑安全框优先，冲突以其为准）。
