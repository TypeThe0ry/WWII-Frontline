# M0-012 sim：步枪射击、弹道与装填

## 元数据
- 里程碑：M0
- 工作流：sim
- 规模：L
- 依赖：M0-011, M0-008
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.3; GAMEDESIGN §6.4; GAMEDESIGN §6.5; GAMEDESIGN §6.6; ARCHITECTURE §6.7.3; ARCHITECTURE §7.2

## 背景
M0 的核心玩法：两把步枪（半自动 Garand、栓动 Kar98k）的开火状态机、散布与后坐、服务器权威弹丸（段扫描，含重力与阻力）、命中体与伤害衰减、弹匣与装填。所有数值读 `/data/weapons/*.json`（M0-008）。客户端用同一份 sim 代码预测弹药/冷却/后坐，**命中结算以服务器为准**（GAMEDESIGN §6.4；ARCHITECTURE §7.2：客户端不显示预测伤害/击杀）。
弹道（GAMEDESIGN §6.4）：每 tick 做线段扫掠；内部子步 4 次/tick（h = Δt/4，等效 120 Hz）积分 `a = (0,−g,0) − k_drag·|v|·v`，`v += a·h`，`x += v·h`；寿命：飞行 > 4 s 或 速度 < 150 m/s 销毁。ARCHITECTURE §6.7.3 另给了每 tick 一段的简化式，**以 GAMEDESIGN 的子步积分为准**（与 `dt` 解耦）。示例：Garand 853 m/s、k=0.00075，500 m 处速度 ≈ 586 m/s、飞行时间 ≈ 0.72 s。散布（§6.5）：`θ_spread = θ_base × stance_spread × move_spread + bloom`；`move_spread` 静止 1.0 / 走 1.5 / 跑 2.5，冲刺**不可射击**；后坐每发 `view_pitch += vkick × stance_recoil × (1±15%)`、`view_yaw += N(0,hσ)`，停火 0.12 s 后以 `recov_deg_s` 回落。伤害（§6.6）：`D = dmg × falloff(d) × part_mult`，头 ×3.0、颈 ×2.0、躯干 ×1.0、盆骨 ×0.9、四肢 ×0.65。M0 姿态只有站/蹲：`stance_recoil/stance_spread` 站 1.00/1.00、蹲 0.75/0.80（§9.5）。

## 具体内容
### 要创建/修改的文件
- `data/loadouts/us_rifleman_1.json`（primary `us_m1_garand`）、`data/loadouts/ger_rifleman_1.json`（primary `ger_kar98k`）；更新 `data/roles/rifleman.json` 的 `loadout_ids`。
- `crates/sim/src/weapon/{mod.rs,state.rs,spread.rs,recoil.rs,ballistics.rs,hitbox.rs,ammo.rs}`；`crates/sim/src/projectile.rs`；`SimEvent::{Shot,Impact,Hit}`（`Hit{shooter,target,zone,damage_hp,dist_m}`）。
- `crates/sim/tests/weapon_*.rs`、`crates/sim/benches/ballistics.rs`。
### 要实现的行为
1. **开火状态机**：`Ready → (fire) → Cooldown/BoltCycle → Ready`；`Reloading{kind,remaining_ticks}`；`Empty`。半自动：按住不连发（需松开再按），最小间隔 = `60/cap_rpm`（120 rpm → 0.5 s = 15 tick）；栓动：开火后 `bolt_cycle_ms`（1500 ms = 45 tick）内不可射击，且装填期间不可射击；冲刺（bit6 且在移动）、装填中均不可射击。时序量化：毫秒→tick 用累加器（分数 tick）避免系统性偏差（REFERENCE_MEASUREMENTS §7.2 第 3 条）。
2. **装填**：reload（bit2）；弹匣有余弹用 `tactical_ms`，空仓用 `empty_ms`；Garand 填满 8 发、Kar98k 夹装填满 5 发（M0 只做夹装，逐发装填属 M1-013）；装填完成扣 `reserve`，`ammo_in_mag/ammo_reserve` 写入 `self_state` 所需字段；无备弹则装填无效。
3. **瞄准方向**：以 `InputCmd.yaw/pitch` 为视线，加后坐累计偏移，再在半角 `θ_spread` 的圆锥内按 `Rng::for_(tick, entity, Purpose::Spread)` 均匀采样；ADS（bit1）用 ADS 散布并有 `ads_ms` 进入时间（M0 简化：ADS 状态在 `ads_ms` 后生效，之前用盲射散布）。
4. **弹丸**：出膛点 = 眼睛位置（站 / 蹲眼高为【待调参】初值，写入 `/data`）沿射线前移 0.5 m；每 tick 内 4 子步，每子步对 M0-010 的 `raycast`（地形）与命中体做段相交；命中地形生成 `Impact`；寿命条件见背景。
5. **命中体**：每个玩家 4 个胶囊（头球 + 躯干 + 大腿 + 小腿，随站/蹲，尺寸【待调参】初值写入 `/data`/常量表并标注，ARCHITECTURE §6.7.1）；射手自身不命中；M0 阵营：同队命中忽略（`friendly_fire_mult = 0`，M1-014 引入友伤）；部位→倍率按上表。
6. **伤害**：`D = dmg × falloff(d) × part_mult`，`falloff` 为 `/data` 分段线性；产生 `Hit` 事件并对目标 `Health` 扣减（死亡处理在 M0-013，此处先写入扣减与事件）。
7. `fire` 判定入口与 `buttons` 的 `fire` 位边沿（半自动用边沿，栓动用边沿）一致；`FireCmd` 里的 `view_tick` 在 M0 仅记录不回溯（M1-025）。
### 接口与数据
- `WeaponState{def_id, mag, reserve, fsm, cooldown_ticks, recoil_pitch, recoil_yaw, bloom_deg, ads_progress, …}` 作为玩家组件；`WeaponState::to_bytes/from_bytes` 供 `Predictor`（M0-014）使用；`SimEvent` 变体按 ARCHITECTURE §6.8.4 的 `Shot/Impact` 字段对齐。

