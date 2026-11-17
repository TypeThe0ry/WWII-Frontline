# MAPS.md — 地图制作、史料与原创关卡设计

> 状态：规划文档（无实现代码）。核实日期：**2026-11-17**。
> 适用里程碑：M1（Carentan 1/3）、M2（Carentan 完整）、M4（Omaha、Foy）。
> 引用方式：任务文件请用 "MAPS.md §3.6" 这种形式；章节编号稳定，勿重排。

## 0. 阅读说明与约定

### 0.1 知识产权边界（硬约束）

1. 参考作品（HLL 等）只能黑盒观测：公开视频、自录视频、官方开发日志、玩家 wiki 上公开描述的规则。**禁止**反编译、拆包、内存读取、协议解析、素材提取。
2. 本文件的**地理与地标**按史料还原；**关卡设计**（据点位置、战线划分、掩体与重生点布局、据点名称）全部原创，只依据真实地理与战术合理性推导。设计者**不得**对照任何商业游戏同名地图的布局来"调整"本文件的布局；若有人发现某据点与参考作品布局雷同，应改动本文件而非辩护。
3. 据点命名规则：真实地名/地标一律标 **[R]**（史实名称，可直接用）；原创描述性名称标 **[G]**（游戏内名称，必须保持原创，不得使用参考作品的据点名）。
4. 本文件不引用任何参考作品或 wiki 的文字、数值表格排版。

### 0.2 核实状态图例

| 标记 | 含义 |
|---|---|
| 已核实 | 2026-11-17 通过网页搜索结果或直接抓取，确认该 URL/页面存在且内容与描述相符 |
| 部分核实 | 页面存在，但直接抓取被拒绝（403/授权页）或只看到摘要；许可证需在使用前到页面原文再确认 |
| 间接核实 | 通过第三方页面/搜索摘要得知，未见官方原页面 |
| 未核实 | 没有查到；使用前必须人工核实。**不得**据此做商用决策 |

> 诚实说明：本次核实主要借助搜索摘要与有限的页面抓取。IGN 的 geoservices/cartes.gouv.fr 页面、McMaster 图书馆页面、NCAP 首页在抓取时返回重定向空壳/403/授权页，故相关条目标"部分核实"。**任何许可证结论在进入 `LICENSES.md` 前，都需要人工到原页面截图留档。**

### 0.3 设计坐标系（"设计框架" Design Frame，贯穿全文）

- 每张图的可玩区是 UTM 投影下**轴对齐**的 2048 m × 2048 m 正方形，**不旋转**。
- 本地坐标 `(x, y)`：原点在西南角（SW），x 向东，y 向北，单位 m，范围 0–2048。
- 宏网格（仅用于 UI 战术地图）：128 m 一格，列 A–P（自西向东），行 1–16（自**北**向南）。为避免歧义，**本文件所有位置一律给 `(x, y)` 米坐标**。
- **坐标可信度分级**：每个坐标后用 `±n m` 表示本文件作者对该位置的不确定度估计。凡 `±150 m` 以上者在制图第一步（§7 任务 T-GEO）用 GSGS 扫描、IGN/Wallonie 现行数据与 OSM 重新锚定；**锚定后设计以"拓扑与相对关系"为准**（见 §0.4）。

### 0.4 锚点—相似变换规则（设计与真实位置的对账）

设计位置先在设计框架中给出，再通过"锚点"与真实位置对齐。流程：

1. 每张图列出 2–4 个**锚点**（容易在 GSGS/现行数据上精确定位的地物，如教堂、十字路口）。
2. 锚点的真实 UTM 坐标确定后，若与本文件给出的设计框架坐标偏差 ≤ 150 m，直接平移，设计位置整体平移同样量；偏差更大时，拟合一个**平移 + 小角度旋转（≤ 10°）** 的相似变换，应用于所有"设计"位置（据点、出生点）；"真实地标"位置则不变换，直接用真实坐标。
3. 变换后，设计据点若落入水体、建筑内或坡度 > 35° 的地方，向最近的可站立开阔地吸附（≤ 60 m），并在 `zones.json` 的 `notes` 里记录。
4. 任何调整都不得改变**拓扑**：战线顺序、HQ 相对位置、主要轴线、水体隔断关系。

---

## 1. 地图制作方法论

### 1.1 总流水线（`/tools/map-pipeline`，Python + GDAL）

```
 [1 数据获取]  DEM / 点云 / 航拍 / 历史地图 / OSM
      │
 [2 统一坐标]  全部重投影到每张图的 UTM 带（见 §1.3），裁剪到"可玩区 + 远景环"
      │
 [3 高度图]    LiDAR 地面点 → 1 m DTM（可玩区 2049×2049，16-bit）
      │         + 远景环 8 m（1025×1025 覆盖 8 km，16-bit）
 [4 1944 修正] 去现代构筑物 / 恢复树篱 / 洪泛区 / 去采石与现代堤坝（人工）
      │
 [5 矢量化]    道路、水系、树篱、森林、田块（OSM + 历史地图手动描摹）
      │
 [6 建筑足迹]  1944 年存在的建筑（历史航拍 + 现代足迹筛选 + 人工补）
      │
 [7 地标]      逐个建模/摆放（素材采购或现成资产），写 landmarks.json
      │
 [8 关卡标注]  zones/spawns/garrison/artillery/OP + 导航网格烘焙
      │
 [9 打包]      64 m 区块切片、splat 权重图、碰撞、navmesh → /data/maps/{map_id}/
```

每一步输出可重现（脚本 + 输入清单哈希），输入原始数据放 `/assets/incoming/maps/{map_id}/`（`.gitignore`），处理产物清单进 git。

### 1.2 DEM → 高度图

| 步骤 | 做法 | 说明 |
|---|---|---|
| 取数 | 法国：IGN LiDAR HD 的 MNT（0.5 m 栅格，地面点插值）；备选 RGE ALTI 1 m；比利时：瓦隆 2021–2022 LiDAR 的 MNT 0.5 m | 见 §2.1 |
| 重采样 | 0.5 m → 1 m，使用 **average（面积平均）**，避免混叠；远景环用 8 m average | GDAL `gdalwarp -r average` |
| 高程基准 | 统一到 EGM/IGN69 正高或椭球高均可，只要整张图一致；游戏内高度 = 高程 − 基准偏移，使最低水面≈ 0 m（便于水体） | 在 `meta.json` 记录偏移 |
| 量化 | 16-bit：`h_u16 = round((h - hmin) / (hmax - hmin) * 65535)`；`hmin/hmax` 写入 `meta.json`。Carentan/Omaha 起伏小（<60 m），实际垂直量化步长 < 1 mm，无问题 | |
| 尺寸 | 2049×2049 样本 = 2048 个 1 m 单元（符合总纲 §3 地形行） | |
| 边缘 | 可玩区边缘外 256 m 缓冲区的高度也要生成，供远景环无缝衔接；远景环中心洞用可玩区覆盖 | |

### 1.3 坐标系与投影

| 项 | Carentan | Omaha | Foy |
|---|---|---|---|
| 源数据 CRS | RGF93 / Lambert-93 (EPSG:2154) | 同左 | Belgian Lambert 2008 (EPSG:3812) |
| 游戏工作 CRS | ETRS89 / UTM 30N (EPSG:25830)；与 WGS84/UTM30N (EPSG:32630) 在本项目精度内等同 | 同左 | ETRS89 / UTM 31N (EPSG:25831)（≈EPSG:32631） |
| UTM 带 | 30N（中央经线 3°W） | 30N | 31N（中央经线 3°E；Foy 约 5.75°E，距带中央 2.75°，比例畸变 ≈ 1.0003–1.0004，可忽略，但**须**在 `meta.json` 注明） |
| 游戏本地系 | 西南角为原点，右手系，y 向北 | 同左 | 同左 |

> UTM 带与四角由 `pyproj` 计算（2026-11-17 实际运行）；中心取 64 m 倍数以便与 64 m 区块对齐。**下列 WGS84 经纬度四角是"设计框架"的暂定值，需在 T-GEO 锚定后复算。**

**每张图的可玩区四角（UTM 与 WGS84，暂定）**

| 地图 | 角 | UTM E | UTM N | 纬度, 经度 |
|---|---|---|---|---|
| Carentan（EPSG:32630） | SW | 626432 | 5462016 | 49.29780, -1.26100 |
| | SE | 628480 | 5462016 | 49.29737, -1.23284 |
| | NE | 628480 | 5464064 | 49.31578, -1.23219 |
| | NW | 626432 | 5464064 | 49.31621, -1.26035 |
| | 中心 | 627456 | 5463040 | 49.30679, -1.24659 |
| Omaha（EPSG:32630） | SW | 652160 | 5469952 | 49.36327, -0.90432 |
| | SE | 654208 | 5469952 | 49.36275, -0.87614 |
| | NE | 654208 | 5472000 | 49.38116, -0.87534 |
| | NW | 652160 | 5472000 | 49.38168, -0.90354 |
| | 中心 | 653184 | 5470976 | 49.37222, -0.88984 |
| Foy（EPSG:32631） | SW | 695488 | 5546112 | 50.03517, 5.72990 |
| | SE | 697536 | 5546112 | 50.03450, 5.75846 |
| | NE | 697536 | 5548160 | 50.05290, 5.75952 |
| | NW | 695488 | 5548160 | 50.05357, 5.73094 |
| | 中心 | 696512 | 5547136 | 50.04404, 5.74470 |

远景环：以可玩区中心为中心的 8192 m × 8192 m（8 m × 1024 样本）正方形，即中心 ± 4096 m；对 Omaha，北侧大部分是海面，远景环用平面水 + 远处不绘制对岸。

### 1.4 比例与"是否压缩真实距离"的决定

**决定：不压缩（1:1）。** 理由：

1. 2048 m 可玩区本身已经是"选段"而非整场战役；压缩会破坏射程感（步枪有效 ~300–400 m、机枪/坦克 >500 m 的视距关系，见总纲 50v50 的节奏）。
2. 真实地标（教堂、桥、堤道）之间的相对距离本身就是史料价值；压缩后需要大量人工"挪动地标"，而 1:1 只需**选框**。
3. 代价：完整战役范围（如 Carentan 的 Saint-Côme-du-Mont 至市区的 ~3 km 堤道；Omaha 全 8 km 海滩）无法全部纳入——靠"选框 + 远景环 + 不可进入的边界缓冲"解决，**不**靠压缩。

**允许的两种"局部形变"（均须在 `meta.json` 登记）**：

- 垂直夸张：不做（1:1）。
- 局部移位：为保证某据点的可玩性，允许将**设计据点**（非真实地标）相对真实位置移动 ≤ 60 m；**真实地标**不得移动，除非整体相似变换（§0.4）。

### 1.5 1944 年地貌修正（人工为主）

| 修正项 | Carentan | Omaha | Foy |
|---|---|---|---|
| 现代建筑 | 删除 1944 年后建筑（新城区、超市等），保留教堂等保存下来的 | 删除海滨度假屋、现代停车场、纪念碑场地；Vierville draw 现代道路护坡需恢复 | 删除 1945 之后的农场扩建、现代住宅 |
| 道路 | 现代 N13 改线/四车道段还原为 1944 堤道；运河与港口范围还原 | 沿海滩的现代车道、Vierville draw 公路拓宽还原 | N30 改线检查；林间现代路径去除 |
| 树篱 bocage | **大量恢复**：田块边界树篱+土堤（见 §1.6） | 主要在高原内陆一带；海滩无 | 无 bocage，有林带与田间矮篱 |
| 水体 | **1944-06 洪泛**：德军放水淹没 Douve 河漫滩（沼泽，车辆不可通行） | 潮间带：以 1944-06-06 潮位（低潮 ≈ 06:00 前后，涨潮中）确定沙滩范围，**潮汐数据未核实，需查** | 小溪、冻结的池塘；雪覆盖 |
| 植被 | 沼泽草地、苹果园、树篱橡树 | 灌木、草坡、悬崖植被 | 冷杉人工林（Bois Jacques）、牧场 |
| 地形扰动 | 无 | 恢复为战前海滩与 shingle 堤形态；去除战后侵蚀/人工海堤维护 | 无（但战时炮坑可作贴花） |

