# M1-033 地图：1944 修正（M1 区域）

## 元数据
- 里程碑：M1
- 工作流：maps
- 规模：M
- 前置依赖：M1-031, M1-032
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §1.5; MAPS §3.5; MAPS §3.6; MAPS §3.7; MAPS §6.2; MAPS §7.1

## 背景
现代 LiDAR 反映 2020s 地面（MAPS §1.5）：堤道、运河、海堤可能被改造，1944-06 德军放水**洪泛** Douve 河漫滩（沼泽，**车辆不可通行**），Carentan 的 **bocage 树篱+土堤**大量被拆并。本任务（T-HEDGE、T-FLOOD、T-VEC 后半，**`[人工]`**）对 **M1 区域（设计框架 x 380–1560, y 520–1700）** 做 1944 修正：
- **树篱描摹**（人工为主）：折线 + 属性 `height_m`（1.5–2.5）、`mound_width_m`（土堤宽）、`density`、`has_gap`（缺口）、`vehicle_blocking`；树篱是 Carentan 掩体与视线分割的主要工具（MAPS §3.6/§3.7）；西南树篱区**步兵专属**（树篱高 ≥ 1.8 m 且无缺口 ≥ 4 m 载具无法通行）；游戏上用样条 + GPU instancing 的"树篱段"，碰撞用线性盒。
- **洪泛区修正多边形**：北与东侧大面积（设计框架 y > 1500 的堤道两侧、x > 1650 的东缘——后者在 M1 矩形外），浅水 + 草沼，车辆不可通行，步兵涉水（速度 ×0.5【待调参】）；与 `water.geojson` 的 `walkable_depth` 联动。
- **地形修正**：`terrain_edits.geojson`：多边形 + 目标高程/抬升量 + 过渡宽度，由脚本烘焙进高度图（还原 1944 堤道/去采石与现代堤坝）。
- **删除现代建筑/道路**：对 M1-032 初稿对照历史资料（航拍/GSGS，**只作描摹参考，不入库**）剔除 1944 年后建筑与改线道路，补全 1944 小路。
- 精度策略：游戏性精度优先于史实精度；树篱用样条 + instancing，降低描摹精度；人工量超预期时减少工作量（H-061 默认假设）。
本任务不是纯脚本：人类提供/审核人工描摹（参考 H-061；M1 区域同时用于校准完整图工时，RISKS R-20）。**人工描摹缺失时本任务标 `partial`，不得用 OSM 自动树篱冒充**（可在 `hedgerows.geojson` 标 `source:"osm_draft"` 并保留）。

## 具体内容
### 要创建/修改的文件
- `data/maps/carentan/terrain_edits.geojson` —— 多边形要素，属性：`id`、`mode`（`set_height|raise|lower|flatten|embankment`）、`target_h_m`/`delta_m`、`transition_m`、`reason`、`source_note`
- `data/maps/carentan/hedgerows.geojson`（更新）—— 属性补全；`source ∈ {traced, osm_draft}`；`traced` 要素的 `confidence`（A/B/C 精度分级，MAPS §1.7）
- `data/maps/carentan/water.geojson`（更新）—— 新增 `flood_marsh` 多边形：`kind:"flood_marsh"`、`depth_m`、`walkable_depth`、`vehicle_blocking:true`
- `data/maps/carentan/roads.geojson`、`buildings.geojson`（更新）—— 现代剔除记录 `removed_modern.json`（id + 理由）
- `tools/map-pipeline/map_pipeline/apply_edits.py` —— `apply_terrain_edits(heightmap, edits) -> (heightmap', diff_report)`：羽化过渡（cos 或 smoothstep），多边形栅格化用 GDAL `rasterize`
- `tools/map-pipeline/map_pipeline/hedgerow_tools.py` —— 样条平滑、缺口检测、长度统计、与目标比较、`vehicle_blocking` 判定（高 ≥ 1.8 m 且无 ≥ 4 m 缺口）
- `reports/m1-1944-edits.json`、`reports/m1-heightmap-diff.json`
- `tools/map-pipeline/tests/test_apply_edits.py`、`test_hedgerows.py`
- `data/maps/carentan/slice_m1_targets.json` —— `hedgerow_total_m_target`（**由人类提供**；缺省 `null`）
### 要实现的行为
1. `apply_terrain_edits`：按多边形顺序（`order` 字段）依次应用；`transition_m` 外的高度不变；输出新的 `heightmap_1m.r16`（保留原始为 `heightmap_1m.modern.r16` 供对比，不入 git 或入 LFS）；记录每个多边形改变的格数、最大/平均 Δh。
2. 差分报告：全图变化格数、最大绝对变化、每个多边形统计、变化区域边界框；变化 > 3 m 的区域需在 `reason` 中说明（脚本告警）。
3. 树篱：人类描摹文件（GeoJSON，放入 `assets/incoming/maps/carentan/traced/hedgerows.geojson`）并入；与 OSM 初稿合并去重（缓冲 3 m 内重复保留 `traced`）；样条重采样（点距 2 m）；总长统计。
4. 洪泛：把 `flood_marsh` 多边形烘焙为 (a) 水体层（`water.geojson`）(b) 高度图中的轻微低洼（可选，`terrain_edits` 的 `lower`）(c) 供 sim 使用的 `vehicle_blocking`/`walkable_depth` 属性。
5. 现代剔除：对 `buildings.geojson/roads.geojson` 按人类提供的 `remove_ids.txt` 剔除；对未覆盖区域保留 OSM 初稿并标 `verified:false`。
6. 重新验证：应用后运行 M1-032 的 schema 校验与坐标范围检查；运行 `check-roads`（载具路线连通，供 M1-034）。
7. 所有人工输入文件缺失时脚本以 `partial` 状态退出（退出码 4），报告列出缺项，不阻塞后续任务的"灰盒"联调（M1-034/M1-035 可用 OSM 初稿继续）。
### 接口与数据
```json
{"type":"Feature","properties":{"id":"flood_douve_north","mode":"lower","delta_m":-0.3,"transition_m":12,"reason":"1944-06 flood marsh (north of causeway)","source_note":"NARA aerial ref pending"},"geometry":{"type":"Polygon","coordinates":[[[…]]]}}
```

