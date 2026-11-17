# M1-056 UI：部署界面

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：L
- 依赖：M1-055, M1-040（部署状态机/roster/出生点状态）, M1-021（sim 小队/名额/部署校验）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.5; UI_SPEC §4.7（部署地图复用）; UI_SPEC §8.4; GAMEDESIGN §2.5; GAMEDESIGN §2.6; GAMEDESIGN §10.6; GAMEDESIGN §4.4; ARCHITECTURE §7.3; ARCHITECTURE §9.4

## 背景
部署界面是信息密度最高的菜单屏：玩家（首次进入、阵亡后、换兵种后）决定"在哪里出生、与谁一起、以什么兵种/装备"。UI_SPEC §4.5 的因果顺序：**位置 → 编组 → 装备**；地图占中央偏左约 56%，右侧小队与兵种/loadout，底部部署按钮与倒计时。GD §2.5 四步：选小队、选兵种、选 loadout、选重生点，"出击"在四步全部有效时亮起；无小队（Unassigned）只能 Rifleman、只能 HQ/据点重生。M1 的 5 兵种：Officer/Rifleman/Medic/Support/Crewman；M1 出生点只有 HQ 与 Garrison。部署流程（ARCHITECTURE §7.3）：`C0 DeployRequest(spawn_point, role, loadout, squad)` → 服务器校验名额/出生点可用/冷却 → `DeployResult(ok, spawn_tick, reason)` → 倒计时期间客户端预取出生点 R_near 区块（§9.4，M1-049 的 `prefetchAround`）。出生时刻规则（GD §10.6）：HQ/据点按 15 s 波次取整，Garrison 基础 8 s，由服务器给出 `spawn_tick`，客户端只显示。名额规则（GD §4.4）：Officer 仅队长且小队 ≥ 2 人、Medic ≤ 2/队、Support ≤ 1/队且需 ≥ 3 人；不可用项灰显并给原因。
重生点状态编码（§4.5.1）：可用/封锁/冷却/被摧毁/已满/未解锁，用形状+颜色+文字。地图复用战术地图渲染器的"部署子集"（M1-058 与本任务共用 `MapRenderer`，实现顺序：本任务先做最小可用部署画布，M1-058 抽成共享渲染器——见备注）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/screens/Deploy.tsx` —— 屏幕容器与布局（态势条、地图、小队、兵种、loadout、部署按钮）。
- `client/src/ui/components/SpawnList.tsx`、`SquadPanel.tsx`、`RolePicker.tsx`、`LoadoutPicker.tsx`、`DeployButton.tsx` —— UI_SPEC §9.2 组件。
- `client/src/ui/map/DeployMap.tsx` —— Canvas2D 部署地图最小版（底图、据点、友军小队位置、可用重生点、缩放/平移）；抽象为 `MapRenderer` 接口以便 M1-058 接管。
- `client/src/game/deploy-actions.ts` —— RPC 封装：`createSquad(type, nameKey)`、`joinSquad`、`leaveSquad`、`lockSquad`、`deploy(req)`；错误映射 `ui.deploy.err.*`。
- `client/src/game/deploy-validate.ts` —— 客户端预校验（名额/解锁/Officer 条件），仅用于禁用与原因展示，权威在服务器。
- 测试：`client/test/ui/deploy.test.ts`、Playwright `deploy-flow.spec.ts`。
### 要实现的行为
1. 状态机（§4.5.3）：`Browsing → SpawnSelected/SquadJoined/RoleSelected → Ready → Waiting → Requesting → Deployed/Rejected`；`Requesting` 期间若服务器拒绝，**不自动换点**，回 `Browsing` 并高亮推荐点。
2. 小队：创建（类型 Infantry 为主，预设代号库 Alpha…Foxtrot，GD §4.2，不允许自由文本）、加入、锁定（队长）；小队列表 ≤ 8 行可视，超出虚拟滚动。
3. 兵种：显示 `已用/上限` 与不可用原因（名额满/小队人数不足/仅队长/未解锁）；Unassigned 只显示 Rifleman。
4. Loadout：槽位候选与武器史实参数行（弹匣、射速、重量；数据来自 `/data/loadouts` 与 `/data/weapons` 的 historical 字段，M1-007/008）。
5. 出生点：列表+地图联动，状态编码（§4.5.1）；非可用点点击显示原因；无可用点时自动选 HQ。
6. 倒计时：显示服务器 `spawn_tick` 对应的剩余秒；倒计时内触发 R_near 预取并显示"预取进度"（`stream.progress`）；部署按钮在倒计时前禁用并显示剩余秒数。
7. 键鼠（§4.5.4）：`1–9` 选重生点列表项、`C` 创建小队、`L` 锁定、`Q/E` 切兵种、`Enter` 部署、`Esc` 暂停；部署到出生 ≤ 2 次点击（选点+部署）。
8. 网络抖动：`connection unstable` 时禁用部署按钮，防止重复请求（请求去重）。
### 接口与数据
```ts
export type DeployUiState = 'browsing'|'ready'|'waiting'|'requesting'|'deployed'|'rejected';
export interface DeployDraft { spawnId?: number; role?: number; loadout?: number; squadId?: number }
export const deployDraft: Signal<DeployDraft>; export const deployUi: Signal<DeployUiState>;
```

## 不做什么
- 不做 Outpost/Airhead/OP 出生点逻辑与显示（M2/M3）、Commander 提示条（M2/M3）、换队流程（M1-061 暂停菜单入口，换队页复用 M1-055）。
- 不做完整战术地图的图层/标记（M1-058）；不做聊天/语音小部件（M2-040）。
- 不做 3D 装备预览（UI_SPEC 明确不做，用线稿+数据行）。
- 不实现服务器校验（M1-021/M1-026）。

## 验收标准
1. 全流程键鼠可达：从 `FactionSelect` 进入后，仅用鼠标或仅用键盘均可完成"选小队→选兵种→选 loadout→选出生点→部署"（Playwright 两套脚本，不使用 `page.evaluate` 改状态）。
2. 非法选择被禁用：名额已满的兵种、`Officer` 在小队 < 2 人时、被封锁的重生点，三类均为 `disabled` 且带 tooltip 原因（vitest + Playwright 断言 i18n key）。
3. 部署到出生 ≤ 2 次点击（选中重生点 + 点击部署，脚本计数）。
4. 倒计时与服务器 `spawn_tick` 误差 ≤ 1 s（Playwright + 服务器 stub，对 20 次采样取最大值）。
5. 倒计时内 R_near 预取完成率 ≥ 95%（100 Mbps 限速，10 次测量；MILESTONES E1-10）。
6. 拒绝处理：服务器返回"重生点失效"后 UI 回到 `Browsing`，高亮推荐点，不自动部署（vitest）。
7. 部署界面 DOM ≤ 1 500 节点；打开到可交互 ≤ 300 ms（`?uiperf=1`）。

## 测试要求
- 单元测试：`deploy-validate` 全分支（名额/解锁阈值 GD §4.4）；状态机转移；错误映射。
- 集成/无头测试：Playwright + 服务器 stub 完整流程；预取进度联动。
- 性能基准：部署界面 UI 脚本耗时入 `reports/m1-deploy.json`。
- 回归：M1-055 的阵营选择 → 部署衔接不破。

## 性能预算
- UI_SPEC §8.1：菜单/部署 DOM ≤ 1 500；UI CPU ≤ 1.5 ms 均值；地图画布 ≤ 30 fps、单次重绘 ≤ 2 ms（§8.4）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材无；缺失素材需求 ID 无（图标用 SVG 占位）
- [ ] 文档同步：与 UI_SPEC §4.5 不一致处写 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L 且含地图画布：INDEX 把 M1-058 排在本任务之后，故本任务先做最小 `DeployMap`，M1-058 将其重构为共享 `MapRenderer`（UI_SPEC §8.4：部署地图与战术地图共用同一渲染器实例）；两任务须在 PLAN_ISSUES 注明接口交接。
- 若超一次会话，拆 `M1-056b`（loadout 选择与预取联动）。
- 小队命名与"不允许自由文本"依赖 GD §4.2；队长后缀 ≤ 16 字符的敏感词过滤放 M2。
