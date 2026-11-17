# M4-012 sim：Skirmish 模式规则

## 元数据
- 里程碑：M4
- 工作流：sim
- 规模：M
- 前置依赖：M2-008
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §3.5; GAMEDESIGN §3.7; GAMEDESIGN §3.2; GAMEDESIGN §10.1; GAMEDESIGN §14.6; ARCHITECTURE §8.1; ARCHITECTURE §4.3.2

## 背景
Skirmish 是小规模单据点模式（GAMEDESIGN §3.5），首发适配 Carentan（M4 起）与 Foy/Omaha 的子区（MAPS 各图 Skirmish 子区表）。关键要求："与 Frontline 共用 sim 代码路径（无分叉）"：模式差异通过 `GameMode::Skirmish` 与 `/data` 规则表驱动，而不是复制规则模块。
规则要点（GAMEDESIGN §3.5，初值【待调参】）：
- 默认 16 v 16，最多 25 v 25（真人 + bot）；地图取一个子区域，边界为硬墙/出界警告。
- 单一 Strongpoint，半径 R_sp = 60 m，位于子区中央，开局中立 `p = 0`；占领速度公式沿用 §3.2。
- 计分：持有点（`p = ±100` 持续）的一方每秒 +1 分；先到 **600 分**获胜；25 min 时间到则分高者胜，相同则 `p` 符号较大者，仍同则平局。
- 精简：无 Commander（HQ 提供固定能力，无资源）；无资源系统；Garrison 上限 3/队；Outpost 允许；Airhead 无；火炮无。
- 载具：每队起始刷新 1 辆轻型装甲 + 1 辆中型坦克 + 1 辆运输车，被毁后冷却 300 s 再刷；兵种全开但每队 Sniper/Spotter 各最多 1；补给箱仍由 HQ Depot 提供；Recon 小队最多 1。
- 重生：HQ 波次 15 s、Garrison 个人 8 s、Outpost 12 s；Strongpoint 本身无重生点（GAMEDESIGN §10.5 ⑤）。
- 确定性：不得使用墙钟/HashMap（ARCHITECTURE §4.3.2）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/rules/skirmish.rs` —— 模式特有的计分/胜负/限制钩子（薄层，调用 Frontline 共享的据点占领与重生模块）。
- `crates/sim/src/rules/mod.rs` —— `GameMode::Skirmish` 分发；共享代码路径中仅通过 `ModeRules` 数据取参数。
- `data/rules/modes/skirmish.json` —— `team_cap`、`sp_radius_m`、`score_per_s`、`win_score`、`time_limit_s`、`garrison_cap`、`role_caps{sniper:1,spotter:1}`、`vehicle_spawns[]`、`vehicle_respawn_cooldown_s`、`commander:false`、`resources:false`（状态 `initial`）。
- `crates/sim/tests/skirmish.rs`。
### 要实现的行为
1. 从 `zones.json.skirmish_areas[i]` 读取子区矩形，生成单据点与出界硬边界（`out_of_bounds` 事件复用现有机制）。
2. 占领与 `p` 更新直接调用 §3.2 公式（与 Frontline 同一函数，不复制）；非活动线/锁点逻辑不适用（始终活动）。
3. 每秒结算 `hold_score`；`p = ±100` 持续才计分；到达 `win_score` 立即结束并给出 `victory_reason = score|time_limit_lead|time_limit_p|draw`。
4. 应用精简规则：关闭 Commander 席位、资源产出与 Commander 能力（返回 `Rejected(ModeDisabled)`）；Garrison/兵种/载具/Recon 小队上限由数据表校验。
5. 载具刷新与冷却复用 §14.6 的刷新点逻辑，仅数据表不同。
6. `SimConfig.mode = Skirmish` 序列化进握手元数据；`BoundaryWarning` 事件触发 UI 提示（UI 在 M4-023）。
### 接口与数据
- `pub struct ModeRules { pub mode: GameMode, pub params: &'static ModeParams }`；`ModeParams` 由 `/data/rules/modes/*.json` 生成（build.rs 嵌入，`DATA_HASH` 覆盖）。

## 不做什么
- 不做 Skirmish 子区地图数据（M4-013）。
- 不做 gateway 对该模式的编排（M4-031）。
- 不做 UI（M4-023）、bot 目标（M4-016）。
- 不新增与 Frontline 重复的占领/重生实现（若发现需要复制，必须停下并写 PLAN_ISSUES）。

## 验收标准
1. 脚本化模拟一局胜负正确：A 队持点 600 s → `victory_reason=score`，终分 A=600；B 队全程不在点内（`cargo test -p sim skirmish_score_win`）。
2. 时间到分支：25 min 到时分高者胜；分相同比较 `p` 符号；仍同则平局（3 个单测各 1）。
3. 与 Frontline 共用代码路径：`cargo test -p sim capture_shared`（同一输入序列，Skirmish 与 Frontline 的 `p(t)` 轨迹逐 tick 相同）；代码审查脚本 `tools/check-no-mode-fork.sh` 断言 `skirmish.rs` 不含占领公式实现（grep 检查，行数 ≤ 150）。
4. 限制：Commander 请求、资源节点建造被拒；第 2 个 Sniper/Spotter 部署被拒；第 4 个 Garrison 被拒（单测）。
5. 载具：被毁后 300 s（± 1 tick）再刷出（单测）。
6. 确定性：同 seed 同输入 1 万 tick `state_hash` 逐 tick 相同。

## 测试要求
- 单元测试：上面 1–5 项；边界：p 刚好到 ±100 的第 1 秒是否计分；双方同时在点内（净人数相同）不推进。
- 集成/无头测试：`headless-client` 起一局 Skirmish（Carentan 子区，灰盒），8 个脚本客户端占点，断言 1 局内产生 `victory`。
- 性能基准：`cargo bench -p sim rules_tick` 不回归（规则阶段 ≤ 0.7 ms，ARCHITECTURE §6.10）。
- 回归：确定性哈希基线；`DATA_HASH` 更新。

## 性能预算
- 规则阶段 ≤ 0.7 ms/tick（§6.10 阶段 5）；Skirmish 人数少于 Frontline，预期低于该值。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GAMEDESIGN §3.7 地图-模式表只列 Carentan 的 Skirmish，MAPS 另给 Omaha/Foy 子区；本任务代码对地图无关，具体适配由 M4-013 与 gateway 编排（M4-031）决定。
- 轻/中/运输车辆依赖 M2/M3 的载具数据；若某地图无对应刷新点则该项在数据里为空，不报错。
- 所有数值【待调参】，校准在 M4-039（三图平衡）。
