# M2-026 语音：客户端接入与空间化

## 元数据
- 里程碑：M2
- 工作流：voice
- 规模：L
- 前置依赖：M2-025, M1-062, M1-040
- 素材需求：H-RAD-001（可选）；素材状态：placeholder
- 引用文档：ARCHITECTURE §14.4; ARCHITECTURE §12.5; ARCHITECTURE §14.5; ARCHITECTURE §14.1; GAMEDESIGN §16.1; ARCHITECTURE §12.6

## 背景
客户端接入 LiveKit（`livekit-client`，版本钉到 exact，npm 包与 M2-024 的服务端版本兼容性在任务中核对），实现 PTT/VOX、近距 3D 空间化、无线电滤波与失败降级。**不做转写（R10）**。
规则要点：
- 连接：用 M2-024 的 `POST /voice/token` 获取 url+token；`autoSubscribe=false`；发布 1 条麦克风轨（Opus 24 kbps、DTX 开启、`maxBitrate` 32 kbps）；token 到期前 ≥ 60 s 续签。
- 近距离轨道（ARCH §14.4/§12.5）：`MediaStreamAudioSourceNode → PannerNode`（`panningModel='HRTF'`、`distanceModel='inverse'`、`refDistance=2`、`maxDistance=50`、`rolloffFactor=1.5`），位置取该玩家实体的**插值位置**（来自 `game` 状态，与语音网络解耦）；遮挡走与 SFX 同一遮挡低通总线（M2-045 提供，此处预留接口，缺失时直通）。
- 小队/指挥/车组频道：不空间化，加入"无线电"滤波（带通 300–3 400 Hz + 轻失真）。
- 键位（GD §16.1，可改，最终以 UI_SPEC 为准）：近距离按住 `V`；小队按住 `B`；指挥按住 `N`；车组在载具内按 `V`。支持 PTT 与 VAD；`VoiceState{ptt,channel_pref}` 经 C0 上报（ARCH §6.8.1，权威仍由服务器推导）。
- 同时收听上限 8（服务器控制）；逐玩家本地静音；输入灵敏度；倒地者音量 −12 dB + 低通（GD §16.1）。
- 失败降级（ARCH §14.5）：LiveKit 不可用或麦克风权限被拒 → **游戏正常进行，仅禁用语音**；UI 明确提示；指数退避重连，上限 60 s，**不重试风暴**。
- 延迟参考（ARCH §12.6）：`AudioContext({latencyHint:'interactive'})`，WebAudio 输出延迟 10–40 ms 起、蓝牙可 > 150 ms，不承诺更低。
素材：PTT 开/闭声与静电噪声需求 ID `H-RAD-001`（可选）；缺失时按 README §5.3：返回短静音 `AudioBuffer`，每个缺失 ID 每会话只打一条日志，**不用其它层顶替、不放提示音**。

## 具体内容
### 要创建/修改的文件
- `client/src/audio/voice/connection.ts` —— 连接/重连/续签/状态机（`idle|connecting|connected|degraded|failed`）
- `client/src/audio/voice/mic.ts` —— 麦克风权限、PTT/VAD、输入电平与灵敏度
- `client/src/audio/voice/spatial.ts` —— 近距轨道 → Panner 图；位置每帧更新；上限 8 路复用节点池
- `client/src/audio/voice/radio.ts` —— 无线电滤波链（BiquadFilter 带通 + WaveShaper 轻失真）、PTT 开/闭音（素材缺失静音）
- `client/src/audio/voice/state.ts` —— 暴露只读状态（连接状态、谁在说话、静音表）给 `ui`（经事件总线）
- `client/src/net/voiceState.ts` —— 发送 `VoiceState`（C0）；`client/src/audio/voice/__tests__/*.test.ts`（vitest）
- `tools/voice-smoke/latency.html` —— 端到端延迟测量页（局域网，两标签页或两机）
### 要实现的行为
1. 连接生命周期与降级：失败不抛到游戏主循环；状态变化通过事件总线 `voice.status` 通知 UI（文案走 i18n key）。
2. 订阅响应：服务器订阅变更后收到 `TrackSubscribed/Unsubscribed`，按轨道所属玩家的频道类别（近距/小队/指挥/车组，来自 `PlayerRoster` 与订阅元数据）挂接对应图。
3. 空间化：HRTF，参数如上；说话者位置 ≥ 60 m 时断开 Panner 以省 CPU（服务器此时已退订）。
4. 本地静音：逐玩家 `GainNode=0`；举报走既有 `Report`（M2-043）。
5. 无线电滤波：频道类轨道经 `radio` 链；开始/结束说话触发 PTT 提示音（素材缺失则静音并日志）。
6. 资源回收：离开对局销毁所有节点，`MediaStream` 全部 stop；测试断言无节点泄漏。
### 接口与数据
- 事件：`voice.speaking{playerId,channel}`、`voice.status{state,reasonKey}`、`voice.mute{playerId,bool}`。

