# M4-027 UI：新手引导

## 元数据
- 里程碑：M4
- 工作流：ui
- 规模：M
- 依赖：M2-043（投票/toast/系统提示）, M1-056（部署界面）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.19; UI_SPEC §4.2; GAMEDESIGN §2; GAMEDESIGN §2.5

## 背景
UI_SPEC §4.19 定义系统提示通道：教程提示位于**底部中，靠近相关元素**，持续"满足条件前或 8 s"，优先级中；**每条仅触发一次**，记录在 `localStorage`（失败则按会话计）；可在设置中全部关闭、可"重置教程"；所有提示 `aria-live="polite"`；`Notice.kind='tutorial'`，i18n 前缀 `ui.tutorial.*`；规则：同一时刻最多 1 个横幅、3 个 toast，相同内容 3 s 内合并。主菜单（§4.2）有"教程/训练场入口，首次登录高亮"，并含 `TutorialPrompt` 状态。
引导覆盖的玩法流程（GAMEDESIGN §2 状态机）：**移动 → 射击 → 部署（§2.5 四步：小队→兵种→loadout→重生点）→ 小队 → 占点**。要求：
- 战斗中文案克制：≤ 3 个词的短语；解释性文字只放部署界面、设置与教程（UI_SPEC 设计原则 8）。
- 不遮挡准星（屏幕中心 10%），不阻断输入。
- 对新玩家（无 `tutorial.done`）首局触发；可跳过、可在设置重开。
本任务不创建训练场地图（Backlog）；引导在**真实对局**里以非阻断提示与可选的"首局高亮"完成。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/tutorial/TutorialEngine.ts` —— 步骤状态机：条件监听（事件总线）→ 显示提示 → 条件满足/超时 → 下一步。
- `client/src/ui/tutorial/steps.ts` + `data/ui/tutorial_steps.json` —— 步骤定义（`id`、触发条件、完成条件、`i18nKey`、锚定元素、`timeout_ms`）。
- `client/src/ui/tutorial/TutorialHint.tsx` —— 提示渲染（底部中，指向锚点的箭头，遵守减少动态）。
- `client/src/ui/settings/TutorialSettings.tsx` —— 开关/重置按钮。
- `client/src/i18n/{en,zh}/tutorial.json`。
- `client/tests/ui/tutorial.spec.ts`、`tests/e2e/tutorial_first_match.spec.ts`。
### 要实现的行为
1. 步骤序列（初值）：①移动（WASD/鼠标观察）②射击（左键、R 装填）③打开部署/选择重生点（部署界面分步高亮）④加入小队⑤占点（进入据点区域并持续）；每步 ≤ 2 句。
2. 条件检测：基于事件总线（`move.distance`、`fire`、`deploy.confirmed`、`squad.joined`、`objective.progress`），不轮询。
3. 一次性触发 + 持久化（`localStorage` try/catch）；"重置教程"清除；"关闭全部提示"开关。
4. 跳过：任意提示可 `Esc`/点击"跳过"；"跳过全部"写 `tutorial.done=true`。
5. 与其它通知排队：教程提示优先级为中，不抢占高优先级横幅（UI_SPEC §4.19.1 规则 1）。
6. 不遮挡准星：提示位于底部中靠近元素，且与屏幕中心 10% 区域无交集（布局断言）。
7. 灰盒：引导在 bot 对局（Skirmish 小规模）中最容易触发完成；服务器无需特殊支持。
### 接口与数据
- `interface TutorialStep { id: string; trigger: EventCond; done: EventCond; i18nKey: string; anchor?: string; timeoutMs?: number; once: true }`；`Notice` 复用 §4.19.2 结构，`kind:'tutorial'`。

## 不做什么
- 不做独立训练场/教学地图。
- 不做语音/动画教学视频（素材缺口）。
- 不做可访问性审计（M4-025）；不做遥测。
- 不修改部署界面主结构（M1-056）。

## 验收标准
1. **新玩家首局 5 分钟内完成部署 + 占点引导**：e2e（`headless-client` 辅助 bot 对局，Skirmish）从进入到 `tutorial.step.capture.done` ≤ 300 s，完成率 100%（脚本模拟新玩家 10 次）。
2. **提示不遮挡准星**：全部教程提示的包围盒与屏幕中心 10% 区域相交面积 = 0（1280×720、1920×1080、3440×1440 各检查）。
3. 一次性：同一步骤重复触发条件，提示只出现 1 次（单测）；刷新页面后不再出现（localStorage），`localStorage` 不可用时按会话计（单测）。
4. "跳过全部"后无任何教程提示；"重置教程"后下一局重新出现（e2e）。
5. 所有步骤文案 en/zh 完整；英文 ≤ 14 词/步、中文 ≤ 28 字/步（脚本检查）；`aria-live="polite"` 存在。
6. 教程提示 DOM 开销：存在时 HUD 节点增量 ≤ 10；UI CPU 增量 ≤ 0.1 ms/帧。

## 测试要求
- 单元测试：`vitest`：步骤状态机、条件匹配、一次性与持久化、队列优先级。
- 集成/无头测试：Playwright 首局流程；与 `headless-client` 的 bot 对局联动。
- 性能基准：提示显示期间 UI CPU 测量。
- 回归：教程提示视觉基线（en/zh，两个分辨率）。

## 性能预算
- UI 预算 ≤ 1.5 ms 均值（UI_SPEC §8.1）；HUD DOM ≤ 350 节点（High）/ ≤ 500（Low）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 键位文案必须读取实际绑定（UI_SPEC §5），不得写死默认键。
- 引导文案不得出现参考作品名称；术语遵守统一命名（Garrison/Outpost 等）。
- 缺少训练场意味着首局体验依赖服务器上有 bot 与真人混合；匹配（M4-031）应优先给新玩家小规模对局（建议，需 Fable 决定，写 PLAN_ISSUES）。