> 1944 年修正的难点：现代 LiDAR 反映 2020s 的地面。堤道、运河、海堤可能已被改造；**用 GSGS 地形图 + 战时/战后早期航拍对照后，在 DEM 上手工绘制修正多边形**（`terrain_edits.geojson`：多边形 + 目标高程/抬升量 + 过渡宽度），由脚本烘焙进高度图。

### 1.6 矢量与建筑

- **道路**：OSM 作初稿 → 对照 GSGS/航拍删除 1944 年不存在的 → 补全 1944 年存在的小路。属性：`class`（paved/dirt/path/rail/causeway）、`width_m`、`surface`、`bridge`（bool）、`elev_mode`（terrain|embankment|bridge）。
- **水系**：河、运河、沼泽区、海岸线（含高/低潮线）。属性：`kind`、`depth_m`、`walkable_depth`（决定可涉水）。
- **树篱**：折线 + 属性 `height_m`（1.5–2.5）、`mound_width_m`（土堤宽）、`density`、`has_gap`（缺口）。Carentan 树篱是掩体与视线分割的主要工具（§3.6）。
- **建筑**：足迹多边形 + `levels`、`roof`、`style`（见 §3.5/§5.5）、`enterable`（bool）、`ruin_stage`。来源：现行 OSM/IGN BD TOPO 足迹 → 与 1944 影像对照 → 人工补缺。**可进入建筑**占比控制（性能与关卡设计），其余为"实心"外壳。
- **地标**：单独 `landmarks.json`，每项记录真实坐标、还原等级（§3.4 表）。

### 1.7 还原精度分级（全文通用）

| 等级 | 含义 |
|---|---|
| **A（精确）** | 位置±5 m、轮廓与体量按史料/现状测量，外观按照片 |
| **B（忠实）** | 位置±15 m、轮廓大致、外观按同类建筑风格 |
| **C（示意）** | 位置±40 m，类型与功能正确，外观通用素材 |

---

## 2. 通用资料渠道清单与许可证

> 本章是**全局**渠道；每张图的具体清单见 §3.3/§4.3/§5.3。

### 2.1 高程

| 资料 | 内容 | URL | 许可证 | 核实状态（2026-11-17） |
|---|---|---|---|---|
| IGN LiDAR HD | 法国全境 LiDAR HD：点云（≥10 脉冲/m²）、MNT/MNS/MNH 栅格（0.5 m；也有其它格网），按 50 km 块组织，约 24 MB/块（MNS 0.5 m GeoTIFF 示例） | 门户：https://cartes.gouv.fr/rechercher-une-donnee/dataset/IGNF_NUAGES-DE-POINTS-LIDAR-HD（`geoservices.ign.fr/lidarhd` 301 跳转至此）；下载：https://diffusion-lidarhd.ign.fr/mnx/；data.gouv 登记：https://www.data.gouv.fr/datasets/mnt-lidar-hd | **Licence Ouverte / Etalab 2.0**（可商用，须署名）；Etalab 2.0 文本：https://www.etalab.gouv.fr/wp-content/uploads/2017/04/ETALAB-Licence-Ouverte-v2.0.pdf | 部分核实：跳转 301 已确认；门户页抓取为空壳；许可证与参数来自搜索摘要。**使用前确认 Calvados/Manche 当前覆盖年份**（见 data.gouv "Carte du LiDAR HD de l'IGN par commune"：https://www.data.gouv.fr/datasets/carte-du-lidar-hd-de-lign-par-commune-date-du-vol-et-couverture-en-france ，间接核实） |
| IGN RGE ALTI | 法国 DTM，1 m 与 5 m 版本 | 门户：https://cartes.gouv.fr/rechercher-une-donnee/dataset/IGNF_RGE-ALTI ；data.gouv：https://www.data.gouv.fr/datasets/rge-alti-1/informations | Etalab 2.0（Earth Engine 目录与 data.gouv 摘要均标注） | 部分核实：门户为空壳；许可证由第三方目录与搜索摘要得出。1 m 版垂直精度约 0.5–0.7 m（第三方目录） |
| 瓦隆 LiDAR / DTM（比利时） | 2021–2022 LiDAR 获取的 0.5 m MNT；ArcGIS REST 服务 | 服务：https://geoservices.wallonie.be/arcgis/rest/services/RELIEF/WALLONIE_MNT_2021_2022/MapServer ；教程 PDF：https://geoportail.wallonie.be/files/PDF/LiDAR24/Tuto_LiDAR%202.pdf ；联系：lidar@spw.wallonie.be（来自教程摘要） | **未能在官方页面直接核实**。第三方 wiki 称"开放，须署明 SPW 为来源"。**须到 Géoportail de la Wallonie 数据集元数据页确认**（`spw.wallonie.be/lidar` 抓取 404） | 部分核实（服务与分辨率）；许可证 **未核实（官方）** |
| Copernicus DEM GLO-30 | 全球 30 m DSM（含建筑/植被）；用于**远景环外圈**或兜底 | 许可证 PDF：https://documentation.dataspace.copernicus.eu/APIs/SentinelHub/Data/DEM/resources/license/License-COPDEM-30.pdf ；公告：https://sentinels.copernicus.eu/-/copernicus-dem-30-metre-dataset-now-freely-available | "Full, Free & Open"；署名："Copernicus DEM GLO-30 (Copernicus WorldDEM™-30) © DLR e.V."（据搜索摘要） | 间接核实；国家例外清单来源互相矛盾，法国/比利时不受影响，仍须读许可证原文 |
| SRTM | 全球 ~30 m（1 弧秒）/90 m | 官方分发渠道（USGS EarthExplorer 等） | 美国政府作品，通常公共领域 | **未核实**（本次未抓取）；仅作兜底，**不推荐**（噪声、含植被） |
| 总纲对应 | 可玩区：LiDAR 1 m；远景环 8 m：由同一 LiDAR 或 RGE ALTI 5 m 重采样；超出 LiDAR 覆盖的远景：GLO-30 | | | |

**取数结论**：Carentan 与 Omaha 用 **IGN LiDAR HD**（RGE ALTI 1 m 作覆盖缺口备选）；Foy 用**瓦隆 LiDAR**。

### 2.2 历史航拍

| 资料 | 内容 | URL | 许可证/费用 | 核实状态 |
|---|---|---|---|---|
| IGN "Remonter le temps" | 在线浏览 IGN 老航拍与老地图（航拍多自 1950 年代起，**不含 1944 年**） | https://remonterletemps.ign.fr/ | 浏览免费；**单张影像的再利用许可证未核实**。部分衍生产品（BD ORTHO® Historique，如 Calvados 1972）以开放许可发布，见 https://www.data.gouv.fr/datasets/photographie-aerienne-bd-ortho-r-historique-1972-calvados/reuses_and_dataservices | 页面存在（标题已确认）；许可证 **未核实** |
| NCAP（National Collection of Aerial Photography） | 英国政府解密航拍；与美国 NARA 合作复制二战影像 | https://www.ncap.org/ ；许可：https://www.ncap.org/products-services/licensing ；研究副本：https://ncap.org/products-services/research-copies | 个人研究免费；**其它用途按用途/地点/时长报价**，费用在报价单中给出 | 间接核实（首页抓取 403；搜索摘要）。注意：NCAP 自述 **英格兰/威尔士/北爱二战 RAF 影像由 English Heritage Archive 保管**，法国/比利时覆盖需向 NCAP 查询 |
| US NARA | 美军航拍与照片（RG 373 等 **未核实** 具体档案号）；NARA 静态影像 RG 319 航拍说明页 | https://www.archives.gov/research/cartographic/aerial-photography/still-pictures-rg319-aerial-photography ；二战影像总页：https://www.archives.gov/research/ww2/photos/ | 军人执勤拍摄的照片视为公共领域，但**个别条目可能有限制**，必须逐条检查 Catalog 的 Use Restrictions；NARA 出版须知：https://www.archives.gov/research/publish-photos | 部分核实（页面存在；条目级许可需逐条查） |
| IWM（Imperial War Museums） | 英国官方二战照片 | https://www.iwm.org.uk/corporate/policies/non-commercial-licence ；商业：https://www.iwm.org.uk/commercial/licensing | 标准分辨率免费用于**私人/非商业**；**商业用途需付费授权** | 已核实（搜索摘要）。**本项目（A1 免费但接受捐赠）定性：捐赠是否构成"商业"未经法律确认——保守按"商业"处理，仅作内部参考，不嵌入游戏** |
| 战场影像 | Normandy War Guide 等站点收录 GSGS 与航拍 | https://www.normandywarguide.com/articles/gsgs-4250-map-series | 站点页脚 "© Normandy War Guide"，**未见对扫描件的使用条款** | 已核实（存在）；条款 **未核实**：仅当"参考"，需要时直接回到 NARA 等原始来源 |

**使用原则**：航拍/老照片**只作为描摹参考**（我们手绘矢量，不使用影像本身做贴图）。这样绕开影像授权问题，且符合 A2（不自制模型之外只用现成素材，但地形矢量由流水线产出）。

### 2.3 历史地形图

| 资料 | 内容 | URL | 许可证 | 核实状态 |
|---|---|---|---|---|
| GSGS 4250（1:50,000，法国，1942–44） | 英国陆军地理处战时法国图幅；Carentan 所在图幅编号 **未核实**（Normandy War Guide 的索引页未列 Carentan 图幅号） | 索引：https://www.normandywarguide.com/articles/gsgs-4250-map-series ；已见的单页示例：https://www.normandywarguide.com/archives/map-8g1 | 原图为英国王室版权（1940s，期限问题**未核实**）；扫描件托管方条款未明 | 已核实（索引存在；比例 1:50,000） |
| GSGS 4347（1:25,000 说法） | 任务书称 1:25,000；本次搜索**未能核实** GSGS 4347 的比例与覆盖 | — | — | **未核实** |
| GSGS 4336（1:100,000，比利时与法国东北） | UT Austin PCL 目录中可见 AMS 再版（含 GSGS 4336） | https://collections.lib.utexas.edu/ 下目录条目（搜索结果出现若干 `utlmaps:*` 条目，如 https://collections.lib.utexas.edu/catalog/utlmaps:70d14907-e811-43dd-840b-34c5975e1b3b） | 美军 AMS 再版；**权利状态以 UT 条目的 rights 字段为准，未核实** | 间接核实 |
| 美军 AMS 系列（例：France & Belgium 1:50,000 M703） | 美国陆军地图处 | https://collections.lib.utexas.edu/catalog/utlmaps:3b5c5cc0-6727-48d2-b327-f56cd8b814e0 | 同上 | 间接核实 |
| McMaster University 二战地形图 | 法国/比利时/荷兰二战地形图系列页 | https://library.mcmaster.ca/wwii-topographic-map-series-france-belgium-holland | **未核实**（页面抓取返回授权页；系搜索结果可见） | 部分核实；**搜索未能确认 McMaster 馆藏包含 GSGS 4250 具体图幅**，需人工查目录 |
| Library of Congress | 二战军事情报地图集合（finding aid 存在） | 搜索结果可见 finding aid 编号 `gm015009`（"WWII Military Intelligence Map Collection"）；官方 URL **未直接核实** | 美国政府作品多为公共领域，**逐件未核实** | 间接核实 |
| Scholars GeoPortal（加拿大） | 二战 1:25,000 图幅（Waterloo 数字化/地理配准），如敦刻尔克防御套印 | https://learn.scholarsportal.info/?p=4264 | **未核实** | 间接核实；是否含诺曼底图幅 **未核实** |

**使用原则**：历史图用于 **(a) 地理配准校验**、**(b) 1944 年道路/树篱/建筑布局描摹**；不作游戏纹理。

### 2.4 公共领域档案

