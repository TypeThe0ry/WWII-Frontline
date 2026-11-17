# M1-055 UI：服务器浏览器与阵营选择

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：M
- 依赖：M1-054, M1-028（gateway：服务器列表/游客鉴权/join ticket）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.3; UI_SPEC §4.4; GAMEDESIGN §2.3; GAMEDESIGN §2.4; ARCHITECTURE §19.2; ARCHITECTURE §3.3

## 背景
玩家从主菜单进入服务器浏览器，选择服务器，加入后选阵营，再进入部署界面（M1-056）。数据来自 gateway（M1-028）：服务器列表（HTTP），join ticket（EdDSA，TTL 60 s、单次，ARCHITECTURE §19.2）。UI_SPEC §4.3.1 的列：服务器名、地图+缩略图、模式、人数（真人+bot 分开显示如 `72+18/100`）、阶段、Ping（≤ 60 好/≤ 120 中/> 120 差【待调参】）、区域、版本兼容标记、密码标记、bot 标记；筛选：地图/模式/区域/ping 上限/人数/含 bot/密码/版本兼容/关键字；排序键默认"推荐"；状态机 `Loading → Listed → Selected → Joining → NeedPassword/Queued/Rejected`。阵营选择（§4.4）：`FactionCard`，US/GER，人数（真人/bot）、平衡条、禁用原因、冷却、自动分配；状态机 `Choosing → Requesting → Joined/Rejected`。
**规则一致性**：GAMEDESIGN §2.4 规定：设 `H_t` 为某队真人数，新玩家只能加入使 `H_t − H_other ≤ 2` 成立的队伍；每队 50 人上限；换队冷却 600 s、加入后 3 min 内不可换队。UI_SPEC §4.4 写的是"差 > 3 不可选、冷却 120 s"，二者不一致。**以 GAMEDESIGN §2.4 为准**（服务器权威），UI 数值一律读服务器下发的规则字段，不硬编码，并在 PLAN_ISSUES 登记此冲突。M1 仅 Carentan + Frontline（M1 简化模式），浏览器筛选项只做必要子集。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/screens/ServerBrowser.tsx` —— 列表、筛选栏、排序、选中详情、加入/密码弹窗；`ServerRow`/`ServerFilters` 组件。
- `client/src/ui/screens/FactionSelect.tsx` —— 阵营选择；`FactionCard`、平衡条。
- `client/src/game/gateway-client.ts` —— `GET /servers`、`POST /join`（游客 token + ticket），错误码映射；`ping` 测量（对服务器 HTTP 端点往返，方法未核实：UI_SPEC §4.3.1 注）。
- `client/src/game/faction-rules.ts` —— 阵营可选性判定（读服务器下发的 `H_us/H_ger/cap/cooldown`，实现 GD §2.4 公式；仅作 UI 预判，权威在服务器）。
- 测试：`client/test/ui/browser.test.ts`、`faction.test.ts`、Playwright `join-flow.spec.ts`。
### 要实现的行为
1. 列表每 10 s 自动刷新（UI_SPEC 的 GW 频率），`F5` 在本屏仅刷新列表；筛选条件持久化到 `localStorage`（try/catch）。
2. 加入流程：选中→`Joining`→服务器返回 `Reject`（版本/满员/封禁/ticket 无效/数据哈希，ARCHITECTURE §6.8.1）映射到 i18n 原因；`NeedPassword` 弹窗；排队 M1 不做（满员直接提示）。
3. 阵营选择：`H_t − H_other ≤ 2` 判定；不可选时显示原因（"该队人数较多"）并禁用；自动分配按钮；请求超时 5 s 报错。
4. 人数变化实时刷新（1 Hz，`PlayerRoster`/gateway 推送），选择期间被禁时 toast"该阵营已不可选"。
5. 全部操作键盘可达：`↑/↓/Enter//`；阵营页 `←/→/Enter/A/Esc`。
### 接口与数据
```ts
export interface ServerInfo { id: string; name: string; map: string; mode: string; humans: number; bots: number; cap: number; phase: string; pingMs?: number; region: string; version: number; locked: boolean; compatible: boolean }
export interface FactionState { team: 0|1; humans: number; bots: number; cap: 50; state: 'open'|'full'|'unbalanced'|'cooldown'; reasonKey?: string }
```

## 不做什么
- 不做部署界面（M1-056）、队列/排队、OAuth 登录、社区服标记（M4-023 完善模式/地图过滤）。
- 不做 gateway 本身（M1-028）与 server 的阵营判定（M1-026 的 C0 处理器）。
- 不做服务器详情页的完整规则展示、好友邀请链接（M4+）。
- 不做 Assault/Skirmish 筛选（M3/M4）。

## 验收标准
1. 列表刷新耗时 ≤ 1 s（gateway 本地，Playwright 计时从触发到渲染完成；INDEX 指标）。
2. 阵营失衡时受限选项被禁用并显示原因：构造 `H_us − H_ger = 3` 的服务器状态，US 卡片 `disabled` 且带 `reasonKey`（vitest + Playwright）；差 = 2 时可选。
3. 一次完整流程"主菜单→选服务器→加入→选阵营→进入部署界面"仅用键鼠完成（Playwright 脚本，不使用 `page.evaluate` 直接改状态）。
4. 版本不兼容的服务器加入按钮禁用并有 tooltip 原因（断言）。
5. 请求超时 5 s 场景：显示错误并可重试（fake timers）。
6. 浏览器页 DOM 节点 ≤ 1 500；50 行列表渲染 ≤ 5 ms（`?uiperf=1`）。
7. en/zh-Hans 键全部存在（`i18n:check`）。

## 测试要求
- 单元测试：筛选/排序；阵营可选性公式边界（差 2/3、满员 50、冷却）；错误码映射。
- 集成/无头测试：gateway mock + Playwright 完整加入流程。
- 性能基准：列表渲染与刷新耗时入 `reports/m1-browser.json`。
- 回归：M1-054 主菜单快速加入仍可用。

## 性能预算
- UI_SPEC §8.1：菜单类 DOM ≤ 1 500 节点；UI 总 CPU ≤ 1.5 ms 均值；本任务无渲染预算新增。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：UI_SPEC §4.4 与 GAMEDESIGN §2.4 数值冲突已登记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 冲突项（见背景）：平衡阈值与换队冷却以 GAMEDESIGN §2.4 为准。
- ping 测量方法在 UI_SPEC 中标"未核实"；M1 用 gateway HTTP 往返估计，WT 探测留 M2。
