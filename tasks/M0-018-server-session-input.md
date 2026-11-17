# M0-018 server：会话、输入缓冲与校验

## 元数据
- 里程碑：M0
- 工作流：server
- 规模：M
- 依赖：M0-017, M0-013
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.3; ARCHITECTURE §15.1; ARCHITECTURE §6.1; ARCHITECTURE §9.3

## 背景
把"连接"变成"玩家"：握手完成后创建会话、把玩家放进 `sim::World`、接收 C1 输入、缓冲、校验、在每 tick 交给 `sim.step`。规则：**不在 `server` 内实现游戏规则**（ARCHITECTURE §4.3.4）——这里只做输入层的反作弊/容错与 tick 对齐。
服务器输入管线（ARCHITECTURE §6.3）：每玩家一个按 `tick` 索引的**输入缓冲（容量 8 tick）**，按 `seq` 去重；丢弃过旧（< 当前 tick − 2）与过远（> 当前 tick + 10）输入；校验：`seq` 单调（环绕比较）、移动向量模长 ≤ 1、yaw/pitch 变化率不超过硬上限（鼠标 8000°/s【待调参】）、按键位合法；缺失输入策略：**重复上一条移动输入最多 5 tick（≈ 167 ms），之后清零移动、保持视角**（§6.1 第 2 步亦同）；若连续 9 tick 无任何输入到达（≥ 300 ms），认为连接疑似中断，保持静止（§6.8.2 说明）。§15.1 的"输入/视角/速率限制"行是本任务的校验清单（移动速度/射速等由 sim 权威处理）。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/{session.rs,input_buffer.rs,validate.rs,sim_bridge.rs,world_loader.rs}`
- `crates/server/tests/{input_buffer.rs,session_flow.rs}`
### 要实现的行为
1. **世界装载**：启动时按 `--map m0` 读取 `/data/maps/m0/chunks/c_*.bin`（M0-009）→ `CollisionData`（I/O 在 server 内完成，传字节给 sim）→ `World::new`；`spawns.json` 提供出生点；`--mode sandbox`。
2. **会话生命周期**：`Connecting → Joined → Gone`。握手成功（M0-016/017）后向 sim 命令通道发 `Join{player_id, team, nick}`；队伍分配：人数少的一队优先（US/GER 交替）；`World::spawn_player` 在出生点生成；断线 → `remove_player` 并释放 `PlayerId`（延迟 1 s 防止 id 立刻复用造成客户端混淆）。
3. **输入缓冲**：`InputBuffer`（8 tick 环，按 `cmd.tick` 索引）；`push(cmd) -> PushResult{Accepted,Duplicate,TooOld,TooFar,Invalid(reason)}`；每 tick 开头 `take(tick) -> Option<InputCmd>`。`InputBundle` 的 1..3 条按各自 `seq/tick` 入缓冲；`ack_snapshot_tick` 与 `pred_hash` 存入会话状态供 M0-019；`FireCmd` 随附最近 2 条按 `seq` 去重后存储（M0 仅记录）。
4. **校验**（`validate.rs` 是**单一入口 `validate_input(&mut PlayerInputState, &InputCmd)`**，bot 与真人共用，M0-026 的验收依赖此点）：`seq` 环绕单调；`hypot(move_x,move_z)/127 ≤ 1 + 1/127`；`buttons` 保留位（bit14/15）必须为 0；`weapon_slot` ≤ 配置；yaw/pitch 与上一条的角速度 ≤ 8000°/s（用 `tick` 差折算；pitch 范围已由类型限定）。违规：钳制或丢弃，并 `input_rejected_total{reason}`++；同一连接连续 30 条违规 → 断开。
5. **缺失输入**：`resolve_input(tick)`：有 → 用；无且 `gap ≤ 5` → 复制上一条的移动轴与姿态位，**清除 fire/use/reload/melee 等边沿位**，保持视角；`gap > 5` → 移动清零保持视角；`gap ≥ 9` → 置 `suspected_stall`，记 `session_stall_total` 指标并日志。
6. **tick 对齐**：sim 线程每 tick 对所有 Joined 玩家按 `PlayerId` 升序产出 `inputs: Vec<(PlayerId, InputCmd)>` 交给 `World::step`。
7. 速率限制（C0 消息）：`Ping` ≤ 5/s，其余 C0 ≤ 10/s（M1 才有部署/聊天），超限丢弃并计数。
### 接口与数据
- `PlayerInputState{last_seq,last_tick,last_yaw,last_pitch,gap,last_ack_snapshot_tick,last_pred_hash,pending_fire:[FireCmd;2]}`；`SimCommand::{Join,Leave,Input(PlayerId,InputBundle)}`。

## 不做什么
- 不做快照与 AOI（M0-019）；不做延迟补偿回溯（M1-025）；不做射速/弹药校验（在 sim，M0-012）；不做完整反作弊（M1-027）。
- 不做 gateway ticket 验证（M1-029）；不做部署/小队消息（M1-026）。
- 不做 bot 宿主（M0-026，但本任务的 `validate_input` 是其必经路径）。

## 验收标准
1. 单测 `input_buffer`：乱序到达（5 条按随机顺序）→ 按 tick 序取出；重复 `seq` → `Duplicate` 且只处理一次；`tick < now − 2` → `TooOld`；`tick > now + 10` → `TooFar`；容量 8 的环覆盖不破坏较新数据。各类用例 ≥ 15 个全绿。
2. 缺失输入：连续 10 tick 不发输入——前 5 tick 玩家保持上一移动方向，**第 6 tick 起位移为 0**（断言速度 = 0），视角不变；fire 位在重复输入中被清除（用连续开火输入后断网验证不再开火）。
3. 校验：yaw 在 1 tick 内变化 > 8000°/33.3 ms（≈ 266°）→ 被钳制/拒绝并计数；`move` 模长 1.2 → 被钳制到 ≤ 1；保留位置 1 → 拒绝；连续 30 条违规 → 连接关闭。
4. 会话流程集成测试 `session_flow`：2 个 WS + 2 个 WT 客户端握手 → 玩家出现在 `World.players()`，队伍均衡（2:2）；发送 3 s 的合成输入后服务端位置与同输入离线 `World` 结果完全一致；断开后实体移除、`server_players` 指标回落。
5. 20 客户端以 30 Hz 发输入 60 s：`server_tick_ms` P99 ≤ 28 ms；`input_queue_full_total == 0`。
6. 所有拒绝/丢弃路径无 panic（随机字节 + 随机合法字段 fuzz 10^5 条）。

## 测试要求
- 单元测试：`seq` 环绕比较（65535→0）、角速度折算、缺失输入状态机、PlayerId 复用延迟。
- 集成/无头测试：验收 4/5；M0-027 复用。
- 性能基准：输入摄取与校验阶段 ≤ 0.8 ms/tick（ARCHITECTURE §6.10 阶段 1，100 玩家合成，记录）。
- 回归：golden 输入序列 → 位置轨迹。

## 性能预算
输入摄取与校验（含反作弊校验）≤ 0.8 ms/tick @100 玩家（ARCHITECTURE §6.10 阶段 1）；M0 以 20 玩家测，按线性外推并记录。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 缺失输入时"重复上一条"要避免把一次性动作（跳、开火、换弹）无限重放，故清除边沿位：这是对 ARCHITECTURE §6.3 的细化（原文只说重复移动输入），记 PLAN_ISSUES。
- yaw 角速度上限 8000°/s 为【待调参】；鼠标高 DPI 玩家的合法瞬时转速需 M1 实测校准，别在 M0 收紧。
- ARCHITECTURE §6.3 的 `tick` 窗口基于 `server_tick`：输入 `tick` 由客户端按 §6.2 的"输入提前量"填写，M0-021/022 须保证。
