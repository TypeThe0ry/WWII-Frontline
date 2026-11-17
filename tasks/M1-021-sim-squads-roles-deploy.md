# M1-021 sim：小队、兵种名额与部署校验

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：L
- 依赖：M1-019, M1-007
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §2.4; GAMEDESIGN §2.5; GAMEDESIGN §4.1; GAMEDESIGN §4.3; GAMEDESIGN §4.4; GAMEDESIGN §4.5; GAMEDESIGN §5; ARCHITECTURE §6.8.1; ARCHITECTURE §15.1; BRIEF R2

## 背景
把"谁能以什么身份、从哪出生"变成服务器可权威校验的规则。**R2：编制是上限而非固定构成**：每队 50 人；步兵小队每队 ≤ 6 人、数量不限；装甲车组 ≤ 3 人、每队最多 4 个【待调参】；侦察组 ≤ 2、每队最多 2；Commander 1；允许**无小队的散兵（Unassigned）**但功能受限。
规则要点：
- 小队类型（GD §4.1）：Infantry（1–6）、Armor Crew（1–3，Tank Commander 为队长）、Recon（1–2）、Commander 席。命名：Alpha…Foxtrot（北约字母）、Armor-1..3、Recon-1..2，可附 ≤16 字符后缀（敏感词过滤在 M1-026）。
- 创建：未入队玩家在 DEPLOY 创建，自动成队长；每人每分钟最多创建 2 次；同类型已满不可建。加入：未满且未锁定。锁定：仅队长邀请进入；队长离开自动解锁并移交。踢人：仅队长；被踢者 5 min 内不能重加该小队；每分钟最多踢 3 人。移交：主动（对方 10 s 内确认）；自动（队长离线/离队/KIA 后未部署超 120 s/AFK）→ 优先 Officer，其次本局累计得分最高，再次加入最早；无人则解散。
- 兵种名额（GD §4.4；**M1 五兵种**）：Officer：每小队 1、小队人数 ≥ 2 方可选、仅队长可选；Rifleman：6、≥1；Medic：2、≥2；Support：1、≥3；Crewman：仅 Armor Crew（每车组至多 2 人）。名额随小队人数动态：人数下降后在世者不被强制换，**下次部署**必须换（`role_valid_on_deploy` 重检）。全队兵种上限硬校验（如 Medic 全队 12）。Unassigned 只能 Rifleman，且只能在 HQ/据点重生点出生（不可用 Garrison/Outpost）。
- 部署四步（GD §2.5）：小队 → 兵种 → loadout（含解锁等级，M1 无 XP 则全部 L1–L3 视为已解锁）→ 重生点。`DeployRequest{spawn_point, role, loadout, squad}` 全部由服务器权威校验。
- 阵营/平衡（GD §2.4）：每队 ≤ 50；`H_t − H_other ≤ 2` 才可加入多的一方；换队：入局 3 min 内不得换、两次间隔 ≥ 10 min（`team_switch_cooldown=600 s`）；Bot 动态让位（M1-039）。
- 权限（GD §4.5）：放小队标记仅队长；**放置 Garrison 仅 Officer（R1）**；Outpost 属 M2。
- **M1 兵种缺口（需总控知晓）**：Armor Crew 的队长应为 Tank Commander（GD §4.1/§4.4），但 M1 的 5 兵种不含 Tank Commander；本任务提供配置 `m1_override.armor_crew_leader_role = "crewman"`，使 M1 车组由 Crewman 组成。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/squad/mod.rs`、`squad.rs` —— `Squad{id, kind, name, leader, members, locked, kick_bans}`、`SquadManager`
- `crates/sim/src/squad/ops.rs` —— `apply_squad_op(player, SquadOp) -> Result<SquadEvent, SquadError>`（create/join/leave/kick/promote/rename/lock/unlock/invite；速率限制用 tick 窗口）
- `crates/sim/src/squad/leader.rs` —— 自动移交逻辑
- `crates/sim/src/roles/quota.rs` —— `role_available(squad, role, team_counts) -> Result<(), RoleError{Full,TeamFull,SquadTooSmall,LeaderOnly,NotInArmor,Unassigned}>`
- `crates/sim/src/roles/loadout.rs` —— loadout 解锁与阵营校验
- `crates/sim/src/deploy/validate.rs` —— `validate_deploy(world, player, req) -> Result<DeployOk, DeployError{…, reason:u8}>`
- `crates/sim/src/team/balance.rs` —— 加入/换队校验
- `crates/sim/tests/squads_*.rs`、`deploy_*.rs`
### 要实现的行为
1. 全部操作为服务器权威纯函数（输入：玩家 id、操作；输出：新状态 + 事件 `PlayerRosterChange`）；不依赖 I/O。
2. 速率限制：创建 2/min/人、踢 3/min/队长；窗口以 tick 滚动。
3. `validate_deploy`：依序检查 玩家状态（需处于 DEPLOY/KIA 后）→ 小队合法性 → 兵种名额/解锁 → loadout 属该兵种与阵营 → 重生点合法（调用 M1-019 `request_spawn`）→ 换兵种冷却（30 s 内 ≤ 1 次）；返回首个失败原因码（供 `DeployResult.reason`）。
4. Unassigned 限制：只能 Rifleman；只能选 HQ。
5. 兵种动态失效：小队人数变化后发出 `RoleInvalidatedOnNextDeploy` 标志，不强制现存活者。
6. 队长权限查询 `can(player, Action)`：放标记、放 Garrison（仅 Officer 兵种）、踢人等。
7. 平衡校验 `can_join_team(team)`：`H_t − H_other ≤ 2`；`can_switch_team(player, now)`；管理员强制换队不受冷却（但每局每人 ≤ 1 次，M2 自动平衡实现）。
8. 快照字段：为 `PlayerRoster`（M1-010）提供 `RosterDelta`。
### 接口与数据
```rust
pub enum SquadKind { Infantry, ArmorCrew, Recon }
pub fn validate_deploy(w:&World, p:PlayerId, req:&DeployRequest) -> Result<(), DeployError>;
pub fn apply_squad_op(w:&mut World, p:PlayerId, op:SquadOp) -> Result<Vec<SimEvent>, SquadError>;
```
名额与上限完全来自 `/data/roles/*.json` 与 `data/rules/squads.json`（新增：每队 50、小队上限、速率限制）。

## 不做什么
- 网络处理、聊天过滤、`DeployRequest` 的收发：M1-026；
- 部署界面与小队列表 UI：M1-056；
- Outpost 放置、Commander/Tank Commander/Spotter/Sniper 兵种：M2/M3；
- bot 编制与填充：M1-039；
- 局间小队保留（GD §4.6）、投票罢免 Commander：M2；
- 兵种等级/XP 解锁：M2。

## 验收标准
1. 非法部署请求全被拒（`deploy_negative_matrix` ≥ 30 例）：满员小队、名额已满的兵种（Medic 第 3 个、Support 第 2 个）、小队人数不足（Support 在 2 人队）、非队长选 Officer、Unassigned 选 Medic/Garrison 出生、阵营错误 loadout、不可用/封锁/槽空的出生点、换兵种冷却内；每例返回对应原因码（枚举 `DeployError` 全分支各 ≥ 1 例）。
2. 编制上限（R2）：第 7 名玩家加入步兵小队被拒；第 4 个 Armor Crew 可建、第 5 个被拒；全队第 51 人被拒（单测）。
3. 速率：1 分钟内第 3 次创建小队被拒，第 61 s 恢复；队长 1 分钟踢第 4 人被拒（单测，tick 精确）。
4. 踢人禁入 5 min（9000 tick）：4 min 59 s 拒绝重加、5 min 01 s 允许（单测）。
5. 队长自动移交优先级：Officer > 得分最高 > 加入最早；无人则解散（4 例）。
6. 动态名额：6 人队降为 2 人后，已选 Support 的在世者保持，下次部署被拒并给出 `SquadTooSmall`（单测）。
7. 平衡：H_a−H_b=2 时可加入多的一方？（规则：加入后差 ≤ 2 才可；差=3 被拒；单测 3 例）；换队 2 min 59 s 被拒、3 min 01 s 允许、两次间隔 9 min 59 s 被拒。
8. 性能：100 名玩家全部提交部署校验一次 ≤ 0.3 ms（`cargo bench -p sim deploy_validate`）；确定性哈希一致。

## 测试要求
- 单元测试：每条规则正反例、枚举全覆盖（`strum` 遍历所有 `DeployError`）、时间窗口边界。
- 集成：headless 通过 sim API 完成"建队→选兵种→选 HQ→出生"。
- 性能基准：criterion `deploy_validate`。
- 回归：确定性哈希基线更新。

## 性能预算
- 并入 ARCHITECTURE §6.10 阶段 1（输入摄取与校验 0.8 ms）与阶段 5（规则）；本任务校验为事件驱动，≤ 5 µs/次。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- M1 兵种缺口（无 Tank Commander）见背景，需总控确认 `m1_override` 做法；写 PLAN_ISSUES。
- 平衡规则 `H_t−H_other ≤ 2` 的语义以 GD §2.4 文字为准；若歧义，保守实现并登记。
- 规模 L：先做小队操作 + 部署校验，名额与平衡校验作为纯函数库，若超出追加 `M1-021b`。
