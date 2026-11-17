# M2-013 sim：M2 武器类（自动步枪、机枪、霰弹）

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：M
- 依赖：M2-006, M1-013
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.3; GAMEDESIGN §6.5; GAMEDESIGN §6.2; GAMEDESIGN §6.4; ARCHITECTURE §8.1; ARCHITECTURE §6.8.2

## 背景
M1-013 实现了全自动/点射/半自动、手枪、近战、后坐模式与战术/空仓装填。M2 新增三类武器行为：
1. **自动步枪 BAR**（`us_bar_m1918a2`）：双档射速 慢 350 / 快 550 rpm；慢档 vkick 1.1°、hσ 0.35°、bloom/发 0.40°；快档 vkick 1.3°、hσ 0.45°、bloom 0.50°；伤害 60（=70×0.85）；20 发弹匣，战术装 2.8 s/空仓 3.4 s；两脚架部署。StG 44 半自动/全自动（550 rpm）与 FG 42（750 rpm，20 发，vkick 1.7°）沿用既有自动武器逻辑。
2. **机枪**：M1919A4（500 rpm，250 发弹箱，换弹箱 6.5 s，dmg 53）、M1919A6（450 rpm，150 发，6.0 s）、MG 34（850 rpm，50 发鼓/75 发双鼓/250 链，换鼓 5.5 s/空 6.0 s）、MG 42（1200 rpm，50 发链段可串联至 250，换链 6.0 s/空 7.0 s，dmg 40，vkick 1.2°、hσ 0.65°）。持续射击模型：弹链供弹（`feed.type="belt"`，发射消耗 1 发/发）、换弹链（空仓换整条 vs 接链）、两脚架部署后后坐 ×0.35（部署判定在 M2-012；此处消费 `deployed` 标志）、未部署 sway 1.5°、曳光每 5 发 1 发（纯视觉，`SimEvent::Shot.tracer`）。**过热**：GD 没有给出过热数值（只提到 MG 42 含备用枪管），故实现为数据驱动的机制 `overheat{enabled,heat_per_round,threshold,cool_per_s,penalty}`，初值 `enabled=false`，并在 PLAN_ISSUES 登记缺口——**不得自行发明过热数值**。
3. **霰弹**（`us_m1897` Trench Gun）：12 号 00 号弹，每发 8 粒，每粒伤害 14，衰减每粒 8 m 起/30 m 止/保留 15%，泵动 cycle 0.9 s，5 发管式弹仓、逐发装填 0.65 s/发，散布半角 4°（弹丸锥内均匀随机），每粒 pen_power 25，近距 `vkick` 4.5°；有效射程 30 m。

