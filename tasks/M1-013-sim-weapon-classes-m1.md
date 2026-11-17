# M1-013 sim：冲锋枪、卡宾、手枪与近战

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：L
- 前置依赖：M1-012, M1-008
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.3; GAMEDESIGN §6.5; GAMEDESIGN §6.8; GAMEDESIGN §6.11; ARCHITECTURE §6.7.3; ARCHITECTURE §15.1

## 背景
M0-012 只有半自动/栓动步枪状态机（Kar98k、Garand）。M1 的五兵种需要：**全自动**（Thompson M1A1 700 rpm、M3 450 rpm、MP40 500 rpm）、**半自动**（M1 Carbine cap 200 rpm、M1911A1/P38 cap 300 rpm）、手枪、**近战**（刺刀/匕首/枪托/手枪托）以及**战术/空仓装填**差异。数据来自 M1-008，行为在 `crates/sim/src/weapon/`。
要点：
- 射速：自动武器 `cyclic = 60/rpm` 秒/发；以 tick 计需处理 30 Hz 下的亚 tick 累积（500 rpm = 3.6 tick/发 ⇒ 用累加器，**长期平均** 误差 ≤ 2%）。半自动用 `cap` 射速上限。
- 后坐/散布（GD §6.5）：每发 `view_pitch += vkick×stance_recoil×(1±15%)`，`view_yaw += N(0,hσ)×stance_recoil`，`bloom += bloom_per_shot`；停止射击 `recoil_delay 0.12 s` 后按 `recov_deg_s` 回落（不超过累积量 60%，剩余 40% 留给玩家下压）；`θ_spread = base×stance_spread×move_spread×supp_spread + bloom`，方向在半角锥内均匀随机；`move_spread`：静止 1.0/走 1.5/跑 2.5；冲刺不可射击。
- 装填：战术装（枪内有弹，弹匣回收为**部分弹匣**）与空仓装（GD §6.3 表：Thompson 2.3/2.7 s、MP40 2.4/2.9 s、M3 2.5/3.0 s、Carbine 2.0/2.4 s、M1911 1.6/2.0 s、P38 1.6/2.0 s）；弹药记法按"弹匣含枪内一个"（GD §5.3、§6.11）。
- 近战（GD §6.8）：刺刀伤害 90、前摇 0.35 s、冷却 1.2 s、有效距 1.7 m；枪托 40/0.45/0.9/1.3；匕首 70/0.25/0.8/1.2；手枪托 20/0.3/0.6/1.0；胶囊扫掠、视角 ±35° 内；**背后**（目标正面与攻击者夹角 > 120°）造成 KIA；冲刺中刺刀 +20% 前扑距离；近战不受 `ff_mult`（友军间无伤害）。
- 随机数一律 `Rng::for_(tick, entity, purpose)`（ARCHITECTURE §4.3.2）；射速校验含 5% 容差（ARCHITECTURE §15.1），多余射击拒绝。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/weapon/fire_modes.rs` —— `FireMode::{Semi, Auto, Burst(n), Bolt}`、射击累加器
- `crates/sim/src/weapon/recoil.rs` —— kick/bloom/回落模型（含 `bipod` 字段占位，M2 使用）
- `crates/sim/src/weapon/reload.rs` —— 战术/空仓装填、`partial_clip_policy`、打断规则（切枪/冲刺打断；翻越期间禁止）
- `crates/sim/src/weapon/melee.rs` —— 近战状态机与命中判定（胶囊扫掠、背后判定、冲刺扑击）
- `crates/sim/src/weapon/sidearm.rs` —— 副武器切换 `swap_in_ms`
- `crates/sim/tests/weapon_*.rs`；`crates/sim/benches/weapon_step.rs`
- `crates/headless-client/src/bin/fire_rate_probe.rs`（或 `crates/sim/examples/`）—— 测量实测 cyclic rate 的脚本
### 要实现的行为
1. 全自动：按住 fire 连发；`rpm_cyclic` 累加器每 tick 增加 `rpm/60/30`，≥1 发则开火（单 tick 最多 2 发的上限，保护 MG 类；M1 武器 ≤ 700 rpm 实际 ≤ 1 发/tick）。
2. 半自动：每次按下发射一发，受 `rate_of_fire_rpm_cap` 间隔限制；无"按住连发"。
3. 空仓：弹匣空时开火按钮触发空仓咔嗒事件（`DryFire`），不扣弹。
4. 装填：开始→结束计时；期间移动 ×0.85（由 M1-011 读取 `reloading` 标志）；被打断（换枪/冲刺/翻越）则取消且不消耗；完成时若战术装填，旧弹匣放回部分弹匣栈；总备弹随之更新。
5. 近战：按 melee 键进入前摇→命中帧→冷却；命中帧用胶囊扫掠检测 ±35° 扇区内最近目标；返回 `MeleeHit{target, dmg, backstab:bool, zone}`；背后 KIA；对倒地者的处决（1.2 s，不可移动）**只预留接口**，实现在 M1-016。
6. 手枪与主武器切换：`swap_in_ms` 内不可射击。
7. 暴露给 M1-014 的事件：`ShotFired{weapon, origin, dir, spread_seed}`、`MeleeHit`；`spread_seed` 与 `FireCmd.spread_seed` 一致以便客户端预测（ARCHITECTURE §6.8.2）。
### 接口与数据
```rust
pub struct WeaponState { pub weapon: WeaponId, pub mag: u16, pub reserve_mags: SmallVec<[u16;12]>,
  pub mode: FireMode, pub cooldown_ticks_q: f32 /*累加器*/, pub reload: Option<ReloadState>, pub recoil: RecoilState }
