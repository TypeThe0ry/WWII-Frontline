# M4-019 render：Ultra 屏幕空间反射

## 元数据
- 里程碑：M4
- 工作流：client-render
- 规模：M
- 前置依赖：M4-017
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §10.2; ARCHITECTURE §13.1; ARCHITECTURE §10.3; ARCHITECTURE §10.1; ARCHITECTURE §9.7

## 背景
ARCHITECTURE §10.2：**SSR 仅 Ultra；Hi-Z 线性跟踪 + 回退到 IBL；仅水面与金属车体**。§13.1：SSR 仅 Ultra 档开启；§10.3：SSR 为 WebGPU 专属（WebGL2 路径无）。INDEX 指标：Ultra（RTX 4070 级）SSR ≤ 1.0 ms；关闭时无残留开销。
关键约束：
- Hi-Z 金字塔复用 M2-034 的深度下采样结果（上一帧深度，≈ 0.3 ms @1080p），**不另建一份**。
- 帧图位置：不透明主通道之后、透明之前（ARCHITECTURE §10.1 的 6–8 步之间）；结果作为反射项在材质着色器中采样，**不新增材质变体**（用 uniform 开关 + 单一宏位，变体上限 ≤ 60，§10.4）。
- 反射对象：海面（M4-014）与金属车体（载具金属材质标记 `ssr_receiver`）；其它材质不接收 SSR（避免噪声与成本）。
- 回退：射线未命中/出屏幕 → 采样 IBL 立方体（§9.8 每地图 2–3 个预设）；边缘淡出。
- 降级：Ultra 档若帧时间压力（动态分辨率触底）可自动关 SSR？—— **不自动改档位**（M4-018）；SSR 仅受档位/设置开关控制。

## 具体内容
### 要创建/修改的文件
- `client/src/render/ssr/SsrPass.ts` —— WebGPU compute：Hi-Z 线性跟踪（≤ 24 步）、二分细化、IBL 回退、半分辨率 + 时间累积。
- `client/src/render/ssr/shaders/ssr_trace.wgsl`、`ssr_resolve.wgsl`。
- `client/src/render/ssr/SsrReceiver.ts` —— 标记接收体（海面、`material.flags & SSR`）。
- `data/render/ssr.json` —— 步数、半分辨率比例、粗糙度截断、淡出参数（`initial`）。
- `client/tests/render/ssr.spec.ts`、`ssr.visual.spec.ts`。
### 要实现的行为
1. 仅在 `tier=ultra` 且 WebGPU 可用时创建 SSR 资源；其它情况 `SsrPass` 不分配任何纹理/管线（"无残留开销"）。
2. 半分辨率 trace + 全分辨率 resolve；时间累积使用运动向量，历史权重在海面法线抖动处降低。
3. 水面使用 M4-014 的法线；金属车体使用材质粗糙度阈值（粗糙度 > 0.5 不接收）。
4. IBL 回退平滑过渡（命中置信度混合）。
5. 切换档位（M4-017 热切换）时 ≤ 1 帧内创建/销毁资源，无显存泄漏（测前后显存估计差值 ≤ 1 MB）。
6. 与 TAAU 兼容：反射结果参与 TAAU 前的历史重投影。
### 接口与数据
- `class SsrPass { init(engine, hiz: HiZPyramid, cfg: SsrConfig): void; render(frame: FrameCtx): void; setEnabled(on: boolean): void; dispose(): void }`；`SsrConfig` 来自 `data/render/ssr.json`。

## 不做什么
- 不做 WebGL2 的 SSR（设计不含）。
- 不做平面反射/光线追踪/其它材质的 SSR（Backlog）。
- 不修改 Hi-Z 构建本身（M2-034）。
- 不做体积雾内的反射。

## 验收标准
1. Ultra（RTX 4070 级，1440p）下 SSR pass GPU 时间 ≤ 1.0 ms（WebGPU timestamp，机位 `omaha_beach_01` 与 `vehicle_close_01`；参考机缺失则 `未测` → partial）。
2. 关闭时无残留开销：`SsrPass` 未创建时帧 GPU 时间与未集成前相差 ≤ 0.02 ms（A/B 对比），且显存估计无增量（单测断言资源计数 = 0）。
3. 材质变体数不变（≤ 60，M4-021 门禁）：SSR 集成前后 `getVariantCount()` 相同。
4. 视觉：Omaha 水面与 Sherman 车体的 SSR 开/关截图差异区域仅限水面与金属部位（掩码内差异像素 ≥ 1%，掩码外 ≤ 0.1%）。
5. 档位切换（Ultra→High→Ultra）10 次后显存估计偏差 ≤ 1 MB，且无 console error。

## 测试要求
- 单元测试：`vitest`：配置校验、资源创建/销毁计数、非 Ultra/非 WebGPU 路径不分配。
- 集成/无头测试：Playwright WebGPU（可用时）固定机位开关对比；WebGL2 路径确认无 SSR 且不报错。
- 性能基准：`pnpm -C client bench:ssr` 输出 ms；与 `reports/perf/m4-tiers.json` 合并。
- 回归：Ultra 视觉基线新增 SSR 开启机位（M4-040）。

## 性能预算
- SSR ≤ 1.0 ms（Ultra，INDEX 指标）；Ultra 档整体 GPU 预算在 M4-041 实测（§13.5.2 注：以实测为准）；Hi-Z 复用，不额外 ≈ 0.3 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 海面与车体接收体依赖 M4-014、M3-029 的材质标记；若 M4-014 未完成，水面部分先用测试平面验收，并记 partial 条目。
- 水面 SSR 在海浪法线大幅抖动时易闪烁，时间累积参数需调；保留 `ssr.json` 可调。
- 4070 级硬件需人类提供（H-021）。
