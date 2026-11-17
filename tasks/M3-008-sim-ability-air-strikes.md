# M3-008 sim：Bombing / Strafing / Precision

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：L
- 前置依赖：M3-006, M2-014
- 素材需求：无（灰盒即可；飞机用十字形灰盒，渲染在 M3-030）；素材状态：n.a.
- 引用文档：GAMEDESIGN §13.4; GAMEDESIGN §13.5; GAMEDESIGN §13.6; GAMEDESIGN §13.1; GAMEDESIGN §6.6; GAMEDESIGN §14.4.3; GAMEDESIGN §8.1

## 背景
三项空袭能力共享"飞机实体 + 条带/单点落弹 + 延迟 + 敌方预警 + 结算"结构。M2-014 已实现爆炸伤害（Bazooka/炸药包的对载具/建筑伤害）与 HEAT 判定，本任务复用爆炸公式 `D = Dmax × (1 − (r/R)²)`（GD §6.6）并加入空袭专用参数。M3-006 提供资源/冷却/空中锁/飞机上限框架与 `AirWarning` 事件。
参数（GD §13.2/13.4–13.6，全部来自 `/data/abilities`，【待调参】）：
- Bombing Run：Munitions 350 + Fuel 120，冷却 600 s，延迟 45 s；点选两点定义条带（长度固定 400 m，以第一点为中心沿方向 ±200 m，宽 70 m）；3 架依次飞越，每架 6 s 内每 0.33 s 投 1 枚，共 18 枚随机落入条带；对步兵 `Dmax 400 / R 14 m`，载具直击（≤ 4 m）600 HP、近失 200，建筑 1500 HP。
- Strafing Run：Munitions 150 + Fuel 60，冷却 360 s，延迟 25 s；条带 300 × 14 m；2 机编队飞越 3 s，每 0.025 s 一个落点，落点半径 1.8 m；步兵 90、载具 25（对 ≤ 20 mm 装甲有效）、建筑 30；每落点压制半径 20 m（`S += 12`）；站立玩家条带内命中概率约 25–30%。
- Precision Strike：Munitions 200 + Fuel 80，冷却 420 s，延迟 22 s；单点 1 枚；步兵 `Dmax 800 / R 22 m`，载具直击（≤ 5 m）1200，建筑 3000；点选显示 22 m 红圈。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/commander/effects/air_strikes.rs` —— `BombingRunEffect`、`StrafingRunEffect`、`PrecisionStrikeEffect`。
- `crates/sim/src/aircraft.rs` —— `Aircraft` 实体（无碰撞，仅位置/航向/eta；`EntityKind::Aircraft` 如 protocol 无该类型则用 `Deployable` 子类，PLAN_ISSUES 记）。
- `crates/sim/src/explosion.rs`（M2-014 已有）—— 增加 `ExplosionSource::AirStrike` 与 `structure_damage`、`vehicle_direct_hit` 参数化。
- `crates/sim/tests/air_strikes.rs`。
### 要实现的行为
1. **条带几何**：两点→方向向量；Bombing 条带中心 = 第一点，沿方向 ±200 m；Strafing 同理 ±150 m；宽度按数据。落弹点在条带矩形内均匀随机（种子 `Rng::for_(tick, ticket.id, purpose)`）。
2. **时间线**：下达后 `delay_s` 到达（Bombing 45 s、Strafing 25 s、Precision 22 s）；到达前 15 s（`warn_enemy_s`）向敌方发 `AirWarning`（以目标为中心半径 200 m 的模糊环，不暴露方向）；到达前取消返还 50%（M3-006）。
3. **Bombing**：3 架依次进入，每架 6 s、每 0.33 s（=10 tick）一枚，共 18 枚；每枚触发爆炸结算；留下弹坑事件（地形变形仅视觉，`Explosion{kind: crater}`）。
4. **Strafing**：3 s 内每 0.025 s（<1 tick，一 tick 内触发 1–2 个落点，用累加器保证总数 = 120）；落点命中判定为半径 1.8 m 圆内；对步兵 90 HP、载具 25 HP、建筑 30 HP；每落点对半径 20 m 内玩家 `suppression += 12`（走 M1-017 接口）。
5. **Precision**：单发，步兵 `Dmax 800 / R 22 m`；载具直击半径 5 m 内 1200 HP；建筑 3000 HP。
6. **载具/建筑结算**：顶部装甲按 GD §14.4.3，空袭炸弹对载具顶部视为击穿（600/1200 HP 直接作用于 `hull_hp`，并按 §14.4.4 的内部损伤随机化模块）；对建筑只扣建筑 HP（破坏阶段由 M3-021 消费）。
7. **友军误伤**：爆炸 `ff_mult = 0.7`；25 m 内友军仅提示不阻止（ticket.friendly_danger）。
8. 空袭不可被击落；飞机实体离开条带后移除。
### 接口与数据
- 与 M3-006 的 `AbilityEffect` trait 对接；爆炸事件复用 `protocol::Explosion{pos, kind, radius_q}`，新增 `kind` 取值 `bomb_small/bomb_large/strafe_hit` 由 M3-005 提供。

## 不做什么
- 飞机渲染/落弹表现/警报音 → M3-030、M3-038；UI 选点预览 → M3-033；
- Recon Flight 与 Supply/Ammo Drop → M3-009/M3-007；建筑破坏阶段切换 → M3-021。

## 验收标准
1. 脚本化断言：Bombing 一次释放共生成 **18** 枚炸弹、总持续（首枚到末枚）≤ 6.0 s×3 架序列内、条带尺寸 400×70 m；Strafing 落点总数 120 ±1、条带 300×14 m；Precision 1 枚、爆炸半径 22 m（均与 `/data` 与 GD §13.2 一致）。
2. 1 000 次 Bombing 采样：全部落点在条带矩形内（违规数 0）；沿条带长度方向 4 分位占比各 25% ±5%。
3. Strafing 对站立不动的玩家（条带中线）的命中概率在 20%–35% 区间（5 000 次蒙特卡洛，GD §13.5 期望 25–30%）。
4. Precision 对步兵：r=0 受 800 HP（KIA）、r=11 m 受 `800×(1−0.25)=600`、r=22 m 受 0；载具直击 4.9 m 内 1200 HP（单测）。
5. 友军误伤：同一爆炸对友军伤害 = 敌军伤害 × 0.7（误差 < 1 HP）。
6. 全局锁/上限的联动：连续释放第 4 次空袭被 `aircap` 拒绝（集成测试）。
7. 同 seed 两次运行 `state_hash` 逐 tick 相同；单次 Bombing 峰值 tick 处理 ≤ 0.3 ms（`criterion`）。

## 测试要求
- 单元测试：条带几何（含斜向）、时间线、取消返还、友军误伤、装甲顶部结算、建筑 HP。
- 集成/无头：`headless-client` 释放三项空袭，客户端收到 `AirWarning`（敌方）与 `Explosion` 序列，事件数与上述一致。
- 性能基准：`crates/sim/benches/air_strike.rs`。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 4（武器与弹道 2.5 ms）与阶段 5（规则 0.7 ms）：空袭活动期间额外 ≤ 0.3 ms/tick（峰值），平时 0。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：若超限，先做 Bombing+Precision，Strafing 拆为 `M3-008b`（README §5.6）。
- GD §13.5 的"每 0.025 s 一个落点"小于 1 tick（33.3 ms），需用累加器；这是 30 Hz 下的实现细节，写 PLAN_ISSUES 细化。
- 延迟/冷却体验长度待 M3-052 校准；数值只改 `/data`。
