# M1-039 bots：小队长 bot、兵种选择与填充

## 元数据
- 里程碑：M1
- 工作流：bots
- 规模：L
- 依赖：M1-038, M1-021（小队/名额/部署校验）, M1-026（部署/小队处理与可靠事件）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §4.7; GAMEDESIGN §20.1; GAMEDESIGN §20.2; GAMEDESIGN §4.4; GAMEDESIGN §2.4; GAMEDESIGN §10.2; ARCHITECTURE §16.1

## 背景
M1 的 20 人测试局里真人不足时由 bot 填满（每队上限 50 人，M1 测试服容量 20，每队 10）。ARCHITECTURE §16.1：分层 指挥官 bot → 小队长 bot → 士兵 bot；兵种选择与部署由效用 AI 决定并遵守名额；真人不足填充、真人加入时淘汰 bot（优先替换阵亡中的 bot）。GD §2.4：真人加入某队时 bot 立即让出（先移除处于 DEPLOY 状态的 bot，其次存活 bot，载具中的 bot 最后）；真人离开 10 s 后 bot 补位。GD §20.2：bot 昵称带 `[BOT]` 前缀；服务器浏览器显示 bot 数。M1 的 5 个兵种：Officer、Rifleman、Medic、Support、Crewman（MILESTONES §2.1）。R1：Garrison 由 Officer 放置（消耗 Supplies）；R6：Supplies 与指挥官资源分离。

## 具体内容
### 要创建/修改的文件
- `crates/bots/src/squad_leader.rs` —— 小队长 bot：目标据点选择、给队员下达 `SquadOrder`（Move/Attack/Defend/Hold）、倒地/阵亡时被替代。
- `crates/bots/src/role_select.rs` —— 兵种效用选择：队内缺口（Officer/Medic/Support 优先）、名额与解锁人数（GD §4.4）、出生点选择（GD §20.3 B-17：最近可用非封锁点，HQ 仅在无其他点时）。
- `crates/bots/src/fill.rs` —— 填充管理器：目标人数、真人加入让位、真人离开 10 s 后补位、`[BOT]` 昵称。
- `crates/bots/src/roles/officer.rs` —— Officer bot：在 ≥ 2 名队员在身边且附近 Supplies 足够（≥ 50，己方/中立区）时放置 Garrison（B-07、R1）。
- `crates/bots/src/roles/support.rs` —— Support bot：搬运 Supply Crate 至 Garrison 放置点附近、在战斗点放 Ammo Crate、响应补弹请求（B-09）。
- `crates/server/src/bot_host.rs`（修改）—— 宿主侧接入：bot 通过与真人相同的 deploy/squad 消息处理器（M1-026）入队，不得直接改 `World`。
### 要实现的行为
1. 每队开局按需创建 Infantry Squad（Alpha、Bravo…，GD §4.2），每队 1 名 Officer bot 当队长（若有真人占用则让位）。编制为上限：不强行凑满 6 人。
2. 兵种选择遵守 GD §4.4：Officer 仅队长、需小队 ≥ 2 人；Medic ≤ 2/队；Support ≤ 1/队且需 ≥ 3 人；未解锁则降级为 Rifleman；Crewman 仅 Armor Crew。
3. 真人加入替换：按 GD §2.4 顺序移除 bot；断线 10 s 内不补位抖动。
4. 小队长每 2–5 s 重选目标（据点活跃度、人数优势），下达 Order；队员（M1-038）响应。
5. Officer bot 放 Garrison 时遵守 GD §10.2 间距/坡度/50 m 无可见敌/Supplies 检查（由 sim 校验，bot 只发起合法请求并处理拒绝原因）。
6. Support bot：从 HQ/补给点取箱、搬运、放置；被拒绝则 30 s 退避。
### 接口与数据
```rust
pub struct FillManager { pub fn reconcile(&mut self, view: &dyn WorldView, team: Team, target: u8) -> Vec<FillAction> }
pub enum FillAction { SpawnBot{team, role_hint}, RemoveBot{id, reason}, }
pub fn select_role(view: &dyn WorldView, squad: SquadId, available: &RoleQuota) -> RoleId; // 效用
pub enum SquadOrder { Move(Vec3), Attack(ObjId), Defend(ObjId), Hold }
```
`FillAction` 由 server 的 bot_host 执行（经正式部署路径）。

## 不做什么
- 不做 Commander bot / LLM 指挥官、资源与能力调度（M2-027、M3）。
- 不做 Engineer/Anti-Tank/MG/Spotter/Sniper bot 行为（M2-027、M3）。
- 不做载具驾驶 bot（M3）；Crewman bot 在 M1 仅占座位不驾驶（驾驶 bot 留 M3）。
- 不实现 Outpost/OP 放置（M2/M3）。

## 验收标准
1. 20 人场景（每队 10，2 名真人+18 bot）：开局 30 s 内 bot 填满，各队总人数 = 10（场景测试 `fill_20_players`）。
2. 真人加入 bot 满员的队伍：1 s 内一个 bot 被移除且顺序符合 GD §2.4（DEPLOY 中 > 存活 > 载具内），真人成功入队（`fill_human_replaces_bot`）。
3. 所有 bot 的兵种选择满足名额与解锁规则：运行 1000 次随机局面，违规次数 = 0（`role_quota_property`）。
4. Officer bot 在条件满足时 90 s 内放置 ≥ 1 个合法 Garrison；非法请求被 sim 拒绝时无重试风暴（30 s 内同类请求 ≤ 2 次）。
5. bot 昵称均带 `[BOT]` 前缀；`/metrics` 输出 `bots_active{team}`。
6. 真人离线 10 s 后才补位，10 s 内抖动重连不产生增删（`fill_flap_guard`）。
7. 5 min 无头局：无 bot 因非法 deploy 被服务器踢出；非法部署请求率 < 1%。

## 测试要求
- 单元测试：`select_role` 对名额/解锁的全覆盖；`FillManager` 让位顺序；小队长目标选择。
- 集成/无头测试：1 个 headless 真人 + bot 填满 20 人局，30 min 无异常（与 M1-068 共用场景）。
- 性能基准：bot 总思考 ≤ ARCHITECTURE §16.5 的 3.0 ms/tick（60 bot）；本任务新增逻辑 ≤ 0.2 ms/tick。
- 回归：M1-038 场景测试保持通过。

## 性能预算
- ARCHITECTURE §16.5：bot 合计 ≈ 3.0 ms/tick（§6.10 阶段 6）；小队长/填充逻辑按秒级运行，不得每 tick 全量遍历。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test -p bots -p server`）；`cargo clippy -D warnings` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材无新增；缺失素材需求 ID 已列出（无）
- [ ] 文档同步：任何与 GAMEDESIGN §20 不一致处写 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：Officer/Support 专属行为若超一次会话，按 README §5.6 追加续作 `M1-039b`（Support bot）。
- Garrison 规则细节依赖 M1-019 的实现；bot 必须读取 sim 返回的拒绝原因枚举而非自行重复校验。
- bot 难度档选择（Recruit/Regular/Veteran）默认 Regular，由服务器配置项指定。
