# M2-037 UI：径向菜单与快捷指令

## 元数据
- 里程碑：M2
- 工作流：ui
- 规模：M
- 前置依赖：M1-057, M2-019
- 素材需求：无（灰盒即可）；素材状态：n.a.（图标由执行者按 UI_SPEC §2.5 以 SVG 原创绘制）
- 引用文档：UI_SPEC §4.8; UI_SPEC §4.7.3; UI_SPEC §5.2; UI_SPEC §8.2; GAMEDESIGN §16.3; GAMEDESIGN §16.4

## 背景
UI_SPEC §4.8：战斗中无需打开地图即可发 Ping、快捷语句、建造。轮盘按住键呼出（保持 Pointer Lock，鼠标移动驱动虚拟光标，松键即确认，死区 20%，80 ms 出现动画）。轮盘定义（§4.8.1）：Ping 轮盘按住 `Mouse3` ≥ 180 ms（短按 = Attention）；语音快捷短语轮盘按住 `T`（Yes/No/Thanks/Need Help/Moving/Covering/Reloading/Sorry）；建造轮盘按住 `Y`（内容由 M2-039 提供）；指挥官快捷轮盘按住 `K`（M3-033）。禁用扇区显示原因，选择无效并发"错误"音；键盘替代 `1–8` 直接选扇区；减少动态下仅 `opacity` 动画。
Ping/标记规则以 sim 为准（M2-019 `sim-comms-markers`）：权限、类型、冷却、数量上限由服务器判定，客户端只做提示（GAMEDESIGN §16.3）。M1-057 提供 HUD 框架与信号状态。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/radial/RadialMenu.tsx` —— 通用径向菜单组件（Preact）：扇区数 6/8、死区、方向选择、禁用态与原因、中心说明、刻度尺弧样式（Plotline，UI_SPEC §2）；输入为 `RadialSpec`。
- `client/src/ui/radial/radial_controller.ts` —— 呼出/确认/取消状态机（UI_SPEC §4.8.4）；Pointer Lock 期间用虚拟光标；呼出时视角锁定（可选设置：30% 灵敏度）。
- `client/src/ui/radial/specs/ping_wheel.ts`、`phrase_wheel.ts` —— Ping 与快捷语句扇区定义（扇区与权限、冷却从 `/data/comms.json` 与 M2-019 的服务端状态读取）。
- `client/src/game/comms_client.ts` —— 发送 `PlacePing{type,pos|target}` / `QuickPhrase{id}` 消息（M2-007 定义的消息）；接收冷却/拒绝事件。
- `client/src/ui/hud/PingMarkers.tsx` —— 世界内 Ping 标识（指南针刻度 + 地面标柱，距离标签）与冷却提示（与服务器一致）。
- `client/src/i18n/{en,zh-CN}/radial.json` —— `ui.radial.*` 文案键（A7 英文 + 简体中文）。
- `client/src/ui/icons/radial/*.svg` —— 原创 SVG 图标（切角/刻度风格，UI_SPEC §2.5）。
### 要实现的行为
1. `Mouse3` 按住 ≥ 180 ms 打开 Ping 轮盘，短按（< 180 ms）直接发 Attention（准星指向敌方实体时由服务端自动判为 Enemy，GAMEDESIGN §16.4）。
2. `T` 按住打开快捷短语轮盘；语音关闭时仍可用文字短语（发送到 Squad 文字频道）。
3. `Y`、`K` 轮盘：本任务提供注册机制 `registerRadial(id, spec)`；建造扇区内容由 M2-039 注册，指挥官轮盘由 M3-033 注册；未注册时按键不呼出（toast"当前无可用快捷指令"）。
4. 冷却与上限：扇区灰置并显示剩余秒数/数量；冷却值来自服务器事件，客户端不硬编码。
5. 选择确认发送后立即在轮盘上给出"已发送/被拒绝（原因）"反馈；被拒绝原因文案走 i18n。
6. 无障碍：键盘 `1–8` 选择；`prefers-reduced-motion` 时无缩放动画。
### 接口与数据
```ts
export interface RadialSector { id: string; labelKey: string; icon: string; enabled: boolean; reasonKey?: string; cooldownMs?: number; onConfirm(): void }
export interface RadialSpec { id: 'ping' | 'phrase' | 'build' | 'command'; trigger: string; holdMs: number; sectors: RadialSector[] }
```

## 不做什么
- 不做建造轮盘的具体蓝图与放置流程（M2-039）；指挥官能力轮盘（M3-033）。
- 不做地图上放置标记的工具栏（M1-058/M2-044 的 Frontline 叠层之外的标记交互已在 M1-058）。
- 不做 Ping 语音台词包（素材未采购，不自制配音）；Ping 音效缺失静音。

## 验收标准
1. 从按键保持达到阈值（180 ms）到轮盘首帧可见 ≤ 200 ms（Playwright `performance.mark` 采样 50 次，P95）；松键到发出消息 ≤ 1 帧（≤ 17 ms）。
2. 扇区命中测试：8 扇区 × 角度步进 1°，死区内无选中、扇区边界选择与几何一致（vitest，360 例）。
3. Ping 冷却：连续发送 3 次，第 2 次因冷却被拒，客户端冷却显示与服务器事件的剩余时间偏差 ≤ 100 ms（headless 联调 `test-ping-cooldown`，数值取 `/data/comms.json`）。
4. 禁用扇区点选不触发 `onConfirm` 且播放"错误"音事件（单测）；`1–8` 键盘选择等价。
5. 动画/DOM：轮盘 DOM 节点 ≤ 60、仅动画 `transform/opacity`（UI_SPEC §8.3 脚本检查）；打开期间脚本耗时 P95 ≤ 1.5 ms/帧。
6. i18n：`en`/`zh-CN` 键完整性测试通过（缺键即失败）。

## 测试要求
- 单元测试：状态机转换全覆盖（含 Esc 取消、中心松键取消）；扇区几何；权限/冷却灰置逻辑。
- 集成/无头测试：Playwright 模拟 Mouse3/T 按住；headless 连入测试服验证 `PlacePing` 往返与拒绝原因。
- 性能基准：UI 性能采集（`?uiperf=1`，UI_SPEC §8.5）。
- 回归：HUD 战斗态节点数仍 ≤ 350（UI_SPEC §8.1）。

## 性能预算
- UI_SPEC §8.1：UI 总 CPU ≤ 1.5 ms 均值/3 ms P95；HUD 节点 ≤ 350；不得使 ARCH §13.5.1 的 UI 0.8 ms 预算回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **文档冲突**：UI_SPEC §4.8.1 的 Ping 轮盘扇区（Attention/Enemy Infantry/Enemy Vehicle/Enemy Structure/Request Medic/Request Ammo/Request Engineer/Cancel）与 GAMEDESIGN §16.4 的 8 项（Attention/Enemy/Need Medic/Ammo/Engineer/AT/Supplies/Thanks）不一致；标记生命周期（UI_SPEC §4.7.3 如 Attention 15 s，GAMEDESIGN §16.3 为 8 s）也不同。已裁定：标记类型、权限、持续时间以 GAMEDESIGN §16.3/§16.4 为准（UI_SPEC §4.8.1 已改为引用 GD，只定义呈现）；UI 动态读取 M2-019 落地的 `/data/comms.json`。
- Mouse3 在部分浏览器/鼠标上可能触发自动滚动；需 `preventDefault`（UI_SPEC §5.5）。
- 图标为原创 SVG（H-003 待人确认）。
