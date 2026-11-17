# M1-062 音频：引擎核心

## 元数据
- 里程碑：M1
- 工作流：audio
- 规模：L
- 前置依赖：M1-006
- 素材需求：无（静音占位）；素材状态：placeholder（缺失需求 ID 返回短静音 `AudioBuffer`，ASSET_GAPS §3.3）
- 引用文档：ARCHITECTURE §12.1; ARCHITECTURE §12.4; ARCHITECTURE §12.6; ARCHITECTURE §12.7; ASSET_GAPS §3.3; ARCHITECTURE §4.3.9

## 背景
`src/audio` 只读 `game` 状态并订阅事件总线，**不 import `render`/`net`**（ARCHITECTURE §4.3.9）。本任务建立 WebAudio 图与资源层，使 M1-063（枪声/脚步/命中）、M2-045/046（遮挡、混响、载具）直接在其上叠加。
图结构（§12.1）：Sources（枪声层近/中/远、脚步/装备/载具、爆炸/炮声、环境床、UI、语音轨道）→ HRTF `PannerNode` →（AudioWorklet 遮挡/压制，M2）→ SFX 总线；混响总线（Convolver，M2）；环境总线、UI 总线、语音增益 → 主总线（压缩器/限幅）→ `destination`。复音（§12.4）：最大并发 64（High）/ 48（Medium）/ 32（Low），按"响度估计 × 重要性 / 距离"排序，超出偷最低者；同类（同武器）每 50 ms 内最多 4 个，超出合并为"群发声"；长音（载具引擎/环境床）用 `AudioBufferSourceNode` loop + 参数自动化，不创建 MediaElement。延迟（§12.6）：`AudioContext({ latencyHint: 'interactive' })`，用 `outputLatency` 调试显示；本地开火音在 `fire.local` 立即触发。格式（§12.7，D-AUD）：Opus 为主（WebM 容器内数据在 `.fpak` 中，§11.8）、AAC 兜底；启动时用 `decodeAudioData` 对内置探测样本实测探测；预解码到 `AudioBuffer`：短音（< 3 s）常驻，长音流式。缺失素材（ASSET_GAPS §3.3）：返回短静音 AudioBuffer（0.1 s），**每个缺失需求 ID 每会话只打一条** `[asset-missing] audio {需求ID} requested by {事件名}`（生产构建只计数上报），不用其它层顶替，不放提示音。

## 具体内容
### 要创建/修改的文件
- `client/src/audio/context.ts` —— `AudioContext` 创建/恢复（等待首次用户手势，由启动页 M1-054 的"点击进入"触发 `resume()`）、`outputLatency` 调试、`visibilitychange` 处理。
- `client/src/audio/graph.ts` —— 总线（SFX/Ambient/UI/Voice）+ 主总线（`DynamicsCompressor` + 限幅），音量设置（主/音效/UI/语音/音乐，来自 `settings.changed`）。
- `client/src/audio/codec-probe.ts` —— `decodeAudioData` 探测样本（内置极短 Opus/AAC 探测数据，体积 < 2 KB）→ 选择包类型；结果缓存。
- `client/src/audio/bank.ts` —— 音频库：按需求 ID/事件 ID 查询 `AudioBuffer`；从 fpak 音频块读取与解码（M1-003/M1-004 格式）；短音常驻、长音流式；缺失→静音占位+日志去重。
- `client/src/audio/voices.ts` —— 复音池：`play(req)`、响度估计/优先级、偷声、同类 50 ms 合并（≤ 4）、`stopAll`。
- `client/src/audio/panner.ts` —— HRTF `PannerNode` 池化与监听器更新（每帧从相机位置/朝向），`refDistance`、`rolloffFactor` 按类别配置（来自 `/data/audio/*.json` 配置，非硬编码）。
- `client/src/audio/events.ts` —— 订阅 `settings.changed`、`quality.changed`（复音上限）；对外 `AudioApi`。
- 测试：`client/test/audio/*.test.ts`（vitest + Web Audio mock / `OfflineAudioContext`）。
### 要实现的行为
1. 首次手势前不创建发声；`resume()` 成功后进入 `running`；失焦时按设置静音。
2. 复音上限由画质档决定（64/48/32）；同类 50 ms 内 > 4 个合并为群发声（使用同一素材的"群发"变体；缺失时降低增益合并）。
3. 探测：启动时探测 Opus 与 AAC，选择能解的包；两者都不能则全部走静音占位并报警（不崩溃）。
4. 缺失素材：`bank.get(id)` 返回静音缓冲（0.1 s），日志去重；开发构建可开关 "缺失音效 beep"，默认关。
5. 监听器：每帧 O(1) 更新；声源 3D 位置更新由调用方节流（每 2 帧）。
6. 零主线程长任务：解码在 `decodeAudioData`（浏览器线程）异步；预热不阻塞首屏。
### 接口与数据
```ts
export interface PlayRequest { id: string /* 需求ID或事件ID */; bus: 'sfx'|'amb'|'ui'|'voice'; pos?: Vec3; gain?: number; rate?: number; priority: number; loop?: boolean; groupKey?: string }
export interface AudioApi { resume(): Promise<void>; play(r: PlayRequest): VoiceHandle | null; setListener(pos: Vec3, fwd: Vec3, up: Vec3): void; setVolume(bus: Bus, v: number): void; stats(): { active: number; stolen: number; missingIds: number } }
```

