# M0-013 sim：生命、死亡与简易重生

## 元数据
- 里程碑：M0
- 工作流：sim
- 规模：S
- 依赖：M0-012
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §7.1; GAMEDESIGN §7.2; GAMEDESIGN §10.6; GAMEDESIGN §7.5; ARCHITECTURE §6.8.3

## 背景
M0 没有倒地/救起（M1-016）：HP ≤ 0 即死亡。需要让 M0 的 10 名 bot 与真人形成"击杀 → 等待 → 重生"的闭环（演示脚本 §1.4 第 2 步"被击杀后 N 秒重生"）。规则全部在 `sim`（服务器不得内含规则，ARCHITECTURE §4.3.4）。
数值取自 GAMEDESIGN：最大生命 100 HP（§7.2）；击杀归属取"最后造成伤害者"的最简版（完整规则 §7.5 在 M1-016）。重生计时（GAMEDESIGN §10.6）：每队 `T_wave` 从对局开始每 15 s 一个波次 tick；死亡镜头 3 s；HQ 类出生点 `base_wait = 5 s`；`ready_time = t_kia + base_wait`；波次类（HQ/Strongpoint）`spawn_time = ceil_to_next_wave_tick(max(ready_time, t_pick))`。M0 只有"固定出生点"（来自 M0-009 `spawns.json`，两队各一点），玩家死后自动在 `t_pick = t_kia + 3 s` 选择该点，按 HQ 规则计算出生时刻。数值【待调参】【待测 → REFERENCE_MEASUREMENTS §4.3 H-14/H-15】。

## 具体内容
### 要创建/修改的文件
- `data/rules/respawn.json` + `data/schemas/rules.schema.json`（`death_cam_ms:3000`、`base_wait_ms:5000`、`wave_interval_ms:15000`、`wave_aligned:true`、`_meta.status:"initial"`、`tunable`）；`GameData` 增加 `rules`（M0-004 的 build.rs 同步纳入）。
- `crates/sim/src/health.rs`（`Health{hp,max_hp}`、`LifeState{Alive,Dead{t_kia,killer,weapon,…}}`）、`crates/sim/src/respawn.rs`（`RespawnQueue`、`respawn_time(t_kia,t_pick,policy,tick_hz)->Tick`）、`SimEvent::Kill{victim,killer:Option<PlayerId>,weapon:u8,headshot:bool,dist_m:f32}`、`SimEvent::Spawn{player,pos}`。
- `crates/sim/tests/damage_death.rs`。
### 要实现的行为
1. `Hit` 事件应用到 `Health`：`HP -= D`；`HP ≤ 0` → `LifeState::Dead`，发 `Kill`（`headshot` 取自命中区）；死亡玩家不再接受输入/射击/被命中；其武器状态在重生时重置（满弹匣与备弹）。
2. 重生：玩家死亡后在 `t_kia + death_cam_ms` 自动选点；`spawn_time` 按上面公式取整到 tick（`ceil`）；到点后在所属队的出生点（可加 ≤ 3 m 的确定性随机偏移，`Rng::for_(.., Purpose::Respawn)`）生成，HP=100，满弹，朝向面向地图中心；M0 无出生保护。
3. `wave_aligned=false` 时跳过波次取整（供测试与 sandbox 配置）。
4. `World` 需要"击杀/重生"事件被 `StepOutput.events` 带出（server 分发给客户端是 M0-019，击杀播报 C4 在 M1-010，M0 仅内部事件+日志）。
5. 玩家被移除（断线）时清理其死亡/重生记录。
### 接口与数据
- `World::player_life(id)->LifeStateView`；`WorldView` 暴露 `health`、`alive`；`EntityState.health_q`（`protocol`）由 `hp` 量化为 3–8 bit（M0-006 位宽）。

## 不做什么
- 不做倒地/救起/流血/放弃（M1-016）；不做出生保护、自杀惩罚、同点洪水限制（M1-019）。
- 不做 Garrison/Outpost/部署界面选点（M1-019/021）。
- 不做计分与击杀播报网络消息（M1-010/M2-017）。

## 验收标准
1. 致死发数与 `/data` 一致：`damage_death::stk_matches_data` 由数据推导每把枪在 50/300/600/800 m 的躯干致死发数（Garand：2/2/2/2；Kar98k：≤200 m 为 1，其余为 2），模拟实际射击验证实际发数相同（8 个用例全绿）。头部命中一发致死（Garand 70×3.0=210、Kar98k 300）。
2. 重生计时：测试用 `wave_interval_ms=15000`、`base_wait_ms=5000`、`death_cam_ms=3000`，死于 t=7.000 s → `ready=12 s`，`t_pick=10 s`，出生时刻 = 15.000 s（下一个 15 s 波次 tick）；死于 t=1.0 s → `ready=6 s`、`t_pick=4 s`，取整后出生于 15 s；死于 t=16.0 s → `ready=21 s`，出生于 30 s；`wave_aligned=false` 时出生于 `max(ready,t_pick)`。出生 tick 与公式误差 ≤ 1 tick（30 Hz）。
3. 重生后状态：HP=100、弹匣/备弹等于 loadout 初值、位置在出生点 3 m 内；重生前玩家实体不可被命中（用射击测试验证命中数为 0）。
4. 死亡玩家的输入被忽略（10 个 tick 的位移 = 0，开火无效）。
5. 确定性：包含死亡与重生的 5000 tick 场景两次运行 `state_hash` 逐 tick 相同。

## 测试要求
- 单元测试：`respawn_time` 的取整边界（恰在波次 tick 上死亡、跨波次）、队伍出生点选择、断线清理。
- 集成/无头测试：20 玩家随机对射 3000 tick 无 panic、击杀数 = 死亡数、重生数 ≥ 击杀数 − 在途数。
- 性能基准：无新增。
- 回归：更新 golden `state_hash`（M0-007/M0-012 的 golden 因新增系统而变，需写明原因）。

## 性能预算
规则阶段 ≤ 0.7 ms/tick（ARCHITECTURE §6.10 阶段 5）；本任务仅增量 O(死亡玩家数)，记录即可。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 说"固定出生点计时重生"；本任务采用 GAMEDESIGN §10.6 的 HQ 公式（含波次取整）而非固定秒数，以便 M1 平滑过渡。若试玩觉得 M0 等待过长，可调 `wave_aligned=false`（数据）。
- `data/rules/` 与 `rules.schema.json` 是 M0-004 schema 列表之外的新增（INDEX 未单列），属细化，PLAN_ISSUES 记一条。
- 死亡/重生数值待 H-14/H-15 对标（M1-071 批次），M0 保持 `initial`。
