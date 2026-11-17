# M4-007 地图：Foy 高度图与矢量

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：L
- 依赖：M4-006, M2-001（Carentan 全图 DEM/矢量管线）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §5.2; MAPS §5.5; MAPS §1.2; MAPS §1.5; MAPS §1.6; MAPS §6.1; ARCHITECTURE §9.1

## 背景
产出 Foy 的 2049² 1 m 高度图、8 m 远景环、森林/道路/村庄矢量，并去除 1945 之后的构筑物。Ardennes 高差约 30–60 m，16-bit 足够（MAPS §9.1 注）。
关键约束：
- 高度图流程同 MAPS §1.2：瓦隆 MNT 0.5 m → `gdalwarp -r average` → 1 m；`hmin/hmax` 写 `meta.json`；256 m 缓冲；远景环 8 m 1025²（覆盖 8192 m，含南 Bastogne、北 Noville/Houffalize、东 Bourcy、西 Recogne/Longvilly 方向，MAPS §5.2）。
- 1944/45 修正（MAPS §1.5）：删除 1945 之后的农场扩建/现代住宅；N30 改线检查；林间现代路径去除；"无 bocage，有林带与田间矮篱"；小溪/冻结池塘；炮坑可作贴花（视觉，非高度）。
- 许可证门禁：瓦隆数据 PENDING 时不得入库（M4-006）；此时只能用 GLO-30 占位 + 手工微地形，`placeholder: true`，质量显著下降，不得标 done。
- 矢量：OSM 初稿 + 瓦隆 PICC/历史图描摹；属性按 MAPS §6.2（`roads.class/width_m/surface/vehicle_ok`，`forests` 的 `density/species`，`buildings.style/levels/roof/enterable`）。建筑风格为阿登农舍（石砌 + 木梁，长条院落，MAPS §5.5）。
- 森林：针叶人工林（冷杉/云杉）+ 落叶树（山毛榉）；密度分区写 `forests.geojson`，但最终实例上限校验在 M4-008。

## 具体内容
### 要创建/修改的文件
- `data/maps/foy/heightmap_1m.r16`、`heightmap_far_8m.r16`、`meta.json`（补全 hmin/hmax/datum）。
- `data/maps/foy/{roads,water,buildings,forests,hedgerows}.geojson`；`terrain_edits.geojson`。
- `tools/map-pipeline/map_pipeline/stages/{dem,vectors_osm,modern_removal}.py` —— 复用阶段，增加 EPSG:3812→25831 重投影与"现代构筑物剔除"规则。
- `reports/m4-foy-geo.json` —— 锚点高程误差、各文件 sha256、剔除统计。
### 要实现的行为
1. 读取 `/assets/incoming/maps/foy/` 的 LiDAR；缺失 → 任务 `blocked`（README §6），不得合成真实数据。
2. 重投影与重采样，输出高度图与远景环；远景外圈用 GLO-30。
3. 现代构筑物剔除：基于 OSM `building` 创建时间不可得，故以人工多边形清单 `modern_removals.geojson` 驱动（候选由脚本按"无 1945 参考证据"列出，人工确认）。
4. 道路分级（N30 主干道冰雪覆盖属材质，不在矢量；`class=paved/dirt/path`）；`vehicle_ok` 依 `width_m ≥ 3.5` 与坡度 ≤ 25°。
5. 村庄建筑足迹按"长条院落沿路"规则校验：沿 N30/支路两侧；`enterable` 比例控制（性能）≤ 30%【待调参】。
6. 输出锚点高程误差报告与哈希。
### 接口与数据
- 与 Omaha 相同的 `terrain_edits.geojson` schema；`forests.geojson` 特征 `{"density":0..1,"species":{"spruce":x,"beech":y},"snow_cover":true}`。

## 不做什么
- 不布置散兵坑/工事、不细分森林密度分布（M4-008）。
- 不写 zones/spawns（M4-009）、不切区块（M4-010）。
- 不渲染雪地/天气（M4-015）。
- 不使用历史图像做贴图。

## 验收标准
1. 高度图文件大小精确 8 396 802 B（2049²×2），远景环 2 101 250 B；`map-pipeline validate` 通过。
2. 锚点位置高程与源 MNT 采样差 ≤ 1 m（`reports/m4-foy-geo.json` 逐锚点列出）。
3. 两次运行输出 sha256 完全一致（"哈希可复现"）。
4. 全部矢量坐标位于 0..2048；几何有效性 100%；`buildings` 与 `roads` 无重叠相交面积 > 1 m²。
5. N30 在可玩区内连通（无断裂段，polyline 连通分量 = 1）。
6. 报告列出剔除的现代构筑物数量与人工确认状态；未确认数 = 0 或有 PLAN_ISSUES 编号。

## 测试要求
- 单元测试：重投影往返误差 < 0.01 m（EPSG:3812→25831→3812）；量化/反量化误差 < 1 mm；现代剔除规则对合成数据的判定。
- 集成测试：Carentan 小样本端到端冒烟（CI 不下载大数据）；真实数据在人工机器运行。
- 性能基准：全流程耗时与内存峰值记录（非阻断）。
- 回归：`hashes.json` 由 CI 校验。

## 性能预算
- 高度图 8.4 MB、远景环 2.1 MB（ARCHITECTURE §9.1）；无运行时预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工输入：DEM 下载、现代构筑物剔除确认、H-072（许可证）。H-072 未回复 → 瓦隆数据不入库，本任务只能产出占位地形并标 `partial`。
- 许可证 PENDING 下产出的占位地形（GLO-30 30 m）无法满足 1 m 精度与锚点 ≤ 1 m 验收，对应验收项记 `未测`。
- 气象/雪深数值未核实（MAPS §5.5），只在 M4-015 作为参数，不在此处体现。
