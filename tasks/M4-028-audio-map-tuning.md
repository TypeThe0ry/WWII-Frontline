# M4-028 音频：分地图混响与环境调校

## 元数据
- 里程碑：M4
- 工作流：audio
- 规模：M
- 前置依赖：M2-046, M4-014, M4-015
- 素材需求：F-AMB-002; F-AMB-004; J-IR-*（缺则静音）；素材状态：placeholder
- 引用文档：ARCHITECTURE §12.2; ARCHITECTURE §12.7; ARCHITECTURE §12.1; ARCHITECTURE §12.4; ARCHITECTURE §11.4; ASSET_GAPS §3.3

## 背景
M2-046 已有混响总线（field/forest/town 三套 ConvolverNode）、环境床、载具声。M4 增加海滩（Omaha）、林地与雪地（Foy），需要为三图选择/调校**混响 IR、环境床与尾音类型映射**。
关键约束（ARCHITECTURE §12）：
- 尾音：`Shot` 事件触发时选 `tail_type`（开阔/林地/镇区），由玩家所处区域与射手所处区域混合决定；到 `ConvolverNode` 的发送量随距离增加（§12.2）。IR 为 J-IR-001（室外开阔/城镇/森林/室内，单声道或 FOA，48 kHz，0.5–4 s）与 J-IR-002（远距炮声大空间 IR，3–6 s）；许可证需逐条检查（OpenAIR 条目多为 CC，**未核实**）。
- 环境：F-AMB-002 海滩海浪（Freesound CC0，状态 OK）、F-AMB-004 冬季森林（CC0 OK）；环境床 Opus 32 kbps（§12.7）；首张地图音频总量 ≈ 18 MB（Medium），High 加入多样化变体至 ≈ 40 MB；长音流式解码。
- 并发：Low 32 / Medium 48 / High 64 / Ultra 64；同武器 50 ms 内最多 4 个（§12.4）。
- 缺素材：返回短静音 `AudioBuffer`，每缺失 ID 每会话 1 条日志，**不得用其它层顶替**（ASSET_GAPS §3.3）。
- 声速延迟：`d/343 m/s`（≤ 3 s），仅对 > 150 m 的远距离射击启用；近处不额外延迟（§12.2）。
- 听感无参考录音：不录制/提取参考作品音频（REF §4.12 仅测行为）。

## 具体内容
### 要创建/修改的文件
- `data/audio/map_acoustics.json` —— 每图每区域类型的 `tail_type` 映射、混响发送曲线（距离→send）、湿/干比、环境床分区（海滩区、悬崖区、林内、雪原、村庄）、风声/海浪强度随海拔与距岸距离的曲线（状态 `initial`）。
- `client/src/audio/AmbientZones.ts` —— 依据玩家位置查询区域（使用 `zones/landmarks` 与 `coast.geojson`/`forests.geojson` 的低精度光栅"声学分区图"）。
- `client/src/audio/TailSelector.ts` —— 增加 `beach`/`snow_forest` 尾音类型，并与 IR 绑定（缺 IR 时回退到已有 field/forest，**并记录 `placeholder`**）。
- `tools/map-pipeline/map_pipeline/stages/acoustic_zones.py` —— 从矢量烘焙 128 m 分辨率声学分区图（`acoustic_zones.bin`，随区块清单进 manifest）。
- `docs/` 不改；听感检查清单文件 `tests/audio/listening_checklist.md`（测试产物，非文档任务）。
- `client/tests/audio/{tail_selector,ambient_zones}.spec.ts`。
### 要实现的行为
1. 三图分区：Carentan（城镇/树篱/沼泽）、Omaha（海滩开阔、悬崖冲沟、高原村落）、Foy（雪地牧场、针叶林内、村庄）各定义 ≥ 4 个声学区域与 `tail_type`。
2. 环境床：海滩海浪随距水线距离衰减（0–200 m 内可闻，曲线数据驱动）；Foy 林内低风声 + 积雪吸声（高频衰减）；换区交叉淡化 ≥ 2 s，避免突变。
3. 混响发送：炮声/爆炸走 J-IR-002 长尾；步枪走近中远三层 + `tail_type` IR；雪林内尾音更短更暗（IR 或滤波参数，缺 IR 时用 `BiquadFilterNode` 调整并标注）。
4. 缺素材行为：F-AMB-002/004、J-IR 缺失时静音并按会话单次日志，其它音频链路不受影响。
5. 体积：各档音频包体积符合预算（Medium 首图音频 ≈ 18 MB 量级，High ≈ 40 MB 量级）：新增环境与 IR 不超过 2 MB/图（Opus 32 kbps 约 4 MB/ 16 min，必要时循环短样本）。
### 接口与数据
- `interface AcousticZone { id: string; tail: 'open'|'forest'|'town'|'beach'|'snow_forest'; ambient: string[]; send_curve: [number,number][]; hf_damp_db: number }`；区域查询 `AmbientZones.at(x: number, z: number): AcousticZone`。

## 不做什么
- 不做素材采购/登记（H-080，人工）；不做枪声/爆炸本体调校（M3-038）。
- 不做 AudioWorklet 新算法（沿用 M2-045）。
- 不做语音相关音频。
- 不做音乐。

## 验收标准
1. **三图各通过听感检查清单**（`tests/audio/listening_checklist.md`：每图 ≥ 10 项，如海浪随距离、林内尾音、区域切换无突变、远炮声延迟合理、无削波），由执行者与人工共同记录；缺人工项标 `未测`（人类耳听）。
2. 区域切换无突变：脚本在 5 条穿越路线上记录主总线 RMS，切换前后 1 s 内 RMS 变化 ≤ 6 dB（离线渲染 `OfflineAudioContext`）。
3. 音频包体积：各档（Medium/High）每图音频总量 ≤ 档位预算（以 manifest 为准，Medium ≤ 18 MB 量级、High ≤ 40 MB 量级；超限 > 1% 则 `manifest-budget` 作业失败）。
4. 缺素材安全：移除 F-AMB-002/004 与 J-IR 后，三图运行 5 min 无异常；每缺失 ID 日志条数 = 1（计数）。
5. 并发：Low ≤ 32、Medium ≤ 48、High/Ultra ≤ 64 并发声音上限在三图战斗压力场景下不被突破（计数器断言）。
6. 声速延迟：> 150 m 的 `Shot` 延迟 = `d/343`（误差 ≤ 1 帧），≤ 150 m 无附加延迟（单测）。

## 测试要求
- 单元测试：`vitest`：分区查询边界、`TailSelector` 混合权重、send 曲线插值、缺素材回退。
- 集成/无头测试：Playwright + `OfflineAudioContext` 离线渲染固定路线；`audio-probe` 脚本。
- 性能基准：音频主线程 ≤ 0.5 ms/帧（§13.5.1），AudioWorklet 块 2.67 ms 内处理；Low 档 32 并发下无 underrun。
- 回归：区域图哈希入 manifest。

## 性能预算
- 音频主线程部分 ≤ 0.5 ms/帧（ARCHITECTURE §13.5.1）；并发声音 32/48/64（§13.1/§12.4）；不得使音频包超过 §12.7 的体积估算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 真素材（F-AMB-002/004、J-IR-*）在 M4-029 接入；本任务在静音/占位下完成逻辑与数据，真调校需素材后复测，对应验收 1 标 partial。
- J-IR 的 OpenAIR 许可证逐条核实未完成（SHOP 标 503/未核实），入构建前必须 LICENSES.md=OK。
- 听感检查依赖人类耳听（无法客观测量），清单需人类签字；缺则 `partial`。
