# M1-019 sim：HQ 出生与 Garrison

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：L
- 前置依赖：M1-018, M1-016
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §10.1; GAMEDESIGN §10.2; GAMEDESIGN §10.5; GAMEDESIGN §10.6; GAMEDESIGN §10.8; GAMEDESIGN §2.6; GAMEDESIGN §2.7; BRIEF R1

## 背景
重生网络是本作的核心循环之一。M1 只实现两类重生点：**HQ**（固定）与 **Garrison**；Outpost/Airhead/OP/据点重生点留 M2/M3。**R1：Garrison 的放置者是 Officer、Spotter、Commander，不是 Engineer**；M1 只有 Officer（Spotter/Commander 的权限字段留好）。
规则（GD §10.2/10.6/10.8/2.6；全部【待调参 / 待测 → REFERENCE_MEASUREMENTS §respawn / §economy】）：
- **HQ**：每队 1 个，出生区半径 40 m，波次 15 s；**保护区**半径 150 m：敌方玩家进入后 3 s 警告，之后每秒受 40 HP（炮击模拟）。
- **Garrison**：补给消耗 己方 Sector 或战区（中立）50、敌方 Sector 100 Supplies（M1 简化：按所在据点/区域所有者判断，见备注）；建造 20 s（1 人），每多 1 名同时施工友军速度 +0.6，最多 3 人（×2.2）；1200 HP，枪弹 ×0.05、爆炸 ×1.0；全队上限 6、每 Sector 2；间距：距己方 Garrison/Outpost ≥ 150 m、距敌方 Garrison ≥ 250 m、距任何活动据点圆心 ≥ `R_sp+100 m`、距任何 HQ ≥ 200 m；放置检测：坡度 ≤ 25°、非水体、周围 50 m 无**可见**已知敌人、放置者 15 m 内箱/车 Supplies ≥ 成本；**封锁**：存活未倒地敌人 30 m 内即停止接受重生（敌离开 3 s 后恢复）；**Overrun**：敌方 20 m 内有人且己方 20 m 内无人持续 25 s 销毁；被摧毁：HP 归零（炸药包 2500、Bazooka 800×2、Panzerschreck 1200×1、HE 直击 600、轰炸 1500）、主动拆除返还 50%；重生槽 24、每次出生耗 1、每 5 s 恢复 1；个人冷却 8 s；吞吐 ≤ 2 人/s。
- **出生计时**：`ready_time = t_kia + base_wait`（HQ 5 s、Garrison 8 s）；波次类（HQ）`spawn_time = ceil_to_next_wave_tick(max(ready_time, t_pick))`（15 s 一个波次 tick）；其余 `max(ready_time, t_pick)`；自杀惩罚：出生后 20 s 内再次 KIA（含放弃）`base_wait +5 s`；同点洪水：10 s 内同点出生 ≥ 8 人后再出生者 +2 s；重生点失效时玩家改选，无选择则落到 HQ；热身期 `base_wait ×0.5`。
- **出生**：重生点周围 8 m 内随机不重叠点（服务器检查 0.4 m 半径球无碰撞）；出生保护 3 s（伤害 ×0.0，移动或射击即取消；**仅 HQ/据点内有效，Garrison/Outpost/OP 不给保护**）；满额弹药；**防重生杀**：10 m 内有敌方存活玩家且有视线，出生点向后偏移到最远可用坐标，仍有威胁则该 Garrison 立即判为封锁。
- 主动重新部署（GD §2.7）：HQ/本方据点/Garrison 15 m 内按住 2 s 回到 DEPLOY，30 s 内最多换 1 次兵种/loadout。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/spawn/points.rs` —— `SpawnPoint{id, kind, team, pos, radius, state, slots, ...}`、`SpawnKind::{Hq, Garrison}`
- `crates/sim/src/spawn/garrison.rs` —— 放置校验 `validate_garrison_place()`、建造进度、血量、封锁/Overrun/销毁
- `crates/sim/src/spawn/queue.rs` —— 重生队列与计时（`ready_time`、波次、惩罚、洪水限制）
- `crates/sim/src/spawn/protection.rs` —— 出生保护与 HQ 保护区伤害
- `crates/sim/src/spawn/place.rs` —— 出生坐标选择与防重生杀
- `crates/sim/src/supply_iface.rs` —— `trait SupplyProvider { fn available(&self, pos:Vec3, r:f32)->u32; fn consume(&mut self, pos:Vec3, r:f32, amount:u32)->bool; }`（**M1-020 提供真实实现**；本任务内置测试桩）
- `data/rules/spawn.json`（M1-007 已建；本任务读取）
- `crates/sim/tests/spawn_*.rs`
### 要实现的行为
1. `request_spawn(player, spawn_point_id, tick)` 校验：点存在/本队/非封锁/槽位 > 0/冷却到期/玩家已 KIA 或在 DEPLOY；通过则入队，返回 `ready_in` 估计秒数（UI 显示用）。
2. 每 tick 处理到期出生：选坐标 → 创建玩家实体（`World::spawn_player`）→ 加保护 → 扣槽位。
3. Garrison 放置 `place_garrison(player, pos)`：校验放置者兵种 ∈ {officer, spotter, commander}（M1 仅 officer 可用）；全部条件失败时返回具体原因码（`TooCloseHq/TooCloseGarrison/Slope/Water/EnemyNear/NoSupplies/TeamCap`）；成功则扣 Supplies（通过 `SupplyProvider`）并创建 `Building` 状态（进度 20% HP 线性增至 100%）。
4. 施工：玩家持续交互；`rate = 1 + 0.6×(n−1)`（n ≤ 3 人）；放置者离开 15 m 不影响他人继续；完工后启用。
5. 封锁/Overrun/销毁：每 tick 空间查询；Overrun 计数 25 s；摧毁返还规则；Garrison 对枪弹/爆炸伤害乘数（0.05/1.0）。
6. HQ 保护区：敌方进入 150 m → 3 s 警告事件（`HqWarning`）→ 40 HP/s 伤害；对友方无影响。
7. 输出事件：`SpawnPointState{id,kind,team,pos,state}`（M1-010）每变更一次；`DeployResult`。
8. 玩家重新部署（15 m 内按 2 s）与"30 s 内换兵种 ≤ 1 次"。
### 接口与数据
```rust
pub enum PlaceError { WrongRole, TooCloseHq, TooCloseOwnGarrison, TooCloseEnemyGarrison, TooCloseObjective,
  Slope, Water, EnemyNear, NoSupplies, TeamCap, SectorCap }
