# M2-031 render：体积雾（froxel）

## 元数据
- 里程碑：M2
- 工作流：client-render
- 规模：M
- 依赖：M1-043
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §10.1; ARCHITECTURE §10.2; ARCHITECTURE §10.3; ARCHITECTURE §13.1; ARCHITECTURE §13.5.2

## 背景
ARCHITECTURE §10.2 体积雾设计：froxel 160×90×64（深度对数分布到 600 m），每帧时间复用（1/2 抖动 + 历史混合）；仅一盏平行光（太阳，用 CSM 阴影采样）+ 环境项；Medium 降为 80×45×48；Low 无体积雾，仅高度指数雾。§10.1 帧图第 5 步为"注入光/散射/积分（compute）"，第 7 步与天空合成。§10.3：WebGL2 无 compute → 高度雾。§13.1：Ultra 为 240×135×96（M4-017 数据化，本任务把分辨率做成参数）。§13.5.2 预算：froxel 0.8 ms（High）。
M1-043 已有方向光、CSM、天空/大气 LUT；Carentan 清晨沼泽雾（MAPS §3.5 时段设定）依赖本效果。

## 具体内容
### 要创建/修改的文件
- `client/src/render/postfx/froxel_fog.ts` —— 雾体积 3D 纹理（RGBA16F）分配、compute 调度编排、历史混合；类 `FroxelFog`。
- `client/src/render/shaders/wgsl/froxel_inject.wgsl`、`froxel_scatter.wgsl`、`froxel_integrate.wgsl` —— 注入密度/光照（采样 CSM）、时间复用、沿深度积分。
- `client/src/render/shaders/wgsl/fog_composite.wgsl` 与 `client/src/render/shaders/glsl/height_fog.frag.glsl` —— 合成（WebGPU）与高度指数雾（WebGL2/Low）。
- `client/src/render/fog_params.ts` —— `FogParams`：分辨率、最大距离（600 m）、散射系数、各向异性 g、基础密度、高度衰减、风漂移。
- `client/src/render/pipeline.ts`（修改）—— 在 AO 之后、不透明主通道前后挂入注入/积分与合成。
- `client/src/world/atmosphere_presets.ts`（修改，若存在）—— 时段预设增加雾参数（清晨：浓、沼泽附近密度加成区由地图数据 `fog_zones` 提供，缺省则全局）。
### 要实现的行为
1. 3D 网格 160×90×64，深度切片 `z = near * (far/near)^(k/64)`，`far = 600 m`；视锥随相机更新。
2. 注入：密度 = 基础密度 × 高度衰减 + 区域加成（沼泽/运河 `fog_zones`）；太阳光经 CSM 阴影采样得到散射；环境项取 IBL 平均色。
3. 时间复用：每帧对网格做半步抖动并与历史（重投影）混合，权重 0.9/0.1；相机剧烈运动或瞬移（重生）时重置历史。
4. 积分后在第 7 步与天空一并合成到场景颜色；天空与远景环使用同一参数避免接缝。
5. 档位：High 160×90×64、Medium 80×45×48（`setResolution`）；Low 与 WebGL2 路径走高度指数雾 pass（不分配 3D 纹理）。
6. 质量与性能：暴露 GPU 时间戳 `pass.froxel`；无 timestamp 时用帧间隔估计。
### 接口与数据
```ts
export interface FogParams { grid: [number, number, number]; maxDistM: number; scattering: number; g: number; baseDensity: number; heightFalloff: number }
export class FroxelFog { constructor(engine: AbstractEngine, params: FogParams); setResolution(grid: [number, number, number]): void; resetHistory(): void }
```
数值（散射系数、密度）不得硬编码在渲染代码：放 `/data/render/atmosphere.json`（状态 `initial`，【待调参】）。

## 不做什么
- 不做 Ultra 240×135×96 的档位表联动与预设（M4-017，仅保证参数可配置）。
- 不做 GTAO（M2-030）、TAAU（M2-032）、云影/体积云。
- 不做点光/车灯体积光（后续）；不做雾对 bot 视野的影响（bot 感知不受渲染雾影响）。

## 验收标准
1. High 参考机（RTX 3060，1080p）`pass.froxel`（注入+散射+积分+合成）GPU 时间 P95 ≤ 0.8 ms（ARCH §13.5.2），测量 1 000 帧 timestamp。
2. Medium 配置（80×45×48）GPU 时间 ≤ 0.45 ms（参考机，估算值需实测记录）。
3. WebGL2 路径：高度雾可用，开/关对比截图差异报告产出；Low 档不分配 3D 纹理（调试 RT 统计 = 0）。
4. 时间复用无鬼影：相机以 90°/s 旋转 3 s，雾边缘拖影宽度 ≤ 8 像素（截图脚本测量）；重生瞬移后 2 帧内历史重置无闪烁。
5. 固定机位（清晨沼泽、老城街道）开/关雾截图差异报告产出到 `reports/render/fog/`；雾使远景（> 300 m）对比度下降 ≥ 20%（脚本统计）。
6. 分辨率热切换 High↔Medium 无报错、无显存泄漏（3D 纹理数量回到初值）。

## 测试要求
- 单元测试（vitest）：对数深度切片函数往返；`FogParams` 校验；历史重置条件逻辑。
- 集成/无头测试：Playwright 两条路径各截图；着色器编译无 warning；WebGPU 不可用时自动回退高度雾（模拟 `navigator.gpu` 缺失）。
- 性能基准：沿用 M1-069 的帧时间采集脚本，输出 `reports/render/fog/perf.json`。
- 回归：新增雾机位基线交给 M2-050。

## 性能预算
- ARCHITECTURE §13.5.2：froxel 0.8 ms（High）；§13.4 显存：3D 纹理 160×90×64×RGBA16F ≈ 7.4 MB ×2（当前+历史）计入渲染目标项。
- 不得使 §13.5.1 CPU 渲染提交预算回归（compute 调度 ≤ 0.1 ms CPU）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright 截图作业）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 雾对 TAA 稳定性有影响：时间复用权重与 TAA 历史需协调，若出现拖影写 PLAN_ISSUES。
- 沼泽雾加成区依赖 M2-002/M2-004 的 `fog_zones`/洪泛多边形；缺失时用全局密度，备注记录。
- 参考机缺失则 GPU 数字标 `未测`，任务 `partial`（H-021）。
