# M2-032 render：TAAU 时间性超分

## 元数据
- 里程碑：M2
- 工作流：client-render
- 规模：L
- 前置依赖：M1-044
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §10.1; ARCHITECTURE §10.2; ARCHITECTURE §10.3; ARCHITECTURE §13.1; ARCHITECTURE §13.2; ARCHITECTURE §13.5.2

## 背景
ARCHITECTURE §10.2 TAAU 设计：输入内部分辨率（如 1280×720 → 输出 1920×1080）；Halton(2,3) 抖动 8 相位；历史重投影用运动向量 + 深度；邻域 clamp（YCoCg 方差 clip）；自适应锐化（RCAS 式）；在过亮高光与高速体上降低历史权重；植被风动物体用 `motion_mask` 降低历史权重（§10.1）。WebGL2 无 compute → 降级为 FSR1 式 EASU/RCAS 空间放大 + 轻 TAA（片元全屏 pass，§10.3）。
§13.1：Medium 内部渲染比例 67%（动态 60–85%），High 100%（动态 75–100%）；超分列 Low=FSR1 式、Medium/High/Ultra=TAAU。§13.5.2 预算：TAA/TAAU 0.8 ms（High）。M1-044 已有 Halton 8 相位 TAA 与运动向量；本任务把它扩展为"输入分辨率 ≠ 输出分辨率"的 TAAU，并提供 `render_scale` 接口。动态分辨率控制器属 M4-018，本任务只提供接口与固定比例。

## 具体内容
### 要创建/修改的文件
- `client/src/render/postfx/taau.ts` —— `TaauPass`：历史缓冲（输出分辨率）、抖动相位管理、compute 调度；`setRenderScale(s)`。
- `client/src/render/shaders/wgsl/taau.wgsl` —— 重投影、方差 clip、历史权重、上采样滤波（Lanczos/Catmull 近似）、RCAS 式自适应锐化。
- `client/src/render/shaders/glsl/easu.frag.glsl`、`rcas.frag.glsl`、`taa_light.frag.glsl` —— WebGL2 路径 FSR1 式空间放大 + 轻 TAA。
- `client/src/render/render_scale.ts` —— `RenderScale` 状态：范围 [0.5, 1.0]，步进 0.01；变化时重建中间 RT（带防抖，≤ 1 次/0.5 s）与历史重置。
- `client/src/render/pipeline.ts`（修改）—— 主通道使用内部分辨率 RT；TAAU 输出至屏幕分辨率；Bloom/色调映射在输出分辨率执行。
- `client/src/render/motion_mask.ts` —— 风动植被/高速体的 `motion_mask`（写入运动向量 alpha 或独立 R8）。
- `client/tests/render/taau.ssim.spec.ts`、`client/tools/ssim.ts` —— 固定机位 SSIM 对比脚本。
### 要实现的行为
1. 抖动：Halton(2,3) 8 相位，抖动量按**输入**分辨率像素；TAA 关闭或测试截图模式下可固定相位（供 M2-050 视觉回归）。
2. 历史：以运动向量 + 深度重投影；邻域 clamp 用 YCoCg 方差 clip；`motion_mask` 与高亮高光（亮度 > 阈值）降低历史权重；重生/镜头切换时重置。
3. 上采样：依据抖动子像素位置做加权，输出分辨率历史累积；锐化强度随历史置信度自适应（运动大则减弱）。
4. WebGL2 路径：EASU 空间放大（内部分辨率→输出）+ RCAS + 轻 TAA（单历史，权重 0.9）；与 WebGPU 路径共用 `setRenderScale` 接口。
5. `render_scale = 1.0` 时退化为原生分辨率 TAA（与 M1-044 行为一致，不得回归）。
6. 暴露 GPU 时间戳 `pass.taau`。
### 接口与数据
```ts
export class TaauPass { constructor(engine: AbstractEngine, opts: { outputSize: () => [number, number] }); setRenderScale(s: number): void; setFixedJitterPhase(p: number | null): void; readonly renderScale: number }
```
锐化强度、历史权重、亮度阈值放 `/data/render/taau.json`（状态 `initial`，【待调参】）。

## 不做什么
- 不做动态分辨率控制器与首次运行基准推荐（M4-018）；不做四档预设表（M4-017）。
- 不做 FXAA 档的细节调参、不做 DLSS/FSR2 等外部库。
- 不做 Ultra "质量模式"特例（M4-017）。

## 验收标准
1. 固定机位（≥ 3 个：开阔、老城街道、树篱）`render_scale = 0.67` 的 TAAU 输出与 `render_scale = 1.0`（同机位、固定相位、原生分辨率 TAA）参考图 SSIM ≥ 0.95（`pnpm -C client test:ssim`，WebGPU 与 WebGL2 各报告，WebGL2 阈值同为 ≥ 0.95 若不达标记 PLAN_ISSUES 并说明实测值）。
2. High 参考机（RTX 3060，1080p 输出，内部 67%）`pass.taau` GPU 时间 P95 ≤ 0.8 ms（ARCH §13.5.2）；`render_scale = 1.0` 同样 ≤ 0.8 ms。
3. 静止场景帧间差：同机位连续 60 帧，输出逐像素均方差 ≤ 阈值（脚本，≤ M1-044 基线的 110%）。
4. 运动鬼影：相机平移 5 m/s 经过细栅栏/铁丝网测试体，鬼影长度 ≤ 12 像素（脚本测量）。
5. `setRenderScale` 在 0.5–1.0 范围内连续调用 100 次无异常、无 RT 泄漏；防抖有单测（0.5 s 内最多重建 1 次）。
6. WebGL2 路径开启后 `pass.taau` 等效 GPU 时间（EASU+RCAS+轻 TAA）≤ 1.2 ms（Medium 参考，估计值标注）。

## 测试要求
- 单元测试（vitest）：Halton 序列前 8 项数值；`RenderScale` 取值裁剪与防抖；固定相位开关。
- 集成/无头测试：Playwright 两条路径 SSIM 报告（`reports/render/taau/ssim.json`）；`navigator.gpu` 缺失时自动走 FSR1 式路径。
- 性能基准：帧时间/GPU timestamp 采集脚本输出 `reports/render/taau/perf.json`。
- 回归：TAAU 机位基线交 M2-050；M1-044 的 TAA 稳定性测试必须继续通过。

## 性能预算
- ARCHITECTURE §13.5.2：TAA/TAAU 0.8 ms（High）；§13.4：历史 RT（输出分辨率 RGBA16F ≈ 16 MB）+ 中间 RT 按 §13.4 渲染目标项（≈ 170 MB 总量不得超）。
- 内部分辨率 67% 时，像素相关 pass 预计 ×0.5（§13.5.2 Medium 备注），本任务提供实测数据给 M2-052/M4-017。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright SSIM 作业）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 植被风动与角色骨骼动画的运动向量精度是 TAAU 鬼影的主因；若 M1-045 未输出骨骼运动向量，需在本任务补最小实现并记录。
- SSIM ≥ 0.95 对高频植被机位可能难达；若不达标，如实记录并写 PLAN_ISSUES（建议调整机位或阈值），不得放宽数字自行通过。
- 软件渲染（SwiftShader）下 TAAU 结果与 GPU 有差异，基线需按运行器类型分存（M2-050）。
