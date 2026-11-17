# M3-014 sim：轻型/侦察载具

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 依赖：M3-013
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §14.3; GAMEDESIGN §14.1; GAMEDESIGN §14.5.4; GAMEDESIGN §14.2; ARCHITECTURE §5.3; ARCHITECTURE §6.10

## 背景
M1-022 已建 raycast 悬挂 + 履带物理（Sherman）；M2-015 加入了轮式的 Jeep/卡车。本任务把四辆轻型/侦察载具接入：US **M8 Greyhound**（轮式装甲车，37 mm M6，车顶 .50 M2HB 300 发 + 同轴 M1919 500 发，史实路速 89 km/h，7.8 t）、**M5A1 Stuart**（履带，37 mm，同轴+车体+车顶 M1919 各 500 发，58 km/h，15.2 t）、GER **Sd.Kfz. 234/2 Puma**（轮式，5 cm KwK 39/1 L/60，同轴 MG 42 1000 发，85 km/h，11.7 t，炮塔仰角 +70°）、**Panzer II Luchs**（履带，2 cm KwK 38：AP 60/HE 30，10 发弹匣，循环 450 rpm、点射 3–5 发、换匣 2.5 s，同轴 MG 34 1000 发，60 km/h，13 t）。
游戏最高速 = 史实路速 × 0.75（GD §14.3 说明）：M8 66.75、M5A1 43.5、Puma 63.75、Luchs 45 km/h。座位 3 个（驾驶/炮手/车长；Puma 后驾驶位由 AI 辅助）。炮塔转速 M8/M5A1 24°/s、Puma/Luchs 28°/s；俯仰 M8 −10/+20、M5A1 −10/+20、Puma −10/+70、Luchs −9/+20（GD §14.5.4，【待调参】）。数据已由 M3-002 落地，本任务写 sim 行为与调参。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/vehicle/wheeled.rs`（M2-015 已有）—— 装甲车轮式模型：4 轮/组 Tires（每组 100 HP，毁一组最高速 ×0.75、下限 0.3，GD §14.5.1）、高速悬挂参数、转向角与速度关系。
- `crates/sim/src/vehicle/tracked.rs`（M1-022 已有）—— Stuart/Luchs 履带参数（质量/推力/摩擦从 `/data` 读）。
- `crates/sim/src/vehicle/autocannon.rs` —— 自动炮连发：Luchs 450 rpm、10 发弹匣、点射 3–5 发（输入按住时连发至弹匣空/松开）、换匣 2.5 s；弹种 AP/HE 切换（切换需 1.0 s 重新装填，GD §14.7）。
- `crates/sim/src/vehicle/mg_mounts.rs` —— 多机枪位：同轴（炮手）、车体（驾驶员，固定向前）、车顶（车长，M8 M2HB、Stuart M1919）；各挂点独立弹药（GD §14.3.1）。
- `crates/sim/src/vehicle/crew_effects.rs` —— 车长缺位装填 ×1.3；舱盖开/关视野标志位。
- `crates/sim/tests/vehicles_light.rs`、`crates/sim/benches/vehicle_step.rs`。
### 要实现的行为
1. 驾驶：W/S/A/D 与 `Shift`；轮式车用 Ackermann 近似，履带车差速；最高速限幅为 `data × 0.75`；爬坡上限 30°（履带）/25°（轮式）。
2. 主炮：M8/M5A1 37 mm 装填 2.0 s，Puma 2.2 s，Luchs 见自动炮；弹药 M8 AP 30/HE 20，Stuart AP 24/HE 12，Puma AP 20/HE 14。
3. 模块损伤效果：Engine（轻型 200 HP，≤ 50% 速度×0.6），Track/Tires，Turret Drive（旋转 ×0.5），Main Gun（装填 ×1.4 散布 ×1.5）。
4. 载具碰撞：与建筑/树木/障碍，车速 > 25 km/h 时 `hull_hp −= 0.5·v²`（v 为 m/s，GD §14.9）。
5. 燃料维持：轻型装甲 1 Fuel/min（M2-009 资源池扣减）。
6. 不依赖渲染；炮塔/炮管 socket 逻辑位置由 `/data` 的 `turret_pivot`/`muzzle_offset` 提供（供 M3-029 对齐）。
### 接口与数据
- `VehicleController::step(&mut self, input:&VehicleInput, dt)`；输出 `VehicleExtra`（M3-005：座位占用、active_weapon、module_flags）。

## 不做什么
- 中/重型坦克 → M3-015；运输车座位 → M3-016；装甲判定 → M3-013（本任务调用其接口）；
- AI 驾驶 → M3-023；渲染/声音 → M3-029/M3-037。

## 验收标准
1. 各车最高速与 `/data`（史实 × 0.75）误差 < 3%：M8 66.75、M5A1 43.5、Puma 63.75、Luchs 45 km/h（`cargo test -p sim vehicles_light::top_speed`，平地 60 s 测量）。
2. Luchs 连发速率 450 rpm，误差 ≤ 2%（单测 10 s 内发射数）；弹匣 10 发后换匣耗时 2.5 s ±1 tick。
3. 炮塔转速 M8/M5A1 24°/s、Puma/Luchs 28°/s，误差 < 3%；Puma 仰角上限 +70°（越界钳制）。
4. 爬坡：履带车在 30° 坡可爬、31° 不可；轮式 25°/26°（单测）。
5. 轮式 Tires 毁 1 组最高速 ×0.75、毁 3 组 ×0.3（下限钳制）。
6. 车长缺位装填时间 ×1.3（误差 ≤ 1 tick）。
7. 单车 `step` 中位数 ≤ 40 µs（`criterion`，参考机）；40 辆 ≤ 2.0 ms/tick（ARCHITECTURE §6.10 阶段 3）。

## 测试要求
- 单元测试：速度限幅、坡度、自动炮状态机、MG 弹药独立、模块损伤联动。
- 集成/无头：`headless-client` 驾驶 M8 沿预设路线 500 m，位置与服务器权威差（对账）≤ 0.05 m 回滚阈值触发率 < 1%（R5 容差对账，仅信息性）。
- 性能基准：`crates/sim/benches/vehicle_step.rs` 基线入库。
- 回归：确定性哈希更新（含载具）。

## 性能预算
ARCHITECTURE §6.10 阶段 3：载具 ≤ 40 辆合计 2.0 ms/tick（每辆 4–8 悬挂射线 + 履带力）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 悬挂/履带力数值无史料来源，由本任务在 `/data` 以 `initial` 调出"手感可驾驶"，调参归 M3-047；手感对标 V-01–V-27 在 M3-052。
- Puma 后驾驶位 AI 辅助（GD §14.3.2）在 sim 里不建模，只在 bot（M3-023）侧忽略。
- GD §14.5.4 的物理说明提及"自写射线悬挂 + 履带模型（见 ARCHITECTURE）"，轮式沿用 M2-015 模型，不另起炉灶。
