# M4-025 UI：可访问性审计与修复

## 元数据
- 里程碑：M4
- 工作流：ui
- 规模：M
- 依赖：M2-041
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §7; UI_SPEC §2.2; UI_SPEC §2.6; UI_SPEC §5; UI_SPEC §8; UI_SPEC §10

## 背景
UI_SPEC §7 规定了辅助功能目标，本任务对**全部 UI 屏幕**做审计并修复，使其可验证地达标。要点：
- **色盲模式**（§7.1）：默认/红绿优化/蓝黄优化/高对比/自定义；形状冗余**始终开启**且不可关闭：友军=切角矩形、敌军=菱形、中立=方形、未知=四瓣；地图区域着色在色盲模式追加图案（我方横向细纹、敌方斜纹、中立点阵）；所有颜色编码列表另有文字/图标标签。
- **字幕与声音可视化**（§7.2）：字幕（语音转写首发**不做**，标 `未实现`）覆盖关键 UI 音效与游戏事件（命令到达、被击倒、医疗靠近、友军呼叫、爆炸/空袭预警）；字幕字号 3 档、背景不透明度 0–100%；声音可视化默认关闭，受服务器声音 AOI 限制。
- **字号与布局**（§7.3）：UI 缩放 75–200%；聊天/字幕独立字号 12–24 px；中文最小 13 px；全部 DOM 文本；辅助模式点击目标 ≥ 40×40 px；菜单类屏 100% 键盘可达；战斗 HUD 不保证读屏（诚实标注）。
- **减少晃动**（§7.4）：总开关默认遵循 `prefers-reduced-motion`；关闭屏幕震动/镜头晃动/受击抖动/过渡缩放；保留静态替代；闪烁 ≤ 3 次/秒；光敏模式。
- **对比度**（§2.2.3、§2.2.4）：`text-hi`/`text-mid` 在 `surface-1` 上 ≥ 7:1 / 4.5:1；HUD 文字依赖 scrim + 描边保证 ≥ 4.5:1；`/client/tools/` 脚本计算 WCAG 对比度并用 Machado 矩阵验证友/敌/小队两两 ΔE ≥ 25（CIEDE2000），CI 失败阻止合并。

## 具体内容
### 要创建/修改的文件
- `client/tools/a11y/contrast.ts` —— token 对比度计算（含 `hud-scrim` 叠加）+ 色盲模拟 ΔE 检查（若 M1-053 已建则扩展）。
- `client/tools/a11y/audit.spec.ts` —— Playwright + `axe-core`（devDependency；需先过依赖许可证政策 H-037，MIT/MPL 许可，写 PLAN_ISSUES 记录引入）扫描全屏。
- `client/src/ui/a11y/{ColorblindMode,Subtitles,ReduceMotion,SoundViz}.ts(x)` —— 缺失功能补齐。
- `client/src/ui/styles/tokens.a11y.css` —— 高对比 token 集。
- `docs/` **不改**；审计结果写 `reports/a11y/m4-audit.json` 与 `tasks/STATUS.md`。
### 要实现的行为
1. 审计覆盖 UI_SPEC §4 的全部屏幕（启动、主菜单、浏览器、阵营、部署、HUD、战术地图、径向菜单、记分板、倒地/阵亡、指挥官、Officer 工具、建造、载具/火炮、聊天/语音、设置、结算、暂停、toast）；每屏产出：对比度、键盘可达性、焦点环、`aria` 属性、读屏标注。
2. 修复所有"阻断级"发现（对比度 < 4.5:1、键盘陷阱、缺焦点环、颜色单独编码）。
3. 色盲模式：对每个颜色编码元素在三种色盲模拟下截图，友/敌/小队 ΔE ≥ 25。
4. 字幕：实现关键事件字幕（来自 i18n key + 事件总线）；声音可视化（可选，默认关）。
5. 减少动态：开启后无屏幕震动与脉动动画；闪烁频率检测脚本（逐帧亮度变化 ≤ 3 次/s）。
6. 战斗 HUD 读屏限制在设置页与 FAQ 中诚实标注。
### 接口与数据
- 设置键 `a11y.colorblind: 'default'|'rg'|'by'|'high_contrast'|'custom'`、`a11y.subtitle_size`、`a11y.reduce_motion`、`a11y.sound_viz`、`a11y.light_sensitive`；Schema 与 M4-024 共用 `settings.schema.json`。

## 不做什么
- 不做语音转写字幕（R10：首发不做）。
- 不重做视觉风格与图标（H-043 范畴）。
- 不保证战斗 HUD 读屏可用（诚实声明）。
- 不做新手引导（M4-027）与超宽屏（M4-026）。

## 验收标准
1. **UI 文本对比度 ≥ 4.5:1（脚本检查）**：`pnpm -C client a11y:contrast` 对全部文字 token/背景组合，最低值 ≥ 4.5；`text-hi` ≥ 7:1；HUD 文本含 scrim 后 ≥ 4.5:1；失败数 = 0。
2. **全部交互键盘可达**：Playwright 键盘脚本在每个菜单类屏（共 ≥ 15 屏）仅用 Tab/方向键/Enter/Space/Esc 完成主要任务，成功率 100%；无键盘陷阱（Tab 循环 ≤ 屏内元素数 + 1）。
3. axe-core 扫描：每屏 `serious/critical` 违规数 = 0（`minor/moderate` 登记 PLAN_ISSUES）。
4. 色盲：三种模拟下友/敌/小队两两 ΔE（CIEDE2000）≥ 25；形状冗余存在性断言（友军切角矩形等）100%。
5. 减少动态：开启后 60 s 录屏中亮度变化 > 3 次/秒的片段数 = 0；无 `transform` 缩放/抖动动画（`getAnimations()` 检查）。
6. 字幕：5 类关键事件触发字幕，延迟 ≤ 100 ms 且 3 档字号可切；文本不遮挡准星区域（中心 10% 屏幕）。
7. 中文最小字号 ≥ 13 px、点击目标辅助模式 ≥ 40×40 px（DOM 测量脚本，失败数 = 0）。

## 测试要求
- 单元测试：`vitest`：对比度函数（已知 WCAG 样例）、ΔE 函数、字幕队列。
- 集成/无头测试：Playwright 全屏遍历；`axe-core`。
- 性能基准：a11y 层 UI CPU 增量 ≤ 0.1 ms/帧。
- 回归：高对比与色盲模式视觉基线纳入 M4-040（子集）。

## 性能预算
- UI 预算 ≤ 1.5 ms 均值（UI_SPEC §8.1）；a11y 功能默认关闭时零开销。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- WCAG 闪光阈值的具体条款引用在 UI_SPEC 附录 B 标"未核实"，实现以"≤ 3 次/秒"为准。
- 引入 `axe-core` 属于新开发依赖：INDEX/ARCH 未列出，需先写 PLAN_ISSUES 并跑 `npm audit`（README §5.8）。
- UI_SPEC §10 是原创性自查清单，本审计顺带复核新增文案/图标（与 H-094 配合），不是可访问性规范。
