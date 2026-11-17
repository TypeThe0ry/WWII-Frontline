# M2-038 UI：记分板

## 元数据
- 里程碑：M2
- 工作流：ui
- 规模：M
- 依赖：M1-057, M2-017
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.9; UI_SPEC §8.1; UI_SPEC §8.2; GAMEDESIGN §18.1; GAMEDESIGN §18.3; GAMEDESIGN §16.1

## 背景
UI_SPEC §4.9：Tab 按住显示（Peek，不解锁指针）、松开隐藏；`Shift+Tab` 锁定（Locked，需 `exitPointerLock`）后可点击玩家名打开玩家菜单（静音/举报/加好友/投票踢）。布局：我方居左、敌方居右；按小队分组（可折叠），"无小队"组与观战/未部署折叠区；行元素：昵称、小队、兵种图标、击杀、阵亡、助攻/支援分、战斗/防守/进攻/补给分、ping、Bot 标识、语音说话指示；队内完整、对方仅显示昵称/兵种/击杀/阵亡/ping【待测 → REFERENCE_MEASUREMENTS】；排序可按分数/击杀/名称/小队；超过 100 行用虚拟滚动（可视行 ± 10）；自己行高亮；"更新于 N s 前"小字。
数据来源：M2-017 `sim-scoring-xp` 的得分类别与局内统计（GAMEDESIGN §18.1 四类：Combat/Offense/Defense/Support；§18.3 列：名字、兵种、K/D/A、四类分数、总分、延迟、小队）。更新频率 2 s（EVT），对局头 1 Hz。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/scoreboard/Scoreboard.tsx` —— 记分板根组件：对局头（地图/模式/剩余时间/战线进度）、两队列、折叠区；Peek/Locked 状态机（UI_SPEC §4.9.3）。
- `client/src/ui/scoreboard/ScoreRow.tsx`、`SquadGroup.tsx` —— 行与小队组头（名称、人数、总分、锁定图标）；Bot 标识；语音指示。
- `client/src/ui/scoreboard/VirtualList.tsx` —— 通用虚拟滚动（固定行高，窗口 ± 10 行）。
- `client/src/ui/scoreboard/PlayerMenu.tsx` —— 玩家菜单（静音/举报入口：调用 M2-043 的举报表单与 M2-040 的静音名单；本任务只发起事件 `ui.report.open(playerId)`）。
- `client/src/game/scoreboard_store.ts` —— 信号状态：由 `EVT` 消息（M2-017 下发的 ScoreSnapshot）更新；排序与分组派生。
- `client/src/i18n/{en,zh-CN}/score.json` —— `ui.score.*`。
### 要实现的行为
1. Tab 按住显示/松开隐藏；`Shift+Tab` 或点击锁定键进入 Locked；Locked 下再按 Tab/Esc 关闭并提示"点击继续"重新锁定（UI_SPEC §4.9.4）。
2. 队内 vs 对方列权限：对方行只渲染受限列（由数据驱动 `visibleColumns[side]`），服务器已不下发的字段不依赖客户端隐藏（反作弊/信息控制）。
3. 分组：按小队分组，Commander 与小队长置顶（GAMEDESIGN §18.3）；"无小队"散兵组；小队数量无固定上限（R2，步兵小队数不限、装甲车组 ≤ 4、侦察组 ≤ 2）。
4. 排序：点击表头切换（分数/击杀/名称/小队），排序稳定（相同键保持原相对顺序）。
5. 数据延迟与空态："更新于 N s 前"；开局无数据显示"暂无数据"；断线重连后立即请求完整快照。
6. 玩家文本一律文本节点渲染，禁止 `innerHTML`。
### 接口与数据
```ts
export interface ScoreRowData { playerId: number; name: string; squadId: number | null; role: RoleId; isBot: boolean; k: number; d: number; a: number; combat: number; offense: number; defense: number; support: number; total: number; pingMs: number; speaking?: boolean }
export const scoreboard = { rows: signal<ScoreRowData[]>([]), updatedAtMs: signal(0), locked: signal(false) };
```
字段与 M2-017 的 `ScoreSnapshot`（protocol）一一对应；不得自行发明字段名。

## 不做什么
- 不做结算界面（M2-042）；不做举报表单/投票面板本体（M2-043）。
- 不做语音静音名单的管理界面（M2-040/M2-041）。
- 不做管理员工具（M2-023 的 API 与后续运营界面）。

## 验收标准
1. 100 行（双方各 50）数据刷新一次的脚本耗时 ≤ 1 ms（`performance.now`，Chromium，参考机，100 次中位数；含虚拟列表重算，不含浏览器绘制）。
2. 记分板打开时 DOM 节点 ≤ 1 500（UI_SPEC §8.1，虚拟化后实测）；100 行刷新期间每秒 DOM 写入 ≤ 400（MutationObserver 采样 10 s）。
3. 排序稳定性：对 100 行随机数据按 4 种键各排序 100 次，相同键元素相对顺序不变（vitest 属性测试）。
4. Peek/Locked 状态机全部转换有单测；Peek 期间 `document.pointerLockElement` 保持不变（Playwright）。
5. 对方行不渲染受限列（单测断言 DOM 中无这些列的文本）。
6. 虚拟滚动：200 行（含观战）滚动到末尾，可视窗口外渲染行数 ≤ 20。
7. i18n 键完整性测试通过（en/zh-CN）。

## 测试要求
- 单元测试：store 派生（分组/排序/置顶）、虚拟列表窗口计算、列权限；快照缺失字段的容错。
- 集成/无头测试：Playwright 模拟 100 单位事件流（UI_SPEC §8.5 场景）；headless-client 取得真实 ScoreSnapshot 渲染。
- 性能基准：`?uiperf=1` 叠层采集；CI 场景超出 UI_SPEC §8.1 阈值 20% 即失败。
- 回归：HUD 战斗态节点数不变。

## 性能预算
- UI_SPEC §8.1：UI 总 CPU ≤ 1.5 ms 均值；菜单/记分板 DOM ≤ 1 500；每秒 DOM 写入 ≤ 400；ARCH §13.5.1 UI 0.8 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 对方信息展示规则【待测 → REFERENCE_MEASUREMENTS】尚无测量值，按 UI_SPEC §4.9.1 初值，数据驱动便于后改。
- 语音说话指示依赖 M2-026（LiveKit 客户端）；未就绪时该列为空，不阻塞。
- Locked 态重新锁定 Pointer Lock 必须落在用户手势上（UI_SPEC §5.5）。
