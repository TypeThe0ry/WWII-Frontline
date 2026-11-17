# M1-053 UI：框架、设计 token 与 i18n

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：L
- 前置依赖：M0-001, M0-003
- 素材需求：V-FNT-*（OFL 字体，见 LICENSES.md）；素材状态：real（字体到位且 OK）/ placeholder（系统无衬线 + SVG，ASSET_GAPS §3.2）
- 引用文档：UI_SPEC §2; UI_SPEC §8; UI_SPEC §9; UI_SPEC §0.3; ARCHITECTURE §2; ARCHITECTURE §4.3.9

## 背景
M1 的全部 UI 屏幕（M1-054..061）共用一套外壳：Preact + `@preact/signals`、Plotline 设计 token、基础组件、i18n、字体加载与 DOM 覆盖层/Canvas 小画布工具。UI_SPEC §0.3：UI = HTML/CSS DOM 覆盖层 + Preact + 信号状态；战术地图 = Canvas2D；语言 en + zh-Hans（A7），文案全走 i18n key，禁止硬编码字符串；基准 1920×1080，适配 1280×720–3440×1440。ARCHITECTURE §4.3.9：`ui` 只依赖 `game` 只读状态（signals 桥）与事件总线，不 import `render` 内部或 `net`。
Plotline token（UI_SPEC §2.2.1）：`--pl-bg-0 #0A1018`、`--pl-bg-1 #101A25`、`--pl-surface-1/2/3`、`--pl-accent #E4CC8A`、`--pl-friend #3DA5FF`、`--pl-enemy #FF7A2F`、`--pl-squad #B58CFF`、`--pl-warn #FFD33D`、`--pl-danger #E5326E`、`--pl-ok #52D6A0`、`--pl-cmd #7FE0E8` 等；间距基础单位 4 px（`--pl-s1..s10`）；字号阶梯 `--pl-fs-xs 12 … hero 64`；z-index 约定 `world(0) < hud(10) < hud-fx(20) < map(30) < panel(40) < modal(50) < toast(60) < tooltip(70) < pointer-lock-overlay(80)`（§2.4）。字体（§2.3）：Barlow Semi Condensed / IBM Plex Sans / IBM Plex Mono / Noto Sans SC，自托管 WOFF2，`font-display: swap`，CJK 按 unicode-range 分 ≥ 4 片并子集化，**许可证需先由 M0-003 核实登记**。UI 预算（§8.1）：UI 总 CPU ≤ 1.5 ms 均值/3 ms P95，战斗态 HUD DOM ≤ 350 节点，菜单 ≤ 1 500。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/main.tsx`、`client/src/ui/root.tsx` —— UI 根（`position: fixed; contain: layout paint`）、屏幕路由（`boot | menu | browser | faction | deploy | hud | map | pause …`）、`ErrorBoundary`（每个顶层屏一个）。
- `client/src/ui/tokens/plotline.css` —— §2.2/2.4 全部 CSS 变量；`client/src/ui/tokens/a11y-variants.css` —— 色盲/高对比变体占位（完整实现 M2-041）。
- `client/src/ui/components/*` —— §9.1 基础组件：`Panel`、`Button`（含 `reason` 禁用提示）、`KeyCap`、`Icon`（SVG sprite `<use>`）、`Frame`、`ProgressRing/Bar`、`Gauge`、`Tabs`、`Select/Slider/Toggle/TextField`、`Tooltip`、`Modal`、`VirtualList`、`Skeleton`、`FocusTrap`。
- `client/src/ui/i18n/index.ts` —— `t(key, params)`、ICU MessageFormat（复数）、运行时语言切换（无需重载）；`client/src/ui/i18n/en.json`、`zh-Hans.json`；`tools/i18n-check/` —— CI 缺键/多余键/占位符不一致校验。
- `client/src/ui/fonts/loader.ts` —— 字体加载、`data-font-fallback`、CJK 分片按需；`tools/font-subset/` —— 子集化脚本（词表由 i18n 自动抽取，输出 WOFF2 到 fpak 外的静态资源，按 UI_SPEC §2.3 与 M0-003 的流程说明）。
- `client/src/ui/canvas/CanvasLayer.ts` —— Canvas 小画布工具（DPR 处理、离屏缓存、`Path2D` 缓存、页面不可见时暂停）。
- `client/src/ui/uiperf.ts` —— `?uiperf=1` 叠层：UI 脚本耗时、DOM 节点数。
- `client/src/ui/bridge.ts` —— 从 `game` 的 `GameState`（M1-040）取只读 signals，不直接 import 其内部。
### 要实现的行为
1. i18n：所有可见文案经 `t()`；缺键开发时显示 `⟦key⟧` 并在 CI 失败；en 与 zh-Hans 键集合必须完全一致。
2. 字体：加载失败回退系统无衬线并设 `data-font-fallback`；中文字体按需分片加载，首屏仅拉当前语言需要的分片。
3. 组件视觉回归：两种语言 × 两种缩放（100%/150%）快照；键盘可达性测试（Tab 顺序、`FocusTrap`）。
4. 缩放：`--pl-ui-scale` 随视口计算（UI_SPEC §6.1），DOM 不依赖浏览器页面缩放。
5. 事件：UI 写操作只通过 RPC/事件接口（M1-040/M1-026），UI 组件内不得直接访问 `net`。
### 接口与数据
```ts
export function t(key: string, params?: Record<string, string|number>): string;
export type Screen = 'boot'|'menu'|'browser'|'faction'|'deploy'|'hud'|'map'|'pause'|'downed';
export const uiScreen: Signal<Screen>;
```

## 不做什么
- 不做任何具体屏幕（M1-054..061）、记分板/径向菜单/聊天（M2）。
- 不做完整无障碍（色盲/字幕/字号，M2-041）；只留 token 占位。
- 不做真图标绘制全集：本任务提供 `Icon` 与 SVG sprite 管线，M1 所需图标由各屏任务按 UI_SPEC §2.5 原创绘制（H-003 待确认）。
- 不购买/嵌入未登记字体（须 M0-003 登记为 OK）。

## 验收标准
1. i18n 缺键 CI 失败：删除一个 zh-Hans 键后 `pnpm -C client run i18n:check` 退出码 ≠ 0；恢复后为 0。
2. en/zh-Hans 键集合差集 = 0（CI 步骤 `i18n:check` 通过）。
3. Preact 全应用更新（模拟 100 个 signal 变更/帧）≤ 0.8 ms/帧（vitest bench + Playwright `?uiperf=1`；INDEX 指标）。
4. 战斗态 DOM 节点 ≤ 350（本任务提供空 HUD 容器基线 ≤ 20；脚本 `querySelectorAll('*').length`）。
5. 子集化后 zh-Hans 字体首屏所需分片总计 ≤ 400 KB（br；UI_SPEC 附录 B-5 预算，实测写 STATUS；不可达写 PLAN_ISSUES）。
6. 每个基础组件有视觉回归快照（2 语言 × 2 缩放）与键盘可达性测试，全部通过。
7. `ui` 目录无对 `net`/`render` 内部的 import（eslint `no-restricted-imports` 通过）。

## 测试要求
- 单元测试：`t()` 插值/复数；字体加载回退；`VirtualList` 窗口计算；`FocusTrap`。
- 集成/无头测试：Playwright 加载组件画廊页（`/ui-gallery`，仅开发构建）并截图。
- 性能基准：UI 更新微基准入 `reports/m1-ui-framework.json`。
- 回归：M0 的 client 构建与 core 包体积不增超 M1-054 门禁（≤ 8 MB）。

## 性能预算
- UI_SPEC §8.1：UI 总 CPU ≤ 1.5 ms 均值、≤ 3 ms P95（High）；ARCHITECTURE §13.5.1 UI ≤ 0.8 ms/帧；core 包 ≤ 8 MB（§11.4，由 M1-054 门禁）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、CI `i18n:check`）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID（V-FNT-*）已列出
- [ ] 文档同步：token/组件 props 如与 UI_SPEC §9 不同，写 PLAN_ISSUES"细化"
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 字体许可证未核实（UI_SPEC 附录 B-1，M0-003 负责）；未登记 OK 时本任务用系统字体占位，`M1-065` 再接入真字体。NEEDS_HUMAN H-036 确认字体方案。
- 规模 L：基础组件 + i18n + 字体 + canvas 工具较多，必要时把"字体子集化工具"拆为 `M1-053b`。
- 图标/字形原创绘制边界见 NEEDS_HUMAN H-003；UI 风格定稿见 H-043。
