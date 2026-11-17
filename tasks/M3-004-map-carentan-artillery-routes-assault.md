# M3-004 地图：Carentan 炮兵阵地、载具路线与 Assault 测试配置

## 元数据
- 里程碑：M3
- 工作流：maps
- 规模：M
- 前置依赖：M2-004
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：MAPS §6.1; MAPS §6.2; MAPS §3.6; MAPS §3.4; GAMEDESIGN §15.1; GAMEDESIGN §15.2; GAMEDESIGN §3.4; GAMEDESIGN §3.7

## 背景
M2-004 已产出 Carentan 的 `zones.json`（5 条战线）、`spawns.json`、`vehicle_routes.json`（初稿）与 `artillery` 占位阵地。M3 需要：①R7 的**预设榴弹炮阵地**（HQ 附近，每队 2–3 个，开局即存在）与观测点，②补全载具路线（M3-023 的驾驶 bot 要用），③在 `zones.json` 内加 Assault 测试用 `assault_stages`，仅用于 M3 模式验证。Assault 的正式地图是 Omaha（M4-004）；Carentan 默认模式仍为 Frontline（GD §3.7）。
MAPS §3.6 提到的"框外/不可进入"火炮属场外叙事火力，**与玩家操作的预设阵地不是同一回事**；预设阵地坐标 MAPS 标"待定"，本任务补充。

## 具体内容
### 要创建/修改的文件
- `data/maps/carentan/artillery.json` —— `sites[]`：每队 2–3 个预设阵地 `{id, team, type: "howitzer", pos[x,z], facing_deg, charge_range_cover[], observer_points[], overrun_radius_m: 15}`；另有 `offmap_fire[]`（叙事火力，仅供视觉/音频）。US 阵地靠近 HQ (250,1750)，GER 靠近 HQ (1850,250)（MAPS §6.2 的 spawns 示例坐标），距 HQ ≥ 60 m、≤ 250 m，且在 HQ 保护区（150 m）外或边缘。
- `data/maps/carentan/vehicle_routes.json` —— 补全：每队 HQ 载具刷新点（轻 2、中 2、重 1、运输 3，GD §14.6）到 L1–L5 各据点的路网路点图 `{nodes[{id,pos}], edges[{a,b,width_m,surface,vehicle_ok}], no_go[]}`；树篱标 `vehicle_blocking`；桥梁/运河桥为瓶颈节点。
- `data/maps/carentan/zones.json` —— 新增 `assault_stages[]`：4 阶段 `{stage, primary[], secondary[], time_limit_s}`，攻方 US，沿 L1→L4（具体 ID 取 zones 中已有 point id）；`fallback_spawn`（防守方后撤点，GD §3.4）。
- `tools/map-pipeline/` —— 新增/扩展 `validate`：炮兵覆盖报告、载具路线连通性检查；输出 `reports/m3-artillery-coverage.json`（不含录像/素材）。
### 要实现的行为
1. 每个预设阵地 `charge_range_cover`：三档装药射程环（800/1800/3200 m，GD §15.1）对**各 Strongpoint 圆心**的可达性矩阵；报告每个据点被至少 1 个本队阵地覆盖的装药档（含 `R_min` 盲区 139/313/556 m）。
2. 阵地数 = 每队 2–3；`observer_points` 为可看到至少 1 个据点的高点/OP 候选（含教堂塔楼，MAPS §3.4）。
3. 载具路线从每个 HQ 载具位出发到每个据点至少 1 条 `vehicle_ok` 路径，且最短路径长度/直线距离 ≤ 2.0（报告列出超限者，不超限为目标）。
4. `assault_stages` 阶段目标可达：每阶段 primary 据点对攻方有 ≥ 3 条步兵入口（沿用 MAPS §6.3 校验）；攻守出生点互不可视（视线检查）。
5. 坐标全在 0..2048；阵地不落水体/实心建筑。
### 接口与数据
- JSON schema 版本：`frontline.map.artillery/1`、`frontline.map.vehicle_routes/1`（在 MAPS §6.2 基础上追加，PLAN_ISSUES 记"细化"）。
- `sim` 的 `GameData` 加载 `artillery.json`（M3-018 使用）；路线由 `bots`（M3-023）读取。

## 不做什么
- 炮兵规则代码 → M3-018/019；Assault 规则 → M3-020；载具驾驶 bot → M3-023。
- Omaha/Foy 关卡 → M4-004/M4-008。
- 不修改 Frontline 的据点布局与默认模式。

## 验收标准
1. `map-pipeline validate carentan` 退出码 0（schema、坐标范围、阵地不在水体/建筑内）。
2. 每队预设榴弹炮阵地数 ∈ {2,3}；`reports/m3-artillery-coverage.json` 显示：每个 Strongpoint 至少被本队 1 个阵地以 ≤ 装药 3 覆盖，且至少 70% 据点同时被两个阵地覆盖（若不满足，报告列出并写 PLAN_ISSUES）。
3. HQ→全部据点的载具路径连通率 = 100%；路径/直线比 ≤ 2.0 的据点占比 ≥ 80%。
4. `assault_stages` 共 4 段；每段 primary 据点步兵入口数 ≥ 3；攻守出生点互不可视（检查脚本输出 true）。
5. 所有 `observer_points` 对各自目标据点有 LoS（地形射线检查，通过率 100%）。

## 测试要求
- 单元测试：validate 的新规则各带 1 个反例（阵地在水中、路线断裂、阶段引用不存在的据点）。
- 集成：`cargo test -p sim map::artillery_load` 加载 JSON 并断言数量。
- 性能基准：无。
- 回归：地图数据哈希写入 `meta.json`（MAPS §6.3 要求）。

## 性能预算
无。数据体量：新增 JSON 合计 ≤ 200 KB（不得使 ARCHITECTURE §11.4 体积预算回归）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 阵地的"最佳位置"需要关卡设计判断（MAPS 把它标"待定"）；若 Fable/人类希望人工定稿，可保留候选并标 `needs_review`，本任务以通过 validate 的程序化选址为准。
- 阵地被毁后的重置规则在 GD §15.2 标【待调参】，本任务只存位置，不存规则。
- 选址须避开 MAPS §3.6 叙事火力占用的"草地炮位 L5b"，以免与玩家阵地语义混淆。
