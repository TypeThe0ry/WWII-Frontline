# M4-013 地图：Skirmish 子区配置

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：S
- 依赖：M2-004, M4-009, M4-012
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §6.2; MAPS §3.6; MAPS §4.6; MAPS §5.6; GAMEDESIGN §3.5; GAMEDESIGN §3.7

## 背景
Skirmish 在地图上取一个子区域，单据点位于子区中央（GAMEDESIGN §3.5）。`zones.json` 已有 `skirmish_areas` 字段示意（MAPS §6.2：`{"id":"S1","rect":[x0,y0,x1,y1]}`）。本任务为 **Carentan 与 Foy**（M4-009 已有 S1–S3 矩形）补全每个子区的 HQ/Garrison 出生点、据点位置、出界边界与载具刷新点；Omaha 的子区（S1 D-1 出口 x0–800,y1150–1900；S2 悬崖与冲沟 x700–1500,y900–1500；S3 Vierville 高原 x0–900,y400–1300）如由 M4-004 已写矩形则同样补全，若 M4-004 未覆盖则本任务一并补全，并在回报里说明。
关键约束：
- 每子区：单据点中央，半径 60 m；两队 HQ 子区出生点（US/GER）距据点 ≥ 200 m 且互不可视；Garrison 候选 ≥ 3/队（上限 3）；Outpost 允许；无 Airhead。
- Foy 子区：S1 林缘散兵坑 x300–1200,y700–1100；S2 牧场与沟渠 x600–1500,y1000–1400；S3 Foy 村 x900–1600,y1350–1750（MAPS §5.6）。Carentan 子区范围取自 MAPS §3.6 的"Skirmish 模式子区域"表（每个约 700 × 700 m，24–40 人；执行时先阅读该表取矩形坐标）。
- 子区边界：硬墙/出界警告；`out_of_bounds_buffer_m` 沿用 64 m；边界避开水体/悬崖以免玩家被困。
- 小规模：16v16 为默认，路径过长会导致节奏慢——两队 HQ 到据点步行 ≤ 90 s（按 sprint 6 m/s 估 ≈ 540 m 上限，初值，【待调参】）。

## 具体内容
### 要创建/修改的文件
- `data/maps/{carentan,omaha_dog_green,foy}/zones.json` —— 补全 `skirmish_areas[]`：`{id, rect, sp_pos, sp_radius_m:60, boundary_polygon}`。
- `data/maps/{...}/spawns.json` —— 增加 `skirmish[area_id].{hq:{US,GER},garrison_candidates[],outpost_candidates[],vehicle_spawns[]}`。
- `locales/{en,zh}/maps.*.json` —— 子区显示名 key。
- `tools/map-pipeline/map_pipeline/validate/skirmish.py` —— 子区专用校验。
### 要实现的行为
1. 为每个子区选择据点位置（子区中央，吸附到可站立开阔地 ≤ 60 m），写 `sp_pos`。
2. 设计两队 HQ 出生点：沿子区对角方向，间距按"步行 ≤ 90 s"校验；与据点、对方出生点互不可视（高度图 LoS 检查）。
3. Garrison/Outpost 候选：每队 ≥ 3 个 Garrison 候选；位置满足 GAMEDESIGN §10.2 间距（距己方 ≥150 m 可能过大于子区——此处按子区尺度写 `min_spacing_m` 覆盖并记 PLAN_ISSUES，初值 60 m【待调参】）。
4. 载具刷新点：轻型装甲、中型坦克、运输车各 1，位于己方 HQ 附近，坡度/路网可达（Foy 雪地、Carentan 洪泛区避开沼泽）。
5. 输出每个子区的"可视化 PNG 与统计报告"（`reports/m4-skirmish-areas.json`）。
### 接口与数据
- `skirmish_areas[i]`：`{"id":"S1","rect":[x0,y0,x1,y1],"sp_pos":[x,y],"sp_radius_m":60,"boundary_polygon":[...],"mode_hint":"skirmish"}`。

## 不做什么
- 不实现 Skirmish 规则（M4-012）。
- 不做 matchd 轮换/随机选择子区（M4-031）。
- 不改动 Frontline/Assault 的 `lines` 数据。
- 不做新子区的区块/navmesh 烘焙（沿用全图 navmesh，子区靠边界约束）。

## 验收标准
1. `map-pipeline validate` 对三张图通过；每个子区 `rect` 在 0..2048 内、不与水体边界重叠（重叠比例 ≤ 5%，且据点与出生点 0 个落水）。
2. 每个子区两队 HQ 出生点互不可视（LoS=false 或距离 > 600 m），且到据点的步兵路径存在（flood fill 100%）。
3. 每个子区 HQ→据点的步行时间估算 ≤ 90 s（报告逐子区数值）。
4. 每队 Garrison 候选 ≥ 3，满足本子区 `min_spacing_m`；载具刷新点可达率 100%。
5. 全部 `name_key` 在 en/zh 存在；i18n 完整性测试通过。

## 测试要求
- 单元测试：`pytest test_skirmish_validate.py`：对反例（据点落水、出生点可视、候选不足、边界越界）逐一报错。
- 集成测试：`cargo test -p sim map_load_skirmish_areas`：加载三图所有子区不报错；与 M4-012 的 `skirmish_score_win` 复用任一子区配置。
- 性能基准：无。
- 回归：`hashes.json`、`DATA_HASH`。

## 性能预算
无（数据任务）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 依赖未列 M4-004/M4-005，但若要覆盖 Omaha 子区则隐含依赖 M4-004；本任务默认只强制覆盖 Carentan + Foy（INDEX 范围），Omaha 子区为可选，若 M4-004 已写入矩形则补全出生点。
- GAMEDESIGN §3.7 不列 Omaha/Foy 的 Skirmish，而 MAPS 列出；以"数据存在、模式开关由 matchd 决定"处理。
- Garrison 间距规则（≥150 m）在小子区不可行，需要新增 per-area 覆盖，写 PLAN_ISSUES 请 Fable 确认。
