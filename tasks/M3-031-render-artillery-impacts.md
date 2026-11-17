# M3-031 render：炮击落点与战场环境表现

## 元数据
- 里程碑：M3
- 工作流：client-render
- 规模：M
- 前置依赖：M2-033, M3-018
- 素材需求：V-FX-*（灰盒可：引擎内置简单粒子）；素材状态：placeholder
- 引用文档：GAMEDESIGN §15.5; GAMEDESIGN §15.6; GAMEDESIGN §14.2; ARCHITECTURE §13.5.3; ARCHITECTURE §13.5.2; ASSET_GAPS §3.2

## 背景
M3-018 在 sim 中实现了榴弹炮阵地（弹道、散布、烟雾云团），M2-033 提供了基础爆炸/FX 系统。本任务把炮击的"战场效果"做足：落点爆炸、尘柱、弹坑贴花、远处炮击闪光、烟雾遮蔽。数字（GD §15.5）：HE 对步兵 `Dmax 600 / R 14 m`（致命半径约 6 m）；压制近失半径 25 m；烟雾云团半径 18 m、持续 60 s（坦克烟雾弹为 12 m、40 s，GD §14.2）。ARCH §13.5.3：主视图粒子活跃 ≤ 1 600（High）；GPU 透明/粒子 pass 预算 0.7 ms（§13.5.2）。特效材质按 ASSET_GAPS §3.2：引擎内置纯色方片/球粒子，不引入外部贴图。

## 具体内容
### 要创建/修改的文件
- `client/src/render/fx/ArtilleryImpact.ts` —— HE 弹着：闪光（1–2 帧）、火球、冲击环、飞溅碎石、尘柱（上升 3–4 s）。尺寸取自 `/data` 爆炸半径（按 R 缩放），不硬编码。
- `client/src/render/fx/CraterDecals.ts` —— 弹坑贴花池（环形缓冲，固定上限 `max_craters`，默认 64【待调参】，最老先回收；贴花直径随 R）。
- `client/src/render/fx/SmokeScreen.ts` —— 烟雾云团（半径 18 m / 12 m，持续 60 s / 40 s，最后 5 s 淡出）；云团体积用 ≤ 40 个大粒子 + 地面软圆；与 sim 云团事件 id 对应（开始/结束由事件驱动）。
- `client/src/render/fx/DistantFlash.ts` —— 地平线远处炮击闪光（距离 > 400 m 的 `Shot`/弹着事件，仅亮度脉冲 + 方向）；己方炮口焰 3 s 内可见、敌方不显示炮位标记（标记在地图 UI）。
- `client/src/render/fx/FxBudget.ts` —— 同帧特效预算器：粒子活跃上限、按距离/重要性降级。
- `client/tests/fx/*.spec.ts`
### 要实现的行为
1. 订阅 M3-005 的炮兵弹着事件与烟雾事件，按事件位置/半径播放；高角射击的下落轨迹不渲染（事件只含弹着点）。
2. 贴花投影到地形（法线对齐），使用地形高度采样，不穿插建筑；弹坑贴花在水体/道路上降级为不显示或浅贴花。
3. 烟雾遮蔽：云团内外视觉不透明度随摄像机位置变化；视觉遮蔽的**权威判定在 sim**，渲染只表现。
4. 同屏爆炸多时：按 `FxBudget` 降低远处爆炸的粒子数（> 150 m 减半，> 300 m 只留闪光 + 一缕烟）。
5. 近失压制效果（25 m 内）：触发已有 HUD 屏幕效果接口（UI 控制器，不在本任务实现 UI）。
6. WebGL2 路径：粒子用同一 API，数量上限按 Medium 档预算 800。
### 接口与数据
```ts
interface ImpactFx { play(pos: Vec3, radius_m: number, kind: 'he'|'smoke'|'wp', seed: number): void }
```
`seed` 来自事件 id，保证两客户端表现一致（不影响 sim）。

## 不做什么
- 空袭炸弹下落与飞机（M3-030）；载具击毁/殉爆（M3-032）。
- 炮声/尖啸/声速延迟（M3-038）。
- 地形永久形变（几何弹坑）：仅贴花，不改高度图。

## 验收标准
1. 同屏 10 次爆炸同时播放：GPU "透明/粒子" pass 增量 ≤ 0.7 ms（High 参考机 `GPU timestamp`；无参考机标"未测"）。
2. 弹坑贴花回收：连续触发 200 次弹着，活动贴花数 ≤ `max_craters`，JS 堆增长 < 3 MB（vitest/Playwright 内存采样）。
3. 烟雾云团视觉半径与 /data 偏差 ≤ 1 m；60 s ±0.5 s 后完全消失（事件驱动测试）。
4. 粒子活跃数在 10 爆炸 + 烟雾 + 3 烟雾弹同屏峰值 ≤ 1 600（High）/ ≤ 800（Medium WebGL2）。
5. 远处闪光：> 400 m 事件产生闪光脉冲而无粒子（粒子数 = 0 的单测）。
6. 两个相同 seed 的 FX 回放逐帧位置一致（确定性，vitest）。

## 测试要求
- 单元：FxBudget 降级阈值；贴花池回收顺序；seed 稳定性。
- 集成：Playwright `?vis_scene=artillery`（与 M3-048 共用）连播脚本事件；截图基线由 M3-048 收录。
- 性能：粒子计数/GPU 耗时写入 `reports/perf/m3-artillery.json`，供 M3-049 汇总。
- 回归：M2 爆炸基线不变。

## 性能预算
- ARCH §13.5.3 粒子 ≤ 1 600；§13.5.2 透明/粒子 0.7 ms；§13.5.1 渲染提交 4.5 ms 不回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 特效真贴图（V-FX-*）属 M2/M3 素材批；缺失时纯色粒子即可，不得自制贴图（A2）。
- 烟雾是否阻挡视线由 sim 决定；若 sim 与渲染云团尺寸不一致，以 sim 为准并写 PLAN_ISSUES。
- `max_craters` 为本任务提议初值，需在 M3-049 实测后复核。
