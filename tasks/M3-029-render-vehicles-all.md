# M3-029 render：全部载具外观与交互

## 元数据
- 里程碑：M3
- 工作流：client-render
- 规模：L
- 前置依赖：M1-048, M3-014, M3-015, M3-016
- 素材需求：V-VEH-US-*; V-VEH-GE-*（灰盒可）；素材状态：placeholder
- 引用文档：GAMEDESIGN §14.1; GAMEDESIGN §14.3; GAMEDESIGN §14.5; ARCHITECTURE §13.5.3; ASSET_GAPS §3.1; ASSET_GAPS §3.2

## 背景
M1-048 已为 1 辆 Sherman 做了灰盒渲染。M3 载具全部上线（GD §14.3）：US：M8、M5A1、M4A1(75)、M4A3(76)W、M4A3E2 Jumbo、CCKW（运输/补给）、M3 半履带、Jeep（含 MG 版）；GER：Puma、Luchs、Pz IV H、Panther G、Tiger I、Opel Blitz（运输/补给）、Sd.Kfz.251、Kübelwagen（含 MG 版）。sim 侧（M3-014/015/016）提供底盘/炮塔/炮管状态与座位。本任务让它们在客户端可见、可交互（座位、炮塔旋转、开火表现、损伤视觉）。素材未到位（M3-041 才接入），一律使用 ASSET_GAPS §3.2 的灰盒：长方体车体 + 炮塔盒 + 炮管圆柱 + 左右履带/轮盒；尺寸与碰撞取自 `/data/vehicles/*.json`，不得写死在渲染代码里；材质 `PH_{需求ID}`，节点 `PH_{需求ID}__{实例名}`；美军橄榄绿、德军灰。
关键预算（ARCH §13.5.3，High）：同屏可见载具 ≤ 12（WebGL2 Medium ≤ 8、Low ≤ 5）；主视图 draw call ≤ 1 200，可见三角形 ≤ 4.0 M。

## 具体内容
### 要创建/修改的文件
- `client/src/render/vehicles/VehicleFactory.ts` —— 按 `vehicle id` 从 `/data` 生成灰盒（或 M3-041 之后加载 `.fpak` 真模型）。
- `client/src/render/vehicles/VehicleRig.ts` —— 挂点（socket）约定：`socket_turret`、`socket_gun_pivot`、`socket_muzzle`、`socket_coax`、`socket_hull_mg`、`socket_roof_mg`、`socket_seat_{seatId}`、`socket_exhaust`；挂点名写入 `/data/vehicles/*.json` 的 `sockets` 字段（若 M3-002 未含，则作为"细化"补充并在 PLAN_ISSUES 登记）。
- `client/src/render/vehicles/TrackAnimator.ts` —— 履带纹理滚动/车轮转动，速度取自快照。
- `client/src/render/vehicles/VehicleLod.ts` —— LOD 切换与距离剔除，实例化同型号。
- `client/src/render/vehicles/MuzzleFx.ts` —— 炮口焰/机枪焰（复用 M2-033 FX API）。
- `client/src/game/vehicleView.ts` —— 快照 → 插值状态（车体、炮塔角、炮管俯仰、模块 HP 档位、座位占用）。
- `client/tests/vehicles/*.spec.ts` —— vitest 与 Playwright 场景。
### 要实现的行为
1. 16 种载具（含变体）全部可渲染，座位占用位显示乘员占位胶囊（`PH_V-CHR-*`）。
2. 炮塔/炮管角度由快照驱动并 100 ms 插值（ARCH §6 插值缓冲）；炮塔旋转速度不得超过 /data 的 `turret_traverse_dps`（视觉不"瞬转"）。
3. 履带/轮式表现：履带车纹理按速度滚动，转向时左右差速；轮式车轮转动 + 前轮转角。
4. 开火表现：主炮后坐（炮管回缩 0.2–0.4 m 后复位，按 /data）、炮口焰 + 烟尘 0.3 s；机枪焰按射速。
5. 模块损伤视觉：履带受损（该侧冒灰烟）、发动机 ≤ 50% 冒黑烟、毁坏车体变焦黑色调；完整击毁见 M3-032。
6. LOD：LOD0 < 50 m，LOD1 50–150 m，LOD2 > 150 m 的盒体合并；超过可见上限时优先保留近处、有人乘坐的载具。
7. 阵营识别：Balkenkreuz/星标以简单贴花方块占位（不含禁用符号，H-024 策略）。
### 接口与数据
- 读取 `data/vehicles/{id}.json`（ARCH §8.5.4 schema；`seats[]`、`turret_traverse_dps`、`gun_elevation_deg`）。
- 事件总线：订阅 `vehicle.state`、`vehicle.fire`、`vehicle.module`（ARCH §8.4）。

## 不做什么
- 击毁燃烧、残骸、殉爆炮塔抛飞 → M3-032。
- 载具 HUD → M3-035/M3-036；载具音频 → M3-037。
- 真模型/内饰接入（GAP-V01）→ M3-041。
- 飞机、榴弹炮/反坦克炮外观 → M3-030、M3-044。

## 验收标准
1. 16 种载具各有一个 Playwright 截图用例，节点命名符合 `PH_{ID}__{inst}` 规则（脚本检查 0 违规）。
2. 同屏 12 辆载具（混合型号）的固定机位场景：主视图 draw call ≤ 1 200、可见三角形 ≤ 4.0 M（Babylon `SceneInstrumentation`，记录两个数字）。
3. 炮塔/炮管 socket 与 sim 一致：vitest 随机 200 组 `(turret_yaw, gun_pitch)`，渲染端 `socket_muzzle` 世界朝向与 sim 计算的炮口方向夹角 ≤ 0.5°。
4. 炮塔插值：以 /data 炮塔转速的 2 倍输入目标角，渲染角速度 ≤ 转速 × 1.05。
5. 12 辆载具场景 High 参考机帧时间 P95 ≤ 16.7 ms（机位见 M3-049；无参考机则标"未测"）。
6. 同型号载具使用实例化，材质变体数 ≤ 60（ARCH §13.5.3），`PH_` 材质数 ≤ 40。
7. 开发构建每辆占位载具显示需求 ID 标签；缺失素材报告列出所有 `V-VEH-*` 为 `placeholder`。

## 测试要求
- 单元（vitest）：socket 变换、插值器限速、LOD 选择、座位占用映射。
- 集成：Playwright + 固定种子的 `?vis_scene=vehicles12` 调试场景（与 M3-048 共用）。
- 性能：`SceneInstrumentation` 采集 draw call/三角形；GPU timestamp（M0-020 叠层）。
- 回归：不得破坏 M2 视觉基线；新增基线由 M3-048 入库。

## 性能预算
- ARCH §13.5.3 全表（见背景）；CPU 渲染提交 4.5 ms（§13.5.1）不得回归；12 载具场景的 draw call/三角形数字写入 STATUS。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：若一次会话内做不完，按 README §5.6 停在稳定点，追加 `M3-029b`（先做坦克，后做软车辆）。
- 灰盒的三角形远低于真模型；真模型接入后（M3-041）必须重测 draw call/三角形（GAP-V01 内饰默认简化盒）。
- 挂点命名属于"细化"，须与 M3-041 的素材处理脚本一致。
