# M3-038 音频：爆炸、炮击与空袭

## 元数据
- 里程碑：M3
- 工作流：audio
- 规模：M
- 前置依赖：M2-045, M3-008, M3-018
- 素材需求：B-ART-*; B-AIR-*; B-EXP-*; D-AIR-001（缺失则静音占位）；素材状态：placeholder（静音）
- 引用文档：ARCHITECTURE §12.1; ARCHITECTURE §12.2; ARCHITECTURE §12.3; ARCHITECTURE §12.4; GAMEDESIGN §13.1; GAMEDESIGN §15.6; GAMEDESIGN §13.3; ASSET_GAPS §3.3

## 背景
M2-045 建好了爆炸/手雷等基础音频与 AudioWorklet 遮挡/压制。本任务扩展到炮击、空袭与大型爆炸。规则与数字：
- 声速延迟（ARCH §12.2）：`delay = d / 343 m/s`，截断 ≤ 3 s，只对 > 150 m 的远处事件启用；近处不额外延迟（避免被当作网络延迟）。
- 枪炮三层：near 0–60 m、mid 60–250 m、far > 250 m；`gain_near = clamp(1-(d-30)/60, 0, 1)`，mid 30–60 m 渐入、200–300 m 渐出，far 200 m 渐入至 600 m 峰值。
- 远炮声走卷积混响总线（field/forest/town，ARCH §12.1）；爆炸冲击（ARCH §12.3）：低频 thump + 2 s 听力模糊（低通 1.2 kHz 渐开）。
- 炮击预警（GD §15.6）：炮弹落地前 4 s，目标 60 m 内玩家听到尖啸（音量按距离）；高角射击预警更长；敌方 Bombing/Strafing/Precision 抵达前 15 s 播放防空警报（GD §13.1）；Supply/Ammo Drop 与 Recon 无警报，仅飞机声，距离 < 400 m 可听。
- 105 mm 榴弹炮开火（B-ART-001，近/中/远三层）、落弹（B-ART-002）、炮击床（B-ART-003，免费远层循环）、航弹（B-AIR-001/002）、飞机（D-AIR-001：P-47/Ju 87/C-47）。缺失一律静音占位，不顶替（ASSET_GAPS §3.3）。

## 具体内容
### 要创建/修改的文件
- `client/src/audio/explosions/SoundDelay.ts` —— `delayFor(distance_m)`；若 M3-037 已建立则直接复用。
- `client/src/audio/explosions/ArtilleryAudio.ts` —— 我方/敌方炮击：炮口声（三层）、尖啸（4 s 前）、落弹（近/远）。
- `client/src/audio/explosions/ArtilleryBed.ts` —— 远处炮击床：按战场炮击事件密度调节循环床增益（上限 −12 dBFS）。
- `client/src/audio/explosions/AirStrikeAudio.ts` —— 飞机进场 + 多普勒（`playbackRate` 随径向速度变化，±6%）、投弹口哨、航弹爆炸、扫射、防空警报（仅对敌方）。
- `client/src/audio/explosions/BlastImpact.ts` —— 近距爆炸冲击：thump、2 s 听力模糊、耳鸣淡出（与 ARCH §12.3 的压制效果联动）。
- `client/src/audio/explosions/ReverbBus.ts`（改）—— 为炮声增加按距离的发送量；混响类型由玩家与射手所处区域混合（开阔/林地/镇区）。
- `client/data/audio_events/explosions.json` —— 事件 → 素材 ID → 层/参数。
- `client/tests/audio/explosions.spec.ts`
### 要实现的行为
1. 订阅 M3-005 的炮击/空袭事件，使用事件携带的 `tick` 与客户端 `estimated_server_tick` 计算本地播放时间，叠加声速延迟。
2. 防空警报：仅当本队为受击方且事件为 Bombing/Strafing/Precision，且距目标 ≤ 200 m 的模糊环范围内才播放（不泄露方向，只播警报）。
3. 尖啸：落弹点 60 m 内玩家，在落地前 4 s 起 `gain` 随剩余时间上升（高角射击事件提前量更长，取 `tof` 的 min(tof, 8 s)【待调参】）。
4. 多普勒：飞机声源为运动 PannerNode，位置由 M3-030 的 `FlightPath` 提供。
5. 复音：爆炸类同型 50 ms 内最多 4 个，超出合并为"群发声"（ARCH §12.4）。
6. 缺失素材处理：静音 + 每 ID 每会话一次日志。
### 接口与数据
```ts
export function delayFor(d_m: number): number { /* d>150 ? min(d/343, 3) : 0 */ }
```

## 不做什么
- 载具声（M3-037）；人声与无线电（M3-039）；真实素材接入（M3-043）。
- 飞机/炮击的视觉（M3-030、M3-031）。
- 敌方炮口位置的地图标记（UI）。

## 验收标准
1. 炮声延迟 = d/343：对 d = 200/343/686/1200 m，OfflineAudioContext 渲染的首个非零采样时刻与理论值 `min(d/343, 3)` 偏差 ≤ 3 ms（≤ 150 m 的用例偏差为 0 延迟）。
2. 三层交叉淡化：对 d = 30/60/150/250/400/600 m 取样，`gain_near/mid/far` 与 ARCH §12.2 公式偏差 ≤ 0.02。
3. 防空警报只对被攻击方播放：4 个用例（友方发起、敌方发起且在环内、在环外、Supply Drop）通过。
4. 远炮走卷积总线：> 250 m 的炮声事件送入 `ConvolverNode` 的发送量 > 0.1，近炮 < 0.05（节点图断言）；听感"远炮空间感"检查记录由人工完成（H-071 之外无需录像，需人工试听，记 `未测` 直至完成）。
5. 全素材缺失时 0 个未捕获错误，缺失日志每 ID 1 条。
6. 同屏 10 次爆炸/炮击并发时 `activeVoices` ≤ 64（High），音频主线程 ≤ 0.5 ms/帧。

## 测试要求
- 单元：`delayFor`、层增益公式、尖啸时间轴、警报可见性判定。
- 集成：Playwright 播放 `?vis_scene=artillery` 与 `airstrike` 脚本事件，读取音频调试面板。
- 性能：音频主线程耗时采样；OfflineAudioContext 渲染峰值 ≤ −1 dBFS（无削波）。
- 回归：M2 爆炸/压制测试不变。

## 性能预算
- ARCH §13.5.1 音频主线程 0.5 ms；§12.4 复音上限；不新增常驻内存（素材由 M3-043 管理，§12.7）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- B-AIR-001（航弹）缺口 GAP-A15；缺失时静音，不用 B-EXP 层顶替。
- "尖啸时间上限 8 s" 与多普勒 ±6% 为提议初值【待调参】；AU 类对标（AU-09–AU-12，REF §4.12）由 M3-052 回填后可修正。
- 防空警报素材（若属 E/F 类）需求 ID 以 SHOP 实际登记为准；缺失则静音。
