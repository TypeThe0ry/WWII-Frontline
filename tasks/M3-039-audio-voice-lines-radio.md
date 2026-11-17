# M3-039 音频：战场人声与无线电

## 元数据
- 里程碑：M3
- 工作流：audio
- 规模：M
- 前置依赖：M2-026, M1-062
- 素材需求：G-VOX-*; H-RAD-001（缺失则静音占位）；素材状态：placeholder（静音）
- 引用文档：ARCHITECTURE §12.4; ARCHITECTURE §12.5; ARCHITECTURE §14.1; GAMEDESIGN §16.1; ASSET_GAPS §3.3

## 背景
战场人声（受击、倒地、喊话/指挥口令）与无线电效果让战场有"人"的存在感。语音聊天本身（LiveKit）在 M2-025/M2-026；本任务是**游戏事件触发的预录人声**与**无线电音频处理链**。
关键规则：
- 无线电滤波（ARCH §12.5）：小队/指挥频道不空间化，加"无线电"滤波 = 带通 300–3 400 Hz + 轻失真；近距离（prox）走 HRTF（refDistance 2、maxDistance 50、rolloff 1.5）且不加无线电滤波。
- 车组频道不空间化，轻混响（ARCH §14.1）。
- 复音与合并（ARCH §12.4）：同类声音 50 ms 内合并；人声事件同类 50 ms 内合并为一个（如多人同时被击倒的呻吟）。
- 频道定义（GD §16.1）：Proximity 50 m；Squad；Command（Commander + Officer 说话；TC 与 Recon 队长仅听）；Vehicle；倒地者仅走 Proximity（−12 dB、低通）。
- 缺失素材：静音占位 + 每 ID 每会话一条日志（ASSET_GAPS §3.3）。无线电噪声底噪不属于素材，由代码程序化生成（白噪声经带通 + 低增益 −42 dBFS）【待调参】。

## 具体内容
### 要创建/修改的文件
- `client/src/audio/voice/VoiceLineBank.ts` —— 事件 → 人声 ID → 变体选择（兵种/阵营/情绪）；每类事件冷却（同一说话者 ≥ 2 s）。
- `client/src/audio/voice/VoiceLineEvents.ts` —— 触发映射：受击（轻/重）、倒地、被救起、投弹预警"Grenade!"、换弹/弹尽、指挥口令（Attack/Defend/Move/Need Medic/Need Ammo 等 Ping，UI_SPEC §4.8）。
- `client/src/audio/voice/VoiceMerge.ts` —— 同类 50 ms 窗口合并：保留最近/最响的 1 个，其余计入"群发声"增益 +1.5 dB 上限 +3 dB【待调参】。
- `client/src/audio/radio/RadioChain.ts` —— 无线电滤波链：`BiquadFilter highpass 300 Hz → lowpass 3 400 Hz → WaveShaper (轻失真) → 底噪混合`；开关"咔哒"声（H-RAD-001，缺失静音）。
- `client/src/audio/radio/ChannelRouting.ts` —— 频道 → 链路：squad/command → 无线电链（不空间化）；crew → 内话链（轻混响）；prox → HRTF Panner（无线电链旁路）。
- `client/data/audio_events/voice_lines.json`
- `client/tests/audio/voice.spec.ts`
### 要实现的行为
1. Ping/命令的口令触发由 UI 事件（`ui.radial.*`）经事件总线到达；自己发出时本地立即播放，不等服务器。
2. 他人人声来自 `Shot/Hit/Down/Revive/Order` 等网络事件，使用说话者位置的 HRTF 3D；> 50 m 的普通人声不播放（节省复音）。
3. 无线电频道：说话开始/结束各播放一次开关声（缺失静音）；激活时叠加底噪，渐入 50 ms、渐出 80 ms。
4. 倒地者的呻吟走 Proximity 且 −12 dB + 低通（与 GD §16.1 一致）。
5. 合并：窗口 50 ms 内同类事件只发一个 voice（见 VoiceMerge）。
6. 人声总复音上限：High ≤ 12（占 64 的约 19%）【待调参】，超出按距离偷最远者。
### 接口与数据
```ts
type VoiceEvent = { kind: 'hit'|'down'|'revive'|'order'|...; speaker: EntityId; faction: 'us'|'ger'; role: RoleId; severity?: number };
```

## 不做什么
- LiveKit 语音聊天本体与订阅（M2-025/M2-026，车组/指挥频道订阅见 M3-040）。
- 语音转写（R10 首发不做）。
- 真实人声/无线电素材接入（M3-043）；字幕/声音可视化（UI_SPEC §7.2，后续 UI 任务）。

## 验收标准
1. 合并规则生效：50 ms 内触发 10 个同类 `hit` 人声事件，实际创建的 voice ≤ 1（+群发声增益 ≤ +3 dB），用例通过；跨 50 ms 的两个事件各 1 个 voice。
2. 无线电频率响应：向 `RadioChain` 输入 100 Hz/1 kHz/8 kHz 正弦，OfflineAudioContext 输出能量分别 ≤ −20 dB/≥ −3 dB/≤ −20 dB（相对 1 kHz），失真 THD ≤ 10%（轻失真）。
3. 频道一致性：squad/command 频道声源经过无线电链且 Panner 为 null；prox 频道声源 Panner 为 HRTF 且旁路无线电链；crew 为内话链（节点图断言，3 种用例）。
4. 倒地人声 −12 dB ±1 dB 且低通 ≤ 4 kHz（OfflineAudioContext 测）。
5. 全缺失素材：0 未捕获错误，缺失日志每 ID 1 条；底噪仍可生成。
6. 人声 voice 数 ≤ 12（High）在 50 人同时喊话的压力测试中成立；音频主线程 ≤ 0.5 ms/帧。

## 测试要求
- 单元：合并逻辑、冷却、复音偷取、频道路由。
- 集成：Playwright 播放 `?vis_scene=squad_fight` 的事件脚本，读取音频调试面板。
- 性能：音频主线程耗时采样。
- 回归：M2-026 语音测试不变。

## 性能预算
- ARCH §13.5.1 音频主线程 0.5 ms；§12.4 复音 64/48/32；人声子预算为本任务提议。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 德语/英语口令的录音需求（G-VOX-*）依赖采购（H-070）与 GAP 决定（H-069）；缺失时本任务仍可完成。
- 无线电开关声与指挥频道提示音的**时序**对标属 AU-10（REF §4.12，M3-052）；本任务先用初值（开 30 ms/关 50 ms 提示）【待调参】。
- 为保证"频道一致"，不要在 prox 链上加无线电滤波，否则会与 §14.4 的遮挡低通叠加失真。
