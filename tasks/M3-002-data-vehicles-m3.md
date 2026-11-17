# M3-002 数据：M3 全部载具

## 元数据
- 里程碑：M3
- 工作流：data
- 规模：M
- 依赖：M2-006
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §14.2; GAMEDESIGN §14.3; GAMEDESIGN §14.5; GAMEDESIGN §14.6; ARCHITECTURE §8.5.4

## 背景
已有载具数据：M1-009 的 `us_m4a1_75`；M2-006 的 Panzer IV H、Jeep/Kübelwagen、CCKW/Opel Blitz。本任务补齐 GD §14.3 车辆总表中**其余 10 辆**：M8 Greyhound、M5A1 Stuart、M4A3(76)W、M4A3E2 Jumbo、M3 Half-track、Sd.Kfz. 234/2 Puma、Panzer II Luchs、Panther Ausf. G、Tiger I、Sd.Kfz. 251/1。INDEX 标题"全部载具"以最终覆盖 GD §14.3 全表为准。
数据按 ARCHITECTURE §8.5.4 的 schema（`historical` 与 `gameplay` 分块、全数值带单位）；游戏最高速 = 史实路速 × 0.75（GD §14.3 说明）；全部 `initial`，复核归 M3-051。本任务只产出数据，装甲判定/物理代码在 M3-013–016。

## 具体内容
### 要创建/修改的文件
- `data/vehicles/{us_m8,us_m5a1,us_m4a3_76w,us_m4a3e2,us_m3_ht,ger_puma,ger_luchs,ger_panther_g,ger_tiger_1,ger_sdkfz251}.json` —— ID 取 GD §14.3 括号内 ID。
- `data/weapons/{us_m6_37mm,us_m1_76mm,ger_kwk39_50mm,ger_kwk38_20mm,ger_kwk42_75mm,ger_kwk36_88mm,...}.json` 与载具机枪（`vehicle_m2hb`、`vehicle_m1919`、`vehicle_mg34`、`vehicle_mg42`，若尚未存在）—— 主炮弹种按 GD §14.2 表：初速、`P100`、`drop_per_100m`、结构伤害、HE 爆炸 Dmax/R。
- `data/schemas/vehicle.schema.json` —— 视需要追加 `armor_zones[]`、`modules[]`、`ammo_rack_cookoff_prob`、`turret.traverse_dps`、`gun.elev_min/max_deg`、`spawn.{cost,cap,cooldown_s,min_crew,unlock_min}`、`seats[].can_fire`。
### 要实现的行为（数据内容）
1. 每辆车 `armor_zones[]` 含 9 区：车体前上/前下/侧/后/顶、炮塔前/侧/后/顶，字段 `{thickness_mm, slope_deg}`，值取 GD §14.3.3（如 Tiger 车体前上 100/9、炮塔前 100/8 防盾 110；Panther 车体前上 80/55；Jumbo 车体前上 101/47、炮塔前 152/30）。开放式（M8 炮塔顶、半履带顶）写 `open: true`。
2. `modules[]`：Engine 300（轻 200、重 400）、Track L/R 250（轮式 Tires 4 组 ×100）、Turret Drive 200、Main Gun 250、Ammo Rack 150，取 GD §14.5.1；殉爆概率取 §14.5.3（M5A1 50%、M8 40%、M4A3(76)W/Jumbo 20%、Panther/Tiger 35%、Puma/Luchs 40%、半履带 0%）。
3. 座位：按 GD §14.3 游戏座位（装甲车 3 座：驾驶/炮手/车长；M3 Half-track 驾驶 + 机枪手 + 乘客 ×8 = 10；251 同；Luchs 另需 10 发弹匣供弹字段）。
4. 弹药：主炮 AP/HE/WP 数量取 GD §14.3（如 Tiger AP 20/HE 14、装填 4.5 s；Panther AP 24/HE 20、3.5 s；Luchs AP 60/HE 30、10 发弹匣、450 rpm、换匣 2.5 s）。
5. 物理：`mass_kg`、`max_speed_kph = historical×0.75`、炮塔转速与俯仰（GD §14.5.4：Tiger 14°/s −8/+15；Panther 20°/s −8/+18；Puma 28°/s −10/+70 等）、原地转向速度 30°/s（中）/22°/s（重）、爬坡上限 30°（履带）/25°（轮式）。
6. 成本/上限/冷却/Armor Crew 需求/解锁时间门槛取 GD §14.6.1 表（Panther M190/U110/F160，重型合计 ≤ 1，冷却 330 s，3 人，15 min 等）；M8/Puma 成本区间在 GD 为 "90–100"，**需分别取值并在 `_note` 记录取法**（建议 M8 取 M90/U40/F70，Puma 取 M100/U50/F80，写 PLAN_ISSUES 细化）。
7. 每个文件 `_meta.tunable` 登记待调参字段；`historical._source` 写"GD §14.3 初值，待 M3-051 复核"。
### 接口与数据
- 与 ARCHITECTURE §8.5.4 示例字段对齐（`gameplay.seats[]`、`suspension`、`tracks`、`spawn_cost`）；悬挂/履带力参数无 GD 来源，给 `initial` 估值并在 PLAN_ISSUES 注明由 M3-014/015 的车辆物理调试确定。

## 不做什么
- 装甲/穿深/跳弹判定算法 → M3-013；各车物理手感调参 → M3-014/015；座位交互规则 → M3-016；呼叫逻辑 → M3-017。
- 史实参数复核 → M3-051。
- 渲染模型/socket → M3-029、M3-041。

## 验收标准
1. `tools/data-validate` 对 `/data/vehicles` 全部通过（错误 0）；10 个新 JSON 加上既有 6 个车型，GD §14.3 表所列 ID 100% 存在（脚本对比 ID 集合）。
2. 装甲表逐项自动比对：10 车 × 9 区的 `thickness_mm/slope_deg` 与 GD §14.3.3 一致（脚本差异 0）。
3. 成本、上限、冷却、解锁时间 10 车与 GD §14.6.1 一致（差异 0；M8/Puma 取值落在区间内）。
4. 每辆车模块表 HP 与 GD §14.5.1 一致；殉爆概率与 §14.5.3 一致。
5. 所有数值字段带单位后缀；`max_speed_kph` 等于 `historical.max_speed_kph × 0.75`（误差 < 0.1）。
6. 主炮弹种表（P100/drop/结构伤害/HE）与 GD §14.2 逐行一致。

## 测试要求
- 单元测试：data-validate 新增规则（装甲区数量 = 9 或标 open、座位数合法、弹药总和 > 0）各配 1 个反例。
- 集成：`cargo test -p sim data::vehicles` 加载全部车辆并检查引用完整。
- 性能基准：无。
- 回归：`DATA_HASH` 更新；`sim-wasm` 体积检查（CI `wasm-build`）。

## 性能预算
无（纯数据）。wasm 体积不得超出 ARCHITECTURE §4.3.3 的 1.2 MB / br 400 KB。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §14.3 把 M3 Half-track 与 251 的乘客可射击、车长位枪盾 M2HB/MG 写进"其它武器"，请在 `seats[].weapon` 引用，不要内联数值。
- 侧裙（Pz IV H）已属 M2-006；Panther/Tiger 无侧裙数据（GD 未给），不要补。
- 装甲 GD 表中 Panther/Tiger 侧装甲有上/下两个值：用 `armor_zones` 的 `side_upper/side_lower` 两区表示，并在 schema 中允许。
