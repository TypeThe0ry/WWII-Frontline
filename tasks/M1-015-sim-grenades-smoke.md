# M1-015 sim：手雷与烟雾

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：M
- 依赖：M1-013, M1-014
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.9; GAMEDESIGN §6.6; GAMEDESIGN §7.4; ARCHITECTURE §6.10; ARCHITECTURE §12.3

## 背景
实现 M1 的两类投掷物：**破片手雷**（US Mk 2、GER M24）与**烟雾**（US M18、GER Nb.Hgr.39）。数据由 M1-008 提供，所有数值【待调参】。规则（GD §6.9.1–6.9.2）：
- 破片：Mk 2 引信 4.0 s、投速 18 m/s（45° ≈ 33 m）、Dmax 220 HP/R 8.0 m；M24 引信 4.5 s、20 m/s、Dmax 200/R 8.5 m。反弹系数 0.4、摩擦 0.6；可**烹饪**（按住投掷计时，松开投出，剩余引信 = 总引信 − 烹饪时间；超时在手中爆炸，自伤 ×1.0）。无直击伤害。
- 爆炸伤害 `D = Dmax × (1 − (r/R)²)`，`r` 为受害者胸腔中心到爆心的距离；需视线：爆心到目标射线被实体挡则 ×0.25，被薄木/帆布挡则 ×0.6（GD §6.6）；≥ 150 HP 瞬间 KIA；友军 `ff_mult` 爆炸 ×0.7；对载具每枚手雷 15 HP（仅对敞篷乘员有意义）。
- 烟雾：M18 引信 2.0 s 后冒烟，云团最大半径 7.0 m（3 s 展开），持续 45 s（末 8 s 衰减）；Nb.Hgr.39 6.0 m、30 s。**阻挡视线**（玩家视线 > 2 m 看不到；bot 视线/LoS 被阻断），**不阻挡弹丸、不压制**；云团为球体 + 地面截头；风忽略。
- 爆炸同时对压制产出事件（M1-017：`S += 40 × (1 − r/R_supp)²`，`R_supp = 15 m`，被墙隔挡 ×0.5）。
- 服务器每 tick 预算：手雷在阶段 4/5 内，单枚爆炸 ≤ 0.2 ms；同时存活投掷物 ≤ 40。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/throwables/grenade.rs` —— 投掷弹道（重力、反弹/摩擦、碰撞用 Rapier 球体查询）、引信、烹饪
- `crates/sim/src/throwables/explosion.rs` —— 爆炸伤害与视线遮挡衰减、产出 `Explosion` 事件与 `SuppressEvent` 输入
- `crates/sim/src/throwables/smoke.rs` —— 烟雾云状态机（Deploying/Active/Fading）与 `smoke_blocks_los(from,to)`
- `crates/sim/src/throwables/mod.rs` —— `ThrowableSystem`；与 `WorldView::los` 集成（烟雾内返回遮蔽）
- `crates/sim/tests/throwables_*.rs`；`crates/sim/benches/explosion.rs`
### 要实现的行为
1. 投掷：按下 gadget 键进入"持握"（烹饪计时），松开投出；投掷方向 = 视线方向 + 向上 5°；位置从手部偏移；速度读 `/data`。
2. 投掷物为服务器权威实体（`EntityKind::Projectile`），30 Hz 步进，4 子步积分；碰撞后按反弹/摩擦更新；静止判定速度 < 0.3 m/s 持续 3 tick。
3. 引信到期爆炸：对半径 R 内所有玩家（含载具内乘员暴露度规则：舱盖开启才受伤，M1 载具为 Sherman，闭舱无伤）求 `D`；视线检测用 `los` 并按材质给 ×0.25/×0.6；输出每个受害者的 `HitInfo{kind:Explosion}` 交给 M1-014 结算（`Killed(explosion)` 直接 KIA）。
4. 手中爆炸：烹饪超时 → 在持有者位置爆炸，自伤 ×1.0；持有者死亡时手雷在尸体处继续倒计时。
5. 烟雾：投出后 2.0 s 冒烟，3 s 内半径线性扩至 7.0 m，保持到总持续 45 s 前 8 s 起半径/浓度线性衰减；`smoke_blocks_los` 判定线段是否穿过任一**浓度 > 0.5** 的云球；`WorldView::los` 内部调用它。
6. 烟雾对 Medic 救治的遮蔽（GD §7.4）：提供 `target_visibility_through_smoke`，M1-016/bots 使用。
7. 事件：`ThrowableThrown`、`Explosion{pos,kind,radius_q}`、`SmokeStarted/Ended`；与 M1-010 消息字段对齐。
8. 弹药：手雷数量扣减在 loadout 的 gadget 计数上；空手雷禁止投掷。
### 接口与数据
```rust
pub struct ThrowableDef { pub fuse_s: f32, pub throw_speed_mps: f32, pub dmax: f32, pub radius_m: f32,
  pub restitution: f32, pub friction: f32, pub kind: ThrowKind /*Frag|Smoke*/, pub smoke: Option<SmokeDef> }
