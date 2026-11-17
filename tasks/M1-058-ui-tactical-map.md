# M1-058 UI：战术地图

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：L
- 依赖：M1-053, M1-040, M1-035（区块烘焙：底图与据点数据）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.7; UI_SPEC §8.4; UI_SPEC §4.5（部署地图共用）; MAPS §6.2; GAMEDESIGN §3.6

## 背景
战术地图（`M` 键）是全屏 Canvas2D：底图（地形着色栅格、道路、水、建筑轮廓，离线烘焙，缓存）、网格 A1–J10（2048 m / 10 = 204.8 m）、据点与所属着色、重生点、友军单位（1 Hz `TEAM`）、标记。UI_SPEC §4.7.1 图层：L-Base、L-Grid、L-Front、L-Points、L-Spawn、L-Units、L-Markers、L-Build（M1 仅补给箱）；M1 不含 L-Cmd/L-Arty。地图打开时鼠标解锁（退出 Pointer Lock），**地图不是暂停**（角色仍可被攻击）；关闭后允许重新 `requestPointerLock`（§4.7.7）。部署界面地图与战术地图共用同一渲染器实例（§8.4）。重绘频率（§8.4）：L-Base 缩放/平移结束才重烘焙，拖拽时用 CSS transform 位移缓存位图；L-Front/L-Points 事件触发（≤ 1/s）；L-Units ≤ 15 fps（打开时 ≤ 30）；单次重绘 ≤ 2 ms（200 单位 + 100 标记，High）、≤ 4 ms（Low）。符号规则（§4.7.2）：己身=带方向的实心三角+外环；友军=切角矩形+兵种框内图形，本小队紫罗兰+外环；据点正方形+所属色填充（斜纹区分，色盲冗余）；倒地友军加"下沉"线+warn 点；敌方仅经标记显示（菱形框）。M1 标记子集（§4.7.3）：Attention（15 s/每人 3 个）、Enemy Infantry（25 s）、Enemy Vehicle（40 s）、Request Medic/Ammo（30 s）；Order 类（Officer）仅 M1 的 Attack/Defend/Move 三种（本小队，至撤销或 5 min）。M1 数据源：`SpawnPointState`、`ObjectiveChange`、`TeamState`（C3/团队通道）与 `MarkerPlace`（C0）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/map/MapRenderer.ts` —— 共享渲染器（`mode: 'full'|'deploy'|'mini'`），多层 Canvas（静态层/动态层/交互层），图层缓存与脏标记；吸收并替换 M1-056 的 `DeployMap`。
- `client/src/ui/map/layers/*.ts` —— base、grid、front、points、spawn、units、markers、build。
- `client/src/ui/map/MapBaseBake.ts` —— 加载地图包预烘焙的底图金字塔（M1-035 输出；缺失时用 heightmap 着色的低分辨率占位）。
- `client/src/ui/map/coords.ts` —— 网格坐标（A–J / 1–10，小键盘 3×3 细分 `C4-7-3`）。
- `client/src/ui/screens/TacticalMap.tsx` —— 全屏屏幕：图层栏、上下文条、缩放/平移/回到自己、标记放置、测距（Shift+拖拽）。
- `client/src/game/markers.ts` —— 标记放置（发 `MarkerPlace`）、过期与数量上限（本地预判，权威在服务器）。
- 测试：`client/test/ui/map.test.ts`、Playwright `map.spec.ts`。
### 要实现的行为
1. 打开/关闭：`M` 或 `Esc`（先关地图，再次才开暂停，§4.7.7）；过渡 240 ms；Pointer Lock 的释放/重获由 `input` 层事件驱动。
2. 平移（左键拖拽/WASD）、缩放（滚轮以光标为中心、`+/-`、双击放大、`Home` 回全图）、`Space` 回到自己；缩放到一级细分显示小格线。
3. 图层栏单选显隐，`1–9` 切换；记忆上次图层预设（`localStorage`，try/catch）。
4. 单位符号：按 §4.7.2；`TEAM` 数据过期（> 5 s）→ 单位框变虚线并显示"态势数据 N 秒前"。
5. 标记：中键 Ping（Attention）；Officer 可放 Order；每人数量上限与过期倒计时环；无权限工具灰置并 tooltip 说明角色要求。
6. 重绘：静态底图 0 次/帧；动态层按 §8.4 频率；页面不可见时暂停；`requestAnimationFrame` 合并鼠标事件。
7. 部署模式：`mode='deploy'` 仅显示据点/友军小队/可用重生点，点击选点回调。
### 接口与数据
```ts
export interface MapRenderer { setMode(m: 'full'|'deploy'|'mini'): void; setLayers(f: LayerFlags): void; pick(x: number, y: number): MapSelection | null; destroy(): void }
export interface MarkerReq { kind: number; pos: [number, number]; ttlS: number }
```

## 不做什么
- 不做 Frontline 多战线叠层/资源节点（M2-044）、指挥官层与能力（M3）、火炮层（M3-035）、径向菜单（M2-037）。
- 不做敌方侦察信息分层的完整规则（R3 的图标由服务器下发，UI 仅渲染）。
- 不做小地图（`mini` 模式留接口，M2）。
- 不做地图烘焙管线（M1-035）。

## 验收标准
1. 100 个图标（含 20 个标记）单次动态层重绘 ≤ 1 ms（High，`performance.now`，1 000 次中位数；INDEX 指标）；200 单位 + 100 标记 ≤ 2 ms（UI_SPEC §8.4）。
2. 地图重绘频率符合 §8.4：静态底图在无缩放/平移时 10 s 内重绘 0 次；动态层 ≤ 30 fps（计数器断言）。
3. 地图打开期间角色仍可被攻击且 `snapshot` 继续应用（无头测试：开地图 5 s，`entity` 更新计数 > 0）。
4. 缩放 1×–8×、平移时帧耗时 P95 ≤ 4 ms；拖拽期间不触发底图重烘焙（计数器 = 0，松手后 = 1）。
5. 网格坐标换算：随机 1 000 点 `world→grid→world` 误差 ≤ 0.5 m（单测）；`C4-7-3` 样例正确。
6. 标记上限：同一玩家放第 4 个 Attention 时最旧的被替换（vitest）。
7. DOM 节点（地图屏）≤ 300（Canvas 为主）。

## 测试要求
- 单元测试：网格/细分换算；缩放平移矩阵；标记上限/过期；符号选择规则（兵种→框内图形）。
- 集成/无头测试：Playwright 开/关地图、拖拽、标记放置；Pointer Lock 重获。
- 性能基准：重绘耗时与频率写入 `reports/m1-map.json`。
- 回归：M1-056 部署界面改用共享 `MapRenderer` 后测试保持通过。

## 性能预算
- UI_SPEC §8.4/§8.1：地图单次重绘 ≤ 2 ms（200 单位 + 100 标记）；UI CPU 均值 ≤ 1.5 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：标记类型子集与 UI_SPEC §4.7.3 差异写 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 底图依赖 M1-035 预烘焙；缺失时用低分辨率高度着色占位，视觉验收标"需人工"。
- 规模 L：若底图烘焙 + 图层 + 标记超一次会话，拆 `M1-058b`（标记与测距）。
- 敌方信息必须遵守 R3：UI 只渲染服务器下发的标记图标，不做任何推断位置。
