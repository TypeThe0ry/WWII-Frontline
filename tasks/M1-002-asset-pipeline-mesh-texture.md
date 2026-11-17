# M1-002 素材管线：网格 LOD、纹理与碰撞简化

## 元数据
- 里程碑：M1
- 工作流：assets
- 规模：L
- 依赖：M1-001
- 素材需求：无（灰盒即可；用 CC0 样例）；素材状态：n.a.
- 引用文档：ARCHITECTURE §11.1; ARCHITECTURE §11.4; ARCHITECTURE §11.8; ARCHITECTURE §13.5; ASSET_GAPS §3.1

## 背景
承接 M1-001 的 `work/{asset_id}/normalized.gltf`，实现 ARCHITECTURE §11.1 的步骤 3–6：LOD、纹理、网格压缩、碰撞简化。输出仍是 `work/` 下的**中间产物**（.glb/.ktx2 仅存在构建机，R11），由 M1-003 打进 `.fpak`。关键数字：
- LOD 比例 1 / 0.5 / 0.25 / 0.1，使用 meshoptimizer simplify，**保留 UV 边界**。
- 纹理：颜色 → ETC1S（背景/大面积）或 UASTC（角色/武器/UI）；法线 → UASTC（RG 双通道）；ORM 打包（R=AO，G=Roughness，B=Metallic）；生成 mip。工具 `toktx`（`gltf-transform` 编排）。
- 网格：`gltf-transform` dedup、prune、meshopt；量化 pos 16-bit、uv 16-bit、normal 8-bit oct。
- 碰撞：视觉网格 → 简化碰撞网格/凸分解（载具/建筑）。
- 武器两套精度（D-FP）：第一人称 25k–60k 三角、第三人称 5k–12k【待调参】——管线必须支持同一 asset 产出两套独立 LOD 链，条目名后缀 `_fp` / `_tp`。
- 每个处理动作前须经 M1-001 的 `assertAllowed`（LOD→`LOD`、纹理→`TEXCOMP`）。

## 具体内容
### 要创建/修改的文件
- `tools/asset-pipeline/src/lod.ts` —— `generateLods(gltf, ratios=[1,0.5,0.25,0.1], opts)`，内部调用 meshoptimizer（`meshoptimizer` npm 的 simplify），锁定 UV 边界/材质边界顶点
- `tools/asset-pipeline/src/textures.ts` —— ORM 打包、mip、`toktx` 调用封装、格式选择策略（读取 `asset-class.json` 的 `texture_profile: bg|hero|ui|normal`）
- `tools/asset-pipeline/src/mesh-compress.ts` —— gltf-transform 流水线（dedup/prune/quantize/meshopt）
- `tools/asset-pipeline/src/collision.ts` —— `simplifyCollision(gltf, {mode:'trimesh'|'convex', maxTris})`
- `tools/asset-pipeline/src/stats.ts` —— 输出每 LOD 的三角数、顶点数、字节数、纹理格式/尺寸 → `work/{asset_id}/stats.json`
- `tools/asset-pipeline/asset-class.schema.json` —— 每素材的处理参数 schema（texture_profile、lod_ratios 覆盖、collision 模式、`first_person: bool`）
- `tools/asset-pipeline/test/`、`fixtures/`（CC0/自造小样例，含一个带 UV 接缝的带纹理立方体与一个约 20k 三角的球体）
### 要实现的行为
1. 对每个资产按比例生成 LOD0..LOD3；每级写 `lodN.glb`（中间产物）并记录 `tris`、`error`（meshoptimizer 的 simplify error）。
2. UV 边界保持：简化时使用 `LockBorder` 并在 UV 接缝处加权；输出后抽样验证接缝处顶点数不减少超过阈值。
3. 纹理：按 profile 选择 ETC1S/UASTC；法线图强制 UASTC 并输出 RG 双通道；有 mip；NPOT 贴图报警并 pad 到 4 的倍数。
4. ORM 打包：输入三张单通道/灰度图（AO/Roughness/Metallic），输出单张 RGB（R=AO、G=Rough、B=Metal）；缺 AO 时填 255。
5. 网格量化后（pos16/uv16/normal-oct8）必须保持包围盒误差 < 0.1% 对角线。
6. 碰撞：`trimesh` 简化到 ≤ `maxTris`（默认 载具 400 / 建筑 800）；`convex` 模式输出 ≤ 32 个凸块；碰撞网格不带 UV/材质。
7. 把武器资产的 `first_person` 标记贯穿 LOD 与统计（两套独立链）。
### 接口与数据
```ts
interface LodResult { lod:number; tris:number; bytes:number; simplifyError:number; file:string }
function processMesh(assetId:string, cls:AssetClass): {lods:LodResult[]; collision?:CollisionResult; textures:TexResult[]}
```
`stats.json` 供 M1-005 manifest（`lods[].tris`、`bytes`）使用，字段名与 ARCHITECTURE §11.3 的 `lods[]` 一致（`lod`、`tris`、`bytes`）。

