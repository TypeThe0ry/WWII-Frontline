# M2-045 音频：遮挡低通与压制效果

## 元数据
- 里程碑：M2
- 工作流：audio
- 规模：M
- 前置依赖：M1-063, M1-017
- 素材需求：C-BUL-005（子弹擦过/压制相关音，缺则静音）；素材状态：placeholder
- 引用文档：ARCHITECTURE §12.1; ARCHITECTURE §12.3; ARCHITECTURE §12.4; ARCHITECTURE §12.6; GAMEDESIGN §8.3; GAMEDESIGN §8.1; GAMEDESIGN §8.2; REFERENCE_MEASUREMENTS §4.12

## 背景
ARCHITECTURE §12.3 AudioWorklet（128 帧/48 kHz = 2.67 ms 量化）负责：
- **遮挡低通**：参数 `cutoff_hz`（1.5 kHz–20 kHz），主线程 **10 Hz** 更新（4 条射线 + 材质，来自 `sim-wasm` 的 `los_samples`），对最近 8 个可听声源逐个更新；
- **压制效果**：近距擦过（< 3 m）弹丸触发：主总线 −6 dB 闪避 400 ms + 高通 + 耳鸣音（4 kHz 正弦淡出，−30 dB 起）；
- **爆炸冲击**：低频 thump + 2 s 听力模糊（低通 1.2 kHz 渐开）。
不支持 AudioWorklet 的环境降级为 `BiquadFilterNode`。GAMEDESIGN §8.3：压制值 `S`（0–100，服务器权威，M1-017 `sim-suppression` 下发到 `self_state`），`e = S/100`，听觉效果"近距离语音与环境音降低 `3 dB·e`（AudioWorklet 低通）"；`S ≥ 5` 时随机触发子弹擦过声，音量 ∝ s。M1-062/M1-063 已有音频图、枪声分层、压制简版滤波。本任务把简版升级为 Worklet 实现，并接入遮挡射线。
测量项 AU-06（墙体/室内遮挡的衰减，REFERENCE_MEASUREMENTS §4.12）、AU-07（爆炸可闻距离）、AU-09（压制听觉效果的起止与时长）可能在 M2-054 回填，数值以 `/data/audio/*.json` 为准。

## 具体内容
### 要创建/修改的文件
- `client/src/audio/worklets/occlusion_processor.ts` —— AudioWorkletProcessor：每声源独立双二阶低通状态；参数 `cutoff`（k-rate，平滑 50 ms）；最多 8 路。
- `client/src/audio/worklets/suppression_processor.ts` —— 主总线闪避 + 高通 + 耳鸣合成 + 爆炸模糊低通；参数由主线程消息驱动。
- `client/src/audio/occlusion_probe.ts` —— 主线程 10 Hz：对最近 8 个可听声源调用 `sim-wasm` 的 `los_samples(listener, source)`（4 条射线 + 材质），映射为 `cutoff_hz`（材质表来自 `/data/audio/occlusion.json`：开阔 20 kHz → 薄木 ~8 kHz → 砖/石 ~3 kHz → 厚墙 1.5 kHz，初值【待调参 → REFERENCE_MEASUREMENTS §4.12 AU-06】）。
- `client/src/audio/suppression_audio.ts` —— 订阅 `self_state.suppression`（S）与事件 `BulletWhiz{dist}`、`Explosion{pos,power}`；计算 `e = S/100`；触发闪避/耳鸣/模糊；近距离语音与环境总线 `−3 dB·e`。
- `client/src/audio/fallback_biquad.ts` —— 无 AudioWorklet 的降级实现。
- `data/audio/occlusion.json`、`data/audio/suppression.json`、对应 schema —— 参数（闪避 −6 dB/400 ms；耳鸣 4 kHz、起始 −30 dB；爆炸模糊 1.2 kHz/2 s；擦过触发距离 < 3 m）状态 `initial`。
- `client/src/audio/graph.ts`（修改 M1-062）—— 在 Panner 之后插入遮挡 Worklet 节点，在 SFX/AMB/语音总线上插入压制 Worklet。
### 要实现的行为
1. 每个声源（枪声/爆炸/脚步/载具）在 Panner 之后经遮挡 Worklet；超出 8 个可听声源时，超出者使用最近一次 cutoff 或默认 20 kHz（不抖动）。
2. cutoff 变化平滑（一阶 50 ms），避免拉链噪声；`los_samples` 失败时保持上次值。
3. 压制：S 驱动 `e`；`BulletWhiz` 近距（< 3 m）触发闪避 + 高通；耳鸣淡出（4 kHz，−30 dB 起，时长取 `/data`）；"被钉死"（S ≥ 85）无额外音频逻辑，仅 UI。
4. 爆炸冲击：按距离/威力触发低频 thump（缺素材则只做滤波效果，不放声音）+ 2 s 低通 1.2 kHz 渐开/渐关。
5. 缺失素材（C-BUL-005 等）：按 ASSET_GAPS §3.3 返回静音缓冲，每需求 ID 每会话仅 1 条日志；**滤波效果仍然生效**（可用示波/测试信号验证）。
6. 降级：`AudioWorklet` 不可用 → `BiquadFilterNode`，功能等价但精度较低。
### 接口与数据
```ts
export interface OcclusionParams { sourceId: number; cutoffHz: number }
export interface SuppressionState { e: number /*0..1*/; ducking: boolean; tinnitusLevelDb: number; blastBlur: boolean }
// 主线程 → Worklet: port.postMessage({type:'cutoff', id, hz}) / {type:'supp', e, duck, blast}
```