pub fn validate_garrison_place(w:&World, who:PlayerId, pos:Vec3, sup:&dyn SupplyProvider) -> Result<GarrisonCost, PlaceError>;
```

## 不做什么
- Supply Crate/Ammo Crate/Depot 与 `SupplyProvider` 的真实实现：M1-020（E1-11 的"Garrison+补给闭环"端到端在 M1-020 完成后验证）；
- 小队/名额/loadout 校验与 `DeployRequest` 的整体处理：M1-021、M1-026；
- Outpost/Airhead/OP、据点重生点、Condemned 倒计时（Sector 易手）、Commander 预选：M2/M3；
- Engineer 拆除返还：Engineer 属 M2（本任务仅放置者本人/Officer 可拆）；
- 客户端全息预览与放置 UI：M1-060。

## 验收标准
1. 出生保护与计时单测（`spawn_timing`）：HQ 出生等待在 5–20 s 之间（波次对齐，tick 误差 ≤ 1）；Garrison = 8 s ±1 tick；自杀惩罚 +5 s、洪水 +2 s、热身 ×0.5 各 1 例。
2. Garrison 放置规则全覆盖（`garrison_rules`）：每种 `PlaceError` 至少 1 个失败夹具 + 1 个成功夹具（含：HQ 距离 199/201 m、己方 Garrison 149/151 m、敌方 249/251 m、据点 `R_sp+100`±1 m、坡度 26°/24°、水体、50 m 内可见敌人 vs 不可见敌人、Supplies 49/50、敌方区 99/100）。
3. 建造时间：1 人 20 s ±1 tick；3 人 `20/2.2 = 9.1 s` ±2%（单测）；HP 随进度从 20% 线性增长。
4. 槽位：24 → 连续出生 24 人后第 25 人需等待；每 5 s 恢复 1 槽（±1 tick）；同时出生吞吐 ≤ 2 人/s（单测）。
5. 封锁：敌人距 30 m 内立即封锁，离开 3 s 后恢复（边界 29.9/30.1 m）；Overrun 25 s 销毁（24.9 s 不销毁）。
6. 伤害：枪弹对 Garrison ×0.05（1200 HP 需 24000 名义伤害）、炸药包 2500 一击摧毁（单测）。
7. 出生保护：3 s 内伤害 ×0.0，移动/射击立即取消；Garrison 出生不给保护；HQ 出生给（单测）。
8. 防重生杀：10 m 内有敌且有视线时坐标后移并可能判封锁（2 例）；HQ 保护区 40 HP/s 在 3 s 警告后生效（单测）。
9. 性能：100 玩家 + 6 Garrison + 队列满载的 spawn 阶段 ≤ 0.1 ms/tick（`cargo bench -p sim spawn`）；确定性哈希两次一致。

## 测试要求
- 单元测试：上述全部边界；`SupplyProvider` 测试桩（可设库存）；出生坐标不与既有实体重叠。
- 集成：sim 内 headless 场景：Officer 放置 Garrison、队友经其重生（Supplies 用桩）。
- 性能基准：criterion `spawn`。
- 回归：确定性哈希基线更新。

## 性能预算
- 并入 ARCHITECTURE §6.10 阶段 5（规则 ≤ 0.7 ms/tick）；本任务 ≤ 0.1 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **INDEX 依赖提示**：Garrison 需要 Supplies，但 M1-020 依赖本任务（编号更大），故本任务用 `SupplyProvider` 桩，闭环验收推迟到 M1-020（E1-11）。
- **地图数据冲突**：MAPS §3.8 的 M1 临时 Garrison 位置（US (420,1550)、GER (1500,600)）与各自 HQ（US (480,1600)、GER (1480,560)）距离约 78 m/45 m，违反本表"距任何 HQ ≥ 200 m"。已裁定：MAPS §3.8 的候选点已移动到距 HQ ≥ 200 m，规则不放宽；本任务单测用合规夹具，M1-034 按修订后的 MAPS 写入数据。
- M1 无 Sector 概念：用"所在位置最近据点的所有者"近似（中立→50，敌方→100）；写 PLAN_ISSUES（细化）。
- 规模 L；若超出，将 HQ 保护区与热身/重新部署拆出 `M1-019b`。
