# M2-011 sim：Outpost、维修站与反坦克炮

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：M
- 依赖：M2-010, M2-006
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §10.3; GAMEDESIGN §12.4; GAMEDESIGN §15.7; GAMEDESIGN §14.8; GAMEDESIGN §14.2; GAMEDESIGN §10.6; BRIEF §3c R1; BRIEF §3c R7

## 背景
三个"建造类"设施，复用 M2-010 的建造框架与 M1-019 的出生点框架：
1. **Outpost**（GD §10.3，R1：由 **Officer** 放置，不是 Engineer；无补给消耗）：放置条件 ≥ 2 名其他小队成员在 15 m 内且存活；周围 25 m 无敌；距任何友方 Garrison/Outpost ≥ 100 m；距敌方据点圆心 ≥ `R_sp+60 m`；坡度 ≤ 30°；不在载具内。放置 6 s（保持不动）；每小队 1 个，放新的则旧的移除，被毁/移除后冷却 120 s；容量 18 次出生、不恢复；个人重生计时 12 s；仅本小队成员可用；HP 300（枪弹 ×0.2，爆炸 ×1.0）；敌在 25 m 内封锁，Overrun：敌 15 m 内且友方 15 m 内无人持续 15 s 销毁；寿命 600 s；小队长死亡保留，转交后新队长可用。Sector 易手时的位置检查见 GD §3.3.6（距新所有者据点 < 120 m 立即销毁，M2-008 调用）。
2. **Repair Station**（GD §12.4/§14.8）：Engineer 建，120 Supplies、15 s、HP 600、全队 ≤ 3；8 m 内载具自动修理：每 1 Supplies 修 5 HP，修复速度 +20 HP/s，初始库存 200 Supplies（Support 可补箱）；同时对载具补弹药（1 发/2 s，扣 Supplies 4/发）；毁坏模块**不能**修；钢制 `structure_armor`：枪弹 0.02、爆炸 1.0（HE 0.7，HEAT 1.0）。
3. **AT 炮阵地**（GD §15.7，R7：**Anti-Tank 兵种**建造并操作，200 Supplies、20 s，全队 ≤ 3 门、每 Sector ≤ 1）：US 57 mm M1（P100=92、drop 3%/100 m、结构伤害 350、HE 140/R4）与 GER 7.5 cm Pak 40（P100=96、drop 2.2%、结构伤害 400、HE 250/R7）；座位炮手 ×1 + 装填手 ×1；阵地 HP 600；防盾 7 mm/0°（正面 ±60° 有效，阻挡枪弹，对 HEAT/AP 视同装甲板）；弹药 AP 18 / HE 8（库存总 26）；装填 3.0 s（2 人）/ 4.2 s（1 人）（Pak 40：3.2/4.5 s）；水平射界出厂方向 ±40°，Engineer 可在 3 s 内重新定向；俯仰 US −5°/+15°、GER −5°/+25°；补弹：靠近 Ammo Crate 或 100 Supplies → 8 AP；AP/HE 切换需 1.0 s 重新装填；无机枪；开炮后 3 s 暴露炮口火光。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/spawn/outpost.rs` —— `OutpostPlace`、容量/寿命/封锁/Overrun（扩展 M1-019 的 `SpawnPoint{kind:Outpost}`）
- `crates/sim/src/build/repair_station.rs` —— 库存、范围修理、补弹
- `crates/sim/src/emplacement/at_gun.rs` —— 炮位实体、座位、射界、装填、弹药库存；射击复用 M1-023 的炮弹弹道与穿深判定函数（不复制穿深逻辑）
- `data/emplacements/{us_at_57mm_m1,ger_at_75mm_pak40}.json` —— **M2-006 未含 AT 炮数据，本任务补**，字段取 GD §15.7，状态 `initial`
- `data/structures/{outpost,repair_station}.json`
### 要实现的行为
1. Outpost 放置校验返回 `OutpostError` 枚举（`NeedSquadmates/EnemyNearby/TooCloseFriendly/TooCloseEnemySP/Slope/InVehicle/Cooldown`），逐项对应上面的条件；6 s 内移动/受伤则中断。
2. Outpost 出生：个人冷却 12 s，仅本小队；每次出生消耗 1 容量；容量 0 或寿命到期（600 s = 18 000 tick）销毁；封锁期间不接受出生；出生保护 3 s（GD §2.6）。
3. Repair Station：每 tick 对 8 m 内己方载具按 20 HP/s 修理，消耗 `ceil(heal/5)` Supplies 自库存；库存 0 停止；Support 补箱 +100/箱（M1-020 交互）；补弹每 2 s 1 发扣 4 Supplies。
4. AT 炮：建造（M2-010 框架，建造者必须是 Anti-Tank）；射击流程 = 装填→开火→`Shell` 弹丸；装填员有无决定装填时间；射界外输入被钳制；炮盾以装甲区参与 M1-023 的命中判定；HP 0 销毁；炮口火光暴露事件（GD §17.2 炮口闪光 5 s、误差 25 m）输出 `SimEvent::MuzzleFlash`。
5. 数量上限：AT 炮全队 3、每 Sector 1；Repair Station 全队 3；均纳入全队 120 建筑总上限（M2-010）。
### 接口与数据
- `OutpostState{ squad:SquadId, hp, spawns_left:u8, expires_tick:u32, blocked:bool }`；`AtGunState{ ammo_ap:u8, ammo_he:u8, loaded:Shell, facing_center_yaw:f32 }`；数值全部读 `/data`。

## 不做什么
- Garrison（M1-019，Officer/Spotter/Commander）、Airhead 与 OP（M3/M1 后续）；榴弹炮（M3，预设阵地，R7）。
- 载具模块损伤与载具自身维修（Engineer 工具箱）→ M2-016；Engineer 工具行为 → M2-012。
- 部署界面显示 Outpost → UI（M1-056 已留位）；建造菜单 → M2-039。

## 验收标准
1. Outpost 放置/消耗/冷却规则单测：7 个失败条件各 1 例 + 成功 1 例；冷却 120 s（3 600 tick）边界 ±1 tick；放置 6 s 中断用例。
2. Outpost 容量 18：第 19 次出生被拒；寿命 600 s 到期销毁；Overrun 15 s（450 tick）销毁，中途友方进入 15 m 则清零。
3. Outpost 仅本小队成员可选；队长死亡后保留、移交队长后新队长可用（单测）。
4. Repair Station：HP 1000/1600 的友军载具停在 8 m 内，20 HP/s 速度误差 ≤ 2%；库存 200 Supplies 最多修 1000 HP；毁坏模块不被修复。
5. AT 炮伤害与 /data 一致：对 12 组（板厚 × 倾角 × 距离）AP 判定结果与 M1-023 手算表一致（复用其测试向量）；P100/drop 取自 /data（57 mm 92/3%，Pak 40 96/2.2%）。
6. AT 炮装填：2 人 3.0 s、1 人 4.2 s（US）；弹种切换 1.0 s；射界 ±40° 钳制与 3 s 重新定向。
7. 非 Anti-Tank 玩家发起 AT 炮建造被拒；全队第 4 门 / 同 Sector 第 2 门被拒。
8. 本任务新增规则阶段 CPU ≤ 0.1 ms/tick（3 Outpost × 6 小队 + 3 站 + 3 炮，`cargo bench -p sim outpost_repair_atgun`）。

## 测试要求
- 单元测试：`outpost_place_rules`、`outpost_lifecycle`、`repair_station_heal`、`at_gun_reload`、`at_gun_penetration_vectors`、`at_gun_build_permission`。
- 集成/无头：headless 脚本：Officer 放置 Outpost，3 名队员使用；AT 兵种建炮并击毁 Sherman（M1-009）。
- 性能基准：criterion 如上。
- 回归：确定性哈希。

## 性能预算
ARCH §6.10 阶段 5（规则）总计 0.7 ms：本任务 ≤ 0.1 ms；阶段 4 弹体扫描（≤ 400 段、2.5 ms）不得因炮弹新增而超出。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 M 但含三类设施；若超出一次会话，把 AT 炮拆为续作 M2-011b（README §5.6），先交付 Outpost 与 Repair Station。
- GD §3.3.6 对 Outpost 的位置检查与 §10.3 条件分属不同文档段，实现时统一放在 `outpost.rs` 并由 M2-008 调用。
- AT 炮数据缺口（M2-006 不含）已在此补；INDEX 未列依赖修正，记 PLAN_ISSUES。
