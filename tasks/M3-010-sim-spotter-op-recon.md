# M3-010 sim：Spotter 标记、OP 与暴露规则

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：L
- 依赖：M3-001, M2-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §10.7; GAMEDESIGN §17.1; GAMEDESIGN §17.2; GAMEDESIGN §17.3; GAMEDESIGN §16.3; ARCHITECTURE §6.5.4; ARCHITECTURE §15.2; BRIEF R3

## 背景
这是 R3 的规则落点：敌方实体仅在「≤ 400 m 且（在玩家视锥 + 30° 余量内 或 ≤ 96 m）」时才下发精确状态；被友军标记（Spotter/Officer/Commander）或 Recon Flight 暴露的敌人**只下发地图图标**（1 Hz、位置量化到 5 m、带有效期）。M2-019 已实现标记/Ping 权限与频率规则；M3-001 已提供 Spotter/Sniper 数据；M3-009 先建了 Recon 的暴露表。本任务完成 **OP（Observation Post）放置/出生**、**精确标记**、**暴露规则矩阵**与 **Sniper 隐蔽**。
OP 规则（GD §10.7，R1：由 Spotter 放置）：周围 40 m 无敌；距敌方据点圆心 ≥ `R_sp+40 m`；距友方 Garrison ≥ 60 m；趴/蹲姿放置，放置 10 s；位置须在 `cover_zone` 内；使用者仅该 Recon Team（2 人）；重生计时 15 s；HP 150，枪弹 ×0.3；敌在 12 m 内封锁，敌在 12 m 内且友 0 人持续 10 s 销毁；寿命 15 min，被毁后冷却 150 s；对敌隐藏，对本队小队长/Commander 可见。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/recon/op.rs` —— `ObservationPost` 实体、放置校验、出生接入 M1-019 `SpawnPointRegistry`（kind=`Op`，base_wait 15 s，无出生保护，GD §2.6）。
- `crates/sim/src/recon/mark.rs` —— `MarkSession`：Spotter 按住 1.2 s（Officer 1.8 s）、准星稳定（视角角速度 < 阈值，初值 15°/s【待调参】）、最大距离 800 m（Spotter）/500 m（Officer）、须 LOS；完成后写入 `ExposureTable`；显示时长：步兵 8 s、载具 15 s（视线内实时更新、丢失后显示最后位置）、火炮/AT 炮/建筑 60 s；重复标记刷新计时；上限（GD §16.3）Spotter ≤ 10、Officer ≤ 5、Commander ≤ 20；Spot Assist 计分钩子（击杀归属，GD §18）。
- `crates/sim/src/exposure.rs` —— 扩展 `ExposureTable` 为完整矩阵：来源 {Mark, Recon, MuzzleFlash（坦克炮/AT 炮/火箭，5 s、误差 25 m、350 m 内敌方）, Howitzer（6 s、误差 40 m、敌方全队地图）, ScopeGlint（见下）}；`WorldView::exposures(team)`；`exposure_audit()`。
- `crates/sim/src/recon/concealment.rs` —— Sniper 隐蔽：被 bot 视觉检测距离 ×0.7；趴姿静止 ≥ 3 s 再 ×0.8（叠加 ×0.56）；开火后 5 s 恢复 ×1.0；对人类仅降低模型对比度（输出 `visibility_factor` 供渲染，不改命中盒）；Scope Glint：Sniper 开镜正对敌人 ±25° 且 ≤ 400 m 时每 0.5 s 一次短暂光点事件。
- `crates/sim/tests/recon_exposure.rs`。
### 要实现的行为
1. OP 放置：校验全部条件，任一不满足返回具体 `OpReject` 原因（给 UI，M3-034）；放置过程可被伤害/移动打断；放置者死亡时未完成的 OP 取消。
2. OP 出生：仅该 Recon Team 成员可选；Spotter 与 Sniper 共享（GD §17.3）；Overrun/封锁/寿命/冷却按表。
3. 标记：对被标记者**不提示**；标记结果写入 `ExposureTable`，**只产生地图图标事件**（5 m 量化，1 Hz）；标记者可见范围 = 全队（Spotter/Commander）/小队+Commander（Officer）。
4. 暴露矩阵：对任何敌人，除"直接视觉"（客户端按 R3 AOI 已有）与上述来源外，服务器不得向对方下发其精确状态；提供审计函数返回"本 tick 本客户端额外获知的敌方信息清单"，要求其只含图标类。
5. 友军协同：Spotter 与 Sniper 100 m 内时 Sniper `sway ×0.8`（GD §17.3，输出修正因子供 M3-011 消费）。
### 接口与数据
- `OpRejectReason { EnemyNear, TooCloseEnemySp, TooCloseGarrison, BadStance, NotCoverZone, Cooldown, Cap }` 与 `protocol` 的 Notice code 对应（M3-005 的 `OpPlaceRequest` 回复）。
- 视线判定复用 `WorldView::los`；`cover_zone` 来自地图 tag（M2-004）。

## 不做什么
- 望远镜视角/准星 UI → M3-034；OP/标记渲染 → 客户端 M3-029 之外的 UI 任务；
- Sniper 武器/屏息 → M3-011；bot 的 Spotter/Sniper 行为 → M3-024；
- Recon Flight 本体 → M3-009（本任务复用其暴露表）。

## 验收标准
1. 暴露规则矩阵单测：5 种来源 × {敌方视角, 友方视角, 非队员} 共 ≥ 15 个断言全部通过。
2. 审计：对一局脚本化场景（10 个被标记敌人 + 1 次 Recon），服务器向敌对客户端额外下发的敌方信息中，**非图标类条目 = 0**；图标位置误差 ≤ 5 m 步长（属性测试）。
3. OP 条件：40 m 内有敌（39/41 m 边界）、距敌据点 `R_sp+39/41 m`、距 Garrison 59/61 m、站姿放置、非 `cover_zone` 各自拒绝；全部满足通过；放置耗时 10 s ±1 tick。
4. OP 生命周期：寿命 900 s、HP 150（枪弹 ×0.3）、12 m 内有敌封锁、12 m 敌且友 0 人 10 s 销毁、被毁冷却 150 s（逐项断言）。
5. 标记：Spotter 1.2 s/Officer 1.8 s 完成；距离 800/500 m 边界；无 LOS 失败；显示时长 8/15/60 s（±1 tick）。
6. Sniper 隐蔽：检测距离系数 0.7、静止趴姿叠加 0.56、开火后 5 s 恢复 1.0（逐项断言）。
7. 暴露表每 tick 开销 ≤ 0.1 ms（`criterion`，20 个活动条目）。

## 测试要求
- 单元测试：上述 1–6；标记上限（第 11 个 Spotter 标记挤掉最早/拒绝，按 GD §16.3 取拒绝并 PLAN_ISSUES 记）。
- 集成/无头：`headless-client` 作为敌方客户端，收到标记图标但在无 AOI 资格时收不到精确状态（与 M1-024 的 AOI 测试复用）。
- 性能基准：`crates/sim/benches/exposure.rs`。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 5（规则 0.7 ms）：暴露/标记/OP 合计 ≤ 0.15 ms/tick。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；暴露规则与 AOI 的耦合点（`server` 端 M1-024）不在本任务，只提供 `WorldView` 数据与审计函数。
- GD §10.7 的 OP 放置姿态/`cover_zone` 为【待调参】，地图是否有 `cover_zone` tag 取决于 M2-004；缺失时放置校验默认通过并写 PLAN_ISSUES。
- 视角稳定阈值 GD 未给出，属"细化"。
