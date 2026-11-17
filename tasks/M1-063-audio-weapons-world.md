# M1-063 音频：枪声分层、脚步与命中

## 元数据
- 里程碑：M1
- 工作流：audio
- 规模：L
- 依赖：M1-062, M1-040（事件：`fire.local/remote`、`impact`、`hit.confirm`、`suppress`）
- 素材需求：A-WPN-*; C-BUL-*; E-FTS-001; H-UI-*（缺则静音）；素材状态：placeholder（静音）
- 引用文档：ARCHITECTURE §12.2; ARCHITECTURE §12.3（压制简版）; ARCHITECTURE §12.6; GAMEDESIGN §6; GAMEDESIGN §8.3; REFERENCE_MEASUREMENTS §4.12

## 背景
战斗音频的核心：枪声三层距离交叉淡化、声速延迟、脚步材质、弹着/命中、压制简版、UI 提示音。ARCHITECTURE §12.2：每把武器 3 层素材——**near**（0–60 m）、**mid**（60–250 m）、**far**（> 250 m）；交叉淡化：`gain_near = clamp(1 − (d−30)/60, 0, 1)`，`gain_mid` 为 30–60 m 渐入、200–300 m 渐出，`gain_far` 为 200 m 渐入至 600 m 峰值，之后随距离自然衰减；**声速延迟** `delay = d / 343 m/s`（≤ 3 s 截断），**仅对 > 150 m 的射击事件启用**，近处不额外延迟（防止被误认为网络延迟）；尾音类型（开阔/林地/镇区）由玩家与射手所处区域混合决定，到 Convolver 的发送量随距离增加（混响总线在 M2-046，本任务只预留 `tail_type` 与发送量参数）。本地开火音在 `fire.local`（预测）时立即触发，不等服务器（§12.6）；远程枪声按 `Shot` 事件触发并叠加声速延迟。压制（GD §8.3 听觉）：近距离语音与环境音降低 `3 dB·e`；ARCHITECTURE §12.3 完整 Worklet（耳鸣、爆炸冲击）在 M2-045，M1 做简版：主总线低通/闪避 `suppress` 事件（强度 → 低通 cutoff 与 -dB，≤ 400 ms）。音频行为阈值（可闻距离、近/中/远切换距离、脚步可闻距离）见 REFERENCE_MEASUREMENTS §4.12（AU-01/02/04）为【待测】，初值放 `/data/audio/*.json`。

## 具体内容
### 要创建/修改的文件
- `client/src/audio/weapons.ts` —— 订阅 `fire.local`/`fire.remote`：计算距离 `d`、三层增益（公式如上）、声速延迟（d > 150 m 时 `setTimeout`/`AudioContext.currentTime` 调度）、选变体（每层 ≥ 3–4 变体，随机但避免连续重复）、尾音参数（占位）。
- `client/src/audio/crossfade.ts` —— 纯函数：`layerGains(d) -> {near, mid, far}`，便于单测与曲线可视化。
- `client/src/audio/footsteps.ts` —— 订阅动画脚步事件（M1-045）与地面材质（M1-050 `getSurfaceAt`）：材质→声音集合（泥/草/石板/碎石/木/沙）；走/跑/蹲/落地；音量与可闻距离按姿态；限制同屏脚步并发（≤ 8，远处合并）。
- `client/src/audio/impacts.ts` —— 订阅 `impact`（材质：泥/混凝土/金属/木/沙袋/水）、跳弹、子弹掠过（`suppress`/近距擦过 whizz 的 `C-BUL-001`，左右声像取自方向）、`hit.confirm`（射手听到命中音，区分头/体）、`damage.taken`（受击音，C-BUL-004 缺则静音）。
- `client/src/audio/suppression-lite.ts` —— 压制简版：`suppress` 事件→SFX/环境总线 `BiquadFilterNode`（低通）与增益闪避，≤ 400 ms 恢复；M2-045 替换为 Worklet 版。
- `client/src/audio/ui-sounds.ts` —— UI 提示音（H-UI-001/002：点击/确认/拒绝/倒计时、复活、占点、补给到达），事件映射。
- `data/audio/weapons.json`、`data/audio/footsteps.json`、`data/audio/world.json` —— 层边界、`speed_of_sound_mps = 343`、延迟启用距离 150 m、脚步可闻距离初值（状态 `initial`，【待测 → REFERENCE_MEASUREMENTS §4.12】）；schema 并入 `/data/schemas`。
- 测试：`client/test/audio/crossfade.test.ts`、`weapons.test.ts`、`footsteps.test.ts`。
### 要实现的行为
1. 本地开火：`fire.local` 同一事件循环内触发 near 层（立即，不等服务器），与 FP 枪口焰同帧。
2. 远程枪声：按距离分层；> 150 m 延迟 `d/343`（精确到 ±1 帧）；≤ 150 m 无额外延迟。
3. 同类合并与复音遵守 M1-062。
4. 脚步：材质来自地形 splat（M1-050）与建筑地板（M1-052 材质标签）；姿态与速度影响音量/可闻距离。
5. 命中：射手命中音在 `hit.confirm` 触发，延迟 ≤ 1 帧；受击音在 `damage.taken` 触发。
6. 缺失素材：任一层缺失按"该层静音"处理，不用其它层顶替；日志去重（M1-062）。
7. 所有可调参数读自 `/data/audio/*.json`，客户端不硬编码数值（ARCHITECTURE §4.4）。
### 接口与数据
```ts
export function layerGains(d: number): { near: number; mid: number; far: number };
export function shotDelaySeconds(d: number): number; // d<=150 → 0 ; else min(d/343, 3)
// data/audio/weapons.json 条目：{ id, layers: {near:[assetId...], mid:[...], far:[...]}, audible_range_m, layer_xfade_m: [..], tail_default }
```

