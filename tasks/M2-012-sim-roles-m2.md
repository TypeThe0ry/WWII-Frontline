# M2-012 sim：M2 兵种特有能力

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：L
- 依赖：M2-006, M1-013, M1-011
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §5.7; GAMEDESIGN §5.8; GAMEDESIGN §5.11; GAMEDESIGN §5.12; GAMEDESIGN §5.13; GAMEDESIGN §5.1; GAMEDESIGN §9.3; GAMEDESIGN §6.5

## 背景
M2-006 写入了 5 个新兵种的数据；M1-011 实现了移动/体力/翻越，M1-013 实现了武器类。本任务把"兵种独有能力"接到 sim 规则里（GD §5.1 总览表）：
- **Assault**：冲刺体力消耗 −20%（20/s → 16/s，GD §9.3）；翻越时间 −25%（0.8 s → 0.6 s）；携带额外手雷与烟雾；近距 SMG/霰弹。
- **Automatic Rifleman**：压制系数 ×1.0；射速可切换（BAR 慢 350 / 快 550 rpm，StG 44 半自动/全自动）；带两脚架。
- **Machine Gunner**：重机枪 + 两脚架，**部署后后坐 ×0.35**（`vkick`、`hσ` ×0.35；部署耗时 0.6 s，MG 0.8 s；需趴下或有可靠支撑面——胸高矮墙/窗台，射线检测；移动或离开支撑面自动收起，GD §6.5）；压制系数 ×1.2/发；未部署 MG 持续射击叠加 1.5° sway。
- **Anti-Tank**：持火箭/Panzerfaust（发射本身由 M2-014）；可用 Supplies 建造并操作 AT 炮（M2-011）；**不携带地雷**。
- **Engineer**：工具箱（建造/维修，对接 M2-010/M2-016）、剪线钳（铁丝 4 s 开口）、探雷（8 m 内显示敌地雷）、拆雷（AP 1.5 s、AT 3.0 s）、修理载具（+60 HP/s，每 6 HP 耗 1 Supplies，修毁坏模块 8 s，GD §14.8）；布雷/炸药的**使用**由 M2-014 实现，本任务只做工具侧挂接点。
所有数值来自 `/data/roles/*.json`（`sprint_mult`、`vault_time_mult`、`abilities[]`，GD §5.1 末尾；客户端不硬编码）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/role/mod.rs` —— `RoleAbilities` 解析 `abilities[]` 并在移动/武器/交互系统中应用修正
- `crates/sim/src/role/{assault,ar,mg,at,engineer}.rs` —— 各兵种钩子（修正乘子、状态机）
- `crates/sim/src/weapon/bipod.rs` —— 两脚架部署判定（趴下/支撑面射线）、收起条件
- `crates/sim/src/role/engineer_tools.rs` —— 剪线、探雷（返回可见敌雷列表）、拆雷、修理工具交互状态机
- `crates/sim/tests/roles_m2_*.rs`
### 要实现的行为
1. `movement` 系统读取 `role.sprint_stamina_mult`、`role.vault_time_mult`；Assault 16/s、0.6 s，其余 20/s、0.8 s。
2. 两脚架：`try_deploy()` 条件=趴姿或前方 1.0 m 内胸高（0.9–1.2 m）可支撑面；成功后武器后坐参数乘 0.35（读 weapon `deploy.recoil_mult`）；耗时 0.6 s（MG 0.8 s）期间不可开火；移动/离开支撑面/换武器 → 收起。
3. AR 射速切换：`fire_mode` 输入事件（`aux` 位）在 `[slow,fast]` 间切换，切换耗时 0.2 s【待调参】；同一 tick 重复输入去抖。
4. MG 压制系数 ×1.2 作为 `SuppressionSource.weight` 输出给 M1-017。
5. AT：装备 loadout 校验（无地雷）；"使用中 AT 炮"状态标记（M2-011 读取）。
6. Engineer：`cut_wire(structure_id)` 需 4 s（120 tick）持续交互，中断条件同 M1-020；`mine_detect()` 8 m 内返回 `Vec<MineId>`（仅己方可见，敌雷以 5 s 图标事件下发）；`defuse(mine)` AP 1.5 s / AT 3.0 s；修理交互把 `repair_rate` 作用于载具（实际模块逻辑 → M2-016）。
7. 兵种名额/解锁校验沿用 M1-021，`deploy` 校验读取 M2-006 的 `max_per_squad / squad_size_min`。
### 接口与数据
- `pub struct RoleMods{ sprint_stamina_mult:f32, vault_time_mult:f32, suppress_weight:f32, bipod_recoil_mult:f32, can_carry_mines:bool, tools:ToolSet }`，由 `GameData.roles` 在 `spawn_player` 时解析，禁止在代码中写死 0.8/0.75/0.35。

## 不做什么
- 火箭/地雷/炸药的使用与爆炸 → M2-014；AT 炮 → M2-011；建造菜单 → M2-010/M2-039；载具模块修理 → M2-016。
- BAR/MG 的射击节奏、过热与换弹链 → M2-013；本任务只处理"能力开关与修正乘子"。
- Spotter/Sniper/Tank Commander/Commander 的能力 → M3。

## 验收标准
1. 各兵种特有数值与 /data 一致：用 `RoleMods` 回读断言 Assault `sprint_stamina_mult=0.8`、`vault_time_mult=0.75`；MG `bipod_recoil_mult=0.35`；MG 压制 1.2（单测 `roles_m2_data_match`）。
2. 体力：Assault 冲刺 5 s 消耗 80（±1），Rifleman 100；翻越 1.3 m 障碍耗时 Assault 18 tick（0.6 s），Rifleman 24 tick（0.8 s）。
3. MG 部署判定单测：趴姿通过；站姿无支撑面拒绝；站姿 + 0.95 m 矮墙通过；0.5 m 矮墙拒绝；部署耗时 24 tick（0.8 s）；移动 0.1 m 自动收起。部署后连续射击 30 发的平均垂直后坐 = 未部署的 0.35 ± 2%（固定种子）。
4. AR 切换 BAR 慢→快：测得 cyclic rate 分别 350 ± 2% 与 550 ± 2% rpm（headless 测量，依赖 M2-013 的射速实现；若 M2-013 未完成则以数据断言替代并记 partial）。
5. 剪线钳 4 s 开口：120 tick ± 1；中断后进度保留规则单测。探雷 8 m 边界：7.9 m 可见、8.1 m 不可见。
6. Anti-Tank loadout 含地雷时 `deploy` 被拒。

## 测试要求
- 单元测试：上述每条 + 异常（死亡/倒地时部署收起、载具内禁用）。
- 集成/无头：headless 脚本：MG 趴下部署压制 + AR 切换；Engineer 剪线。
- 性能基准：`cargo bench -p sim role_mods_tick`（100 玩家）≤ 0.05 ms/tick。
- 回归：确定性哈希。

## 性能预算
ARCH §6.10 阶段 2「角色移动 3.0 ms（100 KCC）」不得回归：以 `cargo bench -p sim kcc_step` 对比 M1 基线，增幅 ≤ 2%。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 依赖未含 M2-010（剪线钳作用对象是 M2-010 的铁丝网）：编号顺序保证 M2-010 先完成，建议 Fable 补依赖；若 M2-010 未完成则剪线验收记 `未测`。
- AR/MG 的 `fire_mode` 输入位使用 `InputCmd.aux`（ARCH §8.2：开火模式切换/标记等），不新增消息。
- 数值状态 `initial`，校准见 M2-054。