## 不做什么
- 不做枪声分层/脚步/命中逻辑（M1-063）、遮挡低通/压制 Worklet（M2-045）、混响/环境/载具声（M2-046）、语音（M2-026）。
- 不做素材打包/响度归一（M1-004）与真素材接入（M1-065）。
- 不做动态范围压缩的设置 UI（M2-041）。
- 不用任何版权不明的音频凑数。

## 验收标准
1. 64 个并发声音时主线程音频开销 < 1 ms/帧（Playwright + `performance.measure`，参考机；INDEX 指标）。
2. 缺失需求 ID 每会话仅 1 条日志：对同一缺失 ID 请求 1 000 次，日志条数 = 1（vitest 计数；INDEX 指标）。
3. 复音上限：请求 200 个声音，活动数始终 ≤ 档位上限（64/48/32），偷声计数 > 0（vitest，三档各一）。
4. 同类合并：同一武器 50 ms 内 10 次触发，实际播放 ≤ 4 个 + 1 个群发声（vitest）。
5. 编码探测：mock 环境下 Opus 不可解时选择 AAC；两者皆不可解时静音占位且无异常（vitest）。
6. `outputLatency` 与 `baseLatency` 在调试叠层可见（Playwright 断言元素存在）。
7. `audio` 目录无对 `render`/`net` 的 import（eslint 规则通过）。

## 测试要求
- 单元测试：优先级排序/偷声、合并窗口、静音占位、探测分支、设置→音量映射、监听器更新。
- 集成/无头测试：Playwright 触发 200 个合成事件（含缺失 ID），采集主线程耗时与日志。
- 性能基准：音频 CPU 开销写入 `reports/m1-audio-core.json`。
- 回归：启动页 `resume()` 流程（M1-054）不被破坏。

## 性能预算
- ARCHITECTURE §13.5.1：音频主线程部分 ≤ 0.5 ms/帧（High）；INDEX 要求 64 并发 < 1 ms 为上限验证；ARCHITECTURE §13.3 内存预算内（音频 ≤ 18 MB Medium，M1-065 门禁）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 处理（静音）；缺失素材需求 ID 已列出
- [ ] 文档同步：探测样本格式与 `/data/audio` 配置 schema 写 PLAN_ISSUES"细化"
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 浏览器自动播放策略：必须由首次用户手势恢复上下文（UI_SPEC §4.1.4 的"点击继续"）。
- Opus 在 Safari 的 `decodeAudioData` 支持程度未核实（ARCHITECTURE §12.7 的探测机制即为此）；AAC 兜底包体积加倍需 M1-004/M1-005 管线配合。
- 蓝牙耳机 > 150 ms 的输出延迟无法规避（§12.6），只显示不补偿。
