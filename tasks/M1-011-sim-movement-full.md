# M1-011 sim：完整移动（姿态、体力、翻越、探身）

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：L
- 前置依赖：M0-011, M1-007
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §9.1; GAMEDESIGN §9.2; GAMEDESIGN §9.3; GAMEDESIGN §9.4; GAMEDESIGN §9.5; GAMEDESIGN §9.6; ARCHITECTURE §5.3; ARCHITECTURE §6.10; ARCHITECTURE §15.1

## 背景
M0-011 已有 Rapier KCC（enhanced-determinism）与 走/冲刺/蹲/跳 的 `step_player`（单次 KCC ≤ 20 µs；100 个 KCC 一 tick ≤ 3 ms）。本任务把移动补全到 GD §9 的完整规格：趴/蹲/站切换时间、体力、翻越/攀爬、探身、姿态对速度与散布/后坐/可见度的修正，以及供服务器反作弊使用的验证接口（M1-027）。该代码同时被服务器与 `sim-wasm` 预测使用，**必须是纯函数式的确定性步进**（ARCHITECTURE §4.3.2：无墙钟、无 HashMap、`libm`）。
关键数值（均【待调参 / 待测 → REFERENCE_MEASUREMENTS §movement】，来自 GD §9）：
- 速度（m/s）：Walk 2.0、Run 4.4、Sprint 6.4、蹲 2.4/1.5、趴 0.9（慢）/1.3（快，体力 8/s）、ADS 乘数 站 0.55/蹲 0.6/趴 0.8、装填中 ×0.85、倒地爬行 0.6。重量修正 `clamp(1 − 0.012×(load_kg−10), 0.85, 1.0)`。
- 姿态切换：站→蹲 0.25 s、蹲→站 0.25、蹲→趴 0.50、趴→蹲 0.60、站→趴（冲刺俯冲）0.70、趴→站 0.90；切换中不可射击。
- 体力：上限 100；冲刺 −20/s；恢复 12/s（延迟 1.0 s；蹲 +20%、趴 +40%）；< 25 呼吸/ADS sway ×1.5；=0 冲刺锁定至 ≥30 且跳跃/翻越锁定；跳跃 0.6 m 高耗 15。
- 翻越（障碍 ≤ 1.3 m）0.8 s 耗 10；攀爬（1.3–1.8 m）1.4 s 耗 20，头顶空间 ≥ 0.6 m；可翻越判定：前方 0.7 m 内障碍高度在区间、上表面宽 ≥ 0.2 m、头部空间满足。
- 探身 ±20°、头位移 0.35 m、耗时 0.15 s；趴姿仅"探头窥视"；探身散布 ×1.10，不可与冲刺并用。
- 姿态修正表（GD §9.5）：站/蹲/趴的 `stance_recoil` 1.00/0.75/0.50、`stance_spread` 1.00/0.80/0.60、命中盒高 1.8/1.2/0.5 m、可见度 1.0/0.85/0.65。碰撞胶囊半径 0.35 m。
- 服务器权威速度上限 = 当前姿态最大速度 × 1.15（GD §9.6；ARCHITECTURE §15.1）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/movement/stance.rs` —— 姿态状态机（含过渡计时，以 tick 计）
- `crates/sim/src/movement/stamina.rs` —— 体力模型
- `crates/sim/src/movement/vault.rs` —— 翻越/攀爬检测（射线/形状查询）与动画时序
- `crates/sim/src/movement/lean.rs` —— 探身与头部命中盒偏移
- `crates/sim/src/movement/modifiers.rs` —— 速度/散布/后坐/可见度修正（读 `/data` roles 与 GD §9.5 系数）
- `crates/sim/src/movement/validate.rs` —— `max_speed_for(state)`、`check_displacement(prev, next, ticks)`（只读，供 server/anticheat）
- `crates/sim/src/movement/mod.rs` —— 重写/扩展 `step_player`，保持 M0 签名兼容
- `crates/sim/tests/movement_*.rs`、`crates/sim/benches/kcc_step.rs`（扩展）
- `data/roles/*.json` 若缺 `vault_time_mult`/`sprint_cost_mult` 等字段，补充（Assault 的 −20%/−25% 在 M2，但字段先留）
### 要实现的行为
1. 姿态：输入位 crouch/prone 切换；过渡期间碰撞胶囊高度按目标线性插值；顶部空间不足时拒绝站起（射线检测）。
2. 速度选择：按姿态/Walk/Run/Sprint/ADS/装填/倒地叠乘；结果按 GD §9.1 取整到 tick 内位移；斜坡超 35°（M0-010 的坡度限制）不可攀。
3. 体力：每 tick 更新；锁定/解锁滞回（0 → 30）；输出给 `self_state.stamina(6b)` 的量化值（0..63）。
4. 翻越/攀爬：检测到可翻越障碍并按下 `use`/跳跃时进入 `Vaulting{start_tick,end_tick,target_pos}`，期间不可开火、位置沿预设曲线插值，结束后落点必须在可站立处否则取消；Assault 倍率读 role 数据。
5. 探身：`lean_l/lean_r` 位；探身角随时间 0→±20° 用 0.15 s；头部命中盒偏移向 M1-014 暴露 `head_offset()`。
6. 修正系数接口：`stance_recoil(state)`、`stance_spread(state)`、`move_spread(state)`（静止 1.0、走 1.5、跑 2.5、冲刺不可射击）、`visibility(state)`。
7. 验证接口：`MoveBound { max_horizontal_mps, max_accel }`；`check_displacement` 对 N tick 位移超 `bound×1.15×N×dt` 返回 `Violation`。
### 接口与数据
```rust
pub enum Stance { Stand, Crouch, Prone }
pub struct MoveState { pub stance: Stance, pub transition: Option<(Stance, u16 /*tick_end*/)>, pub stamina: f32,
  pub lean: i8 /* -20..20 deg */, pub vault: Option<VaultState>, pub sprint_locked: bool }
