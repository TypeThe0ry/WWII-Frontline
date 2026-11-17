# M1-046 render：第一人称武器 pass

## 元数据
- 里程碑：M1
- 工作流：client-render
- 规模：M
- 依赖：M1-045
- 素材需求：V-WPN-US-*; V-WPN-GE-*（第一人称占位可，GAP-V06）；素材状态：placeholder
- 引用文档：GAMEDESIGN §6.10; ASSET_SHOPPING_LIST §4.0; ASSET_GAPS §3.2; ARCHITECTURE §10.1; ARCHITECTURE §13.5.2

## 背景
第一人称武器需独立 FOV 与深度，避免穿墙与近裁剪问题。D-FP：第一人称武器面数 25k–60k【待调参】（第三人称 5k–12k），M1 允许用第三人称品质/占位（GAP-V06，H-045 待人确认）。GD §6.10：机械瞄具（缺口+准星），ADS 视角缩小 1.2×，ADS 时降低 FOV；弹药信息仅显示"满/多/中/少/空"五档（UI 原创，M1-057 负责）。占位约定（ASSET_GAPS §3.2）：武器占位 = 长方体+圆柱组合，按真实长度，保留 `socket_muzzle`/`socket_ejection`/magazine 挂点，第一人称与第三人称各一个占位（同形）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/fp/FpWeaponPass.ts` —— 独立渲染层：在主场景之后用单独的相机/视锥（FOV 默认 54° 垂直【待调参】，与世界 FOV 解耦）绘制，**清除深度或使用独立深度范围**（深度偏移到近平面），保证枪不被世界几何遮挡且不穿墙视觉。
- `client/src/render/fp/WallPushback.ts` —— 贴墙回缩：每帧从相机向前射线（长度 = 枪长 + 0.1 m），命中则把枪沿后方回缩/压低，最大回缩量取武器数据的 `length_m`。
- `client/src/render/fp/FpAnimator.ts` —— 持枪摇摆（走/跑）、开火后坐视觉、装填（根据 `reload` 事件时序）、切枪、开镜（ADS 插值）；动画由事件/状态触发，无真实动画时用程序化偏移。
- `client/src/render/fp/PlaceholderWeapon.ts` —— 程序化占位枪（`PH_V-WPN-US-001__fp` 等命名），尺寸取自 `/data/weapons/*.json`（`historical.length_mm` 若缺则补字段并注来源）。
- `client/src/render/fp/sockets.ts` —— socket 查询：`muzzle`（供枪口焰/曳光起点，M1-047）、`ejection`、`magazine`。
- 测试：`client/test/render/fp.test.ts`、Playwright 贴墙截图。
### 要实现的行为
1. FP 武器使用独立 FOV 与独立深度范围；不接收世界阴影的级联抖动问题（采用简化光照：方向光 + IBL，见 M1-043）。
2. 贴墙测试：玩家紧贴墙壁时枪体不穿过墙面（回缩后枪口距墙 ≥ 5 cm）。
3. ADS：FOV 按武器数据变化（GD §6.10 的 1.2×），过渡时间读取武器 `ads_ms`；ADS 时隐藏准星由 UI 层处理，本任务只发 `ads.state`。
4. 开火：预测帧同时触发枪口焰事件（`fire.local`，见 ARCHITECTURE §8.4）与视觉后坐；视觉后坐不影响 sim 的真实后坐（sim 为权威）。
5. 装填/切枪视觉时序与 `/data` 的 `reload_ms`、`swap_in_ms` 一致。
6. 掌握 socket：真素材到位后不改代码即可读到相同挂点。
### 接口与数据
```ts
export class FpWeaponPass { setWeapon(weaponId: string): void; update(dt: number, input: FpInput): void; getSocket(name: 'muzzle'|'ejection'|'magazine'): Vec3 | null }
export interface FpInput { adsT: number; sprint: boolean; firing: boolean; reloading: boolean; stanceBlend: number; lookDelta: [number, number] }
```

## 不做什么
- 不做手臂与真实 FP 动画资产（V-CHR-FPA-001、V-ANM-009，GAP-V07/V19，由素材接入任务 M1-064 与 M2-047）。
- 不做枪口焰/弹壳的粒子本体（M1-047）、开火音（M1-063）。
- 不做瞄具叠层 UI/准星（M1-057）、狙击镜 RT（M3）。
- 不做第三人称武器挂载（M1-045 的 `socket_hand_r`）。

## 验收标准
1. 贴墙测试：相机距墙 0.1 m、正对墙面，枪体任何顶点穿入墙面深度 = 0（Playwright 截图 + 数学断言 `fp_wall_pushback`）。
2. FP pass GPU 耗时 ≤ 0.4 ms（High 参考机，1080p，GPU timestamp 叠层）。
3. 独立 FOV：改变世界 FOV 40°–110° 时，FP 武器屏幕尺寸变化 < 2%（测试 `fp_fov_independent`）。
4. 开火视觉与 `fire.local` 同帧（延迟 ≤ 1 渲染帧；事件时间戳 vs 渲染提交时间戳断言）。
5. 装填视觉时长与 `/data` 的 `reload_ms` 误差 ≤ ±1 帧。
6. 占位命名全部符合 `PH_{需求ID}__fp`，且含三个 socket（单测遍历）。

## 测试要求
- 单元测试：回缩函数、ADS 插值曲线、socket 查找、FOV 计算。
- 集成/无头测试：Playwright 固定机位贴墙/开阔两张截图，基线入库。
- 性能基准：FP pass GPU 耗时导出到 `reports/m1-fp.json`。
- 回归：M0-025 第一人称占位枪的行为迁移后保持（开火同帧）。

## 性能预算
- GPU：FP pass ≤ 0.4 ms（INDEX 验收），计入 ARCHITECTURE §13.5.2 不透明主通道 4.0 ms；CPU 并入渲染提交 4.5 ms（§13.5.1）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、视觉回归）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID（V-WPN-*、V-ANM-009、V-CHR-FPA-001）已列出
- [ ] 文档同步：`/data/weapons` 新增字段（`length_mm`、`ads_ms`）已回写 schema 并记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 武器数据的 `length_mm`、`ads_ms` 若 M0-004 schema 未含，需在 `/data/schemas` 补字段并跑 data-validate（接口细化，写 PLAN_ISSUES）。
- GAP-V06/V07（FP 高精度武器与手臂）的决定见 NEEDS_HUMAN H-045/H-047；未决时按"第三人称品质先上线"处理。