## 不做什么
- 订阅集合计算（M2-025）；token/部署（M2-024）；聊天/语音 UI 面板与设置页（M2-040/M2-041）。
- 转写（R10）、语音录制、音效素材接入（M2-047）。
- 遮挡低通的具体实现 → M2-045（此处只调用其总线）。

## 验收标准
1. 小队语音端到端延迟 < 400 ms（局域网，`tools/voice-smoke/latency.html`：发送端播放 1 kHz 测试音并打时间戳，接收端 `AnalyserNode` 检出，30 次取 P95）；需两台设备/两标签页，缺失时标 `未测` → `partial`。
2. LiveKit 不可用时游戏不受影响：停止 LiveKit 容器，客户端帧时间 P95 变化 ≤ 0.5 ms（`performance.now` 采样 60 s），UI 显示降级提示，重连间隔序列为 1,2,4,…,60 s（单测断言上限 60 s、无风暴）。
3. 麦克风权限被拒：游戏正常、`voice.status=degraded(mic_denied)`，不重复弹权限请求（单测 + 手测）。
4. 空间化参数断言：Panner `refDistance=2`、`maxDistance=50`、`rolloffFactor=1.5`、`distanceModel='inverse'`、`panningModel='HRTF'`（vitest 读取节点属性）；说话者在 ±X 方向时左/右声道 RMS 差 ≥ 6 dB（离线渲染 `OfflineAudioContext` 测试，近似验证 HRTF 方位）。
5. 8 路同时订阅 CPU：Chrome Performance 采样，音频线程占用 ≤ 3%（参考机，记录型号）；超出则减少滤波链。
6. 离开对局 3 次循环后 WebAudio 节点数回到基线（`AudioContext` 节点计数测试）。
7. 素材缺失路径：`H-RAD-001` 缺失时无异常、只 1 条日志/会话。

## 测试要求
- 单元测试：vitest：状态机、退避序列、静音、节点池、事件总线；
- 集成：headless 浏览器（Playwright Chromium，假麦克风 `--use-fake-device-for-media-stream`）连 compose 起的 LiveKit，检查发布/订阅；
- 性能基准：音频线程占用记录；
- 回归：M1-062 的音频总线测试不回归。

## 性能预算
遵守 ARCH §13.5 的 JS 主线程预算（不得新增 > 0.2 ms/帧的语音相关主线程开销，Chrome Performance 记录）；`AudioWorklet` 不新增。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 浏览器差异（Safari 的 WebRTC/PannerNode HRTF 行为）未核实，CI 只保证 Chromium（ARCH §20 第 6 项）。
- 键位与 UI 最终以 UI_SPEC 为准；若 UI_SPEC 与 GD 默认键位冲突，按 UI_SPEC 并在 PLAN_ISSUES 记录。
- 语音数据合规与举报无语音证据的说明在设置页（M2-041）展示（H-082 的隐私政策为 M4）。
