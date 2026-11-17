# M0-025 client/render：灰盒角色、第一人称枪与射击反馈

## 元数据
- 里程碑：M0
- 工作流：client-render
- 规模：M
- 前置依赖：M0-022, M0-023
- 素材需求：V-CHR-US-001, V-CHR-GE-001, V-WPN-US-001, V-WPN-GE-001, V-FX-001, V-FX-006（均灰盒占位）；素材状态：placeholder
- 引用文档：ASSET_GAPS §3; ARCHITECTURE §7.1; ARCHITECTURE §7.2; ARCHITECTURE §8.4; UI_SPEC §2.5

## 背景
M0 需要让开火、命中、其它玩家"看得见"，但**不依赖任何真素材**：按 ASSET_GAPS §3 的灰盒约定，用胶囊 + 头球做角色、长方体 + 圆柱做枪，并保留真素材将使用的挂点（socket）。数值取自 `/data`：胶囊半径 0.35 m、站高 1.8 m、蹲高 1.2 m（来自 `roleDef("rifleman").capsule`，M0-011/014）；枪的长度、枪口位置取自武器数据/史实尺寸（Garand 约 1.1 m、Kar98k 约 1.1 m 量级，**写在 `/data` 的 `fx/dimensions` 字段，不写进渲染代码**，若缺字段则先补 `weapon.schema` 的可选字段并记 PLAN_ISSUES）。
命名（ASSET_GAPS §3.1）：材质 `PH_{需求ID}`、节点 `PH_{需求ID}__{实例名}`；开发构建在占位旁显示需求 ID 标签（`import.meta.env.DEV`）；阵营色：美军橄榄绿、德军灰（仅调试配色）。挂点命名：武器 `socket_muzzle`、`socket_ejection`、`socket_magazine`；角色 `socket_hand_r`、`socket_hand_l`、`socket_head`。特效用 Babylon 内置粒子/简单网格（纯色方片/球），**不引入外部贴图**（§3.2 特效行）。
射击反馈来源：本地开火 → `fire.local` 事件（M0-022，**本地预测，不等服务器**）；远端开火 → `fire.remote`（由快照事件 `Shot` 派生）；弹着 → `impact`；命中标记 → `hit.confirm`（仅射手收到，ARCHITECTURE §6.8.4）。客户端**不显示预测伤害/击杀**（§7.2）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/actors/{placeholder-character.ts,placeholder-weapon.ts,actor-manager.ts,names.ts,index.ts}`：`ActorManager` 订阅 `entity.spawned/removed`，每帧读 `GameState.entities` 的插值结果更新节点；本地玩家不渲染胶囊，只渲染第一人称枪。
- `client/src/render/fx/{muzzle-flash.ts,tracer.ts,impact-dust.ts,first-person-camera.ts}`；`client/src/ui/{crosshair.ts,hit-marker.ts}`（DOM 覆盖层最小实现：十字准星 + 命中标记闪烁 120 ms；文案无；i18n 无关）。
- 测试：`client/src/render/actors/*.test.ts`、`client/e2e/actors.spec.ts`。
### 要实现的行为
1. **角色占位**：胶囊 + 头球 + 肩/髋朝向标记（小方块指示朝向），站/蹲高度随 `stance` 过渡；阵营着色（US 橄榄绿 `#6B7B3A`、GER 灰 `#7A7F85` 之类，仅调试）；节点名 `PH_V-CHR-US-001__{playerId}`/`PH_V-CHR-GE-001__{playerId}`，带 `socket_*` 子节点；位置/朝向来自插值状态。
2. **第一人称枪占位**：长方体（机匣）+ 圆柱（枪管）+ 矩形弹匣，按武器数据长度；挂在相机下，随后坐（`recoil_pitch/yaw`，来自 `Predictor.weaponState()`）与简单行走摇摆微动；`socket_muzzle` 位于枪口；ADS 时枪移到视线中心且 FOV 过渡（FOV 值取 `/data`，M0 缺则用 `ads_ms` 与常量并记录）；节点 `PH_V-WPN-US-001__fp`（Garand）、`PH_V-WPN-GE-001__fp`（Kar98k）；第三人称同形小一号挂在角色 `socket_hand_r`。
3. **枪口焰**：`fire.local`/`fire.remote` → 在 `socket_muzzle` 位置放置发光球/十字面片，持续 `fx.muzzle_flash_ms`（M0-008 数据 50 ms）；材质/节点命名 `PH_V-FX-001`。**开火与枪口焰同帧**：事件在同一 `scene.onBeforeRenderObservable` 内被消费。
4. **曳光**：调试曳光（`?tracers=1`，M0 默认开）：以细长盒沿弹道方向飞行并在命中点消失，`PH_V-FX-006`；弹着点：撞击处小尘土粒子 + 贴地小方片（`impact`）。
5. **命中标记**：`hit.confirm` → DOM 命中标记闪烁；`damage.taken` → 屏幕边缘红色方向条（简单，4 个方向，120 ms，UI_SPEC 的完整受击指示属 M1-057）。
6. **缺失素材报告**：`window.__placeholders` 列出所有 `PH_*` 需求 ID 与实例数，供 M1-001 的"缺失素材报告"对接；占位项 `placeholder:true`。
### 接口与数据
- `ActorManager.update(alpha)`；`attachWeaponFP(weaponId)`；全部数值来自 `roleDef/weaponDef`（JSON bytes，经 M0-014）。`render` 不 import `net/ui/input`；`ui/crosshair` 通过事件总线订阅。

## 不做什么
- 不做骨骼角色/动画/蒙皮（M1-045）；不做真实枪模与第一人称 pass（M1-046，V-WPN-*）；不做后处理/弹孔贴花（M1-047）。
- 不做音频（M1-062/063）；不做完整 HUD（M1-057）；不做载具渲染（M1-048）。
- 不做受击/死亡动画与布娃娃；死亡仅把胶囊倒下并半透明（≤ 20 行）。

## 验收标准
1. **同帧**：Playwright/`vitest` 渲染测试用 `window.__perf` 记录 `fireEventFrame` 与 `muzzleFlashVisibleFrame`，对连续 30 次本地开火两者**帧号差 = 0**。
2. **命名**：遍历 `scene.meshes/materials`，所有占位对象名匹配 `^PH_V-[A-Z]+-[A-Z0-9-]+(__.+)?$`（单元测试）；非占位对象（地面网格线等）不得以 `PH_` 开头；`window.__placeholders` 至少包含 6 个需求 ID。
3. **挂点**：每个武器节点含 `socket_muzzle`、`socket_ejection`、`socket_magazine`；每个角色含 `socket_hand_r/_l`、`socket_head`（测试断言）。
4. **插值无抖动**：10 名远端实体（M0-026 的 bot 或合成快照流，20 Hz、10% 抖动、2% 丢包）60 fps 渲染 60 s：渲染位置帧间位移的二阶差分 RMS ≤ 0.02 m（实体匀速时），0 次瞬移（> 1 m 单帧位移）。
5. **尺寸来自数据**：改 `data/roles/rifleman.json` 的 `capsule.height_stand_m` 后（重新构建），胶囊高度随之变化（无硬编码常量，grep 验证无 `1.8`/`0.35` 字面量出现在 `render/actors`）。
6. 帧开销：10 远端角色 + 第一人称枪 + FX，CPU 帧时间相对空场景增量 ≤ 0.5 ms（本机/参考机，记录）；draw call 增量 ≤ 40。
7. 命中标记：`hit.confirm` 事件到 DOM 标记可见的延迟 ≤ 1 帧（e2e 断言）。

## 测试要求
- 单元测试：命名规则、挂点表、朝向/高度过渡、FX 计时（fake timers）。
- 集成/无头测试：Playwright 加载场景（软件渲染）截图非空；`actors.spec.ts` 验收 1/7。
- 性能基准：验收 6，交 M0-030。
- 回归：无。

## 性能预算
动态角色 ≤ 60（WebGPU High）、粒子 ≤ 1 600（High）、可见三角形 ≤ 4.0 M（ARCHITECTURE §13.5.3）；M0 角色 ≤ 20，应远低于；本任务不得使 M0-024 的 draw call ≤ 400 结论回归（合计复测）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 把本任务素材需求写成"无（灰盒）"；上面列出的 V-* 是占位所绑定的需求 ID（ASSET_GAPS §3.1 要求每个占位绑定一个需求 ID），并非阻塞项。V-CHR-* 的确切 ID 以 `docs/ASSET_SHOPPING_LIST.md` 为准（M0 采购表 §4.1.2 对角色无专属 ID，沿用 M1 表 `V-CHR-US-001/V-CHR-GE-001`）。
- 准星/命中标记是 DOM 极简实现（UI_SPEC 的最终样式属 M1-057），文案无，故不涉及 i18n；UI 图标/字形的原创绘制待人类确认（NEEDS_HUMAN H-003），本任务只用几何图形。
- 缺失素材需求 ID（回报用）：V-CHR-US-001、V-CHR-GE-001、V-WPN-US-001、V-WPN-GE-001、V-FX-001、V-FX-006；音频全部缺失（M0 不做音频）。
