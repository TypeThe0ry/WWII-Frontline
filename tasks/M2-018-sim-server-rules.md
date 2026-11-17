# M2-018 sim：自动平衡、TK、投票与挂机

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：M
- 依赖：M2-017, M1-021
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §19; GAMEDESIGN §19.1; GAMEDESIGN §19.2; GAMEDESIGN §19.3; GAMEDESIGN §19.4; GAMEDESIGN §2.4; GAMEDESIGN §6.6

## 背景
这些"服务器规则"决定对局公平与可运营性。判定逻辑进 `crates/sim`（确定性、可单测、供 balance-sim 复用），**执行副作用**（踢人、移队）由 sim 输出 `ServerAction` 指令，再由 `crates/server` 的 C0 处理器落地。数值【待调参】，读 `/data/server_rules.json`。
**阵营与换队（GD §2.4）**：每队 ≤ 50（真人 + bot）；新玩家只能加入使 `H_t − H_other ≤ 2` 成立的队；`|H_a−H_b| ≥ 3` 且目标队更多时锁；加入后 3 min 内不得换队，两次换队间隔 ≥ 600 s；换队时存活角色 KIA（不计死亡）、退出小队、Commander 取消；Bot 动态让位（先移除 DEPLOY 中的 bot，其次存活，最后载具内）；真人离开 10 s 后 bot 补位。
**自动平衡（§19.1）**：每 30 s 检查，若 `|H_a−H_b| ≥ 4` 持续 90 s → ① 向多的一队弹邀请（+20% XP 本局）；② 60 s 无响应则强制移动，选择优先级：最近加入、非小队长/Commander、不在小队或小队最小、本局得分最低；不移动 90 s 内刚被移动者；每人每局最多被强制 1 次。
**TK（§19.2）**：友军伤害系数在 M1-014（枪 ×0.5、爆 ×0.7、近战 0、碾压 1.0、坦克炮 0.5、自伤手雷 1.0）；TK 记录保存 30 min；受害者 15 s 内选"原谅/惩罚"（惩罚 `tk_strikes+=1`）；阶梯：每次 TK 分 −30；`tk_strikes=2`（10 min 内）警告，5 min 内不能担任 MG/AT/Engineer/Crewman；`=3` 锁 Rifleman 5 min 且不可认领载具；`=5`（本局）踢出并 30 min 冷却；恶意行为交管理员。
**投票（§19.3）**：Kick——任意真人发起，局内 ≥ 3 min，每人每 5 min ≤ 1 次，对象不得为管理员，45 s，同队票 ≥ 8 且赞成 > 反对 ×2；Remove Commander——队内 Officer ≥ 2 联署，在任 ≥ 10 min，同队 ≥ 40% 赞成；Map Vote——局终自动、3 候选、最高票、平则随机；Mute——管理员即时，玩家发起需同队 ≥ 6 票；Surrender 禁用。
**AFK（§19.4）**：存活 120 s 无有效输入（移动/视角/射击/交互/语音/聊天/地图操作）→ 90 s 起提示、120 s 起 30 s 倒计时后踢出（小队长/Commander 150 s+30 s）；DEPLOY/KIA 中 300 s 踢出；载具中不动同局内规则并标"无人驾驶"；重复同一输入序列视为 AFK 且计入统计异常；小队长 AFK 自动移交，Commander AFK 自动弃权。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/serverrules/{balance,tk,vote,afk,teamswitch}.rs`
- `crates/sim/src/serverrules/mod.rs` —— `ServerRules::step(&mut self, ctx) -> SmallVec<ServerAction>`；`enum ServerAction{ InviteSwitch, ForceMove, Kick, LockRole, LockVehicle, VoteResult, Warn }`
- `crates/server/src/c0/server_rules.rs` —— 把 M2-007 的 `VoteStart/VoteCast/TeamSwitchRequest/ForgivePunish` 转发到 sim，并把 `ServerAction` 落地为 C4 事件与断开连接（规则不写在 server）
- `data/server_rules.json`
### 要实现的行为
1. 加队/换队校验函数 `can_join(team)`/`can_switch()`；返回原因码用于 UI 提示。
2. 平衡：30 s 周期检查、90 s 持续窗口（以 tick 计）、邀请 60 s 窗口、强制移动候选排序（稳定、可复现，按 `(joined_tick desc, score asc, player_id)`）。
3. TK：事件 `Kill{friendly}` → 记录 → 弹 `TkPrompt`（15 s）→ 处罚阶梯；10 min / 30 min 滑动窗口用 tick 环形缓冲。
4. 投票状态机：唯一进行中的 Kick/Mute；人数门槛读当前队伍真人数（提示：8 票门槛在小局不可达，见备注）；结果事件 `VoteResult`。
5. AFK：输入活动计数器（含输入序列重复检测：10 s 内 ≥ 8 次相同 `buttons+移动` 序列）；倒计时与踢出动作；Commander/队长特例。
### 接口与数据
- `ServerRules` 状态参与 `state_hash`；所有计时以 tick；随机选择用 `Rng::for_(tick,0,"vote_tie")`。

## 不做什么
- 管理员强制操作、举报、封禁 → M2-023；聊天/标记 → M2-019；账号/战绩 → M2-022。
- 投票/提示 UI → M2-043；计分（TK −30 分值本身）→ M2-017。
- 统计异常评分（爆头率等）→ M3/M4 运营（ARCH §15.3），此处只提供"重复输入"标志。

## 验收标准
1. 各规则边界单测（`cargo test -p sim serverrules`）：`H_t−H_other = 2` 允许、3 锁定；换队 179.9 s 拒绝/180 s 通过；两次换队 599 s 拒绝/600 s 通过。
2. 自动平衡：差 4 持续 89 s 不触发、90 s 触发邀请；邀请后 60 s 无响应强制移动；候选排序用例 ≥ 5（含不可移动对象：小队长、Commander、90 s 内已移动、已被强制 1 次）。
3. TK 惩罚阈值与 /data 一致：第 2 次惩罚（10 min 内）警告并锁 MG/AT/Engineer/Crewman 5 min；第 3 次锁 Rifleman 且不可认领载具；第 5 次踢出 + 30 min 冷却；11 min 前的 strike 过期不计。
4. 投票：Kick 同队 8 赞 3 反 通过、8 赞 4 反 不通过（> ×2 条件）；Remove Commander 40% 边界；Mute 6 票；Map Vote 平票随机且同种子可复现。
5. AFK：119 s 无输入不踢、倒计时在 120 s 开始、150 s（队长）；DEPLOY 中 300 s 踢出；重复输入序列检测正确率：构造 20 段合成序列（10 重复/10 真实回放）误判 ≤ 1。
6. `ServerAction` 序列在同种子下逐 tick 相同；本模块 CPU ≤ 0.05 ms/tick。

## 测试要求
- 单元测试：时间窗口边界、并发投票拒绝、玩家离队时的状态清理。
- 集成/无头：headless 30 客户端模拟加入/离开/换队序列，队伍差始终 ≤ 目标；TK 序列触发踢出，连接被关闭。
- 性能基准：`cargo bench -p sim serverrules_tick`。
- 回归：确定性哈希。

## 性能预算
ARCH §6.10 阶段 5 内 ≤ 0.05 ms/tick；C0 速率上限遵守 ARCH §15.1（部署 1/s、聊天 2/s、举报 1/min）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 计划缺口：没有任何任务单独负责"server 的 C0 处理器转发 Vote/TeamSwitch/Forgive"，本任务把薄转发层并入（只转发、不含规则）。
- Kick 门槛"同队票 ≥ 8"在 ≤ 16 人局不可达；如要调整须由 Fable 改 GD，本任务按文档实现。
- Commander 相关规则（罢免、AFK 弃权）在 M2 无席位时只做单测，不接线。
