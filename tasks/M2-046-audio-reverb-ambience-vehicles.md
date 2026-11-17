# M2-046 音频：混响总线、环境与载具声

## 元数据
- 里程碑：M2
- 工作流：audio
- 规模：M
- 前置依赖：M1-062
- 素材需求：J-IR-001; J-IR-002; F-AMB-*; D-TNK-*（载具引擎/履带）；素材状态：placeholder（缺则静音）
- 引用文档：ARCHITECTURE §12.1; ARCHITECTURE §12.2; ARCHITECTURE §12.4; ARCHITECTURE §12.7; ASSET_GAPS §3.3; REFERENCE_MEASUREMENTS §4.12

## 背景
ARCHITECTURE §12.1 音频图中，爆炸/炮声（S3）同时送入 **ConvolverNode 混响总线（field/forest/town）**，环境床（S4）进环境总线，载具声（S2）属脚步/装备/载具源。§12.2：枪声尾音按"玩家所处区域与射手所处区域"混合选 `tail_type`（开阔/林地/镇区），到 `ConvolverNode` 的发送量随距离增加。§12.4 复音：最大并发声音 64（High）/48（Medium）/32（Low），按"响度估计 × 重要性 / 距离"排序超出则偷最低者；流式长音（载具引擎、环境床）用 `AudioBufferSourceNode` loop + 参数自动化，不创建 MediaElement。§12.7：Opus 为主（`.fpak` 内，D-AUD），环境床 32 kbps；短音（< 3 s）常驻 AudioBuffer，长音流式解码。
M1-062 已有 AudioContext 图、总线、HRTF Panner、复音与偷声、缺失素材静音占位与日志、编码探测。M2 需要：混响总线与 3 套 IR 切换、环境床循环、载具引擎循环（Jeep/Kübel、卡车、Panzer IV；M2-015 提供载具状态：速度、RPM 近似、履带速度、炮塔转动）、爆炸（近处由 M2-045 的冲击效果配合）。素材 J-IR-001/002（IR）、F-AMB-*（环境）、D-TNK-*（坦克引擎/履带）多半缺失（GAP-A07/A11/A14），按 ASSET_GAPS §3.3 静音占位，**逻辑与参数自动化必须完整可测**。

## 具体内容
### 要创建/修改的文件
- `client/src/audio/reverb_bus.ts` —— 混响总线：3 个 `ConvolverNode`（field/forest/town）+ 干湿发送增益；根据听者与声源所在区域的 `tail_type` 做 100–300 ms 交叉淡化切换（双 Convolver 乒乓，避免爆音）；IR 缺失时旁路并静音。
- `client/src/audio/zone_classifier.ts` —— 由地图数据 `audio_zones`（M2-005/M2-004 的区域标注：town/forest/field）与玩家位置判定尾音类型；缺数据时按植被/建筑密度（区块元数据）近似。
- `client/src/audio/ambience.ts` —— 环境床管理：按区域/时段/天气选择床（风、虫鸣、远炮背景）；`AudioBufferSourceNode` loop，位置无关的 2D 床 + 区域交叉淡化（2–4 s）；总量受环境总线上限。
- `client/src/audio/vehicle_audio.ts` —— 载具声：引擎循环（RPM→`playbackRate` 与滤波/增益自动化）、履带循环（速度→增益/速率）、炮塔转动循环、车内/车外两套混合；复音预算内的每辆载具最多 3 条循环，载具总循环 ≤ 并发预算的 25%。
- `client/src/audio/explosion_audio.ts` —— 爆炸/炮声（近/中/远三层，沿用 M1-063 枪声三层模型）与混响发送；声速延迟 `d/343`。
- `data/audio/vehicles.json`、`data/audio/reverb.json`、`data/audio/ambience.json` + schema —— 参数曲线与 IR/床映射（状态 `initial`，【待调参 → REFERENCE_MEASUREMENTS §4.12 AU-07/AU-08】）。
- `client/src/audio/missing_assets.ts`（沿用/修改）—— 缺失需求 ID 统一静音与日志（J-IR-*、F-AMB-*、D-TNK-*）。
### 要实现的行为
1. 混响：爆炸/炮声/远距枪声按距离提高发送量；`tail_type` 变化时 200 ms 交叉淡化，无爆音。
2. 环境床：每区域至少 1 个循环；离开/进入区域淡变；全部为 2D（非空间化）以节省并发。
3. 载具：引擎循环的 `playbackRate = f(rpm)`、增益 = f(距离, 速度)；起步/怠速/加速/换挡的参数曲线来自 `/data/audio/vehicles.json`；载具被击中/损坏的声音事件映射（M3-037 完整化，本任务仅留 `onVehicleEvent(kind)` 钩子）。
4. 复音与偷声：载具/环境循环占用并发额度，被偷时淡出而不是硬切；超出预算按 §12.4 排序。
5. 编码与降级：IR/床/载具素材从 `.fpak` 的 Opus 条目读取，不通过探测则走 AAC（M1-062）；缺失即静音占位。
### 接口与数据
```ts
export type TailType = 'field' | 'forest' | 'town';
export class ReverbBus { setTail(listener: TailType, source: TailType, distM: number): void; send(src: AudioNode, sendDb: number): void }
export interface VehicleAudioParams { rpm: number; speedMps: number; trackSpeedMps?: number; turretRateDegS?: number; damage?: number }
export class VehicleAudio { update(id: number, p: VehicleAudioParams, pos: Vec3): void; dispose(id: number): void }
```