| 资料 | URL | 许可/状态 | 核实 |
|---|---|---|---|
| US Army "Green Books"（CMH，*United States Army in World War II*）：Cross-Channel Attack（Harrison，1951 前后出版）；Omaha Beachhead（Taylor，1945，*American Forces in Action*） | HyperWar 转录：https://www.ibiblio.org/hyperwar/USA/USA-E-XChannel/USA-E-XChannel-preface.html ；Omaha：https://www.ibiblio.org/hyperwar/USA/USA-A-Omaha | 美国联邦雇员职务作品通常无版权；转录页**未含版权声明**，仅署转录者；**CMH 官网 URL 未核实**；后世重印版权**未核实**（我们只引用史实，不复制文字） | 已核实（ibiblio 页面抓取成功）；公共领域状态 **以通用法律常识为据，非法律意见** |
| NARA 照片 | https://www.archives.gov/research/ww2/photos/ | 大多数为公共领域；条目级核查 | 部分核实 |
| Signal Corps 影像 | 同上 | 军人拍摄的视为公共领域；RG 319 等系列含"已撤回/限制"项 | 间接核实 |

### 2.5 现代矢量：OpenStreetMap

| 项 | 内容 |
|---|---|
| URL | OSM 许可：https://wiki.openstreetmap.org/wiki/License/Use_Cases ；署名指南：https://osmfoundation.org/wiki/Attribution |
| 许可证 | **ODbL 1.0**，署名 "© OpenStreetMap contributors" |
| 影响 | (1) **Produced Work**（例如渲染出的游戏画面/地图）只需署名，不触发 share-alike；(2) **Derivative Database**（我们若把 OSM 数据**提取后合并进** `roads.geojson`/`buildings.geojson` 并**公开发布**）触发 share-alike：该衍生数据库须以 ODbL 公开；(3) 内部使用不触发。 |
| 本项目策略 | 把 OSM 视作**初稿**，经 1944 年人工描摹后，成品 `roads/hedgerows/buildings` 里 OSM 来源的占比大概率仍属"实质提取"。**建议：把 `/data/maps/{id}/` 中的矢量数据层声明为 ODbL（独立 NOTICE），代码与素材不受影响（Collective Database 与 Produced Work 分离）；游戏内署名"© OpenStreetMap contributors"。** 如不愿公开，则只用 OSM 作视觉参考，从历史图重新描摹，不入库。 |
| 核实 | 间接核实（OSM wiki 为社区解读，非法律意见）；**入库前需法律确认** |

### 2.6 渠道结论与风险

1. **高程**：法国与比利时均有开放 LiDAR，质量足够 1 m 高度图；Foy 的许可证是唯一需人工核实的点。
2. **航拍**：1944 年影像在 NARA/NCAP/IWM，免费使用范围有限；策略为"只参考、不入库"。
3. **历史图**：GSGS 4250 扫描可得，但许可证与版权期限不清；同样"只参考"。
4. **OSM**：ODbL 影响最大，须在 M1 前定策略（§2.5）。

---

## 3. Carentan（1944-06-10 ~ 06-13）

### 3.1 战役背景与时间点

- **目的**：美军需连接 Utah 与 Omaha 滩头并封住德军切断科唐坦半岛南侧的通路（据 Wikipedia "Battle of Carentan" 与 Green Books；见 3.3）。
- **时间线（据 Wikipedia，已核实；细节以 Cross-Channel Attack 为准）**：
  - 06-10：第 502 伞兵步兵团（PIR）3 营（Lt. Col. Robert G. Cole）沿 Saint-Côme-du-Mont—Carentan 堤道（N13，后称 "Purple Heart Lane"）推进，伤亡极大。
  - 06-11：Cole 在 Ingouf 农场一带指挥刺刀冲锋（后获荣誉勋章）。注：Wikipedia 本页文本未出现 "Ingouf"；"Ingouf" 来自 beachesofnormandy.com、historynet 等搜索摘要，**需 Green Books 原文确认**。
  - 06-12：德军缺弹药撤出，美军在市中心会合。
  - 06-13：第 17 党卫装甲掷弹兵师（Götz von Berlichingen）反攻（"Bloody Gulch"），被第 2 装甲师 CCA 击退。
- **美军**：101st Airborne：502nd PIR、506th PIR、327th GIR（501st PIR 预备）；2nd Armored Division CCA（06-13）。
- **德军**：6. Fallschirmjäger-Regiment（von der Heydte，2. Fallschirmjäger-Division 编成）两个营；Grenadier-Regiment 1058 残部；17. SS-Panzergrenadier-Division（06-13 反攻）。
- **水文**：德军淹没 Douve 河漫滩，形成车辆不可通行的沼泽，只剩堤道上四座桥可通行。

> 本游戏的 Carentan 采用 **06-11 ~ 06-12** 的时间设定（反攻前），以步兵 + 少量轻装甲/反坦克炮为主。2nd Armored 的坦克作为 06-13 变体（M3 之后再考虑）。

### 3.2 可玩区选择

| 项 | 值 |
|---|---|
| 中心 | 49.30679°N, 1.24659°W（UTM30N E 627456, N 5463040，暂定） |
| 边界框 | 见 §1.3 表（SW 626432/5462016，NE 628480/5464064） |
| 框选理由 | (1) 含老城核心（Notre-Dame 教堂与 Place de la République）；(2) 含 N13 堤道**靠近城镇的一段**（Ingouf 一带及最南一、二座桥）；(3) 西南含树篱田地（bocage）；(4) 东/北含洪泛沼泽。**完整的 Saint-Côme-du-Mont—Carentan 堤道约 3 km，不可能全含；我们不压缩距离（§1.4），堤道北段在可玩区外，由远景环呈现。** |
| 教堂锚点 | Notre-Dame（1 Place Guillaume de Cerisay）约 49.30559°N, 1.24316°W（Cirkwi 目录；Mérimée 记录 PA00110352，已核实存在）→ UTM ≈ E 627709, N 5462913 → **设计框架 (1277, 897)** |
| 远景环范围 | 8 km × 8 km（中心 ± 4 km）：北至科唐坦半岛湿地与 Saint-Côme-du-Mont 方向；东至 Brévands/ Utah 方向沼泽；南至 Saint-Jean-de-Daye/Périers 方向田野；西至 Sainte-Mère-Église 方向 |

### 3.3 史料来源清单

| 资料 | 类型 | URL | 许可证 | 用途 | 核实状态 |
|---|---|---|---|---|---|
| Battle of Carentan（Wikipedia） | 二手综述 | https://en.wikipedia.org/wiki/Battle_of_Carentan | CC BY-SA（Wikipedia 通用；仅取史实不取文字） | 时间线、参战单位 | 已核实（抓取） |
| Cross-Channel Attack（Harrison, CMH Green Book） | 官方战史 | https://www.ibiblio.org/hyperwar/USA/USA-E-XChannel/USA-E-XChannel-preface.html | 美国政府作品（常识性公共领域，非法律意见） | Carentan 章节：部署、桥与地形描述 | 已核实（前言页）；相关章节页 URL **未逐一核实** |
| "Utah Beach to Cherbourg"（AFIA） | 官方战史 | HyperWar 对应路径 **未核实** | 同上 | 502nd/506th 细节 | **未核实**（本次未检索） |
| Purple Heart Lane / Cole's Charge（TracesOfWar、historynet、beachesofnormandy、Normandy War Guide 纪念碑页） | 二手/旅游史料 | https://www.tracesofwar.com/sights/157154/Purple-Heart-Lane.htm ；https://historynet.com/at-the-bayonets-point/ ；https://www.normandywarguide.com/place/coles-bayonet-charge-memorial-carentan | 各站版权所有；仅取史实 | N13 四座石桥（跨运河、Douve、Madeleine 河）；Ingouf 农场位置 | 间接核实（搜索摘要，未逐页抓取） |
| 教堂 Notre-Dame（Mérimée PA00110352） | 文化遗产登记 | https://pop.culture.gouv.fr/notice/merimee/PA00110352 ；https://monumentum.fr/eglise-notre-dame-pa00110352.html | POP 开放数据（具体许可**未核实**） | 教堂年代、结构 | 间接核实（搜索结果出现） |
| Cirkwi 教堂条目 | 目录 | https://www.cirkwi.com/fr/point-interet/429681-l-eglise-notre-dame-de-carentan | 版权所有；仅取坐标作初值 | 教堂近似坐标 | 间接核实 |
| IGN LiDAR HD / RGE ALTI | 地形 | 见 §2.1 | Etalab 2.0 | 高度图 | 部分核实 |
| GSGS 4250 Carentan 图幅 | 历史图 | 见 §2.3 | 未核实 | 道路/树篱/堤道描摹 | 图幅号 **未核实** |
| NARA 1944 航拍（RG 373 等） | 航拍 | 见 §2.2 | 条目级 | 洪泛范围、树篱、建筑 | **未核实**（具体档案号需查） |
| OSM | 现代矢量 | 见 §2.5 | ODbL | 初稿 | 间接核实 |

### 3.4 真实地标清单

> 坐标以设计框架 `(x, y)`（m，±误差）表示；教堂来自目录坐标（±40 m），其余为作者依据史料描述的**暂定**位置，**必须在 T-GEO 中核对**。

| 地标 | 位置（设计框架） | 1944 状态 | 现状 | 还原精度 | 参考资料 |
|---|---|---|---|---|---|
| Église Notre-Dame（教堂，高塔） | (1277, 897) ±40 | 战时有损，塔为视觉与炮兵观察地标 | 保存，PA00110352 | **A** | Mérimée；Cirkwi；航拍 |
| Place de la République（拱廊广场） | 教堂东北 100–200 m：约 (1380, 990) ±120（**未核实**） | 拱廊建筑、市政中心 | 保存 | **A** | 现行图、航拍；本次**未找到坐标源** |
| 火车站 | 暂定 (1500, 520) ±250（**位置待核实**） | 巴黎—瑟堡线；铁路路堤是视线/掩体线 | 站址与线路保留 | **B** | OSM `railway=station` 复核 |
| Douve 河与运河（Carentan 港运河） | 河道在设计框架的东/北侧，方向 NE（**待核实**） | 1944-06 洪泛，沼泽 | 已排水为牧场 | **A（形状）** | LiDAR 低地 + 洪泛多边形（§1.5） |
| N13 堤道 / Purple Heart Lane（含四座桥） | 自西北角 (0, 1980) 向东南入城，进入城区约 (900, 1150) ±150；其中位于框内的桥约 2 座 | 堤道高出沼泽 ~1–2 m，两侧沼泽；四座石桥 | 现为 N13 | **A** | Wikipedia；TracesOfWar；GSGS |
| Ingouf 农场（Cole's Charge） | 约 (900, 1450) ±250（**待核实**） | 石砌农场建筑群，周围树篱 | 部分存在，纪念碑在附近 | **B** | historynet；Normandy War Guide 纪念碑页；Green Books |
| 洪泛沼泽 | 北与东侧大面积；设计框架 y>1500 的堤道两侧、x>1650 的东缘 | 浅水 + 草沼，**车辆不可通行**，步兵涉水慢 | 无 | **A（范围）** | 航拍（NARA）；Wikipedia |
| Augustine 修道院（Couvent des Augustines） | **位置未核实** | 老城大型街区（候选 Garrison/强点） | 存在（目录条目） | **C** | https://www.cirkwi.com/fr/point-interet/1412177-le-couvent-des-augustines |
| 南郊树篱田地（bocage） | 设计框架 x<1100, y<900 | 小田块 + 土堤树篱 | 大量被拆并 | **B（布局）** | GSGS；LiDAR 微地形 |

### 3.5 环境设定

