# M0-023 client/render：CDLOD 地形

## 元数据
- 里程碑：M0
- 工作流：client-render
- 规模：L
- 依赖：M0-020, M0-009
- 素材需求：无（V-NAT-007 CC0 可选）；素材状态：n.a.（M0 程序化着色，不用外部贴图）
- 引用文档：ARCHITECTURE §9.2; ARCHITECTURE §9.8; ARCHITECTURE §13.5.2; ARCHITECTURE §10.3; ARCHITECTURE §9.1

## 背景
CDLOD（Continuous Distance-Dependent LOD）地形：四叉树 + 每个节点共用同一个 **65×65 顶点网格实例**（8 192 三角形），实例化绘制；顶点着色器按到相机距离做 **morph** 保证 LOD 间无裂缝（ARCHITECTURE §9.2）。M0 范围：256 m 近景块 + 8 km 低精度远景环。远景环 = 同一棵四叉树根节点 8192 m 的上层节点，使用 8 m 高度图采样（§9.8）。
LOD 表（§9.2）：LOD0 网格 1 m/节点边长 64 m/选择距离 0–110 m；LOD1 2 m/128 m/110–220 m；LOD2 4 m/256 m/220–440 m；LOD3 8 m/512 m/440–880 m；LOD4 16 m/1 024 m/880–1 760 m；LOD5 32 m/2 048 m/1 760–3 520 m；LOD6 64 m/4 096 m/3 520–7 040 m（远景环，使用 8 m 高度图采样）；范围按 2× 递增，morph 区取每级范围的最后 30%：`morph_k = smoothstep(range*0.7, range, dist)`，把奇数顶点高度插值到相邻偶数顶点。节点选择在 CPU 递归（≈ 200–400 节点/帧，**< 0.3 ms**），视锥 + 距离，输出实例缓冲一次上传。M0 用 R16/R32F **单张**高度纹理（近景 257² 与远景 1025²）；texture array 与区块槽位属 M1-049/050。
预算：地形 GPU **≤ 2 ms**（RTX 3060，1080p；不透明主通道总预算 4.0 ms，ARCHITECTURE §13.5.2）。WebGL2 路径同样可渲染（GLSL ES 3.00，`texelFetch`，**手写双线性**，不依赖浮点纹理线性过滤扩展）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/terrain/{cdlod-quadtree.ts,grid-mesh.ts,terrain-data.ts,terrain-material.wgsl.ts,terrain-material.glsl.ts,terrain.ts,index.ts}` 与 `*.test.ts`。
- `client/public/maps/m0/` 同步脚本（与 M0-022 共用）扩展复制 `heightmap_1m.r16`、`heightmap_far_8m.r16`、`meta.json`。
### 要实现的行为
1. **数据加载**：`fetch` `meta.json` 与两张 `.r16`（小端 u16），按 `hmin/hmax` 反量化为 `Float32Array` → `R32F`（WebGPU `r32float` 用 `textureLoad`；WebGL2 `R32F` + `texelFetch`）。
2. **四叉树**：根 = 8192 m 方块，中心 = 近景块中心；节点递归细分直到 64 m；`selectNodes(camera, frustum) -> InstanceList`：对每个节点按 §9.2 LOD 范围判断细分，视锥外剔除（AABB 用 `hmin/hmax` 估计高度范围）；**近景块之外最小 LOD 取 LOD3（8 m）**（远景高度图只有 8 m 分辨率）。
3. **网格实例**：单个 65×65 顶点网格（8 192 三角形，索引共享），实例属性 `(offset.xz, scale, lod)`；Babylon 用 `thin instances` 或自定义实例缓冲，绘制次数 = 1 个 draw（实例化）或每 LOD 一次。
4. **morph 与采样**：顶点着色器按节点 LOD 的范围参数计算 `morph_k`，奇数顶点高度向相邻偶数顶点插值；高度取自近景纹理（节点完全在近景块内）否则取远景纹理；法线用中心差分重建；`skirt`（裙边）可选以遮盖剩余 T 形接缝。
5. **程序化着色**：无外部贴图：基色按坡度/高度混合（草—土—岩），每 16 m 淡网格线与 64 m 区块线（便于观察 LOD/区块），按 `?terrain_debug=lod|morph|normal` 显示 LOD 色/morph 系数/法线。
6. **调试相机**：复用 M0-020 的 `?fly=1`；叠层显示节点数、各 LOD 数量、选择耗时（`performance.now()`）。
### 接口与数据
- `Terrain.update(camera)`（每帧）、`Terrain.heightAt(x,z)`（CPU 端双线性，供 M0-024 植被撒点与 M0-025 角色贴地，**必须与 `sim` 的 `height_at` 在 1e-3 m 内一致**）、`Terrain.dispose()`。

## 不做什么
- 不做 splat 多层材质、宏观色图、triplanar（M1-050）；不做区块流式/texture array 槽位（M1-049）。
- 不做阴影、水面、遮挡剔除（M1-043、M2-035、M2-034）。
- 不做植被（M0-024）。

## 验收标准
1. **GPU 时间**：RTX 3060 / 1080p / WebGPU，仅地形（`?scene=terrain`）固定机位（M0-030 定义的 3 个机位各测）下 GPU 帧时间中位数 ≤ 2 ms（叠层 GPU timer；需参考机，缺则 `未测`）。
2. **节点选择 CPU**：vitest bench：随机 1000 个相机位姿，`selectNodes` 中位数 < 0.3 ms，节点数中位 ∈ [150, 450]（§9.2 的 200–400 量级）。
3. **无裂缝（数值验证）**：单元测试在 CPU 复现 morph 公式，对随机相邻节点对（相同或相差 1 级 LOD）沿共享边采样 10 000 点，共享边两侧高度差 ≤ 1e-4 m（`terrain_morph_seam.test.ts`）；人工在 `?terrain_debug=lod` 下绕场景一圈截图，无肉眼可见裂缝/洞（截图路径入 STATUS）。
4. **一致性**：`Terrain.heightAt` 与 sim-wasm 的高度查询（M0-014 暴露 `Predictor` 内部高度，或通过测试用 Node 调用 `sim`）在 1000 个随机点上误差 ≤ 1e-3 m。
5. **双路径**：`?renderer=webgl2` 下地形可渲染，无着色器编译错误，LOD/morph 调试模式均可用；WebGL2 路径帧时间记录（Iris Xe 参考机；缺则记 `未测`）。
6. 加载：近景 + 远景纹理总上传 < 4 MB；首次渲染 ≤ 500 ms（本机，记录）。
7. 远景环外缘：相机位于 256 m 块中心，8 km 外的地形边缘可见且不出现近景/远景交界的高度跳变（交界处两侧高度差 ≤ 0.5 m 的数值检查 + 截图）。

## 测试要求
- 单元测试：四叉树细分距离阈值、AABB 剔除、morph 数值、`heightAt` 双线性；`.r16` 解码字节序。
- 集成/无头测试：Playwright 软件渲染下 WebGL2 路径加载地形，截图非空且无 console 错误。
- 性能基准：验收 1/2（结果交 M0-030）。
- 回归：视觉基线从 M2-050 起；M0 只存截图作为参考。

## 性能预算
地形 GPU ≤ 2 ms（RTX 3060，1080p；INDEX 与 ARCHITECTURE §13.5.2 不透明主通道 4.0 ms 的子预算）；节点选择 CPU < 0.3 ms（§9.2）；主视图 draw call 随 §13.5.3（≤ 1 200 / WebGL2 Medium ≤ 900）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 1 m 高度图在近距离坡面有"阶梯感"（ARCHITECTURE §5.9 风险）：M0 只用法线与程序化网格线缓解，不做细节法线层。
- WebGL2 顶点纹理读取数量限制与 `R32F` 过滤差异是主要平台风险；若 WebGL2 无法满足需用 `RGBA8` 打包高度（写 PLAN_ISSUES 并记录差异）。
- 地形材质名与节点按 ASSET_GAPS §3 约定：`PH_V-NAT-007`（地表材质占位，需求 ID V-NAT-007 为 CC0 免费材质，M1 起接入）。