## 不做什么
- 不做 AudioWorklet 遮挡低通（M2-045）与完整压制/耳鸣/爆炸冲击（M2-045）、混响总线（M2-046）。
- 不做载具声（坦克引擎/炮塔，M2-046 与 D-TNK，M1 的 Sherman 开炮声归 `A-WPN` 类或缺则静音）。
- 不做语音、无线电（M2-026、M3-039）。
- 不做真素材接入与响度归一（M1-004/M1-065）。

## 验收标准
1. 交叉淡化曲线单测：`layerGains` 在 d = 0/30/60/100/200/250/300/600/1000 m 的输出与 §12.2 公式逐点一致（误差 ≤ 1e-6；`crossfade.test`；INDEX 指标）。
2. 声速延迟：d = 200/343/686/1 029 m 的延迟分别 = d/343 ±1 帧（±16.7 ms），d ≤ 150 m 为 0（vitest + `OfflineAudioContext` 采样时间）。
3. 本地开火音与预测同帧：`fire.local` 触发到 `AudioBufferSourceNode.start()` 的调度间隔 ≤ 1 渲染帧（时间戳断言；缺素材时仍调用播放链路）。
4. 缺素材静音不报错：全部 A-WPN/C-BUL/E-FTS 缺失时，200 次事件无异常、日志每缺失 ID 1 条。
5. 脚步并发 ≤ 8（远处合并）；10 名玩家同时跑动时音频主线程 < 1 ms/帧（叠层）。
6. 压制简版：`suppress(intensity=255)` 触发后 400 ms 内低通 cutoff 回到初值 ±5%，且命中后立即可再次触发（vitest，`OfflineAudioContext`）。
7. `data/audio/*.json` 通过 data-validate，字段全部带单位且状态 `initial`。

## 测试要求
- 单元测试：`layerGains`、延迟函数、变体防重复、材质→脚步映射、受击/命中事件映射。
- 集成/无头测试：Playwright 触发合成枪战（本地+远程+命中），采集播放调度日志。
- 性能基准：事件→播放调度开销写入 `reports/m1-audio-weapons.json`。
- 回归：M1-062 复音/合并测试保持通过。

## 性能预算
- ARCHITECTURE §13.5.1：音频主线程 ≤ 0.5 ms/帧（High）；§12.4 复音 ≤ 64（High）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright、`data-validate`）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3.3 处理；缺失素材需求 ID（A-WPN-*、C-BUL-*、E-FTS-001、H-UI-*）已列出
- [ ] 文档同步：`/data/audio` schema 写入 PLAN_ISSUES"细化"
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 近/中/远切换距离与可闻距离均为【待测】初值（REFERENCE_MEASUREMENTS §4.12 AU-01/02/04）；M1-071 回填后更新 `/data/audio`。
- 部分素材存在 GAP（C-BUL-004 人体命中、A-WPN-005 BAR 全自动等，见 ASSET_GAPS §2.2）；缺失一律静音，不顶替。
- 规模 L：脚步+命中+UI 音若超一次会话，拆 `M1-063b`（脚步与 UI 提示音）。