| 项 | 设定 |
|---|---|
| 季节/天气 | 1944-06，诺曼底初夏；多云，间歇性阵雨，地面湿润 |
| 时段 | 供选：清晨 06:00（薄雾，沼泽雾气）、午后 14:00、傍晚 19:00（长影）。M1 默认 **清晨** |
| 植被 | 草沼（芦苇、莎草）、苹果园、橡树/榆树树篱、牧草、小块耕地；少量杨树 |
| 土壤/材质调色板 | 湿润泥土（暗褐）、碎石路、石板路、草地（高湿度）、沼泽水（浑浊棕绿）、石灰岩墙（米黄）、石板瓦（深灰）、砖（红褐）、苹果园落叶 |
| 建筑风格 | **诺曼底石屋**：石灰岩/砂岩墙、陡坡板岩屋顶、石烟囱、木百叶窗；市中心 2–3 层，广场有拱廊；农场为 U 形院落 + 石谷仓 |
| 素材采购对照（A2 只买/用现成） | "Normandy stone farmhouse"、"Norman cottage"、"French village church (Gothic/Romanesque)"、"arcade square"、"stone bridge"、"hedgerow bocage kit"、"apple tree"、"marsh reeds"、"railway station (period)" |

### 3.6 原创关卡设计

**设计原则（原创推导，见本节末尾"原创性声明"）**：
- 主轴线是**真实的**——美军从西北堤道/侧翼进入，德军守城并依托南郊与东侧；因此战线沿 **西北 → 东南** 对角线，沿线依次穿过：堤道出口、树篱西郊、老城、铁路与南缘、东南草地。
- 水体与沼泽（东、北）作为天然边界，限制侧翼；西南树篱区提供**步兵专属**的迂回走廊；堤道与城区街道为**载具通行**路线。

**HQ 与出生点**

| 阵营 | HQ 位置（设计框架） | 说明 |
|---|---|---|
| US | (250, 1750) ±60，西北角干地（树篱围合农场） | 位于堤道西侧干地，真实地理上是堤道西岸干燥高地 |
| GER | (1850, 250) ±60，东南角（铁路与东侧草地之间的干地） | 靠近南缘与东侧运河、沼泽的交汇处 |

**Frontline 模式：5 条战线 × 2–3 候选据点**（"战线"=从 US 到 GER 的第 N 排；每条战线运行时选定 1 个据点为"活动据点"，其余作为同排候选，由赛季轮换或地图变体选择）。强点半径（Strongpoint radius）为【待调参】初值。

| 战线 | 据点 | 位置 (x, y) ±误差 | 战术特征 | 强点半径 |
|---|---|---|---|---|
| **L1 堤道头** | L1a Ingouf Farm [R] | (900, 1450) ±250 | 石质院落、周围树篱、正对堤道出口；视线沿堤道向西北 | 45 m【待调参】 |
| | L1b Southern Bridge [R/G] | (560, 1700) ±150 | 堤道石桥，窄口，掩体仅桥栏与路堤；步兵与载具只此一路 | 35 m |
| | L1c Orchard Rise [G] | (300, 1300) ±80 | 西侧干地果园，树篱圈，可迂回入城 | 50 m |
| **L2 西郊** | L2a Market-Garden Fields [G] | (700, 800) ±80 | 菜园与树篱网格，近距离战斗 | 55 m |
| | L2b Canal Lock [G] | (1200, 1350) ±120 | 运河闸口（若 T-GEO 确认存在）；水域阻隔，两岸交火 | 40 m |
| | L2c Orchard Cross [G] | (500, 1000) ±80 | 田埂交叉口，树篱缺口；载具难进 | 50 m |
| **L3 老城核心** | L3a Notre-Dame [R] | (1277, 897) ±40 | 教堂与周边墓地围墙；高塔可作观察（OP 点） | 50 m |
| | L3b Place de la République [R] | (1380, 990) ±120 | 拱廊广场，**长视线被拱廊打断**，多入口 | 45 m |
| | L3c Convent Block [G] | (1100, 720) ±120 | 大型街区，院墙与内院 | 50 m |
| **L4 铁路南缘** | L4a Station [R] | (1500, 520) ±250 | 站房与站台，开阔铁轨 | 45 m |
| | L4b Goods Yard [G] | (1250, 400) ±150 | 货场与路堤，线性掩体 | 50 m |
| | L4c Mill Quarter [G] | (1650, 800) ±150 | 东侧街区靠近沼泽，边缘暴露 | 45 m |
| **L5 东南草地** | L5a South Ridge Farm [G] | (1000, 200) ±80 | 南缘农场，较高地势，俯瞰田野 | 50 m |
| | L5b Taute Meadow Battery [G] | (1700, 250) ±80 | 草地炮位（德军后方），靠近 GER HQ | 55 m |
| | L5c Lane Crossroads [G] | (1400, 150) ±80 | 田间十字路口，树篱封闭 | 45 m |

**ASCII 示意（设计框架，北在上，约 1 字符 ≈ 55 m；`~`=洪泛沼泽，`#`=建成区，`+`=树篱区，`=`=堤道/道路，`R`=铁路）**

```
y=2048  +-----------------------------------------------------------+
        |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~|
        |~ [US HQ]=====B2(bridge)~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~|
  1500  |~~~++=======L1b  ~~~~~L2b(lock)~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~|
        |++++L1c ++ ====L1a(Ingouf)=====                  ~~~~~~~~~~~|
        |+++++++++  +++ ======= ###                      ~~~~~~~~~~~~|
  1000  |++L2c+++ ++++  =======L3b ###L4c     ~~~~~~~~~~~~~~~~~~~~~~~~|
        |+++ L2a +++  L3a###L3b##### ###                              |
        |++++++++++ +L3c### ####### RRRRRRRR~~~~~~~~~~~~~~~~~~~~~~~~~~|
   500  |++++++++++++++++++ L4b #### L4a ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~|
        |+++++++++++ L5a  +++ L5c       RRRR   L5b [GER HQ] ~~~~~~~~~~|
     0  +-----------------------------------------------------------+
        x=0                       1024                          x=2048
```

**主要进攻轴线与侧翼通道**

- **主轴**：堤道头 → L1a → 西北街区 → 老城 → 铁路 → 东南。
- **西南侧翼（仅步兵）**：L1c → L2c → L2a → 南缘农场；树篱缺口与田埂，**载具无法通行**（树篱高 ≥ 1.8 m 且无缺口 ≥ 4 m）。
- **东侧侧翼**：沼泽边缘浅水带，步兵可涉水（移速 ×0.5【待调参】），**暴露**于铁路路堤与东侧街区火力，作为高风险偷袭路线。
- **载具路线**：N13 堤道（单车道，易被一辆坦克封死）→ 市区主路（Rue 名称**未核实**，使用 OSM 道路）→ 南郊土路。堤道是整张图的**咽喉点**，也是反坦克炮/Panzerfaust 的预设位置。
- **步兵专属区**：西南树篱区、东侧沼泽浅滩、教堂墓地与修道院内院。

**视线管理（长视线如何打断）**

| 长视线来源 | 打断手段 |
|---|---|
| N13 堤道直线 | 路堤上的农舍与树篱穿插；路边设置报废车辆残骸 + 弹坑；每 ≈ 150 m 有路障/桥栏 |
| 教堂高塔俯瞰全城 | 塔内部作为 OP 位，**不可由玩家占领为狙击点**（仅 Spotter 兵种可进入，窗口数量 2）；周围 40–60 m 内保留较高屋顶遮挡 |
| 铁路路堤东西向 | 货场车厢（可破坏）、站台雨棚 |
| 沼泽开阔视距 | 芦苇丛（植被密度 ≥ 3 株/m²，视线遮蔽 50%）、雾 |
| 老城街道 | 街道不超过 120 m 的直线，用转角、拱廊、车辆残骸切断 |

**Garrison（自然候选位置，以 `garrison_candidates` 提供）**

| 阵营 | 候选 | 说明 |
|---|---|---|
| US | 西北果园 (250,1750)；西南农场 (350, 600) | 靠树篱与建筑掩护 |
| GER | 老城东端街区 (1700, 900)；南缘农场 (1000, 200) | 贴近据点，有掩护 |

**火炮阵地**

| 阵营 | 位置（设计框架外沿） | 说明 |
|---|---|---|
| US | 地图外西北（可玩区外 800 m，Saint-Côme 方向） | 105 mm 榴弹炮，**不可进入** |
| GER | 南部（可玩区外，Périers 方向）+ 草地炮位 L5b 的可进入迫击炮阵 | 81 mm 迫击炮 |

**Assault 模式分段方案（US 进攻、GER 防守，逐段推进，每段 1 个主攻据点 + 1 个次要）**

| 段 | 目标 | 说明 |
|---|---|---|
| 1 | L1a Ingouf Farm + L1b Southern Bridge | 夺取堤道头，建立前进 Outpost |
| 2 | L2a Market-Garden Fields + L2c Orchard Cross | 突破西郊树篱 |
| 3 | L3a Notre-Dame + L3b Place de la République | 夺取老城 |
| 4 | L4a Station + L4b Goods Yard | 越过铁路 |
| 5 | L5a South Ridge Farm（并摧毁 L5b 迫击炮阵） | 终局 |

**Skirmish 模式子区域**（每个 ~ 700 × 700 m，24–40 人）

| 子区 | 范围（设计框架） | 特点 |
|---|---|---|
| S1 老城 | x 900–1700, y 500–1200 | 城镇巷战 |
| S2 堤道—农场 | x 200–1100, y 1100–1900 | 堤道与桥口、农场 |
| S3 西南树篱 | x 100–1000, y 200–1100 | 步兵树篱战 |

**原创性声明（Carentan）**：本布局的依据只有三项：(1) 真实地理（堤道自西北入城、老城居中、东/北为洪泛沼泽、西南为 bocage），(2) 史实的攻防方向（美军自西北与北沿堤道推进，德军依托城镇与南郊防御），(3) 战术合理性（咽喉点、侧翼水体隔断、树篱步兵走廊）。据点命名与分布来自对上述三项的推导与我们自行设计的"5 排对角线"，没有参照任何商业游戏的同名地图；所有 [G] 名称为本项目原创。

### 3.7 性能与制作风险

| 风险 | 说明 | 对策 |
|---|---|---|
| 城镇建筑密度 | 老城核心 ~ 800×600 m，建筑 300–500 栋 | 可进入建筑 ≤ 25%；其余用低面数外壳 + 合并；按 64 m 区块流式；建筑 LOD0/1/2；WebGL2 无 Hi-Z → 预计算建筑遮挡体（§1.6 `occluders`） |
| 树篱数量 | 西南区树篱总长度 > 15 km | 树篱用样条 + GPU instancing 的"树篱段"，2 种变体 + 缺口件；碰撞用线性盒 |
| 沼泽水体 | 大面积水面 | 单一水面网格 + 简化着色（折射仅 High 以上）；浅水可涉水的**逻辑区**与视觉解耦 |
| 苹果园/树 | 植被数量 | 八面体 impostor；Low 档减半 |
| 1944 年修正工作量 | 手工多边形 | 见 §7 工时估计 |
| 数据缺口 | 车站/广场/农场位置未核实 | T-GEO 为第一任务；设计以拓扑为准（§0.4） |

### 3.8 M1 垂直切片用的 "1/3 区域"

**选择**：设计框架矩形 **x 380–1560，y 520–1700**（约 1180 × 1180 m ≈ 全图面积的 1/3），覆盖 **L1（堤道头）、L2（西郊）、L3（老城核心）**。

| 项 | 值 |
|---|---|
| 含据点 | L1a Ingouf Farm、L1b Southern Bridge、L2a、L2c、L3a、L3b（共 6 个，M1 取 3 个活动据点：L1a、L2a、L3a） |
| M1 HQ | US：(480, 1600)；GER：(1480, 560)（相对完整图的临时位置，**不要写入完整图的数据**） |
| 地形 | 在该矩形内用完整 1 m 精度；矩形外用 8 m 远景环，边界处加不可通行墙（视觉为矮灌木与雾） |
| 20 人规模 | 每队 10 人；Garrison 各 1 个（US：(420, 1550)；GER：(1500, 600)） |
| 目的 | 验证 5 兵种、1 辆坦克、Garrison 与补给、部署流程 |
| 坦克路线 | N13 堤道 → L1a → L3 老城（M1 仅含主路） |

---

## 4. Omaha Beach（1944-06-06）

### 4.1 战役背景与时间点