随机数：`Pcg64Mcg`，种子 `hash(match_seed, tick, entity_id, "spread")`，客户端预测用同一公式并使用 `FireCmd.spread_seed`（ARCH §6.8.2）。射速上限与节奏【待调参 / 待测 → REF §fire-rate】。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/weapon/belt.rs` —— 弹链/弹箱/鼓供弹、换弹状态机（接链 vs 换箱）
- `crates/sim/src/weapon/firemode.rs` —— 扩展：BAR 双档（`rpm_modes:[350,550]`）、StG 44 半/全自动
- `crates/sim/src/weapon/shotgun.rs` —— 多弹丸发射、泵动循环、逐发装填（可中断装填）
- `crates/sim/src/weapon/overheat.rs` —— 数据驱动的枪管热模型（默认关闭）
- `crates/sim/src/weapon/sway.rs` —— 未部署 MG sway（振幅 1.5°，低频随机游走）
- `data/weapons/*.json` 增补 `rpm_modes`、`pellets`、`pellet_damage`、`pellet_falloff`、`overheat`、`reload.per_round_s`
### 要实现的行为
1. 射速：发射间隔按 `tick` 量化（30 Hz）；同一武器 cyclic rate 通过"累积小数 tick"保证长期平均误差 ≤ 2%（高射速 MG 42 1200 rpm = 每 tick 0.67 发，一 tick 可出 1 发或 0 发，用累加器）。
2. BAR 切换档：`aux` 位触发（M2-012 提供输入）；切换在两发之间生效。
3. 弹链/弹箱：剩余发数、换弹耗时（战术装 vs 空仓，GD §6.3）；弹箱掉到背包后继续累计总弹药；MG 总弹药 = 弹箱数 × 容量。
4. 霰弹：一次扣扳机 → 8 粒各自在 4° 锥内取方向，各自走 M1-012 的弹道/穿透；总伤害 = Σ 命中弹丸伤害（含头/肢体倍率，M1-014）；衰减按每粒距离计算；泵动期间不可再射；逐发装填可被开火中断。
5. 曳光：每武器 `shot_index % 5 == 0` 标记；仅事件字段，不影响判定。
6. 过热机制：`enabled=false` 时 100% 无影响（回归测试保证与 M1 行为一致）。
### 接口与数据
- `WeaponState{ ammo_in_mag:u16, fire_mode_idx:u8, heat:f32, pump_ready_tick:u32, deployed:bool }`；`fn fire(&mut self, ctx:&FireCtx) -> SmallVec<[Projectile;8]>`（霰弹返回 8 个）。

## 不做什么
- 两脚架部署判定与后坐 ×0.35 的应用开关 → M2-012；火箭/地雷/炸药 → M2-014；载具机枪 → M1-023/M2-015。
- 弹药补给箱对 MG 弹箱的补给量（GD §12.3：MG 弹箱 12 弹药点）→ M1-020 已覆盖，本任务只读取。
- 压制效果数值 → M1-017。

## 验收标准
1. cyclic rate 误差 ≤ 2%：headless 连续射击 600 发，实测 rpm 与 /data 比较：BAR 慢 350、BAR 快 550、M1919A4 500、M1919A6 450、MG 34 850、MG 42 1200、StG 44 550、FG 42 750，共 8 项全部 ≤ 2%（`cargo test -p sim cyclic_rate_m2`）。
2. 霰弹：固定种子 10 000 次齐射，对 30 m 外 1.8 m × 0.5 m 的目标命中盒，平均命中弹丸数与由 /data 解析计算的期望值差 ≤ 5%（期望由测试内以锥面积/目标面积公式算出）；8 m 内每粒伤害 = 14，30 m 处 = 14×0.15 = 2.1（±0.1）。
3. 泵动 cycle 0.9 s（27 tick）与逐发装填 0.65 s（≈ 20 tick）误差 ≤ 1 tick；装填中途开火立即中断并保留已装入发数。
4. 弹箱换弹耗时与 /data 一致：M1919A4 6.5 s（195 tick）、MG 42 战术 6.0 s / 空 7.0 s，误差 ≤ 1 tick。
5. 过热 `enabled=false` 时，M1 回归测试（`cargo test -p sim weapon_m1_regression`）全通过且 `state_hash` 与 M1 基线一致。
6. 同种子同输入，两次运行霰弹弹丸方向序列逐位相同；wasm 与 native 的方向差 ≤ 1e-4 rad（信息性比较，R5）。
7. 性能：MG 42 持续 30 发/s × 40 门，射击系统 ≤ 0.1 ms/tick（`cargo bench -p sim fire_m2`）。

## 测试要求
- 单元测试：累加器边界（tick 对齐）、换弹中断、空弹匣、弹链串联至 250、霰弹 pellet 数量守恒。
- 集成/无头：headless 5 min 全自动射击对靶，统计 rpm 与伤害累计。
- 性能基准：`fire_m2`。
- 回归：M1 武器 `state_hash` 基线不变。

## 性能预算
ARCH §6.10 阶段 4「武器与弹道 2.5 ms（≤ 400 活动弹体段扫描）」：新增霰弹每发 8 粒弹丸按 8 段计入同一预算；本任务不得使阶段 4 基线回归 > 2%。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 过热数值缺口：GD 无数字 → PLAN_ISSUES，待 Fable 或 M2-054 对标测量裁定。
- 曳光标志在线协议中无字段（ARCH §6.8.4 的 `Shot`）；客户端可由弹序推导，是否加位交 Fable（协议"细化"）。
- 霰弹 8 粒在 100 人密集战的弹体预算风险：若超 ARCH §6.10 阶段 4 预算，把每发弹丸合并为"扇形扫描"并写 PLAN_ISSUES。
