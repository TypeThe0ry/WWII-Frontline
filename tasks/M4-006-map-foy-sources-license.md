# M4-006 地图：Foy 数据源许可证与锚点对账

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：M
- 依赖：M1-030（Carentan 的 T-LIC/T-GEO 流程与脚本）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §5.3; MAPS §2; MAPS §7.2; MAPS §5.2; MAPS §1.3; MAPS §0.4; NEEDS_HUMAN H-072, H-078

## 背景
Foy（`map_id = foy`，1945-01，Bois Jacques 至 Foy 村）的唯一高程来源是瓦隆 LiDAR/DTM（0.5 m MNT，ArcGIS REST：`geoservices.wallonie.be/arcgis/rest/services/RELIEF/WALLONIE_MNT_2021_2022/MapServer`），**官方许可证未核实**（MAPS §2.1）。本任务的核心是"许可证准入 + 锚点对账"，决定 M4-007 能否使用瓦隆数据。
关键约束：
- 坐标系：源 Belgian Lambert 2008（EPSG:3812），游戏工作 CRS 为 ETRS89/UTM 31N（EPSG:25831）；Foy 约 5.75°E，距带中央 2.75°，比例畸变 ≈ 1.0003–1.0004，可忽略但**须在 `meta.json` 注明**（MAPS §1.3）。
- 可玩区暂定 SW=(695488, 5546112)，NE=(697536, 5548160)；中心 50.04404°N, 5.74470°E **仅度分精度，±1 km**，必须以瓦隆 LiDAR + OSM 重新锚定（MAPS §5.2）。
- 锚点：A1 Foy 村十字路口（OSM）；A2 Bois Jacques 北缘/林缘（纪念碑为现代物，不入游戏）。设计相对关系（村在北半部、森林在南/西南、Bastogne 在南、Noville 在北）是**推断，需验证**。
- 数据备选：Copernicus GLO-30 + 手工微地形（质量显著下降，仅应急；MAPS §5.7）。
- 德军番号"未核实"，不得写入任何文案（H-078）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/maps/foy/sources.yaml` —— 数据源清单（瓦隆 LiDAR/MNT、瓦隆 PICC、GLO-30、OSM、GSGS 4336/AMS 比利时图），含 `license`、`status`、`verified_date`、`attribution`。
- `tools/map-pipeline/maps/foy/anchors.json` —— 锚点；`tools/map-pipeline/map_pipeline/geo/fit_similarity.py` 复用 M4-001 的实现。
- `data/maps/foy/meta.json`、`data/maps/foy/LICENSE_NOTICE.md`（含署名文案草稿"SPW"/ODbL，待人确认）。
- `licenses/texts/foy_*/` —— 原页许可证快照/回函。
### 要实现的行为
1. 对每个源记录状态；瓦隆 LiDAR 状态在收到书面确认（H-072）前一律 `PENDING`，`sources.yaml` 写 `blocked_by: H-072`。
2. 许可证为 PENDING 时：脚本**拒绝**把瓦隆衍生数据写入 `data/maps/foy/`（`map-pipeline` 的 license gate，退出码非 0），仅允许 GLO-30 占位地形（`placeholder: true`）；PENDING 数据不进主分支（D-LIC）。
3. 锚点 ≥ 6 个（INDEX 未给数量，沿用 Omaha 的稳健性要求取 ≥ 6）：Foy 十字路口、N30 与支路交点 ×2、林缘拐点 ×2、教堂/小礼拜堂（位置未核实则标 `confidence_m ≥ 100`）。
4. 拟合相似变换并写入 `meta.json.similarity`；N30 走向、森林位置与设计假设不一致时，在报告中标出并写 `docs/PLAN_ISSUES.md`（不擅自改拓扑）。
5. 在 `docs/NEEDS_HUMAN.md` 末尾追加条目引用 H-072、H-078（只追加）。
### 接口与数据
- `meta.json`：`map_id="foy"`、`date_setting="1945-01-13"`（日期细节需核对，MAPS §5.1，写 `date_verified=false`）、`crs.epsg=25831`、`height.datum`（待定）、`sources[]`、`anchors[]`、`similarity`。

## 不做什么
- 不生成高度图与矢量（M4-007）。
- 不布置森林/散兵坑（M4-008）、不写 zones（M4-009）。
- 不在文案中使用德军番号或未核实的日期；不自行修改 LICENSES.md 状态。

## 验收标准
1. `sources.yaml` 覆盖 MAPS §5.3 全部数据源；每条有明确 `status ∈ {OK, PENDING}`；`map-pipeline validate-sources foy` 退出码 0。
2. 许可证门禁测试：把瓦隆源状态设为 PENDING 时，`map-pipeline dem foy` 以非零退出且不写出任何瓦隆衍生文件（`pytest test_license_gate_foy.py` 通过）。
3. `anchors.json` 锚点数 ≥ 6，每个有 `confidence_m`；拟合报告输出 `residual_rms_m`，并明确 `applied` 判定（≤150 m 平移 / 否则 ≤10° 旋转）。
4. `meta.json` 通过 `frontline.map.meta/1` schema 校验；含 UTM31N 比例畸变说明字段 `notes.scale_factor_note`。
5. 重复运行输出逐字节一致。

## 测试要求
- 单元测试：许可证门禁（OK/PENDING/缺失三态各 1 个用例）；拟合脚本沿用 M4-001 的单测并加 Foy 合成用例。
- 集成测试：`pytest tools/map-pipeline/tests/test_foy_meta.py`。
- 性能基准：无。
- 回归：无（首次）。

## 性能预算
无（离线工具任务）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖 NEEDS_HUMAN H-072（瓦隆 LiDAR 许可证书面确认）与 H-078（锚点校验）。H-072 默认假设：无默认；未确认前 Foy 只用 GLO-30 占位、不入库瓦隆数据。此时任务状态至多 `partial`，并在 STATUS 的 blocked/partial 清单登记。
- 联系邮箱 `lidar@spw.wallonie.be` 来自教程摘要，**未核实**；邮件由人类发送。
- 第三方 wiki 称"开放、须署明 SPW"并非官方依据，不得据此入库。
