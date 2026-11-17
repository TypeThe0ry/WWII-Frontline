# M1-037 bots：navmesh 寻路与避障

## 元数据
- 里程碑：M1
- 工作流：bots
- 规模：L
- 依赖：M1-036（navmesh.bin 与 off-mesh 链接格式）, M0-026（M0 基础 bot、BotBrain trait、服务器 bot 宿主）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §16.2; ARCHITECTURE §16.5; ARCHITECTURE §4.3.6; GAMEDESIGN §20.3（B-01、B-21）

## 背景
M0 的 bot 只会在高度场上直线走（无 navmesh）。M1 的 Carentan 1/3 区域有建筑、树篱、多层室内，bot 必须走 Recast navmesh。`M1-036` 已烘焙 `navmesh.bin`（128 m 瓦片）并定义 off-mesh 链接（窗户翻越、梯子、楼梯）。本任务在 `crates/bots` 内实现"加载 → 查询 → 平滑 → 局部避障 → 输出移动意图"，供 `M1-038` 的行为树动作"移动至点"调用。
关键约束（ARCHITECTURE §16.2、§16.5）：navmesh 参数 `cell_size 0.3 m / cell_height 0.2 m / agent_radius 0.4 m / agent_height 1.8 m / max_climb 0.5 m / max_slope 45°`；A*（多边形图）+ 漏斗平滑；**每 bot 路径重算最多 2 Hz**；全部路径查询 ≤ 1.0 ms/tick（60 bot）。bot 只能产出与真人同构的 `InputCmd`（§4.3.6），不得直接改 `World`。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/nav/mod.rs` —— `NavMesh` 加载（读 M1-036 的 `navmesh.bin`，格式以 M1-036 产物为准）、瓦片索引、`NavQuery`。
- `crates/bots/src/nav/path.rs` —— 多边形 A*、漏斗（string-pulling）、路径缓存与失效。
- `crates/bots/src/nav/avoid.rs` —— 局部避障（友军/载具/动态障碍的简化 ORCA 或 steering 分离力）。
- `crates/bots/src/nav/offmesh.rs` —— off-mesh 链接状态机（翻窗/爬梯/上楼）：到达入口点 → 发出对应 `InputCmd` 动作位（跳/前进/翻越）→ 出口点确认。
- `crates/bots/src/nav/steer.rs` —— 把"下一路点"转为 `InputCmd`（move_x/move_z、yaw、姿态、冲刺位）。
- `crates/bots/tests/nav_*.rs`、`crates/bots/benches/nav_query.rs`。
### 要实现的行为
1. 加载 navmesh：只在服务器进程启动/换图时加载；加载失败返回错误并回退 M0 的高度场网格寻路（保留 M0 实现作 fallback，记日志）。
2. `NavQuery::find_path(from, to, area_mask) -> PathHandle`：最近多边形投影（容差 2 m）、A*、漏斗平滑；不可达返回 `NoPath`，不得死循环（节点展开上限 4 096）。
3. 每 bot 重算频率 ≤ 2 Hz（500 ms 冷却）；目标移动 < 3 m 且路径未被阻断时复用旧路径。
4. 卡住检测（GD §20.3 B-21）：5 s 内位移 < 1 m → 强制重规划，再卡则随机侧移 + 跳跃，仍卡则回报上层"目标不可达"。
5. off-mesh：路径含 off-mesh 链接时，按链接类型执行动作；翻越窗户与 `sim` 的翻越规则一致（不绕过 sim 规则）。
6. 局部避障：与友军保持 ≥ 0.8 m 间距；让行载具（M1 仅 1 辆坦克，按其包围盒外扩 3 m 绕行）。
7. 已知危险区（铁丝网/地雷，M2+）预留 `area_cost` 接口，M1 仅实现 `area_mask` 过滤。
### 接口与数据
```rust
pub struct NavMesh { /* tiles, polys, offmesh_links */ }
pub struct NavQuery<'a> { mesh: &'a NavMesh }
pub enum PathResult { Found(PathHandle), NoPath, Partial(PathHandle) }
pub struct Steering { pub fn step(&mut self, me: &PlayerView, path: &PathHandle, dt: f32) -> MoveIntent }
pub struct MoveIntent { pub dir: Vec2, pub yaw: f32, pub sprint: bool, pub stance: Stance, pub jump: bool, pub vault: bool }
```
`MoveIntent` 由 `steer.rs` 转成 `protocol::InputCmd`，走与真人相同的服务器校验。

## 不做什么
- 不做行为树/效用目标选择（M1-038）；不做小队长/兵种选择（M1-039）。
- 不做载具驾驶 bot 与道路图（M3，ARCHITECTURE §16.2）。
- 不做 tile cache 动态障碍（建筑被摧毁，M3）。
- 不改 navmesh 烘焙参数与格式（M1-036 负责）。

## 验收标准
1. 60 个 bot 同时路径查询的服务器 tick 内总耗时 ≤ 1.0 ms/tick（`cargo bench -p bots nav_query`，Carentan 1/3 navmesh，随机起终点 60 对，中位数换算为每 tick；参考机）。
2. HQ 出生点到 L1a/L2a/L3a 三个据点中心的路径均 `Found`，且路径长度 ≤ 直线距离 × 2.5（集成测试 `nav_hq_to_objectives`）。
3. 建筑内多层路径连通：选定的 ≥ 3 栋 `enterable` 建筑，一层门口到二层房间 `Found`，且路径经过楼梯 off-mesh 链接（`nav_multifloor`）。
4. 单 bot 重算频率 ≤ 2 Hz（测试以计数器断言：10 s 内 `find_path` 调用 ≤ 20）。
5. 卡住检测：构造不可通行障碍，bot 在 ≤ 8 s 内触发重规划并上报不可达（`nav_stuck_recover`）。
6. 20 个 bot 走向同一据点 60 s，任意两 bot 间距 < 0.5 m 的采样占比 < 2%。
7. navmesh 缺失时回退 M0 寻路且不 panic（单测）。

## 测试要求
- 单元测试：A* 在手工小网格上的最优性；漏斗对 L 形走廊的拐角；off-mesh 入出口；投影失败路径；节点上限。
- 集成/无头测试：服务器 + 10 bot 在 Carentan 1/3 跑 5 min 无 panic；记录 `bot_nav_query_us` 直方图。
- 性能基准：`crates/bots/benches/nav_query.rs`（criterion），基线写入 `reports/m1-bots-nav.json`。
- 回归：与 M0-026 的 bot 测试保持通过。

## 性能预算
- ARCHITECTURE §16.2/§16.5：路径查询 ≤ 1.0 ms/tick（60 bot）；§6.10 阶段 6 bot 思考合计 ≤ 3.0 ms/tick，本任务不得吃掉他项份额。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test -p bots`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：接口/数据变更已更新对应 docs"变更说明"并在 PLAN_ISSUES 登记
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Recast 绑定选择（`recast-navigation` 绑定 vs 纯 Rust）须符合 ARCHITECTURE §4.5 禁用依赖表；选型写 PLAN_ISSUES。
- navmesh 若来自灰盒建筑，室内多层连通依赖 M1-034 的 `levels`/off-mesh 标注；缺标注时验收 3 记 `partial`。
- 取 `WorldView::nav_hint()` 作为 navmesh 句柄来源（ARCHITECTURE §8.1），不得在 bots 内读取 `World` 内部状态。
