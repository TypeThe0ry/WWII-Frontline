# M0-029 测试：确定性与 WASM 漂移

## 元数据
- 里程碑：M0
- 工作流：test
- 规模：M
- 依赖：M0-014, M0-012
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.1; ARCHITECTURE §2.1; ARCHITECTURE §20.1; ARCHITECTURE §17.5; BRIEF R5

## 背景
E0-4：native×2 与 wasm×2 的逐 tick `state_hash` 必须**完全一致**（阻断）；native vs wasm 10 min 随机输入回放的位置 RMS 漂移 < 2 cm/s（**信息性**，超出写 PLAN_ISSUES，不阻断）。依据：T1/R5——WASM 与原生浮点结果**不要求**比特一致，预测与权威按容差对账（> 0.05 m 触发回滚）；因此"native vs wasm 比特一致"**不得**作为阻断项（ARCHITECTURE §17.1、§2.1 T1）。本任务量化漂移，为 M0-022 的对账阈值与 PLAN_ISSUES 提供数据。CI 作业 `determinism`：native 双跑 + wasm 双跑 + 交叉比较，目标 < 6 min（§17.5）。
被比较对象：native 侧 = `sim::World`（含 `PredictSim` 等价路径）；wasm 侧 = `sim-wasm` 的 `Predictor`（M0-014）。输入 = 一份**固定的 10 min 随机输入序列**（30 Hz × 600 s = 18 000 tick），含走/跑/冲刺/蹲/跳/开火/换弹与大幅视角变化，在 M0 地形上进行，包含贴近陡坡与区块边界的路径。

## 具体内容
### 要创建/修改的文件
- `tools/determinism/gen-inputs.rs`（或 `crates/sim/examples/gen_inputs.rs`）：用 `Pcg64Mcg` 固定种子生成输入序列 → `testdata/determinism/inputs_seed{1,2,3}.bin`（`InputCmd` 的 `protocol` 编码，18 000 条/文件，约 200 KB；入库）。
- `crates/sim/tests/determinism_native.rs`（native×2：同输入两次运行，逐 tick 比较 `state_hash`；同时 10 玩家的 `World` 版本）。
- `tools/determinism/{run-wasm.mjs,compare.mjs}`（Node 里加载 `sim-wasm` pkg：wasm×2 逐 tick `Predictor.stateHash()` 比较；导出每 tick 位置 `Float32Array` 到 `reports/m0/determinism/{native,wasm}_pos_seed{N}.f32`）。
- `crates/sim/examples/dump_positions.rs`（native 端导出同格式位置序列）。
- `.github/workflows/determinism.yml`（接入 §17.5 的 `determinism` 作业；PR 触发；用 10 min 输入跑，目标 < 6 min）。
- `reports/m0/determinism/summary.json`（运行产物摘要，入库）。
### 要实现的行为
1. **阻断检查**：native×2 `state_hash` 逐 tick 相同（18 000/18 000）；wasm×2（同一 wasm 二进制，两个独立实例，且第二次在不同的内存分配历史下运行，例如先做无关分配）逐 tick 相同。任一 tick 不同 → 作业失败并打印首个差异 tick 及字段。
2. **信息性漂移**：对 3 个种子分别计算 `err(t) = |pos_native(t) − pos_wasm(t)|`（3D 欧氏距离，两侧从同一初始状态、**无对账纠正**）；输出 `rms_err_m`、`max_err_m`、`drift_cm_per_s`（`err(t)` 对时间的最小二乘斜率 × 100）、`ticks_over_0.05m`（超过 R5 回滚阈值的 tick 数，及占比）；另报告分段（每 60 s）的最大误差，以判断是否随时间发散。
3. **阈值**：`drift_cm_per_s < 2.0` 为目标；超出 → 在 PLAN_ISSUES 写条目（附 3 个种子的数字与首个明显发散的 tick/场景），**作业仍通过**（信息性）。同时输出对 M0-022 对账的建议（若 `ticks_over_0.05m` 占比 > 1%，建议复核阈值或改进 sim 的 libm 使用）。
4. `state_hash` 的组成与 M0-007/011/012 一致（位置/速度/姿态/武器状态/RNG 流位置），不得为了让哈希一致而剔除字段；`Predictor.stateHash()` 与 native 的哈希函数相同但 **跨平台不比较**（仅各自内部）。
5. 报告同时记录 sim 的 `DATA_HASH`、rustc 版本、`rapier3d` 版本、wasm 构建 SHA-256，使结果可复现；Rapier `enhanced-determinism` 状态写入。
### 接口与数据
- 输入文件格式：`[u32 count][InputCmd bit-packed × count]`，读取函数放 `protocol` 测试辅助（`#[cfg(test)]`/dev-dependency）；`summary.json` schema：`{seed, ticks, native_x2:{equal:bool, first_diff_tick?}, wasm_x2:{…}, drift:{rms_m,max_m,cm_per_s,over_0_05m_ratio}}`。

