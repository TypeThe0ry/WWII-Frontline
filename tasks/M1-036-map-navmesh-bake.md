# M1-036 地图：Recast 导航网格烘焙

## 元数据
- 里程碑：M1
- 工作流：maps
- 规模：M
- 依赖：M1-035
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §16.2; MAPS §6.1; MAPS §6.2; MAPS §7.1; GAMEDESIGN §20.3

## 背景
bot 导航用离线烘焙的 **Recast** 网格（ARCHITECTURE §16.2），由 M1-037 加载与寻路。本任务只做**烘焙脚本、数据格式、连通性校验**（MAPS §7.1 T-NAV，AI 任务）。
烘焙参数（**必须与 ARCHITECTURE §16.2 一致**）：`cell_size 0.3 m`、`cell_height 0.2 m`、`agent_radius 0.4 m`、`agent_height 1.8 m`、`max_climb 0.5 m`、`max_slope 45°`；2 km 地图按 **128 m 瓦片**切分（`tile cache` 支持动态障碍如被摧毁建筑，属 M3，本任务只留接口）。
**文档差异**：MAPS §6.2 的 navmesh 参数为"半径 0.4 m、高度 1.8 m、爬坡 35°、台阶 0.4 m【待调参】"，**与 ARCHITECTURE §16.2（max_slope 45°、max_climb 0.5 m）不一致**；INDEX 明确"参数与 ARCH §16.2 一致"，故本任务取 ARCH，并把 MAPS 的差异登记 PLAN_ISSUES；参数全部放入 `navmesh_params.json` 以便调参（含 KCC 实际限制：步行坡度上限 35°，见 M0-010/GD §9）。
载具网格独立：半径 2.0 m、坡度 25°（MAPS §6.2），用于载具 bot（M3），M1 只烘焙并做 Sherman 路线连通检查。
off-mesh links（ARCHITECTURE §16.2）：窗户翻越、梯子、楼梯，**手工/半自动标注，写在地图数据中**；本任务定义标注格式并产出 M1 区域的初版（建筑门/楼梯来自 M1-034 的 `doors[]/floors[]`）。
输入：碰撞几何（M1-035 的高度场与建筑碰撞盒组合）、`water.geojson`（深水不可通行；`walkable_depth ≤ 1.4` 的涉水区可通行但高代价，GD §9.1 涉水 ×0.6）、树篱碰撞（`vehicle_blocking` 与步兵可翻越/缺口）、`nav_critical.json`。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/nav/bake.ts`（Node 子工具，使用 Recast/Detour 的 WASM 绑定，如 `recast-navigation`；**依赖选型与许可证需登记 LICENSES.md §5，若需新依赖先写 PLAN_ISSUES**）或 `tools/map-pipeline/map_pipeline/nav_bake.py` 经调用封装；入口统一为 `map-pipeline nav --slice m1`
- `tools/map-pipeline/nav/inputs.py` —— 生成 Recast 输入三角网（高度场按 1 m 抽样三角化 + 建筑碰撞盒 + 树篱线性盒 + 水面屏蔽）
- `tools/map-pipeline/nav/offmesh.py` —— 读 `offmesh.json`，校验起止点在 navmesh 上、方向、半径
- `data/maps/carentan/m1/offmesh.json` —— 链接标注格式与 M1 初版：`{id, kind: window|ladder|stairs|vault, start:[x,y,z], end:[x,y,z], radius, bidirectional, cost}`
- `data/maps/carentan/m1/navmesh_params.json` —— 上述烘焙参数 + `tile_size_m 128` + 区域 `areas`（`ground=1, wade=2(cost×2), door=3, water=0(blocked)`）
- `data/maps/carentan/m1/navmesh.bin`、`navmesh_vehicle.bin` —— 输出（见格式）
- `tools/map-pipeline/nav/check_connectivity.py` —— 连通性检查（调用 Detour 查询或导出多边形图后在 Python 里求连通分量）
- `reports/m1-navmesh.json`（瓦片数、多边形数、bin 体积、烘焙耗时、连通性结果、参数哈希）
- `tools/map-pipeline/tests/test_nav_*.py`（小夹具地形）
### 要实现的行为
1. **格式**：`navmesh.bin` = 自有头（`magic "FNAV"`、`version:u16`、`params_hash:u64`、`tile_count:u32`、`origin`、`tile_size`）+ 标准 **Detour tile blob** 序列（每瓦片 `{tx, tz, size, data}`）；加载器（M1-037）只需 Detour 兼容反序列化；头中 `params_hash` 用于与 `navmesh_params.json` 一致性校验。
2. 烘焙：对 M1 区域 `playable_polygon` 范围按 128 m 瓦片（≈ 10×10 = 100 块）逐块烘焙；瓦片边缘外扩 `border = agent_radius×3+…`（Recast 标准 border）避免缝；并行（进程/线程池）。
3. 区域类型：水（不可通行）、涉水（可通行代价 ×2，对应 `walkable_depth` 区）、门洞（`door`）、普通地面；树篱：无缺口段不可穿越（步兵可翻越的低树篱留 off-mesh `vault` 链接，高 ≥ 1.8 m 的整段阻挡）。
4. 建筑内部：灰盒建筑有楼层/楼梯/门洞；每层作为独立可行走表面（用 Recast 多高度 span 自然支持）；楼梯用 off-mesh `stairs` 链接（若坡度/台阶高度超限）；窗户翻越用 `window`（可选，按 H-061 默认少量）。
5. 载具网格：半径 2.0 m、`max_slope 25°`，输入排除建筑内部与不可通行树篱/洪泛；输出独立文件。
6. 连通性检查（`check_connectivity`）：US HQ (480,1600) ↔ L1a/L2a/L3a/GER HQ (1480,560) 步兵连通；两 HQ 之间 Sherman 主路（M1-034 `m1_main_route`）载具连通；每个**可进入建筑**的门口 → 室内每层可达（连通分量数 = 1，孤岛面积 < 阈值需报告）。
7. 烘焙参数哈希与输出哈希可重现；烘焙耗时记录。
8. 提供 Python/Node 命令 `map-pipeline nav-query --from x,z --to x,z` 返回路径长度（调试用，≤ 50 行包装）。
### 接口与数据
```json
{"offmesh":[{"id":"om_door_001","kind":"window","start":[1280.2,3.1,901.5],"end":[1280.2,3.1,903.4],"radius":0.4,"bidirectional":true,"cost":2.0}],
 "params":{"cell_size":0.3,"cell_height":0.2,"agent_radius":0.4,"agent_height":1.8,"max_climb":0.5,"max_slope":45,"tile_size_m":128}}
