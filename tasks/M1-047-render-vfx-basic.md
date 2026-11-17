# M1-047 render：粒子与贴花基础

## 元数据
- 里程碑：M1
- 工作流：client-render
- 规模：M
- 前置依赖：M1-042
- 素材需求：V-FX-*（占位可）；素材状态：placeholder（引擎内置纯色方片/球，不引入外部贴图）
- 引用文档：ARCHITECTURE §13.5.3; ASSET_GAPS §3.2; ARCHITECTURE §8.4; ARCHITECTURE §6.8.4

## 背景
战斗反馈需要：枪口焰、曳光、烟、爆炸、尘土、弹孔贴花。ASSET_GAPS §3.2：特效占位 = 引擎内置简单粒子（纯色方片/球），不引入外部贴图，强度/持续时间取玩法数据。ARCHITECTURE §13.5.3：活跃粒子 ≤ 1 600（WebGPU High）/ 800（WebGL2 Medium）/ 300（Low）；M1 验收 800 活动粒子 ≤ 0.7 ms。事件来源：`fire.local`/`fire.remote`/`impact`（含 `material`）/ Explosion（C2.events，ARCHITECTURE §6.8.4）。全烟雾体积 flipbook、火焰、弹坑/血迹贴花属 M2-033。

## 具体内容
### 要创建/修改的文件
- `client/src/render/vfx/ParticleSystem.ts` —— 池化粒子系统：单一大缓冲（SoA）、GPU instancing 方片（billboard）；WebGPU 可用 compute 更新，WebGL2 用 CPU 更新+instance buffer 上传；固定预算与优先级丢弃。
- `client/src/render/vfx/Emitters.ts` —— 预设：`muzzle_flash`（按武器类别：步枪/冲锋枪/机枪/炮）、`tracer`（曳光线，长度/速度取武器 `/data`）、`smoke_grenade`（简化）、`explosion_small/medium`、`dust_impact`（按材质：土/沙/木/砖/金属）、`shell_eject`（弹壳，可选）。
- `client/src/render/vfx/Decals.ts` —— 弹孔贴花池（环形，上限 128 活动，按距离淡出，投影到地形/建筑）；材质区分（土/木/砖/金属）。
- `client/src/render/vfx/Budget.ts` —— 预算与优先级：玩家自己 > 近处 > 远处；超限按优先级丢弃，计数 `vfx_dropped`。
- `client/src/render/vfx/events.ts` —— 订阅 `fire.local`、`fire.remote`、`impact`、`explosion`，从武器 `socket_muzzle`（M1-046/045）取起点。
- `client/test/render/vfx.test.ts`。
### 要实现的行为
1. 粒子池容量 = 档位预算（1600/800/300）；满则按优先级驱逐最低优先级，绝不分配新内存（热路径零分配）。
2. 枪口焰/曳光/弹着与 `fire.*`、`impact` 同帧触发（本地预测事件零延迟）。
3. 曳光：每 N 发一发（N 取武器数据，默认 5【待调参】），沿弹道直线+重力段渲染，寿命 ≤ 0.3 s。
4. 弹孔贴花：命中静态几何时放置，环形覆盖；对动态物体不放贴花（M2-033 再扩展）。
5. 远处（> 100 m）的枪口焰/弹着合并为一个小闪光点以省粒子。
6. 质量档：Low 档关闭弹壳与小尘土，保留枪口焰/曳光/爆炸。
### 接口与数据
```ts
export interface VfxEmitRequest { kind: VfxKind; pos: Vec3; dir?: Vec3; material?: number; priority: number; seed: number }
export class ParticleSystem { emit(req: VfxEmitRequest): void; update(dt: number): void; stats(): { active: number; dropped: number } }
```
特效强度、持续时间来自 `/data`（武器/爆炸条目），不在渲染代码硬编码。

## 不做什么
- 不做烟雾体积 flipbook、火焰、弹坑/血迹贴花、粒子 LOD 大规模（M2-033）。
- 不做粒子光照（仅简单自发光/无光照）、不做软粒子深度淡出以外的高级效果。
- 不做枪口焰真贴图接入（M1-064，V-FX-001）。
- 不做爆炸冲击波对画面的震动（相机震动归 game/camera，M2）。

## 验收标准
1. 800 个活动粒子的更新+绘制 GPU+CPU 合计 ≤ 0.7 ms（High 参考机；叠层导出；固定机位爆炸场景压测）。
2. 粒子超限时按优先级丢弃：压测注入 3 000 请求/秒，活动数始终 ≤ 档位上限，`vfx_dropped` 计数 > 0，玩家自己的枪口焰丢弃数 = 0（`vfx_priority_drop` 测试）。
3. 10 000 次 `emit` 调用 JS 堆增量 < 64 KB（零分配验证）。
4. 贴花池环形：连续放置 500 个，活动数 ≤ 128，最旧者被回收（单测）。
5. 开火到枪口焰可见延迟 ≤ 1 渲染帧（时间戳断言）。
6. WebGL2 Medium 档 800 粒子 ≤ 1.2 ms（Iris Xe 或 GTX 1650 级，记录机型）。

## 测试要求
- 单元测试：池/驱逐优先级、曳光频率、贴花环缓冲、材质→预设映射。
- 集成/无头测试：Playwright 固定机位"枪战"场景截图（枪口焰/弹着/爆炸），两路径基线。
- 性能基准：粒子压测脚本导出到 `reports/m1-vfx.json`。
- 回归：M0-025 的内置粒子迁移后行为保持。

## 性能预算
- ARCHITECTURE §13.5.2：透明/粒子 pass ≤ 0.7 ms（High）；§13.5.3 粒子上限 1 600/800/300。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、视觉回归）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_V-FX-*`）；缺失素材需求 ID（V-FX-001..004）已列出
- [ ] 文档同步：特效数据字段若新增，回写 `/data/schemas` 并记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- V-FX 真贴图许可证多为 PENDING（见 ASSET_SHOPPING_LIST V-FX-001..004），缺失不阻塞本任务。
- WebGL2 路径无 compute，CPU 更新 800 粒子需控制在预算内；必要时降低更新频率。