## 不做什么
- 完整 2048 m 区的描摹（M2）；Omaha 潮位/Foy 林地（M4）；
- 区块烘焙与 splat：M1-035；关卡标注：M1-034；
- 历史影像嵌入（只参考，不入库；IWM 不嵌入）；
- 渲染洪泛水面/芦苇：M2（M1 灰盒）；树篱渲染与碰撞盒：M1-051；
- 自动生成"看起来像"的树篱来补足人工缺口（禁止冒充史料）。

## 验收标准
1. **区域树篱总长与目标 ±15%**：`reports/m1-1944-edits.json` 给出 M1 矩形内 `traced+osm_draft` 总长与 `slice_m1_targets.json` 目标比较，偏差 ≤ ±15%（目标缺失则该项记 `未测`，任务 `partial`）。
2. **应用后高度差分报告**存在：`reports/m1-heightmap-diff.json` 含总变化格数、最大绝对变化（应 ≤ 5 m，超出需 `reason` 说明）、逐多边形统计；无多边形时报告为空且脚本通过。
3. 羽化过渡正确：夹具单测（平地 + 一个 `raise 2 m, transition 10 m` 的圆形多边形）→ 中心 +2.0 m ±0.01、边缘外 10 m 处 0、过渡带单调；多多边形重叠按 `order` 叠加（3 例）。
4. 应用幂等：对同一输入连续应用两次得到逐字节相同的输出（输入原高度图 + edits，不对输出再次应用）；输出哈希可重现。
5. 树篱规则：`vehicle_blocking` 判定边界（高 1.79/1.81 m、缺口 3.9/4.1 m）单测 4 例；总长统计误差 < 1%（单测折线）。
6. 洪泛：`flood_marsh` 多边形内 `walkable_depth ≤ 1.4` 且 `vehicle_blocking:true`；矢量通过 schema；与 M1-032 初稿无自相交（`validate-vectors` 0 错误）。
7. 数据完整性：所有 geojson 坐标仍在 0..2048；`removed_modern.json` 每项含 id 与理由。
8. 缺少人工描摹时：脚本退出码 4（`partial`），STATUS 列出缺项（树篱描摹/洪泛多边形/剔除清单），不静默通过。

## 测试要求
- 单元测试：羽化、栅格化、树篱统计、缺口检测、合并去重。
- 集成：夹具高度图 + edits 全流程；与 M1-031 输出联调（真实数据到位后）。
- 性能基准：整图 2049² 应用 20 个多边形 ≤ 30 s。
- 回归：差分报告入库。

## 性能预算
- 构建期；树篱总长影响运行期实例数：设计目标遵守 ARCHITECTURE §9.6（树 ≤ 192/块、灌木 ≤ 256/块）与 §13.5 draw call 预算（由 M1-051/M1-035 验证）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **人工项**：树篱描摹、洪泛多边形、现代剔除清单（相关 NEEDS_HUMAN：H-061 为完整图审核，M1 区域为其工时校准；锚点 H-051）。缺失时状态至多 `partial`，写入 STATUS blocked 清单。
- 目标树篱总长无文档数值（MAPS 只给"西南区 > 15 km"）：本任务需要人类提供 M1 区域目标，缺省时验收 1 记 `未测`，并写 PLAN_ISSUES。
- 历史资料许可证（NARA 条目、GSGS 扫描件）仍待核实（H-052）：只作参考，**不得把影像/扫描件放入仓库**。
- 风险：人工量可能超预期（RISKS R-20）；超出时按 H-061 默认假设降低精度，不要延长任务。
