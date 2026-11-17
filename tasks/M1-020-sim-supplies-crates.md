# M1-020 sim：Supplies 与补给箱

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：M
- 前置依赖：M1-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §12.1; GAMEDESIGN §12.2; GAMEDESIGN §12.3; GAMEDESIGN §12.6; GAMEDESIGN §6.11; BRIEF R6

## 背景
**R6：Supplies（建造货币，由补给箱承载）与 Manpower/Munitions/Fuel（指挥官资源池）是两套独立系统**；M1 只做前者（战略资源 M2）。本任务实现 Supplies 货币、Supply Crate、Ammo Crate、补弹规则与反滥用，并**提供 M1-019 `SupplyProvider` 的真实实现**，完成 E1-11 闭环：Officer 放置 Garrison → 队友经 Garrison 重生 → Support 放置 Supply/Ammo Crate → 队友补弹。
规则（GD §12，均【待调参】；经济项【待测 → REFERENCE_MEASUREMENTS §economy】）：
- **Supplies**：1 箱 = 100；建造时从工地 **15 m** 内所有 Supply Crate、卡车/半履带车载补给中扣除，不足禁止开工（M1 无卡车）。
- **Supply Crate**：内容 100，可部分消耗；仅 **Support** 可徒手搬运（每次 1 箱，移动 ×0.75，只持手枪）；放置/放下 1.5 s；每队地面箱总数 ≤ 40（超出时最早的消失）；HP 150（枪弹 ×0.1、爆炸 ×1.0；被毁则箱内 Supplies 丢失）；来源：HQ Supply Depot 每 45 s 生成 1 箱，库存上限 8；据点补给点（己方据点后方 1 个）每 90 s 1 箱、库存上限 4，仅本方持有时生成；Depot 库存满则不再生成。
- **Ammo Crate**：Support 放置（loadout 工具），每命 2 个，在 HQ/据点补给点/己方 Ammo Crate 旁重装；内容 600 弹药点；使用：4 m 内按住 1.5 s 补满到 loadout 初始量，点数不足则尽量补；单价（点）：手枪匣 1、冲锋枪匣 3、步枪弹夹/弹匣 4、BAR 匣 5、MG 弹箱 12、火箭弹 20、手雷 6、地雷 10、绷带 2、载具炮弹 40/发；放置冷却 30 s；寿命 150 s；HP 100。
- **反滥用**（§12.6）：不可在敌方 Sector 据点圆心 80 m 内放置 Supply Crate；Support 在非本队 Sector 仍可搬运；Depot 不溢出累积。
- 补给后弹药回满该 loadout 初始配置，不超出（GD §5.3、§6.11）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/supply/crate.rs` —— `SupplyCrate{id, team, pos, supplies, hp, ttl?, carried_by}`、搬运/放下
- `crates/sim/src/supply/ammo.rs` —— `AmmoCrate{points, hp, expires_at}`、`resupply(player, crate)`、单价表
- `crates/sim/src/supply/depot.rs` —— `Depot`（HQ）与 `SupplyPoint`（据点）生成逻辑
- `crates/sim/src/supply/provider.rs` —— `impl SupplyProvider for SupplyWorld`（15 m 内跨箱抽取，按距离从近到远、按箱 ID 升序，保证确定性）
- `crates/sim/src/supply/limits.rs` —— 地面箱 ≤ 40 的 FIFO 淘汰；放置位置限制
- `data/rules/supplies.json`（M1-007 已建；本任务读取并补充 Ammo 单价表）
- `crates/sim/tests/supply_*.rs`
### 要实现的行为
1. **Depot/据点补给点**：HQ Depot 以 tick 计时生成箱子（45 s=1350 tick）；库存 ≤ 8 时才继续生成；Support 在 HQ 内与 Depot 交互取箱（≤ 2 m，1.5 s）。据点补给点仅在所有者为本队时计时，易手后库存清零。
2. **搬运**：Support 交互拾取 1 箱 → 状态 `CarryingCrate`：移速 ×0.75、主武器收起仅手枪（供 M1-011/M1-013 读取标志位）；再次交互放下（1.5 s），放下点 ≥ 敌方据点 80 m（否则拒绝）。
3. **箱体生命周期**：地面箱数量 > 40 时淘汰最早；被摧毁则内容丢失；Supplies 被建造抽取后箱内数量递减，归零则箱移除。
4. **SupplyProvider**：`available(pos,15m)` 求和；`consume` 按距离升序逐箱扣除，原子性（不足则不扣并返回 false）；供 Garrison（M1-019）与后续 Engineer（M2）。
5. **Ammo Crate**：Support 放置（放置冷却 30 s、每命 2 个）；任意友军 4 m 内按住 1.5 s 补弹，按单价扣点；补满到 loadout 初始量；总点数 600，耗尽移除；150 s 寿命；HP 100。
6. **补弹规则**：每个弹匣/手雷/绷带逐项按单价判断；顺序：主武器弹匣 → 副武器 → 手雷 → 绷带；点数不足则按顺序尽量补；返回 `ResupplyResult{ items, points_spent }`。
7. 事件：`CratePlaced/CrateDestroyed/Resupplied`；`SpawnPointState` 不涉及；对客户端的 `Notice{code}`（如"补给不足"）。
8. 反滥用：位置限制校验函数 `can_place_supply_crate(team, pos)`；Ammo Crate 同点叠放 ≤ 3 个（细化）。
### 接口与数据
```rust
pub struct SupplyWorld { crates: Vec<SupplyCrate>, ammo: Vec<AmmoCrate>, depots: Vec<Depot> }
impl SupplyProvider for SupplyWorld { /* available / consume */ }
pub fn resupply(p:&mut PlayerInventory, c:&mut AmmoCrate, loadout:&LoadoutDef, costs:&AmmoCosts) -> ResupplyResult;
```

## 不做什么
- 卡车/半履带车载补给（600/300）与装卸：M2/M3；
- Supply Drop/Ammo Drop 空投：M3；
- Engineer 建造（节点/铁丝网等）与 Repair Station：M2；
- Commander 资源池（Manpower/Munitions/Fuel）：M2；
- Shell Stock 炮弹堆：M3；
- 客户端搬运/放置交互动画与 HUD 提示：M1-057/M1-060。

## 验收标准
1. 补给箱补弹量与 `/data` 一致：对 Garand 步枪手（8 发 ×10 夹）耗尽 6 夹后在 Ammo Crate 旁补弹，花费 6×4=24 点、补满到 10 夹（`cargo test ammo_resupply_rifle`）；SMG 手补 3 夹花费 9 点；手雷 Mk2 补 2 枚花费 12 点。
2. 点数不足时按顺序尽量补：箱剩 10 点、需补 3 夹步枪（12 点）→ 补 2 夹（8 点）并剩 2 点（单测）。
3. 600 点总量在耗尽后箱移除；寿命 150 s（4500 tick）±1 tick 到期移除（单测）。
4. Depot：45 s ±1 tick 生成 1 箱，库存 8 时停止，取走 1 箱后恢复生成；据点补给点 90 s、上限 4、易手后库存清零（单测）。
5. 搬运：携箱移速 ×0.75 ±1%（与 M1-011 联合断言）；放下耗时 1.5 s；携带期间拒绝切换主武器（单测）。
6. SupplyProvider：15 m 内两箱（100+60）建造 Garrison 己方区需 50：扣最近箱 50，剩 50+60；敌方区需 100：最近箱归零，距离次近的不动；不足时 `consume` 返回 false 且库存不变（单测）。
7. 反滥用：敌方据点圆心 80 m 内放箱被拒（79/81 m）；地面第 41 个箱使最早的消失；Ammo Crate 每命 2 个、冷却 30 s 内再放被拒（单测）。
8. **闭环演示测试**（`e2e_garrison_supply_loop`）：Support 取 Depot 箱 → 搬到 Officer 旁 → Officer 放置 Garrison（成功扣 50）→ 队友 8 s 后经 Garrison 重生 → Support 放置 Ammo Crate → 队友补弹；全程 sim 内无头断言；确定性哈希两次一致。
9. 性能：40 箱 + 6 Ammo 箱 + 100 玩家的补给阶段 ≤ 0.1 ms/tick。

## 测试要求
- 单元测试：单价表、补弹顺序、原子扣减、FIFO 淘汰、限时、冷却、位置限制。
- 集成：`e2e_garrison_supply_loop`；可选 headless 通过 server（待 M1-026）。
- 性能基准：criterion `supply`。
- 回归：`DATA_HASH` 与确定性哈希基线更新。

## 性能预算
- 并入 ARCHITECTURE §6.10 阶段 5（规则 ≤ 0.7 ms/tick）；本任务 ≤ 0.1 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §12.5 示例称"敌方区 100 即 1 整箱"，与 §10.2 一致；M1 无 Sector，按所在据点所有者近似（见 M1-019 备注）。
- 实体归属：箱体是否经 AOI 快照下发（`EntityKind::Deployable`）由 M1-024 决定；本任务只产出 sim 状态与事件。
- 经济数值待对标（B-01–B-03、B-07 等，M1-071 回填，需人工录像 H-053）。