pub fn explosion_damage(dmax: f32, r: f32, big_r: f32, occlusion: f32) -> f32   // D = dmax*(1-(r/R)^2)*occ
pub fn smoke_blocks_los(w:&SmokeWorld, a:Vec3, b:Vec3) -> bool
```

## 不做什么
- 手雷/炸药包/地雷/Bazooka/Panzerfaust 的其余类：炸药包、地雷属 M2；**Panzerfaust 60 的发射与 HEAT 判定**：INDEX 未明确归属，建议并入 M1-023（见其备注）；
- 压制累积与效果：M1-017（本任务只发爆炸输入）；
- 客户端投掷动画/爆炸特效/声音：M1-047、M1-063；
- 烟雾对 bot 感知的使用：M1-038。

## 验收标准
1. 爆炸伤害-距离曲线与 `/data` 一致：Mk 2 在 r = 0/2/4/6/8 m 的 D = 220/211.7/165/121/0（`Dmax×(1−(r/8)²)`，误差 < 0.5 HP；`cargo test explosion_curve`）。
2. 遮挡：同一 r=3 m 下，隔实心墙 D×0.25、隔薄木 ×0.6、无遮挡 ×1.0（单测 3 例）。
3. 引信与烹饪：Mk 2 烹饪 1.5 s 后投出，爆炸发生在投出后 2.5 s ±1 tick；烹饪 4.2 s 在手中爆炸并自伤（单测）。
4. 投掷距离：Mk 2 以 45° 投出落点 ≈ 33 m ±10%（平地，忽略阻力）。
5. 烟雾半径曲线：冒烟 +1.5 s 半径 3.5 m ±0.2；+3 s 为 7.0 m；总持续 45 s ±1 tick，最后 8 s 内半径单调下降（单测）。
6. 烟雾内 LoS：穿过浓度 > 0.5 云球的线段 `los==false`，不穿过为 `true`；弹丸射线不受影响（`ballistics` 穿透不读烟雾，单测）。
7. 友军爆炸 ×0.7、自伤 ×1.0（单测）；≥150 HP 的爆炸伤害直接 KIA（r=3 m Mk 2：D=220×(1−0.141)=189 ⇒ KIA）。
8. 性能：同时 40 枚投掷物步进 + 1 次爆炸（30 人在半径内）≤ 0.5 ms（`cargo bench -p sim explosion`）。

## 测试要求
- 单元测试：伤害曲线、遮挡分级、烹饪边界、反弹（restitution 0.4）、烟雾状态机、`smoke_blocks_los`。
- 集成/无头：headless 玩家投掷并断言 `Explosion` 事件与受害者 HP。
- 性能基准：criterion `explosion`。
- 回归：确定性哈希（含反弹随机偏移时用 `Rng::for_`）。

## 性能预算
- 引用 ARCHITECTURE §6.10 阶段 4/5 的总预算（2.5 ms、0.7 ms）；本任务增量 ≤ 0.5 ms/tick（最坏）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **文档内部不一致（需 PLAN_ISSUES）**：GD §6.9.1 写"致命半径（≥100 HP）≈ 3.2 m（Mk 2）"，但按其公式 `D=220×(1−(r/8)²) ≥ 100` 的半径为 ≈ 5.9 m（≥150 HP 为 ≈ 4.5 m）。本任务以**公式**为准，致命半径表项视为描述性数值，不作验收；请 Fable 裁决。
- 烟雾"浓度"模型为细化（GD 只给半径与时长）；`>0.5` 阈值写入 `/data` 并登记 PLAN_ISSUES。
- Panzerfaust 缺任务归属见上文；由总控确认。
