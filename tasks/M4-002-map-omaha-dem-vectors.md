# M4-002 地图：Omaha 高度图、海岸与矢量

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：L
- 依赖：M4-001, M2-001（Carentan 全图 DEM/矢量管线，本任务复用）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §4.2; MAPS §4.5; MAPS §1.2; MAPS §1.5; MAPS §1.6; MAPS §6.1; ARCHITECTURE §9.1; ARCHITECTURE §9.8

## 背景
M4-001 已给出 `meta.json` 与相似变换。本任务产出 Omaha 的地形数据：2049² 16-bit 1 m 高度图（可玩区）+ 1025² 8 m 远景环（8192 m），以及海岸线、潮线、道路、建筑、水系矢量。
关键约束：
- 高度图流程（MAPS §1.2）：IGN LiDAR HD MNT 0.5 m → `gdalwarp -r average` 重采样到 1 m；`h_u16 = round((h-hmin)/(hmax-hmin)*65535)`；`hmin/hmax` 写入 `meta.json`；高程基准使最低水面 ≈ 0 m；可玩区外 256 m 缓冲也要生成。
- 1944 修正（MAPS §1.5）：删除海滨度假屋/现代停车场/纪念场地，恢复 Vierville draw 现代护坡前的形态，恢复 shingle 堤与局部混凝土海堤形态；用 `terrain_edits.geojson`（多边形 + 目标高程/抬升 + 过渡宽度）烘焙进高度图。
- 潮位：静态潮位（固定 06:30，不做潮汐动画，MAPS §4.7）；潮线/湿沙范围取自潮位假设，**潮汐数据未核实**（H-077）；先用 LiDAR 中的 HWL 近似并标注 `tide_verified=false`。
- 水下：近岸水下浅滩需要测深数据；未获得时用平缓坡面合成并标注（见 M4-001 备注）。
- 远景环：北侧大部分为海面，用平面水 + 远处不绘制对岸（MAPS §1.3）；Copernicus GLO-30 作 LiDAR 覆盖外圈。
- 区块：1024 块（64 m），本任务只产出原始栅格与矢量，切片在 M4-005。

## 具体内容
### 要创建/修改的文件
- `data/maps/omaha_dog_green/heightmap_1m.r16` —— 2049×2049 小端 uint16。
- `data/maps/omaha_dog_green/heightmap_far_8m.r16` —— 1025×1025。
- `data/maps/omaha_dog_green/{roads,water,buildings,hedgerows,forests}.geojson` —— 属性按 MAPS §6.2。
- `data/maps/omaha_dog_green/terrain_edits.geojson` —— 1944 修正多边形。
- `data/maps/omaha_dog_green/coast.geojson` —— 海岸线、HWL、LWL、shingle 堤、海堤线（`kind`、`tide_state`）。
- `tools/map-pipeline/map_pipeline/stages/{dem,coast,vectors_osm}.py` —— 扩展 Carentan 阶段以支持海岸与潮线。
- `data/maps/omaha_dog_green/meta.json` —— 补全 `height.hmin/hmax/datum`、哈希。
### 要实现的行为
1. 下载/读取 LiDAR 块（输入在 `/assets/incoming/maps/omaha_dog_green/`，`.gitignore`）；人工下载缺失时任务 `blocked`，按 README §6 登记。
2. 生成高度图（含 256 m 缓冲）与远景环，并对相似变换后的设计框架裁剪。
3. 从 OSM 提取道路/建筑/水系初稿，对照 1944 参考手工删减/补全，标注 `vehicle_ok`（D-1 铺装、D-3 土路，MAPS §4.6 载具路线）。
4. 生成悬崖带/冲沟标注（坡度 > 35° 区域多边形，供 M4-005 导航与 M4-004 载具路线使用）。
5. 微地形增强：潮沙脊（高 0.5–1 m，手工增强 ≤ 0.5 m，MAPS §4.6 视线管理）通过 `terrain_edits` 实现并记录。
6. 输出 `reports/m4-omaha-geo.json`：锚点高程误差、海岸线与历史图对账结论、各数据文件 sha256。
### 接口与数据
- `terrain_edits.geojson` 每个特征：`{"kind":"raise|lower|flatten","target_m":x,"falloff_m":y,"note":"…","source":"gsgs|aerial|manual"}`。
- `coast.geojson` 特征 `kind ∈ {coastline,hwl,lwl,seawall,shingle_bank,cliff_toe}`。

## 不做什么
- 不摆放障碍、WN、铁丝网（M4-003）。
- 不写 zones/spawns/artillery（M4-004）。
- 不切区块、不烘焙碰撞与导航（M4-005）。
- 不渲染海面/潮线材质（M4-014）。
- 不把历史航拍/GSGS 图像入库，只作描摹参考。

## 验收标准
1. 可玩区高度图 2049×2049，文件大小精确 8 396 802 B；远景环 1025×1025 = 2 101 250 B；`map-pipeline validate` 通过。
2. 全部锚点位置的高程与 LiDAR 采样值差 ≤ 1 m（报告 `reports/m4-omaha-geo.json` 列出每个锚点的差值）。
3. 同一输入、同一脚本版本两次运行，输出文件 sha256 完全一致。
4. 海岸线与 HWL 与历史图（人工描摹对账）偏差 ≤ 15 m（B 级精度）的比例 ≥ 90%（对账报告列采样点）；不满足的段落标 `fidelity: C`。
5. 悬崖带多边形覆盖全部坡度 > 35° 连片区域（面积 ≥ 400 m²），脚本交叉检查漏检率 = 0。
6. 全部矢量坐标位于 0..2048（含缓冲前的可玩区）；`water/coast` 无自相交（`shapely.is_valid` 100%）。

## 测试要求
- 单元测试：`pytest tools/map-pipeline/tests/`：量化/反量化往返误差 < 1 mm；`terrain_edits` 烘焙对已知多边形的目标高程误差 < 5 cm；海岸线简化保持拓扑。
- 集成测试：用 Carentan 小样本 + 合成海岸做端到端冒烟（CI 不下载大数据）；真实数据运行在人工机器上。
- 性能基准：全流程在 16 GB 内存机器 ≤ 30 min（记录机器与耗时，非阻断）。
- 回归：把输出哈希写入 `data/maps/omaha_dog_green/hashes.json`，CI 校验。

## 性能预算
- 原始高度图 ≈ 8.4 MB（ARCHITECTURE §9.1）；远景环 ≈ 2.1 MB。本任务不新增运行时预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工输入：DEM/测深下载、1944 修正审核（H-052、H-078）。DEM 未到位 → `blocked`，不得编造数据；可用程序化占位地形先跑通下游（命名 `PH_`，manifest `placeholder: true`），但不得标 done。
- 潮位、气象未核实（H-077）；`tide_verified=false` 必须出现在 `coast.geojson` 属性中。
- 海滩开阔视距是最大视线风险（MAPS §4.7），潮沙脊/残骸/障碍的配合在 M4-003 完成。
- OSM 来源矢量层按 R9 以 ODbL 公开并署名（H-004）。
