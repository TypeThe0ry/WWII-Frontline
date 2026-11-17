# M2-014 sim：火箭、地雷与炸药

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：L
- 前置依赖：M2-013, M1-015, M1-023
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.9; GAMEDESIGN §14.4; GAMEDESIGN §14.2; GAMEDESIGN §12.4; GAMEDESIGN §5.12; GAMEDESIGN §5.13

## 背景
把 GD §6.9.3–6.9.5 的反装甲/爆破武器落地。弹丸沿用 M1-012 的弹道与 M1-023 的装甲判定；爆炸伤害沿用 M1-015 的 `D = Dmax × (1 − (r/R)²)`。所有数值【待调参】，读自 `/data`。

**发射器（GD §6.9.4，HEAT，穿深不随距离衰减，`drop=0`）**

| 项 | Bazooka M1A1 | Panzerschreck | Panzerfaust 60 |
|---|---|---|---|
| 初速 | 82 m/s | 110 m/s | 45 m/s |
| 有效射程 | 150 m | 150 m（200 m 下坠明显） | 60 m（最大约 80 m） |
| 飞行 | 推力段 ≤ 20 m 无下坠，之后重力 | 同 | 抛物线（无推力） |
| HEAT 穿深 | 90 mm | 200 mm | 200 mm |
| 装填 | 4.5 s（有 Loader 2.5 s） | 5.0 s | 一次性 |
| 后焰 | 后方 12 m、15° 锥、60 HP/s、0.3 s | 20 m 锥、120 HP | 5 m、30 HP |
| 对建筑 / 步兵爆炸 | 800 / R4 Dmax150 | 1200 / R5 Dmax180 | 1000 / R5 Dmax150 |
| 结构伤害（装甲穿透后） | 600 | 900 | 900 |

后焰需检查背后 5 m 内是否有墙，有墙则弹回对发射者 40 HP。Support 可作 Loader（交互加速）。HEAT 判定（GD §14.4.1）：`φ_eff = φ`（不做 normalization）；`φ_eff ≥ 75°` 跳弹；过匹配（口径 ≥ 3×板厚）不跳弹且 `φ_eff = min(φ_eff,45°)`；`t_eff = t/cos φ_eff`（上限 ×3）；`P×U(0.95,1.05) ≥ t_eff` 击穿；Pz IV H 裙板区 HEAT 穿深 ×0.6（GD §14.3.3 说明）。