- **时间**：1944-06-06，H-Hour 06:30（Omaha 登陆起始），潮水上涨中；**潮位数据未核实**。
- **美军**：V Corps：29th Infantry Division（116th RCT，Dog/Charlie 一带，Vierville 方向）、1st Infantry Division（16th、18th RCT，Easy/Fox 一带，Colleville 方向）；第 2、5 游骑兵营（Pointe du Hoc、Pointe de la Percée 一带）。据 Wikipedia 摘要：116th 登陆 Dog Green、Dog White、Dog Red、Easy Green（Vierville-sur-Mer 之下）。
- **德军**：352. Infanterie-Division（海岸防御，抗登陆；716. 师一部在滩头前线）；以 Widerstandsnest（WN，抵抗巢）布防于 draws（出口冲沟）两侧。
- **性质**：本章对应"强攻 + 缓慢突破海滩"，**Assault 模式为主**；Frontline 模式给出对称化变体。

### 4.2 可玩区选择

**选段：Dog Green / Vierville 至 Les Moulins（D-1 至 D-3 出口）**，理由：

1. **地理边界天然**：西侧为 Pointe de la Percée 悬崖带，作为不可通行边界；东侧为 Saint-Laurent/Les Moulins 一带。
2. **出口清晰**：D-1（Vierville draw）、D-3（Les Moulins）等 draws 位置有大量史料；D-1 是通向 Vierville 的**唯一铺装出口**，战术身份鲜明。
3. **WN 覆盖丰富**：WN 72、71、70、68、66 等落在该段（编号对应与位置在资料间有冲突，见 4.4）。
4. **数据可得**：Calvados 属 IGN LiDAR HD 覆盖范围（**未核实具体年份**）。
5. **避免 Colleville 段的问题**：Easy Red/Fox Green 一带现为 Normandy American Cemetery 及纪念园区，大面积人工地貌，1944 状态恢复成本更高，且游戏性上与"Dog Green"相比更偏平线。

| 项 | 值 |
|---|---|
| 中心 | 49.37222°N, 0.88984°W（UTM30N E 653184, N 5470976，**暂定**） |
| 边界框 | 见 §1.3（SW 652160/5469952；NE 654208/5472000） |
| 框选理由 | 西北角含 Vierville 村与 D-1 出口；东南含 Saint-Laurent 外沿与 D-3；北部为海滩与近岸水域（含障碍带）；南部含约 1 km 纵深的高原。**海岸线走向大致 WNW—ESE，暂定贯穿设计框架东北部；T-GEO 需用 OSM 海岸线与 GSGS 校正，这可能导致框内海面比例较大。** |
| 锚点 | A1：Vierville-sur-Mer 教堂（Wikipedia 给出的是**市镇质心** 49.375°N, 0.9039°W ≈ UTM E 652155, N 5471257，用作粗略位置）；A2：Saint-Laurent-sur-Mer（质心 49.3594°N, 0.8764°W ≈ E 654199, N 5469579，**位于框外南缘 ~370 m**）。教堂精确坐标 **未核实** |
| 远景环 | 8 km × 8 km：北为英吉利海峡（水面）；东至 Colleville、Port-en-Bessin 方向海岸；西至 Pointe du Hoc、Grandcamp 方向（悬崖海岸）；南至 Bayeux 方向田野 |

> **注意**：Wikipedia 给的坐标为市镇质心，不等于村中心，也不等于海滩出口位置；本节所有"设计框架"位置均为**暂定**，T-GEO 必须用 OSM `natural=coastline` 与 WN 史料点位重新锚定。

### 4.3 史料来源清单

| 资料 | 类型 | URL | 许可证 | 用途 | 核实状态 |
|---|---|---|---|---|---|
| Omaha Beachhead（Taylor, American Forces in Action, 1945） | 官方战史 | https://www.ibiblio.org/hyperwar/USA/USA-A-Omaha | 美国政府作品（常识，非法律意见）；转录页版权声明**未逐页核实** | 登陆部署、各 draw、障碍与 WN 描述 | 已核实（搜索结果 + 页面存在） |
| Cross-Channel Attack（Green Book） | 官方战史 | 见 §3.3 | 同上 | Omaha 章节 | 已核实（前言） |
| German strongpoints in Normandy: WN 60/61/62…（dday-overlord.com） | 二手 | https://www.dday-overlord.com/en/d-day/atlantic-wall/strongpoints/wn-62 ；https://www.dday-overlord.com/en/d-day/atlantic-wall/strongpoints/wn-61 | 版权所有，仅取史实 | WN 位置与武器 | 间接核实（搜索摘要；各页内容与 draw 编号之间**有互相矛盾**） |
| Map of German Defenses at Omaha Beach（Jewish Virtual Library） | 二手 | https://www.jewishvirtuallibrary.org/map-of-german-defenses-at-omaha-beach | 未核实 | 防御配置参考 | 间接核实 |
| strijdbewijs.nl Omaha German 页 | 二手 | https://www.strijdbewijs.nl/omaha/omahagerman.htm | 未核实 | WN 列表 | 间接核实 |
| Forgotten Fights: The Taking of WN61（NWWIIM） | 二手 | https://www.nationalww2museum.org/node/53861 | 版权所有 | WN 61 细节（Fox Green，**框外**，仅作对比） | 间接核实 |
| Vierville-sur-Mer / Saint-Laurent-sur-Mer / Colleville-sur-Mer（Wikipedia） | 坐标/概述 | https://en.wikipedia.org/wiki/Vierville-sur-Mer ；https://en.wikipedia.org/wiki/Saint-Laurent-sur-Mer ；https://en.wikipedia.org/wiki/Colleville-sur-Mer | CC BY-SA；仅取坐标 | 锚点（质心） | 已核实（抓取） |
| IGN LiDAR HD / RGE ALTI | 地形 | §2.1 | Etalab 2.0 | 高度图 | 部分核实 |
| GSGS 4250 对应图幅（Omaha 一带） | 历史图 | §2.3 | 未核实 | 道路、海岸、悬崖 | 图幅号 **未核实** |
| NARA/Signal Corps D-Day 照片 | 影像 | §2.4 | 公共领域（条目级核查） | 障碍带形态、车辆、人员装备参考 | 部分核实 |

### 4.4 真实地标清单

> **WN 编号冲突说明**：搜索结果对 WN 66/67/68/69/71/72 的位置描述互相矛盾（例如有来源把 88 mm 炮归于 WN 71，另有来源归于 WN 72；WN 67 在"保护 Saint-Laurent"与"西部高原"之间摇摆）。本项目**以 Taylor（Omaha Beachhead）与 Cross-Channel Attack 原文为准**，原文未核实前下表只给"候选"，由 T-GEO 的史料校对任务裁定。

| 地标 | 位置（设计框架，暂定） | 1944 状态 | 现状 | 还原精度 | 参考资料 |
|---|---|---|---|---|---|
| 海滩障碍带（Rommel's asparagus、Element C、水雷柱、刺猬） | 高潮线向海 40–250 m 的三排 | 满潮区障碍、潮涨时淹没 | 无 | **B（密度/排布）** | Taylor；NARA 照片 |
| Shingle 砾石堤与海堤 | 沿 HWL 向陆 5–20 m，宽 10–15 m | 砾石堤 + 局部木/混凝土海堤（Vierville 段有混凝土海堤） | 局部存在 | **A（轮廓）** | LiDAR；航拍；Taylor |
| D-1 Vierville draw | 设计框架西北，约 (200, 1650) ±250（**待锚定**） | 唯一铺装出口；两侧 WN；路障/反坦克壕 | 现为 D517 | **A** | Taylor；OSM |
| D-3 Les Moulins draw | 设计框架东部，约 (1800, 1050) ±250（**待锚定**） | 土路、沟壑 | 存在 | **A** | Taylor；GSGS |
| WN 72（Vierville draw 一侧，含 88 mm 说法冲突） | 约 (130, 1560) ±200（**候选**） | 最坚固支点之一（多来源） | 部分遗存 | **B** | dday-overlord；Taylor 待校 |
| WN 71（D-1 另一侧/高原） | 约 (330, 1680) ±200（**候选**） | | | **B** | 同上 |
| WN 70（Saint-Laurent—Vierville 之间） | 约 (780, 1250) ±250（**候选**） | 守开阔田地 | | **B** | 搜索摘要 |
| WN 68 / WN 66（D-3 两侧） | 约 (1650, 1170)/(1950, 1020) ±250（**候选**） | | | **B** | 搜索摘要，位置互相矛盾 |
| WN 67（Saint-Laurent） | 框外南缘附近 | 来源矛盾 | | **C** | |
| WN 73（Pointe de la Percée/Raz de la Percée） | 框外西北 | | | 不入框（远景） | |
| Vierville-sur-Mer 村与教堂 | 约 (70, 1190) ±300（质心推算，**教堂精确位置未核实**） | 石屋村落，教堂 | 存在 | **B** | Wikipedia 质心；OSM |
| Saint-Laurent-sur-Mer 外沿与教堂 | 框外南缘，约 (2040, -370) | | | 框外 | |
| Hamel-au-Prêtre 一带小村落（原住宅点） | **未核实** | | | | |
| 悬崖与高原 | 西北至 Pointe de la Percée；南缘高原 30–50 m **（需 LiDAR 实测）** | | | **A（LiDAR）** | LiDAR |

### 4.5 环境设定

| 项 | 设定 |
|---|---|
| 季节/天气 | 1944-06-06，初夏；低云、海风、能见度 8–10 km；海浪中等偏大（**未核实：历史气象需查**） |
| 时段 | 清晨 06:30（朝东偏北方向有薄明；影子长；烟雾与炮烟）；M4 可选 08:30（上午） |
| 植被 | 海滩无植被；悬崖草坡与灌木；高原果园、田块、小片树篱；海滨稀疏芦苇 |
| 材质 | 潮湿深色沙、砾石（圆润）、混凝土（海堤、碉堡）、金属障碍（锈蚀）、泥土路、海面（灰绿） |
| 建筑 | 诺曼底石屋（村落）、海滨度假屋（战前木/砖）、混凝土 WN 工事（Tobruk、H677/H667 炮位等，具体型号**需按史料**） |
| 采购对照 | "Atlantic Wall bunker"、"Czech hedgehog / hedgehog"、"Rommel asparagus"、"LCVP landing craft"、"Higgins boat"、"seawall"、"shingle terrain"、"stone farmhouse" |

### 4.6 原创关卡设计

**设计原则**：
- 美军（攻方）从**海上**（北/东北）登陆，德军守**高原与悬崖**（南/西）。
- Frontline 模式中 5 条战线按**海岸—悬崖—高原**纵深排列。但 Omaha 的非对称性很强：**Assault 为主模式**，Frontline 为变体（US 默认拥有更多 Manpower 重生，DE 守 + 反击；平衡【待调参】）。
- US HQ：海上登陆艇锚地（**不可进入的出生区**，部署后由登陆艇送上滩）；GER HQ：高原森林/村落后方。

**HQ 与出生点**

| 阵营 | HQ（设计框架） | 说明 |
|---|---|---|
| US | 离岸 Landing Craft Anchorage：(1500, 1950) ±100（水上） | 玩家通过登陆艇上岸；到岸点分布于 HWL 外 |
| GER | (900, 300) ±80，高原树篱围合的农场 | 靠近高原与 Saint-Laurent 外沿 |

**Frontline 变体：5 条战线 × 2–3 候选**

