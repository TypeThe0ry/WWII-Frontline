# M2-044 UI：战术地图的 Frontline 叠层

## 元数据
- 里程碑：M2
- 工作流：ui
- 规模：M
- 前置依赖：M1-058, M2-008, M2-009, M2-005
- 素材需求：无（灰盒即可）；素材状态：n.a.（底图由 M2-005 的区块/矢量数据烘焙，图标为原创 SVG/Path2D）
- 引用文档：UI_SPEC §4.7; UI_SPEC §8.4; UI_SPEC §7.1; GAMEDESIGN §3.3; GAMEDESIGN §11.3; GAMEDESIGN §10.1; GAMEDESIGN §17.4

## 背景
M1-058 已有全屏 Canvas 战术地图：底图、据点/出生点/小队、标记、缩放平移。M2 需要把 Carentan 全图的 Frontline 模式状态叠加到地图：GAMEDESIGN §3.3 的 **5 条战线 L1–L5**（L1 在 US 侧最深处，L3 中线）、每条线 2–3 个候选 Strongpoint 中服务器抽出的 1 个、当前 `Active` 活动线集合（1 或 2 条且相邻）、非活动线上锁（锁图标，不计占领）、占领进度环、Sector 多边形着色（所有者色 + 中立战区）、据点易主。M2-009 提供资源节点（Manpower/Munitions/Fuel，每类每队 ≤ 3，建在 Sector 内 `resource_site`）；M2-011 提供 Outpost/Garrison/Repair Station/AT 炮；M2-008 提供战线状态事件。
UI_SPEC §4.7.1 图层：L-Front（战线与区域着色：蓝我/橙敌/灰中立，色盲追加图案：我方横向细纹、敌方斜纹、中立点阵，§7.1）、L-Points、L-Spawn（友方 HQ/Garrison/Outpost/Airhead；敌方仅被发现/标记后显示）、L-Build（友方建造物、补给点）；§4.7.2 符号规则（Garrison 方框门洞、Outpost 小方框+旗杆、Airhead 向下箭头、HQ 较大切角方框；形状冗余始终开启）。**侦察信息分层**（R3、GAMEDESIGN §17）：友军 1 Hz 团队通道；被友军标记/侦察暴露的敌人只显示地图图标（位置量化 5 m、1 Hz）；敌方 Garrison/OP 仅在被发现/标记后显示。重绘频率（UI_SPEC §8.4）：L-Base 缓存；L-Front/L-Points 事件触发 ≤ 1/s；L-Units ≤ 15 fps（地图打开时 ≤ 30）；L-Markers ≤ 10 fps。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/map/layers/FrontLayer.ts` —— 战线与 Sector 着色（离屏缓存 + 事件触发重绘）、前线边界高亮（"front_A/front_B 边界"）、活动线描边、锁定线灰化 + 锁图标。
- `client/src/ui/map/layers/PointsLayer.ts`（修改 M1-058）—— 据点方框、名称、占领环、所有者填充 + 色盲图案；被"战线切换"重算的动画仅 `opacity`。
- `client/src/ui/map/layers/ResourceNodeLayer.ts` —— 三类资源节点图标（原创 SVG：补给站/弹药堆场/油料堆场）+ 可建造 `resource_site` 空位提示（仅己方 Sector 内、Engineer 视角强调）。
- `client/src/ui/map/layers/SpawnBuildLayer.ts`（修改）—— Garrison/Outpost/Airhead/HQ/Repair Station/AT 炮/OP 图标与状态编码（可用/封锁/拥挤/被压制，对齐 UI_SPEC §4.5 状态），Condemned Garrison 的 30 s 倒计时环。
- `client/src/ui/map/layers/IntelLayer.ts` —— 被标记/侦察暴露的敌方图标（菱形 + 内标 + 剩余时间环），位置量化 5 m、1 Hz；过期淡出。
- `client/src/game/frontline_store.ts` —— 信号：`lines[5]{owner, candidate, progress, locked}`、`active: number[]`、`sectors`、`resourceNodes`、`structures`；由 M2-008/M2-009/M2-011 的事件与快照驱动。
- `client/src/ui/map/legend.ts` —— 图例条目（战线/据点/资源节点/出生点）。
- `client/src/i18n/{en,zh-CN}/map.json` —— `ui.map.layer.front` 等。
### 要实现的行为
1. 全图 5 条战线按 `/data/maps/carentan/zones.json`（M2-004）的 Sector 多边形着色；候选据点仅显示本局抽中的那个（`seed = hash(match_id)`，GAMEDESIGN §3.3.1）。
2. 战线状态实时：据点占领/易主后 ≤ 1 s 内重绘 L-Front/L-Points；活动线（≤ 2 条且相邻）描边；非活动线显示锁。
3. 资源节点与结构：友军可见；敌方结构仅被发现后显示；Sector 易手时被销毁的节点图标立即移除（GAMEDESIGN §3.3.6）。
4. 战线推进时 Commander/队长地图出现新的"前线边界"高亮（边界线由 `front_A/front_B` 计算，客户端只渲染服务器给出的边界）。
5. 色盲模式：区域着色追加图案；所有状态色附形/文字（UI_SPEC §7.1）。
6. 地图关闭时不绘制；部署界面与战术地图共用同一渲染器实例（UI_SPEC §8.4）。
### 接口与数据
```ts
export interface LineView { index: 1|2|3|4|5; owner: 'A'|'B'|'neutral'; candidateId: string; progress: number /* -100..100 */; locked: boolean }
export const frontline = { lines: signal<LineView[]>([]), active: signal<number[]>([]), resourceNodes: signal<ResourceNodeView[]>([]), structures: signal<StructureView[]>([]) };
```

## 不做什么
- 不做战线规则/资源规则本身（M2-008/M2-009）、地图关卡数据标注（M2-004）。
- 不做指挥官层 L-Cmd 与火炮层 L-Arty（M3-033、M3）。
- 不做小地图（若启用）之外的新功能；不做地图上的 Ping/命令工具栏（M1-058 已有，M2-037 的轮盘接入）。

## 验收标准
1. 渲染正确性：用 5 组固定夹具状态（开局 L3 中立；A 占 L3；B 夺回；A 占 L4；A 全占胜利）分别渲染，Active 集合、锁图标、所有者着色与 GAMEDESIGN §3.3.3 的计算结果 5/5 一致（截图基线 + `vitest` 夹具断言 `front_A/front_B` → `Active`）。
2. 重绘频率符合 UI_SPEC §8.4：地图打开 10 s，L-Base 重绘 0 次（缓存），L-Front/L-Points ≤ 1 次/s（无事件时 0），L-Units ≤ 30 fps，L-Markers ≤ 10 fps（帧计数脚本）。
3. 单次动态层重绘 ≤ 2 ms（200 单位 + 100 标记 + 18 个结构 + 9 个资源节点，`performance.now` 100 次 P95，参考机）。
4. 战线变更到界面更新延迟：服务器事件到达 → L-Front 重绘完成 ≤ 100 ms（Playwright 计时）。
5. 色盲模式 rg/by/hc 下三种区域图案（横纹/斜纹/点阵）均可区分（截图基线 3 张），且单色灰度截图中三类区域的图案/形状可区分（脚本对灰度图做图案频率检测）。
6. 敌方信息泄露检查：渲染输入不含未被标记的敌方实体（单测：喂入全量敌方列表仅显示标记集合，R3）。
7. 地图 DOM ≤ 1 500；打开地图时 UI CPU ≤ 3 ms/帧 P95。

## 测试要求
- 单元测试：`front_A/front_B/Active` 推导（与 sim 同一夹具，防止两端偏差）；位置量化 5 m；过期淡出；资源节点销毁事件。
- 集成/无头测试：Playwright 加载 Carentan 灰盒数据，驱动 5 组战线夹具；headless 联调真实战线事件。
- 性能基准：`?uiperf=1` + 帧计数；记录到 `reports/ui/map_frontline.json`。
- 回归：M1-058 的"100 图标 ≤ 1 ms"用例继续通过。

## 性能预算
- UI_SPEC §8.1/§8.4：动态层 ≤ 15 fps（打开 ≤ 30），单次重绘 ≤ 2 ms，静态底图 0 次/帧；UI 总 CPU ≤ 1.5 ms 均值。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 底图烘焙依赖 M2-005 的地形/道路/水系矢量；若底图仅有低分辨率占位，先用等高线占位（UI_SPEC §4.7.8）。
- 图标为执行者原创 SVG/Path2D（H-003 待确认）；形状语言遵循 UI_SPEC §2.5.3，避免与任何参考作品符号相似（UI_SPEC §10 自查）。
- 战线切换动画不得造成每帧 DOM 写入；仅 `opacity`。