## 不做什么
- 不做穿透/跳射/材质表（M1-012）；不做命中区细化、友伤与倒地（M1-014/016）；不做延迟补偿回溯（M1-025）。
- 不做其它武器类别、投掷物（M1-013/015）；不做压制（M1-017）。
- 不做生命耗尽后的死亡/重生（M0-013）。

## 验收标准
1. 弹道对照测试 `ballistics_vs_reference`：以 Garand 数据在水平发射、无风、`g=9.81` 下，500 m 处的下坠量与速度，对同一 ODE（GAMEDESIGN §6.4）用 RK4、步长 0.1 ms 的参考数值解相比误差 < 3%；同时输出与 GAMEDESIGN §6.4 示例（下坠 ≈ 2.5 m、速度 ≈ 586 m/s）的偏差（信息性，说明耦合阻力使下坠略小于 ½gt²）。
2. 射速上限：对 Garand 以 30 Hz 每 tick 持续按下开火（含边沿处理）10 s：发射数 ≤ 21 发（120 rpm × 10 s = 20 发，容差 1）；Kar98k 同样 10 s ≤ 7 发（1.5 s 循环 → 6.67，向上取整）；任何情况下间隔不小于数据值减 1 tick。
3. 状态机单测覆盖：半自动必须松开重按、栓动循环期间不能射击、装填中不能射击、冲刺不能射击、空膛点击无弹丸、装填中途切换无效（M0 无切枪）——至少 12 个用例全绿。
4. 装填：Garand 空仓装填耗时 = `empty_ms`±1 tick；Kar98k 夹装 3000 ms±1 tick；`ammo_in_mag/reserve` 守恒（总弹药 = 初值 − 已射发数）。
5. 散布统计：固定种子下对 Garand 盲射 10 000 发，落点角度分布 99.9% 在半角 `θ_base_hip` 内；站立 ADS 静止时（`ads_deg=0.15°`）在 100 m 处弹着点 95% 半径 ≤ 0.35 m。
6. 伤害：目标静止在 50/300/600 m，Garand 躯干命中伤害 = 70 / 70×falloff(300)/70×0.72（±0.01 HP）；爆头 ×3.0；Kar98k 同理。
7. 确定性：相同种子/输入 5000 tick 两次运行，`state_hash`（含弹丸）逐 tick 相同；服务器与 `PredictSim` 对同一输入的 `WeaponState` 逐 tick 位相同。

## 测试要求
- 单元测试：累加器取整、falloff 插值边界（F0、F1 之后）、后坐回落、命中体几何。
- 集成/无头测试：`weapon_duel`：两玩家对射 Garand vs Kar98k，事件序列与伤害守恒。
- 性能基准：`cargo bench -p sim ballistics_step`（400 弹丸×4 子步）；ARCHITECTURE §6.10 阶段 4 预算 2.5 ms/400 段。
- 回归：golden 事件序列哈希。

## 性能预算
武器与弹道阶段 ≤ 2.5 ms/tick（≤ 400 活动弹体，§6.10 阶段 4；M0 仅 20 玩家，记录实测）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GAMEDESIGN §6.4（子步积分）与 ARCHITECTURE §6.7.3（逐 tick 简化式）略有出入：按 GAMEDESIGN，并在 PLAN_ISSUES 记"文档冲突"。
- 验收 1 的"解析值"实际无闭式解（阻力与重力耦合），故以高精度数值参考解为准；INDEX 原写"与解析值误差 < 3%"，此为细化。
- 眼高、命中体尺寸等 GAMEDESIGN 未给数值，使用合理初值并全部标【待调参】【待测 → REFERENCE_MEASUREMENTS M-15】。