| 战线 | 据点 | 位置 (x,y) ±误差 | 战术特征 | 强点半径 |
|---|---|---|---|---|
| **L1 滩头障碍带** | L1a Tide-Line Obstacles [G] | (1500, 1700) ±100 | 障碍林，爆破才能开通；仅轻掩体 | 50 m【待调参】 |
| | L1b West Surf Line [G] | (500, 1640) ±100 | 靠近 Vierville draw 前沙滩 | 50 m |
| | L1c East Surf Line [G] | (1850, 1400) ±100 | 靠近 D-3 | 50 m |
| **L2 砾石堤/海堤** | L2a Seawall Run [G] | (700, 1480) ±120 | 海堤线性掩体，近水 | 45 m |
| | L2b Shingle Bank Centre [G] | (1250, 1330) ±120 | 砾石堤，起伏小，开阔 | 45 m |
| | L2c D-1 Mouth [R/G] | (200, 1600) ±200 | 出口封堵，步兵拥塞 | 40 m |
| **L3 悬崖与出口** | L3a D-1 Vierville Draw [R] | (260, 1500) ±250 | 唯一铺装道，坦克可过；两侧 WN | 45 m |
| | L3b WN 70 Field [R/G] | (780, 1250) ±250 | 开阔田间的防御工事 | 50 m |
| | L3c D-3 Les Moulins [R] | (1800, 1050) ±250 | 土路冲沟 | 45 m |
| **L4 高原村落** | L4a Vierville Village [R] | (70, 1190) ±300 | 石屋村、教堂 | 55 m |
| | L4b Orchard Plateau [G] | (700, 900) ±150 | 高原果园、树篱 | 55 m |
| | L4c Moulin Farms [G] | (1500, 700) ±150 | 农场群 | 50 m |
| **L5 德军纵深** | L5a Reserve Hedgerow Line [G] | (1000, 450) ±100 | 德军预备线、树篱 | 55 m |
| | L5b Plateau Crossroads [G] | (1500, 300) ±100 | 十字路口，炮位 | 50 m |
| | L5c HQ Farm [G] | (900, 300) ±80 | GER HQ 附近 | 50 m |

**ASCII 示意（设计框架，北在上；`~`=海，`.`=沙滩，`=`=海堤/砾石，`^`=悬崖/坡，`#`=村落，`+`=树篱高原；海岸线为暂定）**

```
y=2048  +-----------------------------------------------------------+
        |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~|
        |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~[US HQ boats]~~~~~~~~~~~~~~~|
  1500  |....L1b........L1a(obstacles)....~~~~~~~~~~~~~~~~~~~~~~~~~~~|
        |=L2c==L2a=====L2b=======  ......L1c.......~~~~~~~~~~~~~~~~~~|
        |^^L3a(D-1)^^^^^^^^^^^^^^^^^^^^^^^^=====.......~~~~~~~~~~~~~~|
  1000  |^^#L4a  +++L3b+++^^^^^^^^^^^^^^^^^^^^^^^^^L3c(D-3)====......|
        |##   ++++ L4b ++++++++++++^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^|
        |   +++++++++++++++++++ L5a +++++++++ L4c ++++++++++++++++++|
   500  |++++++++++++ L5c [GER HQ] ++++++++  L5b +++++++++++++++++++|
     0  +-----------------------------------------------------------+
        x=0                       1024                          x=2048
        （海岸线、悬崖位置以 T-GEO 结果为准，此图仅表示拓扑顺序）
```

**主要进攻轴线与侧翼通道**

- **主轴 A（西）**：滩头 → D-1 Vierville draw → Vierville 村（唯一铺装路，载具）。
- **主轴 B（东）**：滩头 → D-3 Les Moulins 冲沟 → 高原农场群（土路，轻载具）。
- **中央（仅步兵）**：滩头 → 陡坡与冲沟 → WN 70 田地 → 果园高原（载具无法通行，坡度 > 35°）。
- **侧翼**：西侧悬崖脚下沿海岸徒步（沙滩狭窄、暴露于 WN 72/71 火力）；东侧沿海堤向 D-3 横向移动。
- **载具通行路线**：D-1 铺装路（Sherman/Crewman），D-3 土路（视坡度）；其余仅步兵。
- **步兵专属区**：悬崖带、海堤线、冲沟。

**视线管理**

| 长视线 | 手段 |
|---|---|
| 海滩 800 m+ 开阔视距（最大风险） | 障碍带、沉没/搁浅登陆艇与车辆残骸（至少每 60 m 一个）、弹坑、沙丘/沙脊起伏（高 0.5–1 m 的"潮沙脊"，LiDAR 的真实微地形 + 手工增强 ≤ 0.5 m）；烟幕与炮烟（由游戏逻辑触发） |
| 高原俯瞰滩头 | 悬崖边缘的灌木、WN 的混凝土掩体自身遮挡；德军纵深火力的"最大视距"限制于 500 m（雾 + 烟） |
| 海堤直线 | 海堤中途设"断口"（排水沟、台阶、物资堆） |

**Garrison / Outpost**

| 阵营 | 候选 | 说明 |
|---|---|---|
| US | **海滩上不设 Garrison**；砾石堤后（≥ 2 号战线占领后）可建 Outpost：(700, 1400)、(1250, 1300)；Airhead 不适用 | 登陆艇补给；Garrison 在 L3 占领后启用于 (1000, 1100) 一线 |
| GER | Vierville 村石屋 (70, 1190)；高原农场 (900, 300) | 靠建筑与树篱 |

**火炮阵地**

| 阵营 | 位置 | 说明 |
|---|---|---|
| US | 舰炮（海上，框外北） | 对"已观测到"的目标提供精确打击，作为 Commander 能力（Precision Strike） |
| GER | 高原后方（框外南）+ L5b 的 88 mm / 迫击炮阵 | 对滩头区域有压制 |

**Assault 分段方案（US 进攻）**

| 段 | 目标 | 说明 |
|---|---|---|
| 1 | L1a/L1b/L1c（任选其二） | 清障、占滩 |
| 2 | L2a + L2b（海堤/砾石堤） | 站住脚跟 |
| 3 | L3a D-1 + L3c D-3（拿下一个出口即可进入下一段） | 突破 |
| 4 | L4a Vierville + L4b Orchard Plateau | 内陆 |
| 5 | L5a + L5b | 纵深 |

**Skirmish 子区域**

| 子区 | 范围（设计框架） | 特点 |
|---|---|---|
| S1 D-1 出口 | x 0–800, y 1150–1900 | 狭窄出口战 |
| S2 悬崖与冲沟 | x 700–1500, y 900–1500 | 步兵垂直战 |
| S3 Vierville 高原 | x 0–900, y 400–1300 | 村落与树篱 |

**原创性声明（Omaha）**：布局来自三项：(1) 登陆方向与海岸/海堤/悬崖/高原的真实垂向剖面；(2) draws 作为唯一通道的真实地貌与德军 WN 的火力覆盖；(3) 战术合理性——步兵被引导进入出口的瓶颈，坡度 > 35° 的冲沟与悬崖形成步兵专属路线。所有 [G] 名称原创。

### 4.7 性能与制作风险

| 风险 | 说明 | 对策 |
|---|---|---|
| 海滩开阔视距 | 海滩 + 近海水面，开阔 | 障碍/残骸/起伏 + 雾；海面远处截断 |
| 障碍物实例数 | 障碍带可达数千实例 | GPU instancing + 距离分级 + 合并 |
| 水体与潮线 | 潮间带动态 | 静态潮位（固定 06:30），不做潮汐动画 |
| 现状地形与 1944 差异 | 大面积人工修复 | §1.5 修正；人工 |
| 悬崖碰撞 | 陡坡 | 1 m 高度图 + 专用悬崖网格（可选）；导航网格标注不可通行 |
| 数据缺口 | 海岸线与 WN 位置 | T-GEO + 史料校对 |

### 4.8 M4 说明

Omaha 的 M4 范围：Assault 为主、Frontline 变体、Skirmish；M2 的所有通用系统（Garrison、补给）需支持"登陆艇出生"这类特殊出生点（`spawns.json` 的 `kind: "boat"`）。

---

## 5. Foy（1945-01）

### 5.1 战役背景与时间点

- **时间**：1945-01 上旬至中旬。101st Airborne 的 506th PIR 在 Foy 北侧的 Bois Jacques（"Jack's Wood"）掘壕，对德军占领的 Foy 村发动进攻（据搜索摘要：Easy Company 于 01-09 至 01-13 受损最重，01-13 率先进攻 Foy，村庄多次易手；01-14 北移指向 Noville；01-15 肃清 Rachamps——**日期细节需 Green Books（如 Cole 的 *The Ardennes: Battle of the Bulge*）确认**）。
- **美军**：101st Airborne，506th PIR（含 2nd Bn 的 Easy Company）。
- **德军**：参与 Bastogne 北线的德军部队；**具体番号未核实**（需查 Cole, *The Ardennes: Battle of the Bulge* 与德军战斗序列，候选：26. Volksgrenadier-Division、第 15 装甲掷弹兵师一带单位，**皆未核实，不得写入游戏而不经核实**）。
- **性质**：冬季、雪地、林缘与开阔牧场攻防；**炮击为主要伤害来源**。

### 5.2 可玩区选择

| 项 | 值 |
|---|---|
| 中心 | 50.04404°N, 5.74470°E（UTM31N E 696512, N 5547136，**暂定，±1 km**） |
| 边界框 | 见 §1.3（SW 695488/5546112；NE 697536/5548160） |
| 框选理由 | Foy 在维基镜像中仅给到度分精度（50°03′N, 5°45′E，即 ±1 km 误差），因此**必须以瓦隆 LiDAR + OSM 重新锚定**。框要包含：Foy 村及其十字路口、N30（Bastogne—Houffalize）、Bois Jacques 森林线与散兵坑、南北向牧场；北侧延伸至 Noville 方向（框外）；西侧延伸至 Recogne 方向（框外，**方向待核实**） |
| 锚点 | A1：Foy 村十字路口（OSM）；A2：Bois Jacques 北缘与 Easy Company 纪念碑（2004 年，在林缘，Wikipedia 提及） |
| 远景环 | 8 km × 8 km：南为 Bastogne 市镇；北为 Noville/Houffalize 方向；东为 Bourcy；西为 Recogne、Longvilly 方向；全覆盖冬季针叶林与牧场 |

> **设计假设**（用于设计框架）：N30 沿设计框架中部近南北向穿过 Foy（x ≈ 1150–1300）；Foy 村在北半部，Bois Jacques 在村庄南/西南；Bastogne 方向为南；Noville 方向为北。**这些相对关系依据"Easy Company 自 Bois Jacques 向村庄进攻、村庄在其前方"这一描述作的推断**，需 T-GEO 验证。

### 5.3 史料来源清单

| 资料 | 类型 | URL | 许可证 | 用途 | 核实状态 |
|---|---|---|---|---|---|
| Foy, Belgium（Wikipedia） | 概述 | https://en.wikipedia.org/wiki/Foy,_Belgium | CC BY-SA；仅取事实 | 位置（度分）、Bois Jacques 纪念碑 | 已核实（抓取） |
| Jack's Wood（Liberation Route Europe） | 旅游/史料 | https://www.liberationroute.com/pois/386/jacks-wood | 版权所有 | 散兵坑仍可见 | 间接核实 |
| Band of Brothers 研究（PRT） | 研究笔记 | https://www.band-of-brothers.nl/artikel/res_prt_foy_01.htm ；https://band-of-brothers.nl/artikel/res_prt_foy_00.htm | 版权所有，仅取史实 | Bois Jacques 阵地位置 | 间接核实 |
| Easy Company in Foy（Landmark Scout） | 旅游 | https://www.landmarkscout.com/easy-company-101st-airborne-division-in-the-battle-of-the-bulge-foy-belgium | 版权所有 | 路线 | 间接核实 |
| The Ardennes: Battle of the Bulge（Cole, CMH Green Book） | 官方战史 | **URL 未核实**（HyperWar 或 CMH 官网） | 美国政府作品（常识） | 506th 与德军番号、日期 | **未核实**（需人工找原文） |
| 瓦隆 LiDAR | 高程 | §2.1 | **官方许可证未核实** | 高度图 | 部分核实 |
| GSGS 4336（1:100,000 比利时） / AMS 比利时 1:50,000 | 历史图 | §2.3 | 未核实 | 1945 道路/森林布局 | 间接核实 |
| 瓦隆历史正射影像/PICC | 影像/矢量 | PICC 说明 PDF：https://geoportail.wallonie.Be/files/PDF/PICC/PICC%2017102023/2%20-%20Nouveaut%c3%a9s%20PICC%2c%20G%c3%a9odonn%c3%a9es%2c%20SPW%20Digital.pdf（搜索结果出现） | **未核实** | 建筑足迹、道路 | 间接核实 |
| OSM | 现代矢量 | §2.5 | ODbL | 初稿 | 间接核实 |

