# M3-024 bots：Spotter 与 Sniper bot

## 元数据
- 里程碑：M3
- 工作流：bots
- 规模：M
- 前置依赖：M2-029, M3-010, M3-011
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §20.3; GAMEDESIGN §17.1; GAMEDESIGN §17.2; GAMEDESIGN §17.3; GAMEDESIGN §10.7; ARCHITECTURE §16.3

## 背景
GD §20.3 B-13：Sniper/Spotter bot 前往 OP 位置并建 OP；Spotter 对视野内敌人标记；Sniper 长距离射击并转移（射击 2 次后换位）。B-05：**无透视**——只能基于视觉（视锥、LoS、可见度）与听觉；反应时间按难度（GD §20.6）。B-07：Spotter bot 在前线供给充足时放置 Garrison（R1：放置者为 Officer/Spotter/Commander，Engineer 不放）。
前置：M3-010 提供 OP 放置规则与暴露/标记系统，M3-011 提供狙击武器、屏息与瞄具；M2-029 已给出 bot 感知缓存与分片框架。Recon Team ≤ 2 人（1 Spotter + 1 Sniper，R2），OP 仅该 Recon Team 使用，周围 40 m 无敌、`cover_zone` 内、趴/蹲放置 10 s。
bot 只能输出与真人同构的 `InputCmd`（标记操作按住 1.2 s、OP 放置、屏息、瞄具分划切换均走既有输入位）。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/recon/op_site.rs` —— OP 选址：候选集 = 地图 `cover_zone` ∩ 距己方当前活动据点 120–400 m ∩ 视野可见据点的高点（地图数据 `observer_points`，M3-004）；过滤 M3-010 的放置规则（40 m 无敌、距敌据点 `R_sp+40 m`、距 Garrison ≥ 60 m）；效用评分 = 对活动据点的视线覆盖 + 隐蔽度 + 离己方出生点距离。
- `crates/bots/src/recon/spotter.rs` —— Spotter 行为树：前往 OP 点→趴下放置 OP（10 s）→观察（扫描视野）→对 LoS 内、≤ 800 m 的敌人按 B-05 感知规则挑选目标并标记（按住 1.2 s，稳定准星）→每次标记后冷却 3 s；优先级：载具/AT 炮/火炮 > 步兵；对 Fire Mission 观察者角色：给炮兵发修正（与 M3-025 对接）。
- `crates/bots/src/recon/sniper.rs` —— Sniper 行为树：占据狙击位（距活动据点 250–600 m，开阔射界，`cover_points`）→趴下静止 ≥ 3 s 隐蔽（M3-010 的 ×0.56）→扫描→识别目标（非全知，使用视觉识别概率）→屏息（M3-011：4 s 窗口）→校正零点分划→射击→**射击 2 次后换位**（B-13）。
- `crates/bots/src/recon/team_coord.rs` —— 与队友协同：Sniper 保持在 Spotter 100 m 内以获得 sway ×0.8（GD §17.3）；Spotter 标记目标时通知 Sniper 优先；共享 OP 出生。
- `crates/bots/src/recon/perception.rs` —— 识别概率：基于距离/姿态/迷彩/静止时间/光照（`visibility_factor`，M3-010），Recruit/Regular/Veteran 视距 150/220/300 m（GD §20.6），迷彩 ×0.7、静止 ×0.8。
- `crates/bots/tests/recon_bots.rs`。
### 要实现的行为
1. 部署：bot 兵种选择在 Recon 小队中取 Spotter+Sniper（M1-039 的选择器扩展）；Recon 小队仅当 Commander bot 判断需要或真人认领时开启（GD §4.7）。
2. OP：每局每个 Recon Team 至少尝试放置 1 个 OP；放置失败（被敌封锁）后 20 s 内换点；OP 被毁后遵守 150 s 冷却。
3. 标记：对被标记者无任何额外信息（避免作弊）；标记仅在 LoS 与 800 m 内，且 bot 视觉已"识别"该目标。
4. 狙击：只对"已识别"的敌人射击；瞄具分划依据估计距离（带误差 ±5%，Veteran ±2%）；射击后按 B-13 换位；开火后 5 s 隐蔽恢复。
5. 安全：被压制/受伤/敌方接近 30 m 内时撤退到队友。
### 接口与数据
- `BotRole::{Spotter,Sniper}` 注册到 `behavior_tree` 工厂；`WorldView` 只读（含已识别目标列表，由 `perception.rs` 产出）。

## 不做什么
- Fire Mission 的炮兵响应 → M3-025；载具 bot → M3-023；指挥官能力 → M3-026；
- 真人 Spotter/Sniper UI → M3-034；暴露规则/标记 sim 逻辑 → M3-010。

## 验收标准
1. **bot Spotter 每局放置 OP ≥ 1**：`balance-sim` 20 局（Frontline，Carentan），≥ 90% 的有 Spotter 的局中至少放置 1 个 OP（报告 `reports/m3-recon-bots.json`）。
2. **狙击 bot 非全知**：审计测试——bot 在无 LoS 或超视距时对目标的开火次数 = 0；使用迷彩目标（距离 200 m、静止 ≥ 3 s）被识别的概率与 `visibility_factor` 的期望差 ≤ 10 个百分点（1 000 次采样）。
3. Spotter 标记：每个 bot Spotter 每分钟标记次数 ≥ 1（有可见目标时），被标记目标 100% 满足 LoS 与 ≤ 800 m（审计 0 违规）。
4. 换位：Sniper 每射击 2 次后在 ≤ 20 s 内位移 ≥ 30 m（单测+统计，成功率 ≥ 90%）。
5. 屏息使用：射程 > 300 m 的射击中屏息使用率 ≥ 80%（统计），分划误差使 800 m 目标命中率 ≥ 与零散布解析期望的 60%（统计，信息性）。
6. 性能：10 个 Recon bot 的 `think` 合计 ≤ 0.2 ms/tick（`criterion`；ARCHITECTURE §16.5 预算内）。
7. 合规：`anticheat_reject_total` = 0（10 分钟）。

## 测试要求
- 单元测试：OP 选址过滤、识别概率、换位触发、标记 LOS/距离守卫、行为树 smoke。
- 集成/无头：`balance-sim` 20 局；服务器 + `headless-client` 观察 bot OP 事件序列。
- 性能基准：`crates/bots/benches/recon_think.rs`。
- 回归：bot 行为轨迹哈希（信息性）。

## 性能预算
ARCHITECTURE §16.5：bot 总预算 ≈ 3.0 ms/tick；Recon bot ≤ 0.2 ms；感知射线计入既有预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- "识别概率"曲线 GD 只给视距与迷彩系数，具体曲线是本任务"细化"，写 PLAN_ISSUES。
- 若 `cover_zone` tag 缺失（M2-004），OP 选址退化为林地/废墟多边形（MAPS §1.6 的 forests/buildings）。
- Sniper bot 的命中率受 M3-011 的 sway/屏息影响，调参归 M3-047。
