# M2-040 UI：文字聊天与语音状态

## 元数据
- 里程碑：M2
- 工作流：ui
- 规模：M
- 前置依赖：M1-057, M2-026, M2-019
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.15; UI_SPEC §5.2; GAMEDESIGN §16.1; GAMEDESIGN §16.2; ARCHITECTURE §14.5; ARCHITECTURE §19.3

## 背景
UI_SPEC §4.15：聊天窗口（左下，战斗态淡出，展开显示 100 条）、频道标签（颜色+形+缩写）、自己 PTT 指示（准星下方右侧，含输入电平条）、说话者列表（右上）、设备状态（无麦克风/权限被拒/设备断开/服务器连接失败）、静音/屏蔽名单、速率限制提示、系统消息过滤。频道（§4.15.1）：Proximity（`B`，≤ 50 m，空心圆）、Squad（`V`，实心圆）、Command（`N`，六边形）、Crew（车内自动，`N` 优先，方形）、Team（文字）、All（文字，默认关闭）。默认 PTT 键以 UI_SPEC §5.2 为准（`V` 小队、`B` 近距、`N` 指挥/车组）。
规则：GAMEDESIGN §16.2 文字聊天 All/Team/Squad/Command 四频道，**每人每 8 s 最多 5 条、每条 ≤ 200 字符**，敏感词过滤（客户端提示 + 服务器强制），聊天记录服务器保留 24 h；§16.1 语音规则（倒地者仅走 Proximity 且受限；KIA 可听 Squad 不可说；同时收听上限 8 路）。M2-026 提供 `livekit-client` 接入、PTT、空间化与失败降级；M2-019 提供聊天频道规则消息。ARCH §14.5：LiveKit 不可用时游戏不受影响，文字聊天仍可用。
安全：玩家文本用文本节点渲染，禁止 `innerHTML`（UI_SPEC §4.15.5、README §5.8）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/chat/ChatWindow.tsx`、`ChatInput.tsx`、`ChannelTabs.tsx` —— 聊天窗口/输入/频道标签；`Enter` 打开、`Tab`（输入态）循环频道、`Esc` 关闭；打开时暂停游戏键位。
- `client/src/ui/chat/chat_store.ts` —— 消息队列（上限 100 条，DOM 回收）、系统消息过滤、重复折叠、静音名单过滤。
- `client/src/ui/voice/VoiceIndicators.tsx` —— PTT 指示、说话者列表（订阅 M2-026 的 `voiceState` 信号，4–10 Hz）、设备/连接状态点（红色 + 叉形，重试按钮）。
- `client/src/ui/voice/MuteList.tsx` —— 个人静音/屏蔽管理（与记分板玩家菜单 M2-038 共享 `mute_store`）。
- `client/src/game/chat_client.ts` —— 发送 `ChatSend{channel,text}`、接收 `ChatMsg`、`ChatRejected{reason,retry_after_ms}`（M2-007/M2-019 消息）。
- `client/src/ui/chat/rate_limit.ts` —— 客户端预检查（与服务器同数值，来自 `/data/comms.json`：`chat.max_per_window=5`、`window_s=8`、`max_len=200`），仅用于提示。
- `client/src/i18n/{en,zh-CN}/chat.json` —— `ui.chat.*`、`ui.voice.*`。
### 要实现的行为
1. 发送：超长截断提示；速率超限时显示"发送过快"并保留输入文本；服务器 `ChatRejected` 的原因枚举（rate_limited、profanity、muted_by_admin、no_permission）都有 i18n 文案。
2. 权限：Command 频道仅对 Commander + Officer 显示可发；All 默认关闭（设置项开启后才显示）；Dead/KIA 状态按 GAMEDESIGN §16.1 限制发言频道。
3. 麦克风权限：首次使用 PTT 时请求（用户手势）；拒绝 → "麦克风被禁用"并降级只听；默认只有 PTT，不做自动检测（UI_SPEC §4.15.5）。
4. 语音失败降级：LiveKit 连接失败/断开时状态点变红 + toast，文字聊天与游戏不受影响；重试按钮调用 M2-026 `reconnect()`。
5. 系统消息（据点占领、建造、能力使用）走独立队列并可过滤。
6. 聊天窗口战斗态渐隐（不拦截鼠标，Pointer Lock 保持）；发送成功后保持 Pointer Lock。
### 接口与数据
```ts
export type ChatChannel = 'all' | 'team' | 'squad' | 'command';
export interface ChatMsg { id: number; channel: ChatChannel; senderId: number; senderName: string; text: string; tsMs: number; isSystem?: boolean }
export interface VoiceUiState { transmitting: 'squad' | 'proximity' | 'command' | 'crew' | null; level: number; speakers: { playerId: number; channel: string }[]; device: 'ok' | 'no_mic' | 'denied' | 'disconnected' | 'server_failed' }
```

## 不做什么
- 不做语音传输、空间化、LiveKit 订阅管理（M2-024/M2-025/M2-026）。
- 不做语音转写/字幕化（R10，首发不做）。
- 不做聊天举报表单本体（M2-043）与管理员禁言工具（M2-023）。
- 不做设置页中的语音设备选择（M2-041）。

## 验收标准
1. 速率限制提示：连续在 8 s 内发送第 6 条消息时，客户端在 ≤ 100 ms 内显示"发送过快"提示，且服务器 `ChatRejected{rate_limited}` 路径也显示同一提示（headless 联调；阈值 5 条/8 s 取自 `/data/comms.json`）。
2. 长度：201 字符输入被截断/拒绝，界面提示；`vitest` 覆盖 0/1/200/201 字符边界。
3. 语音失败降级：模拟 LiveKit 连接失败（`M2-026` 的 mock），状态点在 ≤ 1 s 变为红色 + 叉形，toast 出现，文字聊天仍可发送成功（Playwright）。
4. 麦克风被拒：UI 显示"麦克风被禁用"，PTT 按下不抛错，说话者列表仍显示他人（Playwright 授权拒绝场景）。
5. 100 条上限：连续灌入 300 条，DOM 消息节点 ≤ 100（单测）；聊天窗口 DOM 总节点 ≤ 200。
6. XSS：注入 `<img src=x onerror=alert(1)>` 的消息以纯文本显示，无脚本执行（Playwright 断言）。
7. i18n 键完整性（en/zh-CN）。

## 测试要求
- 单元测试：速率限制预检查、频道权限矩阵、聊天队列回收、静音过滤、i18n 完整性。
- 集成/无头测试：headless-client 发送/接收聊天；LiveKit mock 与失败场景；Playwright 键位（Enter/Tab/Esc/PTT）。
- 性能基准：UI 采集 `?uiperf=1`。
- 回归：HUD 战斗态节点数 ≤ 350。

## 性能预算
- UI_SPEC §8.1：UI CPU ≤ 1.5 ms 均值；说话者列表更新 ≤ 10 Hz；每秒 DOM 写入 ≤ 400。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **已裁定**：聊天限速以 GAMEDESIGN §16.2（每人每 8 s 最多 5 条）为准，INDEX 已改；数值取 `/data/comms.json`。PTT 键位以 UI_SPEC §5.2 为准（小队 V、近距 B、指挥/车组 N）。
- **键位不一致**：GAMEDESIGN §16.1 写近距 `V`/小队 `B`，UI_SPEC §5.2 为小队 `V`/近距 `B`；GAMEDESIGN 明确"具体见 UI_SPEC"，按 UI_SPEC 实现。
- 敏感词表初稿属人类待办（H-075）；服务器侧过滤为准，客户端仅提示。
