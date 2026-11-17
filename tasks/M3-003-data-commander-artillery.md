# M3-003 数据：指挥官能力与火炮

## 元数据
- 里程碑：M3
- 工作流：data
- 规模：S
- 依赖：M3-001
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §13.2; GAMEDESIGN §15.1; GAMEDESIGN §15.5; GAMEDESIGN §15.7; GAMEDESIGN §10.4; GAMEDESIGN §10.7

## 背景
M3 要实现 8 项指挥官能力（Supply Drop、Ammo Drop、Bombing Run、Strafing Run、Precision Strike、Recon Flight、Rally、Airhead）与载具呼叫、榴弹炮预设阵地（R7）、反坦克炮。本任务把 GD 的数值表落成 `/data` JSON（GD §0.2 规定 `/data/abilities/`、`/data/structures/` 为数据目录），供 M3-006–M3-019 的 sim 代码读取。全部 `initial`。
本任务**不含**载具成本（已在 M3-002 / GD §14.6.1），也不含 Garrison/Outpost 数据（M1/M2 已有）。

## 具体内容
### 要创建/修改的文件
- `data/abilities/{supply_drop,ammo_drop,bombing_run,strafing_run,precision_strike,recon_flight,rally,airhead}.json` —— 字段：`cost{manpower,munitions,fuel}`、`cooldown_s`、`delay_s`、`effect`、`duration_s`、`placement`（点/条带/小队）、`warn_enemy_s`（15 s 或 0）、`cancel_refund`（0.5）、`air_lock_s`（10）。
- `data/abilities/_global.json` —— 通则：全局空中支援锁 10 s、同时在空飞机 ≤ 3、友军 25 m 内"危险近距"阈值、取消返还 50%、敌方警告环半径 200 m、`ff_mult_explosion 0.7`（GD §13.1）。
- `data/structures/{us_m2a1_105,ger_lefh18}.json` —— 榴弹炮：阵地 HP 1500、炮手+装填手、全队 2 门、三档装药 `rmax_m = [800,1800,3200]`、`v_mps = sqrt(rmax·g)`（88.6/132.9/177.2）、方位转速 4°/s（微调 0.1°/s）、仰角 5°–85°、转速 3°/s、循环 6.0/10 s（M2A1）、7.0/11.5 s（leFH）、初始库存 24 HE + 8 烟雾、上限 48 HE + 16 烟雾、HE `Dmax 600/R 14`、对载具直击 900、对建筑 1500、烟雾 R 18 m/60 s、散布 `σ_range=0.012R σ_lat=0.006R` 截断 3σ、校射 ×0.7（40 m/120 s/100 m）。
- `data/structures/{us_at57_m1,ger_pak40}.json` —— 反坦克炮：HP 600、防盾 7 mm/0°、AP 18 + HE 8、装填 3.0/4.2 s 与 3.2/4.5 s、射界 ±40°、俯仰 −5/+15 与 −5/+25、`P100` 92/96、drop 3%/2.2%、全队 3 门/每 Sector ≤ 1、200 Supplies/20 s（R7，Anti-Tank 建造）。
- `data/structures/_artillery_exposure.json` —— 炮口闪光 6 s 误差 40 m；AT 炮 3 s；尖啸预警 4 s/60 m（GD §15.6）。
- `data/schemas/{ability,structure_artillery}.schema.json` —— 新 schema。
### 要实现的行为
1. 能力表逐项取 GD §13.2：例如 Bombing Run = 350 Munitions + 120 Fuel、冷却 600 s、延迟 45 s、条带 400×70 m、3 架 × 6 弹、6 s；Strafing = 150/60、360 s、25 s、300×14 m、2 机、3 s；Precision = 200/80、420 s、22 s、R 22 m；Recon = Fuel 60、240 s、18 s、300×120 m、飞越 6 s + 揭示 12 s；Rally = Manpower 120、300 s、3 s、R 40 m、45 s；Airhead = Manpower 200、360 s、20 s、R 25 m、120 s、容量 16、个人重生 5 s、全队 1 个（GD §10.4）；Supply Drop = M100 + F20、240 s、15 s、4 箱、散落 R 15 m、箱 300 s；Ammo Drop = U120、180 s、15 s、2 箱（1200 弹药点）、R 10 m、箱 120 s。
2. 炸弹/扫射伤害参数（GD §13.4–13.6）：步兵 `Dmax 400/R 14`（Bombing）、载具直击 ≤ 4 m 600 HP、近失 200、建筑 1500；Strafing 落点半径 1.8 m、步兵 90、载具 25、建筑 30、压制 R 20 m `S+=12`、间隔 0.025 s；Precision `Dmax 800/R 22`、载具直击 ≤ 5 m 1200、建筑 3000。
3. Rally buff：冲刺 +10%、体力消耗 ×0.7、压制累积 ×0.7、HP 恢复阈值 60→80（GD §13.8）。
4. 射表预计算：脚本 `tools/data-validate/artillery_table.*` 按 GD §15.4 公式生成低/高角射表，与 GD 表（14 行）逐行对比（容差 ±0.1°、TOF ±0.1 s）。
### 接口与数据
- 能力 id 与 `protocol::AbilityId` 枚举的取值一一对应（M3-005 定义枚举，本任务先固定 id 字符串与顺序：见上方 8 项，载具呼叫 id=8）。
- 所有字段带单位后缀，`_meta.tunable` 登记，状态 `initial`，`calibration_ref` 指向 REFERENCE_MEASUREMENTS commander/artillery 节。

## 不做什么
- 能力执行逻辑 → M3-006–M3-009；炮兵逻辑 → M3-018/019。
- 载具呼叫成本 → 已在 M3-002（GD §14.6）；呼叫逻辑 → M3-017。
- 预设阵地坐标 → M3-004。

## 验收标准
1. `tools/data-validate` 通过，8 个能力 + 4 个火炮文件加载成功；数值与 GD §13.2、§15.1、§15.7 表逐字段一致（脚本差异 0）。
2. 射表脚本生成的 14 行（GD §15.4）与文档误差 θ ≤ 0.1°、TOF ≤ 0.1 s；最小射程 139/313/556 m 吻合（±1 m）。
3. 三档装药 `v_mps` 与 `sqrt(rmax×9.81)` 误差 < 0.1 m/s。
4. `_global.json` 含 10 s 全局空中锁与 3 架上限；`cancel_refund = 0.5`。
5. R7 断言：`data/roles/engineer.json` 的 `abilities`、建造清单中无榴弹炮；AT 炮建造者仅 `anti_tank`。
6. `DATA_HASH` 同内容稳定。

## 测试要求
- 单元测试：schema 反例（缺 cooldown、单位错误）必报错；射表函数 3 组手算样例。
- 集成：`cargo test -p sim data::abilities`、`data::structures` 加载并断言数量（8 + 4）。
- 性能基准：无。
- 回归：`DATA_HASH` 基线更新。

## 性能预算
无（纯数据）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §0.2 列了 `abilities/`、`structures/` 目录但 ARCHITECTURE §8.5 无对应 schema，属"细化"，请在 PLAN_ISSUES 登记 schema 字段。
- Airhead 同时出现在 GD §10.4 与 §13.2，数值一致（资源 200、冷却 360 s、延迟 20 s），以 JSON 单一定义，sim 代码不得再硬编码。
- 延迟/冷却的"体验长度"待 M3-052 对标测量校准（REFERENCE_MEASUREMENTS commander 节），改值走 M3-047。
