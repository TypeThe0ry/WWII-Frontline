# M2-041 UI：完整设置与可访问性基础

## 元数据
- 里程碑：M2
- 工作流：ui
- 规模：L
- 依赖：M1-061
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.16; UI_SPEC §7; UI_SPEC §5.4; UI_SPEC §5.3; UI_SPEC §2.2.3; UI_SPEC §6.1

## 背景
M1-061 提供暂停菜单与基础设置。UI_SPEC §4.16 的完整设置分类：画质、画质细项、音频、控制、语音、界面、辅助功能、语言、隐私、网络/诊断；左侧分类栏 + 右侧表单，每项有默认值/还原小按钮，顶部搜索，画质类变更需"应用/还原"（10 s 无操作自动还原，状态机 Viewing→Editing→Previewing→Applied/Reverted→Rebinding）；不可用选项灰置并显示原因；本地存储不可用时设置仅会话内有效并提示（try/catch）。
重绑定（§5.4）：全部游戏动作可重绑定（除 `Esc`；投票键 `F1/F2` 可改但仅限 F 键区）；主键 + 副键；重置此项/全部；3 个键位方案槽（本地存储）；导出/导入 JSON；冲突实时检测（§5.3：同上下文冲突红框 + ⚠ 形，提供"互换/解除对方/取消"）；保留键不可绑定（`Esc`、`F11`、`F12`、Win/Meta、PrintScreen、`Ctrl+W/T/N/Q/Tab`）；用 `KeyboardEvent.code` 存储，显示用 `navigator.keyboard.getLayoutMap()`（Chromium，未核实其它浏览器）。
可访问性（§7）：色盲模式（默认/红绿/蓝黄/高对比/自定义，§2.2.3 运行时替换 token）；形状冗余始终开启且不可关闭（友军切角矩形、敌军菱形、中立方形、未知四瓣）；字幕（字号 3 档、背景不透明度 0–100%、说话者色标；首发不做语音转写）；UI 缩放 75–200%、聊天/字幕字号 12–24 px、中文最小 13 px；"减少动态"（默认遵循 `prefers-reduced-motion`）；闪光限制（每秒闪烁 ≤ 3 次，光敏模式）；按住/切换替代；长按时长系数 ×0.5–×2。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/settings/SettingsScreen.tsx`、`SettingsNav.tsx`、`SettingRow.tsx`、`SettingsSearch.tsx` —— 框架与搜索。
- `client/src/ui/settings/pages/{Graphics,GraphicsAdvanced,Audio,Controls,Voice,Interface,Accessibility,Language,Privacy,Network}.tsx` —— 各分类页。
- `client/src/settings/settings_store.ts` —— 带 schema 版本的设置存储（`localStorage` try/catch、内存兜底），迁移函数；提供 `GraphicsSettings` 订阅接口：渲染模块（M2-030/031/032 等）注册自己可用的选项与能力，未注册项灰置并显示原因。
- `client/src/input/keybinds.ts`（修改）—— 上下文化绑定表（`OnFoot/InVehicle/Gunner/Downed/Map/Menu/Chat`）、冲突检测、保留键拒绝、主/副键、3 个方案槽、JSON 导入导出（导入需 schema 校验，非法即拒绝）。
- `client/src/ui/theme/colorblind.ts` —— 色盲 token 替换（UI_SPEC §2.2.3）与图案叠加开关（地图着色图案：我方横向细纹、敌方斜纹、中立点阵，供 M2-044 使用）。
- `client/src/ui/a11y/subtitles.ts`、`reduce_motion.ts`、`flash_limiter.ts` —— 字幕框架（事件 → 文字/图标：命令到达、被击倒、医疗靠近、友军呼叫、爆炸/空袭预警）、减少动态总开关、闪光限制。
- `client/src/settings/sync.ts` —— 登录用户的设置同步接口 `SettingsSync { pull(); push() }`（实现对接 M2-022；本任务只留接口与失败 toast，未登录/失败时本地仍生效）。
- `client/src/i18n/{en,zh-CN}/settings.json` —— `ui.settings.*`（含 `.gfx/.audio/.controls/.voice/.a11y/.lang`）。
### 要实现的行为
1. 全部设置项读写、默认值、"还原此项/全部"；画质类变更进入 Previewing，10 s 倒计时无确认则自动还原。
2. 重绑定：点击键位进入 Rebinding，按键确认/`Esc` 取消；冲突提示三选项；保留键直接拒绝并说明原因。
3. 辅助功能项全部即时生效（字号、缩放、色盲、减少动态、字幕样式预览）；`prefers-reduced-motion` 作为默认。
4. 语言：English / 简体中文运行时切换，无需重载（A7）；字体分片按需加载（依赖 M0-003 的字体登记）。
5. 本地存储不可用（隐私模式/配额）：整页顶部提示"设置仅在本次会话有效"，功能不受影响。
6. 能力探测结果驱动灰置：WebGL2 下"froxel 体积雾"项显示"WebGL2 不支持"，不隐藏。
### 接口与数据
```ts
export interface SettingsSchema { version: number; graphics: {...}; audio: {...}; controls: { binds: Record<ActionId, [KeyCode?, KeyCode?]>; slot: 0|1|2 }; a11y: { colorMode: 'default'|'rg'|'by'|'hc'|'custom'; uiScale: number; reduceMotion: boolean; subtitles: { size: 0|1|2; bgOpacity: number } } }
export function registerGraphicsOption(id: string, spec: { available: boolean; reasonKey?: string; apply(v: unknown): void }): void;
```

## 不做什么
- 不做四档画质预设表与预设热切换、基准向导（M4-017、M4-024）；M2 的画质页只暴露已落地的开关（AO/雾/超分/渲染比例）。
- 不做完整可访问性审计与对比度脚本修复（M4-025）；不做超宽屏测试矩阵（M4-026）。
- 不做语音转写字幕；不做手柄绑定（A4）。
- 不做 gateway 侧设置持久化 API（M2-022）。

## 验收标准
1. 持久化：修改 ≥ 30 个设置项（脚本枚举 schema 全部叶子），刷新页面后 100% 还原到所改值；`localStorage` 禁用（Playwright 模拟）时无异常、显示"仅本次会话有效"提示。
2. 重绑定：单测覆盖冲突检测（同上下文冲突 100% 检出；跨上下文复用不误报）、保留键拒绝（`Esc`、`F11`、`F12`、`Meta`、`PrintScreen`）、副键、方案槽切换、导出再导入后绑定表深度相等；非法 JSON 导入被拒绝且原绑定不变。
3. 色盲模式覆盖：在 default/rg/by/hc 4 种模式下，据点（友/敌/中立）、队友、敌方标识的 token 全部有替换值且与形状冗余同在（单测遍历标识清单；截图基线 4 张）。
4. 对比度：4 种模式下 UI_SPEC §2.2.4 规定的前景/背景 token 对比 ≥ 4.5:1（脚本 `tools/contrast-check.ts` 输出报告，未达标项列入 PLAN_ISSUES）。
5. UI 缩放 75%/100%/200% 下设置页无水平滚动、HUD 不越出安全框（Playwright 3 档截图）；中文文本最小渲染字号 ≥ 13 px（脚本检查）。
6. 减少动态：开启后 `transform` 过渡/脉动/抖动类 CSS 动画计数 = 0（脚本遍历 `getAnimations()`）。
7. 画质类变更：10 s 无确认自动还原（用假定时器 vitest）；还原后渲染模块收到 `apply(旧值)`。
8. 设置页打开 DOM 节点 ≤ 1 500（UI_SPEC §8.1）。

## 测试要求
- 单元测试：store 迁移、schema 校验、键位冲突/导入导出、色盲 token 映射、闪光限制（> 3 次/s 被抑制）、i18n 完整性。
- 集成/无头测试：Playwright 全页面遍历、键盘仅操作（Tab/方向键/Enter/Space/Esc）可完成修改一项设置。
- 性能基准：`?uiperf=1` 记录设置页打开耗时 ≤ 100 ms。
- 回归：M1-061 基础设置用例；快捷键默认表与 UI_SPEC §5.2 一致（快照测试）。

## 性能预算
- UI_SPEC §8.1：菜单/设置界面 DOM ≤ 1 500；UI 内存 ≤ 60 MB（High）；设置修改引起的重排 ≤ 1 次/修改。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：若会话内无法完成全部 10 个分类页，优先顺序为 控制（重绑定）→ 辅助功能 → 画质 → 音频/语音/语言，剩余按 README §5.6 以 `M2-041b` 续作。
- 色盲/对比度脚本依赖 UI_SPEC §2.2.4 的自动校验要求；若 M1-053 已有 token 校验，复用而非重写。
- 字体与 CJK 子集许可证见 H-036；图标/插画为原创 SVG（H-003）。
