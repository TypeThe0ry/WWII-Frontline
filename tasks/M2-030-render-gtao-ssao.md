# M2-030 render：GTAO/SSAO

## 元数据
- 里程碑：M2
- 工作流：client-render
- 规模：M
- 依赖：M1-044
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §10.1; ARCHITECTURE §10.2; ARCHITECTURE §10.3; ARCHITECTURE §13.1; ARCHITECTURE §13.5.2

## 背景
ARCHITECTURE §10.1 帧图第 4 步为"GTAO（半分辨率）+ 双边模糊"，位于深度预通道（第 3 步）之后、体积雾之前。§10.2：GTAO 半分辨率 8 方向 × 2 步（High）、更低（Medium）、关闭（Low）。§10.3 差异表：WebGPU 用 GTAO，WebGL2 用 SSAO（Medium+）/关闭（Low）。§13.5.2 GPU 预算：GTAO + 模糊 0.9 ms（High，RTX 3060，1080p 内部 100%）。M2 只有 High 基准与对应 WebGL2 路径（MILESTONES §3.2），四档预设数据化属于 M4-017，本任务只提供"质量参数 + 开关"接口。
M1-044 已建立 TAA、运动向量与后处理链；本任务要在其上增加 AO 输出并在主通道中被采样（作为环境光遮蔽项）。GTAO 与 TAA 联用时用时间累积降噪，需考虑 TAA 抖动。

## 具体内容
### 要创建/修改的文件
- `client/src/render/postfx/gtao.ts` —— GTAO 管线：半分辨率深度/法线输入、水平线搜索、时间累积与双边模糊编排；暴露 `GtaoPass`。
- `client/src/render/shaders/wgsl/gtao.wgsl`、`gtao_blur.wgsl` —— WebGPU compute 着色器。
- `client/src/render/shaders/glsl/ssao.frag.glsl`、`ssao_blur.frag.glsl` —— WebGL2 SSAO 片元 pass（半分辨率，16 个半球样本，双边模糊）。
- `client/src/render/postfx/ao_params.ts` —— 质量参数结构 `AoQuality`（方向数、步数、半径、强度、是否时间累积）；M2 预置 `high`/`medium`/`off` 三组，数据化留给 M4-017。
- `client/src/render/pipeline.ts`（修改）—— 在深度预通道后挂入 AO；主通道材质宏采样 `aoTexture`。
- `client/tests/render/ao.shot.spec.ts` —— Playwright 固定机位截图（开/关）与差异报告。
### 要实现的行为
1. WebGPU 路径：输入深度预通道结果与法线（若无法线缓冲则由深度重建）；半分辨率 8 方向 × 2 步（High）、Medium 4 方向 × 2 步；输出 R8 AO；双边模糊（深度边缘感知，半径 ≤ 4 像素）；与 TAA 抖动相位对齐的时间累积。
2. WebGL2 路径：SSAO 半分辨率 + 双边模糊；Low 档（质量 `off`）完全跳过 pass，不分配 RT。
3. AO 只作用于间接/环境光项（不压暗直接太阳光，避免与 CSM 重复），强度参数 `aoStrength`。
4. 开关与质量热切换：`GtaoPass.setQuality(q: AoQuality)` 不重建整条管线；切换 ≤ 1 帧。
5. 暴露 GPU 时间戳标签 `pass.gtao`（WebGPU timestamp query；不可用时用帧间隔估计，标注"估计"）。
### 接口与数据
```ts
export interface AoQuality { mode: 'gtao' | 'ssao' | 'off'; directions: number; steps: number; radiusM: number; strength: number; temporal: boolean }
export class GtaoPass { constructor(engine: AbstractEngine, depth: RenderTargetTexture); setQuality(q: AoQuality): void; readonly output: BaseTexture }
```

## 不做什么
- 不做四档预设表与设置界面联动（M4-017、M2-041 提供设置 UI，M4-024）。
- 不做 SSR（M4-019）、体积雾（M2-031）、TAAU（M2-032）。
- 不做动态分辨率（M4-018）。

## 验收标准
1. High 参考机（RTX 3060，1080p，内部 100%）上 `pass.gtao` GPU 时间（含模糊）P95 ≤ 0.9 ms（ARCH §13.5.2），测量：渲染调试叠层的 timestamp 采样 1 000 帧。
2. WebGL2 路径 SSAO 在 Medium 配置（67% 内部分辨率）GPU 时间 ≤ 1.0 ms（参考机，帧间隔估计时标"估计"）；Low 配置 AO 相关 GPU 时间 = 0（pass 未执行，用调试计数验证）。
3. 固定机位（老城街道、树篱、室内入口）开/关 AO 截图差异报告产出到 `reports/render/ao/`，差异像素占比 > 0 且 AO 区域（墙角/车底）亮度下降 ≥ 10%（脚本统计）。
4. 静止场景 AO 时间稳定性：连续 60 帧 AO 缓冲逐像素标准差均值 ≤ 0.01（无闪烁，脚本）。
5. 质量热切换 `high→medium→off→high` 无报错、无 RT 泄漏（调试统计 RT 数量回到初值）。
6. 两条路径（WebGPU/WebGL2）均有固定机位基线截图交给 M2-050。

## 测试要求
- 单元测试：`ao_params` 取值范围与默认值；管线开关状态机（vitest）。
- 集成/无头测试：Playwright 加载测试场景，WebGPU 与 WebGL2 各一轮截图并生成差异报告；着色器编译无 warning。
- 性能基准：调试叠层采样脚本 `client/tools/perf-capture.ts`（M1-069 若已有则复用）输出 JSON。
- 回归：新增 AO 机位基线供 M2-050 入库。

## 性能预算
- ARCHITECTURE §13.5.2：GTAO + 模糊 0.9 ms（High）；不得使 M1 已测的其余 pass（阴影 2.2、不透明主通道 4.0、TAA 0.8 等）回归超 5%。
- 显存：AO RT（半分辨率 R8 + 历史）≤ 8 MB（计入 §13.4 渲染目标项）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright 截图作业）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 材质变体总数 ≤ 60（ARCH §10.4）：AO 采样宏不得新增变体维度，用 uniform 开关。
- 若 M1-044 的深度预通道尚无法线输出，从深度重建法线并在备注记录精度损失；需要真实法线缓冲则写 PLAN_ISSUES。
- 参考机缺失（RTX 3060）时 GPU 数字标 `未测`，任务 `partial`（需求 H-021 测试设备矩阵）。
