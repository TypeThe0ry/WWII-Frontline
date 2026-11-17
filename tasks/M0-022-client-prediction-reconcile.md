# M0-022 client/game：输入、预测、对账与插值

## 元数据
- 里程碑：M0
- 工作流：net
- 规模：L
- 前置依赖：M0-021, M0-020
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.3; ARCHITECTURE §6.4; ARCHITECTURE §8.4; ARCHITECTURE §7.1; BRIEF R5

## 背景
让本地玩家"零延迟"并与权威状态一致：键鼠 → `InputCmd`（30 Hz 固定步长）→ `sim-wasm` 预测 → 发送 `InputBundle`（最近 3 条冗余）→ 收到快照后**对账**（回滚 + 重放未确认输入）→ 其它实体做 **100 ms 缓冲插值**（延后 3 tick）。这是 E0-8（预测对账手感）的承载任务。
对账规则以 **R5 / ARCHITECTURE §6.4** 为准：预测与权威按**容差**对账，位置误差 `err0 ≤ 0.05 m` → 保持预测（不回滚、不重放）；`> 0.05 m` → 回滚到权威状态并重放全部未确认输入；重放后新旧预测位置差 `err < 1.0 m` → 视觉平滑（`render_offset = old_pos − new_pos`，在 100 ms 内指数衰减，τ = 40 ms），否则 snap 并仅在调试叠层提示。平滑只作用于**渲染位置与相机**，不改模拟状态。（INDEX 把阈值写成 0.02 m，与 R5 的 0.05 m 冲突；**以 R5 为准**，见备注。）预测不模拟玩家间碰撞；预测只覆盖本地玩家移动与本地武器开火表现（后坐/弹药显示，不判定命中，§6.4）。输入采样（§6.3）：每渲染帧聚合 `RawInput`（按键位图、鼠标 yaw/pitch 增量，增量累计到下个模拟步）；每模拟 tick 生成一条 `InputCmd`（`seq:u16`、`tick:u32`、`move_x/z:i8`、`yaw:u16`、`pitch:i16`、`buttons:u16`、`weapon_slot`、`aux`、`view_tick`）；"未确认输入环"容量 128。

## 具体内容
### 要创建/修改的文件
- `client/src/input/{pointer-lock.ts,keymap.ts,sampler.ts,index.ts}`：Pointer Lock 获取/丢失、WASD/Shift/Ctrl(C)/Space/鼠标左右键/R 映射（默认键位表为常量，重绑定属 M2-041）、灵敏度常量（`/data` 之外的纯输入偏好，可存 `localStorage`，初值 1.0°/count×DPI 无关的"度/像素"系数写入配置）。
- `client/src/game/{game-loop.ts,predictor.ts,unacked-ring.ts,reconcile.ts,interpolation.ts,state.ts,snapshot-apply.ts,events.ts}` 与同名 `.test.ts`；`client/src/game/state.ts` 的 `GameState` 用 `@preact/signals` 暴露只读视图（M0 仅叠层使用）。
- 扩展 `client/src/game/event-bus.ts` 的 `EventMap`（`snapshot.applied`、`entity.spawned/removed`、`fire.local`、`fire.remote`、`impact`、`hit.confirm`、`damage.taken`，字段按 ARCHITECTURE §8.4 目录）。
- `client/public/maps/m0/` 同步脚本（`pnpm -C client run sync-map`：复制 `data/maps/m0/chunks/c_*.bin` 与高度图供客户端 `fetch`），启动时 `Predictor.loadCollision` 载入全部 16 块（M0 全图，ARCHITECTURE §9.5）。
### 要实现的行为
1. **主循环接线**（组合根在 `main.ts`，由 M0-020 的 `FixedStepper` 驱动）：每模拟步 → `sampler.build()` 产出 `InputCmd` → `Predictor.step(cmd)` → 入未确认环 → 每步发送 `InputBundle(最近 3 条, ack_snapshot_tick, pred_hash)`。`cmd.tick = clock.estimated_server_tick + input_lead_ticks`（M0-021）；`view_tick = estimated_server_tick − 3`（插值延迟）。
2. **快照处理**：C2 字节 → `SnapshotDecoder.decode`（M0-014）→ 取 `ack_input_seq` 丢弃已确认输入；`self` 状态做对账（上述规则）；其它实体写入插值缓冲；发 `snapshot.applied`；记录 `ack_snapshot_tick` 供下一个 `InputBundle`。
3. **`pred_hash`**：客户端在预测的 `ack_tick` 位置量化后 CRC8 填入 `InputBundle`（与服务器 M0-019 同函数，`sim-wasm` 暴露 `predHash(posBytes)`）。
4. **插值**：每个远端实体保存最近 ≥ 4 个快照 `(tick,pos,yaw,pitch,…)` 的环形缓冲（SoA）；渲染时间 = `clock.estimated_server_tick − 3 + alpha`；线性插值位置、最短弧插值 yaw；缓冲不足时保持最后值，**不外推超过 150 ms**【待调参】；`entity.spawned/removed` 事件在首次出现/超时（> 1.5 s 未更新且在远场，§6.5.4）时发出。
5. **本地开火表现**：`fire` 键按下 → 按预测的 `WeaponState`（M0-014 的 `weaponState()`）判定能否开火；能 → 发 `fire.local{weapon, muzzlePos, dir}`；**不显示预测伤害/击杀**，仅在收到 `HitConfirm` 事件时发 `hit.confirm`。
6. **纠正计数**：`corrections`（回滚次数）、`snaps`（瞬移次数）、`lastErrM` 供叠层与测试读取；后台标签页暂停渲染但保持输入/心跳（§20 #6）。
### 接口与数据
- `GameState.local{pos,vel,yaw,pitch,stance,ammo,…}`、`GameState.entities`（SoA：`Float32Array` 位置、`Uint8Array` 标志）供 `render` 只读；`render/ui/audio` 通过事件总线与只读状态访问，**不被 `game` import**。

