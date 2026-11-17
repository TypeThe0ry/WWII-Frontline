# M4-001 地图：Omaha 数据源、许可证与锚点对账

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：M
- 前置依赖：M1-030
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §4.3; MAPS §2; MAPS §0.4; MAPS §1.3; MAPS §6.2; NEEDS_HUMAN H-052, H-077, H-078

## 背景
Omaha Beach（`map_id = omaha_dog_green`，Dog Green/Vierville 至 Les Moulins，1944-06-06）是 M4 的第一张新图。与 Carentan 一样，先做"数据源准入 + 设计框架锚定"，后续 M4-002（高度图/矢量）、M4-003（障碍/WN）、M4-004（关卡标注）都依赖本任务产出的 `meta.json` 与相似变换。
关键约束：
- 可玩区为 UTM 30N（EPSG:32630）轴对齐 2048 m 正方形，SW=(652160, 5469952)，NE=(654208, 5472000)，暂定值（MAPS §1.3）；本地坐标原点在 SW，x 东、y 北。
- 设计位置与真实位置通过**锚点相似变换**对账（MAPS §0.4）：偏差 ≤ 150 m 仅平移；更大则平移 + 旋转（≤ 10°）；真实地标不变换；变换后据点吸附 ≤ 60 m；拓扑不得改变。
- 数据源：IGN LiDAR HD（Etalab 2.0，门户页抓取为空壳，须人工原页核实）；RGE ALTI 1 m 备选；Copernicus GLO-30 作远景环外圈；OSM（ODbL，R9 默认接受并公开衍生矢量层）。历史航拍/GSGS 图**只作描摹参考，不入库**；IWM 影像不嵌入（MAPS §2.2、§2.5）。
- 海岸线走向暂定 WNW—ESE 且框内海面比例可能较大，必须用 OSM `natural=coastline` 与史料点位重新锚定（MAPS §4.2 注）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/maps/omaha_dog_green/sources.yaml` —— 数据源清单：id、URL、许可证、核实状态、取数日期、输入文件 sha256。
- `tools/map-pipeline/maps/omaha_dog_green/anchors.json` —— 锚点（设计坐标 + 真实 UTM + `confidence_m`）。
- `tools/map-pipeline/map_pipeline/geo/fit_similarity.py` —— 复用/扩展 M1-030 的拟合脚本（最小二乘 平移+旋转，输出残差）。
- `data/maps/omaha_dog_green/meta.json` —— 按 MAPS §6.2 schema（`crs`、`height`、`far_ring`、`anchors`、`similarity`、`sources`）。
- `data/maps/omaha_dog_green/LICENSE_NOTICE.md` —— 署名与 ODbL 声明草稿。
- `licenses/texts/omaha_*/` —— 各数据源原页许可证快照（由人工放入或执行者按 URL 抓取后存档）。
### 要实现的行为
1. 对 MAPS §4.3 / §2.1–2.5 列出的每个数据源写一条 `sources.yaml` 记录；状态用 `OK/PENDING`，未能原页核实的写 PENDING 并说明。**PENDING 源的数据不得进入主分支**。
2. 候选锚点 ≥ 8 个：Vierville-sur-Mer 教堂、Saint-Laurent-sur-Mer 教堂/质心、D-1 出口路口（D517）、D-3 出口路口、悬崖脚下海堤端点、OSM 海岸线上的 2 个明显拐点、2 个内陆十字路口。每个给 OSM 坐标与置信度；WN 点位**不作锚点**（史料矛盾，MAPS §4.4）。
3. 运行拟合：输出 `dx, dy, rot_deg, residual_rms_m`；按 §0.4 规则判定是否 `applied`；写入 `meta.json.similarity`。
4. 统计设计框架内海面占比并写入 `meta.json.notes`，供 M4-002 决定海岸高度图范围。
5. 生成"待人工核实清单"，在 `docs/NEEDS_HUMAN.md` 末尾追加条目（只追加）：引用 H-052（数据源许可证）、H-077（WN 编号、潮位、气象）、H-078（锚点人工校验）。
### 接口与数据
- `meta.json` 必含：`map_id="omaha_dog_green"`、`date_setting="1944-06-06"`、`crs.epsg=32630`、`height.samples=2049`、`far_ring.samples=1025/extent_m=8192`、`anchors[]`、`similarity{dx,dy,rot_deg,applied}`、`sources[]{id,license,verified}`。

## 不做什么
- 不下载大体量 DEM、不生成高度图与矢量（M4-002）。
- 不摆放 WN/障碍（M4-003）、不写 zones/spawns（M4-004）。
- 不嵌入任何 IWM/NARA 影像；不把 GSGS 扫描件入库。
- 不自行把 LICENSES.md 状态改为 OK（由人类登记，H-052）。

## 验收标准
1. `sources.yaml` 覆盖 MAPS §4.3 中全部数据源，每条含 `license`、`status`、`verified_date`；`map-pipeline validate-sources omaha_dog_green` 退出码 0。
2. `anchors.json` 锚点数 ≥ 8，每个 `confidence_m` 已填；脚本报告拟合残差 RMS，若 > 60 m 则在报告中列出最差 3 个锚点。
3. `meta.json` 通过 `map-pipeline validate`（JSON schema `frontline.map.meta/1`）；`similarity.applied` 与"偏差 ≤150 m 仅平移 / 否则 ≤10° 旋转"规则自洽（单测覆盖两种分支）。
4. 每个 PENDING 源在 `docs/NEEDS_HUMAN.md` 有对应追加条目；人工项均标出对应 H- 编号。
5. 重复运行两次，`meta.json` 与拟合输出逐字节相同（固定排序与浮点格式化）。

## 测试要求
- 单元测试：`fit_similarity` 对合成点集（已知 dx/dy/rot）恢复误差 < 0.01 m / 0.01°；旋转 > 10° 时拒绝并报错；锚点 < 3 个时报错。
- 集成测试：`pytest tools/map-pipeline/tests/test_omaha_meta.py` 校验 `meta.json` schema 与 sources 引用完整性。
- 性能基准：无。
- 回归：无（首次生成）。

## 性能预算
无（离线工具任务）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest`、`cargo test`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工输入：H-052（数据源许可证原页核实）、H-077、H-078；缺失时任务标 `blocked` 或 `partial`，继续做不依赖者。H-052 无默认假设：未核实源只作参考不入库。
- MAPS §0.4 建议每图 2–4 个锚点，INDEX 要求 ≥ 8；多出的锚点用于拟合稳健性，不改变 §0.4 规则。
- 测深（水下浅滩/登陆艇航道）数据源在 MAPS 中未列出，**未核实**：本任务只登记"待查"，由人工补充；缺失则 M4-002 水下浅滩用平缓坡面合成。
- 教堂精确坐标与潮位均为"未核实"，不得在文案中当作史实展示。
