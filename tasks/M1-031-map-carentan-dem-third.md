# M1-031 地图：Carentan 1/3 区域高度图

## 元数据
- 里程碑：M1
- 工作流：maps
- 规模：M
- 前置依赖：M1-030
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §1.2; MAPS §1.3; MAPS §3.8; MAPS §6.1; MAPS §6.2; ARCHITECTURE §9.1; ARCHITECTURE §9.8

## 背景
`[人工：DEM 下载]` 范围：人类把 IGN LiDAR HD 的 MNT 栅格（0.5 m，GeoTIFF；下载入口见 MAPS §2.1：`https://diffusion-lidarhd.ign.fr/mnx/`，**部分核实**）或备选 RGE ALTI 1 m 放入 `assets/incoming/maps/carentan/dem/`；执行者写 GDAL 脚本把它变成游戏高度图。M0-009 已有 `tools/map-pipeline` 第一版（程序化分形高度图 + GDAL 路径、导出 `heightmap_1m.r16`、`meta.json`、64 m 区块切片，同输入输出哈希一致）。
MAPS §1.2 规则：
- 重采样 0.5 m → 1 m 用 **average（面积平均）**（`gdalwarp -r average`）；远景环 8 m 同样 average；
- 高程基准统一（IGN69 或椭球高均可，全图一致）；游戏内高度 = 高程 − 基准偏移，使**最低水面 ≈ 0 m**；`meta.json` 记录偏移；
- 16-bit 量化：`h_u16 = round((h − hmin)/(hmax − hmin)×65535)`；`hmin/hmax` 写入 meta（示例 −1.2 / 45.0 m，量化步长 < 1 mm）；
- 尺寸 2049×2049 样本（2048 个 1 m 单元）；可玩区外 256 m 缓冲区也要生成；远景环 1025×1025（8 m，覆盖 8192 m，中心 ± 4096）；远景环中心洞由可玩区覆盖；
- CRS：源 Lambert-93（EPSG:2154）→ 工作 ETRS89/UTM 30N（EPSG:25830，与 EPSG:32630 在本精度内等同）；本地系：西南角原点、x 东、y 北。
**M1 切片（MAPS §3.8）**：设计框架矩形 **x 380–1560、y 520–1700（≈ 1180×1180 m）** 用**完整 1 m 精度**；矩形外使用 8 m 远景环数据（上采样/双线性）；边界处加**不可通行墙**（视觉为矮灌木与雾）。区外数据与墙的游戏性语义见 `zones.json`（M1-034）的 `playable_polygon` 与 `out_of_bounds_buffer_m`（默认 64 m）。
若 DEM 缺失：任务标 `blocked`（所缺：DEM 栅格；解除：人类），**不得用程序化高度图假装完成**；M0 的程序化高度图仅作联调兜底并明示 `meta.height.source="procedural"`（MILESTONES §2.8 风险缓解）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/map_pipeline/dem.py` —— 读取 GeoTIFF 集合、拼接（`gdalbuildvrt`）、裁剪到"可玩区 + 256 m 缓冲"、重采样、写 `.r16`
- `tools/map-pipeline/map_pipeline/far_ring.py` —— 8 m 远景环 1025² 生成（含 GLO-30 兜底拼接，如 LiDAR 不覆盖）
- `tools/map-pipeline/map_pipeline/slice_m1.py` —— 切片逻辑：矩形内 1 m、外部 8 m 上采样混合（宽 32 m 羽化带）、写 `slice_mask.png`
- `tools/map-pipeline/map_pipeline/qa.py` —— 锚点高程核对：与独立来源（RGE ALTI 1 m vs LiDAR MNT）≥ 10 个点比较，输出 `reports/m1-dem-check.csv`
- `data/maps/carentan/heightmap_1m.r16`（2049×2049，小端 uint16；**大文件，按仓库约定入 git 或 Git LFS——先在 PLAN_ISSUES 登记决策**，8.4 MB）、`heightmap_far_8m.r16`（2.1 MB）
- `data/maps/carentan/meta.json`（更新 `height{res_m,samples,hmin_m,hmax_m,datum,offset_m,source}`、`far_ring`、`slice_m1{rect_design,rect_local,feather_m}`）
- `tools/map-pipeline/tests/test_dem.py`（用合成 GeoTIFF 夹具，不依赖真数据）
### 要实现的行为
1. CLI：`map-pipeline dem --incoming assets/incoming/maps/carentan/dem --out data/maps/carentan`；输入哈希清单写入 `meta.json.inputs[]`（文件名、sha256），保证可重现。
2. 拼接/裁剪/投影：目标范围由 `meta.json.crs.origin_e/n` + 2048 m（若 M1-030 的相似变换 `applied=true`，则按变换后范围）；坐标偏差校验：输出角点的 UTM 与 `meta` 一致（≤ 0.5 m）。
3. 重采样：`gdalwarp -r average`；无数据值（水域/空洞）用邻域填充并计数 `nodata_filled_cells`；超过 1% 则报错。
4. 基准偏移：计算最低水面高度，写入 `offset_m`；`hmin/hmax` 取实际范围向外圆整到 0.1 m。
5. 量化与写出：`.r16` 小端；同时写 `heightmap_1m.preview.png`（8 位灰度，仅调试，.gitignore）。
6. 切片：矩形内严格保持 1 m 数据；矩形外用远景环（8 m，双线性上采样）；32 m 羽化带线性混合避免台阶；输出 `slice_mask`（1=全精度）。
7. 不可通行墙数据：在 `meta.json.slice_m1` 写 `blocker_polygon`（矩形外轮廓外扩 0 m、内缩 4 m 的墙带）供 M1-034/sim 使用；视觉矮灌木/雾由 M1-050/M1-051 渲染。
8. 高程核对：选 ≥ 10 个点（道路交叉口、桥面、教堂前广场等；与 `anchors` 无关亦可）比较两种 DEM 或已知基准点，记录误差。
### 接口与数据
`meta.json.height = {"res_m":1,"samples":2049,"hmin_m":-1.2,"hmax_m":45.0,"datum":"IGN69","offset_m":…,"source":"ign_lidar_hd","inputs":[{"file":"…","sha256":"…"}]}`；`.r16`：行主序，自南向北或自北向南须在 `meta.row_order` 声明（默认 `south_to_north`，与本地系 y 向北一致）。

## 不做什么
- 1944 修正（洪泛、去堤坝、树篱）：M1-033；
- 区块切片、splat、碰撞、植被：M1-035；
- OSM 矢量：M1-032；
- 完整 2048 m 全精度（M2）；Omaha/Foy（M4）；
- 许可证核实：M1-030（本任务读取其 `sources` 状态，PENDING 时拒绝使用该输入并报错）。

## 验收标准
1. **锚点高程核对 ≥ 10 点误差 ≤ 1 m**：`reports/m1-dem-check.csv` 含 ≥ 10 行，`max|Δh| ≤ 1.0 m`，且 `median|Δh| ≤ 0.5 m`（RGE ALTI 1 m 垂直精度约 0.5–0.7 m 为参考，**未核实**）。
2. 同输入输出哈希一致：连续运行两次，`heightmap_1m.r16` 与 `heightmap_far_8m.r16` 的 sha256 相同（CI 作业对比）。
3. 尺寸与格式：`heightmap_1m.r16` 文件字节数 = 2049×2049×2 = 8,396,802；`heightmap_far_8m.r16` = 1025×1025×2 = 2,101,250（脚本断言）。
4. 量化往返误差：随机 1 万点 `|h − dequant(quant(h))| ≤ (hmax−hmin)/65535/2 + 1e-6`（< 1 mm，单测）。
5. 切片：矩形内（x 380–1560, y 520–1700）任意点与 1 m 重采样结果一致（误差 0），矩形外 ≥ 32 m 处与 8 m 上采样一致（单测，合成夹具）；羽化带单调。
6. 无数据填充率 ≤ 1%；投影角点误差 ≤ 0.5 m（单测）。
7. **DEM 缺失时**任务状态为 `blocked`，STATUS 登记"所缺：IGN LiDAR HD MNT 栅格/RGE ALTI；解除：人类放入 incoming"；脚本在缺失时退出码 3 并输出清晰提示（单测）。
8. 全流程运行时间 ≤ 10 min（`time`，记录机器；含 ≥ 4 个 1 km² 的 0.5 m 瓦片）。

## 测试要求
- 单元测试：合成 GeoTIFF 夹具（Lambert-93、0.5 m、含 NoData）→ 全流程；量化；羽化；偏移计算。
- 集成：对真实 DEM（若到位）运行并产出核对报告。
- 性能基准：验收 8。
- 回归：`meta.json` 输入哈希与输出哈希写入 STATUS。

## 性能预算
- 构建期工具；产物体积服务 ARCHITECTURE §9.1（高度图 2049² ≈ 8.4 MB；远景环 ≈ 2.1 MB）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **人工项**：DEM 下载（NEEDS_HUMAN 无独立 H- 条目；与 H-052 数据源许可证核实同批，缺失时追加条目）；LiDAR HD 在 Calvados/Manche 的覆盖年份须核实（MAPS §2.1 指出）。
- 大文件入库方式（git / LFS / 仅存哈希）：ARCHITECTURE 未规定，写 PLAN_ISSUES；`heightmap_1m.r16` 8.4 MB 不必 LFS，但 M2/M4 多图时需决策。
- "1944 年地貌"差异（现代 DEM 反映 2020s 地面，MAPS §1.5）由 M1-033 处理，本任务不得"顺手"修改地形。
- 地图坐标锚定若发生相似变换（M1-030），本任务的裁剪范围随之变化；两任务联调时留意 `similarity.applied`。