```

## 不做什么
- 运行期加载、A*、漏斗、避障、bot 使用 off-mesh：M1-037；
- tile cache 动态障碍（被摧毁建筑）：M3；
- 载具 bot 沿道路图驾驶：M3（本任务只烘焙并检连通性）；
- 完整地图 navmesh：M2；
- 建筑内部精修（更多楼梯/窗户）：随 M1-034b/M2。

## 验收标准
1. **HQ↔据点导航连通**：`check_connectivity` 报告 US HQ ↔ L1a、L2a、L3a、GER HQ 四条步兵路径存在；Sherman 主路载具连通；连通检查结果写入 `reports/m1-navmesh.json`（全部 `true`）。
2. **烘焙 < 5 min**（切片 100 瓦片，参考机，记录机器与核数）；单瓦片最大耗时记录。
3. **参数与 ARCH §16.2 一致**：单测 `test_params_match_arch` 读取 `navmesh_params.json` 断言 `cell_size==0.3, cell_height==0.2, agent_radius==0.4, agent_height==1.8, max_climb==0.5, max_slope==45, tile_size_m==128`；`navmesh.bin` 头的 `params_hash` 与文件一致。
4. 建筑可达：切片内所有 `enterable` 建筑的门口到每个楼层的可达率 = 100%，孤岛（面积 > 4 m²）数 0（脚本统计；不合格建筑列入报告并回写 M1-034 修正）。
5. 禁区正确：随机 1 000 个水体/洪泛深水点查询最近多边形距离 > 0.5 m（即不被网格覆盖）；随机 1 000 个路面点覆盖率 ≥ 99%。
6. off-mesh 链接：`offmesh.json` 中全部链接起止点距 navmesh ≤ 0.5 m（校验脚本 0 违规），数量 ≥ 建筑门洞数的 80%（初版）。
7. 载具网格：半径 2.0 m；Sherman 路线最窄处通过检查（窄于 4 m 的路段在载具网格中不可通行，报告被剔除段）。
8. 体积：`navmesh.bin` ≤ 6 MB（ARCHITECTURE §11.4 "导航/碰撞 ≈ 6 MB"全图预算，M1 切片应小得多；记录实际值）；同输入两次烘焙 `navmesh.bin` sha256 一致。

## 测试要求
- 单元测试：参数一致性、`offmesh` 校验、区域标注、夹具小地形（含楼梯建筑）烘焙连通。
- 集成：对 M1-035 输出的完整切片烘焙并跑连通检查；与 M1-037 的加载器联调（反序列化后多边形数一致）。
- 性能基准：验收 2。
- 回归：`reports/m1-navmesh.json` 入库；任何参数变更须重新烘焙并更新哈希。

## 性能预算
- 构建期 < 5 min；运行期预算（60 bot 路径查询 ≤ 1.0 ms/tick，ARCHITECTURE §16.2/§16.5）由 M1-037 验收，本任务的网格复杂度（多边形数）需记录供其评估。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Recast 的 Rust/Node/Python 绑定选型 INDEX/ARCH 未定（ARCHITECTURE §4.3.6 写"`recast-navigation` 绑定或纯 Rust 实现"）；格式取"Detour 标准 tile blob"以保持运行期可选 Rust 绑定。选型与许可证登记 PLAN_ISSUES/LICENSES §5。
- 灰盒建筑的门洞/楼梯数据质量直接决定 E1 风险"bot 在建筑内多层导航失败"（MILESTONES §2.8）；M1-034 必须保证门洞与楼层字段完整。
- MAPS §6.2 与 ARCH §16.2 的 `max_slope/max_climb` 差异需 Fable 裁决（ARCH 优先）；KCC 步行坡度上限（M0-010 为 35°）与 navmesh 45° 不一致可能导致 bot 卡坡，bot 层（M1-037 的卡住检测 B-21）兜底。
- 地图上游若 `partial`（M1-031 DEM 缺失/M1-033 缺树篱），本任务仍可用程序化/初稿数据烘焙联调，但 STATUS 须标注数据质量。
