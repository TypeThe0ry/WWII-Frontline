# M3-021 sim：建筑破坏阶段与动态导航障碍

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 依赖：M2-014, M2-005
- 素材需求：无（灰盒即可；破坏阶段用程序化替换盒体）；素材状态：n.a.
- 引用文档：MAPS §6.2; MAPS §1.6; ARCHITECTURE §16.2; ARCHITECTURE §9.5; ARCHITECTURE §8.1; GAMEDESIGN §13.4

## 背景
地图建筑数据（`buildings.geojson`，MAPS §6.2）带有 `destructible`、`ruin_stage`、`enterable` 等属性。M2-014 已实现炸药/火箭/地雷对建筑的伤害，M2-005 已烘焙 Carentan 全图碰撞与导航（步兵与载具 navmesh，128 m 瓦片，支持 `tile cache` 动态障碍，ARCHITECTURE §16.2："tile cache 支持动态障碍如被摧毁建筑，M3"）。M3 的炮击/空袭（Bombing 对建筑 1500 HP、Precision 3000、榴弹炮 1500、坦克 HE）使建筑受损成为常态。本任务实现：建筑 HP 与**破坏阶段切换**（intact → damaged → ruin）、**碰撞更新**（Rapier 碰撞体替换）、**导航更新**（把变化作为 `NavObstacleChange` 供 bots 的 tile cache 应用），并保证**确定性**（`state_hash` 一致，MILESTONES 风险项）。
GD 对建筑破坏阶段没有规则条目，只有 MAPS 的属性字段：阶段数、阶段阈值、残骸碰撞形状属于"细化"，须写 PLAN_ISSUES；本任务采用保守默认：3 阶段（`intact`、`damaged`（HP ≤ 50%：窗/墙洞，可进入性不变）、`ruin`（HP = 0：低矮残骸，原墙体碰撞替换为 ≤ 1.2 m 高的废墟碰撞体，导航可通行））。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/world/buildings.rs` —— `BuildingState{id, hp, stage, destructible, enterable}`；伤害入口 `apply_structure_damage(id, amount, source)`；仅 `destructible=true` 受损；爆炸对建筑伤害用 GD 值（HE 炮 1500、Precision 3000 等，来自 `/data` 能力/弹药表）。
- `crates/sim/src/world/collision_update.rs` —— 阶段切换时更新 Rapier 碰撞：移除旧碰撞体、插入替换体（`ruin` 低矮体）；更新 LOS/穿透材质表（砖墙/石墙→瓦砾）；`CollisionRevision` 计数写入 `state_hash`。
- `crates/sim/src/world/nav_changes.rs` —— `NavObstacleChange{tile, polys_dirty, tick}` 队列；`WorldView::nav_changes_since(rev)`；**sim 不做寻路**，只发布变化。
- `crates/bots/src/nav/tile_cache.rs` —— 薄适配：消费 `nav_changes_since`，调用导航库的 tile cache 重建受影响瓦片；失败时回退为"整瓦片标记不可达"。（bots 侧适配，工作量小；若 M1-037 导航库不暴露 tile cache API，则写 PLAN_ISSUES 并只实现标记。）
- `crates/sim/tests/destruction.rs`。
### 要实现的行为
1. 伤害累积：多次伤害累加 HP；`hp ≤ 50%` → `damaged`；`hp ≤ 0` → `ruin`；事件 `BuildingStage{id, stage}`（C4 可靠事件，M3-005 若无该事件则在 `Notice`/`Explosion` 复合事件中表示，并写 PLAN_ISSUES）。
2. 碰撞替换在**下一 tick 开头**统一应用（避免同 tick 内顺序依赖）；应用顺序按 `building_id` 升序（确定性）。
3. 导航：受影响 128 m 瓦片标脏，bots 在 ≤ 1 s 内更新；`enterable` 建筑变 `ruin` 后室内寻路作废，改为开放地面通行。
4. 玩家在被毁建筑内时：受 `hp_zero` 爆炸伤害（来自触发爆炸），随后被推出到最近可站立点（不嵌入碰撞）。
5. 掩体变化：ruin 阶段提供矮掩体（碰撞高度 ≤ 1.2 m，视线遮挡 ≈ 蹲姿），影响步兵 LOS 判定与子弹穿透（材质由 `ruin` 材质表）。
6. 预测：客户端 `sim-wasm` 的 `PredictSim` 需同步 `CollisionRevision`；服务器通过 `BuildingStage` 事件通知，客户端在对应 tick 切换碰撞（预测对账容差见 R5）。
### 接口与数据
- `buildings.geojson` 新增/使用字段：`hp`（默认按 `levels`：1 层 1000、2 层 1500、3 层 2000，写数据表）、`ruin_collision`（`low_debris|none`）；`ruin_stage` 在 schema 中取值 `[intact, damaged, ruin]`。

## 不做什么
- 破坏的视觉替换/粒子/弹坑 → M3-031/M3-032；地形凹坑（Bombing 弹坑地形变形）仅视觉，不改高度场碰撞（M4 评估）；
- 建筑受击音频 → M3-038；载具撞毁建筑的物理结算（GD §14.9 公式已有）只调用本接口。

## 验收标准
1. **建筑被摧毁后碰撞/导航在 1 s 内更新**：单测在 HP 归零的 tick 之后 ≤ 30 tick 内，`collision_revision` 递增且射线穿过原墙位置；bots 侧 `navmesh` 在该瓦片可通行（路径查询成功，查询耗时 ≤ 1.0 ms）。
2. **不破坏确定性**：同 seed 与同输入（含 20 栋建筑被炸毁）两次运行 10 000 tick 的 `state_hash` 逐 tick 相同；native vs native 与 wasm vs wasm 均通过。
3. 阶段阈值：HP 剩 51% 为 `intact`、50% 为 `damaged`、0 为 `ruin`（边界单测）。
4. 不可破坏建筑（`destructible=false`）受任何伤害 HP 不变（单测）。
5. 建筑内玩家被推出后不卡入碰撞（100 次随机位置采样，嵌入次数 0）。
6. 20 栋同 tick 破坏的处理开销 ≤ 0.5 ms（`criterion`），常态 0。
7. `CollisionRevision` 参与 `state_hash`，修改后哈希变化（单测）。

## 测试要求
- 单元测试：伤害累积、阶段边界、碰撞替换顺序、玩家推出、LOS/穿透材质变化。
- 集成/无头：`headless-client` + 服务器：Bombing 炸毁建筑后客户端预测位置与权威偏差 ≤ 0.05 m（R5 阈值，信息性）；bot 在建筑被毁后改走通路。
- 性能基准：`crates/sim/benches/building_stage.rs`。
- 回归：确定性哈希、`navmesh` 瓦片哈希（动态更新后）。

## 性能预算
ARCHITECTURE §6.10 阶段 2–4 总和不得回归：碰撞替换峰值 ≤ 0.5 ms/tick（偶发）；§16.2 路径查询 ≤ 1.0 ms/tick（60 bot）不回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 文档对"破坏阶段/碰撞形状/建筑 HP"几乎无规定，上述默认均为本任务"细化"，必须写 PLAN_ISSUES，由 Fable 回写 MAPS/GD。
- 与 INDEX 的流向：INDEX 把本任务归在 sim 工作流，但 tile cache 适配落在 `crates/bots`；这是跨 crate 小改动，不违反 ARCHITECTURE §4.5 依赖方向（bots 依赖 sim）。
- 残骸/弹坑的"碰撞保留 60 s"只适用于载具残骸（GD §14.5.2），建筑 ruin 永久保留。
