# M3-037 音频：全部载具与重武器声

## 元数据
- 里程碑：M3
- 工作流：audio
- 规模：M
- 前置依赖：M2-046, M3-014
- 素材需求：D-TNK-*; D-VEH-001; B-HVY-001（缺失则静音占位）；素材状态：placeholder（静音）
- 引用文档：ARCHITECTURE §12.1; ARCHITECTURE §12.3; ARCHITECTURE §12.4; ARCHITECTURE §12.7; GAMEDESIGN §14.9; ASSET_GAPS §3.3

## 背景
M2-046 建好了音频事件映射框架（事件 → 素材 ID → 3D PannerNode → 总线）。M3-014 等 sim 任务提供了载具状态（转速/速度/履带/炮塔/模块状态）。本任务把全部载具的声音事件映射到参数自动化，并把"重武器"（Bazooka/Panzerschreck/Panzerfaust、车载机枪）接入。
架构要点：
- 图结构（ARCH §12.1）：声源 → HRTF PannerNode → AudioWorklet（遮挡低通 + 压制）→ SFX 总线；爆炸/炮声另送卷积混响总线。
- 复音（ARCH §12.4）：最大并发 64（High）/ 48（Medium）/ 32（Low）；按"响度估计 × 重要性 / 距离"排序，超出则偷最低者；同类枪声 50 ms 内最多 4 个；长音（引擎、环境床）用 `AudioBufferSourceNode` loop + 参数自动化，不创建 MediaElement。
- 载具声学（GD §14.9）：载具内乘员的声音对外 −10 dB；封闭车辆对内环境声低通。
- 缺失素材：返回短静音 `AudioBuffer`（0.1 s），每个缺失需求 ID 每会话只打一条日志，不用其它层顶替（ASSET_GAPS §3.3）。

## 具体内容
### 要创建/修改的文件
- `client/src/audio/vehicles/EngineVoice.ts` —— 引擎 loop：`rpm`（由速度/档位/油门映射）→ `playbackRate`（0.7–1.6）与 gain；点火/熄火 one-shot；发动机损伤（≤ 50%）改变音色（低通 + 抖动），毁坏时熄火。
- `client/src/audio/vehicles/TrackVoice.ts` —— 履带/轮胎 loop：按地面速度调 gain 与滤波；转向差速时增强。
- `client/src/audio/vehicles/TurretVoice.ts` —— 炮塔转动电机 loop（按角速度）；到位停止。
- `client/src/audio/vehicles/MainGunVoice.ts` —— 主炮发射（近/中/远三层，ARCH §12.2）、装填金属声、空弹壳；机枪使用既有枪声层。
- `client/src/audio/vehicles/HitVoice.ts` —— 被击中/跳弹/击穿内部回响（D-TNK-005），对车内乘员更闷更响。
- `client/src/audio/vehicles/HeavyWeapons.ts` —— Bazooka/Panzerschreck/Panzerfaust 发射与命中（B-HVY-001）。
- `client/src/audio/vehicles/VehicleVoicePool.ts` —— 载具 loop 的并发控制：每辆载具最多 2 个常驻 loop（引擎、履带）；距离 > 150 m 的同型号载具合并为一个"群引擎"voice【阈值待调参】；本类最多占用总复音的 40%（High 约 24 个）【待调参】。
- `client/data/audio_events/vehicles.json`（或沿用 M2-046 的映射表路径）—— 事件 → 素材 ID → 层/参数曲线。
- `client/tests/audio/vehicles.spec.ts`
### 要实现的行为
1. 订阅 `vehicle.state`（10 Hz）与 `vehicle.fire/hit/module/seat` 事件；参数用 `AudioParam.setTargetAtTime`，时间常数 0.1 s，避免爆音。
2. 车内视角（玩家在载具内）：引擎/履带切到"内部"混音（低通 2 kHz 以下 + 增益），外部声整体 −10 dB（GD §14.9 的对外 −10 dB 反向应用于车外听者）。
3. 主炮发射：近层 0–60 m、中层 60–250 m、远层 > 250 m 交叉淡化；> 150 m 启用声速延迟 `d/343`（共用 M3-038 的 `SoundDelay` 工具，若尚未合入则在此建立并由 M3-038 复用）。
4. 复音保护：同屏 12 辆载具场景 `ActiveVoices ≤ 预算`；被偷的优先级最低者为远处履带。
5. 缺失素材一律静音占位 + 日志（每 ID 每会话一次）；严禁用别的素材顶替。
6. 载具被毁时引擎 loop 在 0.5 s 内淡出，燃烧由 M3-038/素材提供（缺失静音）。
### 接口与数据
```ts
interface VehicleAudioProfile { engine_id: string; track_id: string; turret_id?: string; main_gun_id?: string; rpm_curve: [number, number][]; }
```
档案放在 `/data/vehicles/*.json` 的 `audio` 字段（若未含，补字段并 PLAN_ISSUES 记"细化"）。

## 不做什么
- 爆炸、炮击、空袭声（M3-038）；人声与无线电（M3-039）；车组语音频道（M3-040）。
- 真实素材接入与 Opus/AAC 打包（M3-043）。

## 验收标准
1. 12 辆载具同时运行（混合型号，随机速度）：`activeVoices` ≤ 64（High）/ 48（Medium）/ 32（Low），其中载具占比 ≤ 40%（Playwright 读取音频调试面板数值，各档位各测一次）。
2. 素材全缺失时无异常：控制台 0 个未捕获错误，缺失日志每个需求 ID 恰好 1 条（日志计数用例）。
3. 引擎参数连续性：速度从 0 阶跃到 30 km/h，`playbackRate` 变化无单帧跳变 > 0.15（OfflineAudioContext 渲染后检查包络，无削波、峰值 ≤ −1 dBFS）。
4. 声速延迟用例：距离 343 m 的主炮事件延迟 1.00 s ±3 ms（OfflineAudioContext）；距离 < 150 m 不额外延迟。
5. 车内/车外切换时增益过渡 ≤ 0.2 s，无爆音（包络检查）。
6. 音频主线程 CPU（参数更新与事件）≤ 0.5 ms/帧（ARCH §13.5.1）。

## 测试要求
- 单元：rpm 曲线映射、复音偷取排序、群引擎合并阈值。
- 集成：Playwright 12 载具调试场景；使用静音占位全缺失模式。
- 性能：音频主线程耗时采样（`performance.measure`）。
- 回归：M2 枪声映射测试不变。

## 性能预算
- ARCH §13.5.1 音频主线程 0.5 ms；§12.4 复音上限；§12.7 音频总量不在本任务增加（占位静音）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- D-TNK-006（Pz IV/Panther 全套）、D-TNK-005 在素材清单里有 GAP（GAP-A05 等）；素材缺失时本任务仍应完成，听感调校放到 M3-043。
- "载具占复音 40%" 与 "150 m 群引擎" 是本任务提议初值，需在 M3-049/听感检查后复核。
- 听感类验收（是否自然）需人工，记 `未测`。
