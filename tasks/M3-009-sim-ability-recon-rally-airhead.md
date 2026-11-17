# M3-009 sim：Recon Flight、Rally 与 Airhead

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 前置依赖：M3-006, M1-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §13.7; GAMEDESIGN §13.8; GAMEDESIGN §10.4; GAMEDESIGN §10.6; GAMEDESIGN §2.6; GAMEDESIGN §17.2; ARCHITECTURE §6.5.4

## 背景
三项非杀伤性指挥官能力，规则各异：Recon Flight 揭示敌人（**必须遵守 R3：只下发地图图标**），Rally 给范围内友军临时 buff，Airhead 创建临时重生点（接入 M1-019 的重生网络）。M3-006 提供通用流程。
参数（GD §13.2/§13.7/§13.8/§10.4，来自 `/data/abilities`）：
- Recon Flight：Fuel 60，冷却 240 s，延迟 18 s；条带 300 × 120 m（点选两点定义）；飞越 6 s 期间条带内所有存活敌人（含载具、迷彩 Sniper、OP）在本队地图上标记并实时更新，飞越结束后 12 s 内显示"最后已知位置"逐渐淡出；对敌无警告。
- Rally：Manpower 120，冷却 300 s，延迟 3 s；点选本方小队或地面点，半径 40 m 内本方玩家 45 s 增益：冲刺速度 +10%、体力消耗 ×0.7、压制累积 ×0.7、HP 恢复阈值 60→80。
- Airhead：Manpower 200，冷却 360 s，延迟 20 s（对敌方发提示音与 ±100 m 模糊环，5 s 前开始）；半径 25 m 的临时重生点，持续 120 s，容量 16 次出生，个人重生计时 5 s，3 s 出生保护，全队同时 1 个；放置限制：本方或战区 Sector，距任何敌方据点圆心 ≥ `R_sp+150 m`，放置时 60 m 内无已知敌人；15 m 内有敌即封锁。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/commander/effects/recon.rs` —— `ReconFlightEffect`：飞行体沿条带中线匀速飞越 6 s，每 tick 对条带内敌人生成/刷新 `MarkEntry{target, source=Recon, pos_q5m, expires_at}`。
- `crates/sim/src/commander/effects/rally.rs` —— `RallyEffect` + `RallyBuff` 组件；buff 修改项以乘数形式暴露给 M1-011（移动/体力）与 M1-017（压制）与 M1-016（回血阈值）的查询点。
- `crates/sim/src/commander/effects/airhead.rs` —— `AirheadEffect`：延迟到期创建 `SpawnPoint{kind: Airhead}`，接入 M1-019 的 `SpawnPointRegistry`（容量/寿命/封锁/出生保护）。
- `crates/sim/src/exposure.rs`（若 M3-010 尚未创建则先建，M3-010 扩展）—— 暴露条目表 `ExposureTable`：只存图标需要的信息。
- `crates/sim/tests/ability_recon_rally_airhead.rs`。
### 要实现的行为
1. **Recon 揭示**：敌人进入条带矩形即被标记；位置量化到 5 m；条目 1 Hz 刷新；飞越结束 12 s 内保留"最后已知位置"，随后移除。**不向客户端下发被揭示敌人的精确状态**（仍走 AOI 的 R3 过滤，ARCHITECTURE §6.5.4）；审计钩子 `exposure_audit()` 供 M3-010/M3-045 校验。
2. **Rally**：3 s 后对 40 m 内本方玩家（存活、未倒地）施加 45 s buff；buff 起止精确；离开范围**不**失效（buff 随人，GD 未规定则不绑定位置，写 PLAN_ISSUES）；重复 Rally 覆盖而非叠加；对敌不可见（不进 AOI 敌方视图）。
3. **Airhead**：到期创建临时重生点；120 s 后消失（已出生者不受影响）；容量用尽立即关闭；全队同时 1 个（再次下达被 M3-006 的通用规则或本任务的 `exists` 检查拒绝）；封锁规则 15 m 有敌；重生等待 5 s（GD §10.6 `Airhead = 5 s`）。
4. **放置合法性**：Airhead 的"60 m 无已知敌人"以**本队已知**敌人（标记/视野）为准，不得使用全知信息（防止"规则偷看"，与 R3 一致）。
5. 所有计时 tick 化，取消返还 50%。
### 接口与数据
- 暴露表：`ExposureEntry { target: EntityId, source: ExposureSource, pos_q5: (i16,i16), expires_at: Tick }`，由 `server` 通过 `WorldView::exposures(team)` 读取并编码为 `Marker` 事件（M3-005）。

## 不做什么
- 飞机渲染/警报声 → M3-030/M3-038；Spotter 标记与 OP → M3-010；Rally 头顶小旗渲染与 UI → M3-033/M3-029 范围外；
- 重生点网络本体（HQ/Garrison/Outpost）→ M1-019/M2-011。

## 验收标准
1. Recon：对 20 个随机放置的敌人，条带内 100% 被标记、条带外 0%；飞越 6 s + 12 s 的条目寿命误差 ≤ 1 tick。
2. Recon 审计：任何被揭示敌人的 `Marker` 位置量化步长 = 5 m（属性测试）；服务器侧该敌人的精确状态**未**因揭示而进入客户端快照候选集（单测 `aoi_candidates` 未新增）。
3. Rally：buff 起始延迟 3 s ±1 tick，持续 45 s ±1 tick；冲刺速度 +10%（误差 < 1%）、体力消耗 ×0.7（误差 < 1%）、压制累积 ×0.7、回血阈值 80。
4. Airhead：容量 16 次出生后关闭；120 s 到期；15 m 内有敌即封锁；个人重生计时 5 s 与 3 s 出生保护（与 GD §10.4/§2.6 一致）；全队第 2 个被拒。
5. Airhead 放置距敌方据点 `R_sp+149 m` 拒绝、`+151 m` 通过（边界单测）。
6. `state_hash` 两次运行一致；三项能力常态开销 ≤ 0.05 ms/tick（`criterion`）。

## 测试要求
- 单元测试：条带内/外判定（含斜向、含载具与隐蔽 Sniper）；buff 乘数；Airhead 生命周期；取消返还。
- 集成/无头：`headless-client` 以敌方视角验证 Recon 期间只收到图标事件、不收到精确位置；以己方视角验证 Airhead 出现在 `SpawnPointState`。
- 性能基准：`crates/sim/benches/recon_rally.rs`。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 5 规则 0.7 ms/tick：本任务活动期间 ≤ 0.1 ms，平时 0。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 暴露表在 M3-010（Spotter/OP）里扩展为完整的暴露规则矩阵；本任务只建 Recon 部分，避免与 M3-010 重复，若 M3-010 先行请在其基础上扩展。
- Rally 需要 M1-011/M1-017 暴露乘数查询点，若不存在则在本任务内补最小接口（不改其行为）。
- 隐蔽 Sniper/OP 被 Recon 揭示（GD §13.7）依赖 M3-010 的 Sniper 隐蔽状态；Sniper 隐蔽未实现前，Recon 对它们按普通敌人处理。
