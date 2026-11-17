# M1-014 sim：命中区与伤害模型

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：M
- 依赖：M1-012, M0-013
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.6; GAMEDESIGN §7.1; GAMEDESIGN §7.5; GAMEDESIGN §9.5; ARCHITECTURE §6.7.1; ARCHITECTURE §6.7.3; ARCHITECTURE §8.1

## 背景
M0-013 只有"躯干致死发数与 /data 一致"的简易伤害与死亡。M1 需要完整命中区模型，是延迟补偿（M1-025）、倒地（M1-016）、压制（M1-017）、手雷（M1-015）的共同基础。规则（GD §6.6）：
- 部位倍率：头（含头盔）×3.0（枪弹；有头盔时手枪弹/冲锋枪弹头部 ×2.0；步枪弹/MG 不受头盔影响）、颈 ×2.0、躯干/胸 ×1.0、盆骨/下腹 ×0.9、上肢 ×0.65、下肢 ×0.65。
- 伤害公式：`D = dmg × falloff(d) × part_mult × pen_factor × target_state_mult`；`falloff` 线性由 F0/F1/min（M1-008 数据）；`pen_factor` 来自 M1-012；`target_state_mult`：对倒地者按 `downed_hp` 扣减（M1-016）。
- 倒地/死亡判定：命中后 `HP ← HP − D`；`HP ≤ 0`：命中头部（枪弹）、`HP ≤ −60`（超杀）、爆炸/载具碾压 → **KIA**，否则 **Downed**（倒地血量 40）。
- 友军伤害系数 `ff_mult`：枪弹 ×0.5、爆炸 ×0.7、近战 ×0、载具碾压 ×1.0、坦克炮 ×0.5；自伤（手雷）×1.0【待调参】。
- 命中体（ARCHITECTURE §6.7.1）：玩家 4 个胶囊（头/躯干/大腿/小腿，随姿态）+ 世界位置 + 朝向，每条约 40 B；**回溯历史格式与此一致**（16 tick 环）。姿态命中盒高度：站 1.8、蹲 1.2、趴 0.5 m（GD §9.5）；探身时头部命中盒独立偏移（M1-011 `head_offset()`）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/combat/hitbox.rs` —— `HitCapsules`（4 胶囊）、`from_player(state)`、与 `LagCompHistory`（`HitboxFrame`）共用的数据布局
- `crates/sim/src/combat/zones.rs` —— `HitZone::{Head,Neck,Torso,Pelvis,UpperLimb,LowerLimb}`、倍率表（`/data/combat/hit_zones.json`）
- `crates/sim/src/combat/damage.rs` —— `compute_damage(&HitInfo, &Shooter, &Target, &WeaponDef) -> DamageResult`
- `crates/sim/src/combat/friendly_fire.rs` —— `ff_mult(kind, shooter_team, target_team)`
- `crates/sim/src/combat/credit.rs` —— 击杀归属输入记录（最近伤害来源 + 15 s 窗口累计伤害，供 M1-016 使用）
- `crates/sim/src/combat/lagcomp_history.rs` —— `LagCompHistory`（ARCHITECTURE §8.1 签名）的 `record`/`hitboxes_at`（亚 tick 插值）**数据结构部分**；回溯使用逻辑归 M1-025
- `data/combat/hit_zones.json`、`data/combat/friendly_fire.json`（+ schema）
- `crates/sim/tests/damage_*.rs`
### 要实现的行为
1. 射线/段与 4 胶囊求交，返回最先命中的胶囊 → 映射到 `HitZone`（头胶囊含颈上段；躯干胶囊含盆骨下段，按命中点高度分割）。
2. 头盔规则：武器弹种分类 `pistol/smg` → 头部 ×2.0；其余 ×3.0；步枪弹头部伤害 ≥ 100 → 必 KIA。
3. 距离衰减：以弹丸**飞行距离**（非直线距离）计算 `falloff`；超 F1 保持 `min`。
4. 状态变更输出：`DamageResult{hp_after, outcome: Hit|Downed|Killed, zone, damage_dealt, overkill}`；`Killed` 原因含 `headshot/overkill/explosion/vehicle`。
5. 友军伤害开关：`FriendlyFireMode::{Off, On}`（M1 默认 On，系数如上，TK 惩罚在 M2）。
6. 击杀归属输入：记录每个受害者的 `recent_damage: Vec<(attacker, amount, tick)>`（15 s 窗口）、`last_hit`；M1-016 据此产出 Kill/Assist/Finish。
7. 命中体历史：`record(tick, world)` 每 tick 写 100 玩家 4 胶囊（约 40 B/人）；`hitboxes_at(t, frac)` 线性插值；写入成本 ≈ 0.05 ms/tick（ARCHITECTURE §6.7.1）。
### 接口与数据
```rust
pub struct HitInfo { pub attacker: PlayerId, pub weapon: WeaponId, pub point: Vec3, pub dir: Vec3,
  pub travelled_m: f32, pub pen_mult: f32, pub zone: HitZone, pub kind: DamageKind /*Bullet|Explosion|Melee|Vehicle|Tank*/ }