## 不做什么
- 不做全部载具/重武器/炮击/空袭音频（M3-037、M3-038）。
- 不做遮挡/压制（M2-045）；不做语音（M2-026）。
- 不做分地图混响调校（M4-028）。
- 不做素材接入替换（M2-047）；不做自制/AI 生成音频。

## 验收标准
1. IR 切换无爆音：`OfflineAudioContext` 渲染 `field→forest→town→field` 连续切换的输出，相邻采样最大差 ≤ 0.05（满量程 1.0；测试信号为 1 kHz 正弦 −12 dBFS）；缺 IR 时旁路也无爆音。
2. 3 套 IR（J-IR-001/002 或占位）切换耗时（从 `setTail` 到新 Convolver 全开）≤ 300 ms（假定时器单测，误差 ±20 ms）。
3. 载具声并发：同屏 12 辆载具 + 64 并发压力（High）下，活动声音数 ≤ 64（Medium 48 / Low 32），载具循环占比 ≤ 25%（计数断言）；被偷的循环淡出 ≥ 100 ms 无爆音。
4. 自动化曲线：对 `/data/audio/vehicles.json` 的 rpm→playbackRate 曲线，在 5 个采样点处实测 `playbackRate` 与配置误差 ≤ 1%（单测）。
5. 主线程成本：64 并发 + 12 载具 + 3 环境床下，音频主线程更新 ≤ 0.5 ms/帧（P95，`performance.now`，参考机，§13.5.1）。
6. 缺失素材：J-IR-001/002、F-AMB-*、D-TNK-* 缺失时全部不报错，每需求 ID 每会话 1 条日志，效果链路（发送量/自动化）仍执行（单测以测试信号验证）。
7. 环境床交叉淡化 2–4 s 期间总增益起伏 ≤ 1.5 dB（离线测量，等功率曲线）。

## 测试要求
- 单元测试：`tail_type` 判定；发送量曲线；载具曲线插值；复音排序与偷声；日志去重。
- 集成/无头测试：Playwright 测试页 + `OfflineAudioContext` 离线渲染断言；headless 联调载具状态。
- 性能基准：音频主线程分段计时输出 `reports/audio/m2_perf.json`。
- 回归：M1-062 并发/偷声与 M1-063 枪声测试继续通过。

## 性能预算
- ARCHITECTURE §12.4：并发 64/48/32；§13.5.1 音频主线程 0.5 ms；§12.7 首张地图音频总量 ≈ 18 MB（Medium）/ ≈ 40 MB（High），由 M2-047 门禁。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（J-IR-001/002、F-AMB-*、D-TNK-*）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 卷积混响 CPU 成本随 IR 长度上升；IR ≤ 6 s（J-IR-002 为长尾 3–6 s），若 Low 档超预算，写 PLAN_ISSUES（M4-020 优化）。
- 载具声完全依赖 M2-015 的载具状态字段；若缺 RPM，用速度比近似并在备注记录。
- 听觉测量项 AU-07/AU-08（REFERENCE_MEASUREMENTS §4.12）由 M2-054 回填；录像不得提取其音频作素材。
