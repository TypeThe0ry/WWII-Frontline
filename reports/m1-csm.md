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

---

# PI-117 续：自写 CSM，地形与 PBR 材质接收阴影（2026-11-19，工作站 RTX 3080）

Babylon 的 `CascadedShadowGenerator` 把矩阵和阴影图数组都设成私有，自定义着色器没法采样，所以另写了一套（`?csm=custom`，上面的 `?csm=1` Babylon 版保留给 StandardMaterial 接收者）：
- `lighting/cascades.ts`（纯数学，5 项单测）：每个级联用视锥切片的外接球（旋转稳定）+ 正交窗口，窗口中心按阴影贴图纹素吸附（相机移动不闪烁）。
- `lighting/custom-csm.ts`：每级联一张 RGBA32F 渲染目标（NEAREST），正交相机，投射体用深度材质绘制（树篱用 thin-instance 版）；每帧重新拟合并把 `uShadowVP0..3 / uShadowSplits / uShadowInfo / uShadowTexel / uShadowWorld / uShadowParams` 与 4 张深度图推给接收者材质。
- `lighting/csm-shader.ts`：GLSL 与 WGSL 双语言的接收者片段（3×3 PCF、法线偏移、按视距选级联；WGSL 用 `textureLoad`，所以级联分支可以非一致），**地形片段着色器与 PBR 光照变体**（不增加变体数）都已接入；没有 CSM 时绑定 1×1 虚拟深度图、`uShadowInfo.x=0` 关闭。
- 太阳方向统一为 (0.45, -0.8, 0.35)（行进方向，地形着色器原本的“指向太阳”常量取反）：世界场景的 DirectionalLight 和 PBR 默认光也改成这个，之前三处方向不一致。

## 正确性（`tools/perf/shadow-truth.mjs`，CPU 射线对地面点逐点验真）
在 old_town 视角取相机可见的地面点（半径 100 m、步长 2 m，107 个点，31 个应在建筑阴影里），用建筑棱柱沿太阳方向做射线投射得到“应有阴影”，再读渲染图里该点是否明显变暗（亮度 < 亮点中位数的 55 %）：
- WebGPU：**一致率 97.2 %**（阴影 28 命中 / 3 漏 / 0 误报）；WebGL2：**96.3 %**（27 命中 / 4 漏 / 0 误报）。漏的都是阴影边缘附近的点（PCF 软边与 55 % 阈值）。原始数据 `reports/m1-csm-custom-truth.json`。
- 首版在 WebGPU 上一致率只有 62 %（0 命中、6 误报）：阴影图查询多翻转了一次 V；这个验真脚本正是用来发现并修正它的（现在两个后端都不翻转，`uShadowInfo.y` 保留为开关）。
- WebGL2 vs WebGPU 同配置（都用 3 级联）整帧截图 ΔE（`reports/m1-csm-custom-delta-e.json`）：均值 0.024、P99 0.54，>1 的像素 0.25 %、>5 的 0.15 %（对比 Babylon 版 CSM 的 1.2 %）。PBR 测试场景（`?scene=pbr&csm=custom`，球投射到 PBR 地面与彼此）：均值 0.023，>1 的像素 0.2 %。

## 开销（old_town 1080p，3 次交替，整帧 GPU / CPU P50）
| 渲染器 | CSM | 级联 | 整帧 GPU ms | CPU P50 ms | scene_render ms | draw call | `csm_update` ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WebGPU | 关 | 0 | 0.28–0.31 | 3.8–4.4 | 1.5–1.7 | 65 | — |
| WebGPU | 自写 | 4 | **0.75–0.82（+0.5）** | **7.6–8.2（+3.8–4.4）** | 5.7–5.9 | 761 | 0.1 |
| WebGL2 | 关 | 0 | 0.19–0.24 | 3.2–3.9 | 0.9–1.1 | 65 | — |
| WebGL2 | 自写 | 3 | **0.42（+0.2）** | **5.6–6.1（+2.2–2.7）** | 3.2–3.5 | 587 | 0.1 |

和 Babylon 版相比：WebGPU GPU 0.49 → 0.75–0.82（接收者现在包括地形，每个地面像素多 9 次 PCF 读取）；WebGL2 GPU **6.59 → 0.42**（上一版那次读数明显偏高，换成自写后不再出现）。CPU 的主要成本仍是阴影 pass 的 draw call（每个级联重画落在其窗口里的约 174 个建筑网格 + 树篱）。

## 没做
投射体只有建筑与树篱：占位角色/坦克/植被不投射阴影（实例网格的深度材质覆盖与植被的摆动没做）；`CascadeScheduler`（静止时级联 0/1 隔帧）没接；级联之间没有混合带，PCF 固定 3×3；没有接 Babylon 的 StandardMaterial 接收者（它们用 `?csm=1` 的 Babylon 版）。
