# M1-016 sim：倒地、救起与击杀归属

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：M
- 前置依赖：M1-014
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §7.1; GAMEDESIGN §7.2; GAMEDESIGN §7.3; GAMEDESIGN §7.4; GAMEDESIGN §7.5; GAMEDESIGN §7.6; GAMEDESIGN §2.2; GAMEDESIGN §5.9

## 背景
M1-014 提供伤害结算（`Hit/Downed/Killed`）与归属输入。本任务实现完整生命状态机与医疗规则（GD §7）：`ALIVE → DOWNED → KIA` 与 Medic 救起。GD §7.2 数值（均【待调参】；救起/流血项【待测 → REFERENCE_MEASUREMENTS §medical】）：
- 最大 HP 100；自然恢复仅 HP < 60：受伤 10 s 后以 1 HP/s 恢复到 60。
- 流血：单次命中致 ≥ 30 HP 伤害且未倒地，或被爆炸破片命中 → 1.0 HP/s，**不致死**（ALIVE 下最低 5 HP）。
- 自救绷带：按住 3.5 s，止血 +25 HP，最高 75；共 2 条。Medic 包扎友军：2.5 s，止血 +40 至多 100；Medic 6 条。
- 倒地：`downed_hp` 40；倒地流血 0.5 HP/s（≈ 80 s）；放弃（Give Up）倒地 5 s 后可用、按住 3 s → KIA；爬行 0.6 m/s，不能射击/用物品；可发"求救" Ping（每 5 s）。
- 救起：Medic 在 ≤ 1.5 m 按住交互 **5.0 s**；被救起者 HP 40、无流血、2 s 内不能射击；受伤 ≥ 20 HP 或离开 1.5 m 打断，进度 3 s 内可续；同一玩家 120 s 内被救起第 n 次，下次倒地流血时长 −20 s×n（最低 20 s）。
- 非 Medic：可对倒地者包扎使流血停止（2.5 s，耗绷带，不恢复 HP）；敌方近战/工兵铲对倒地者 = 处决（1.2 s，攻击者期间不可移动）。
- 医疗包（Aid Kit，Medic）：3 m 半径、8 HP/s、10 s、最多 3 次；对倒地者无效。
- 击杀归属（GD §7.5）：击倒记 Kill；倒地→KIA 若敌伤致死记 Finish（不计击杀数）；助攻：被击倒前 15 s 内累计 ≥ 25 HP 的非击倒者；直接 KIA 记 Kill；载具击毁 20 s/30% 助攻；自杀/环境不计 Kill；友军击倒记 TK 不计 Kill。
- 其它伤害（§7.6）：跌落（≤3 m 无伤；3–12 m 每米 +12 HP；≥12 m KIA）；淹水（≥1.8 m 且 > 20 s：10 HP/s）；出界 10 s 倒计时 KIA；载具碾压（≥15 km/h KIA、5–15 km/h 70 HP）。
- 客户端状态机对应：`IN_MATCH.ALIVE/DOWNED/KIA`（GD §2.2），KIA 后 3 s 死亡镜头 → DEPLOY。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/health/state.rs` —— `LifeState::{Alive, Downed{...}, Kia{...}}` 与转移函数
- `crates/sim/src/health/bleed.rs`、`regen.rs`、`bandage.rs`
- `crates/sim/src/health/revive.rs` —— 救起交互（范围、打断、续进度、疲劳）
- `crates/sim/src/health/downed_actions.rs` —— 爬行、求救、放弃、被处决
- `crates/sim/src/health/environment.rs` —— 跌落/淹水/出界/碾压伤害
- `crates/sim/src/health/kill_credit.rs` —— 归属结算：输出 `KillRecord{killer, victim, weapon, headshot, dist, assists, kind: Down|Finish|Direct}`
- `crates/sim/tests/health_*.rs`
- 与 M1-010 对齐：产出 `Death{victim,respawn_after,can_be_revived}`、`Revive{medic,target}`、`Kill{...}` 事件
### 要实现的行为
1. 转移：`Alive --(HP≤0 非超杀/爆头/爆炸)--> Downed`；`Alive/Downed --(爆头|HP≤−60|爆炸|碾压)--> Kia`；`Downed --(downed_hp≤0|流血耗尽|放弃|被处决)--> Kia`；`Downed --(救起)--> Alive(HP 40)`。
2. 倒地中：受任何伤害扣 `downed_hp`；不可射击/用物品；移动 0.6 m/s；`can_be_revived` 位随 `Death` 事件发出（M1 简化：倒地即可被救起；KIA 不可）。
3. 救起：Medic 兵种校验（`role==medic`）；距离/视线（≤1.5 m）；5.0 s 计时；中断/续进度（3 s 窗）；疲劳；被救起者 2 s 射击锁；Medic 获得 `ScoreDelta`（计分在 M2，本任务产出事件 `RevivePerformed`）。
4. 包扎：自救 3.5 s（+25 至 75，耗 1 条）；Medic 对友军 2.5 s（+40 至 100）；非 Medic 对倒地者包扎 2.5 s（止血，不回血，耗绷带）。
5. 流血与自然恢复：每 tick 更新；ALIVE 流血下限 5 HP。
6. 处决：近战命中倒地者→ 1.2 s 处决动作；攻击者锁定移动；完成则 Kia；被打断（攻击者死亡/被击中 ≥ N）取消。接口由 M1-013 的近战事件触发。
7. 环境伤害：跌落按落差、淹水、出界（接口 `OutOfBounds` 由地图边界提供）、碾压（由 M1-022 提供速度）。
8. 归属：按 GD §7.5 表实现，含 15 s 助攻窗口与 ≥ 25 HP 阈值、TK 规则（友军击倒不计 Kill）。
### 接口与数据
```rust
pub enum LifeState { Alive, Downed { downed_hp: f32, bleed_ticks_left: u32, revived_count_window: u8 },
                     Kia { tick: Tick, reason: KiaReason } }