## 不做什么
- `.fpak` 打包、哈希命名、manifest：M1-003、M1-005。
- 动画重定向/换骨架权重传递：属 M1-064（素材接入），本任务只需透传骨骼与动画。
- 运行期 GPU 上传与 KTX2 转码：M1-006。
- 建筑遮挡体 OBB 预计算（WebGL2）：M2。

## 验收标准
1. CC0 样例（≥ 1 个带纹理模型）处理后，LOD0 中间产物总字节 ≤ 原始 glTF+PNG 总字节的 25%（`stats.json` 对比脚本 `check-size-ratio`）。
2. 各 LOD 三角数相对 LOD0 的比例落在目标 ±5% 内（目标 0.5 / 0.25 / 0.1；单测以 20k 三角球体断言）。
3. UV 边界保持：带 UV 接缝的立方体经 LOD1 后，UV 接缝处的边界顶点保留率 ≥ 95%（单测）。
4. 法线贴图输出 KTX2 的 `vkFormat`/basis 模式为 UASTC 且通道数 = 2；颜色贴图 profile=`bg` 为 ETC1S（用 `ktx info` 或 basis 头校验）。
5. 量化后包围盒对角线相对误差 < 0.1%。
6. 碰撞简化：2 万三角样例 → trimesh 模式 ≤ 400 三角；convex 模式块数 ≤ 32。
7. 越权保护：`allowed_processing` 不含 `LOD` 的素材运行 LOD 被拒（复用 M1-001 `assertAllowed`）。
8. 单个中等样例（≤ 5 万三角 + 4 张 1024² 贴图）全流程 ≤ 30 s（`time` 实测，记录机器型号）。

## 测试要求
- 单元测试：`generateLods` 比例、UV 锁边、ORM 通道映射、格式选择策略表、碰撞三角预算。
- 集成：`pnpm -C tools/asset-pipeline test:e2e`：ingest → normalize → process → 检查 `stats.json` 与产物存在。
- 性能基准：验收 8 的耗时记录于 STATUS。
- 回归：fixtures 输出的 `stats.json` 作为快照（容许 ±1% 数值抖动）。

## 性能预算
- 构建期工具，无运行期预算；输出体积须服务于 ARCHITECTURE §11.4（Medium 首图 ≤ 150 MB）与 §13.5 三角预算——LOD 比例不得自行放宽。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `toktx`/KTX-Software 与 meshoptimizer 需在 CI 镜像中安装；版本与许可证登记到 LICENSES.md §5（第三方软件依赖）。网络受限时在 PLAN_ISSUES 记录。
- 25% 体积比依赖样例内容（PNG 已压缩的纹理比例会低于预期）；样例选带 2048² PNG 的 CC0 材质模型。
- 凸分解算法（V-HACD 类）的选型属实现细节，若需新依赖先写 PLAN_ISSUES。
