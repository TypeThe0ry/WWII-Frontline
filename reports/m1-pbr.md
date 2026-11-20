# M1-042b：PBR 材质实测（工作站 RTX 3080，PI-109）

场景 `?scene=pbr`（固定相机，10 个球 = 金属/非金属 × 5 档粗糙度，法线贴图 + 细节图平面，alpha 测试平面，混合平面），1280×720，headless Chromium 153，WebGPU 与 WebGL2 各一次。着色器 `render/materials/pbr-shader.ts`（GLSL/WGSL 双语言，4 个宏展开成源码文本）：Lambert + GGX（Smith-Schlick、Schlick 菲涅尔），1 盏平行光 + 半球环境光；材质由 `pbr-material.ts` 的 `MaterialFactory` 每变体建一个 `ShaderMaterial`。

## 预热（验收 2/3）
`prewarmVariants` 编译全部 32 个合法变体（`variant-count` 数字同 M1-042a：opaque 16 / alpha 8 / depth 4 / shadow 4）：

| 渲染器 | 变体数 | 总耗时 | 单个最长 | 失败 |
| --- | --- | --- | --- | --- |
| WebGPU | 32 | 15.8 ms（另一次 16.3 ms） | 1.7–1.9 ms | 0 |
| WebGL2 | 32 | 69.7 ms（另一次 105 ms） | 10.6–17.8 ms | 0 |

（单次页面加载，3 次内的上下浮动如括号；WebGPU 的“就绪”在管线异步创建前返回得早，所以这里是 `isReady` 的耗时，不是首帧不卡顿的证明。）

## WGSL/GLSL 视觉回归（验收 4）：`node tools/perf/delta-e.mjs`
两张截图（WebGPU vs WebGL2）逐像素 CIE76 ΔE（`reports/m1-pbr-delta-e.json`）：921 600 像素，**均值 0.018，P99 = 0，P99.99 = 24.2，最大 28.2**；ΔE>1 的像素占 0.195 %，>2 占 0.185 %，>5 占 0.091 %。差异几乎全在几何与 alpha 测试孔洞的边缘像素（两个后端的光栅化/采样边界不同），球体与平面内部一致。验收阈值在任务卡里没有数字，这里只给数字不给 PASS。

## 材质冻结基准（验收 5）：`?scene=pbr&pbr_many=3000[&pbr_freeze=1]`
3000 个盒子、两种变体、固定世界矩阵，可见 draw call 181。`scene.freezeMaterials()` + `scene.freezeActiveMeshes()`（注意：同时冻结了可见性判定，相机静止时才成立）。每组单次 8 s，P50 毫秒，2 次交替：

| 渲染器 | scene_render 未冻结 | scene_render 冻结 | CPU P50 未冻结 → 冻结 |
| --- | --- | --- | --- |
| WebGPU | 4.5 / 5.0 | 1.4 / 1.5 | 5.4–5.9 → 2.0–2.3 |
| WebGL2 | 4.4 / 3.8 | 0.8 / 1.0 | 4.8–5.1 → 1.6–2.0 |

## 单测
`pbr-shader.test.ts`（4 项）：两种语言的 attribute/uniform/sampler 接口一致、每个声明的 uniform 都在源码中用到、宏只在开启时展开、depth/shadow 无光照输入。`delta-e.test.mjs`（2 项）。

## 未做
真实 `.ktx2` 纹理与 mip 分层（等 M1-064）、`variant-count` 的 CI 接线（`local-ci.sh` 未加该步骤）、材质在 CSM 阴影 pass 里的实际使用（M1-043b）、PBR 材质对 M1 场景现有占位 ShaderMaterial 的替换。

## 变体统计与 CI 门禁更新（B3-C3 / B6-variantci，2026-11-20）
- **自写着色器纳入变体注册表**：`client/src/render/materials/variants.ts` 已定义 `CUSTOM_VARIANTS` 5 项——自写 CSM 深度通道（`csm:plain`、`csm:instanced`，`custom-csm.ts`）与 TAA 速度通道（`velocity:plain`、`velocity:instanced`、`velocity:terrain`，`velocity.ts`），经 `VariantRegistry.registerCustom` 统一计入上限并在超限时抛错。
- **CI 门禁接线**：`scripts/local-ci.sh ts` 已运行 `pnpm -C client run variant-count`（随后运行 `pnpm -C client test`），每后端门禁实测 **High 37/60**（PBR 32 + custom 5）、**Low 25/40**（PBR 20 + custom 5），并含 `custom-csm.ts`/`velocity.ts` `registerCustom` 与 `CUSTOM_VARIANTS` 一致性断言（`variants.test.ts` 8/8 通过）。
- **状态说明**：上文“未做”中的 `variant-count` CI 接线已完成；其余项（真实 `.ktx2` 与 mip 分层、PBR 替换全场景占位材质、RTX 3060 / Iris Xe 参考机预热实测，见 H-021 / M1-064 / PI-109）仍未完成。
