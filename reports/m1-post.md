# M1-044b：后处理链实测（工作站 RTX 3080，PI-109）

`render/post/post-chain.ts`：Babylon `DefaultRenderingPipeline`（Bloom + ACES 色调映射 + FXAA）与 `TAARenderingPipeline`（8 相位抖动 + 历史混合，`samples=8` 对应 `JITTER_PHASES`），`?post=high|medium|low`（`taa/bloom/fxaa/tonemap=0|1` 可单项覆盖）：high = TAA + Bloom + ACES，medium = FXAA + Bloom + ACES，low = FXAA + ACES。曝光由 `ExposureController`（1.5 EV/s 限幅）驱动，目标 EV 由 `setTargetEv` 给定。

场景 `?scene=world&map=carentan&cam=old_town&veg=0&actors=demo&entities=0`，1920×1080，headless Chromium 153，单次 10 s（预热 3 s）；整帧 GPU ms（timestamp / disjoint-timer），无后处理基线 0.25（WebGPU 与 WebGL2）：

| 档 | WebGPU GPU ms | WebGL2 GPU ms | CPU P50 ms（WebGPU / WebGL2） | draw call |
| --- | --- | --- | --- | --- |
| 无后处理 | 0.25 | 0.25 | 2.9 / 2.6 | 65 |
| low（FXAA+ACES） | 0.35（+0.10） | 0.23（-0.02，噪声） | 2.8 / 2.9 | 67 |
| medium（+Bloom） | 0.45（+0.20） | 0.32（+0.07；另一次读到 1.47，GPU 被其它任务占用时的离群值） | 3.5 / 3.1 | 71 |
| high（TAA+Bloom） | 0.48（+0.23） | 0.31–0.38（+0.06–0.13） | 4.2 / 2.4–2.7 | 72 |

- **WebGL2 vs WebGPU 截图（`tools/perf/delta-e.mjs --query`）**：low：均值 ΔE 0.0072、P99 0.43、ΔE>1 的像素 0.005 %；high：均值 0.0070、>1 的像素 0.0017 %（最大 18–37，仅个别边缘像素）。两个后端的 Babylon 管线输出基本一致。
- **静止闪烁（相机静止，连续 6 张截图的逐像素 |ΔL|）**：无后处理 / 仅关闭各项 / FXAA / TAA 全部为 0.0000（0 像素 >2）——静止相机下 TAA 抖动已收敛，这个指标分辨不出差别，**不能当作 TAA 比不用更好的证据**；拖影（运动物体）没有测。

## 限制 / 没做（PI-118）
- **没有运动向量**：地形、植被、PBR 都是自定义 `ShaderMaterial`，不输出速度，Babylon 的 TAA 只做抖动 + 历史累积，运动物体会拖影；运动向量缓冲与按对象重投影、`motion_mask` 没做。
- **没有自动曝光的亮度测量**（64-bin 直方图、2 EV 阶跃 ≤2 s 的实机验证）：`ExposureController` 只由 `setTargetEv` 手动给目标，实机阶跃响应没测。
- 色调映射接在已经是 LDR 的光照输出之上，画面偏灰（`ip.exposure=1`、ACES）；真正的 HDR 光照要等 PBR 材质铺开。
- FXAA 在 TAA 关闭的档位启用，没有做“TAA 失败回退 FXAA”的运行时判断；变体不增长门禁(6)、静止闪烁/拖影截图基线、Bloom 的单独 GPU 分项（Babylon 管线内部）没做。

---

# PI-118 续：TAA 拖影实测与 TAA→FXAA 运行时回退（2026-11-19，工作站 RTX 3080）

## 拖影（`node tools/perf/taa-ghost.mjs`，`?scene=taa`）
相机静止，棋盘格地面 + 一个亮盒每个循环帧移动约 3 px（位置是帧序号的函数，渲染循环跑到第 60 帧后停下截图）；参照图是同一条后处理管线里所有效果都关掉的同一帧（颜色空间一致），`trail` = 盒子最近 14 帧扫过的带状区域（不含盒子本身）里的平均 |ΔL|，`outside` = 其余区域（AA 本身带来的差，噪声底）。原始数据 `reports/m1-taa-ghost.json`，WebGPU 与 WebGL2 的数字一致到小数点后两位：

