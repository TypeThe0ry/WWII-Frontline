# M2-010 sim：Engineer 建造系统

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：L
- 前置依赖：M1-020, M2-009
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §12.4; GAMEDESIGN §12.6; GAMEDESIGN §12.1; GAMEDESIGN §11.3; GAMEDESIGN §6.9.5; ARCHITECTURE §8.1; ARCHITECTURE §6.10

## 背景
建造用 **Supplies**（M1-020 的 Supply Crate/车载补给，建造时自动从工地 15 m 内所有箱与卡车/半履带车载补给中扣除，不足则禁止开工，GD §12.1），与战略资源不同池（R6）。本任务提供通用**建造框架**（"建造框架先于具体建筑"，MILESTONES §3.8），并落地 Engineer 的 6 类建筑。Garrison 由 Officer/Spotter/Commander 放置（M1-019，R1）；Outpost/维修站/AT 炮 → M2-011；榴弹炮不可建造（R7，预设阵地）。
建造清单（GD §12.4；【待调参】，读自 `/data/structures/*.json`）：

| 建筑 | 补给 | 时间(1 人) | HP | 全队上限 | 备注 |
|---|---|---|---|---|---|
| Barbed Wire（4 m 段） | 20 | 4 s | 150 | 24 | 通过速度 ×0.35、每秒 8 HP；剪线钳 4 s 开口；爆炸/碾压毁 |
| Tank Trap | 40 | 8 s | 1000 | 16 | 阻挡载具；步兵可通过 |
| Sandbag Wall（3 m 段） | 25 | 5 s | 400 | 24 | 矮墙可作两脚架支撑面 |
| Foxhole | 10 | 10 s | — | 20 | 蹲/趴时受爆炸 ×0.4；寿命 10 min |
| MG Nest | 100 | 12 s | 800 | 6 | 内置 M1919A4/MG 34 一挺，300 发，无人不自动射击 |
| Resource Node | 250 | 25 s | 1500 | 各 3 | 产出逻辑见 M2-009 |

