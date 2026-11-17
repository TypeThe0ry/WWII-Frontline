# M3-018 sim：榴弹炮阵地

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：L
- 前置依赖：M3-003, M3-004, M3-006, M2-019
- 素材需求：无（灰盒即可；火炮用盒体+socket 占位，接入见 M3-044）；素材状态：n.a.
- 引用文档：GAMEDESIGN §15.1; GAMEDESIGN §15.2; GAMEDESIGN §15.3; GAMEDESIGN §15.4; GAMEDESIGN §15.5; GAMEDESIGN §15.6; GAMEDESIGN §16.3; BRIEF R7

## 背景
R7：榴弹炮是 HQ 附近**地图预设阵地**（`artillery.json` 的 `sites`，每队 2–3 个，开局即存在，无需建造），由玩家操作；Engineer 不建造榴弹炮。阵地数据来自 M3-004，射表/弹药/散布数据来自 M3-003（`/data/structures/{us_m2a1_105,ger_lefh18}.json`）。
游戏压缩射程：三档装药 `Rmax = 800/1800/3200 m`，`v = √(Rmax·g)`（88.6/132.9/177.2 m/s），**不模拟空气阻力**，抛物线。低角 `θ = ½·asin(R/Rmax_c)`，需 `R ≥ R_min = Rmax·sin10°`（139/313/556 m）；高角 `θ_high = 90° − θ`，上限 85°；飞行时间 `t = 2v·sinθ/g`；Δh≠0 用迭代求解（服务器给推荐仰角）。服务器按**真实弹道积分**结算，不做吸附。
座位：炮手 ×1 + 装填手 ×1；循环 M2A1 6.0 s（2 人）/10 s（1 人），leFH 18 7.0 s/11.5 s，装填 3.5 s（装弹→装药→闭锁）；方位转速 4°/s（微调 0.1°/s），仰角 5°–85° 转速 3°/s。阵地 HP 1500；炸药包 2500 一次摧毁；Overrun：敌 15 m 内持续 20 s 且己方 15 m 内无人 → 毁。库存：初始 24 HE + 8 烟雾，上限 48 HE + 16 烟雾；Shell Stock：100 Supplies → 8 HE 或 4 烟雾（炮位 6 m 内操作）。
伤害（GD §15.5）：HE 步兵 `Dmax 600 / R 14 m`（致命半径 ≈ 6 m）、载具直击（≤ 3 m）900 HP（HE 穿深 21 mm 对顶部有效）、建筑 1500 HP；压制近失半径 25 m；烟雾云 R 18 m 持续 60 s。散布：`σ_range = 0.012R`、`σ_lateral = 0.006R`，截断 3σ；Registration（校射）：落点距目标 ≤ 40 m 且被友方 Spotter/Officer 看到，随后 120 s、目标 100 m 内同装药射击 σ×0.7。暴露（§15.6）：开炮 6 s 内在敌方地图显示"炮口闪光"（误差 40 m）；落地前 4 s 在 60 m 内玩家处预警；友军爆炸误伤 ×0.7。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/artillery/site.rs` —— `ArtillerySite` 实体（来自 `artillery.json`）、HP、Overrun、`seats[2]`、状态机 `Idle→Loading→Ready→Firing→Cycle`。
- `crates/sim/src/artillery/ballistics.rs` —— `solve_elevation(R, Δh, charge, high_angle) -> Option<f32>`（迭代求解）、`flight_time`、`integrate_shell`（4 子步/tick 的抛物线，无阻力）；射表 helper 供 UI。
- `crates/sim/src/artillery/fire_mission.rs` —— Fire Mission 标记（由 Commander 指派给指定炮位，或 Officer 的 Fire Request 被 Commander 采纳，GD §16.3）；观察者修正：`向左/右/增加/减少 25/50/100 m`（相对原炮目线）；计算目标数据（方位角、距离、推荐装药与仰角）。
- `crates/sim/src/artillery/ammo.rs` —— 库存与 Shell Stock 补充、HE/烟雾选择。
- `crates/sim/src/artillery/scatter.rs` —— 高斯散布（`Rng::for_(tick, gun, purpose)`），3σ 截断，Registration 窗口。
- `crates/sim/src/artillery/exposure.rs` —— 炮口闪光/尖啸预警事件（对接 M3-010 暴露表）。
- `crates/sim/tests/artillery_howitzer.rs`。
### 要实现的行为
1. 操作流程（GD §15.3）：就座→接收 Fire Mission→炮手设方位/仰角、装填手选装药/弹种并装填→开火；无 Fire Mission 时可手动操作。
2. 服务器权威：转速上限、仰角范围 5°–85°、`R < R_min` 或 `R > Rmax_c` 拒绝（返回原因，供 UI）。
3. 弹道：炮弹为服务器弹体，在 TOF 后落地，触发爆炸结算（M2-014 爆炸接口）；对载具/建筑/步兵伤害按 GD §15.5；烟雾落地展开云团（与 M1-015 烟雾接口对接）。
4. 暴露与反制：开炮发炮口闪光条目（6 s，±40 m）；落地前 4 s 在目标 60 m 内玩家发预警事件（高角射击更早，因 TOF 更长）。
5. 炮手/装填手被杀则炮停用；单人操作循环变长；炮被毁后重置规则【待调参】（GD §15.2）按数据 `respawn_s`（默认原地复原冷却 300 s，写 PLAN_ISSUES 细化）。
6. 数量上限：全队 2 门（数据）；阵地数 ≥ 此限时多余阵地标 `reserve`。
### 接口与数据
- 事件：`ArtyShot{gun, azimuth, elevation, charge}`（C2.events）、`ArtyImpact{pos, eta}`（M3-005）；`WorldView::artillery_sites(team)`。
- `FireMissionTarget{azimuth_deg, range_m, charge, elev_low, elev_high}` 供 `sim-wasm` 暴露（UI 用，M3-035）。

## 不做什么
- 反坦克炮与多操作员权限细化 → M3-019；炮兵 bot → M3-025；炮位 UI → M3-035；落点渲染/声音 → M3-031/M3-038。
- 不做空气阻力、风、地形弹跳；不做场外叙事火力的玩法（仅视觉/音效）。

## 验收标准
1. 射表落点：对 GD §15.4 表中的 14 组（装药 × 距离）取推荐仰角发射，**无散布**时落点误差 ≤ 0.5 m（`cargo test -p sim artillery::table`）。
2. 散布统计：每组（装药 2，R=1 500 m）发射 1 000 发，落点纵向标准差与 `σ_range = 18 m`、横向与 `σ_lateral = 9 m` 误差 ≤ 5%，且全部在 3σ 截断内。
3. 校射生效：距目标 ≤ 40 m 且被 Spotter 看到后 120 s 内，100 m 内同装药 σ ×0.7（统计误差 ≤ 5%）；121 s 后恢复。
4. 弹药库存：初始 24 HE + 8 烟雾；消耗/补充 100 Supplies→8 HE 或 4 烟雾；上限 48/16 不可突破（单测）；0 库存开火被拒。
5. 循环时间：M2A1 2 人 6.0 s、1 人 10 s；leFH 7.0/11.5 s（±1 tick）。
6. 限制：`R < R_min`、仰角 4°/86° 被拒；转速超限被钳制（方位 4°/s、仰角 3°/s，误差 < 3%）。
7. 暴露：开炮后 6 s 内敌方暴露条目存在、位置误差 ≤ 40 m；7 s 后消失。
8. Overrun 20 s、炸药包 2500 一击摧毁、阵地 HP 1 500（单测）。
9. 单次装填/开火/弹道推进总开销 ≤ 0.05 ms/tick（活动期，`criterion`）。

## 测试要求
- 单元测试：解仰角（含 Δh 迭代，Δh = ±30 m 样例）、TOF、散布、校射窗口、库存、状态机、座位缺失。
- 集成/无头：`headless-client` 作炮手/装填手/观察者完成"标记→修正→命中"全流程，3 发内命中目标 R<6 m（有校射时）。
- 性能基准：`crates/sim/benches/artillery.rs`。
- 回归：确定性哈希更新（含弹体）。

## 性能预算
ARCHITECTURE §6.10 阶段 4（武器与弹道 2.5 ms，含活动弹体）与阶段 5（规则 0.7 ms）；火炮活动期合计 ≤ 0.1 ms/tick。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L；依赖已含 M2-019（Fire Mission/Fire Request 标记权限，GD §16.3）。
- INDEX 验收写"射表落点误差与 /data 散布一致"：本任务把"无散布时 ≤ 0.5 m、有散布时 σ 与数据一致"两项都测。
- 炮击预警的 4 s 与 TOF 的关系：TOF<4 s 时预警在弹出膛时立即发出。
- 数值【待调参 / 待测 → REFERENCE_MEASUREMENTS §artillery（A-01–A-13，M3-052）】。
