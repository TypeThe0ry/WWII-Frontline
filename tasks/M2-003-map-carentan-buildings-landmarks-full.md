# M2-003 地图：建筑样式与地标（全图）

## 元数据
- 里程碑：M2
- 工作流：maps
- 规模：M
- 依赖：M2-002, M1-034
- 素材需求：V-BLD-*（灰盒可）；素材状态：placeholder
- 引用文档：MAPS §3.4; MAPS §3.6; MAPS §3.7; MAPS §1.6; MAPS §1.7; MAPS §3.5

## 背景
M1-034 对 1/3 区域做了建筑样式、可进入性与核心地标灰盒定义。M2 要全图 300–500 栋建筑（老城核心约 800×600 m，MAPS §3.7），其中**可进入建筑 ≤ 25%**（性能与关卡设计），其余为"实心"外壳（MAPS §1.6）。风格：诺曼底石屋——石灰岩/砂岩墙、陡坡板岩屋顶、石烟囱；市中心 2–3 层，广场有拱廊；农场 U 形院落 + 石谷仓（MAPS §3.5）。
地标（MAPS §3.4，还原精度 §1.7：A 位置 ±5 m、B ±15 m、C ±40 m）：Église Notre-Dame（设计框架 (1277, 897) ±40，A）、Place de la République（约 (1380, 990) ±120，**未核实**，A）、火车站（暂定 (1500, 520) ±250，B）、Douve 河与运河、N13 堤道/Purple Heart Lane 含四座桥、Ingouf 农场（约 (900, 1450) ±250，B）、Augustine 修道院（位置未核实，C）。位置以 M1-030 锚点与 H-051 人工核对结果为准，未核实者在 landmarks.json 里标 `verified:false`。

## 具体内容
### 要创建/修改的文件
- `data/maps/carentan/buildings.geojson` —— 全图建筑足迹，属性：`style`、`levels`、`roof`、`enterable`、`destructible`、`asset_ref`、`ruin_stage`
- `data/maps/carentan/landmarks.json` —— 每项 `id`、`name`、`kind(church|bridge|station|square|farm|convent|canal)`、`pos_e/pos_n`、`design:[x,y]`、`fidelity(A|B|C)`、`state_1944`、`refs[]`、`asset_ref`、`verified`
- `tools/map-pipeline/buildings.py` —— 风格规则（按区域/足迹面积/OSM tag 分配 style 与 levels）、可进入性挑选（人工清单优先，余者规则选择）、占比统计
- `data/maps/carentan/building_styles.json` —— 风格表：`norman_townhouse`、`norman_farmhouse`、`farm_courtyard_U`、`stone_barn`、`arcade_block`、`station_building`、`shed` 等 → 占位体量参数（高度 = levels×3.0 m，屋顶类型）
### 要实现的行为
1. 全图建筑清洗：沿用 M2-002 删除现代建筑后的足迹；对 1944 影像未见者按人工清单补缺（`source:hand`）。
2. 风格分配：老城（x 900–1700, y 500–1200）以 `norman_townhouse`/`arcade_block` 为主；农场区 `farm_courtyard_U`；levels：市中心 2–3、郊区 1–2。
3. 可进入性：`enterable=true` 的建筑数 / 总数 ≤ 25%（H-061 默认假设：人工量超预期时降到 ≤ 15%）；可进入建筑必须有至少 1 个门（`door` 点）与窗翻越点标注（供 M2-005 off-mesh 与 bot 使用）。
4. 地标灰盒：`asset_ref` 指向 `PH_V-BLD-xxx` 占位（程序化盒体，尺寸取自足迹与 levels）；教堂塔楼高度作为 OP 点候选记录在 landmarks（MAPS §3.6：塔内部为 Spotter 专用，窗口 2 个）。
5. 所有地标在 `landmarks.json` 与 `buildings.geojson` 之间可交叉引用（`landmark_id` 字段），不得重复建筑。
### 接口与数据
- CLI：`map-pipeline buildings --map carentan --full`；`map-pipeline validate --map carentan --stage buildings`
- `asset_ref` 约定：`assets/processed/carentan/{id}.glb` 为真素材目标路径；缺失时清单中 `placeholder:true`（ASSET_GAPS §3）。

## 不做什么
- 真素材接入与外观 → M2-047（assets-m2-integrate）；建筑渲染 → M1-052 已有、LOD 优化 → M2-034/M2-036。
- 据点/出生点放置 → M2-004；碰撞与 navmesh → M2-005。
- 建筑可破坏逻辑属 M3 以后（`destructible` 字段仅预留）。

## 验收标准
1. `buildings.geojson` 总数在 300–500 之间；`enterable` 占比 ≤ 25%（`validate --stage buildings` 打印数量与百分比）。
2. 每栋 `enterable` 建筑有 ≥ 1 个 `door` 点与 `levels ≥ 1`；100% 建筑有 `style`/`levels`/`roof`/`asset_ref`。
3. `landmarks.json` 含教堂、广场、车站、桥（≥ 2 座框内）、Ingouf 农场、运河各 ≥ 1 项，每项 `asset_ref` 非空且 `fidelity` 合法；未核实位置 `verified:false` 比例在报告中列出。
4. 建筑与水体/洪泛区重叠面积 = 0；建筑两两重叠 = 0（`validate`）。
5. 同输入两次运行输出 sha256 一致。

## 测试要求
- 单元测试：`pytest tools/map-pipeline/tests/test_buildings.py`：风格规则确定性、可进入占比约束（超限时报错）、重叠检测。
- 集成：对 M1 区域，M2 产物与 M1-034 的 style/enterable 分配一致或差异已记录。
- 性能基准：无。
- 回归：更新 `hashes.json`。

## 性能预算
无（离线数据）。运行时约束：建筑 LOD0/1/2 与 WebGL2 预计算遮挡体由 M2-005/M2-034 承担（MAPS §3.7）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工项：建筑样式/可进入性/重点建筑标注（T-BLD）、地标摆放（T-LAND，不自制模型）；见 NEEDS_HUMAN H-061、H-051。无人工清单时用规则分配并将状态记 `partial`。
- 缺失素材需求：V-BLD-004/008/010 等（SHOP §4.3）；全部以灰盒通过。
- 车站/广场/修道院坐标未核实，据点（M2-004）不得依赖这些点的精确位置。
