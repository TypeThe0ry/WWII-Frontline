# M1-041 client/net：载具驾驶预测与同步

## 元数据
- 里程碑：M1
- 工作流：net
- 规模：M
- 前置依赖：M1-022, M0-022
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.4; ARCHITECTURE §6.5.2; ARCHITECTURE §5.3; GAMEDESIGN §14.7; GAMEDESIGN §14.1

## 背景
M1 有 1 辆 M4A1 Sherman，3 座位（驾驶/炮手/车长）。ARCHITECTURE §6.4：预测范围包含"本地玩家的移动（含载具驾驶员）"；其它实体使用插值位置（延后 100 ms）。对账规则（R5）：位置误差 ≤ 0.05 m 保持预测，> 0.05 m 回滚到权威状态并重放未确认输入，误差 < 1.0 m 做 100 ms 视觉平滑（τ = 40 ms），≥ 1.0 m 瞬移。载具快照：quat 用 smallest-three 压为 32 bit，仅载具下发 4 B 速度辅助外推（§6.5.2）。GD §14.7：座位切换在停车或速度 < 5 km/h 时 F1–F4 立即切换，移动中切换需 2 s。
本任务让 sim-wasm 的载具步进在客户端驾驶员侧预测，并让乘员座位与远端载具表现正确。

## 具体内容
### 要创建/修改的文件
- `client/src/game/vehicle-predict.ts` —— 驾驶员载具预测：当本玩家为 Driver 时，用 sim-wasm 载具步进替代角色步进；未确认输入环包含载具输入（油门/转向/手刹）。
- `client/src/game/seat.ts` —— 座位状态：本玩家座位（driver/gunner/commander/none）、进出请求与服务器确认、换座冷却（2 s，移动中）。
- `client/src/game/vehicle-interp.ts` —— 远端载具插值（100 ms 缓冲 + 4 B 速度辅助外推，外推上限 150 ms）；炮塔/炮管角度与履带相位字段单独插值。
- `client/src/net/vehicle-codec.ts`（如 sim-wasm 已提供解码则仅胶水）—— 载具实体 `EntityState` 的客户端解码调用。
- `crates/sim-wasm/src/lib.rs`（修改）—— 暴露 `step_vehicle(state, cmd)`、`vehicle_state()`、`reset_vehicle_to(auth)`（与 ARCHITECTURE §8.3 风格一致）。
- `client/test/game/vehicle-*.test.ts`。
### 要实现的行为
1. 驾驶员预测：本地立即应用油门/转向；对账走 §6.4 流程，误差阈值与平滑参数与步兵一致（0.05 m / 1.0 m / 100 ms）。
2. 炮手/车长不预测载具位置（用插值），仅预测本地炮塔瞄准的视觉（炮手视角炮塔转向立即响应，服务器权威炮塔角度到达后平滑校正）。
3. 乘员座位同步：进入/离开/换座通过 C0 请求，服务器确认前显示"请求中"；上下车期间摄像机过渡由 render 层处理，本任务只发状态。
4. 载具内玩家不运行角色 KCC 预测（避免双重预测）；下车时从服务器权威位置重建角色预测状态。
5. 载具碰撞对撞步兵的结果以服务器为准；客户端不预测对其它实体的推挤（§6.4）。
6. 调试：叠层显示载具对账次数与误差（仅调试构建）。
### 接口与数据
```ts
export interface SeatState { seat: 'none'|'driver'|'gunner'|'commander'; vehicleId?: number; pending?: 'enter'|'exit'|'swap' }
export class VehiclePredictor { step(cmd: VehicleInput): void; onSnapshot(auth: VehicleAuthState, ackTick: number): void }
```
`VehicleInput` 字段以 M1-010/M1-022 为准（油门、转向、手刹、sprint）；不得在客户端硬编码车速等数值（ARCHITECTURE §4.4）。

## 不做什么
- 不做坦克渲染与履带视觉（M1-048）、座位 UI（M1-060）。
- 不做载具武器命中表现的细节（M1-023 已有 sim；表现归 M1-047/M1-063）。
- 不做炮手瞄准镜 UI（M3-035）。
- 不做其它载具（Jeep 等，M2/M3）。

## 验收标准
1. 120 ms RTT、2% 丢包（`tc netem`）下，直线行驶 60 s，驾驶员载具位置校正（回滚+平滑）次数 ≤ 6 次（即 ≤ 1 次/10 s）；用 headless/Playwright 采集调试计数。
2. 同条件下单次校正最大视觉位移 < 0.3 m（采样记录）。
3. 乘员（炮手/车长）视角下载具位置插值抖动：相邻帧位移二阶差分 RMS < 2 cm（同场景）。
4. 座位切换：停车时 F1–F4 在 1 个往返（≤ RTT+1 tick）内确认；移动中 2 s ±1 tick 生效（vitest 状态机测试 + 服务器配合场景）。
5. 对账确定性：对同一份录制快照+输入，两次回放的预测状态逐帧一致（vitest，容差 1e-5 m）。
6. 上下车切换后角色位置与服务器权威位置偏差 ≤ 0.05 m（`exit_vehicle_resync`）。

## 测试要求
- 单元测试：座位状态机全转移；外推上限；回滚阈值边界（0.05/1.0 m）。
- 集成/无头测试：`crates/headless-client` 驾驶 Sherman 直线+转弯脚本 60 s，对比服务器权威轨迹误差；netem 条件见验收 1。
- 性能基准：载具预测单步 ≤ 0.05 ms（vitest bench 或 wasm 内计时）。
- 回归：M0-022 步兵预测测试通过。

## 性能预算
- ARCHITECTURE §13.5.1：主线程预算内，预测+对账合计不超过既有"预测"行；载具单步 sim-wasm ≤ 50 µs。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、`cargo test -p sim-wasm`）；`clippy`/`eslint`/`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：sim-wasm 新增 API 回写 ARCHITECTURE §8.3"变更说明"并记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 载具预测对 sim 物理的 WASM/原生差异敏感（R5 容差对账，不要求逐比特）；若校正频率超标，先检查悬挂射线数与地形碰撞数据一致性，再写 PLAN_ISSUES。
- 多座位下"谁预测"以服务器 `Welcome`/座位事件为准；驾驶员掉线时由服务器清座。
