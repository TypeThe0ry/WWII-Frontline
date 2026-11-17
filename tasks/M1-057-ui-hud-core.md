# M1-057 UI：核心 HUD

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：L
- 前置依赖：M1-053, M1-040
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.6; UI_SPEC §8; UI_SPEC §1.2; UI_SPEC §1.3; GAMEDESIGN §6.10; GAMEDESIGN §8.3

## 背景
战斗中的常驻信息层。UI_SPEC §4.6.1 的元素总表（M1 子集）：准星（散布扩张、命中确认=四角外展 80 ms、爆头提示）、指南针与方位刻度（顶部居中，每帧 `translateX`）、弹匣/备弹（右下，低弹 ≤ 25% 变 `warn`+⚠ 形，GD §6.10 仅显示"满/多/中/少/空"五档而非精确数字，除在弹药箱旁/loadout 内）、换弹进度环、武器槽与射击模式、投掷物数量、姿态图标、体力条（≤ 20% `warn`）、生命状态带（不显示数值，状态带+暗角）、小队成员列表（左中，≤ 8 行可视，1–10 Hz）、据点占领环（进入圈淡入）、对局计时与战线缩略条（左上）、击杀播报（右上，最多 3 条，每条 5 s）、交互提示（底部居中）、受击方向楔形（600 ms 淡出）、压制/流血屏幕效果（统一 FX 控制器）。屏幕分区（§1.3）：安全框 左右 48 px、上下 32 px，屏幕中心 60% 留白。信息层级 L0–L4（§1.2）：L0（准星/受击/倒地/弹匣/换弹）不可关闭、最高对比；L1 事件文本同时最多 3 条+1 条横幅。性能（§8）：HUD DOM ≤ 350 节点；仅动画 `transform/opacity`；每个元素订阅独立信号（禁止订阅整个状态对象）；`SNAP`→UI 最高 20 Hz；禁用大面积 `backdrop-filter`。HUD 在 Pointer Lock 下无点击交互。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/hud/Hud.tsx` —— HUD 根与分区布局（锚点用"角+偏移"，8 px 吸附）。
- `client/src/ui/hud/{Crosshair,Compass,AmmoCounter,WeaponSlots,StanceIndicator,StaminaBar,SquadList,CapturePanel,MatchClock,KillFeed,InteractPrompt,DamageIndicator,ScreenFx}.tsx` —— UI_SPEC §9.2 组件，各自独立信号订阅。
- `client/src/ui/hud/fx-controller.ts` —— 统一 FX 状态机（`Clear/Suppressed/Bleeding/Downed`，§4.6.3），消费 `suppress` 事件与 `self_state.suppression`（来自 M1-017 的输出，由 M1-040 桥接）。
- `client/src/ui/hud/hud-signals.ts` —— 从 `GameState` 派生细粒度信号（弹药、姿态、体力、生命带等）。
- 测试：`client/test/ui/hud.test.ts`、Playwright `hud.spec.ts`（模拟 100 单位事件流）。
### 要实现的行为
1. 弹药：五档显示（满/多/中/少/空）为默认；`ADS`/换弹时的精确数字规则按 GD §6.10；空匣闪描边；换弹进度 30 Hz。
2. 指南针：0–360° 刻度带，每 10° 小刻度，据点与小队成员方向标记；中文环境显示北/东北…；每帧仅写一个 `translateX`。
3. 小队列表：成员图标/名字/兵种/状态（存活/倒地/阵亡）/距离；`TEAM` 数据 > 5 s 未更新隐藏精确距离。
4. 击杀播报：来自 `kill.feed` 事件，最多 3 条，每条 5 s，"我击杀/击倒我"高亮；友军击杀按阵营着色并带形状冗余。
5. 据点占领环：三段环（我方/中立/敌方）；状态机 `Hidden/Visible/Contested/Captured`。
6. 受击方向楔形：`damage.taken` 事件→按相机相对角度，600 ms 淡出；强度映射伤害。
7. 压制/流血/倒地屏幕效果：暗角收缩+斜线纹（减少晃动模式下静态替代，§7.4 占位开关）；压制值来自 `suppress` 事件。
8. 交互提示：`F` 键帽取绑定（`KeyCap`），长按进度环；M1 覆盖拾取/上车/救起/放置补给。
9. 命中确认（`hit.confirm`）：准星四角外展 80 ms，爆头加强；纯事件。
### 接口与数据
```ts
export const hudSignals: { ammoTier: ReadonlySignal<'full'|'high'|'mid'|'low'|'empty'>; stance: ReadonlySignal<Stance>; stamina: ReadonlySignal<number>; healthBand: ReadonlySignal<0|1|2|3>; /* … */ };
export type FxState = 'clear'|'suppressed'|'bleeding'|'downed';
```

## 不做什么
- 不做倒地/阵亡全屏界面（M1-059）、战术地图（M1-058）、载具 HUD（M3-036，M1 仅让载具内显示座位占位见 M1-060）。
- 不做语音指示、聊天、投票横幅（M2）；不做记分板（M2-038）。
- 不做 HUD 元素的显隐/缩放设置面板（M1-061 基础、M2-041 完整）。
- 不做原创图标全集（仅 M1 用到的图标，按 UI_SPEC §2.5 SVG 原创）。

## 验收标准
1. HUD 更新 P95 ≤ 0.8 ms/帧（Playwright 100 单位事件流场景，`performance.measure` 脚本+样式+布局+绘制；INDEX 指标；High 参考机）。
2. 战斗态 HUD DOM 节点数 ≤ 350（脚本统计 HUD 根下 `querySelectorAll('*')`）。
3. 每秒 DOM 写入 ≤ 400（MutationObserver 采样 30 s）；强制同步布局次数/帧 = 0（tracing）。
4. 信息层级符合 UI_SPEC §1.2：L0 元素（准星、受击、弹匣、换弹）不受设置关闭；评审清单逐项勾选写入 STATUS（"需人工"项标明）。
5. 击杀播报最多同时 3 条，第 4 条入队等待（vitest）。
6. 弹药显示：25% 阈值（低弹）转 `warn` 且带 ⚠ 形；空匣闪描边（Playwright 截图断言类名）。
7. 压制事件到屏幕暗角生效延迟 ≤ 1 渲染帧；`prefers-reduced-motion`/减少晃动开关下无抖动动画（vitest）。

## 测试要求
- 单元测试：弹药档位映射、占领环状态机、FX 控制器转移、方向楔角度计算、指南针刻度映射。
- 集成/无头测试：Playwright 注入合成事件流（击杀/受击/压制/占点）并采集 UI 耗时；两种语言快照。
- 性能基准：HUD 脚本耗时 P50/P95 写入 `reports/m1-hud.json`。
- 回归：M1-053 组件快照；CI "HUD 战斗态" 性能场景（超阈值 20% 失败，UI_SPEC §8.5）。

## 性能预算
- UI_SPEC §8.1：UI 总 CPU ≤ 1.5 ms 均值/3 ms P95；HUD DOM ≤ 350；ARCHITECTURE §13.5.1 UI ≤ 0.8 ms/帧。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：HUD 元素与 UI_SPEC 差异写 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：HUD 元素众多；若超一次会话，按"弹药/武器/姿态/体力+准星指南针"与"小队/据点/播报/受击/FX"拆 `M1-057b`。
- 生命"状态带"不显示数值（UI_SPEC 注：【待测 → REFERENCE_MEASUREMENTS】）；M1 先实现 4 档，校准在 M1-071。
- GD §6.10 弹药五档的阈值（满/多/中/少）未在文档给出具体百分比；取初值 100/75/50/25% 并写 PLAN_ISSUES。
