# M3-030 render：空袭飞机与落弹表现

## 元数据
- 里程碑：M3
- 工作流：client-render
- 规模：M
- 前置依赖：M2-033, M3-008
- 素材需求：V-VEH-AIR-*（灰盒可）；素材状态：placeholder
- 引用文档：GAMEDESIGN §13.1; GAMEDESIGN §13.3; GAMEDESIGN §13.4; GAMEDESIGN §13.5; GAMEDESIGN §13.6; GAMEDESIGN §13.7; ASSET_GAPS §3.2; ARCHITECTURE §6.2

## 背景
M3-008 在 sim 中实现了飞机实体（无碰撞）、Bombing/Strafing/Precision 的条带落弹与延迟，M3-007/009 提供 Supply/Ammo Drop 与 Recon Flight。本任务把它们画出来，且**飞机位置必须是服务器时间的函数**，保证与服务器的 ETA 一致。
关键数字（GD §13.2/13.4–13.7）：Bombing 3 架 × 6 弹，6 s 内每 0.33 s 一枚，条带 400 m × 70 m，点击后 45 s 到达；Strafing 2 机编队 3 s、每 0.025 s 一落点（共约 120 个）、条带 300 m × 14 m、25 s；Precision 单弹 R 22 m、22 s；Recon 条带 300 m × 120 m、飞越 6 s；Supply/Ammo Drop 运输机空投带降落伞箱（15 s）；同时在空飞机 ≤ 3（GD §13.1）；敌方警告：抵达前 15 s（防空警报在 M3-038，地图 200 m 模糊环在 M3-033）。
灰盒：飞机 = 十字形盒（机身 + 机翼），ASSET_GAPS §3.2；节点 `PH_V-VEH-AIR-00x__{inst}`。

## 具体内容
### 要创建/修改的文件
- `client/src/render/air/AircraftManager.ts` —— 生成/回收飞机实例（上限 3 并发 + 对象池）。
- `client/src/render/air/FlightPath.ts` —— 纯函数 `positionAt(t_server_ms, strip, speed, altitude)`；所有飞机位置由它计算，不用本地累加。
- `client/src/render/air/StripMarker.ts` —— 条带预警：仅**己方**在世界/地形上看到条带轮廓线与落弹范围（敌方只在地图上见 200 m 模糊环，不暴露方向，GD §13.1）。
- `client/src/render/air/BombDrop.ts` —— 炸弹下落（小圆柱）与 `AbilityExecuted` 弹着事件对齐，落地触发 M2-033 爆炸 FX。
- `client/src/render/air/StrafeTracer.ts` —— 扫射曳光与地面点状溅射（每落点一个 0.2 s 粒子）。
- `client/src/render/air/ParachuteCrate.ts` —— 空投箱 + 降落伞 + 落地彩色烟（Supply/Ammo 不同色，20 s，GD §13.3）。
- `client/tests/air/*.spec.ts`
### 要实现的行为
1. 订阅 M3-005 的 `AbilityExecuted`/弹着事件；根据事件内 `eta_tick` 与条带端点计算飞机路径，飞行速度使"到达条带中心"恰为 `eta`，条带两端前后各延伸 600 m 做进场/离场。
2. 时钟使用 `estimated_server_tick`（ARCH §6.2）；时钟校正时飞机位置平滑修正（≤ 0.3 s 内收敛），不跳变。
3. Bombing：3 架依次间隔；每枚炸弹在弹着 tick 到达地面（下落时间 = 高度/速度，高度取 /data，预先发射）。
4. Strafing：2 机编队沿条带；落点依服务器事件而非客户端随机。
5. Precision：俯冲机动画（简单抛物线俯冲），22 m 圆预览仅己方可见。
6. Recon：飞越 6 s，不投弹；揭示图标在 UI/地图（M3-033/034 之外的团队通道），此处只画飞机。
7. 空投：机在高空直线飞过，投放点随机散落 15 m/10 m（来自事件，不自行随机）。
### 接口与数据
- 输入事件字段（以 M3-005 为准）：`ability, team, strip{p0,p1,width}, eta_tick, impacts[{tick,pos}]`。
- 粒子预算：活跃 ≤ 1 600（High，ARCH §13.5.3），空袭场景峰值需在预算内。

## 不做什么
- 爆炸本体特效/弹坑贴花（沿用 M2-033；炮击增强见 M3-031）。
- 空袭/飞机音频与防空警报（M3-038）；指挥官点选界面（M3-033）。
- 真飞机模型接入（M3-044，V-VEH-AIR-001–004）。

## 验收标准
1. 飞机到达条带中心的客户端计算时刻与服务器事件 `eta_tick` 的差 ≤ 200 ms：vitest 在模拟 ±80 ms 网络时钟偏移下 100 次随机条带测试，最大误差 ≤ 200 ms。
2. Bombing 全程（含 18 枚弹着）场景峰值活跃粒子 ≤ 1 600，空袭场景 GPU "透明/粒子" pass ≤ 0.7 ms（ARCH §13.5.2，High 参考机；无参考机标"未测"）。
3. 并发上限：同时触发 4 个空袭，只有 3 架同时渲染且无报错（对应 sim 的 ≤ 3 限制，测试用例）。
4. 敌方视角下世界中无条带轮廓与方向线索（Playwright 断言：敌队客户端场景中 `StripMarker` 节点数 = 0）。
5. 飞机使用对象池：连续 50 次空袭后 JS 堆增长 < 5 MB，Babylon mesh 数不增长（`scene.meshes.length` 稳定）。
6. 灰盒命名合规，开发构建显示需求 ID 标签。

## 测试要求
- 单元：`FlightPath.positionAt` 边界（eta 前/后、条带端点）、时钟修正平滑。
- 集成：Playwright 固定机位回放"Bombing + Strafing + Drop"脚本事件序列（与 M3-048 共用 `?vis_scene=airstrike`）。
- 性能：粒子计数与 GPU 耗时采样，写入 `reports/perf/m3-air.json`。
- 回归：M2 爆炸 FX 基线不变。

## 性能预算
- ARCH §13.5.3 粒子 ≤ 1 600、§13.5.2 透明/粒子 ≤ 0.7 ms；不得使 CPU 渲染提交（4.5 ms）回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 空投机"不可被击落"（GD §13.3，v1 无防空），飞机无需命中体。
- 美军飞机 P-47 类、德军 Ju 87 类仅在 Precision 使用（GD §13.6）；灰盒不区分机型，仅阵营着色。
- M3-005 的事件字段若与此处不同，以 M3-005 为准并更新本文件备注。