pub fn step_player(w: &mut PlayerCtx, cmd: &InputCmd) -> PlayerAuthState; // M0 签名保持兼容，状态字段追加
pub fn max_speed_for(s: &MoveState, load_kg: f32, data: &RoleDef) -> f32;
```
`PlayerAuthState`/`EntityState` 新增的 `stance`(3b)、`stamina`(6b) 字段与 M1-010 的 `self_state` 对齐。

## 不做什么
- 倒地爬行与救起状态机：M1-016；
- 载具驾驶：M1-022；
- 梯子、游泳/涉水细节、雪地减速：M2/M4（本任务可留接口但不实现）；
- 服务器侧违规计数与处置：M1-027（本任务只提供 `validate.rs`）；
- 动画与 FP 手臂表现：M1-045/M1-046。

## 验收标准
1. 各姿态/动作稳态速度与 `/data` 值误差 < 2%：对 Walk/Run/Sprint/蹲(Run,Walk)/趴(慢,快)/ADS 三姿态/装填中共 ≥ 12 项，各自跑 90 tick 取平均速度（`cargo test movement_speeds`）。
2. 姿态切换时间与 GD §9.2 表误差 ≤ 1 tick（6 项转换逐一测）。
3. 体力：冲刺 5 s 恰好耗尽（100/20）±1 tick；耗尽后锁定，恢复到 30 才解锁；静止恢复速率 12/s ±2%（含 1.0 s 延迟）；蹲 +20%、趴 +40%（单测）。
4. 翻越时间 0.8 s ±1 tick；攀爬 1.4 s ±1 tick；障碍高 1.31 m 走攀爬、1.29 m 走翻越、1.81 m 拒绝（边界单测）。
5. 探身：0.15 s 达 ±20°；趴姿不可探身（仅窥视，抬高 0.15 m）；探身时散布 ×1.10（单测）。
6. 验证接口：1.3× 速度的合成输入序列在 10 tick 内被 `check_displacement` 标 `Violation`；1.10× 的合法抖动输入 0 次误报（各 1000 条序列）。
7. 性能：`cargo bench -p sim kcc_step` 含新逻辑单次中位数 ≤ 20 µs（参考机，M0 同一阈值，不得回归）；100 个玩家一 tick 的移动阶段 ≤ 3.0 ms（ARCHITECTURE §6.10 阶段 2）。
8. 确定性：同 seed + 同输入两次运行 1 万 tick 的 `state_hash` 逐 tick 相同（M0-007 的测试扩展用例）。

## 测试要求
- 单元测试：状态机转移全覆盖（3 姿态×切换×被打断）、体力曲线、翻越判定边界、探身、修正系数表。
- 集成/无头：`headless-client` 预设脚本回放（走/冲刺/翻越）与服务器权威位置差 < 0.05 m（R5 容差）。
- 性能基准：`kcc_step` criterion 基线更新到 `crates/sim/benches/baseline.json`。
- 回归：M0-029 确定性哈希基线更新并记录新值。

## 性能预算
- 引用 ARCHITECTURE §6.10：阶段 2 角色移动 100 个 KCC ≈ 3.0 ms/tick（单个 ≈ 20 µs）；超出触发 §5.3 降级方案（写 PLAN_ISSUES，不要自行改架构）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：翻越/攀爬的几何检测最易膨胀，先做固定高度阈值 + 一次射线判定，复杂窗口/梯子留 M2。
- 所有速度/时间为【待调参 / 待测 → REFERENCE_MEASUREMENTS §movement】，批 M1 的 M-07–M-11 等测量回填在 M1-071，误差口径以 `/data` 为准。
- 预测与对账不要求 WASM/原生逐比特一致（R5）；确定性哈希仅 native/native 与 wasm/wasm 阻断。
