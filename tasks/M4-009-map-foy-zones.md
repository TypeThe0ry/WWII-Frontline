# M4-009 地图：Foy 关卡标注（Frontline 与 Assault）

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：M
- 前置依赖：M4-008, M2-004
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §5.6; MAPS §6.2; MAPS §6.3; GAMEDESIGN §3.3; GAMEDESIGN §3.4; GAMEDESIGN §3.7; GAMEDESIGN §10.2; GAMEDESIGN §10.5

## 背景
Foy 同时支持 **Frontline** 与 **Assault（美军进攻，总控裁定）**（GAMEDESIGN §3.7、§3.4；MAPS §5.6）。本任务把 MAPS §5.6 写成 `zones.json/spawns.json/artillery.json/vehicle_routes.json`。
关键内容（MAPS §5.6）：
- HQ：US (700,250) ±80（Bois Jacques 南部林内空地）；GER (1350,1900) ±80（村北、N30 沿线农场）。
- Frontline：5 线 × 2–3 候选：L1 森林南部（L1a Rear Clearing / L1b Fir Plantation / L1c South Road Cut）、L2 林缘北缘（Foxhole Line West/East、Log Dugouts）、L3 牧场田埂（Snow Meadow Fence / Drainage Ditch / Roadside Barn）、L4 Foy 村（Foy Crossroads / Chapel Yard / Long Farm）、L5 北部道路（North Road Bend / Ridge Farm / Forest Edge North）；半径 45–55 m。任意候选组合可拼满地图；L3 候选间距 ≥ 300 m；同线路径距离差 ≤ 25%（GAMEDESIGN §3.3.1）。
- Garrison 候选：US (700,250)、(450,850)；GER Long Farm (1300,1450)、北农场 (1000,1850)。
- Assault：**美军进攻**（总控裁定：史实上 1945-01-13 美军 506th PIR 攻 Foy，以 MAPS §5.6 为准；GAMEDESIGN §3.4/§3.7 同步修订为 US 攻）。`attacker="US"`，阶段线序即 MAPS §5.6 的 L1→L5（`assault.line_order:["L1","L2","L3","L4","L5"]`）；GER 为防守方。
- 火炮（R7）：玩家操作榴弹炮为 HQ 附近预设阵地，坐标本任务补充；US 105 mm 在框外南、GER 迫击炮/榴弹炮在框外北为场外叙事火力。
- 载具：N30（冰雪减速）与林内路堑 L1c；牧场雪地可通行但陷入概率（待调参）。

## 具体内容
### 要创建/修改的文件
- `data/maps/foy/zones.json` —— `lines[]`、`modes: ["frontline","assault"]`、`assault:{attacker:"US",line_order:["L1","L2","L3","L4","L5"],stages:[…]}`、`skirmish_areas`（S1–S3，MAPS §5.6）。
- `data/maps/foy/spawns.json`、`artillery.json`、`vehicle_routes.json`。
- `locales/{en,zh}/maps.foy.json` —— `name_key` 文案（[G] 原创名；[R] Foy/N30 等可用）。
### 要实现的行为
1. 写入 Frontline 5 线 × 候选，含每候选对应的 Sector 多边形边界（对齐固定战线分界线）。
2. 写入美军 Assault 阶段：按 MAPS §5.6 的 5 段（林缘出击→牧场→村庄→礼拜堂清扫→追击）顺序写入，`notes` 说明各段与 Frontline 线/Sector 的对应；不得改变拓扑。
3. 设置 `initial_time_s`、`capture_bonus_s` 取 GAMEDESIGN §3.4 初值；`fallback_spawn`（GER 防守方后撤点）取北部农场 (1000,1850) 附近并校验可站立。
4. 校验：HQ 间可通行；每据点 ≥ 3 条步兵入口；US/GER 出生点互不可视；两种模式各自完整（`validate --mode frontline|assault` 各通过）。
5. 记录 `pos_error_m` 与吸附记录（≤ 60 m）。
### 接口与数据
- 与 Omaha 同一 `zones.json` schema（`frontline.map.zones/1`）；新增字段只增不改，并同步 `/data/schemas/maps` 与 DATA_HASH。

## 不做什么
- 不切区块/烘焙 navmesh（M4-010）；不做 Skirmish 出生配置（M4-013）。
- 不做天气/雪地渲染（M4-015）；不做 bot 参数（M4-016）。
- 不裁定 Foy 参战德军番号或日期（H-078，文案不写）。

## 验收标准
1. `map-pipeline validate foy --mode frontline` 与 `--mode assault` 均通过（两种模式各有完整配置）。
2. Frontline：任意随机抽取 200 组候选组合，Sector 拼满地图覆盖率 = 100%（无缝隙/重叠 > 1 m²）；L3 候选两两距离 ≥ 300 m；同线路径距离差 ≤ 25%（报告）。
3. Assault（US 攻）：从 US HQ 出发，每个阶段目标的 flood-fill 可达率 100%；阶段数 5；`attacker="US"` 单测断言。
4. 每据点 ≥ 3 条步兵入口；强点半径落在 35–70 m。
5. US/GER 出生点（含 Garrison 候选）两两无视线或距离 > 600 m【待调参】（报告例外 = 0）。
6. 全部 `name_key` 在 en/zh 中存在（i18n 完整性测试通过）。

## 测试要求
- 单元测试：zones 校验器对反例（缺入口、候选间距不足、Sector 缝隙、attacker 缺失）逐类报错。
- 集成测试：`cargo test -p sim map_load_foy`（加载两种模式配置）；`cargo test -p sim assault_foy_stage_order`。
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
- Foy Assault 攻方已裁定为美军（以 MAPS §5.6 与史实为准）；GAMEDESIGN §3.4/§3.7 由文档编辑同步，本任务只按此实现。
- 依赖 H-078 锚点与 H-079；默认以设计框架暂定值继续、拓扑优先。
- 防守方（GER）的 fallback 后撤点若与 Frontline L4/L5 Sector 重叠，需检查 Garrison 不可建规则（GAMEDESIGN §3.4）。
