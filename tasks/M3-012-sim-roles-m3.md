# M3-012 sim：Commander / Tank Commander / Spotter / Sniper 特有规则

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 依赖：M3-001, M2-012, M3-006
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §5.4; GAMEDESIGN §5.14; GAMEDESIGN §5.16; GAMEDESIGN §5.17; GAMEDESIGN §4.1; GAMEDESIGN §4.4; GAMEDESIGN §14.1; GAMEDESIGN §14.6; GAMEDESIGN §5.2

## 背景
M2-012 已把 Assault/AR/MG/AT/Engineer 的兵种特有能力接入 sim。本任务补齐新增四个兵种的"兵种身份规则"，把数据（M3-001）与功能模块（M3-006 指挥官、M3-010 Spotter/OP、M3-011 狙击、M3-016/017 载具）接成完整兵种：
- **Commander**：唯一席位（M3-006）、可放 Garrison（R1，消耗 Supplies：己方/中立区 50，敌方区 100【待调参】）、望远镜、无线电、Command 频道。
- **Tank Commander**：Armor Crew 队长；在 HQ 载具刷新点**认领**载具（成本由 Commander 资源池支付，GD §14.6）；车长位；双筒望远镜；车辆无线电；仅 Armor Crew 可选，每 Armor Crew 1 人。
- **Spotter**：Recon Team 成员；10× 望远镜；放置 OP 与 Garrison（R1）；每 Recon Team 1 人。
- **Sniper**：Recon Team 成员；与 Spotter 共享 OP 出生；每 Recon Team 1 人。
编制（R2 与 GD §4.1/§4.4）：Armor Crew ≤ 3 人、每队最多 4 个【待调参】；Recon Team ≤ 2 人、每队最多 2 个；Commander 1。全队上限：Tank Commander 4、Crewman 8、Spotter 2、Sniper 2、Commander 1。允许无小队散兵但只能 Rifleman。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/roles/commander.rs`、`tank_commander.rs`、`spotter.rs`、`sniper.rs` —— 兵种工具/能力的 sim 入口（`on_use_tool` 派发到对应模块）。
- `crates/sim/src/squad/composition.rs`（M1-021 已有）—— 扩展 `role_valid_on_deploy`：Armor Crew 与 Recon Team 的特有限制、`min_squad_size` 与阈值动态变化（人数降到阈值下：存活者不强制换，下次部署重检，GD §4.4）。
- `crates/sim/src/vehicle/claim.rs` —— `claim_vehicle(player, vehicle)`：TC 靠近载具（≤ 6 m）按交互占据；整个 Armor Crew 可上车；认领后乘员离队不影响载具；认领者所在小队当前人数 ≥ 载具需求（重/中/轻装甲 3/2/2，GD §14.6.1），不足拒绝；每 Armor Crew 同时 1 辆。
- `crates/sim/src/roles/binoculars.rs` —— 望远镜视野状态（Officer 4×、Spotter 10× FOV 6°、TC 双筒）；标记能力由 M3-010 消费。
- `crates/sim/tests/roles_m3.rs`。
### 要实现的行为
1. 名额表自动化：用 `/data/roles` 与 GD §4.4 生成 14 行断言（每小队上限、解锁小队人数、全队上限、专属编制）。
2. Commander 席位与 Garrison 套件：Commander/Officer/Spotter 放置 Garrison（复用 M1-019 的放置规则，距敌/区域校验）；Engineer 无此能力（R1）。
3. TC 认领：见上；Tank Commander 在载具中占车长位（`seat=commander`）；车长缺位时装填 ×1.3（GD §14.1，供 M3-014/015 使用的标志位）。
4. Spotter/Sniper：Recon Team 创建/限制（≤ 2 人、每队 ≤ 2 个）；共享 OP 出生（M3-010 提供出生点，本任务提供成员资格）；Sniper 的迷彩/隐蔽接口开关。
5. 兵种等级（GD §5.2）：`XP(n)=125·n·(n−1)`；Commander 等级以指挥分计（能力命中得分钩子由 M3-006/008 提供）；等级只解锁 loadout，不加战斗力。
6. 所有兵种能力枚举经 `/data/roles/*.abilities[]` 驱动，禁止按兵种名硬编码分支超过 1 处派发表。
### 接口与数据
- `RoleRules::can_choose(role, squad, team) -> Result<(), RoleReject>`；`RoleReject` 映射 `DeployResult.reason`。

## 不做什么
- 指挥官能力效果 → M3-006–009；OP/标记 → M3-010；狙击武器 → M3-011；载具座位/呼叫 → M3-016/017；
- 兵种 UI（部署界面 loadout 选择）→ UI 任务；bot 兵种选择 → M3-024/025。

## 验收标准
1. 14 兵种名额表（GD §4.4）全部与 `/data/roles` 一致，单测断言 ≥ 14 行，差异 0。
2. 人数阈值动态：小队从 6 降到 2 人时，已选 Machine Gunner（需 4 人）存活者保持，下次 `DeployRequest` 被拒（单测）。
3. TC 认领：人数不足拒绝（重型需 3 人，2 人拒、3 人通过）；认领后 TC 离队载具仍属该车组；60 s 内无人认领回收由 M3-017 负责，本任务断言认领标记写入。
4. Recon Team 限制：第 3 名成员加入被拒；每队第 3 个 Recon Team 创建被拒（R2 数字 2）。
5. Garrison 放置者矩阵：Commander/Officer/Spotter 通过、Engineer/Rifleman 被拒（R1，6 个角色 × 1 断言）。
6. 兵种等级曲线 L2–L10 与 GD §5.2（250,750,1500,2500,3750,5250,7000,9000,11250）一致。
7. `cargo test -p sim roles_m3` 全绿；每 tick 开销可忽略（< 0.02 ms）。

## 测试要求
- 单元测试：名额表驱动测试（表来自 JSON，非手写）；阈值动态；认领；Recon 编制；等级。
- 集成/无头：headless 部署各兵种，非法组合 100% 被服务器拒绝。
- 性能基准：无。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 5 规则 0.7 ms/tick 内；本任务 ≤ 0.02 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 依赖里没有 M3-010/M3-011/M3-016，但本任务的 Spotter/Sniper/认领在逻辑上与它们耦合；本任务只提供资格与接口，具体行为在对应任务，若这些任务尚未完成请先用 trait 桩实现。
- 载具认领与"呼叫"（M3-017）的衔接点是 `vehicle.claimed_by`，认领超时回收归 M3-017。
- GD §5.1 称 Commander 可放 Garrison，R1 以裁定为准。