**地雷（GD §6.9.5）**：AP（US M2A4 弹跳雷 / GER S-Mine 35）：步兵靠近 ≤ 0.9 m 触发，弹起 1.2 m 爆炸 Dmax180/R6，布设 2.0 s，武装延迟 3 s，解除 1.5 s；AT（US M1A1 / GER Tellermine 43）：重量 ≥ 1 t 的车辆压发，对触发侧履带/轮 200 HP + 车体 300 HP，吉普/轻卡直接摧毁，布设 3.0 s，武装延迟 5 s，解除 3.0 s，步兵不触发；HP 1，≥ 30 HP 爆炸且 ≤ 2 m 可殉爆，枪击无效；己方也触发；全队同时 AP ≤ 20、AT ≤ 16（GD §12.4）；对敌不可见，受雷伤者队友 5 s 内小地图看到爆点；探雷/拆雷由 M2-012 的 Engineer 工具调用本任务接口。
**炸药包（GD §6.9.3）**：Satchel/Geballte Ladung，Engineer 携带；贴放 ≤ 2.5 m、放置 1.5 s、引信 6 s（可手动提前起爆；脱离 40 m 前可 3 s 拆除）；对载具 700 HP 作用于贴附点模块并 50% 传相邻模块、忽略装甲；对建筑/工事 2500 HP；步兵 Dmax300/R10；抛掷 12 m 仅地面爆炸、对载具 ×0.4；全队未引爆 ≤ 8。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/weapon/launcher.rs` —— 发射器状态、装填（含 Loader）、后焰、一次性弹药
- `crates/sim/src/projectile/heat.rs` —— HEAT 弹丸（推力段/抛物线）、命中调用 `armor::resolve_hit(HeatKind)`（复用 M1-023）
- `crates/sim/src/mine.rs` —— `Mine` 实体、触发/武装/殉爆/解除/可见性
- `crates/sim/src/explosive.rs` —— `Satchel`：贴附、计时、起爆、结构伤害分配
- `data/weapons/{us_bazooka_m1a1,ger_panzerschreck,ger_panzerfaust_60,us_satchel,ger_geballte_ladung,us_ap_mine_m2a4,ger_s_mine_35,us_at_mine_m1a1,ger_tellermine_43}.json`（M2-006 已建的字段；缺项在此补）
### 要实现的行为
1. 发射/装填/后焰：后焰锥内（12/20/5 m）按持续时间对玩家与可燃物（含己方）结算；背后 5 m 墙壁射线检测。
2. HEAT 命中走 M1-023 装甲管线：区域识别、入射角、跳弹、过匹配、击穿后内部损伤（`dmg_pen = 结构伤害×U(0.9,1.1)`、`internal_len = 0.6 + P_rem/40`）。
3. 对建筑/结构：直接伤害按表（800/1200/1000），经 `structure_armor`（爆炸 1.0）。
4. 地雷：布设校验（地面、非水体、数量上限）、武装计时、触发判定（距离或车辆重量 ≥ 1 t）、爆炸与坑；殉爆链；己方 Engineer 可见己方雷的标记数据。
5. 炸药：贴附点 → 命中模块（M2-016 的模块 ID；M2-016 前落到 `hull_hp`）；引信、提前起爆、拆除、全队计数。
6. 事件：`SimEvent::Explosion{pos,kind,radius}`、`MineTriggered`、`LauncherFired`（炮口闪光，GD §17.2：火箭发射暴露 5 s、位置误差 25 m）。
### 接口与数据
- `enum LauncherKind{Bazooka,Panzerschreck,Panzerfaust}`；`Mine{ kind:AP|AT, owner:PlayerId, armed_at:Tick, pos:Vec3, hp:f32 }`；`Satchel{ attached:Option<(EntityId,ModuleId)>, fuse_tick:Tick }`。

## 不做什么
- 载具模块损伤细节与起火 → M2-016；AT 炮 → M2-011；M7 枪榴弹（us_at_3）→ 不在 M2 范围，loadout 若存在则标 `m3_only`。
- Engineer 探雷/拆雷操作输入 → M2-012；火箭/爆炸音画 → M2-033/M2-046。
- 地雷在 bot 导航的避让（B-01 已知雷）→ M2-027。

## 验收标准
1. HEAT 对装甲判定与 GD §14.4.2 手算一致（单测向量）：Bazooka（90）vs Tiger 侧板 t=80 → 击穿；Bazooka vs Tiger 前上 t_eff=100/cos9°=101.2 → 不击穿；Panzerfaust（200）vs Jumbo 前上 t_eff=101/cos47°=148.1 → 击穿；vs Pz IV H 裙板区 P=120 vs t=30 → 击穿。
2. 跳弹：HEAT 入射 76° 跳弹、74° 不跳（过匹配除外）。
3. 初速/射程：Bazooka 弹丸 150 m 处飞行时间与解析解误差 ≤ 3%（固定种子）；Panzerfaust 80 m 处下坠 ≥ 与 /data 公式一致。
4. 后焰：站在 11 m 后方 15° 锥内的玩家 0.3 s 内受 ≈ 18 HP（60 HP/s×0.3）；13 m 处 0 伤；背后 4 m 有墙发射者受 40 HP。
5. 地雷触发规则单测：AP 0.9 m 触发、1.0 m 不触发；AT 对步兵不触发、对 ≥ 1 t 车辆触发；武装延迟 3 s/5 s 内不触发；殉爆条件（30 HP/2 m 边界）；枪击无效；己方触发；全队上限 AP 20/AT 16。
6. 炸药：载具 700 HP 作用于贴附点模块、相邻 50%；建筑 2500 HP 摧毁 1200 HP Garrison；抛掷对载具 ×0.4；全队未引爆第 9 枚被拒；引信 6 s = 180 tick。
7. 爆炸伤害曲线与 /data 一致：Mk2/Satchel/地雷在 r = 0、R/2、R 的伤害 = Dmax、0.75·Dmax、0。

## 测试要求
- 单元测试：上述全部；proptest：爆炸伤害单调递减。
- 集成/无头：headless 脚本：AT 兵种击毁 Sherman；Engineer 布 AT 雷炸 Jeep。
- 性能基准：`cargo bench -p sim mines_tick`（100 颗地雷）≤ 0.05 ms/tick。
- 回归：确定性哈希。

## 性能预算
ARCH §6.10 阶段 4「武器与弹道 2.5 ms」不回归；地雷触发检测用空间网格（64 m 格）以避免 O(玩家×地雷)。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；若发射器+地雷+炸药超一次会话，拆续作 M2-014b（炸药与地雷）。
- 数值未经史实复核（M2-053），穿深保持 `initial`。
- M1-015 的 `Dmax×(1−(r/R)²)` 为共用函数，不要复制。