## 不做什么
- 不做混响总线与环境床、载具声（M2-046）；爆炸/炮声/空袭的完整听感（M3-038）。
- 不做语音的无线电滤波与近距离 HRTF（M2-026）。
- 不做实际素材采购与替换（M2-047）。
- 不改服务器压制数值计算（M1-017）。

## 验收标准
1. Worklet 处理耗时：8 路遮挡 + 压制处理器在 `OfflineAudioContext` 离线基准中，每 128 帧块平均处理 ≤ 0.3 ms（参考机，`performance.now` 包裹渲染循环统计，10 万块）。
2. 低通正确性：对 1 kHz/8 kHz 正弦与白噪声测试信号，cutoff = 1.5 kHz 时 8 kHz 衰减 ≥ 24 dB、1 kHz 衰减 ≤ 3 dB（离线 FFT 测试，误差 ±2 dB）。
3. 10 Hz 更新：`occlusion_probe` 每秒产生 10 ± 1 次更新（假定时器单测）；cutoff 阶跃响应 50 ms 内到 63% 值（单测）。
4. 压制与服务器同步：`self_state.suppression` 从 0 → 80 的序列下，音频侧 `e` 在 ≤ 1 个快照间隔（50 ms）内跟随，误差 ≤ 0.02（headless 联调录制）；近距离语音/环境总线增益 = `−3·e` dB（±0.3 dB）。
5. 擦过触发：`BulletWhiz{dist < 3 m}` 触发闪避，主总线增益在 ≤ 50 ms 内降至 −6 dB（±1 dB），400 ms 后恢复到 ≥ −0.5 dB（离线测量）。
6. 爆炸模糊：触发后 2 s 内低通 1.2 kHz 渐开/渐关，曲线与 `/data` 一致；耳鸣 4 kHz 峰值电平 −30 dB（±2 dB）。
7. 降级路径：禁用 AudioWorklet（模拟）时效果仍工作，功能测试全部通过（精度阈值放宽到 ±4 dB 并在 STATUS 记录）。
8. 缺失素材 C-BUL-005 时仅 1 条日志，滤波效果照常（单测）。

## 测试要求
- 单元测试：材质→cutoff 映射；`e` 计算；耳鸣/闪避时序（假定时器）；日志去重。
- 集成/无头测试：Playwright + OfflineAudioContext 测试页；headless 联调压制值。
- 性能基准：Worklet 离线基准（上）+ 主线程 `occlusion_probe` 每次 ≤ 0.3 ms（4 射线 × 8 源）。
- 回归：M1-063 的枪声交叉淡化与声速延迟单测继续通过；本地开火音与预测同帧不变。

## 性能预算
- ARCHITECTURE §12.3：处理块 2.67 ms，Worklet ≤ 0.3 ms/128 帧；§13.5.1 音频主线程部分 0.5 ms；§12.4 并发声音 64/48/32 不变。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（C-BUL-005）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 听觉数值（AU-06/AU-09）均为初值，待 M2-054 回填；`/data` 状态保持 `initial` 直至测量。
- 压制与受击方向的 UI 效果不在本任务；仅音频。
- 声音可视化（UI_SPEC §7.2）属 M4-025，与本任务的遮挡数据共享时需受服务器声音 AOI 限制。
