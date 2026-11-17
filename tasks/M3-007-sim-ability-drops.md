# M3-007 sim：Supply Drop 与 Ammo Drop

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 前置依赖：M3-006, M1-020
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §13.3; GAMEDESIGN §13.2; GAMEDESIGN §12.2; GAMEDESIGN §12.3; GAMEDESIGN §12.6

## 背景
M3-006 提供了能力通用流程（资源/冷却/空中锁/延迟）。M1-020 已实现 Supplies 货币与 Supply Crate/Ammo Crate（含搬运、放置、补弹规则、反滥用）。本任务实现这两个"空投"能力的 `AbilityEffect`：飞机沿固定航线飞过，在设定圆内随机落下箱子，并与既有补给系统衔接（R6：Supplies 与战略资源是两套系统，Supply Drop 消耗 Manpower+Fuel，产出 Supplies）。
规则表（GD §13.2）：Supply Drop = Manpower 100 + Fuel 20，冷却 240 s，延迟 15 s，4 个 Supply Crate（共 400 Supplies），散落半径 15 m，箱子寿命 300 s；Ammo Drop = Munitions 120，冷却 180 s，延迟 15 s，2 个 Ammo Crate（共 1200 弹药点，即每箱 600），散落半径 10 m，箱子寿命 120 s。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/commander/effects/drops.rs` —— `SupplyDropEffect`、`AmmoDropEffect`（实现 `AbilityEffect`）。
- `crates/sim/src/supply/crate.rs`（M1-020 已建）—— 增加 `spawn_air_crate(kind, pos, ttl_ticks, parachute: bool)` 接口与降落伞下降状态（下降速度 ≈ 5 m/s，初值【待调参】，落地前不可拾取）。
- `crates/sim/tests/ability_drops.rs`。
### 要实现的行为
1. **落点校验**（在 M3-006 之外）：落点须在本方或战区 Sector；距敌方据点圆心 ≥ `R_sp+80 m`（GD §13.3）；否则 `invalid_pos`。
2. **航线**：固定航线 = 以落点为中心、沿"航向"（默认从己方 HQ 指向落点）直线飞过，飞机实体无碰撞、不可被击落（v1 无防空）；飞机实体仅承载 `eta` 与视觉，位置 = 线性插值。
3. **落箱**：延迟到期时在半径 15 m（Supply）/10 m（Ammo）圆内用 `Rng::for_(tick, ticket.id, purpose)` 均匀随机落 4/2 个箱；箱类型 Supply Crate（100 Supplies/箱）/Ammo Crate（600 弹药点/箱）。
4. **寿命**：Supply 箱 300 s、Ammo 箱 120 s，到期移除（不返还）；落地时发出烟雾标识事件（Supply = 彩色烟雾 20 s，Ammo = 另一种颜色；只发事件，渲染 M3-031）。
5. **拾取**：箱子可被敌人拾取（Supply 可搬运；Ammo 可使用），不加阵营限制（GD §13.3）；与 M1-020 的"每队地面 Supply Crate ≤ 40，超出时最早的消失"上限规则一致。
6. 无敌方警告（仅飞机声音，<400 m 可听），故不发 `AirWarning`。
### 接口与数据
- 读取 `/data/abilities/{supply_drop,ammo_drop}.json`（M3-003）；散布圆半径、箱数、寿命全部来自数据，禁止硬编码。

## 不做什么
- 能力通用流程 → M3-006；空袭 → M3-008；渲染/烟雾/降落伞 → M3-031；UI → M3-033。
- Supply Crate 搬运/反滥用规则本体 → M1-020（本任务仅复用）。

## 验收标准
1. 统计 1 000 次空投：箱落点到落点圆心距离的 `max ≤ 15.0 m`（Supply）/ `≤ 10.0 m`（Ammo），且均匀圆盘分布（半径² 的 KS 检验 p > 0.01 或 4 分位偏差 ≤ 10%）。
2. 单次空投生成箱数 = 4（Supply，总 400 Supplies）/ 2（Ammo，总 1 200 弹药点）；箱子寿命误差 ≤ 1 tick。
3. 延迟 15 s ±1 tick 后落箱；取消（到达前）返还 50%，不落箱。
4. 超出每队 40 箱上限时最早箱消失（单测）；敌方可拾取（单测断言）。
5. `state_hash` 同 seed 同输入逐 tick 相同。
6. 单次落箱处理 ≤ 0.02 ms（`criterion`）。

## 测试要求
- 单元测试：落点校验 4 个边界（Sector 外、敌据点 79/81 m）；箱数/寿命；取消返还；降落伞下降期间不可拾取。
- 集成/无头：headless 注入 Supply Drop，客户端收到 `AbilityExecuted(done)` 与 4 个实体出现。
- 性能基准：`crates/sim/benches/drops.rs`。
- 回归：确定性哈希更新。

## 性能预算
规则阶段 ≤ 0.7 ms/tick（ARCHITECTURE §6.10 阶段 5）；本任务单次触发 ≤ 0.02 ms，常态 0。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §12.2 把 Supply Drop 写成"见 §13.1"，实际规则在 §13.3；以 §13.3 为准。
- 空投机的视觉素材缺失时用十字形灰盒（V-VEH-AIR-* 占位，渲染见 M3-030）。
- 降落伞下降速度 GD 未给出，属本任务"细化"，写 PLAN_ISSUES。
