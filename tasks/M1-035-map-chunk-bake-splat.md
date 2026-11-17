# M1-035 地图：区块烘焙、splat 与植被分布

## 元数据
- 里程碑：M1
- 工作流：maps
- 规模：L
- 依赖：M1-031, M1-033, M1-034（**隐含：M1-003 fpak、M1-005 manifest**，见备注）
- 素材需求：无（灰盒即可；地表材质 V-NAT 由 M1-064 接入）；素材状态：placeholder
- 引用文档：ARCHITECTURE §9.1; ARCHITECTURE §9.5; ARCHITECTURE §9.6; ARCHITECTURE §11.2; ARCHITECTURE §11.3; ARCHITECTURE §11.8; MAPS §1.1; MAPS §6.1

## 背景
把地图数据切成 **64 m 区块**（ARCHITECTURE §9.1）并生成运行期可流式加载的产物（MAPS §1.1 步骤 9"打包"、T-SPLAT）。全图 2048 m = 32×32 = 1 024 块；**M1 只烘焙切片矩形（x 380–1560, y 520–1700）覆盖的区块：bx 5..24（20 列）× bz 8..26（19 行）= 380 块**（块索引 `floor(x/64)`；矩形外只靠远景环，不烘焙）。
单块内容（ARCHITECTURE §9.1/§9.5/§9.6）：
- **高度**：65×65 u16 样本（原始 8 450 B），delta 滤波 + brotli ≈ 3–4 KB；
- **splat**：64×64×4 权重（8 位，两张 RGBA 共 8 层：`grass,dirt,mud,road,cobble,rock,sand,forest_floor`，MAPS §6.1/ARCHITECTURE §8.5.5），压缩后 ≈ 8–16 KB；规则：坡度、道路缓冲、水边、洪泛区（沼泽→`mud`）、建筑周边（`cobble`/`dirt`）；
- **碰撞** `collision/c_{bx}_{bz}.bin`：`[heights f32/u16][static_mesh_indices][material_ids]`，Meshopt 压缩；与视觉块分开（客户端 WASM 仅载 5×5 块，内存 ≤ 5 MB；服务器载全图高度场 2049² f32 = 16.8 MB + 静态建筑碰撞）；
- **props 表**：建筑/地标/道具实例（逻辑 ID 如 `bld_*`、`prop_*`，引用 `buildings.geojson`/`landmarks.json`，位置/旋转/缩放，`PH_` 占位标记）；
- **植被实例表**：`(x:u16, z:u16, species:u8, scale:u8, rot:u8)` = 6 B；每块上限：**草 4 096（仅近景）、灌木 256、树 192**；分布由噪声 + 材质层 + 路径排除预计算；树篱以"样条段"表记录（M1-051 实例化）；
- 每块总计约 **30–80 KB**；全图 1 024 块 30–80 MB；M1 切片 380 块 ≈ 11–30 MB。验收线：**单块 ≤ 80 KB**。
- 输出：区块索引（`chunks/index.json`：块 → 文件/条目、LOD 可用性）、**manifest 条目**（逻辑 ID `terrain_carentan/b_{bx}_{bz}`，bundle `map_carentan_medium`，ARCHITECTURE §11.3）与 **`.fpak` 区块包**（按区域分包，每包 1–16 MB，ARCHITECTURE §11.8；不暴露独立标准格式文件）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/map_pipeline/chunker.py` —— 切块（高度、splat、碰撞、props、植被、树篱段）；并行（进程池）；确定性输出
- `tools/map-pipeline/map_pipeline/splat.py` —— 规则引擎：坡度/道路/水边/洪泛/建筑缓冲 → 8 层权重（归一化，每像素和 255）
- `tools/map-pipeline/map_pipeline/vegetation.py` —— 泊松盘/噪声分布 + 排除（道路、建筑、水、树篱缓冲）；种子 = `hash(map_id, bx, bz)`
- `tools/map-pipeline/map_pipeline/collision_bake.py` —— 高度场碰撞块 + 建筑碰撞（灰盒盒体/多盒组合，来自 `landmarks.json` 与建筑标注）
- `tools/map-pipeline/map_pipeline/pack_chunks.py` —— 调用 `tools/asset-pipeline` 的 `pack`（M1-003）把区块条目打成 `.fpak`；写 manifest 片段 `assets/manifests/terrain_carentan.json`（M1-005 格式）
- `data/maps/carentan/m1/chunks/index.json`（进 git，体积小）、`data/maps/carentan/m1/splat_rules.json`
- `tools/map-pipeline/tests/test_chunker.py`、`test_splat.py`、`test_vegetation.py`
- `reports/m1-chunk-bake.json`（块数、各块字节分布 P50/P99/max、总字节、splat 层占比）
### 要实现的行为
1. 高度块边界对齐：相邻块共享边缘样本（65 样本 = 64 m + 1），拼接无缝（边缘样本逐值相等）。
2. 碰撞块与视觉块同源：同一份重采样高度；碰撞块写出与客户端 `PredictSim::loadCollision`（M0-010）一致的数据格式；高度场三角剖分对角线方向与 CDLOD 顶点布局一致（文档记入 `chunks/FORMAT.txt` 说明，避免 §验收 4 的系统偏差）。
3. splat 规则：坡度 > 35° → `rock`；道路中心缓冲（按 `width_m`）→ `road/cobble`（市区 `cobble`）；水边 3 m 带 → `sand/mud`；`flood_marsh` → `mud`；建筑周边 2 m → `dirt`；其余 `grass`，林地 `forest_floor`；输出归一化并带 1.5 m 级抗锯齿过渡（高斯）。
4. 植被：草仅 R_near 渲染故在块内写种子/密度而非全部实例（≤ 4 096）；树按 `forests.geojson` 密度；苹果园/树篱橡树物种 ID 映射表 `species.json`（占位物种，真素材 V-NAT 由 M1-064 绑定）。
5. props：把 `buildings.geojson`（已标注）与 `landmarks.json` 的实例按 64 m 块分桶；跨块建筑记录在其质心所在块并带 `spans[]` 以便相邻块预取。
6. 块字节统计与上限：任何块 > 80 KB → 脚本失败并输出该块的构成明细（高度/splat/碰撞/props/植被）。
7. 打包：区块按 4×4 块一组（≈ 16 块/包）输出 `.fpak`（目标 1–16 MB/包）；manifest 条目含 `bundle:"map_carentan_medium"` 与 `tier` 标签；Medium 外增量块（High/Ultra 更高分辨率）本任务不做。
8. 可重现：输入清单哈希、`random_seed` 固定；两次运行产物哈希一致。
### 接口与数据
区块键 `(bx,bz)`、逻辑 ID `terrain_carentan/b_{bx}_{bz}`；块内布局以 `chunks/FORMAT.txt` 为准（`header{magic,ver,bx,bz,flags}`、`height`、`splat`、`props`、`veg`、`hedge_segments` 段偏移表）。

## 不做什么
- 地形渲染、CDLOD、splat 着色器：M1-049/M1-050；植被/树篱渲染：M1-051；建筑渲染：M1-052；
- 完整 1 024 块（M2）；High/Ultra 额外分辨率；
- navmesh：M1-036；
- 真素材（地表 PBR、树）接入：M1-064；
- 流式状态机与碰撞装载：M1-049。

## 验收标准
1. **区块全部生成**：切片矩形覆盖的 380 块（bx 5..24 × bz 8..26）均存在，缺块数 0（`chunks/index.json` 校验）。
2. **单块 ≤ 80 KB**（含高度、splat、碰撞、props、植被、树篱段）：`reports/m1-chunk-bake.json` 的 `max_bytes ≤ 81920`；P50 在 30–80 KB 区间内（超出写 PLAN_ISSUES）。
3. 总量：380 块总字节 ≤ 30.4 MB（≤ 380×80 KB）；与 ARCHITECTURE §11.4 "地形（高度+splat+颜色）≈ 30 MB" 比较并记录。
4. **抽样 200 条射线碰撞与视觉高度一致**：随机 200 个 (x,z)，视觉高度（块内 65² 双线性、与 CDLOD LOD0 同对角线）与碰撞高度场射线命中 y 的差 ≤ 0.05 m（`max|Δ| ≤ 0.05`，均值 ≤ 0.01）。
5. 相邻块边缘样本完全相等（抽样 100 对边）；splat 每像素权重和 = 255 ±1（抽样 1 万像素）。
6. 植被上限：每块草 ≤ 4 096、灌木 ≤ 256、树 ≤ 192（全部块统计，超出数 0）；任何实例不落入水/建筑/道路（抽样 1 万实例，违规 0）。
7. `.fpak` 通过 M1-003 的 `assert-no-standalone`；manifest 条目与 fpak 索引一致（M1-005 `consistency` 通过）；两次烘焙产物哈希一致。
8. 全流程耗时 ≤ 15 min（8 进程，记录机器）。

## 测试要求
- 单元测试：边缘对齐、splat 规则（每条规则夹具）、植被排除、字节统计、种子确定性。
- 集成：对 M1-031/033/034 输出完整烘焙；与 `sim` 碰撞加载单测联调（`cargo test -p sim chunk_collision_load`）。
- 性能基准：验收 8。
- 回归：`reports/m1-chunk-bake.json` 入库；块格式版本号变更须同步 `FORMAT.txt` 并登记 PLAN_ISSUES。

## 性能预算
- 引用 ARCHITECTURE §9.1（块 ≈ 30–80 KB）、§9.4（出生点周围 81 块 ≈ 10 MB 预取）、§9.5（客户端碰撞内存 ≤ 5 MB）、§11.4（地形 ≈ 30 MB / Medium ≤ 150 MB 总预算）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **INDEX 依赖遗漏**：本任务要"输出 manifest 条目与 fpak 区块包"，实际需要 M1-003（fpak）与 M1-005（manifest 格式）已完成；编号上二者均小于本任务，仅需在依赖列补上。
- 视觉与碰撞对角线方向若不一致，坡度大处会偏差 > 5 cm，须在 CDLOD（M1-049）与 Rapier `HeightField` 构造处对齐；不一致时写 PLAN_ISSUES。
- 规模 L；若超出，可拆 `M1-035b`（植被分布 + props）。
- 上游若为 `partial`（M1-033 缺人工树篱）：本任务照常用 OSM 初稿烘焙，manifest 条目标记 `data_quality:"draft"`，不阻塞联调。
