# M2-017 sim：计分与经验

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：M
- 前置依赖：M1-016, M2-008, M2-010, M2-011, M2-015, M2-016
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §18; GAMEDESIGN §18.1; GAMEDESIGN §18.2; GAMEDESIGN §18.4; GAMEDESIGN §5.2; ARCHITECTURE §6.8.3

## 背景
GD §18：四类得分 Combat / Offense / Defense / Support，总分 = 四类之和；全部【待调参 / 待测 → REF §scoring】。M1-016 提供击杀归属输入与倒地/救起事件；M2-008 提供据点占领事件。本任务建立 `ScoreEngine`：**消费 sim 事件流 → 产出 `ScoreDelta` + 局内统计**，供计分板（M2-038）、结算页（M2-042）与 gateway 战绩（M2-022）使用。规则只在 sim，数值只在 `/data/scoring.json`。
主要得分（GD §18.1，节选，完整表以文档为准）：Kill +10（爆头 +5）、助攻 +5、Finish +3、载具击毁（轻 +40/中 +60/重 +80/软 +20，车组其余成员 +15）、摧毁 AT 炮/榴弹炮 +30、摧毁 Garrison +40 与资源节点 +30（计入 Offense）、连杀（10 s 内第 3 次）+10、压制助攻 +3、TK −30、3 s 内复仇 +5；Offense：占领区内使进度前进每 5 s +2、占领完成（区内累计 ≥ 20 s）+50、Garrison 出生每次 +2（上限 +20/min）；Defense：敌入区内每 5 s +2、区内击杀额外 +5、击退占领（进度 > 50% 被压回 0）在场防守者 +30、击杀正在炸毁/占领 Garrison 的敌人 +10；Support：救起 +20、包扎每 10 HP +1（上限每次 +10）、补给弹药每次 +3、建造每消耗 10 Supplies +1 与完成资源节点 +30、放置 Garrison 每 10 Supplies +1 与完成 +20、Garrison 出生放置者 +1（上限 +15/min）、Outpost 出生 +1（上限 +10/min）、载具修理每 30 HP +1 与修复模块 +5、同一对象 120 s 内再救起仅 +10。
防刷：同一击杀者对同一受害者 60 s 内第 4 次起不计 Combat 分；同据点 90 s 内反复易手后续不计 Offense；AFK 期间不计分；自杀/被友军击杀/环境死亡的**受害者**不计分。
经验（§18.2）：Match XP = 总分 + 胜负奖励（胜 +10%、负 +5%、平 +7%，需局内 ≥ 5 min）+ 完整参与奖励（≥ 70% 对局时间 +100）；账号等级 `XP_acc(n) = 400×(n−1)^1.5`（L10≈10 800、L50≈137 200、L100≈394 000）；兵种等级 `XP(n)=125·n·(n−1)`（L2=250 … L10=11 250）；Role XP = 该兵种存活期间所得分，本职行为 ×1.5（Medic 救起/包扎、Support 补给搬运、Engineer 建造修理、Officer 的 Outpost/Garrison 出生等）。
结算统计（§18.4）：K/D/A、四类分数、命中率、爆头率、最远击杀距离、最长连杀、救起次数、建造 Supplies、Garrison 出生数、修理 HP、载具击毁/伤害、小队排行/MVP、双方资源曲线（M2-009 提供）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/score/mod.rs` —— `ScoreEngine::on_event(&SimEvent) -> SmallVec<ScoreDelta>`；每 tick 结算周期性得分（5 s 档、每分钟上限）
- `crates/sim/src/score/xp.rs` —— `match_xp()`、`account_level(xp)`、`role_level(xp)`
- `crates/sim/src/score/stats.rs` —— `PlayerStats`、`MatchSummary`（序列化给 gateway）
- `data/scoring.json` —— 全部分值/上限/窗口（含 `enabled:false` 的 M3 条目：Commander 能力命中、Recon Flight、火炮击杀、Spot Assist 的 Spotter 来源）
### 要实现的行为
1. 事件 → 得分映射表驱动（`scoring.json`）；每条得分记录 `category`、`reason`、`value`、`role_at_time`。
2. 周期性得分：占领区内每 5 s 窗口、每分钟上限（Garrison 出生 +20、放置者 +15、Outpost +10、Ammo Crate 使用 +30）按 tick 计窗口，整数运算。
3. 反刷分：60 s 同对击杀计数、90 s 易手冷却、AFK 标志（来自 M2-018 的 `afk` 组件，未就绪时默认 false）。
4. 连杀、复仇、助攻窗口：沿用 M1-016 的击杀归属输入（助攻 = 近 N s 内造成 ≥ X 伤害，N/X 读 `/data`，GD 未给数值则 PLAN_ISSUES）。
5. Role XP：按分数事件的 `role_at_time` 累加，本职标志 ×1.5（`xp_role_bonus` 表）；局末输出每个兵种的 `RoleXpDelta`。
6. 局末 `MatchSummary`：胜负奖励、完整参与判定（在线时间 / 对局时间 ≥ 70%）、账号 XP 增量与等级。
### 接口与数据
- `ScoreDelta{ player:PlayerId, kind:ScoreKind, category:u8, value:i16 }` 对应 C4 `ScoreDelta`（ARCH §6.8.3）。

## 不做什么
- 计分板/结算 UI → M2-038/M2-042；战绩持久化 → M2-022。
- Commander 能力、火炮、Recon Flight、Spotter 相关得分 → 条目留在数据里 `enabled:false`，M3 打开。
- 兵种等级对 loadout 的解锁判定 → M1-021 已读 `unlock_condition`，此处只给 `role_level()`。

## 验收标准
1. 得分表逐类单测：对 §18.1 每个事件类别各 ≥ 1 个用例，断言分值与 `/data/scoring.json` 一致（共 ≥ 40 条，`cargo test -p sim score_table`）。
2. 防刷：同对第 4 次击杀 Combat 分 = 0；90 s 内同据点二次易手无 Offense 分；周期上限（Garrison 出生每分钟 ≤ +20）单测。
3. 等级曲线与 /data 一致：`role_level` L2=250、L5=2 500、L10=11 250 的边界 ±1；`account_level` L10 10 800、L50 137 200、L100 394 000 误差 ≤ 0.1%。
4. Match XP：总分 1 000、胜方、在线 100% → 1 000 + 100 + 100 = 1 200；败方 5% → 1 150；在线 < 5 min 无胜负奖励。
5. 局末摘要含 §18.4 个人数据全部字段（字段清单测试）。
6. 计分系统 CPU ≤ 0.05 ms/tick（100 玩家、模拟 50 事件/tick，`cargo bench -p sim score_tick`）。

## 测试要求
- 单元测试：表驱动 + proptest（分数单调、无溢出 i16）；
- 集成/无头：headless 5v5 + bot 20 min，`MatchSummary` 总分与逐条 `ScoreDelta` 之和一致。
- 性能基准：`score_tick`。
- 回归：确定性哈希（分数参与 `state_hash`）。

## 性能预算
ARCH §6.10 阶段 5「规则 0.7 ms」内 ≤ 0.05 ms/tick。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 建造/修理/资源节点/载具击毁等事件来自 M2-010/M2-011/M2-015/M2-016（依赖已列），须确保这些任务发出 `SimEvent`。
- 助攻窗口等 GD 未给数值的参数：不自拟，写 PLAN_ISSUES 并用最小默认（5 s / 伤害 ≥ 20 HP）标注 `initial`。
- 数值待 REF §scoring 校准（M2-054）。