通用规则：建造阶段 HP 从 20% 线性增至 100%；任何友军 Engineer 可加入，每多 1 名同时施工速度 +0.6，最多 3 人（×2.2）；敌方看不到未完工全息图；拆除返还 50% Supplies（仅 Engineer）。**位置限制**（GD §12.4）：只能建在己方 Sector 或战区；距据点圆心 `R_sp+15 m` 内仅允许 Sandbag Wall（≤ 8 段）与 Foxhole；距敌方存活玩家 ≥ 15 m（开工时检测）；不得阻挡 `nav_critical` 路径（放置时检测导航连通，不得把 HQ/据点完全包围）；**全队总建筑数 ≤ 120**。建筑受伤（`structure_armor`）：铁丝 枪弹 0/爆炸 1.0；沙袋/障碍 枪弹 0.02/爆炸 1.0；资源节点枪弹 0.02。工事不易主（GD §3.3.6）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/build/mod.rs` —— `BuildSystem`：`validate_placement()`、`start_build()`、`tick_build()`、`demolish()`、`structure_damage()`
- `crates/sim/src/build/rules.rs` —— 位置限制规则（纯函数，便于 100% 覆盖）
- `crates/sim/src/build/connectivity.rs` —— `nav_critical` 连通检查（粗网格洪水填充，2 m 栅格）
- `crates/sim/src/build/effects.rs` —— 铁丝减速/伤害、散兵坑减伤、MG Nest 座位、Tank Trap 碰撞
- `data/structures/{barbed_wire,tank_trap,sandbag_wall,foxhole,mg_nest}.json`
- `crates/server/src/c0/build.rs` —— C0 `BuildPlace/BuildCancel/BuildJoin/Demolish`（M2-007）的薄转发层：速率限制后调用 sim，并广播 C4 `StructureState`（无规则逻辑；INDEX 没有独立任务承担此胶水）
### 要实现的行为
1. `validate_placement(team, kind, pos, yaw) -> Result<(), BuildError>`：错误码枚举 `NotOwnSector / TooCloseToStrongpoint / EnemyNearby / BlocksNavCritical / TeamCapReached / TypeCapReached / NoSupplies / SlopeTooSteep / InWater / Overlap`；客户端全息红/绿直接用同一函数（经 `sim-wasm` 暴露只读校验）。
2. 开工：扣 Supplies（从 15 m 内箱按距离升序抽取；失败回滚）；生成 `Structure{state:Building,hp:20%}`；施工进度按施工者数量计速并每 tick 推进；施工者离开 > 3 m 或受伤暂停（沿用 M1-020 交互规则）。
3. 完工 → `Done`；资源节点完工调用 M2-009 `spawn_node`；MG Nest 生成座位实体（Engineer 或任意友军可上，使用 `/data` 里的武器）。
4. 受伤：按 `structure_armor` 乘数；HP 0 → `Destroyed`，发 `SimEvent::StructureDestroyed`。
5. 拆除：仅 Engineer、返还 50% Supplies（向下取整）；Sector 易手时工事保持原队所有。
6. 铁丝网：步兵穿越速度 ×0.35、每秒 8 HP；载具碾压 → 摧毁；剪线钳 4 s 移除（Engineer 工具行为在 M2-012 调用 `Structure::cut()`）。
7. `nav_critical` 检查：从 HQ 洪水填充到各据点，临时加入新建筑障碍后需仍连通；只在放置时运行，≤ 0.3 ms/次。
### 接口与数据
- `pub struct Structure{ id:EntityId, kind:StructureKind, team:Team, pos:Vec3, yaw:f32, hp:f32, hp_max:f32, state:BuildState, builders:SmallVec<[PlayerId;3]> }`；对应线上 `StructureState`（M2-007）。

## 不做什么
- 资源节点的产出与数量上限逻辑 → M2-009（本任务只调用）；Outpost/维修站/AT 炮 → M2-011；地雷/炸药 → M2-014；剪线钳/探雷工具行为 → M2-012。
- 建造菜单与全息渲染 → M2-039；建筑模型 → M2-047（占位用灰盒）。
- 弹药箱/Supply Crate 本体 → M1-020。

## 验收标准
1. 位置限制规则单测 100% 行覆盖（`cargo llvm-cov -p sim --lcov` 对 `build/rules.rs` = 100%），含 10 个错误码各至少 1 个用例。
2. `nav_critical`：构造封死 HQ 出口的铁丝网/沙袋布局被拒；留 1 条 ≥ 1.2 m 缺口则通过（单测）。
3. 团队总数 120 个上限与各类上限（铁丝 24、障碍 16、沙袋 24、散兵坑 20、MG 巢 6）逐项拒绝第 N+1 个。
4. 补给扣除：25 Supplies 沙袋在 15 m 内两箱（各 20）时从两箱合计扣除；14.9 m / 15.1 m 边界用例；失败不扣。
5. 建造时间：1 人 Sandbag 5 s ± 1 tick；3 人 = 5/2.2 ≈ 2.27 s ± 1 tick；HP 在 50% 进度时 = 60% ± 1%。
6. 据点圆心 `R_sp+15` 内：仅沙袋（≤ 8 段）与散兵坑通过，其余 `TooCloseToStrongpoint`。
7. 拆除返还 50%（20→10）；Sector 易手后工事仍存在（单测）。
8. 性能：200 个结构存在时每 tick 建造/结构更新 ≤ 0.1 ms（`cargo bench -p sim build_tick`）。

## 测试要求
- 单元测试：规则纯函数表驱动；proptest：随机放置序列不违反上限与最小间距不变量。
- 集成/无头：headless 脚本——Engineer bot（M2-027 之前用脚本输入）建造并摧毁每种建筑；快照 `StructureState` 一致。
- 性能基准：`build_tick`、`validate_placement`（criterion）。
- 回归：确定性哈希。

## 性能预算
ARCH §6.10 阶段 5「规则 0.7 ms」内，本任务 ≤ 0.1 ms/tick（200 结构）；放置校验为事件驱动、每次 ≤ 0.3 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 100% 覆盖仅限位置限制规则模块，其余按常规覆盖。
- 反滥用（GD §12.6）：Supply Crate 不可在敌 Sector 据点圆心 80 m 内放置属 M1-020；本任务不重复。
- 粗网格连通检查是近似；真 navmesh 连通复核留给 M2-049 压测中的 bot "无进展计数"。
