# M1-032 地图：OSM 矢量提取与清洗

## 元数据
- 里程碑：M1
- 工作流：maps
- 规模：M
- 前置依赖：M1-030
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §1.6; MAPS §2.5; MAPS §6.1; MAPS §6.2; MAPS §3.5; BRIEF R9

## 背景
从 OpenStreetMap 提取 Carentan 可玩区的现代矢量作为**初稿**（MAPS §7.1 的 T-VEC 前半），后续由 M1-033 对照历史资料做 1944 年修正。许可证：**ODbL 1.0**，署名 "© OpenStreetMap contributors"；**R9 默认执行**：接受把衍生矢量层（`roads` / `hedgerows` / `buildings`）按 ODbL 公开，游戏内署名；`/data/maps/{id}/` 内矢量层声明 ODbL（独立 NOTICE），代码与素材不受影响（Collective Database 与 Produced Work 分离，MAPS §2.5）。该事项仍待人最终确认（H-004）；入库前需法律确认。
输出文件（MAPS §6.1/§6.2），**坐标全为本地米坐标（西南角原点，0..2048）**：
- `roads.geojson`：属性 `class`（paved/dirt/path/rail/causeway）、`width_m`、`surface`、`bridge`（bool）、`elev_mode`（terrain|embankment|bridge）、`vehicle_ok`；
- `water.geojson`：`kind`、`depth_m`、`walkable_depth`（决定可涉水）；
- `hedgerows.geojson`（OSM `barrier=hedge` 等初稿）：`height_m`（1.5–2.5）、`mound_width_m`、`gap`（bool）、`vehicle_blocking`（bool）；
- `buildings.geojson`：足迹；属性 `style`、`levels`、`roof`、`enterable`、`destructible`、`asset_ref`、`ruin_stage`（默认值，样式/可进入性由 M1-034 标注）；
- `forests.geojson`（森林/植被多边形：密度、树种）。
属性规范化原则：`vehicle_ok` = 路宽 ≥ 3.0 m 且 class ∈ {paved,dirt,causeway} 且非 `bridge` 限重（未知则 true）；树篱 `vehicle_blocking` 默认 true（高 ≥ 1.8 m 且无缺口 ≥ 4 m，MAPS §3.6）；缺失属性给默认并标 `_derived:true`。
裁剪范围：可玩区（M1 重点为切片矩形 x 380–1560、y 520–1700，MAPS §3.8，但提取整个 2048 m 区以便 M2 复用，矩形外不进入 M1 切片数据）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/map_pipeline/osm.py` —— 提取：优先读 `assets/incoming/maps/carentan/osm/*.osm.pbf`（Geofabrik Normandie 提取，人类放置）或离线 Overpass 导出（`.osm/.json`）；用 GDAL/OGR OSM 驱动（`ogr2ogr`）解析，不依赖运行期网络
- `tools/map-pipeline/map_pipeline/osm_rules.py` —— 标签 → 游戏属性规则表（`highway=*`、`railway=*`、`waterway=*`、`natural=wetland|water`、`barrier=hedge`、`building=*`、`landuse=forest|orchard`）
- `tools/map-pipeline/map_pipeline/clean.py` —— 拓扑清洗：自相交修复、悬空线延伸/删除（< 2 m）、重复要素、最小面积 4 m²、坐标截断到 1 cm
- `tools/map-pipeline/map_pipeline/modern_filter.py` —— 初步现代剔除（`highway=motorway/trunk`（N13 保留为 `causeway` 候选，另列清单）、`landuse=retail`、`shop=*`…）；**仅初稿**，细修见 M1-033
- `data/maps/carentan/{roads,water,hedgerows,buildings,forests}.geojson`
- `data/maps/carentan/ATTRIBUTION_OSM.txt` 与 `LICENSE_NOTICE.md`（追加 ODbL 段落，M1-030 已创建基础版）
- `tools/map-pipeline/schemas/{roads,water,hedgerows,buildings,forests}.schema.json`
- `tools/map-pipeline/tests/test_osm_*.py`（含小型 `.osm` 夹具）
### 要实现的行为
1. 输入哈希记录：`meta.json.sources[].inputs[]`（文件名、sha256、下载日期）；OSM 数据时间戳写入 `osm_timestamp`。
2. 坐标：WGS84 → UTM30N → 本地米（含 M1-030 相似变换，若 `applied`）；裁剪到 [0,2048]²；越界要素被截断，跨界要素保留裁剪后部分。
3. 道路：`highway`→class 映射（motorway/trunk/primary/secondary→paved；tertiary/unclassified/residential→paved；track→dirt；path/footway→path；railway→rail；堤道候选标 `causeway`）；`width_m` 取 OSM `width` 否则按 class 默认（paved 6 / dirt 4 / path 1.5 / rail 5）；桥梁 `bridge=yes` → `bridge:true`、`elev_mode:bridge`。
4. 水系：`waterway=river/canal` 线 → 宽度/`depth_m` 默认；`natural=wetland/water` 面 → `kind`，`walkable_depth` 默认 0.5；**洪泛区不在此处生成**（M1-033）。
5. 树篱：`barrier=hedge`、`natural=tree_row` 的线，高度默认 2.0 m、`mound_width_m` 1.0；缺口：线上无要素时 `gap:false`。
6. 建筑：`building=*` 面，`levels` 取 `building:levels` 默认 1–2；`enterable:false`、`style:"unknown"`（M1-034 标注）。
7. 清洗后运行 schema 校验；同输入输出字节一致（排序要素、固定浮点格式）。
8. 报告：`reports/m1-osm-summary.json`（各类要素数、道路总长、树篱总长、建筑数、剔除的现代要素数、`_derived` 比例）。
### 接口与数据
```json
{"type":"Feature","properties":{"class":"paved","width_m":6.0,"surface":"asphalt","bridge":false,"elev_mode":"terrain","vehicle_ok":true,"_osm_id":"way/123","_derived":["width_m"]},"geometry":{"type":"LineString","coordinates":[[412.31,1530.02],[…]]}}
```

## 不做什么
- 1944 年修正（删除现代建筑/道路、恢复堤道/运河、洪泛多边形、树篱人工描摹）：M1-033；
- 建筑样式/可进入性/地标：M1-034；
- 植被分布/splat：M1-035；
- 历史影像/GSGS 描摹（只作参考，不入库）；
- OSM 在线下载自动化（须人类提供提取文件或批准网络访问）。

## 验收标准
1. 全部 geojson 通过各自 JSON Schema（`map-pipeline validate-vectors`，0 错误）。
2. **坐标全在 0..2048**：脚本遍历所有顶点断言 `0 ≤ x,y ≤ 2048`（违规数 0）；单位为米，抽样 10 个道路交叉口与 OSM 原始 UTM 坐标反算误差 < 0.1 m。
3. **署名文件存在**：`ATTRIBUTION_OSM.txt` 含 "© OpenStreetMap contributors"；`LICENSE_NOTICE.md` 含 ODbL 1.0 声明（grep 断言）。
4. 要素统计：`roads` 总长 ≥ 1 km、`buildings` ≥ 50 栋（切片矩形内，真实数据到位时；夹具测试不受此限，单列"数据到位后验收"）；`vehicle_ok` 为 true 的道路构成与 N13 堤道相连的连通分量（`map-pipeline check-roads` 输出连通性）。
5. 清洗：自相交数 0、重复要素数 0、面积 < 4 m² 要素 0（单测 + 真实数据报告）。
6. 同输入两次运行各 geojson 的 sha256 一致。
7. `_derived` 记录：所有默认补全的属性都有 `_derived` 标注，比例写入摘要（用于评估人工修正量）。
8. OSM 提取文件缺失时任务 `blocked`，退出码 3、提示清晰；不得改用其它来源或自制数据。

## 测试要求
- 单元测试：标签→属性规则表（每条规则 ≥ 1 例）、清洗算法、坐标变换、截断。
- 集成：夹具 `.osm`（含道路/桥/运河/树篱/建筑各数条）全流程。
- 性能基准：Geofabrik 省级 PBF 裁剪提取 ≤ 5 min（记录）。
- 回归：`m1-osm-summary.json` 入库作为后续对比。

## 性能预算
- 构建期工具；输出体积记录供 M1-035（`buildings.geojson` 随区块切片进入 props 表）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **人工/授权**：OSM 提取文件需由人类放置或批准联网下载；ODbL 公开策略待 H-004 最终确认，**确认前不得把 OSM 衍生层公开发布**（仓库私有时可入库）。
- OSM 是现代数据，与 1944 年差异大（MAPS §1.5）：本任务产出仅"初稿"，不要把它当史实。
- 若 R9 被人类否决（备选方案：OSM 只作视觉参考、从历史图重新描摹，工时 +30–50%），本任务产出须改为不入库的参考层，写 PLAN_ISSUES。
- 树篱在 OSM 中覆盖不全（Carentan 西南 bocage 总长 > 15 km，MAPS §3.7），OSM 初稿远低于目标，缺口由 M1-033 人工补。
