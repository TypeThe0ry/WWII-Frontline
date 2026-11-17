# M2-009 sim：战略资源与资源节点

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：L
- 依赖：M2-008
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §11; GAMEDESIGN §3.3.6; GAMEDESIGN §12.1; ARCHITECTURE §6.8; ARCHITECTURE §6.10; BRIEF §3c R6

## 背景
战略资源 Manpower / Munitions / Fuel 是全队共用池（GD §11.1），与建造货币 **Supplies**（补给箱承载，M1-020）是**两套独立系统**（R6）。资源池由 Commander 支配；M2 范围内 Commander 席位未开放（M3），所以本任务提供与席位无关的 `try_spend` API，调用方为规则指挥官 bot（M2-028）与 M2-015 的载具刷新。
数值（GD §11.2，全部【待调参 / 待测 → REF §economy】，读自 `/data`）：
- 每整分钟结算（`t=60,120,…`）：`income_X = base_X + sectors_held × sector_rate_X + nodes_X × node_rate_X`。
- Manpower：base 30、sector_rate 6、node_rate 25、上限 1500；Munitions：12 / 3 / 10 / 800；Fuel：6 / 2 / 6 / 400（依次为 base / sector_rate / node_rate / 上限）（node_rate Fuel = 6）。起始储备 200 / 80 / 40。开局持有 2 个 Sector → 42 / 18 / 10 每分钟。
- `sectors_held` = 己方已占领据点的 Sector 数（中立线不计）。
- 节点（GD §11.3）：Engineer 建造；仅在己方 Sector 内 `resource_site` 对应类型处；距敌方存活玩家 ≥ 40 m；250 Supplies；25 s（1 Engineer，每多 1 名 +0.6，上限 3 人）；HP 1500（枪弹 ×0.02，爆炸 ×1.0）；建成后下一整分钟起产出；每类每队 ≤ 3 个，全队 9 个；每 Sector 每类 ≤ 1；Overrun：敌 15 m 内且己方 15 m 内无人 30 s 销毁；Sector 易手立即销毁，Y 方获 +50 Manpower 战利品（仅 Manpower 节点已建者）；Engineer 拆除返还 50% Supplies；摧毁后同一 site 可立即重建。
- 燃料维持（GD §11.4）：存活载具持续消耗 Fuel：轻型装甲 1/min、中型 2/min、重型 3/min、其余 0；Fuel = 0 → 载具"低燃料"最大速度 ×0.6、无法再刷新载具。取消冷却中的呼叫返还 50%。溢出收入丢弃，UI 显示"已满"。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/rules/resources.rs` —— `TeamResources`、`income_tick()`、`try_spend()`、`refund()`、`upkeep` 注册表
- `crates/sim/src/rules/resource_node.rs` —— `ResourceNode` 组件、产出登记、Overrun 计时、销毁、战利品钩子（响应 M2-008 的 `on_sector_flip`）
- `data/structures/resource_node_{manpower,munitions,fuel}.json`、`data/economy.json` —— base/rate/cap/起始/upkeep/战利品
- `crates/sim/tests/resources_*.rs`
### 要实现的行为
1. 每个整分钟 tick（`tick % (60×30) == 0`，`tick>0`）结算三类收入，应用上限；溢出量记入指标 `resource_overflow_total{kind}`。
2. `try_spend(team, Cost{m,u,f}) -> Result<(), Insufficient>`：原子扣除，任一类不足则全部不扣；`refund(team, cost, frac)`。
3. `upkeep`：`register_upkeep(vehicle_id, class)` / `unregister`；每分钟扣除合计；Fuel 不足时按 GD §11.4 设置队伍标志 `low_fuel`（M2-015 读取）。M2-015 之前注册表为空，行为必须正确。
4. 资源节点：提供 `spawn_node(site_id, kind, team, builder_ids)` / `destroy_node(id, reason)`；M2-010 的建造框架在节点施工完成时调用（放置/Supplies/建造时间归 M2-010）。节点上限、每 Sector 每类 1 个、Overrun、拆除返还、易手销毁与 +50 战利品均在本任务实现并单测。
5. 事件：`SimEvent::ResourceDelta{team,kind,value,reason}`（对应 C4 `ResourceDelta`，M2-007 增加 `reason`）；`WorldView::team_resources(team)` 返回当前池（ARCH §8.1）。`WorldState` 1 Hz 下发三资源（ARCH §6.8.2）。
6. Skirmish 无资源系统：`mode != Frontline/Assault` 时本模块 no-op（M4 再开分支）。
### 接口与数据
- `pub struct Resources{ pub manpower:i32, pub munitions:i32, pub fuel:i32 }`（用整数，避免浮点累计误差）；`pub fn income_per_min(&self, team) -> Resources`。

## 不做什么
- Engineer 建造放置/全息检测/Supplies 扣费 → M2-010；Supplies 自身逻辑 → M1-020（不得与资源池混用，R6）。
- Commander 面板、能力成本（Airhead/Rally/Bombing 等）→ M3；Assault 起始储备 ×1.25 → M3。
- 载具刷新成本/上限实现 → M2-015（仅调用本任务 `try_spend`）。
- 资源 HUD → UI 任务（M1-057 / M2-044）。

## 验收标准
1. 整分钟结算精确到 1 单位：开局持有 2 Sector、无节点，`t=60 s` 后三资源 = 200+42 / 80+18 / 40+10 = 242 / 98 / 50（单测 `resources_first_minute`）。
2. 双方各 2 Sector + 2 Manpower 节点 + 1 Munitions + 1 Fuel：每分钟 Manpower 92 / Munitions 28 / Fuel 16（GD §11.2 示例，单测）。
3. 上限：Manpower 1500、Munitions 800、Fuel 400；超出部分被丢弃且 `overflow` 计数正确（单测）。
4. 节点数量上限单测：第 4 个同类节点、同 Sector 第 2 个同类节点被拒；全队 9 个上限。
5. 易手销毁：Sector 易手瞬间 X 的节点 100% 销毁，Y 获 +50 Manpower（仅当销毁的是 Manpower 节点）。
6. Overrun：敌 15 m 内且己方无人持续 30 s（900 tick）节点销毁；中途有己方进入则计时清零（单测）。
7. `try_spend` 不足不扣任何一类；`refund(…, 0.5)` 取整规则单测（向下取整）。
8. 燃料维持：注册 1 轻 + 1 中型 + 1 重型 → 每分钟 −6 Fuel；Fuel 降到 0 时 `low_fuel=true`。
9. 资源阶段 CPU 本任务新增 ≤ 0.1 ms/tick（`cargo bench -p sim resources_tick`）。

## 测试要求
- 单元测试：上述 + 溢出、负数、同 tick 多次花费顺序确定性；proptest：任意花费序列资源不为负、不超上限。
- 集成/无头：headless 5v5 跑 5 min，客户端收到的 `WorldState.resources` 与服务器 `team_resources` 逐分钟一致。
- 性能基准：`resources_tick`。
- 回归：`state_hash` 快照更新（资源池参与哈希）。

## 性能预算
ARCH §6.10 阶段 5「规则 0.7 ms」：本任务新增 ≤ 0.1 ms；与 M2-008（≤ 0.2 ms）合计不得使阶段 5 超 0.7 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §11.2 表中 Fuel `node_rate`=6、`sector_rate`=2（上文 income 公式已按表）；以 `/data/economy.json` 为准。
- 无 Commander 时资源由 RuleCommander（M2-028）支配；人类玩家的资源开销入口在 M3，M2 阶段不要为此新增 UI 消息。
- 资源曲线（结算页）需要每分钟快照：本任务在 `StepOutput` 中提供 `resource_history` 迭代器，供 M2-042 使用。
