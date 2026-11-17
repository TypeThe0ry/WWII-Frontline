# M2-033 render：烟雾、火焰、弹坑与血迹

## 元数据
- 里程碑：M2
- 工作流：client-render
- 规模：M
- 前置依赖：M1-047
- 素材需求：V-FX-*（含 V-FX-005 血迹/飞溅/拖痕贴花）；素材状态：placeholder（灰盒即可，真素材由 M2-047 接入）
- 引用文档：ARCHITECTURE §13.1; ARCHITECTURE §13.5.3; ARCHITECTURE §13.5.2; ASSET_GAPS §3.2; GAMEDESIGN §6.9.2; GAMEDESIGN §7.2

## 背景
M1-047 已建立粒子系统（枪口焰/曳光/烟/爆炸/尘土）、弹孔贴花、粒子预算与池。M2 需要完整的战场特效：烟雾手雷的体积烟（会遮挡视线，规则见 GAMEDESIGN §6.9.2）、燃烧火焰、爆炸尘土、弹坑与血迹贴花，并按档位控制粒子上限（ARCH §13.1：Low 300 / Medium 800 / High 1 600 / Ultra 3 000；§13.5.3 活动粒子 ≤ 1 600/800/300）。§13.5.2 预算：透明/粒子 0.7 ms（High）。
特效缺失素材时按 ASSET_GAPS §3.2 用引擎内置简单粒子（纯色方片/球），不引入外部贴图；强度/持续时间取玩法数据（烟雾持续时间、半径来自 `/data`，不得写死）。血迹贴花对应 V-FX-005（GAP-V15，M2-047 接入真素材）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/vfx/smoke_volume.ts` —— 烟雾 flipbook 公告牌体积（多层叠加、软粒子深度淡出）与沿生命周期的半径/密度曲线；烟雾参数来自 `/data/weapons/smoke_*.json` 的半径与持续时间（经快照事件下发的 `SmokeDeployed{pos,radius,duration}`）。
- `client/src/render/vfx/fire.ts` —— 火焰/燃烧（载具起火、燃烧体）粒子系统：加色混合、闪烁、烟柱。
- `client/src/render/vfx/dust_explosion.ts` —— 爆炸尘土与冲击环、碎屑；按爆炸威力分三档（手雷/炮弹/炸药）。
- `client/src/render/vfx/decals.ts` —— 贴花管理器：弹孔（M1-047 已有，纳入统一管理）、弹坑（爆炸后地形凹陷贴花）、血迹/飞溅/拖痕；环形缓冲回收 + 按距离/重要性淘汰。
- `client/src/render/vfx/particle_budget.ts` —— 全局粒子/贴花预算与 LOD：超限按优先级丢弃（玩家附近 > 远处；爆炸 > 烟 > 尘）。
- `data/render/vfx.json`、`data/schemas/render_vfx.schema.json` —— 贴花上限与粒子档位表（初值【待调参】：弹孔 256、弹坑 64、血迹 128；粒子上限 Low 300/Medium 800/High 1 600）。
- `client/src/render/vfx/placeholders.ts` —— 占位：纯色方片/球，材质命名 `PH_V-FX-005`、`PH_V-FX-001..`（按 SHOP 需求 ID）。
### 要实现的行为
1. 烟雾：由服务器事件驱动；烟团内多层 flipbook，随风漂移；烟团半径与视线遮挡**视觉**一致（规则判定在 sim，客户端不判定遮挡）。
2. 火焰/爆炸：事件 `Explosion{pos,power}` 触发尘土+闪光+冲击环；`VehicleBurning` 持续火焰直至击毁/熄灭。
3. 贴花：爆炸地面生成弹坑贴花并淡入；击中人体按方向生成血迹贴花（按"血液设置"可关闭——预留 `settings.gore` 开关，M2-041 提供界面）。
4. 预算与 LOD：粒子数量与大小随距离衰减；超过档位上限时丢弃最低优先级；贴花超过上限按 FIFO + 距离回收。
5. 缺失素材：全部使用占位；不报错；开发构建叠加显示需求 ID 标签。
### 接口与数据
```ts
export interface VfxBudget { maxParticles: number; maxBulletDecals: number; maxCraterDecals: number; maxBloodDecals: number }
export class DecalManager { spawn(kind: DecalKind, pos: Vec3, normal: Vec3, size: number, priority: number): void; update(camera: Vec3): void; readonly active: number }
```

## 不做什么
- 不做四档预设联动与热切换 UI（M4-017）；本任务只提供 `VfxBudget` 与 `setBudget()`。
- 不做飞机过场与空袭落弹表现（M3-030）、炮击落点表现（M3-031）、载具击毁/残骸（M3-032）。
- 不做真素材接入（M2-047）。

## 验收标准
1. High 配置（1 600 活动粒子，固定压力场景：10 烟团 + 5 爆炸 + 3 起火载具）透明/粒子 pass GPU 时间 P95 ≤ 0.7 ms（ARCH §13.5.2，RTX 3060，1080p）。
2. `particle_budget` 单测：注入 3 000 个粒子请求，活动数 ≤ 档位上限（300/800/1 600），且被丢弃的是最低优先级。
3. `DecalManager` 单测：生成 1 000 个贴花，各类别活动数 ≤ `data/render/vfx.json` 上限；超限回收顺序正确（FIFO + 距离）；回收后纹理/实例槽位无泄漏。
4. 烟雾 flipbook 在 30 s 持续时间内透明度/半径曲线与数据一致（单测采样 5 个时刻，误差 ≤ 2%）。
5. 缺失 V-FX-005 时不报错且 `PH_V-FX-005` 占位材质存在（缺失素材报告中状态 `placeholder`）。
6. 固定机位截图（烟雾、爆炸、弹坑、血迹各 1）入基线，交 M2-050。

## 测试要求
- 单元测试（vitest）：预算器、贴花回收、烟雾曲线、占位命名。
- 集成/无头测试：Playwright 触发事件序列（模拟事件总线）并截图；粒子计数调试接口 `__vfxStats()`。
- 性能基准：压力场景 1 000 帧 GPU timestamp，输出 `reports/render/vfx/perf.json`。
- 回归：M1-047 的 800 粒子 ≤ 0.7 ms 基线不得回归。

## 性能预算
- ARCHITECTURE §13.5.2 透明/粒子 0.7 ms（High）；§13.5.3 活动粒子 ≤ 1 600（High）/ 800（Medium）/ 300（Low）；§13.4 特效/粒子/UI 纹理 ≈ 120 MB。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（V-FX-005 等）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 贴花上限初值为本任务设定的【待调参】数值（文档未给出），需在 PLAN_ISSUES 记一条"细化"。
- 烟雾对 AI/玩家视线的遮挡由 sim 判定（GAMEDESIGN §6.9.2）；视觉与规则半径不一致是已知风险，验收用同一数据源。
- 血液表现涉及年龄分级与地区策略（H-083），默认保守强度并保留开关。
