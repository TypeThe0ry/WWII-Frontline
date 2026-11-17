# M4-020 render：Low 档与 WebGL2 路径优化

## 元数据
- 里程碑：M4
- 工作流：client-render
- 规模：L
- 前置依赖：M4-017, M2-034
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §10.3; ARCHITECTURE §13.5.3; ARCHITECTURE §13.1; ARCHITECTURE §9.7; ARCHITECTURE §10.2; ARCHITECTURE §20

## 背景
Low 档（Iris Xe 级核显，720p 30 fps）与 WebGL2 路径是一等公民（ARCHITECTURE §10.3 末行）。M2-034 已有 WebGL2 的视锥+距离+遮挡体剔除；M4-017 已落地 Low 档参数。本任务做**路径级优化**，使 Low/WebGL2 达到 §13.5.3 预算。
预算与差异（§13.5.3 / §10.3 / §13.1）：
- Low：主视图 draw call ≤ **500**，阴影 draw call ≤ 250，可见三角形 ≤ **0.8 M**，阴影三角形 ≤ 0.3 M，动态角色 ≤ 24，可见载具 ≤ 5，粒子 ≤ 300，材质变体 ≤ 40。WebGL2 Medium：≤ 900 draw、2.0 M 三角形、40 角色。
- WebGL2 阴影默认 3 级 1024–2048；Low 为 3×1024、PCF 1×1、距离 150 m；AO 关；体积雾→高度雾；超分 FSR1 式 EASU/RCAS 空间放大 + 轻 TAA（片元全屏 pass）。
- WebGL2 draw 提交开销大：渲染提交 CPU +2.5 ms（§13.5.1），要求更激进合批（Babylon `freezeMaterials/freezeActiveMeshes`、`Thin Instances`、静态网格合并）。
- 遮挡体：预计算 OBB ≤ 24/块，CPU 软件光栅 64×36 ≈ 0.4 ms（§9.7）。
- 粒子/植被/动态灯削减：草 35%/20 m、道具 40%、动态灯 2、树 impostor 近（§13.1）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/webgl2/BatchingPolicy.ts` —— 静态网格按材质/区块合并、冻结策略、实例化阈值。
- `client/src/render/webgl2/FsrUpscale.ts` + `shaders/{easu,rcas}.glsl` —— FSR1 式空间放大（EASU/RCAS 原创实现，不拷贝商业着色器）。
- `client/src/render/webgl2/ShadowLow.ts` —— 3 级 CSM（1024）、PCF 1×1、距离 150 m。
- `client/src/render/webgl2/LodBias.ts` —— 角色/载具 LOD 偏置 +1、植被距离削减。
- `client/src/render/webgl2/CullingBudget.ts` —— 每帧可见实例上限与降级策略（超限时按距离裁）。
- `client/tests/render/webgl2_budget.spec.ts`。
### 要实现的行为
1. 开启 Low 档（WebGL2）后，对 Carentan/Omaha/Foy 的预设机位逐一统计 draw call、可见三角形，超限即用降级链（先削草/道具，再减 LOD，再减阴影距离）并记录。
2. FSR1 式放大：内部 67–75% → 720p 输出；RCAS 锐化强度读 `data/render/fsr.json`（`initial`）；与轻 TAA（可关）兼容。
3. 合批：每区块一个静态合并网格/材质；动态物体（角色/载具）不合并；冻结材质与活动网格仅在无动态变化时。
4. 粒子 ≤ 300（含降雪/烟/枪口焰），超限丢弃最远粒子。
5. 动态灯 ≤ 2；室内探针光照保持。
6. 对比基线：输出每地图 Low 档统计报告 `reports/perf/m4-low-stats.json`。
### 接口与数据
- `interface BudgetReport { drawCalls: number; shadowDrawCalls: number; visibleTris: number; shadowTris: number; characters: number; vehicles: number; particles: number; variants: number }`；通过 `QualityManager`/`render/stats` 暴露给 Playwright。

## 不做什么
- 不做 WebGPU 路径的优化（沿用 M2-034）。
- 不引入新的 WebGL2 专属材质变体（变体 ≤ 40）。
- 不做性能矩阵实机测量（M4-041），本任务提供统计与 CI 近似基准。
- 不做 SSR/TAAU（WebGL2 本就无）。

## 验收标准
1. Iris Xe 级机器 720p WebGL2 Low：三图预设机位 **≥ 30 fps（P95 帧时间 ≤ 33.3 ms）**（需人工/参考机；缺则 `未测`）。
2. 三图各 12 机位的 Low 档统计：draw call ≤ 500、可见三角形 ≤ 0.8 M（Playwright 软件 WebGL2 可测，`BudgetReport` 逐机位 100% 达标）。
3. 阴影 draw call ≤ 250、阴影三角形 ≤ 0.3 M（同上）。
4. 材质变体 ≤ 40（Low），CI 断言。
5. FSR1 式放大主观质量：与"双线性放大"相比，固定机位 SSIM（对原生 720p 参考）提升 ≥ 0.02；RCAS 无环状伪影（人工看图，记录）。
6. 渲染提交 CPU（WebGL2 Low，参考机）≤ 7.0 ms（§13.5.1 基线 4.5 + 2.5；`performance.measure`，缺机则 `未测`）。

## 测试要求
- 单元测试：`vitest`：降级链顺序与幂等；合批策略对合成场景的 draw call 数；FSR 参数边界。
- 集成/无头测试：Playwright 强制 WebGL2（`?backend=webgl2`），三图 × 12 机位统计；CI 软件渲染仅验证 draw/三角形数，不验证 fps。
- 性能基准：`pnpm -C client bench:webgl2-low` 输出 CPU/GPU ms，机型写入报告。
- 回归：Low 档视觉基线并入 M4-040。

## 性能预算
- Low：draw ≤ 500、阴影 draw ≤ 250、可见三角形 ≤ 0.8 M、阴影三角形 ≤ 0.3 M、角色 ≤ 24、载具 ≤ 5、粒子 ≤ 300、变体 ≤ 40（§13.5.3）；CPU 渲染提交 +2.5 ms 约束（§13.5.1）；JS+WASM 内存 ≤ 0.9 GB（§13.3）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：建议拆分线：①合批/统计/降级链 ②FSR1 放大。若超会话，按 README §5.6 建 `M4-020b`。
- Iris Xe 达标是 M0 风险（ARCHITECTURE §20.1 第 5 项）的延续；若实测不达，依 H-039 默认假设：Low 降为 720p/25–30 fps 并标注"最低体验"。
- FSR1 的算法为公开论文思想的原创实现，禁止复制厂商着色器源码（D-LIC/依赖许可证政策，H-037）。