## 不做什么
- 不要求也不测试 native 与 wasm 的逐比特一致；不为一致性改 sim 数学库以外的行为（若发现 `libm` 之外的差异来源，写 PLAN_ISSUES，不在本任务大改 sim）。
- 不做多玩家 wasm 对拍（预测只含本地玩家，ARCHITECTURE §6.4）；不做服务器权威全 World 的 wasm 版本。
- 不做对账逻辑本身（M0-022）。

## 验收标准
1. native×2：3 个种子 × 18 000 tick，**逐 tick `state_hash` 全部相等**（54 000/54 000）。
2. wasm×2：同上 54 000/54 000 全部相等。
3. 漂移指标已为 3 个种子各生成并写入 `reports/m0/determinism/summary.json` 与 STATUS：`rms_err_m`、`max_err_m`、`drift_cm_per_s`、`over_0_05m_ratio`；目标 `drift_cm_per_s < 2`（信息性：不达标则 PLAN_ISSUES，且本验收项标"信息性未达标"，任务仍可 `done` 若其它项通过——按 MILESTONES §1.3 E0-4 的规定）。
4. CI `determinism` 作业在 PR 上运行并通过，墙钟 < 6 min（取 Actions 数字）。
5. 注入故意的非确定性（在测试里临时用 `HashMap` 迭代顺序或 `Instant::now()` 影响状态——仅 `#[cfg(test)]` 的故障注入开关）能使 native×2 检查失败（证明检查有效，1 个用例）。
6. 报告含可复现信息（版本号、哈希）；同一机器重复运行 `summary.json` 的漂移数字逐位相同。

## 测试要求
- 单元测试：`compare.mjs` 的斜率/RMS 计算（用已知序列验证）；输入文件读写往返。
- 集成/无头测试：CI 作业本身。
- 性能基准：记录 18 000 tick 的 native 与 wasm 耗时（用于 M0-030 的 `Predictor.step` 数字交叉）。
- 回归：golden `state_hash` 最终值（native，同目标同工具链内）。

## 性能预算
单个种子 10 min 输入的 native 重放应 < 5 s（KCC ≈ 20 µs × 18 000）；CI 作业总时长 < 6 min（ARCHITECTURE §17.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- "RMS < 2 cm/s"在 ARCHITECTURE §20.1-3 中未精确定义，本任务把它落为"`err(t)` 的线性斜率"并同时报告 RMS 与 `over_0_05m` 占比；这是度量定义的细化，写入 PLAN_ISSUES 供 Fable 确认。
- wasm 与 native 都开启 `enhanced-determinism`；若某特性在 wasm32 下不可用导致两端配置不同，漂移结果需如实标注。
- 若 native×2 或 wasm×2 失败，这是**阻断缺陷**（说明 sim 内有非确定性来源），必须先修再继续 M0-033。
