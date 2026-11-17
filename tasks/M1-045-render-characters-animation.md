# M1-045 render：骨骼角色与动画状态机

## 元数据
- 里程碑：M1
- 工作流：client-render
- 规模：L
- 前置依赖：M1-006, M0-025, M1-042
- 素材需求：V-CHR-US-001; V-CHR-GE-001; V-ANM-001; V-ANM-002; V-ANM-004; V-ANM-005（占位可）；素材状态：placeholder（胶囊+头球，程序化占位动画）
- 引用文档：ARCHITECTURE §13.5.3; ASSET_GAPS §3（§3.1、§3.2）; ARCHITECTURE §7.1; GAMEDESIGN §9.2

## 背景
M0-025 的角色是不带骨骼的胶囊。M1 要在同屏 ≤ 60 个动态角色（High，§13.5.3；WebGL2 Medium ≤ 40，Low ≤ 24）的预算内渲染带 GPU 蒙皮的步兵，并驱动动画：走/跑/蹲/趴/受击/死亡等。真素材（V-CHR、V-ANM，MoCap/Mixamo 重定向，M1-064 接入）可能晚到，所以本任务的核心是**接口与流水线**：骨架约定、动画状态机、LOD、重定向接口，以及程序化占位动画，使"占位与真资源切换不改代码"。ASSET_GAPS §3.1 的占位规则：占位绑定需求 ID，材质 `PH_{需求ID}`，节点 `PH_{需求ID}__{实例名}`，保留 socket；§3.2：角色占位 = 胶囊体 + 头部球 + 简易肩/髋标记；动画缺失时只做位移与朝向。姿态时序见 GD §9.2（站→蹲 0.25 s、蹲→趴 0.50 s、站→趴 0.70 s 等）。远端实体使用 100 ms 插值缓冲（ARCHITECTURE §7.1）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/actors/Skeleton.ts` —— 统一骨架约定（骨名映射表，Humanoid 命名；含 `socket_hand_r`、`socket_hand_l`、`socket_head`、`socket_back`、`socket_muzzle`（来自武器）），重定向表数据格式 `retarget/*.json`。
- `client/src/render/actors/CharacterRenderer.ts` —— 实例管理、LOD0–2（距离切换阈值取自 `/data` 或配置）、GPU 蒙皮（Babylon 的 bone texture / WebGPU storage buffer）、阵营着色。
- `client/src/render/actors/AnimGraph.ts` —— 动画状态机：层（下半身移动层 / 上半身瞄准+射击层）、状态（idle/walk/run/sprint/crouch_*/prone_*/vault/hit_react/downed/dead/throw/reload/fire）、过渡时间取 GD §9.2。
- `client/src/render/actors/ProceduralAnim.ts` —— 无动画资源时的程序化占位：位移/朝向/姿态高度（站 1.8 m / 蹲 1.2 m / 趴 0.5 m 的胶囊缩放）、受击闪白、死亡倒伏。
- `client/src/render/actors/Retarget.ts` —— 重定向流水线接口（离线工具调用，运行时只读已重定向剪辑）。
- `client/src/render/actors/actors.test.ts`、`client/test/visual/characters.spec.ts`。
### 要实现的行为
1. 角色状态来自 `game` 只读状态（SoA）：位置、yaw/pitch、姿态、速度、`anim_state`、武器、健康/倒地标志；渲染层不反向写 game。
2. 动画图根据 `anim_state` 与速度混合；瞄准偏移用 pitch 驱动上身；开火/装填由事件触发（`fire.remote` 等）。
3. LOD：LOD0 ≤ 45k 三角形（真素材规格，ASSET_SHOPPING_LIST §4.0），LOD1/2 递减；远处（> 150 m【待调参】）切换为单 draw 的低骨骼数 LOD；占位 LOD 同接口。
4. 资产回退：未找到真资产时自动用程序化占位；manifest `placeholder` 字段为真时显示开发构建的需求 ID 标签。
5. 动画事件：脚步事件（左右脚落地）发 `entity.footstep`（供 M1-063 脚步声）。
6. 实例化：同一模型的角色共享骨架纹理布局，批量蒙皮。
### 接口与数据
```ts
export interface ActorView { id: number; team: 0|1; role: number; pos: Vec3; yaw: number; pitch: number; stance: Stance; animState: number; weaponId: number; flags: number }
export class CharacterRenderer { sync(views: ReadonlyArray<ActorView>, alpha: number): void; setLodBias(b: number): void }
export type AnimEvent = { type: 'footstep'; foot: 'l'|'r'; actor: number } | { type: 'fire'; actor: number };
```

## 不做什么
- 不做第一人称手臂/武器（M1-046）、坦克（M1-048）。
- 不做布娃娃物理、服装/兵种变体（M2 起，V-CHR-US-003 等）。
- 不做面部/口型；不做载具乘员姿势（GAP-V08，M3-029）。
- 不做真素材的采购与许可（M1-064 接入）。

## 验收标准
1. 60 个蒙皮角色（LOD0/1 混合，动画全开）GPU 耗时 ≤ 1.5 ms（High 参考机，1080p；GPU timestamp 叠层，固定机位"开阔"）。
2. 60 个角色的 CPU 动画更新 + 提交 ≤ 1.0 ms/帧（叠层 CPU 项）。
3. 占位与真资源切换不改代码：用一个测试资产（任意 CC0 骨骼 glTF）经 manifest 替换占位，角色渲染与动画状态机照常工作（`characters_swap.spec`），代码 diff 为 0。
4. 姿态过渡时长与 GD §9.2 一致：站→蹲 0.25 s ±1 帧、蹲→趴 0.50 s ±1 帧（`animgraph.test`）。
5. 所有占位节点命名匹配正则 `^PH_[A-Z]-[A-Z0-9-]+__.+$`（遍历断言）。
6. 同屏三角形：60 角色 ≤ 0.9 M 三角形（统计，LOD 分布按距离）。

## 测试要求
- 单元测试：动画图转移表、过渡时间、LOD 选择、重定向表解析、`PH_` 命名。
- 集成/无头测试：Playwright 固定机位 60 角色场景截图（WebGPU/WebGL2 各一套基线）。
- 性能基准：叠层导出 GPU/CPU 耗时到 `reports/m1-characters.json`。
- 回归：M0-025 的 10 实体插值无抖动测试保持通过。

## 性能预算
- ARCHITECTURE §13.5.3：动态角色 ≤ 60、骨骼动画 ≤ 60（High）；WebGL2 Medium ≤ 40，Low ≤ 24；§13.5.2 角色计入不透明主通道 4.0 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、视觉回归）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID（V-CHR-*、V-ANM-*）已列出
- [ ] 文档同步：骨架/socket 命名写入 PLAN_ISSUES"细化"
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：若骨架约定+动画图+LOD 一次做不完，优先完成"占位 + 接口 + 动画图"，GPU 蒙皮批处理可拆 `M1-045b`。
- 角色骨架的最终选择取决于所购素材（H-049 采购批、GAP-V17/V18 决定 H-047）；本任务骨架约定须兼容 Mixamo/Humanoid 命名。
- 缺失的 V-ANM 动画一律用程序化占位，不得自制动画资源（D-MOD）。
