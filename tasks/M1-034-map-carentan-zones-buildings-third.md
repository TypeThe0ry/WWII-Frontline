# M1-034 地图：Carentan 1/3 关卡标注与建筑足迹

## 元数据
- 里程碑：M1
- 工作流：maps
- 规模：L
- 前置依赖：M1-030, M1-032, M1-033
- 素材需求：无（地标用灰盒）；素材状态：placeholder（V-BLD-* 建筑地标，真素材由 M1-064 接入）
- 引用文档：MAPS §3.8; MAPS §3.6; MAPS §3.4; MAPS §6.2; MAPS §6.3; MAPS §1.6; MAPS §1.7; GAMEDESIGN §3.6

## 背景
把 MAPS §3.6/§3.8 的"原创关卡设计"写成可校验的数据（T-ZONES、T-BLD），仅限 **M1 的 1/3 区域（设计框架 x 380–1560、y 520–1700）**。输出遵守 MAPS §6.1/§6.2 schema，并由 `map-pipeline validate`（MAPS §6.3）检验。
**M1 切片参数（MAPS §3.8）**：
- 3 个活动据点：**L1a Ingouf Farm**（设计 (900,1450) ±250，半径 45 m）、**L2a Market-Garden Fields**（(700,800) ±80，半径 55 m）、**L3a Notre-Dame**（(1277,897) ±40，半径 50 m）（半径【待调参】，MAPS §3.6）；区内另有 L1b、L2c、L3b 作候选记录但 M1 不激活。
- **M1 临时 HQ**：US (480,1600)、GER (1480,560)，半径 40 m；**M1 临时 Garrison 位置**：US (420,1550)、GER (1500,600)；**"相对完整图的临时位置，不要写入完整图的数据"**——因此本任务把切片专属数据写到 `data/maps/carentan/m1/{zones,spawns}.json`（M1-018/M1-019 读取），**不写入** `data/maps/carentan/zones.json|spawns.json`（M2 完整图用）。细化须登记 PLAN_ISSUES（ARCHITECTURE §8.5.5/MAPS §6.1 未定义切片布局）。
- 坦克路线：N13 堤道 → L1a → L3 老城（M1 仅含主路），写 `vehicle_routes.json` 的 `m1_main_route`。
- 20 人规模：每队 10 人。
- 若 M1-030 的相似变换 `applied=true`，设计位置须经变换（真实地标不变换）；设计据点落入水体/实心建筑/坡度 > 35° 时吸附 ≤ 60 m 并记 `notes`。
**建筑**：`buildings.geojson` 的 `style`（诺曼底石屋/农场 U 形院落/教堂/拱廊，MAPS §3.5）、`levels`（市中心 2–3 层）、`roof`、`enterable`（**可进入建筑占比 ≤ 25%**，MAPS §3.7/H-061 默认）、`destructible:false`（M1）、`asset_ref`（需求 ID 占位，如 `V-BLD-…`）、`ruin_stage`。**核心地标灰盒定义**（`landmarks.json`）：Notre-Dame 教堂（还原精度 A，位置 (1277,897) ±40，带塔——OP 点 M1 不启用）、Place de la République 拱廊广场（B/A，(1380,990) ±120，**未核实**）、Ingouf 农场（B，U 形院落+谷仓）、N13 石桥（A）；占位按 ASSET_GAPS §3.2：盒体 + 屋顶斜面，**门/窗洞按玩法需要保留**，可进入建筑必须有可通行门洞与楼层。

## 具体内容
### 要创建/修改的文件
- `data/maps/carentan/m1/zones.json` —— `lines`（L1/L2/L3 各含 a 点，`origin:"R|G"`、`pos`、`pos_error_m`、`radius_m`、`tags`）、`playable_polygon`（切片矩形，含 `out_of_bounds_buffer_m 64`）、`m1_active:["L1a","L2a","L3a"]`
- `data/maps/carentan/m1/spawns.json` —— `hq`（US/GER，pos、radius 40）、`garrison_candidates`（M1 切片候选点，已按裁定移到距各自 HQ ≥ 200 m，见 MAPS §3.8）、`kinds_supported:["hq","garrison"]`、`cover_zones`
- `data/maps/carentan/vehicle_routes.json` —— `m1_main_route` 折线 + 不可通行区引用（`flood_marsh`、树篱封锁段）
- `data/maps/carentan/landmarks.json` —— M1 区域内核心地标灰盒定义（`id/name/kind/pos_e/pos_n/design/fidelity/state_1944/asset_ref`、`placeholder_dims_m`）
- `data/maps/carentan/buildings.geojson`（更新：样式/levels/enterable/roof 标注）
- `data/maps/carentan/m1/cover_points.json`（供 bots：据点周边掩体点 ≥ 8/据点）、`nav_critical.json`（关键路径标记，GD §12.4 的"不得包围 HQ/据点"检测用）
- `tools/map-pipeline/map_pipeline/zones.py`、`buildings_annotate.py`、`validate.py`（`map-pipeline validate --slice m1`）
- `tools/map-pipeline/tests/test_validate_*.py`、`data/maps/carentan/m1/__fixtures__/`（失败夹具）
### 要实现的行为
1. 从 MAPS §3.6/§3.8 取设计值，经 M1-030 变换 + 吸附，写入 zones/spawns；每个据点 `pos_error_m` 保留原值。
2. 建筑标注：规则 + 半自动：`style` 由所在区域（市中心/农场/郊区）与足迹形状启发式；`levels` 由足迹面积与区位（核心区 2–3、其余 1–2）；`enterable` 选取据点周边（≤ 60 m）面积 ≥ 60 m² 的建筑，总占比 ≤ 25%；教堂/农场/拱廊建筑按地标定义覆盖。
3. 核心地标：每个地标输出 `placeholder_dims_m {l,w,h}`、`doors[]`（门洞中心/宽度）、`floors[]`，用于 M1-052 生成灰盒 + 碰撞；教堂塔高度标注（供视线/掩体设计）。
4. 载具路线：从 N13 堤道入口（切片边界）到 L1a 再到老城主路，沿 `roads.geojson` 的 `vehicle_ok` 路段；检查宽度 ≥ 车体宽 + 1 m（Sherman 2.62 m）；标出咽喉点（桥）。
5. 校验规则（`map-pipeline validate`）：JSON schema；坐标 0..2048；**据点不落入水体与实心建筑**；**每据点 ≥ 3 条步兵入口**（以 `nav_critical`+建筑门洞+开阔弧段计数，脚本实现为：据点圆周上与通行区的离散扇区≥3）；**HQ 间可通行**（步兵与载具分别用栅格 A* 做粗连通检查，精确检查在 M1-036 navmesh）；Garrison 候选距离规则（距 HQ ≥ 200 m，GD §10.2）为**错误**级校验；`out_of_bounds_buffer_m` 存在。
6. 写 `reports/m1-zones-validate.json`（每条规则 pass/fail 与细节）。
### 接口与数据
```json
{"schema":"frontline.map.zones/1","lines":[{"id":"L1","points":[{"id":"L1a","name_key":"carentan.ingouf_farm","origin":"R","pos":[900,1450],"pos_error_m":250,"radius_m":45,"tags":["stone_courtyard","hedgerow"]}]}],
 "m1_active":["L1a","L2a","L3a"],"playable_polygon":[[380,520],[1560,520],[1560,1700],[380,1700]],"out_of_bounds_buffer_m":64}
```