| 配置 | trail |ΔL| | outside |ΔL| |
| --- | --- | --- |
| 仅 FXAA | 2.08 / 2.01（WebGPU / WebGL2） | 0.84 |
| TAA，无历史夹取 | **4.67** | 0.84 |
| **TAA + 3×3 历史夹取（现在默认）** | **3.11** | 0.83 |
| 同上，factor 0.8 / 0.95 / 0.5 | 3.25 / 3.09 / 3.98 | 0.81 / 0.85 / 0.79 |

历史夹取把拖影从 4.67 降到 3.11（-33 %），仍比 FXAA 的 2.08 多一点（多出的是运动物体后面的残影）。`factor` 取 0.9（0.95 几乎相同，0.5 更差）。

## 运动向量：没有做出来（PI-118）
Babylon 的 `reprojectHistory` 用 PrePass 的速度纹理：本管线里开启它会让渲染循环卡死（WebGPU 与 WebGL2 都试过，已去掉该选项），而且 PrePass 只会让 StandardMaterial/PBRMaterial 写速度，地形、植被和 PBR `ShaderMaterial` 这些自定义着色器不写。要做对需要：每个自定义着色器各加一个“速度”变体（地形要复刻顶点位移与 LOD 形变、植被要复刻摆动与 thin-instance 上一帧矩阵、PBR 要上一帧世界矩阵），渲染到一张 RG16F 目标，再自写 TAA 解析着色器（YCoCg 夹取、速度重投影、`motion_mask`）。相机移动时 Babylon 的 TAA 本来就自动关闭累积（`disableOnCameraMove`），所以静态场景的相机运动没有拖影；残留问题只是“相机静止但物体在动”。

## TAA→FXAA 运行时回退（`post-chain.ts`、`post-guard.ts`，2 项单测）
- 触发：(1) TAA 管线创建抛错（例如不支持）；(2) 帧预算守卫：连续 90 帧窗口的帧间隔 P95 超过 25 ms（默认，`?post_guard=ms`，`0` 关闭）；(3) 手动 `fallbackToFxaa(reason)`。回退后释放 TAA 管线、打开 FXAA，`fallbackReason()` 给出原因，重复调用无副作用。
- 实机验证（WebGPU 与 WebGL2）：60 fps 下默认守卫不触发；`post_guard=1`（人为把上限降到 1 ms）90 帧后触发并切到 FXAA，渲染循环继续（271 帧）；手动回退第二次调用返回 false。

## TAA 速度重投影（`render/post/velocity.ts`，PI-118 续二）
Babylon 的 PrePass 速度会让渲染循环卡死（见上），所以改成自己的速度通道：一个 FLOAT `RenderTargetTexture`，每帧用**不带抖动**的 VP 与上一帧的 VP / 世界矩阵写 NDC 速度；地形、植被/薄实例和普通网格各有一个速度材质（GLSL + WGSL 两份）。`post-chain.ts` 在 `?taa_vel=1` 时强制 Babylon 的重投影模式并把这张纹理绑为 `velocitySampler`，`disableOnCameraMove` 关掉（镜头移动时 TAA 不再关）。默认**仍关**（`taa_vel` 未设时行为不变），因为只在合成场景里测过。

测量（`tools/perf/taa-ghost.mjs --frames 60`，工作站 GPU，WebGPU 与 WebGL2 数值几乎一致；原始数据 `reports/taa-velocity-box.json`、`taa-velocity-pan.json`）：
| 配置 | 方块拖尾 trail ΔL | 其余区域 ΔL | 镜头平移 ΔL |
| --- | --- | --- | --- |
| FXAA | 2.01–2.08 | 0.83 | 0.84 |
| TAA clamp（无速度） | 3.11 | 0.83 | 0（TAA 在镜头移动时被关，等于无抗锯齿） |
| TAA clamp + 速度 | **0.92** | **0.29** | 0.29（TAA 全程开着） |
| TAA clamp + 速度 + flipY | 0.95 | 0.29 | 0.29 |

**诚实说明**：(1) 方块（单个网格）的拖尾从 3.11 降到 0.92，低于 FXAA 的 2.0；(2) flipY 两种取值差别在噪声内（0.92 vs 0.95），这一场景无法分辨方向约定，默认值不改；(3) 平移项的“0”只表示旧路径在镜头移动时关掉 TAA，不是更好；(4) **未做**：角色（薄实例）速度目前按静止处理，所以快速移动的角色仍可能有残影；YCoCg 邻域裁剪与运动遮罩没有实现，沿用 Babylon 自带的历史裁剪；(5) 没有在真实地图场景（地形+植被+载具）测，也没测帧时间开销，所以不改默认值。