## 不做什么
- 不做灰盒角色/第一人称枪/枪口焰（M0-025）；不做载具预测（M1-041）；不做延迟补偿相关的 `FireCmd` 回溯语义（M1-025）。
- 不做重绑定与设置界面（M2-041）；不做完整 HUD（M1-057）。
- 不做 WS/WT 传输细节（M0-021）。

## 验收标准
1. vitest `reconcile_threshold`：构造预测与权威相差 0.04 m → `corrections` 不增加、不重放；相差 0.06 m → 回滚并重放，`corrections` +1；重放后误差 0.5 m → 触发平滑且 100 ms 后残余 offset < 5%；误差 1.2 m → `snaps` +1。（阈值 0.05 m / 1.0 m，来自 R5。）
2. vitest 回放测试 `replay_recorded`：用 `sim-wasm` 的 `Predictor` 同时充当"服务器"（同输入、按 ack 延迟 100 ms、丢弃 2% 的快照），静止 60 s、走动 60 s 两个场景：**静止时纠正 ≤ 1 次/10 s**（即 60 s 内 ≤ 6 次）；走动场景 `snaps == 0`。
3. 实机验收（需运行服务器 + `tc netem`，参考机/人工）：`tc qdisc add dev lo root netem delay 50ms loss 2%`（RTT ≈ 100 ms、2% 丢包）下，站立静止 120 s 叠层 `corrections ≤ 12` 次（≤ 1 次/10 s）；跑动 120 s `snaps == 0`；结果与命令写入 STATUS（E0-8）。
4. 输入：`InputCmd` 编码与键盘状态映射表（vitest）：W 前进 → `move_z=+127`；W+D → 模长归一后 `|move| ≤ 127`；Shift = walk 位（M0-011 约定）；`buttons` 位序与 `protocol` 常量一致（引用同一常量表，禁止复制数字）。
5. 插值：10 个远端实体以 20 Hz 快照、10% 抖动、2% 丢包输入，匀速直线运动；渲染采样（60 fps，60 s）中 `|Δpos_frame − v·dt| / (v·dt)` 的 P99 ≤ 5%，且 0 次位置回退（单调性）。
6. `unacked-ring` 容量 128，溢出时丢最旧并记录日志，不抛异常；`seq` 环绕（65535→0）处正确。
7. 主线程开销：`game` 更新 + 预测 0–2 步 + 解码 + 插值（60 实体合成）P99 ≤ 2.1 ms（§13.5.1 的 1.5 + 0.6，参考机/本机，记录机型）。

## 测试要求
- 单元测试：对账阈值边界、平滑指数衰减数值（τ = 40 ms 下 100 ms 残余 ≈ 8%——以实现为准并断言）、插值环、`seq` 环绕、Pointer Lock 丢失时按键位清零。
- 集成/无头测试：验收 2（Node + wasm）与验收 3（实机）。
- 性能基准：验收 7。
- 回归：把验收 2 的录制快照序列存 `client/tests/fixtures/`，供后续改动回放。

## 性能预算
CPU：输入+`game` 更新 ≤ 1.5 ms（含预测约 0.3 ms）、快照解码+插值 ≤ 0.6 ms（ARCHITECTURE §13.5.1）；上行 ≈ 22 kbps ≤ 32 kbps（§6.9.5，叠层显示实测）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **阈值已裁定**：INDEX 已改为"误差 < 0.05 m 不回滚"，与 ARCHITECTURE §6.4 及总控裁定 R5 一致。
- native 与 wasm 的浮点可能不比特一致（R5/T1）：真实服务器对拍会偶发小纠正，验收 3 的 12 次上限就是为此留的余量；若超标，先看 M0-029 的漂移报告再动阈值。
- 验收 2 里的 wasm-vs-wasm "服务器"无法暴露 native/wasm 差异，因此验收 3（实机）不可省；缺环境时记 `未测`，任务至多 `partial`。
