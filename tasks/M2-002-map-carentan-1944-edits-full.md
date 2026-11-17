# M2-002 地图：Carentan 1944 修正（全图）

## 元数据
- 里程碑：M2
- 工作流：maps
- 规模：L
- 前置依赖：M2-001, M1-033
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §1.5; MAPS §3.5; MAPS §7.1; MAPS §3.6; MAPS §3.7

## 背景
现代 LiDAR 反映 2020s 地面，而游戏设定为 1944-06-10 至 06-13 的 Carentan（MAPS §3.1/§3.5）。MAPS §1.5 规定修正项：删除 1944 年后建筑；N13 改线/四车道段还原为 1944 堤道，运河与港口范围还原；**大量恢复树篱**（田块边界树篱 + 土堤）；**1944-06 洪泛**——德军放水淹没 Douve 河漫滩，浅水 + 草沼，**车辆不可通行**、步兵涉水慢；植被为沼泽草地/苹果园/橡树树篱。M1-033 已做 1/3 区域的描摹与 `terrain_edits.geojson` 流程；本任务扩展到全图。
树篱是整张图的战术骨架：西南 bocage 区（设计框架 x<1100、y<900）树篱总长 > 15 km（MAPS §3.7），高 1.5–2.5 m，高 ≥ 1.8 m 且无 ≥ 4 m 缺口者对载具阻挡（MAPS §3.6 西南侧翼"载具无法通行"）。洪泛区：设计框架 y>1500 的堤道两侧、x>1650 的东缘（MAPS §3.4）。

## 具体内容
### 要创建/修改的文件
- `data/maps/carentan/hedgerows.geojson` —— 全图树篱折线（在 M2-001 初稿上补全人工描摹），属性 `height_m`、`mound_width_m`、`density`、`gap`、`vehicle_blocking`、`source`
- `data/maps/carentan/water.geojson` —— 增加洪泛区/沼泽多边形：`kind=flood_marsh`、`walkable_depth`（步兵可涉水深度）、`vehicle_ok=false`
- `data/maps/carentan/terrain_edits.geojson` —— 修正多边形：`target_elev_m` 或 `raise_m`、`transition_m`、`reason`（causeway|canal|remove_modern|embankment）
- `data/maps/carentan/roads.geojson`、`buildings.geojson` —— 删除/改线的现代要素（保留 `removed_reason` 到差分报告，不留在最终文件）
- `tools/map-pipeline/apply_edits.py` —— 把 terrain_edits 烘焙进高度图（M1-033 已有则扩展）；`tools/map-pipeline/diff_report.py` —— 输出差分报告（高度差统计、PNG 热图、删除要素清单）
- `data/maps/carentan/edits_report.md`（生成物，或存 `reports/`，按 M1-033 做法）
### 要实现的行为
1. 树篱：全图总长目标由人工给定（默认按 MAPS §3.7 的"西南区 > 15 km"）；脚本统计总长与分区长度，缺口比例，写入报告。
2. 洪泛区：多边形与 `water.geojson` 的水体逻辑区一致；洪泛区内的道路除堤道/桥外标记 `vehicle_ok=false`；堤道（N13）保持高于沼泽 ~1–2 m（用 terrain_edits 抬升）。
3. 去现代：新城区、超市、四车道改线等按人工清单删除；教堂 Notre-Dame（MAPS §3.4 A 级）等保存下来的建筑**保留**。
4. 应用 terrain_edits 后输出差分高度图与报告；应用是幂等且可复现的（输入哈希 → 输出哈希）。
5. 人工缺失时降级：树篱只描摹了部分区域则标记 `coverage` 并状态 `partial`，优先保证战线与据点周边（MILESTONES §3.8 风险缓解）。
### 接口与数据
- CLI：`map-pipeline edits --map carentan --apply`、`map-pipeline validate --map carentan --stage edits`
- `terrain_edits.geojson` 属性模式沿用 M1-033。

## 不做什么
- 建筑样式/可进入性/地标 → M2-003；据点与出生点 → M2-004。
- 水面渲染/涉水速度表现 → M2-035；涉水移速本身属 sim 移动规则，M1-011 已覆盖（MAPS §3.6 初值 ×0.5【待调参】）。
- 区块切片 → M2-005。

## 验收标准
1. 树篱总长与人工目标值偏差在 ±15% 内（`map-pipeline validate --stage edits` 打印总长/目标）；西南区（x<1100,y<900）长度 > 15 km，否则状态 `partial` 并写 PLAN_ISSUES。
2. 洪泛区多边形内水体逻辑区一致：抽样 200 点，`water.geojson` 判定与洪泛多边形判定不一致数 = 0；洪泛区内 `vehicle_ok=true` 的道路仅限堤道与桥。
3. 应用 terrain_edits 后，堤道路面高于相邻沼泽 ≥ 1.0 m（沿 N13 抽样 30 点）；差分报告含修改面积（m²）与最大绝对变化量。
4. 同一输入运行两次输出高度图 sha256 一致。
5. 全部 geojson 通过 schema（坐标 0..2048）；删除的现代要素清单已写入报告。

## 测试要求
- 单元测试：`pytest tools/map-pipeline/tests/test_edits.py`：过渡带（`transition_m`）插值边界、多边形重叠顺序、幂等性。
- 集成：对 M1 区域抽样，结果与 M1-033 产物一致。
- 性能基准：记录脚本耗时。
- 回归：更新 `hashes.json`。

## 性能预算
无（离线工具）。树篱 > 15 km 对运行时的预算影响由 M2-005/M2-036 承担（ARCH §9.6：树篱用样条 + instancing 段，碰撞用线性盒）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工项：树篱全图描摹（约 1–3 天，MAPS §7.1 T-HEDGE）、洪泛区（T-FLOOD，对照 NARA 航拍）；对应 NEEDS_HUMAN H-061。缺失时本任务 `partial`，不得编造树篱位置。
- NARA 航拍档案号、GSGS 图幅号在 MAPS §3.3 标"未核实"，由人类提供后再描摹。
- 默认假设（H-061）：游戏性精度优先于史实精度；树篱用样条 + instancing 段，描摹可降精度。