pub fn apply_damage(p:&mut PlayerHealth, r:&DamageResult, src:&DamageSource) -> Vec<SimEvent>;
pub fn step_revive(medic:&mut ReviveAction, target:&PlayerHealth, w:&dyn WorldView) -> ReviveProgress;
```
`/data/health.json`：所有上述数值（单位后缀），状态 `initial`。

## 不做什么
- 重生/部署（Kia 后）：M1-019/M1-021；
- 得分与结算：M2；TK 惩罚/投票：M2；
- 倒地/死亡 UI、视角暗角：M1-059；
- 非 Medic 的 Medic 呼叫 Ping 的 UI：M1-058；
- 医疗包范围治疗的完整实现细节：本任务只实现"放置/半径/速率/次数"，对倒地者无效。

## 验收标准
1. 状态机全转移单测（`health_transitions`）：Alive→Downed、Alive→Kia（爆头/超杀/爆炸/碾压 4 种）、Downed→Kia（耗尽/放弃/处决/再受击 4 种）、Downed→Alive（救起），共 ≥ 10 条转移全部通过。
2. 救起耗时 5.0 s，与 `/data` 误差 ≤ 1 tick；中断后 3 s 内续上进度保留，3.1 s 后重置（边界单测）。
3. 倒地流血 0.5 HP/s：40 HP 在 80 s ±1 tick 耗尽；第 3 次被救起后流血时长缩短 60 s 且不低于 20 s（疲劳单测）。
4. 自救绷带 3.5 s、+25 HP 至多 75；Medic 绷带 2.5 s、+40 至多 100；ALIVE 流血最低 5 HP（各 1 例）。
5. 放弃：倒地 4.9 s 时无法放弃，5 s 后可放弃，按住 3 s 转 Kia；不给对方 Finish（单测）。
6. 归属：A 先打 30 HP，B 打倒 ⇒ B 记 Kill、A 记 Assist（≥25）；C 最后击毙倒地者 ⇒ C 记 Finish 而非 Kill；流血耗尽 ⇒ 无归属；友军击倒 ⇒ TK 且 Kill=0（4 例）。
7. 环境：落差 6 m ⇒ 36 HP（3 m 以内无伤，每米 12）；落差 12 m ⇒ Kia；出界倒计时 10 s（单测）。
8. 处决 1.2 s ±1 tick；期间攻击者位移 = 0（单测）。

## 测试要求
- 单元测试：全转移、边界（HP=0/−60/−59.9）、计时器（以 tick 计）、疲劳窗口 120 s、归属 15 s 窗口。
- 集成/无头：headless 两客户端演示倒地→救起，服务器 `Revive` 事件被送达（依赖 M1-026 前仅在 sim 内断言）。
- 性能基准：100 名玩家全部处于 Alive/流血状态的健康系统一 tick ≤ 0.1 ms。
- 回归：确定性哈希基线更新。

## 性能预算
- 并入 ARCHITECTURE §6.10 阶段 5（规则 ≤ 0.7 ms/tick）；本系统 ≤ 0.1 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §7.2 中"救起时间 5.0 s、倒地 40 HP、疲劳 −20 s×n"均【待调参】；批 M1 对标（H-04–H-17）在 M1-071 回填。
- INDEX 验收"救起耗时与 /data 误差 ≤ 1 tick"即本任务验收 2。
- 倒地者对 Medic 的可见性与地图图标规则属 AOI/UI（M1-024、M1-058），不在本任务。
