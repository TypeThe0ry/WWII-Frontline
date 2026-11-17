# E0-6：Babylon `WebGPUEngine` + `ComputeShader` 可行性（M0-020）

- 日期：2026-11-17；Babylon `@babylonjs/core` 8.56.2；Chromium（Playwright 1.63 自带，chromium-1243）。
- **测试环境限制（重要）**：本机无 GPU。WebGPU 只能用 SwiftShader 软件适配器（`--enable-unsafe-webgpu --enable-unsafe-swiftshader --use-webgpu-adapter=swiftshader`）。该适配器上**向画布 present（`GPUCanvasContext.getCurrentTexture`）会让 Dawn 实例失效**（裸 WebGPU 最小用例复现，与 Babylon 无关，尝试 6 组 flag 均无效）。因此测试通过 `client/e2e/webgpu-swiftshader-shim.js` 把 `getCurrentTexture` 换成离屏纹理：Babylon 的引擎初始化、渲染、compute、timestamp-query 全链路真实执行，但**画面不可见、耗时是软件适配器耗时**。下表所有 `ms` 无任何 GPU 性能含义。真实 GPU 上的 compute 冒烟与帧时间需在参考机复测（NEEDS_HUMAN H-021）：`pnpm -C client dev` 后打开 `/?renderer=webgpu&smoke=compute&debug=1`，读 `window.__smoke`。
- 复现：`pnpm -C client test:e2e`（用例"WebGPU path on the SwiftShader software adapter"）。

## 结论总表

| 项 | 试验 | 结果（软件适配器） | 结论 |
|---|---|---|---|
| 基础 | 64×64 `u32` 存储缓冲写入 `x*64+y`，读回比对 | 4096/4096 一致，mismatches=0 | **可行**（`StorageBuffer` + `ComputeShader` + `read()`） |
| 基础 | 64×64 `r32float` 存储纹理写入同图案，`readPixels()` 读回 | 4096/4096 一致 | **可行**（`RawTexture.CreateRStorageTexture`） |
| Hi-Z | GPU 生成 64×64 R32F "深度"，compute 2×2 max-reduce 链 64→32→16→8→4，逐级读回对 CPU 参考 | 全部一致，mismatches=0 | **可行，需自写薄封装**（见缺口 1–3） |
| froxel | 160×90×64 `r32float` 3D 存储纹理写常数 1，第二个 pass 用 `textureLoad` 读 3D 纹理并 `atomicAdd` 求和 | 921600 / 921600 | **可行**（`RawTexture3D` + `TEXTURE_CREATIONFLAG_STORAGE`）；缺口 4 |
| TAAU | 两张 RGBA8 存储 RT（current/history），compute 做常量 1 px 重投影 + 0.25 混合，写第三张 RT 并读回 | 3×4096 通道在 ±1.5/255 内一致 | **占位 pass 可行**；真实 TAAU 另需抖动/运动向量/RTT 接入（缺口 5），**不在本试验内** |
| GPU 计时 | `EngineInstrumentation.captureGPUFrameTime` + `timestamp-query` | 叠层/`__perf.dump()` 的 `gpu_ms.source = "timestamp-query"`，有数值 | **可行**（需设备特性 `timestamp-query`，引擎构造时经 `deviceDescriptor.requiredFeatures` 请求） |

每个试验各 ≤ 60 行 WGSL，位于 `client/src/render/compute-smoke.ts`。软件适配器上各试验端到端耗时 70–260 ms（含管线创建与读回，不具参考价值）。

## 遇到的 Babylon 8.56 API 缺口 / 注意点

1. **无逐 mip 存储视图**：`ComputeShader.setStorageTexture(name, BaseTexture)` 绑定整张纹理，没有"指定 mip level/array layer"参数。Hi-Z 金字塔只能"每级一张独立纹理"（本试验做法），或自写封装直接创建 `GPUTextureView`。Hi-Z 需要真 mip 链时（M1-04x）要自写薄封装。
2. **深度纹理绑定未测**：本试验的"深度"是 compute 生成的 R32F，没有验证把 Babylon `RenderTargetTexture` 的深度附件以 `texture_depth_2d` 绑到 compute。真实 Hi-Z 需要它（或先拷贝到 R32F）。**未测**，需在 M1 渲染任务里先做 spike。
3. **间接绘制**：§9.7 的 "Hi-Z + indirect draw" 需要 compute 写 indirect 参数并用 `drawIndexedIndirect` 驱动 Babylon 网格；本次未探查 Babylon 是否有公开 API。**未测**；预期需要绕过 Mesh 渲染路径（自写封装）或退回 CPU 剔除（WebGL2 路径本来就是这样）。
4. **3D 纹理**：读回走"GPU 内求和"而不是 `readPixels`；3D 纹理的 `readPixels`、三线性采样（`texture_3d` + sampler）、RGBA16F 3D 均**未测**。
5. **TAAU 接入点未测**：投影矩阵抖动、`GeometryBufferRenderer` 运动向量、把 `RenderTargetTexture` 当 history ping-pong 绑给 compute、后处理链顺序。按 ARCHITECTURE T11，不依赖 Babylon FrameGraph，自行组织 pass 顺序（本试验也是手动 `await dispatchWhenReady`）。
6. **绑定映射手写**：`ComputeShader` 必须手写 `bindingsMapping`（浏览器不支持 WGSL 反射），WGSL 变量名/绑定号与 TS 要同步维护；`dispatchWhenReady` 是 await 式，生产里应使用 `dispatch` 并保证每帧顺序。
7. 非过滤浮点纹理（R32F）用 `setTexture(name, tex, false)`（不绑 sampler）+ `textureLoad`，可用。
8. WebGL2 路径无 compute：`?smoke=compute` 报告 `unsupported (expected)`（e2e 覆盖）。

## 对 E0-6 退出条件的判断（待参考机复核）

- Babylon `WebGPUEngine` + `ComputeShader` 冒烟：**软件适配器上通过**；真 GPU 上**未测**。
- Hi-Z / froxel / TAAU：数据通路均可行；其中 Hi-Z 需自写薄封装（逐 mip 视图、深度纹理绑定、indirect draw 三项里至少前两项）；froxel 基本不需要封装；TAAU 的难点在渲染管线接入而非 compute 本身。
- 建议：M1 渲染任务开工前，在参考机上先跑 `?smoke=compute`，并就缺口 2/3/5 各做 ≤ 1 天 spike。已记 PLAN_ISSUES。