pub fn step_weapon(ws: &mut WeaponState, cmd: &InputCmd, ctx: &WeaponCtx) -> WeaponEvents;
pub fn melee_trace(attacker: &PlayerCtx, kind: MeleeKind, world: &dyn CollisionWorld) -> Option<MeleeHit>;
```

## 不做什么
- 命中部位倍率与伤害公式：M1-014；穿透/跳弹：M1-012；
- 手雷/烟雾：M1-015；Panzerfaust 与火箭类：见 M1-023 备注；
- 压制：M1-017；两脚架部署与 MG：M2；BAR/StG44/MG 等其余武器：M2；
- 近战处决动画/状态：M1-016；
- 客户端后坐表现、第一人称动画：M1-046。

## 验收标准
1. 各武器 cyclic rate 与 `/data` 误差 ≤ 2%：对 Thompson（700）、M3（450）、MP40（500）各开火 30 s 的 headless 测量实发数（`cargo run --example fire_rate_probe`），Carbine/M1911/P38 半自动在连续点击下实发间隔 = cap 间隔 ±1 tick。
2. 装填时间：6 把武器的战术/空仓装填耗时与 GD §6.3 表误差 ≤ 1 tick（12 项单测）。
3. 战术装填后背包出现 1 个"剩余 k 发"部分弹匣，总弹药守恒（单测，含 mag=0/满/部分）。
4. 后坐：连发 10 发后累计 pitch 与 `Σ vkick×stance_recoil` 误差 < 20%（含 ±15% 随机，固定 seed 回放精确可重复）；停火 0.12 s 后回落速率 = `recov_deg_s` ±5%，自动回落量 ≤ 累积量的 60%。
5. 近战：刺刀在 1.7 m 内 0.35 s 前摇后命中；1.8 m 不命中；扇区 ±35°/±36° 边界各 1 例；背后夹角 > 120° 判 `backstab` KIA，119° 不判（单测）。
6. 友军近战伤害 = 0（`ff_mult` 近战 0）；自动武器 `spread_seed` 逐发不同且与 seed 公式一致。
7. 射速注入：伪造 2× 射速输入时服务器接受的发数仍 ≤ cap×1.05（单测，作为 M1-027 的前置）。
8. 性能：`cargo bench -p sim weapon_step` 100 名玩家全部连发一 tick ≤ 0.3 ms（不使 ARCHITECTURE §6.10 阶段 4 的 2.5 ms 回归）。

## 测试要求
- 单元测试：射击累加器长期平均、半自动 cap、装填打断全路径、部分弹匣栈、近战几何边界、背后判定。
- 集成/无头：`headless-client` 与 `server` 对射 1 分钟，实测射速写入 STATUS。
- 性能基准：criterion `weapon_step`。
- 回归：M0 的步枪测试全部保持通过；确定性哈希基线更新。

## 性能预算
- 引用 ARCHITECTURE §6.10 阶段 4（武器与弹道 ≤ 2.5 ms/tick）；武器状态更新单独 ≤ 0.3 ms/100 玩家。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；若超出，优先保证全自动 + 装填 + 近战，后坐细节（回落 60%/40% 手动下压）放入 `M1-013b`。
- 30 Hz 下 700 rpm = 0.39 发/tick；累加器必须保存跨 tick 小数，否则 2% 精度不可达。
- 对标测量（REFERENCE_MEASUREMENTS §fire-rate、§recoil）在 M1-071 回填，射速以 `/data` 为准。
