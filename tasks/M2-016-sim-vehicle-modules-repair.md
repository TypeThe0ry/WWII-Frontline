# M2-016 sim：载具模块损伤与维修

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：M
- 依赖：M2-015, M2-010
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §14.5; GAMEDESIGN §14.8; GAMEDESIGN §14.4.4; GAMEDESIGN §12.4; GAMEDESIGN §5.1

## 背景
M1-023 已有"基础模块损伤"。本任务补完 GD §14.5 的完整模块系统与 §14.8 的维修。

**模块表（GD §14.5.1，【待调参】）**：Engine HP 300（轻 200、重 400）：≤50% 最大速度/加速 ×0.6；=0 停转，若被击穿则 40% 概率起火。Track L/R HP 250（轮式 Tires 4 组各 100）：≤50% 该侧推力 ×0.6 并向受损侧偏转；=0 该侧失驱，只能向该侧转圈、速度 ×0.3，两侧全毁不能动；Tires 每组毁最高速 ×0.75（下限 0.3）。Turret Drive HP 200：≤50% 旋转 ×0.5；=0 炮塔卡死，仅手动 3°/s。Main Gun HP 250：≤50% 装填 ×1.4、散布 ×1.5；=0 无法开火。Ammo Rack HP 150：=0 触发殉爆判定，不可修。乘员 100 HP，KIA 后座位空出可补位。
**击毁/起火（§14.5.2）**：`hull_hp=0` 或殉爆 → Burning：非殉爆击毁燃烧 8 s，乘员可逃（离车受 20 HP 燃烧伤害），8 s 后爆炸 R8 Dmax200，残骸碰撞体保留 60 s；发动机起火持续 15 s，每秒 hull −20、乘员 −5，灭火器每车 2 次、每次扑灭 6 s 内的火。**殉爆概率（§14.5.3）**：M4A1(75) 60%、Pz IV H 50%、M5A1 50%、M8 40%、半履带/卡车/Jeep 0%；殉爆 R12 Dmax350、乘员全灭。
**维修（§14.8）**：Engineer 工具箱 车体 +60 HP/s，每 6 HP 耗 1 Supplies（15 m 内补给箱，无补给不能修），修复毁坏模块 8 s（回到 50% HP）；Crewman 工具箱 +25 HP/s、最高修到 60% 最大 HP、无需 Supplies、修模块 12 s；主炮模块 Engineer 10 s / Crewman 15 s；Repair Station（M2-011）+20 HP/s（每 1 Supplies 5 HP），**毁坏模块不能修**；修理中断：修理者受伤 ≥ 15 HP 或离开 3 m；车体击毁后不可修；Ammo Rack 仅补给时恢复。内部命中与 `dmg_pen` 分配沿用 M1-023（§14.4.4）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/modules.rs` —— `ModuleSet`、性能降级曲线（输出 `PerfMods{speed,accel,track_bias,turret_rate,reload_mult,spread_mult}`）
- `crates/sim/src/vehicle/destroy.rs` —— 击毁/Burning/发动机起火/殉爆状态机、灭火器
- `crates/sim/src/vehicle/repair.rs` —— 维修交互（车体与模块）、供给扣费、中断条件
- `data/vehicles/*.json` 增补：`modules[]{id,hp,fire_chance,repairable}`、`ammo_rack_cook_off`、`extinguishers:2`
### 要实现的行为
1. 模块伤害来源：M1-023 的内部射线命中（`dmg_pen×1.0`）、地雷（履带/轮，M2-014）、炸药（贴附模块 700 HP，M2-014）。统一入口 `damage_module(vehicle, module, dmg, source)`。
2. 降级：每 tick 由 `ModuleSet` 计算 `PerfMods` 并交给载具物理（M1-022/M2-015）与炮塔/装填（M1-023）。
3. 击毁/起火/殉爆按上述状态机，用 `Rng::for_(tick, vehicle_id, "cookoff")` 判定概率（可复现）；Burning 期间乘员离车不受跌落伤害但受 20 HP。
4. 维修：交互持续按住；速率/耗费/中断规则如上；模块修复回到 50% HP；Crewman 车体上限 60%；Repair Station 与 Engineer 工具箱共用 `heal(vehicle,hp)` 入口，扣费在调用方。
5. 事件：`ModuleDamaged`、`VehicleBurning`、`VehicleExploded`、`ModuleRepaired`，供 UI/计分（M2-017 的"修复模块 +5"）使用。
### 接口与数据
- `pub fn perf_mods(&self) -> PerfMods`；`pub fn start_repair(&mut self, actor:PlayerId, target:RepairTarget) -> Result<RepairHandle,_>`。

## 不做什么
- 载具刷新/成本 → M2-015；Repair Station 实体 → M2-011；火箭/地雷/炸药本体 → M2-014。
- 载具起火/爆炸的粒子与音效 → M2-033/M2-046；UI 模块图标 → M3 前再定。
- 乘员补位的 UI → M1-060/后续。

## 验收标准
1. 模块损伤 → 性能降级曲线单测（逐项）：Engine 150/300 → `speed=0.6,accel=0.6`；Track 125/250 → 推力 ×0.6 且 `track_bias` 指向受损侧；Turret 100/200 → 旋转 ×0.5；Main Gun 125/250 → `reload×1.4, spread×1.5`；Tires 每毁一组 ×0.75，下限 0.3。
2. HP=0 效果：Engine 停转；单侧履带毁速度 ×0.3；两侧毁不能动；Turret 卡死手动 3°/s；Main Gun 不能开火。
3. 维修耗时与 /data 一致：Engineer 修发动机 8 s（240 tick ± 1），Crewman 12 s（360 tick）；主炮 10 s / 15 s；修后模块 HP = 50% 最大值。
4. 车体维修：Engineer 60 HP/s ± 2%，每 6 HP 扣 1 Supplies（无补给不修）；Crewman 25 HP/s 且在 60% 最大 HP 停止；Repair Station 路径模块不被修复。
5. 修理中断：修理者受 15 HP 伤害或离开 3 m 即中断；击毁后不可修。
6. 殉爆概率：固定种子 10 000 次判定，Pz IV H 命中率 50% ± 2%、M4A1 60% ± 2%（`cargo test -p sim cookoff_prob`）。
7. 起火：发动机起火 15 s 期间 hull 掉 300 HP（20/s）；灭火器第 3 次使用被拒；Burning 8 s 后爆炸 R8 Dmax200 对 5 m 步兵伤害 = Dmax×(1−(5/8)²)。
8. 性能：40 辆载具 × 5 模块更新 ≤ 0.1 ms/tick（并入 ARCH §6.10 阶段 3 的 2.0 ms 预算）。

## 测试要求
- 单元测试：降级曲线边界（50% 恰好、0）、重复损伤、修理叠加；proptest：HP 不越界。
- 集成/无头：Pz IV vs Sherman 对射，模块变化事件按序到达。
- 性能基准：`cargo bench -p sim vehicle_modules`。
- 回归：M1-023 的 12 组判定向量不变。

## 性能预算
ARCH §6.10 阶段 3「载具 ≤ 40 辆，2.0 ms」：本任务合计新增 ≤ 0.1 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 把"维修站"列入范围，但维修站实体在 M2-011（本任务只提供 `heal` 入口），依赖里没有 M2-011；按编号顺序 M2-011 先完成，无问题但应补依赖。
- 模块 HP 对 Pz IV 的具体值读 `/data`；GD 仅给通用档（轻 200/重 400 的发动机）。