pub fn compute_damage(h:&HitInfo, t:&TargetCtx, d:&GameData) -> DamageResult;
pub struct HitboxFrame { pub tick: Tick, pub players: SmallVec<[PlayerHitboxes; 128]>, pub vehicles: SmallVec<[VehicleHitboxes; 48]> }
```

## 不做什么
- 倒地/救起/流血状态机本体：M1-016；
- 回溯（rewind）逻辑与墙体拒绝：M1-025；
- 爆炸衰减公式：M1-015；载具装甲：M1-023；
- 压制：M1-017；TK 惩罚与计分：M2；
- 客户端命中反馈 UI/音：M1-057/M1-063。

## 验收标准
1. 致死发数表与 `/data` 一致（躯干命中，近距离 ≤F0 计倒地所需发数）：Garand 2、Kar98k 1、M1 Carbine 3（48×2=96<100）、Thompson 3（40×3）、MP40 3（36×3）、M1911/P38 3（45×3、34×3=102）（GD §6.6 TTK 表；`cargo test ttk_table`）。
2. 中距与远距：Garand 在 600 m（F1，mult 0.72）躯干伤害 = 70×0.72 = 50.4 ⇒ 倒地仍需 2 发；Carbine 250 m 伤害 48×0.55=26.4 ⇒ 倒地需 4 发（单测，表由数据派生）。
3. 头部：Garand 头部 70×3.0 = 210 ≥ 100 ⇒ 单发 KIA；MP40 戴盔头部 36×2.0 = 72 ⇒ 不单发致命（单测）。
4. 超杀：HP=30 受躯干 100 → HP=−70 ≤ −60 ⇒ KIA；HP=30 受 60 → HP=−30 ⇒ Downed（单测）。
5. 友军：Garand 躯干打队友 70×0.5=35；近战 0；手雷自伤 ×1.0（单测）。
6. 姿态命中盒：站/蹲/趴 的胶囊总高度 1.8/1.2/0.5 m（误差 < 1 cm）；探身时头胶囊水平偏移 0.35 m ±1 cm。
7. 命中体历史：16 tick 环写入 100 玩家的耗时 ≤ 0.1 ms/tick（`cargo bench -p sim hitbox_record`）；`hitboxes_at(t, 0.5)` 为相邻帧中点（单测，误差 < 1e-4 m）。
8. 所有随机/排序使用确定性规则；1 万 tick 回放 `state_hash` 两次一致。

## 测试要求
- 单元测试：zones 映射、倍率表、衰减边界（F0、F1、>F1）、ff 矩阵、超杀阈值、历史环绕与插值。
- 集成：与 M1-012 穿透联调：穿墙后命中 `pen_mult` 生效。
- 性能基准：`hitbox_record`、`compute_damage`（≤ 0.5 µs/次）。
- 回归：TTK 表快照入库（`tests/snapshots/ttk.json`），数据变更时 diff 可见。

## 性能预算
- 引用 ARCHITECTURE §6.10 阶段 7：历史缓冲写入 ≈ 0.1 ms/tick；阶段 4 回溯命中体检测并入 2.5 ms 总预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 的"致死发数表（近/中/远）"在 GD §6.6 只给近距 TTK 表，中/远表由 F0/F1/min 派生，属本任务细化，门槛以数据为准。
- GD §6.6 "步枪全射程内保持 ≥ 50 HP/发以满足 2 发倒地" 需核对：Kar98k F1 800 m mult 0.80 ⇒ 80 HP/发 ⇒ 2 发才 ≥100？实际 1 发 80 < 100 ⇒ 远距需 2 发（与"高致死率"一致，单测不要写死 1 发）。
- 命中盒胶囊半径/高度分割为细化（GD 仅给总高）：头 0.12 m 半径、躯干 0.25 m… 取值写 `hit_zones.json` 并登记 PLAN_ISSUES。