### 5.4 真实地标清单

| 地标 | 位置（设计框架，暂定） | 1945 状态 | 现状 | 还原精度 | 参考资料 |
|---|---|---|---|---|---|
| Foy 村十字路口 | 约 (1180, 1500) ±300（**锚点，待核实**） | 村中心；德军坚守 | 存在 | **A** | OSM；PICC |
| 村教堂 / St. Barbara Chapel | **位置未核实**（Wikipedia 仅有 "St. Barbara Chapel" 图说） | | | **B** | 待定 |
| N30（Bastogne—Houffalize） | 近南北向穿过设计框架 x≈1150–1300 | 主干道，被冰雪覆盖；两侧田地 | 存在 | **A** | OSM；GSGS |
| Bois Jacques（Jack's Wood）森林线 | 村庄南/西南 (300–1100, 450–1000)（**待核实**） | 冷杉林；101st 掘散兵坑，掩体有木盖 | 散兵坑犹存（多来源） | **A（林缘）/B（坑位）** | liberationroute；band-of-brothers.nl |
| 散兵坑群 | 林内沿北缘分布，间距 ~10–25 m | 单/双人坑，木盖/原木，积雪 | 凹坑可见 | **B** | PRT 研究笔记（具体坑位**需人工**） |
| Easy Company 纪念碑（2004） | 林缘，位置**待核实** | 无（战后） | 存在 | 不入游戏（现代） | Wikipedia |
| Recogne 方向 | 框外西缘（**方向待核实**） | | | 远景 | |
| Noville 方向 | 框外北（N30 北） | | | 远景 | |
| 田间/牧场 | 森林与村庄之间 300–400 m 开阔雪地 | 开阔，雪厚，冻土 | 牧场 | **A（范围）** | LiDAR |

### 5.5 环境设定

| 项 | 设定 |
|---|---|
| 季节/天气 | 1945 年 1 月，严寒（-10 ~ -15 °C 量级，**未核实**），积雪 20–40 cm（**未核实**）；雾、低云；偶有炮击烟 |
| 时段 | 供选：黎明 08:00（冷蓝色调）、正午、黄昏 16:30（短日）；夜间不做 |
| 植被 | 针叶人工林（冷杉/云杉）、落叶树（山毛榉）、牧场、篱笆 |
| 材质 | 积雪、冻土、结冰路面、雪覆盖的木头/树枝、石材墙、木板屋 |
| 建筑风格 | **阿登农舍**：石砌（砂岩/页岩）+ 木梁，板岩/瓦屋顶，村庄为长条形院落沿路；谷仓 + 住屋合一 |
| 采购对照 | "Ardennes farmhouse"、"Belgian stone village house"、"snow terrain material"、"spruce forest pack"、"foxhole / log-covered dugout"、"winter tree pack" |

### 5.6 原创关卡设计

**设计原则**：
- 真实的态势：美军在**森林南缘/林内**防守并向北进攻，德军控制**村庄与 N30 北段**。游戏中双方轮换攻守，Frontline 战线沿 **南 → 北** 排列：森林 → 林缘—牧场 → 村庄 → 北部道路。
- 开阔牧场（~ 300–400 m）是整张图的**杀戮区**；森林内部视距短（< 40 m）；村庄是复杂近战核心。
- 雪地：移动噪声、脚印（装饰）、视距受雾影响。

**HQ**

| 阵营 | HQ（设计框架） | 说明 |
|---|---|---|
| US | (700, 250) ±80，Bois Jacques 南部林内空地 | 距 Bastogne 方向最近 |
| GER | (1350, 1900) ±80，村庄北、N30 沿线农场 | 靠 Noville 方向 |

**Frontline：5 条战线 × 2–3 候选**

| 战线 | 据点 | 位置 (x,y) ±误差 | 战术特征 | 强点半径 |
|---|---|---|---|---|
| **L1 森林南部** | L1a Rear Clearing [G] | (700, 400) ±100 | 林内空地，US 重整点 | 50 m【待调参】 |
| | L1b Fir Plantation [G] | (400, 600) ±120 | 密植幼林，视距极短 | 45 m |
| | L1c South Road Cut [G] | (1000, 450) ±100 | 林内路堑，车辆可入（雪地减速） | 45 m |
| **L2 Bois Jacques 北缘** | L2a Foxhole Line West [G] | (500, 900) ±120 | 林缘散兵坑，俯瞰牧场 | 50 m |
| | L2b Foxhole Line East [G] | (900, 950) ±120 | 林缘散兵坑，N30 视线 | 50 m |
| | L2c Log Dugouts [G] | (700, 800) ±100 | 林内加盖掩体，抗炮击 | 45 m |
| **L3 牧场与田埂** | L3a Snow Meadow Fence [G] | (800, 1150) ±120 | 开阔雪地中孤立围栏与低坡 | 55 m |
| | L3b Drainage Ditch [G] | (1100, 1100) ±120 | 沟渠线，步兵掩体 | 45 m |
| | L3c Roadside Barn [G] | (1250, 1050) ±120 | 路边谷仓（N30 旁） | 50 m |
| **L4 Foy 村** | L4a Foy Crossroads [R] | (1180, 1500) ±300 | 村十字路口，N30 与支路 | 50 m |
| | L4b Chapel Yard [R/G] | (1050, 1580) ±300 | 教堂/小礼拜堂周边（位置待核实） | 45 m |
| | L4c Long Farm [G] | (1300, 1450) ±250 | 阿登长条农舍，多房间 | 50 m |
| **L5 北部道路** | L5a North Road Bend [G] | (1250, 1800) ±120 | N30 北段弯道，向 Noville | 50 m |
| | L5b Ridge Farm [G] | (1000, 1850) ±120 | 北侧高地农场，德军炮观 | 50 m |
| | L5c Forest Edge North [G] | (1500, 1750) ±120 | 东北林缘 | 50 m |

**ASCII 示意（设计框架，北在上；`T`=针叶林，`:`=雪地牧场，`#`=村庄，`N`=N30 公路，`.`=田埂/沟渠）**

```
y=2048  +-----------------------------------------------------------+
        |     TTTT     L5b   N   L5a      L5c TTTT       [GER HQ]    |
        |    TTTT          N   N                TT                   |
  1500  |  :::::::::::::::: N  #L4b#L4a#L4c#::::::::::::::::::::::::: |
        |  ::::::::::::::: N   ###########    ::::::::::::::::::::::: |
        |  ::::L3a:::::::: N L3b..L3c ::::::::::::::::::::::::::::::: |
  1000  |  ::::::::::::::::N: ::::::::::::::::::::::::::::::::::::::: |
        |TTTTT L2a TTTTTT L2b TTT N TTTTTTTTT         TTTTTTT         |
        |TTTTTTT L2c TTTTTTTTTTTT N TTTTTTTTT          TTTT           |
   500  |TTTTTTTTT L1b TTTT L1a TTTTT L1c  TTT                        |
        |TTTTTTTTTTTTTTT [US HQ] TTTTTTTT N                           |
     0  +-----------------------------------------------------------+
        x=0                       1024                          x=2048
        （森林、村庄范围依 LiDAR/OSM 锚定后确定）
```

**主要进攻轴线与侧翼通道**

- **主轴**：林缘（L2）→ 牧场（L3，杀戮区）→ 村庄南缘（L4）。
- **西侧翼**：沿林缘西行再向北，利用树篱与雪沟接近村西；暴露较少，距离长。
- **东侧翼**：沿 N30 东侧沟渠；有车辆可能；**暴露于北部道路德军炮观**。
- **载具**：N30（车辆主路，冰雪减速）；林内路堑 L1c；牧场雪地可通行但**陷入概率**（雪深 > 0.4 m：减速；【待调参】）。
- **步兵专属区**：林内、散兵坑线、沟渠。

**视线管理**

| 长视线 | 手段 |
|---|---|
| 牧场 400 m 杀戮区 | 田埂、孤立围栏、弹坑、雪堆、雾（能见度 450–600 m）、倒木；每 80 m 至少一处可蹲伏的矮掩体 |
| N30 直线 | 路旁树列、车辆残骸、路障 |
| 森林内 | 视距自然 < 40 m；密度控制，避免性能与"完全看不见" |
| 村庄 | 房屋错位布置，院墙与转角 |

**Garrison**：US 在林内 (700, 250)、(450, 850)；GER 在 Long Farm (1300, 1450)、北农场 (1000, 1850)。

**火炮阵地**

| 阵营 | 位置 | 说明 |
|---|---|---|
| US | 框外南（Bastogne 方向）；105 mm | 对牧场与村庄的炮击 |
| GER | 框外北（Noville 方向）；迫击炮与榴弹炮 | 对林缘散兵坑的炮击（炸树的树爆效果，**视觉特效，需素材**） |

**Assault 分段方案（US 进攻 Foy，对应史实）**

| 段 | 目标 | 说明 |
|---|---|---|
| 1 | L2 任一散兵坑线 → 越过 L3 | 林缘出击 |
| 2 | L3a + L3c | 夺取牧场据点，站稳 |
| 3 | L4a Foy Crossroads + L4c Long Farm | 夺村 |
| 4 | L4b Chapel Yard | 清扫 |
| 5 | L5a + L5b | 追击 |

**Skirmish 子区域**

| 子区 | 范围 | 特点 |
|---|---|---|
| S1 林缘散兵坑 | x 300–1200, y 700–1100 | 森林对射 |
| S2 牧场与沟渠 | x 600–1500, y 1000–1400 | 开阔交火 |
| S3 Foy 村 | x 900–1600, y 1350–1750 | 村庄巷战 |

**原创性声明（Foy）**：布局来自三项：(1) Bois Jacques—牧场—Foy 的真实相对关系（林—空地—村）；(2) N30 作为南北主干线；(3) 战术合理性（森林提供隐蔽与防炮、开阔牧场作杀戮区、村庄作近战核心）。所有 [G] 名称原创。

### 5.7 性能与制作风险

| 风险 | 说明 | 对策 |
|---|---|---|
| 雪地森林植被量 | 针叶林密度高，树数以十万计 | 八面体 impostor、LOD、剔除；Low 档减半；林内视距短，可降低远景树密度 |
| 雪材质 | 大面积雪面反光 | 使用统一 PBR + 噪声；SSAO/GTAO 在林内易产生噪声，Low 关闭 |
| 雾 | 与 WebGL2 降级 | 高度雾降级，保持能见度设定 |
| 数据许可证 | 瓦隆 LiDAR 官方许可证未核实 | 人工核实后写入 LICENSES.md；备选：用 Copernicus GLO-30 + 手工微地形（质量显著下降，仅应急） |
| 位置锚定误差 | Foy 坐标仅度分精度 | T-GEO 优先 |

---

## 6. 地图数据交付格式（`/data/maps/{map_id}/`）

`map_id` 取：`carentan`、`omaha_dog_green`、`foy`。所有坐标除特别说明外使用**本地米坐标**（西南角原点，x 东，y 北，z 上）。

### 6.1 文件清单

```
/data/maps/{map_id}/
  meta.json                 坐标系、锚点、版本、来源清单、授权
  heightmap_1m.r16          2049×2049，小端 uint16，高程量化（meta 内含 hmin/hmax）
  heightmap_far_8m.r16      1025×1025 覆盖 8192 m
  splat_0.png / splat_1.png 材质权重（RGBA×2，最多 8 层），2048×2048（2 m/px 起，High 可 4096）
  colormap_far.ktx2         远景环颜色（可选）
  roads.geojson             道路折线 + 属性
  water.geojson             水体多边形与折线
  hedgerows.geojson         树篱折线
  forests.geojson           森林/植被多边形（密度、树种）
  buildings.geojson         建筑足迹（含样式、可进入性）
  landmarks.json            真实地标
  zones.json                据点、战线、Assault 段、Skirmish 子区
  spawns.json               HQ、Garrison 候选、Outpost 候选、Airhead 候选、OP
  artillery.json            火炮阵地与观测点
  vehicle_routes.json       载具路线与不可通行区
  terrain_edits.geojson     1944 修正多边形
  occluders.json            预计算遮挡体（WebGL2）
  navmesh.bin               Recast 烘焙结果（步兵）+ navmesh_vehicle.bin
  chunks/                   64 m 区块资源索引（由流水线生成）
  LICENSE_NOTICE.md         本图数据的署名与许可证（含 ODbL 声明）
```

