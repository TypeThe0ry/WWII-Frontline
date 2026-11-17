# M1-038 bots：行为树与效用 AI

## 元数据
- 里程碑：M1
- 工作流：bots
- 规模：L
- 依赖：M1-037, M1-014（命中区/伤害，供射击决策与目标威胁评分）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §16.3; ARCHITECTURE §16.5; GAMEDESIGN §20.3; GAMEDESIGN §20.6; GAMEDESIGN §20.1

## 背景
在 M1-037 的导航之上，bot 需要"战术动作"（行为树）与"高层目标选择"（效用 AI）。ARCHITECTURE §16.3 规定：节点 `Sequence/Selector/Parallel/Condition/Action/Decorator(cooldown, until)`；叶子动作含移动至点、占掩体、射击（含目标选择）、投掷、换弹、救治、补给、爬高、撤退；**每 bot 树节点数 ≤ 80，运行成本 ≈ 10–30 µs**；效用 `U(goal)=Π(consideration_i^w_i)` 或加权和，considerations 含距离、威胁、小队指令、资源（弹药/血量）、任务价值、拥挤度。感知必须"无透视"：视锥 120°、距离衰减的识别概率、听觉事件 `Shot/Explosion`（GD §20.3 B-05）。bot 每 3 tick（10 Hz）思考一次，按 `bot_id % 3` 分片（ARCHITECTURE §16.5）。难度参数见 GD §20.6（Recruit/Regular/Veteran）。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/bt/mod.rs`、`node.rs`、`blackboard.rs` —— 行为树引擎（无堆分配热路径，节点 enum + 索引）。
- `crates/bots/src/bt/actions/*.rs` —— 叶子动作：`move_to`、`take_cover`、`shoot`、`reload`、`throw_grenade`、`throw_smoke`、`revive`、`resupply`（占位，M1 仅响应 Ammo Crate）、`retreat`、`vault_or_climb`。
- `crates/bots/src/utility/mod.rs` —— considerations、响应曲线（线性/二次/逻辑）、`select_goal()`。
- `crates/bots/src/perception.rs` —— 视觉（120° 视锥 + `WorldView::los` + 可见度）、听觉事件队列、目标记忆（记忆时间取 GD §20.6）。
- `crates/bots/src/difficulty.rs` —— 难度参数表（读 `/data` 或常量表；初值照 GD §20.6，标 `initial`）。
- `crates/bots/src/brain.rs` —— `SoldierBrain: BotBrain`，组装感知 → 效用 → 行为树 → `InputCmd`。
### 要实现的行为
1. GD §20.3 逐条覆盖 M1 范围内的 B-01..B-06、B-08（Medic 救倒地友军，烟雾遮蔽后救起）、B-17、B-20（倒地呼救、30 s 无人救起放弃）、B-21、B-22；B-07/B-09 由 M1-039 补；B-10..B-16 M2 起。
2. 战斗：威胁评分（距离、武器、是否瞄准自己）；按武器射程选姿态；开火散布/后坐与真人同规则（经 sim，不绕过）；手雷/烟雾使用规则：目标进入 ≤ 25 m 掩体后、或需封锁视线时。
3. 感知无透视：反应延迟按难度（Regular 0.35–0.60 s）；视线丢失后按难度保持记忆（Regular 4 s）；最大视距（Regular 站姿 220 m）。
4. 效用目标：{占点, 防守, 交战, 救治, 补弹, 撤退, 跟随小队}；每 3 tick 评估，加入迟滞（新目标分数需 > 当前 ×1.15）防抖。
5. 分片：`bot_id % 3 == tick % 3` 才思考；其余 tick 复用上次 `InputCmd` 并仅更新朝向插值。
6. 行为树节点上限校验：构造期断言每棵树 ≤ 80 节点。
### 接口与数据
```rust
pub trait BotBrain { fn think(&mut self, view: &dyn WorldView, ctx: &BotCtx) -> InputCmd; }
pub struct BotCtx { pub me: PlayerId, pub difficulty: Difficulty, pub squad_order: Option<SquadOrder>, pub nav: &NavQuery }
pub enum Goal { CapturePoint(ObjId), Defend(ObjId), Engage(PlayerId), Revive(PlayerId), Resupply, Retreat, FollowLeader }
```

## 不做什么
- 不做小队长 bot、兵种/出生点选择、真人替换（M1-039）。
- 不做 Engineer/Anti-Tank/MG 专属行为（M2-027）、载具驾驶/火炮（M3）。
- 不做 Commander bot / LLM 指挥官（M2+，ARCHITECTURE §16.4）。
- 不做 bot 语音（永不）；不发聊天（GD §20.3 B-19）。

## 验收标准
1. GD §20.3 的 B-01..B-06、B-08、B-17、B-20、B-21、B-22 各至少 1 个场景测试，全部通过（`cargo test -p bots --test bt_scenarios`）。
2. 每棵行为树节点数 ≤ 80（单测遍历断言）。
3. 单 bot 单次 `think`（含感知）中位数 ≤ 60 µs（`cargo bench -p bots think`，参考机）。
4. 感知无透视：墙后敌人在 LoS 为假时 bot 不朝其开火（场景测试 `perception_no_wallhack`，1000 tick 内开火次数 = 0）。
5. 难度参数生效：Recruit/Veteran 的"发现到开火"延迟均值落在 GD §20.6 对应区间内（headless 统计，各 ≥ 200 次采样）。
6. 迟滞防抖：两个等分目标场景下 30 s 内目标切换次数 ≤ 3。
7. bot 的 `InputCmd` 经 M0-018 的同一校验路径（单测断言 bot 无旁路）。

## 测试要求
- 单元测试：BT 各节点语义（Sequence/Selector/Parallel/Decorator cooldown/until）、效用曲线数值、感知识别概率、记忆衰减。
- 集成/无头测试：10 bot 对 10 bot 在 Carentan 1/3 跑 5 min，记录击杀数 > 0、无 panic、`bot_think_us` P95。
- 性能基准：`benches/think.rs`，结果入 `reports/m1-bots-bt.json`。
- 回归：M1-037 的导航测试与 M0-026 测试不得失败。

## 性能预算
- ARCHITECTURE §16.5：士兵 bot 思考单次 ≈ 30–60 µs；战术/射击决策 ≈ 1.0 ms/tick；bot 合计 ≈ 3.0 ms/tick（§6.10 阶段 6）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test -p bots`）；`cargo clippy -D warnings` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材命名无新增；缺失素材需求 ID 已列出（无）
- [ ] 文档同步：难度参数若落入 `/data`，更新 GAMEDESIGN §20.6 对应"变更说明"并记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 难度参数均为【待调参】初值，状态 `initial`；校准留给平衡模拟（M2+）。
- 室内战斗的掩体点依赖地图数据 `cover_points`（GAMEDESIGN §20.3 B-03；字段若未在 MAPS §6.2 定义，写 PLAN_ISSUES）；M1 若缺则用 navmesh 边缘 + 墙体射线自动生成，写 PLAN_ISSUES。
- 感知性能：LoS 射线数量需限预算（每次思考 ≤ 8 条射线），否则超 60 µs。
