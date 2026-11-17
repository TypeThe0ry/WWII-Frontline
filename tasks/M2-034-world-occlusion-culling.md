# M2-034 world：遮挡剔除

## 元数据
- 里程碑：M2
- 工作流：world
- 规模：L
- 依赖：M1-052, M2-005
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §9.7; ARCHITECTURE §10.1; ARCHITECTURE §10.3; ARCHITECTURE §13.5.2; ARCHITECTURE §13.5.3; MAPS §3.7

## 背景
Carentan 老城核心约 800×600 m，建筑 300–500 栋（MAPS §3.7），WebGL2 无 Hi-Z，是 M2 的头号渲染风险（MILESTONES §3.8）。ARCHITECTURE §9.7 定义两条路径：
- WebGPU：Hi-Z——上一帧深度 → 下采样金字塔（compute，约 0.3 ms @1080p）；实例包围盒 compute 测试 → indirect draw 压缩，对建筑/树/道具实例剔除。
- WebGL2：视锥 + 距离 + 预计算建筑遮挡体（OBB，工具链从建筑网格生成最多 24 个大平面/块，`occluders.json`，由 M2-005 烘焙）：CPU 把遮挡体光栅化为 64×36 低分辨率深度缓冲（软件光栅约 0.4 ms），对候选实例做"被遮挡体完全覆盖"判定；近似剔除，不保证命中率。
§13.5.2 预算：Hi-Z + 实例剔除 0.5 ms（High）。§13.5.3 主视图 draw call ≤ 1 200（WebGPU High）/ ≤ 900（WebGL2 Medium）/ ≤ 500（Low）。M1-052 已有建筑与可进入室内，M2-005 提供全图区块与 `occluders.json`。

## 具体内容
### 要创建/修改的文件
- `client/src/render/culling/hiz_builder.ts`、`client/src/render/shaders/wgsl/hiz_downsample.wgsl` —— 由上一帧深度构建 Hi-Z 金字塔（max/min 约定与反向 Z 一致）。
- `client/src/render/culling/instance_cull.ts`、`client/src/render/shaders/wgsl/instance_cull.wgsl` —— 实例包围盒视锥 + Hi-Z 测试，输出压缩后的实例缓冲与 indirect draw 参数。
- `client/src/render/culling/cpu_occluder_raster.ts` —— WebGL2：遮挡体 OBB → 64×36 软件深度缓冲 → 实例包围球/盒测试；放入 Worker 或分帧执行，结果延迟 ≤ 1 帧。
- `client/src/world/occluders.ts` —— 加载并按区块管理 `occluders.json`（每块 ≤ 24 个平面/OBB），与 `.fpak`/区块流式（M1-049）对齐。
- `client/src/render/culling/culling_manager.ts` —— 统一入口 `CullingManager`：按路径选择实现，暴露开关与统计（候选数/通过数/draw call 数）。
- `client/src/debug/culling_overlay.ts` —— 调试叠层：draw call 数、被剔除实例数、Hi-Z 可视化（`?cull=off|on|debug`）。
### 要实现的行为
1. WebGPU：每帧 `Hi-Z 构建 → 实例剔除 → indirect draw`；使用上一帧深度（保守：包围盒略放大 + 相机大位移/重生时跳过 Hi-Z 本帧只做视锥）。
2. 剔除对象：建筑外壳、树/大型植被、道具实例（thin instances 的每实例 AABB）；不剔除角色、载具（数量少）、地形节点（CDLOD 已选择）。
3. WebGL2：视锥 + 距离 + OBB 遮挡体；每帧遮挡体 ≤ 64 个（最近区块的 24×邻块过滤），仅对"距离 > 30 m 且在遮挡体后方"的实例剔除；室内相机（玩家在建筑内）关闭自身建筑的遮挡体以免自遮挡。
4. 无"闪现"：剔除保守，禁止视线内可见物体被误剔（见验收 4）。
5. `?cull=off` 关闭用于对比；统计接口供 M2-052 使用。
### 接口与数据
```ts
export interface CullStats { candidates: number; visible: number; drawCalls: number; gpuMs?: number; cpuMs?: number }
export class CullingManager { constructor(engine: AbstractEngine, scene: Scene); setEnabled(on: boolean): void; readonly stats: CullStats }
```
`occluders.json` 格式以 M2-005/MAPS §6.1 文件清单为准（本任务不得改格式，只消费）。

## 不做什么
- 不做遮挡体的离线烘焙（M2-005）；不做 Low 档与 WebGL2 路径的全面优化（M4-020）。
- 不做阴影 pass 剔除优化之外的 CSM 重构；不做动态遮挡体（被摧毁建筑，M3 tile cache 之后）。
- 不做 SSR/AO（M2-030、M4-019）。

## 验收标准
1. WebGPU High 参考机：`pass.hiz_cull`（Hi-Z 构建 + 实例剔除）GPU 时间 P95 ≤ 0.5 ms（ARCH §13.5.2，RTX 3060，1080p）。
2. 老城固定机位（街道纵深 + 教堂广场两处）：开启剔除后主视图 draw call 比 `?cull=off` 降低 ≥ 30%（`CullStats`，脚本比较，两条路径各测）。
3. WebGL2 路径 CPU 遮挡判定耗时 P95 ≤ 0.4 ms/帧（`performance.now` 分段，参考机）。
4. 误剔除检查：6 个固定机位开/关剔除截图逐像素差异像素占比 ≤ 0.1%（`pixelmatch`，同相位）；任一机位超限则失败。
5. 室内/门口机位无自遮挡：玩家站在建筑内与门口时，该建筑实例 100% 可见（单测 + 截图）。
6. 老城机位帧时间不劣于关闭剔除（P95 帧时间差 ≤ +0.2 ms，防止剔除开销抵消收益）。

## 测试要求
- 单元测试（vitest）：OBB 光栅化与保守性（随机实例对比暴力判定，漏剔除率可有、误剔除率 = 0）；视锥/包围盒函数；统计累加。
- 集成/无头测试：Playwright 两条路径 `?cull=on/off` 截图对比与 draw call 采集，输出 `reports/render/culling/summary.json`。
- 性能基准：GPU timestamp 与 CPU 分段采样 1 000 帧。
- 回归：M1-052 建筑/室内测试继续通过；视觉基线交 M2-050。

## 性能预算
- ARCHITECTURE §13.5.2：Hi-Z + 实例剔除 0.5 ms（High）；§13.5.3 draw call 上限 1 200 / 900 / 500；WebGL2 路径渲染提交 +2.5 ms 已含在 §13.5.1 备注。
- Hi-Z 金字塔显存 ≈ 1080p 深度的 1/3（≈ 3 MB），计入 §13.4 渲染目标项。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright 作业）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Hi-Z 依赖上一帧深度，快速转身时会有保守误差；用包围盒外扩 + 大位移时跳过。TAAU（M2-032）抖动会影响深度，需用未抖动深度或抖动补偿。
- WebGL2 遮挡体质量取决于 M2-005 烘焙；若命中率不足以达成 -30%，写 PLAN_ISSUES（建议增加遮挡体数量或调整 64×36 分辨率），不得放宽数字。
- 灰盒建筑的 footprint/门洞已按 ASSET_GAPS §3.2 保留，遮挡体应基于灰盒墙体生成。
