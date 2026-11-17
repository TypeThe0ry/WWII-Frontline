# M3-019 sim：炮组操作与反坦克炮

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 依赖：M3-018, M2-011
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §15.3; GAMEDESIGN §15.7; GAMEDESIGN §14.2; GAMEDESIGN §14.4; GAMEDESIGN §12.4; BRIEF R7

## 背景
M3-018 实现了榴弹炮的核心（弹道/散布/库存/Fire Mission）。M2-011 已实现 AT 炮阵地的**建造**（Anti-Tank 兵种用 200 Supplies、20 s，R7）与伤害基础。本任务补齐两件事：
1. **炮组操作层**：多操作员的角色分工、炮位权限（谁能坐、同时 1 炮手 + 1 装填手）、座位切换、单人操作降速；对榴弹炮与 AT 炮统一。
2. **AT 炮完整规则**（GD §15.7）：US 57 mm M1 与 GER 7.5 cm Pak 40：HP 600；防盾 7 mm/0°，正面 ±60° 有效，阻挡枪弹，对 HEAT/AP 视同装甲板；弹药 AP 18 / HE 8（总 26）；装填 3.0 s（2 人）/4.2 s（1 人）（Pak 40：3.2/4.5 s）；穿深 `P100 = 92`、drop 3%（US）与 96/2.2%（GER）；出厂水平射界 ±40°，Engineer 可在 3 s 内重新定向（转动尾架）；俯仰 −5°/+15°（US）、−5°/+25°（GER）；全队 ≤ 3 门、每 Sector ≤ 1；补充弹药：Support/Engineer 靠近 Ammo Crate，或 Supplies 直接补（100 Supplies→8 AP）；AP/HE 切换需 1.0 s 重新装填；无机枪；炮口火光与烟尘发射后 3 s 内可见，不上敌方地图（除非被 Spotter 标记）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/artillery/crew.rs` —— `CrewPosts{gunner, loader}`；`can_occupy(player, gun)` 权限；`swap_post`；离岗/死亡处理；单人/双人循环时间切换。
- `crates/sim/src/artillery/atgun.rs`（M2-011 已有骨架）—— 回转（射界 ±40° 限位、Engineer 3 s 转架）、俯仰限位、装填、AP/HE 切换、弹药库存、发射与弹道（复用 M3-013 的 `ShellDef`/`resolve_shell_hit`）。
- `crates/sim/src/artillery/shield.rs` —— 防盾几何：正面 ±60° 扇区内命中视作 7 mm/0° 装甲板（调用装甲判定）；扇区外直接命中炮手/炮架；枪弹被盾阻挡但可打穿炮手侧（沿射线判定先盾后人）。
- `crates/sim/src/artillery/exposure.rs`（M3-018 已有）—— AT 炮 3 s 火光与烟尘事件。
- `crates/sim/tests/artillery_crew_atgun.rs`。
### 要实现的行为
1. **权限矩阵**：榴弹炮——任一本方步兵（非载具乘员）可坐炮手/装填手，需靠近座位 ≤ 3 m；AT 炮——任一本方步兵可操作（GD 写"AT/Engineer 操作"，**建造**只限 Anti-Tank；本任务取"AT/Engineer 优先、其它兵种也可操作但装填时间 ×1.0"，写 PLAN_ISSUES 待确认）；敌方不可占用；座位已占用拒绝。
2. **角色分工**：炮手瞄准/开火，装填手选弹种/装药/装填；榴弹炮两座都在才用 2 人循环；缺装填手时炮手自装填（循环 10/11.5 s）；缺炮手时无法开火。
3. **AT 炮战斗**：发射 AP 走 M3-013 的穿深判定（P100 92/96，距离衰减 3%/2.2% 每 100 m）；HE 爆炸 `Dmax 140/R 4`（US）、`250/R 7`（Pak 40）；炮口初速 823/750 m/s（来自 `/data`）。
4. **回转**：出厂方向 ±40° 内转动；超出需 Engineer 重新定向 3 s（读条，被打断取消）；数据中的方向写入 `facing_deg`。
5. **弹药补充**：Ammo Crate 4 m 内补 AP/HE 至上限；Supplies 直补 100→8 AP；全队 AT 炮总数 ≤ 3、每 Sector ≤ 1（建造规则 M2-011，此处仅断言）。
6. **摧毁**：HP 600 归零或炸药包（≥ 600，即一次性）；防盾对枪弹有效但不阻挡 HEAT/AP（视作装甲板）。
7. 炮手/装填手被杀则炮停用；炮本身保留，补人后恢复。
### 接口与数据
- `GunCrewQuery`（供 bots/UI）：`can_operate(player,gun)`、`recommended_post(player,gun)`；数据字段取 `/data/structures`（M3-003）。

## 不做什么
- 榴弹炮弹道/Fire Mission → M3-018；AT 炮建造规则 → M2-011（已有）；炮兵/车长 bot → M3-025；炮位 UI → M3-035；
- AT 炮被击毁特效/渲染 → M3-031/M3-044；炮弹声音 → M3-038。

## 验收标准
1. 权限矩阵单测：{榴弹炮, AT 炮} × {炮手位, 装填手位} × {友方/敌方, 步兵/载具乘员, 距离 2.9/3.1 m} 共 ≥ 24 个断言通过。
2. AT 炮装填：US 2 人 3.0 s、1 人 4.2 s；Pak 40 3.2/4.5 s（±1 tick）；AP/HE 切换重新装填 1.0 s（±1 tick）。
3. 回转：±40° 内可转、41° 被拒；Engineer 重新定向 3 s ±1 tick；俯仰 US −5/+15°、GER −5/+25°（越界钳制）。
4. AT 炮对装甲判定与 GD §14.4 一致：对 Panther 车体侧（50 mm/30°）300 m 击穿，对 Tiger 前上不击穿（沿用 M3-013 手算表中的 6 组案例）。
5. 防盾：正面 ±60° 内枪弹命中盾不伤炮手（100 发采样，炮手伤害 0）；±60° 外命中炮手；AP 对盾按 7 mm 判定击穿并继续（单测）。
6. 弹药：AP 18/HE 8 初始；补 100 Supplies→8 AP；上限不可突破；0 库存开火被拒。
7. 单门 AT 炮活动开销 ≤ 0.02 ms/tick（`criterion`）。

## 测试要求
- 单元测试：权限矩阵、装填/切换时序、回转限位、防盾几何、弹药。
- 集成/无头：headless 两玩家操作 US 57 mm 对 100–500 m 靶坦克（灰盒 Pz IV）10 发，击穿/未击穿分布与 M3-013 手算期望一致（容差 ±2 发）。
- 性能基准：`crates/sim/benches/atgun.rs`。
- 回归：确定性哈希更新。

## 性能预算
ARCHITECTURE §6.10 阶段 4（武器与弹道 2.5 ms）与阶段 5（规则 0.7 ms）：每门活动 AT 炮 ≤ 0.02 ms；全队 ≤ 3 门。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §15.3 只说"炮手/装填手座位"，谁能就座没有明文，权限矩阵取最宽松解释并登记 PLAN_ISSUES，Fable 可收紧。
- 与 M2-011 重叠部分（建造、基础伤害）不重复实现，先阅读其产出再扩展。
- 手感与暴露时长待 M3-052（A-01–A-13）校准。
