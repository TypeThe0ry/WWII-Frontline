# M0-020 client/render：引擎初始化与调试叠层

## 元数据
- 里程碑：M0
- 工作流：client-render
- 规模：M
- 前置依赖：M0-001
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §5.1; ARCHITECTURE §10.3; ARCHITECTURE §13.6; ARCHITECTURE §6.1; ARCHITECTURE §20.1; ARCHITECTURE §13.5.1

## 背景
渲染基座：Babylon.js 8.x，`WebGPUEngine` 优先，能力探测失败自动回退 `Engine`（WebGL2）（ARCHITECTURE §5.1/§10.3）。M0 要同时回答 E0-6："Babylon `WebGPUEngine` + `ComputeShader` 冒烟通过；给出 Hi-Z/froxel/TAAU 的可行/不可行/需自写封装结论"（ARCHITECTURE §20.1-4、T11：Babylon FrameGraph 成熟度**未核实**，不作为依赖，自行组织 pass 顺序）。渲染循环与 30 Hz 模拟解耦：每渲染帧累计 `real_dt`，跑 0..N 个模拟步（**N 上限 5**，防螺旋），余数用于渲染插值（§6.1）。能力探测（§13.6）：`navigator.gpu?.requestAdapter()` → `adapter.info`（可用时）、限制项（`maxTextureDimension2D`、`maxStorageBufferBindingSize`）、WebGL2 `MAX_TEXTURE_SIZE`/扩展、`navigator.deviceMemory`（Chromium 上报值最大 8）、`hardwareConcurrency`；结果**只用于推荐档位，不上传**（隐私，§19）。
下游：M0-023/024/025 在此引擎上画地形/植被/角色；M0-030 通过叠层导出 JSON 采集性能；M0-022 使用 `FixedStepper` 驱动预测。

## 具体内容
### 要创建/修改的文件
- `client/src/render/{engine.ts,capabilities.ts,test-scene.ts,compute-smoke.ts,index.ts}`
- `client/src/render/debug/{overlay.ts,frame-stats.ts,gpu-timing.ts,perf-export.ts}`（叠层与 `window.__perf.dump()`）
- `client/src/game/fixed-step.ts`（纯函数式 `FixedStepper`，放 `game` 以免 `render` 依赖 `game` 之外的循环语义；`main.ts` 作为组合根驱动）
- `client/src/main.ts`（装配：探测 → 创建引擎 → 测试场景 → 循环）；`client/e2e/render-bootstrap.spec.ts`（Playwright，`@playwright/test` 作为 devDependency）；`client/src/**/*.test.ts`。
- `reports/m0/compute-feasibility.md`（E0-6 结论）。
### 要实现的行为
1. **引擎创建**：URL 参数 `?renderer=webgpu|webgl2|auto`（默认 auto）。auto：`WebGPUEngine.IsSupportedAsync` 且 `requestAdapter()` 成功 → `new WebGPUEngine(canvas)` + `initAsync()`；否则 `new Engine(canvas, antialias, {…})`（WebGL2）。任何异常自动回退并在叠层标出原因。启动即记录 `engine.kind`、`adapter.info`（可用字段）、限制项到 `window.__caps`。
2. **测试场景**：平面 + 若干盒/球 + 1 盏平行光 + 地面网格线；轨道/自由相机（`?fly=1` 为 WASD+鼠标自由相机，M0-023 复用）；无阴影与后处理。
3. **固定步长**：`FixedStepper(step_ms=33.333, max_steps=5)`：`advance(real_dt_ms) -> {steps, alpha}`；`steps>5` 时丢弃多余累积时间并计数 `stepper_clamped`；`alpha ∈ [0,1)` 供渲染插值。
4. **叠层**（F3 开关，`?debug=1` 默认开）：FPS、CPU 帧时间（`performance.now()` 差，含 P50/P99 滑窗 600 帧）、**GPU 帧时间**（WebGPU：Babylon 的 GPU timing/`timestamp-query` 计数器——以所装版本的 API 为准；WebGL2：`EXT_disjoint_timer_query_webgl2` 可用时用，否则显示 `n/a` 并用帧间隔估计）、draw call 数（`SceneInstrumentation`）、活动网格/三角形数、渲染路径（WebGPU/WebGL2）、`render_scale`、占位槽位 `RTT`/`transport`/`corrections`（后续任务填充）。`window.__perf.dump()` 返回含上述统计的 JSON（固定字段，供 M0-030）。
5. **compute 冒烟**（`?smoke=compute`，仅 WebGPU）：Babylon `ComputeShader`（WGSL）向 64×64 存储纹理/存储缓冲写入已知图案（如 `value = x*64+y`），读回并逐元素比对，结果写 `window.__smoke = {ok, mismatches, ms}`；WebGL2 路径报告 `unsupported (expected)`。另写三段**可行性试验**（各 ≤ 60 行 WGSL，不要求画质）：①Hi-Z 金字塔：对深度纹理做 mip 下采样 compute；②froxel 注入：向 160×90×64 3D 存储纹理写入常数并读回求和；③TAAU 历史混合：两张 RT 做重投影占位 pass。把"可行 / 不可行 / 需自写薄封装"及理由、遇到的 Babylon API 缺口写入 `reports/m0/compute-feasibility.md`，同时记 PLAN_ISSUES（若需自写封装）。
6. WebGL2 路径下 draw call 预算低（ARCHITECTURE §13.5.3）：此处只确认两条路径均能渲染，不做优化。
### 接口与数据
- `createRenderer(canvas, opts): Promise<RenderContext{engine, scene, kind:'webgpu'|'webgl2', caps}>`；`RenderContext.onFrame(cb)`。`render` 目录不得 import `net/ui/input`（ARCHITECTURE §4.3.9）。

