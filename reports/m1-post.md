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
