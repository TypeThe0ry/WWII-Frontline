# M1-059 UI：倒地与阵亡界面

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：M
- 依赖：M1-057, M1-016（sim 倒地/救起/击杀归属）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.10; GAMEDESIGN §7; GAMEDESIGN §10.6; GAMEDESIGN §2.2; UI_SPEC §7.4

## 背景
高致死率品类里，倒地等待是一个"社交时刻"。UI_SPEC §4.10 的重点不是惩罚，而是让玩家知道还有希望、有多近、怎么放弃。GD §7.2 数值：倒地血量 `downed_hp` 40 HP，倒地流血 0.5 HP/s（约 80 s 耗尽）；放弃等待倒地 5 s 后可用，按住 3 s 转 KIA；倒地移动爬行 0.6 m/s；倒地视角低位、视野 −30%、暗角；Medic 在 ≤ 1.5 m 处按住 5.0 s 救起，被救起者 HP 40、无流血、2 s 内不能射击；呼救（Medic Ping）每 5 s 一次。UI_SPEC §4.10.1 信息元素：状态标题、流血倒计时环（10 Hz）、最近医疗兵（距离、方向箭头、是否在途；`TEAM`/近距 AOI，2 Hz）、"呼叫医疗"（`F`/`Mouse3`，冷却 5 s）、放弃按钮（按住 `Space` 1.5 s 或确认——注意：GD §7.2 放弃为"倒地 5 s 后可用、按住 3 s"，**以 GD 为准**，UI 数值读取服务器下发的规则字段）、击杀者信息（规则见下）、重新部署入口（阵亡后 `Enter`→M1-056）、友军观察视角（阵亡后可切换队友，倒地时不可）。**击杀者信息显示规则**（§4.10.1）：倒地阶段不显示击杀者昵称与位置，仅受击楔形；阵亡后立即显示击杀者昵称、武器、距离（不显示当前位置）；击杀者为 bot 时显示"AI 控制"标记；设置"隐藏击杀者昵称"开启时只显示武器与距离；友军误杀显示"友军误伤"。状态机（§4.10.3）：`Alive → Downed → Revived → Alive`，`Downed → Dead`，`Dead → Spectating → Deploying`。死亡镜头 3 s，之后进入 DEPLOY（GD §10.6）；重生倒计时由服务器 `spawn_tick` 给出。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/screens/DownedDeath.tsx` —— 倒地/阵亡覆盖屏（面板+世界降饱和斜线纹边框）。
- `client/src/ui/components/BleedoutRing.tsx`、`NearestMedic.tsx`、`CallMedicButton.tsx`、`GiveUpButton.tsx`、`KillerInfo.tsx` —— 子组件。
- `client/src/game/downed-state.ts` —— 从 `self_state`/`Death`/`Revive` 事件派生 `downedPhase`（`alive|downed|dead|spectating`）、流血剩余、可否放弃、可否呼叫、击杀者信息（遵守显示规则，服务器仅在阵亡后下发击杀者详情，客户端不缓存倒地期的击杀者）。
- `client/src/game/spectate.ts` —— 阵亡后观察队友（M1 最小版：循环切换队友第一人称/第三人称跟随，来自 AOI 内已有实体；无实体则固定死亡镜头）。
- `client/src/render/camera/DownedCamera.ts` —— 通知接口（降饱和/暗角由 `ScreenFx` 的 `downed` 状态驱动；相机低位视野由 game/render 另有逻辑，本任务只发状态）。
- 测试：`client/test/ui/downed.test.ts`、Playwright `downed.spec.ts`。
### 要实现的行为
1. 倒地面板：标题"被击倒"、流血环（剩余秒数 = `downed_hp / 0.5`，以服务器权威数据为准）、最近医疗兵与方向箭头（无医疗兵显示"附近没有医疗兵"）、呼叫医疗（冷却 5 s 显示剩余秒数）、放弃（倒地 5 s 内禁用并显示原因）。
2. 阵亡面板：标题"阵亡"、一行击杀信息（"被 X 的 Y 击杀 · N m"）、[重新部署 (Enter)] 按钮、重生倒计时（来自服务器）；`Space` 切换观察对象。
3. 被救起：服务器 `Revive` 事件→面板在 1 帧内关闭并恢复 HUD；救起进度（Medic 靠近时显示"正在救治"）。
4. 网络断线时的倒地：显示"重连中，状态可能不同步"，禁用放弃。
5. 减少晃动模式：静态斜线纹+低饱和代替抖动（UI_SPEC §7.4）。
6. 所有状态切换在同一帧内完成（无空白帧）：面板挂载/卸载与 HUD 显隐使用同一信号批更新。
### 接口与数据
```ts
export type DownedPhase = 'alive'|'downed'|'dead'|'spectating';
export interface KillerInfo { nameKey?: string; name?: string; weapon: number; distM: number; isBot: boolean; friendly: boolean; hidden: boolean }
export const downedPhase: Signal<DownedPhase>;
```

## 不做什么
- 不做击杀回放/死亡镜头的复杂镜头动画（"回放"仅信息呈现，不做录制回放）；"回放/死亡摄像机提示"最小版。
- 不做救起、流血、归属的规则（M1-016）；不做 Medic 的救治交互 HUD（归 M1-057 的交互提示与进度环）。
- 不做观战模式的全局观战（M3/M4）、友军观察视角的完整 UI（仅最小）。
- 不做换队/举报入口（M1-061 暂停菜单 / M2-043）。

## 验收标准
1. 状态切换无空白帧：Playwright 录制 `Alive→Downed→Revived`、`Downed→Dead→Deploying` 全程每帧截图差分，未检测到全黑/空白帧（0 帧）。
2. 重生倒计时与服务器 `spawn_tick` 误差 ≤ 1 s（服务器 stub，20 次采样最大值；INDEX 指标）。
3. 倒地阶段不显示击杀者昵称与位置：UI 树中无击杀者节点（vitest 断言）；阵亡后显示昵称+武器+距离。
4. "隐藏击杀者昵称"开启时，击杀者节点只含武器与距离（vitest）。
5. 呼叫医疗冷却 5 s：4.9 s 内重复点击无效，5.1 s 后有效（fake timers）。
6. 放弃：倒地 < 5 s 禁用，≥ 5 s 按住 3 s 才触发（fake timers，数值读取规则字段）。
7. 流血环剩余时间与服务器 `downed_hp/0.5` 误差 ≤ 1 s。

## 测试要求
- 单元测试：`downedPhase` 状态机全部转移；击杀者信息可见性矩阵（阶段×bot×友军×隐藏设置）；冷却计时。
- 集成/无头测试：Playwright + 服务器 stub 脚本（倒地→被救、倒地→流血→死亡、直接死亡）。
- 性能基准：面板打开 UI 耗时 ≤ 2 ms（`?uiperf=1`）。
- 回归：M1-057 HUD 的 FX 控制器 `downed` 状态联动不破。

## 性能预算
- UI_SPEC §8.1：UI CPU ≤ 1.5 ms 均值；`will-change` 层 ≤ 6 个（§8.3）；本屏不新增大面积 `backdrop-filter`（降饱和由渲染后处理或叠加层 `opacity` 实现）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：UI_SPEC §4.10 与 GD §7.2 放弃时间差异已登记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- UI_SPEC 的放弃为"按住 1.5 s"，GAMEDESIGN §7.2 为"倒地 5 s 后、按住 3 s"：以 GAMEDESIGN 为准，读服务器规则字段。
- "击杀者信息仅阵亡后下发"需 M1-026 的 `Death`/`Kill` 事件配合；若服务器提前下发，客户端必须丢弃（信息控制）。
