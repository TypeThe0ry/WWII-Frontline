# M1-054 UI：启动加载与主菜单

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：M
- 前置依赖：M1-053, M1-006
- 素材需求：I-MUS-001（可选，主菜单音乐；缺则静音）；素材状态：placeholder（静音）
- 引用文档：UI_SPEC §4.1; UI_SPEC §4.2; ARCHITECTURE §13.6; ARCHITECTURE §11.4; ARCHITECTURE §10.4

## 背景
首次加载是浏览器游戏最大的流失点。UI_SPEC §4.1 把启动分三段：A）≤ 8 MB 的"壳"（JS+WASM+UI，ARCHITECTURE §11.4 core 包 ≤ 8 MB）极速进入主菜单；B）进入服务器后再下载地图资源（Medium ≤ 150 MB）；C）着色器预热（M1-042 `prewarm()`）。启动状态机（§4.1.3）：`Probing → ShellLoading → Ready`，`Unsupported`、`Failed`；地图阶段 `CheckCache → Downloading → Compiling → Warmup`。信息元素（§4.1.1）：品牌/版本、图形后端检测（WebGPU/WebGL2/不支持）、降级提示、画质自动推荐档、壳进度（10 Hz）、地图分项进度（地形/纹理/模型/音频/着色器编译，5 Hz）、缓存状态、磁盘配额提示（`navigator.storage.estimate`）、提示轮播、错误/重试面板。用户首次交互必须有"点击继续"触发点以解锁 WebAudio 与 Pointer Lock（§4.1.4）。主菜单（§4.2）：昵称+账号状态、快速加入、服务器浏览器入口、设置、推荐服务器卡片（≤ 3，来自 gateway）、公告、语言切换（EN/简体中文，立即生效）、版本号。M1 无教程与战绩（M4/M2）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/screens/Boot.tsx` —— 启动页：探测→壳加载→"点击进入"；地图加载阶段进度视图（可由 `deploy`/`browser` 后续复用）。
- `client/src/ui/screens/Menu.tsx` —— 主菜单：左栏导航（快速加入/服务器浏览器/设置/语言）、右栏推荐卡片与公告占位。
- `client/src/game/boot/capabilities.ts` —— 能力探测（`navigator.gpu.requestAdapter()`、WebGL2 探测、`adapter.info`、`deviceMemory`、`hardwareConcurrency`、`storage.estimate`；仅本地使用，不上传，ARCHITECTURE §13.6）与画质推荐（占位：M1 仅 Low/Medium/High 推荐，规则写入 `/data` 或配置）。
- `client/src/game/boot/shell-progress.ts` —— 壳资源进度（fetch 进度 → signal，10 Hz 节流）。
- `client/src/ui/screens/Unsupported.tsx` —— 无 WebGPU 且无 WebGL2：全屏不支持页，推荐 Chrome/Edge，复制诊断；不提供"强行继续"。
- `tools/ci/core-size.mjs` —— CI 门禁：`pnpm build` 后对 JS+WASM+UI 做 brotli 压缩并求和 ≤ 8 MB（8 388 608 B）。
- 测试：`client/test/ui/boot.test.ts`、Playwright `boot.spec.ts`。
### 要实现的行为
1. 启动状态机与 UI_SPEC §4.1.3 一致；无 WebGPU 有 WebGL2 → 降级面板，列出被禁用特性（体积雾→高度雾、TAAU→空间放大、SSR 关闭）；无任何后端 → `Unsupported`。
2. 壳加载失败：自动退避重试 1/2/4/8 s，显示失败资源名，"复制诊断信息"按钮。
3. 着色器预热：调用 M1-042 `prewarm()`，进度项"着色器编译"在地图阶段显示；预热完成才允许进入部署界面。
4. 主菜单：`↑/↓` 移动、`Enter` 激活；推荐服务器 3 张骨架卡→gateway 数据（M1-028，失败显示"无法获取服务器列表"+重试）；语言切换立即生效。
5. 点击继续：首次用户手势用于创建/恢复 `AudioContext`（交给 M1-062）。
6. 所有文案走 i18n（`ui.boot.*`、`ui.menu.*`）。
### 接口与数据
```ts
export type BootPhase = 'probing'|'shell_loading'|'ready'|'unsupported'|'failed';
export interface Capabilities { backend: 'webgpu'|'webgl2'|'none'; gpuName?: string; deviceMemoryGb?: number; cores: number; storageQuotaMb?: number; recommendedTier: 'low'|'medium'|'high' }
export const bootPhase: Signal<BootPhase>;
```

## 不做什么
- 不做服务器浏览器/阵营选择（M1-055）、设置全页（M1-061 基础、M2-041 完整）、账号面板与 OAuth（M4）、教程/战绩（M4-027/M2）。
- 不做地图资源下载逻辑本身（M1-006/M1-049），只展示其进度。
- 不做画质四档完整自动推荐与 5 s 基准（M4）。
- 不做主菜单背景动画/3D（只用静态 Plotline 线稿背景）。

## 验收标准
1. core 包（JS+WASM+UI，brotli）≤ 8 MB：`node tools/ci/core-size.mjs` 通过（CI 门禁；INDEX 指标）。
2. 首屏可交互 ≤ 3 s（100 Mbps，缓存冷启动；Playwright 限速 `Network.emulateNetworkConditions` 与 `performance.now()` 计时，主菜单按钮可点击）。
3. 二次访问（缓存命中）主菜单可点击 ≤ 2 s（UI_SPEC §8.1）。
4. 强制 WebGL2 的测试配置下，降级面板显示且列出 ≥ 3 项被禁用特性（Playwright 断言 i18n key）。
5. 模拟全部后端不可用时进入 `Unsupported` 页，无"继续"按钮（Playwright）。
6. 网络失败注入：重试间隔序列 1/2/4/8 s ±200 ms（vitest fake timers）。
7. en/zh-Hans 两种语言下启动页与主菜单键全部存在（`i18n:check` 通过）。

## 测试要求
- 单元测试：能力探测分支（mock `navigator.gpu`）；启动状态机转移；退避序列。
- 集成/无头测试：Playwright 三场景（WebGPU / 强制 WebGL2 / 不支持）。
- 性能基准：首屏时间与 core 包体积写入 `reports/m1-boot.json`。
- 回归：M1-053 组件快照；core 包门禁纳入 CI 常驻。

## 性能预算
- ARCHITECTURE §11.4：core 包 ≤ 8 MB；UI_SPEC §8.1：二次访问首次交互 ≤ 2 s，首次访问 ≤ 6 s（4G 估算）；本任务验收 ≤ 3 s（100 Mbps）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright、CI `core-size`）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID（I-MUS-001）已列出
- [ ] 文档同步：画质推荐规则写 PLAN_ISSUES"细化"
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `adapter.info` 在各浏览器的可得性未核实（UI_SPEC 附录 B-2）；缺失时回退到 `deviceMemory` + 核数推荐。
- core 包预算为全项目硬指标，Preact/WASM 体积增长要持续监控；超 1% 门禁失败（见 M1-005）。