## 不做什么
- 不做地形/植被/角色（M0-023/024/025）；不做阴影、后处理、TAA/TAAU 的真实实现（M1-043/044、M2-032）。
- 不做画质分级与动态分辨率（M4-017/018）；不做自动档位推荐 UI。
- 不做着色器变体管理与预热（M1-042、M4-021）。

## 验收标准
1. Playwright `render-bootstrap.spec.ts`：`?renderer=webgl2` 路径下 Chromium 渲染测试场景，画布采样（中心 + 4 角共 5 点）与清屏色不同且稳定 ≥ 60 帧（CI 可用软件渲染 SwiftShader）；`?renderer=webgpu` 在有 WebGPU 的机器上同样通过（需人工/参考机，CI 无 GPU 时 skip 并记录）。
2. 叠层显示 GPU/CPU 帧时间：WebGPU 路径 GPU 帧时间 ≠ n/a（需 `timestamp-query` 支持；不支持时明示原因）；`window.__perf.dump()` 返回的 JSON 字段齐全（schema 测试）。
3. compute 冒烟：WebGPU 路径 `__smoke.ok === true` 且 `mismatches === 0`（参考机/本机，需人工）；三个可行性试验各有"通过/失败 + 数据"记录。
4. `FixedStepper` vitest：`advance(100 ms)` 返回 `steps=3`；`advance(1000 ms)` 返回 `steps=5` 且 `stepper_clamped` +1；累积 1 h 随机 `real_dt` 后实际步数与理论 `floor(总时间/33.333 ms)` 偏差 ≤ 1。
5. 强制回退：把 `navigator.gpu` 置 `undefined`（Playwright `addInitScript`）后 auto 模式在 ≤ 2 s 内回退到 WebGL2 并正常渲染，叠层标出回退原因。
6. 空场景稳定帧率：本机或参考机 1080p 空场景 60 s，CPU 帧时间 P99 ≤ 4 ms（记录机型；仅信息性，为 M0-030 提供底噪）。

## 测试要求
- 单元测试：`FixedStepper`、能力探测解析（mock `navigator.gpu`/`deviceMemory`）、`perf-export` schema。
- 集成/无头测试：Playwright（验收 1、5）；compute 冒烟（人工/参考机）。
- 性能基准：验收 6。
- 回归：无（视觉基线从 M2-050 开始）。

## 性能预算
渲染提交 ≤ 4.5 ms、`game` 更新 ≤ 1.5 ms 等 CPU 预算见 ARCHITECTURE §13.5.1；本任务的空场景应远低于这些值（验收 6 记录）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Babylon 的 GPU 计时 API 名称与可用性以安装的 8.x 版本文档为准，不要凭记忆写死；`timestamp-query` 在部分浏览器需要 flag 或默认关闭。
- 参考硬件（RTX 3060 / Iris Xe）见 NEEDS_HUMAN H-021；无硬件时验收 1/3/6 的 WebGPU 部分记 `未测`，任务 `partial`。
- INDEX 要求"compute 写纹理冒烟通过（§20.1-4）"；上面的 Hi-Z/froxel/TAAU 三段试验是为交付 E0-6 的"可行性结论"所做的细化，未超出一次会话的体量时全做，否则优先 Hi-Z 并在结论中如实标注未测项。
