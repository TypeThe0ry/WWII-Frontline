# M4-016 bots：新地图与新模式 bot 适配

## 元数据
- 里程碑：M4
- 工作流：bots
- 规模：M
- 前置依赖：M3-028, M4-011, M4-012, M4-005, M4-010
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §20.3; GAMEDESIGN §20.4; GAMEDESIGN §3.4; GAMEDESIGN §3.5; ARCHITECTURE §16.1; ARCHITECTURE §16.2; ARCHITECTURE §16.5

## 背景
bot 通过与真人同构的 `InputCmd` 路径行动（ARCHITECTURE §16.1），已有 Frontline（M2-027/028）与 Assault（M3-028）行为。本任务让 bot 能在 **Omaha（登陆艇、滩头）、Foy（森林、雪地）与 Skirmish 模式**下正常填充对局。
必须覆盖的行为需求（GAMEDESIGN §20.3 条目号）：B-01 导航（off-mesh：海堤台阶、悬崖攀爬点、残骸翻越、窗口）、B-02 占点、B-03 防守（`cover_points`，M4-003/008 提供）、B-17 使用重生网络（含登陆艇出生）、B-21 反堵塞（5 s 位移 < 1 m 即重规划）、B-22 遵守规则（无作弊）。
各场景要点：
- **Omaha 登陆艇**：bot 部署时选择 `Boat` 出生点（M4-011）；抵滩后有"冲出滩头"行为：向最近掩体（残骸/海堤）移动，避免原地停留；对障碍带使用已知障碍避让（`blocks_vehicle`/障碍实体）。
- **Foy 森林/雪地**：森林内导航（树干障碍），林缘散兵坑作为 `cover_points`；雪深区域载具减速（`snow_depth_zones.json`）；视觉感知距离受雾影响（能见度 450–600 m，难度表最大视距 150/220/300 m 为上限，GAMEDESIGN §20.6）。
- **Skirmish**：单据点；bot 数 ≤ 50（25v25）；无 Commander，故无 Commander bot，由小队长 bot 直接选目标；载具刷新点使用规则（初版 bot 不驾驶，M3-023 已有则沿用）。
- 性能：单局 bot 每 tick ≤ 3 ms（ARCHITECTURE §16.5 / GAMEDESIGN §20.5）；路径查询 ≤ 1.0 ms/tick。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/params/mod.rs` + `data/bots/maps/{omaha_dog_green,foy,carentan}.json` —— 每图/模式参数（`spawn_preference`、`beach_dash_s`、`min_cover_dist_m`、`stuck_timeout_s`、雪地减速系数引用、森林视距上限）；状态 `initial`。
- `crates/bots/src/goals/skirmish.rs` —— Skirmish 目标效用（占点/防守/回补）。
- `crates/bots/src/goals/beach.rs` —— 登陆艇上岸后的冲滩与掩体选择。
- `data/maps/*/offmesh_links.json` —— 补充缺失的 off-mesh（与 M4-005/010 协调，若缺则在此补并写 PLAN_ISSUES）。
- `crates/bots/tests/{omaha_landing.rs,foy_forest.rs,skirmish_fill.rs}`；`crates/headless-client` 增加 `scenario bot-match --map --mode`。
### 要实现的行为
1. 部署：bot 在 Omaha（Assault 攻方 US）使用 Boat 出生点；Skirmish 使用 HQ/Garrison；Foy 美军攻方使用 HQ、德军防守方使用 HQ/fallback。
2. 冲滩：落地后 `beach_dash_s`（初值 8 s【待调参】）内向 ≥ 30 m 外的最近可用掩体移动，随后转入 B-02/B-03。
3. 卡住检测 B-21：5 s 位移 < 1 m → 重规划 / 跳跃 / 切换目标；记录 `bot_stuck_total` 指标。
4. 路线：Assault 阶段切换时重新评估目标（M3-028 已有，复用）；Skirmish 目标仅据点。
5. 与真人公平：不使用未暴露敌人的位置（B-05）；雾/雪不增加视距。
6. 难度参数沿用 §20.6 三档；仅新增地图参数文件。
### 接口与数据
- `pub struct MapBotParams { pub beach_dash_s: f32, pub stuck_timeout_s: f32, pub forest_sight_cap_m: f32, pub prefer_cover_types: Vec<CoverType> }`，从 `/data/bots/maps/*.json` 载入。

## 不做什么
- 不做 LLM 指挥官的新地图提示（沿用 M3-027，必要时另立任务）。
- 不实现登陆艇驾驶（bot 仅作为载员）。
- 不做平衡调参（M4-039）。
- 不改 `sim` 规则；缺口写 PLAN_ISSUES。

## 验收标准
1. 三图 × 三模式（Carentan/Foy: Frontline、Assault、Skirmish；Omaha: Assault、Skirmish，Frontline 变体可选）各完成 1 局全 bot 对局（`balance-sim` 或 headless），**> 30 s 无进展计数 = 0**（`bot_no_progress_30s_total` 指标）。
2. `bot_stuck_total` 每局 ≤ 每 bot 平均 1 次/10 min；记录并列出 top 5 卡点位置。
3. Omaha 上岸后 30 s 内存活 bot 向掩体/前进的比例 ≥ 90%（`cargo test -p bots omaha_landing_dash` 统计）。
4. 3 ms/tick 预算：60 bot 的 `bots` 阶段 P99 ≤ 3.0 ms（`cargo bench -p bots tick_60`，参考机）。
5. 单测：同 seed 同输入 bot 决策确定性（1 万 tick `state_hash` 相同）。
6. Skirmish 50 bot（25v25）一局 25 min 内产生胜负（不超时无据点交替，`victory_reason` 非空）。

## 测试要求
- 单元测试：`cargo test -p bots`：冲滩掩体选择、Skirmish 效用曲线、卡住检测（合成轨迹）。
- 集成/无头测试：`balance-sim --map omaha_dog_green --mode assault --games 3 --bots 100`、`foy`/`carentan` 同理；检查无卡死指标。
- 性能基准：`cargo bench -p bots` 基线文件 `reports/perf/bots-m4.json`。
- 回归：确定性哈希基线；`DATA_HASH`。

## 性能预算
- bot 合计 ≤ 3.0 ms/tick（ARCHITECTURE §16.5）；路径查询 ≤ 1.0 ms/tick（§16.2）；服务器总预算 16.4 ms（§6.10）不得回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Foy/Omaha 地图数据若因许可证仅为占位地形，对局验收结论标 `partial`。
- 滩头开阔区 bot 易被机枪集中杀伤，导致 Assault 攻方胜率过低；调参留给 M4-039。
- 森林密集 navmesh 与 bot 避障（局部避障）是主要卡死来源，必要时调整 `agent_radius` 缓冲（参数在地图文件）。
