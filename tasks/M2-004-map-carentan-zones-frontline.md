# M2-004 地图：Carentan Frontline 关卡标注

## 元数据
- 里程碑：M2
- 工作流：maps
- 规模：M
- 依赖：M2-003
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §3.6; MAPS §6.2; MAPS §6.3; GAMEDESIGN §3.3; GAMEDESIGN §3.7; GAMEDESIGN §10.5; GAMEDESIGN §15.1

## 背景
把 MAPS §3.6 的原创关卡设计写成可执行数据。Frontline 模式（GD §3.3）：5 条战线 L1–L5，每线 2–3 个候选据点；开局服务器用 `seed = hash(match_id)` 每线随机选 1 个，Sector 多边形必须任意组合可无缝拼满地图（候选 Sector 在主轴方向的边界对齐固定"战线分界线"）。地图作者约束（GD §3.3.1）：任意选点组合 ≥ 2 条可行进攻路线；L3 候选间距 ≥ 300 m；同线候选间路径距离差 ≤ 25%。
候选据点（MAPS §3.6，位置 (x,y) m ±误差，强点半径【待调参】）：L1 {L1a Ingouf Farm (900,1450) 45 m；L1b Southern Bridge (560,1700) 35 m；L1c Orchard Rise (300,1300) 50 m}；L2 {L2a (700,800) 55；L2b Canal Lock (1200,1350) 40；L2c (500,1000) 50}；L3 {L3a Notre-Dame (1277,897) 50；L3b Place de la République (1380,990) 45；L3c Convent Block (1100,720) 50}；L4 {L4a Station (1500,520) 45；L4b Goods Yard (1250,400) 50；L4c Mill Quarter (1650,800) 45}；L5 {L5a South Ridge Farm (1000,200) 50；L5b Taute Meadow Battery (1700,250) 55；L5c Lane Crossroads (1400,150) 45}。HQ：US (250,1750)、GER (1850,250)，半径 40 m，HQ 距 L1 据点 ≥ 200 m（GD §3.3.2）。
设计位置按 §0.4 与 M1-030 的相似变换平移；落入水体/建筑内/坡度 > 35° 者吸附到最近开阔地 ≤ 60 m 并在 `notes` 记录；**M1 临时 HQ（(480,1600)/(1480,560)）不得写入**。

## 具体内容
### 要创建/修改的文件
- `data/maps/carentan/zones.json` —— `lines[]`（每线 `candidates[]`：`id/name_key/origin(R|G)/pos/pos_error_m/radius_m/sector_polygon/tags/resource_sites[3]`）、`assault_stages[]`（MAPS §3.6 五段，供 M3 使用）、`skirmish_areas[]`（S1 老城 [900,500,1700,1200]、S2 堤道—农场 [200,1100,1100,1900]、S3 西南树篱 [100,200,1000,1100]，M4 使用）、`playable_polygon`、`out_of_bounds_buffer_m:64`、`line_boundaries[]`（战线分界线折线）
- `data/maps/carentan/spawns.json` —— `hq`、`garrison_candidates[]`（US：(250,1750)、(350,600)；GER：(1700,900)、(1000,200)）、`outpost_candidates[]`、`airhead_candidates[]`、`observation_posts[]`（教堂塔窗口等，`cover_zone` 内）、每个据点的 `strongpoint_spawn`（距圆心 ≥ 60 m，GD §10.5）、`fallback_spawn`（Assault，M3 用）
- `data/maps/carentan/vehicle_routes.json` —— 路线点图、咽喉点（N13 堤道单车道）、不可通行区（洪泛区、`vehicle_blocking` 树篱）、每队 HQ 载具位（轻 2、中 2、重 1、运输 3，GD §14.6）
- `data/maps/carentan/artillery.json` —— 每队 HQ 附近 2–3 个预设火炮阵地占位（`sites[]`，R7；坐标待人工在 T-GEO 后确认，先放占位并标 `placeholder:true`）与场外炮火源（US 西北、GER 南部，MAPS §3.6）
- `tools/map-pipeline/zones.py`、`validate.py`（扩展：据点、路径、入口、Sector 拼合检查）
### 要实现的行为
1. 每个候选据点计算 ≥ 3 条步兵入口（不同方位象限，按可达 navmesh 近似：可站立开阔地 + 非洪泛 + 非实心建筑），写入 `entrances[]`。
2. Sector 多边形：对 5 线 × 3 候选的 3^5 = 243 种组合，脚本逐组合检查"并集 = 可玩区、两两不重叠"（允许 0.5 m 容差）；失败组合列表输出。
3. 可行路线：对每种组合检查 US HQ → 各据点 → GER HQ 步兵连通且 ≥ 2 条边不相交路径；同线候选路径差 ≤ 25%；L3 候选两两距离 ≥ 300 m。
4. 载具路线：HQ 载具位 ↔ 各 L1/L2 据点连通（沿 N13 堤道与市区主路）。
5. `resource_sites`：每个 Sector 预设 3 个（Manpower/Munitions/Fuel 各 1，GD §11.2）且在己方 Sector 内、非水体。
### 接口与数据
- JSON 格式沿用 MAPS §6.2（`frontline.map.zones/1`、`frontline.map.spawns/1`）；`name_key` 为 i18n key（A7）。
- 与 ARCH §8.5.5 的 `map.json` 字段不同的部分：本任务按 MAPS §6.1 文件清单产出；若 sim 加载器需要 `map.json` 汇总文件，由 M2-008 读取这些文件，不在此处合并。

## 不做什么
- 区块切片/碰撞/navmesh → M2-005（本任务路径检查用几何近似，navmesh 连通在 M2-005 复核）。
- Assault 的 `fallback_spawn` 与分段只留数据，规则 → M3；Skirmish 子区只留数据，规则 → M4。
- 火炮操作规则 → M3；这里只放阵地占位。

## 验收标准
1. `map-pipeline validate --map carentan --stage zones` 通过：所有坐标在 0..2048，据点不落入水体/实心建筑，坡度 ≤ 35°。
2. 15 个候选据点各 ≥ 3 条步兵入口；两队 HQ 间步兵与载具（主路）均连通。
3. 243 种选点组合：Sector 拼满地图（缺口/重叠面积 = 0 m² 容差 0.5 m）；每组合 ≥ 2 条不相交进攻路线；同线候选路径差 ≤ 25%；L3 候选两两距离 ≥ 300 m。
4. 每个 Sector 恰有 3 个 `resource_site`（M/Mu/F）；每队 HQ 载具位数 = 轻 2、中 2、重 1、运输 3。
5. `spawns.json` 不含 M1 临时 HQ 坐标（脚本字符串/坐标检查）。
6. 产物 sha256 可复现。

## 测试要求
- 单元测试：`pytest tools/map-pipeline/tests/test_zones.py`：Sector 拼合检查（构造重叠/缺口用例必须报错）、路径差计算、吸附 ≤ 60 m。
- 集成：用 M1 的 L1a/L2a/L3a 子集回归，与 M1-034 数据一致。
- 性能基准：243 组合检查脚本耗时记录。
- 回归：`hashes.json` 更新。

## 性能预算
无（离线数据）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 据点位置依赖 H-051（锚点人工校验）；未核实者（L1a ±250、L4a ±250 等）若吸附后拓扑变化须在 PLAN_ISSUES 登记。
- 火炮阵地具体坐标 MAPS §3.6 标"待定"，先占位；M3 火炮任务前需人工确认。
- 命名与布局为原创推导（MAPS §3.6 原创性声明），不得参照商业游戏同名图。