## 不做什么
- 完整 5 线 Frontline 数据、Assault/Skirmish 分段、完整 HQ/Garrison/OP/炮兵数据（M2/M3）；
- 真建筑/地标素材接入：M1-064（需求 ID 前缀 V-BLD）；灰盒生成与渲染：M1-052；
- 区块烘焙：M1-035；navmesh：M1-036；
- 视线/性能 QA（T-QA）：M1-069 之后；
- 对 MAPS 设计内容的修改（拓扑/战线顺序不得改变，MAPS §0.4）。

## 验收标准
1. `map-pipeline validate --slice m1` **全部通过**：据点不在水体/实心建筑内；3 个活动据点各 ≥ 3 条步兵入口；US HQ↔GER HQ 步兵与载具粗连通；`playable_polygon` 与 buffer 存在（报告 `reports/m1-zones-validate.json` 中全部规则 pass）。
2. 数据位置：`data/maps/carentan/zones.json|spawns.json`（完整图）**不存在 M1 临时 HQ/Garrison 坐标**（grep 断言 (480,1600)/(1480,560)/(420,1550)/(1500,600) 仅出现在 `m1/` 下）。
3. 可进入建筑占比 ≤ 25%（统计脚本，切片矩形内）；每栋 `enterable` 建筑 ≥ 1 个门洞且有 `levels ≥ 1`；教堂/农场/拱廊/桥 4 个核心地标均在 `landmarks.json` 中有 `placeholder_dims_m` 与 `asset_ref`（需求 ID）。
4. 载具主路：从切片入口到 L1a 到 L3a 附近连通，最窄处宽度 ≥ 3.6 m（Sherman 2.62 m + 1 m），咽喉点列表输出（≥ 1 个桥/窄口）。
5. 每个据点周围 ≥ 8 个 `cover_points`（距圆心 ≤ R+30 m，不在水体/建筑内）；`nav_critical` 通过"不将 HQ/据点完全包围"的占位检查。
6. 吸附记录：若有据点被吸附，`notes` 记录原位与新位且距离 ≤ 60 m；超出则校验失败（单测夹具）。
7. 失败夹具：把一个据点置于水体、置于实心建筑、仅 2 条入口、HQ 被封死的 4 个夹具各自使 `validate` 失败并指向规则名（4 例单测）。
8. 同输入输出哈希一致；`validate` 运行 ≤ 60 s。

## 测试要求
- 单元测试：吸附、入口扇区计数、占比统计、连通检查、各规则正反夹具。
- 集成：对 M1-031/032/033 真实输出运行；与 M1-018/M1-019 的 sim 夹具对读（`zones.json` 被 `cargo test -p sim map_zones_load` 解析）。
- 性能基准：`validate` 耗时记录。
- 回归：`reports/m1-zones-validate.json` 入库。

## 性能预算
- 构建期；建筑数量与可进入比例间接约束运行期预算（ARCHITECTURE §13.5：High 主视图可见三角形 ≤ 4.0 M、draw call ≤ 1 200）——本任务只控制数量（可进入 ≤ 25%）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **已裁定**：MAPS §3.8 的 M1 临时 Garrison 候选点已移动到距各自 HQ ≥ 200 m（满足 GAMEDESIGN §10.2），不再需要 `m1_temp` 例外；本任务按修订后的 MAPS §3.8 写入。
- 据点 L1a 半径 45 m、位置 ±250 m 不确定度较大，锚定（H-051）后位置可能整体平移；锚定未完成时写 `pos_error_m` 并保持设计值。
- 地标/建筑真素材缺口见 ASSET_GAPS（GAP-V02 灰盒）；M1 允许灰盒（MILESTONES §2.5）。
- Place de la République 与 Augustine 修道院位置**未核实**（MAPS §3.4）；M1 仅用前者且标 `fidelity:"B"`、`verified:false`。
- 规模 L；若超出，将建筑标注与 cover_points 拆为 `M1-034b`。
