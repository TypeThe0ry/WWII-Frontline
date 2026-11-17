# M2-001 地图：Carentan 全图高度图与矢量

## 元数据
- 里程碑：M2
- 工作流：maps
- 规模：L
- 依赖：M1-031, M1-032
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §3.2; MAPS §1.2; MAPS §1.6; MAPS §3.5; MAPS §1.3; MAPS §6.1–§6.3

## 背景
M1 只做了 Carentan 的 1/3 区域（设计框架 x 380–1560、y 520–1700，MAPS §3.8）。M2 要整张图：可玩区 2048 m × 2048 m（UTM 30N，EPSG:32630，SW 角 E 626432 / N 5462016，NE 角 E 628480 / N 5464064，MAPS §1.3）。本任务把 M1 的脚本与数据扩展到全图，是 M2-002（1944 修正）、M2-003（建筑）、M2-004（关卡标注）、M2-005（区块烘焙）的输入。
关键数字（MAPS §1.2 / ARCH §9.1）：近景高度图 2049×2049 样本、16-bit、1 m；远景环 1025×1025、8 m、覆盖 8192 m（中心 ± 4096 m，中心 E 627456 / N 5463040）；可玩区外另生成 256 m 缓冲；重采样一律 `gdalwarp -r average`；量化 `h_u16 = round((h - hmin)/(hmax - hmin) * 65535)`，`hmin/hmax` 写入 `meta.json`；游戏高度 = 高程 − 基准偏移，使最低水面 ≈ 0 m。OSM 衍生层按 ODbL 公开并署名（BRIEF R9；人类确认见 NEEDS_HUMAN H-004）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/dem.py` —— 在 M1-031 脚本上扩展：全图拼接、0.5 m→1 m average 重采样、256 m 缓冲、8 m 远景环；输入清单哈希写入 lock 文件
- `tools/map-pipeline/vectors.py` —— 在 M1-032 脚本上扩展到全图：OSM → `roads/water/hedgerows/forests/buildings.geojson` 初稿、属性规范化、坐标转本地米（0..2048）
- `tools/map-pipeline/schemas/*.schema.json` —— 各 geojson 的 JSON Schema（属性见下）
- `data/maps/carentan/meta.json` —— 更新 `height`（hmin/hmax/datum）、`far_ring`、`sources[]`、`anchors[]`（沿用 M1-030 的锚点表与相似变换，不重做）
- `data/maps/carentan/{roads,water,hedgerows,forests,buildings}.geojson`、`LICENSE_NOTICE.md`（ODbL 声明与 © OpenStreetMap contributors）
- `data/maps/carentan/hashes.json` —— 高度图等大文件的 sha256 与尺寸；大文件本体（`heightmap_1m.r16`、`heightmap_far_8m.r16`）放 `assets/processed/maps/carentan/`，若 M1-031 已另定位置则沿用其做法
### 要实现的行为
1. 全图高度图：输出 `heightmap_1m.r16`（小端 uint16，2049×2049，字节数必须为 8 396 802）与 `heightmap_far_8m.r16`（1025×1025，2 101 250 字节）。M1 区域数据不得被改动（输入相同则逐字节相同）。
2. DEM 缺失处理：缺块时脚本退出并列出缺失块文件名，不得用插值/噪声补洞。
3. 矢量属性（MAPS §1.6/§6.2）：roads `class(paved|dirt|path|rail|causeway)`、`width_m`、`surface`、`bridge`、`elev_mode(terrain|embankment|bridge)`、`vehicle_ok`；water `kind`、`depth_m`、`walkable_depth`；hedgerows `height_m(1.5–2.5)`、`mound_width_m`、`gap`、`vehicle_blocking`（初稿由 OSM 给出，M2-002 补全人工描摹）；forests 密度/树种。每个要素带 `source: osm|hand`，OSM 与手绘分开存放以便 ODbL 隔离。
4. 桥梁标注：N13 堤道四座石桥（框内约 2 座，MAPS §3.4）及所有道路×水体相交处，必须标 `bridge=true` + `elev_mode=bridge` 或显式 `ford=true`；道路与水体相交而二者皆无者视为错误。
5. 1944 不存在的现代要素本任务**不删**（见不做什么），仅在属性中保留 `modern_suspect` 标记供 M2-002 使用。
### 接口与数据
- CLI：`map-pipeline dem --map carentan --full`、`map-pipeline vectors --map carentan --full`、`map-pipeline validate --map carentan --stage vectors`
- `meta.json` 字段沿用 MAPS §6.2 的 `frontline.map.meta/1`。

## 不做什么
- 1944 修正（树篱描摹、洪泛、去现代构筑物、terrain_edits）→ M2-002。
- 建筑样式/可进入性/地标 → M2-003；zones/spawns → M2-004；区块切片/碰撞/navmesh → M2-005。
- 不引入 IWM 影像（BRIEF R9）。

## 验收标准
1. 至少 20 个控制点（含 M1-030 的锚点，空间上覆盖全图四象限）的高度图高程与参考高程误差 ≤ 1 m（`map-pipeline validate --stage dem` 输出表）。
2. 同一输入清单运行两次，`heightmap_1m.r16`、`heightmap_far_8m.r16` 与全部 geojson 的 sha256 完全一致（`hashes.json` 比对脚本退出码 0）。
3. 两个 `.r16` 的字节数分别为 8 396 802 与 2 101 250；`meta.json` 含 hmin/hmax/datum/偏移，且最低水面高度在 ±0.3 m 内（抽样 `water.geojson` 内 50 点）。
4. 全部 geojson 通过 schema；所有坐标在 [0, 2048]；`source` 字段 100% 存在。
5. 道路×水体相交未标 bridge/ford 的数量 = 0。
6. `LICENSE_NOTICE.md` 存在，`meta.json.sources[]` 每项含 `license` 与 `verified` 日期。

## 测试要求
- 单元测试：`pytest tools/map-pipeline/tests/test_dem.py`（量化往返误差 < 1.5 mm；索引与世界坐标互转）、`test_vectors.py`（schema、坐标裁剪、相交检测）。
- 集成：用 M1 区域子集跑全图脚本并与 M1 产物逐字节比对。
- 性能基准：记录全图脚本耗时与峰值内存到 STATUS（无阈值）。
- 回归：`hashes.json` 作为后续任务的基线。

## 性能预算
无（离线工具）。运行时体积预算见 ARCH §9.1：全图高度图原始 ≈ 8.4 MB，远景环 ≈ 2.1 MB。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工输入：DEM 下载（IGN LiDAR HD / RGE ALTI，需人下载进 `assets/incoming/maps/carentan/`，许可证核实见 NEEDS_HUMAN H-052）；锚点复核 H-051；ODbL 接受 H-004。DEM 缺失则本任务 `blocked`，不得合成数据。
- 现代 LiDAR 反映 2020s 地面（堤道、运河可能被改造），本任务只忠实输出，不做 1944 判断。
- 车站/广场/农场位置在 MAPS §3.4 标"未核实"，以 M1-030 的锚点结果为准。