### 6.2 Schema 草案（JSON；仅示意，非代码）

**meta.json**

```json
{
  "schema": "frontline.map.meta/1",
  "map_id": "carentan",
  "display_name": "Carentan",
  "date_setting": "1944-06-11",
  "crs": { "epsg": 32630, "origin_e": 626432, "origin_n": 5462016, "size_m": 2048 },
  "height": { "res_m": 1, "samples": 2049, "hmin_m": -1.2, "hmax_m": 45.0, "datum": "IGN69" },
  "far_ring": { "res_m": 8, "samples": 1025, "extent_m": 8192 },
  "anchors": [ { "id": "church_notre_dame", "e": 627709, "n": 5462913, "design": [1277, 897], "confidence_m": 40 } ],
  "similarity": { "dx": 0, "dy": 0, "rot_deg": 0, "applied": false },
  "sources": [ { "id": "ign_lidar_hd", "license": "Etalab-2.0", "verified": "2026-11-17" } ]
}
```

**zones.json**

```json
{
  "schema": "frontline.map.zones/1",
  "lines": [
    { "id": "L1", "name_key": "carentan.line1", "order": 1,
      "points": [
        { "id": "L1a", "name_key": "carentan.ingouf_farm", "origin": "R",
          "pos": [900, 1450], "pos_error_m": 250, "radius_m": 45,
          "tags": ["stone_courtyard", "hedgerow"] }
      ] }
  ],
  "assault_stages": [ { "stage": 1, "primary": ["L1a"], "secondary": ["L1b"] } ],
  "skirmish_areas": [ { "id": "S1", "rect": [900, 500, 1700, 1200] } ],
  "playable_polygon": [[0,0],[2048,0],[2048,2048],[0,2048]],
  "out_of_bounds_buffer_m": 64
}
```

**spawns.json**

```json
{
  "schema": "frontline.map.spawns/1",
  "hq": { "US": { "pos": [250,1750], "radius_m": 40 }, "GER": { "pos": [1850,250], "radius_m": 40 } },
  "garrison_candidates": [ { "side": "US", "pos": [250,1750], "notes": "..." } ],
  "outpost_candidates": [], "airhead_candidates": [], "observation_posts": [],
  "kinds_supported": ["hq","garrison","outpost","airhead","op","boat"]
}
```

**landmarks.json**（每项）

```json
{ "id": "church_notre_dame", "name": "Église Notre-Dame", "kind": "church",
  "pos_e": 627709, "pos_n": 5462913, "design": [1277, 897],
  "fidelity": "A", "state_1944": "damaged", "refs": ["merimee:PA00110352"],
  "asset_ref": "assets/processed/carentan/church_notre_dame.glb" }
```

**hedgerows.geojson** 属性：`height_m`、`mound_width_m`、`gap` (bool)、`vehicle_blocking` (bool)。
**roads.geojson** 属性：`class`、`width_m`、`surface`、`bridge`、`elev_mode`、`vehicle_ok`。
**buildings.geojson** 属性：`style`、`levels`、`roof`、`enterable`、`destructible`、`asset_ref`、`ruin_stage`。
**navmesh**：参数（半径 0.4 m、高度 1.8 m、爬坡 35°、台阶 0.4 m）【待调参】；载具使用独立网格（半径 2.0 m、坡度 25°）。

### 6.3 校验

- `map-pipeline validate`：JSON schema、所有坐标在 0..2048、据点不落入水体与实心建筑、HQ 之间可通行、每个据点至少有 3 条步兵入口、载具路线连通。
- 数据版本号写入 `meta.json`，服务器与客户端启动时校验哈希。

---

## 7. 地图制作任务分解建议

> 供任务编写者参考；`人工`=必须由人类判断/绘制或需要人类核实；`AI`=可由执行者自动完成。

### 7.1 通用任务

| ID | 任务 | 类型 | 说明 |
|---|---|---|---|
| T-LIC | 许可证核实（各数据源原页面截图归档，写入 LICENSES.md） | **人工**（AI 辅助） | 重点：瓦隆 LiDAR、NCAP/NARA 条目、OSM ODbL 策略、Normandy War Guide 扫描件 |
| T-GEO | 锚点定位与相似变换（§0.4）：核对教堂、车站、广场、农场、WN、十字路口位置 | **人工**（AI 辅助脚本） | 每图 0.5–1 天；是后续任务的前提 |
| T-DEM | 取 LiDAR → 重采样 → 16-bit 高度图（+8 m 远景） | AI | GDAL 脚本；需要下载大量块 |
| T-VEC | OSM 初稿提取 + 清洗 + 1944 修正 | AI + **人工** | 道路/水系/建筑；人工对照历史图 |
| T-HEDGE | 树篱描摹 | **人工**（Carentan 重） | 约 1–3 天 |
| T-FLOOD | 洪泛区多边形与地形修正（Carentan） | **人工** | 对照航拍 |
| T-BLD | 建筑足迹与样式标注 | AI + **人工** | 可进入性、重点建筑 |
| T-LAND | 地标建模/采购与摆放 | **人工**（采购 + 摆放） | A2：不自制模型，仅用现成素材 |
| T-SPLAT | 材质权重（splat）生成 | AI（规则）+ **人工**微调 | 坡度、道路、水边规则 |
| T-ZONES | 将 §3.6/§4.6/§5.6 写成 `zones.json`/`spawns.json` | AI | 然后做试玩调整 |
| T-NAV | Recast 烘焙与连通性校验 | AI | |
| T-QA | 视线与性能测试（固定机位截图、Hi-Z/WebGL2 对比） | AI + **人工** | |
| T-BAL | `balance-sim` 平衡迭代 | AI | 调参 |

### 7.2 各图任务量级与特殊点

| 地图 | 特殊点 |
|---|---|
| Carentan | 洪泛修正、树篱描摹是主要人工量；市中心建筑与教堂的采购；M1 先做 1/3 区域（§3.8） |
| Omaha | 海岸线/障碍带布置；WN 工事与障碍的素材采购；需要**潮位**与**WN 位置**史料校对（人工）；登陆艇出生机制 |
| Foy | 瓦隆数据许可证核实（人工）；森林布置；散兵坑布置；雪材质与天气 |

### 7.3 里程碑对应

| 里程碑 | 地图任务 |
|---|---|
| M0 | 单块地形（Carentan 局部 256 m）验证 T-DEM 管线 |
| M1 | Carentan 1/3：T-LIC、T-GEO、T-DEM、T-VEC、T-FLOOD/T-HEDGE（局部）、T-BLD、T-LAND（核心地标）、T-ZONES、T-NAV |
| M2 | Carentan 完整 |
| M4 | Omaha、Foy 全流程 |

---

## 8. Backlog：后续地图资料可得性简评

> 以下仅为**简评**，未做逐项核实；"未核实"表示需要在立项前补做调研。

| 地图 | 高程 | 历史图/航拍 | 难点 | 评价 |
|---|---|---|---|---|
| **Stalingrad（1942）** | 俄罗斯现代 DEM 可用度**未核实**；Copernicus GLO-30 可用（§2.1，国家例外**需核实**）；城区 1942 已被毁，现代建筑与 1942 差异极大 | 德军航拍与苏军地图（德军 Luftbild 在 NARA 有大量副本，**未核实**）；1942 城市平面 **未核实** | 城市重建、苏军阵营（超出 A3 范围：M4 之后）；废墟资产需求巨大 | 资料可得性**中**，制作成本**高** |
| **Kursk（1943）** | 平坦草原，Copernicus GLO-30 足够，LiDAR 无 | 德军航拍 NARA 副本（**未核实**）；苏军地图 **未核实** | 地形平缓，设计上靠反坦克壕、雷区、村庄；需要苏军与坦克战（Panther/T-34/Tiger） | 资料可得性**中**；设计简单、素材（坦克）需求大 |
| 其它（Hürtgen、Remagen、Kasserine 等） | 德国/美国数据 **未核实** | NARA 有大量照片 | — | 暂不评估 |

---

## 9. 附录

### 9.1 与总纲的一致性与异议

1. **异议 1（高度图尺寸）**：总纲写"高度图 1 m 分辨率（2049²，16-bit）"，与本文件一致；但 Omaha/Carentan 起伏小，16-bit 量化浪费不大；Foy 起伏更大（Ardennes 约 30–60 m 高差）也够用。无异议，仅注。
2. **异议 2（OSM 许可）**：总纲 A1（免费、捐赠、无付费）与 ODbL 兼容，但**矢量数据层需 ODbL 公开**（§2.5）。建议总控确认是否接受把 `/data/maps/*/roads|hedgerows|buildings.geojson` 以 ODbL 公开；否则改为"OSM 仅作参考、全部手绘"，工时 +30–50%。
3. **异议 3（GSGS 4347 的任务书描述）**：任务书称 GSGS 4347 为 1:25,000；本次**未能核实**，且 GSGS 4250 为 1:50,000（已核实）。文档按"未核实"处理。
4. **异议 4（IWM 与"捐赠"）**：IWM 的非商业许可与"接受捐赠"的关系未经法律确认，本文件保守地**不嵌入任何 IWM 影像**。

### 9.2 核实日志（2026-11-17，摘要）

| 对象 | 方式 | 结论 |
|---|---|---|
| IGN LiDAR HD、RGE ALTI（geoservices.ign.fr） | 抓取 | 301 跳转至 cartes.gouv.fr；门户页抓取仅得空壳；搜索摘要提供许可证（Etalab 2.0） |
| IGN Remonter le temps | 抓取 + 搜索 | 页面存在；单张影像再利用条款**未核实** |
| NCAP | 抓取（301→403）+ 搜索 | 许可页存在（搜索结果）；费用**未核实** |
| 瓦隆 LiDAR | 抓取 404（spw.wallonie.be/lidar）+ 搜索 | 服务存在；官方许可证**未核实** |
| Copernicus DEM GLO-30 | 搜索 | 许可 PDF 与公告存在 |
| McMaster / UT / LoC | 抓取（McMaster 被拒）+ 搜索 | McMaster 页面存在但访问被拒；UT 条目存在；LoC finding aid 编号来自搜索 |
| ibiblio HyperWar Green Books | 抓取 | 存在；未含版权声明 |
| Wikipedia（Carentan 战役、Carentan-les-Marais、Vierville、Saint-Laurent、Colleville、Foy） | 抓取 | 存在；坐标为质心/度分精度 |
| Cirkwi / Mérimée（教堂） | 搜索 | 目录存在；坐标取自 Cirkwi |
| OSM ODbL 解读 | 搜索 | 社区 wiki 解读，**非法律意见** |
| IWM 非商业许可 | 搜索 | 存在；商业需付费 |
| NARA 照片 | 搜索 | 存在；条目级核查 |
| 本文件坐标的 UTM/WGS84 换算 | `pyproj` 实际运行 | 已执行 |

### 9.3 对其他文档作者的关键约定

- 地图 ID：`carentan`、`omaha_dog_green`、`foy`；数据目录 `/data/maps/{map_id}/`（§6）。
- 设计框架：2048 m 轴对齐方形，西南角原点；中心取 64 m 倍数；1:1 不压缩。
- 战线 ID：`L1`–`L5`，据点 ID：`L{n}{a|b|c}`；Assault 段 `stage 1–5`；Skirmish 子区 `S1–S3`。
- 出生点类型：`hq`、`garrison`、`outpost`、`airhead`、`op`、`boat`（Omaha 特有）。
- 据点强点半径：35–55 m 初值【待调参】。
- 阵营标识：`US`、`GER`。
