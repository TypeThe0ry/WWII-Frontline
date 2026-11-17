# M4-004 地图：Omaha Assault 关卡标注

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：M
- 前置依赖：M4-003, M3-004
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §4.6; MAPS §6.2; MAPS §6.3; GAMEDESIGN §3.4; GAMEDESIGN §3.7; GAMEDESIGN §10.1; GAMEDESIGN §10.2

## 背景
Omaha 的默认模式是 **Assault（美军进攻）**，Frontline 不适配（GAMEDESIGN §3.7；MAPS §4.6 另给了 Frontline 对称变体，本任务只要求 Assault 为必交付，变体可选）。
关键规则：
- Assault：进攻方（US）从 L1 起逐线推进，防守方（GER）持有 L2–L5；仅"当前目标线"活动；占领后 +8 min 加时（上限总剩余 60 min）；初始 20 min；防守方有后撤 Rally 点 `fallback_spawn`（GAMEDESIGN §3.4）。
- MAPS §4.6 的 5 段方案：1 = L1a/L1b/L1c 任选其二；2 = L2a+L2b；3 = L3a D-1 + L3c D-3（拿下一个出口即可）；4 = L4a+L4b；5 = L5a+L5b。每线 2–3 候选，开局随机选 1（GAMEDESIGN §3.3.1）。
- HQ：US 为离岸 Landing Craft Anchorage (1500, 1950) ±100（**不可进入的水上出生区**，玩家由登陆艇送上滩）；GER HQ (900, 300) ±80。
- Garrison：海滩上不设；US Outpost 候选 (700,1400)、(1250,1300)；L3 占领后 Garrison 启用于 (1000,1100) 一线；GER Garrison 候选 Vierville 村 (70,1190)、高原农场 (900,300)。
- 火炮（R7）：玩家操作的榴弹炮是 HQ 附近预设阵地，坐标 MAPS 标"待定"，本任务补充；US 舰炮为框外叙事火力（Precision Strike 能力）。
- 载具：D-1 铺装路（Sherman/Crewman）、D-3 土路（视坡度）；其余仅步兵（坡度 > 35°）。
- 本任务的所有坐标为设计框架值，需经 M4-001 相似变换后吸附可站立点（≤ 60 m），记 `notes`。

## 具体内容
### 要创建/修改的文件
- `data/maps/omaha_dog_green/zones.json` —— `lines[L1..L5].points[]`（`id/name_key/origin[R|G]/pos/pos_error_m/radius_m/tags`）、`assault_stages[]`、`skirmish_areas[]`（S1–S3，MAPS §4.6 表）、`playable_polygon`、`out_of_bounds_buffer_m`、`attacker: "US"`、`initial_time_s: 1200`、`capture_bonus_s: 480`。
- `data/maps/omaha_dog_green/spawns.json` —— `hq`（含 `kind:"boat"` 的 US 出生区）、`boat_spawn_points[]`（到岸点沿 HWL 外分布，字段见下）、`garrison_candidates`、`outpost_candidates`、`fallback_spawn`（GER）、`observation_posts`。
- `data/maps/omaha_dog_green/artillery.json` —— 预设榴弹炮阵地（HQ 附近）与场外炮火。
- `data/maps/omaha_dog_green/vehicle_routes.json` —— D-1/D-3 路线与不可通行区（悬崖、冲沟）。
- `locales/en|zh/maps.omaha.json`（沿用 UI i18n 目录约定）—— 据点 `name_key` 文案（仅 [G] 原创名，如 Tide-Line Obstacles、Seawall Run；[R] 地名可用）。
### 要实现的行为
1. 将 MAPS §4.6 的 5 线 × 候选写入 `zones.json`；保证任意候选组合都能拼满地图（Sector 边界对齐固定"战线分界线"，GAMEDESIGN §3.3.1）。
2. `boat_spawn_points`：每个点含 `{id,pos_beach,pos_sea,craft_capacity:int,ramp_heading_deg,protect_s}`；US 登陆艇到岸点分布于 HWL 外 40–120 m 范围，间距 ≥ 40 m；到岸后的 3 s 出生保护按 M4-011 规则。
3. 攻防出生点互不可视：US 出生区与 GER HQ/Garrison/fallback 之间做视线检查（高度图射线，`LoS=false`）。
4. 每个 Assault 段目标可达：从 US 出生区经 navmesh 近似连通性（高度图坡度 ≤ 35° 的 8 邻域 flood fill）能到达该段所有候选点；至少 3 个步兵入口/点（MAPS §6.3）。
5. 为 Frontline 变体仅预留字段，不强制交付；若交付，须在 `zones.json.modes` 标注 `["assault","frontline_variant"]`。
### 接口与数据
- `zones.json.modes = ["assault"]`；`assault_stages[i] = {"stage":i,"primary":[...],"secondary":[...],"pick":"any_of_primary|n_of","n":2}`（第 1 段 n=2，第 3 段 n=1）。
- `spawns.json` 的 `kinds_supported` 含 `"boat"`（MAPS §6.2）。

## 不做什么
- 不实现登陆艇生命周期/出生保护规则（M4-011）。
- 不烘焙 navmesh、不切区块（M4-005）。
- 不做 Skirmish 子区的出生配置（M4-013，仅在此写 S1–S3 矩形）。
- 不做 UI 登陆艇出生界面（M4-023）。

## 验收标准
1. `map-pipeline validate omaha_dog_green` 通过：坐标 100% 在 0..2048、据点 0 个落入水体/实心建筑、HQ 之间可通行（US 到岸点→GER HQ 存在步兵连通路径）。
2. 5 个 Assault 段目标可达性检查：每段每个候选点从 US boat 出生点可达（flood fill），失败数 = 0；报告含最短路径长度（m）。
3. US boat 出生点与 GER 出生/Garrison/fallback 的视线检查：对 100% 点对 `LoS=false` 或距离 > 800 m（报告列出例外，须为 0）。
4. 每个据点至少 3 条步兵入口；强点半径在 35–70 m（GAMEDESIGN §3.6）；L3 候选间距 ≥ 300 m（GAMEDESIGN §3.3.1 作者约束）。
5. `artillery.json` 的预设阵地距各自 HQ ≤ 250 m【待调参】且不落水；每队 ≥ 1 个。
6. 所有 `name_key` 在 `en` 与 `zh` 文案文件中均存在；i18n 完整性测试通过。

## 测试要求
- 单元测试：`pytest tools/map-pipeline/tests/test_zones_validate.py`：对故意损坏的 `zones.json`（点落水、缺入口、半径越界）每类至少 1 个反例，校验器必须报错。
- 集成测试：沿用 `sim` 的 map loader 单测：加载 Omaha zones/spawns 不报错（`cargo test -p sim map_load_omaha`）。
- 性能基准：无。
- 回归：`hashes.json`；`DATA_HASH` 更新。

## 性能预算
无（数据任务）。加载时间由 M4-005 记录。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖 H-077、H-078（锚点人工校验）。默认假设：以设计框架暂定值继续，拓扑优先。
- GAMEDESIGN §3.7 写"Frontline 不适配"，MAPS §4.6 又给出 Frontline 变体：本任务以 GAMEDESIGN 为准（Assault 必交付，变体可选）；差异写 PLAN_ISSUES。
- 平衡风险：Assault 攻方胜率 35–65% 由 M4-039 验证；本任务的 `initial_time_s`、`capture_bonus_s` 取 GAMEDESIGN §3.4 初值。
