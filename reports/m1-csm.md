# M1-043b：CSM 级联阴影实测（工作站 RTX 3080，PI-109）

`?scene=world&map=carentan&cam=old_town&veg=0&actors=demo&entities=10&csm=0|1`，1920×1080，headless Chromium 153，单次 12 s 采集（预热 3 s）。`render/lighting/csm.ts` 把 Babylon `CascadedShadowGenerator` 接到场景的平行光，档位/级联数/贴图尺寸/PCF 来自 `csm-math.ts`（WebGPU high：4 级 2048/2048/2048/1024、PCF 5；WebGL2：3 级 2048/1024/1024、PCF 3）；建筑与树篱既投射也接收阴影（`?csm=1`，`csm_tier`、`csm_cascades` 可调）。

| 渲染器 | CSM | 级联 | draw call | 三角形 | 整帧 GPU ms | CPU P50 ms | scene_render P50 ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WebGPU | 关 | 0 | 81 | 29 068 | 0.28 | 4.0 | 1.7 |
| WebGPU | 开 | 4 | 777 | 275 708 | 0.49（+0.21） | 8.4（+4.4） | 6.4 |
| WebGL2 | 关 | 0 | 81 | 29 068 | 0.22 | 3.6 | 1.6 |
| WebGL2 | 开 | 3 | 603 | 214 048 | **6.59（+6.4）** | 6.4（+2.8） | 4.1 |

- 阴影 pass 把 draw call 从 81 抬到 600–780（每个级联重画落在其视锥里的 173 个建筑网格和树篱），**CPU 代价约 +3–4 ms，比 GPU 代价更显著**（WebGPU GPU 只 +0.2 ms）。WebGL2 的整帧 GPU +6.4 ms 是 `disjoint-timer-query` 的读数，明显高于 WebGPU，没有进一步查原因（可能是 PCF/贴图尺寸或计时器口径不同）。任务卡里没有 GPU 毫秒阈值数字可对照，这里只给数字。
- **WebGL2 vs WebGPU 截图一致性（同为 3 级联，`--query ... csm=1&csm_cascades=3`，`tools/perf/delta-e.mjs`，`reports/m1-csm-delta-e.json`）**：均值 ΔE 0.088，P99 1.35，ΔE>1 占 1.23 %、>2 占 0.86 %、>5 占 0.46 %，最大 19.2。同场景不开 CSM 的基线是均值 0.019、>1 占 0.17 %，所以后端间的差异主要来自阴影边缘（PCF 实现不同）。
- 级联分割：Babylon 没有显式分割表，只能用 `shadowMaxZ` = 最后一个分割 + `lambda = 0.85` 近似 `csm-math` 的 `[12,40,120,350]` / `[15,60,250]`（PI-117）。`CascadeScheduler`（静止时级联 0/1 隔帧重绘）还没接到 Babylon 的重绘控制。

## 验证方式
`CascadedShadowGenerator` 在 NullEngine（无 GPU 的单测引擎）里无法构造，所以没有 vitest 单测；验证靠真实 GPU 上的两个后端运行、截图和 ΔE（`reports/m1-csm-delta-e.json`）。

## 没做
- **地形与 PBR `ShaderMaterial` 不接收阴影**（自定义着色器不采样阴影图）：画面里只有建筑/树篱/占位角色等 `StandardMaterial` 上有阴影，地面上没有；这是最大的缺口。
- 室内外亮度对比(4)：没有可进入室内；大气 LUT 与云层(5)、IBL KTX2 加载（要真实素材）、`/data` 里的地图光照预设与 `probes.bin`（M1-035 没产出）、`setLightingPreset` 总装：都没做。
