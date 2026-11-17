# M1-030 地图：Carentan 数据源许可证与锚点对账

## 元数据
- 里程碑：M1
- 工作流：maps
- 规模：M
- 前置依赖：M0-009
- 素材需求：无（灰盒即可）；素材状态：n.a.（地图数据源登记属素材台账范畴，见 LICENSES.md）
- 引用文档：MAPS §2.1; MAPS §2.5; MAPS §2.6; MAPS §0.4; MAPS §1.3; MAPS §3.4; MAPS §6.2; MAPS §7.1; LICENSES.md §1; LICENSES.md §3

## 背景
这是 Carentan 地图关键路径的第一项（MILESTONES §2.9 关键路径起点），对应 MAPS §7.1 的 **T-LIC**（数据源许可证核实）与 **T-GEO**（锚点定位与相似变换）。范围含 `[人工]`：许可证核实必须由人类到官方原页面确认并截图归档（H-052），锚点位置由人类对照 OSM/IGN/GSGS 校验（H-051）；执行者负责**脚本与存档结构**，不得编造许可证结论、不得把未核实的数据源写成 `OK`（BRIEF R8/§6）。
要点：
- 数据源（MAPS §2.1/§2.5）：**IGN LiDAR HD**（Etalab 2.0，须署名，部分核实）、**IGN RGE ALTI 1 m**（备选，Etalab 2.0）、**Copernicus DEM GLO-30**（远景环外圈兜底；许可证原文与国家例外清单须读原文）、**OpenStreetMap**（ODbL 1.0，署名 "© OpenStreetMap contributors"）。历史航拍/老图（IGN Remonter le temps、NARA、GSGS、IWM）**只作描摹参考，不入库**；**IWM 影像不嵌入**（R9）。
- **OSM/ODbL（R9，默认执行，待人最终确认 H-004）**：接受把衍生矢量层（`roads/hedgerows/buildings`）按 ODbL 公开，游戏内署名；`/data/maps/{id}/` 内矢量数据层声明 ODbL（独立 NOTICE），代码与素材不受影响。
- **锚点—相似变换（MAPS §0.4）**：每图列锚点；锚点真实 UTM 与设计框架偏差 ≤ 150 m → 直接平移；更大则拟合"平移 + 小角度旋转（≤ 10°）"，作用于所有"设计"位置（据点、出生点），**真实地标不变换**；变换后落入水体/实心建筑/坡度 > 35° 的设计据点吸附到最近可站立开阔地（≤ 60 m）并在 notes 记录；不得改变拓扑。
- 暂定坐标（MAPS §1.3，EPSG:32630）：SW E 626432 / N 5462016，NE E 628480 / N 5464064；设计框架：原点 SW，x 东、y 北、0–2048 m。已知锚点候选：Notre-Dame（设计 (1277,897)，±40 m，UTM ≈ E 627709 N 5462913）、Place de la République (1380,990) ±120、火车站 (1500,520) ±250、Ingouf 农场 (900,1450) ±250、N13 石桥、运河闸口(1200,1350) ±120、Augustine 修道院（位置**未核实**）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/map_pipeline/licenses.py` —— 读取 `data/maps/carentan/meta.json` 的 `sources[]`，对照 `LICENSES.md` §3 台账：每个 source 必须有台账行与 `licenses/texts/` 存档；输出缺口报告 `reports/m1-map-licenses.json`
- `tools/map-pipeline/map_pipeline/anchors.py` —— `fit_similarity(anchors) -> {dx,dy,rot_deg,applied}`（最小二乘，旋转限 ±10°）、`apply(points)`、残差报告
- `tools/map-pipeline/map_pipeline/geo.py` —— UTM30N(EPSG:25830/32630) ↔ 本地米坐标；`pyproj`
- `data/maps/carentan/meta.json` —— 按 MAPS §6.2 schema：`crs`、`height`（占位）、`far_ring`、`anchors[]`（**≥ 8 个**，每项 `id,e,n,design,confidence_m,source`）、`similarity`、`sources[]`
- `data/maps/carentan/LICENSE_NOTICE.md` —— 本图数据署名与许可证（含 ODbL 声明、Etalab 2.0 署名）
- `reports/m1-map-license-rows.md` —— **给人类粘贴到 LICENSES.md §3 的草稿行**（执行者不直接编辑 LICENSES.md，README §1）
- `tools/map-pipeline/tests/test_anchors.py`、`test_licenses.py`
### 要实现的行为
1. 许可证检查：对每个数据源输出 `OK/PENDING/MISSING`；`MISSING`（台账无行）与 `PENDING` 均使 CI 的 `map-pipeline validate-sources` 返回非 0（该检查只在改 `data/maps/**` 时运行）。
2. 草稿行生成：按 LICENSES §2 字段模板，预填 `source_channel/url/license_name_version`（取自 MAPS §2.1/§2.5 已有信息）、`chk_*` 一律填 `PENDING`、`status=PENDING`、`notes` 写"待 H-052 核实"，**不得写 OK**。
3. 锚点：从 OSM（M1-032 的原始提取或手工坐标）与 MAPS 暂定值合成锚点表；每个锚点记录 `confidence_m`（教堂 40 m、广场 120 m、车站 250 m…，核实后由人类改小）；`fit_similarity` 在偏差 ≤ 150 m 时仅输出平移，> 150 m 才拟合旋转，旋转 > 10° 报错。
4. 变换应用：`apply_to_design(zones/spawns)` 提供给 M1-034；保留变换前后值以便审计；`similarity.applied` 默认 `false`，人类校验锚点（H-051）后置 `true`。
5. 吸附：设计据点落入水体/实心建筑/坡度 > 35° 时吸附 ≤ 60 m（依赖 M1-031/032 数据；本任务提供函数与夹具单测）。
6. 署名：生成 `credits` 条目草稿（`credit_text`）：OSM、IGN LiDAR HD/RGE ALTI（Etalab 2.0）。
7. 记录核实日期：`sources[].verified` 只有在存在 `licenses/texts/{id}.*` 存档且人类标记后才写日期，否则 `null`。
### 接口与数据
```json
{ "anchors":[ {"id":"church_notre_dame","e":627709,"n":5462913,"design":[1277,897],"confidence_m":40,"source":"cirkwi+merimee"} ],
  "similarity":{"dx":0,"dy":0,"rot_deg":0,"applied":false},
  "sources":[{"id":"ign_lidar_hd","license":"Etalab-2.0","verified":null,"archive":"licenses/texts/ign_lidar_hd.pdf"}] }
```

## 不做什么
- 高度图生成：M1-031；OSM 矢量提取：M1-032；
- 实际登记 LICENSES.md 与采购：人类（H-052、H-004）；执行者只读台账；
- 1944 修正与树篱描摹：M1-033；
- Omaha/Foy 的数据源（瓦隆 LiDAR 许可证 H-072）：M4；
- 法律意见：不做；诸"未核实"项原样保留。

## 验收标准
1. `meta.json` 通过 JSON Schema（MAPS §6.2 schema，`map-pipeline validate`）；每个 `sources[]` 项在 `reports/m1-map-licenses.json` 里有状态且**无任何伪 OK**（脚本断言：状态为 OK 的项必须在 LICENSES.md 有 `status=OK` 行且 `licenses/texts` 文件存在）。
2. 锚点 **≥ 8 个**，每个都有 `confidence_m`（>0）与 `source`；其中 `confidence_m ≤ 150` 的 ≥ 3 个（用于判定平移/旋转）；单测 `test_anchor_count`。
3. 相似变换单测：夹具偏差 100 m（纯平移）→ `rot_deg==0`；夹具偏差 220 m 且含 6° 旋转 → 拟合 `rot_deg ∈ [5.5, 6.5]`、残差 RMS ≤ 20 m；旋转 12° → 报错（3 例）。
4. 吸附单测：落入水体的夹具据点被吸附到 ≤ 60 m 内的陆地点，`notes` 记录；吸附距离 > 60 m 报错。
5. 每个数据源有"许可证快照与状态"：存在 `licenses/texts/` 存档文件的 100% 有 `verified` 日期；缺存档者状态 `PENDING` 并列入 `NEEDS_HUMAN` 引用（H-052）。
6. `data/maps/carentan/LICENSE_NOTICE.md` 含 ODbL 声明与署名文本 "© OpenStreetMap contributors"（grep 断言）。
7. 同输入两次运行 `meta.json` 哈希一致（确定性）。

## 测试要求
- 单元测试：`fit_similarity`、`apply`、坐标转换往返（UTM↔本地，误差 < 1 cm）、`licenses` 检查矩阵。
- 集成：对仓库当前 LICENSES.md（空模板）运行，所有数据源应为 `MISSING`/`PENDING` 且 CI 命令返回非 0——此状态在人类登记前**应被允许**，故该作业仅在存在 `data/maps/carentan/meta.json` 且标记 `m1_sources_checked:true` 后才强制（细化，写 PLAN_ISSUES）。
- 性能基准：无。
- 回归：锚点表作为数据快照入库。

## 性能预算
- 无。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **人工项**：H-052（官方页面核实 + 截图归档）、H-051（锚点坐标人工校验）、H-004（OSM/ODbL 最终确认）。人类输入缺失时：脚本与夹具测试可完成，状态记 `partial`，所缺条目写入 STATUS blocked 清单与 `docs/NEEDS_HUMAN.md`（README §6）。
- 注意 MAPS 的 UTM 四角为"设计框架暂定值"，锚定后须复算；执行者不得自行改变 MAPS 的章节结论，复算结果写 `meta.json` 并在 PLAN_ISSUES 登记。
- MAPS §2.1 的 IGN 门户页面抓取为空壳、许可证来自搜索摘要，**未核实**；不要在 `LICENSE_NOTICE.md` 里写成已核实。
- INDEX 验收要求锚点 ≥ 8，而 MAPS §0.4 建议每图 2–4 个；本任务取 INDEX（更严）。
