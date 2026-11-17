# M0-030 测试：M0 性能基线与报告

## 元数据
- 里程碑：M0
- 工作流：test
- 规模：M
- 前置依赖：M0-024, M0-027
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §13.5; ARCHITECTURE §20.1; ARCHITECTURE §13.1; MILESTONES §1.3

## 背景
把 M0 的性能数字固定成**可重复的基线**：①客户端帧时间（RTX 3060 / 1080p / WebGPU，与 Iris Xe 级 / WebGL2 / 720p），②Rust 微基准（KCC 单次 µs、快照编码 µs），③服务器 `tick_ms`/AOI+编码耗时（来自 M0-027 报告）。规则（README §5.5）：基准必须可重复——固定种子、固定机位、固定参考机；记录机器型号与命令；**没有参考机时标 `未测`，不得用别的机器数字顶替后标 done**。预算对照：客户端 CPU ≈ 10.4 ms/帧、GPU ≤ 14 ms 目标（High，60 fps，§13.5.1/§13.5.2）；E0-5：RTX 3060/1080p ≥ 60 fps、Iris Xe/WebGL2/720p ≥ 30 fps。本任务只**测与记录**，不优化（优化若必需，开新 PLAN_ISSUES 并交 Fable 裁决）。

## 具体内容
### 要创建/修改的文件
- `tools/perf/capture.mjs`（Playwright：启动 Chromium 的 WebGPU/WebGL2 配置，打开 `?perfcam=N&debug=1`，预热 5 s，采集 30 s，调用 `window.__perf.dump()` 写 JSON）、`tools/perf/report.mjs`（聚合多次运行 → Markdown + JSON）、`tools/perf/README.md`（**参考机测量流程文档**：机型清单、驱动/浏览器版本记录项、启动参数、省电/热节流注意、结果文件命名 `perf-{machine}-{renderer}-{cam}.json`）。
- 客户端：`?perfcam=1|2|3` 三个固定机位（开阔地形、植被密集、山脊远眺，坐标写在 `client/src/render/perf-cameras.ts`，固定相机朝向与时间、固定植被种子、关闭动态分辨率）；`?perfclock=fixed`（固定时间步用于可重复）。
- Rust 基准基线：`reports/baselines/{sim-m0.json,protocol-m0.json,server-m0.json}`（criterion 结果摘要）；`scripts/bench-m0.sh`（运行 `cargo bench -p sim kcc_step kcc_100_players_tick ballistics_step`、`cargo bench -p protocol snapshot_encode_client snapshot_decode`，导出 JSON）。
- CI：`bench` 作业（`workflow_dispatch` + nightly，**不进 PR 必跑**）运行 criterion 并上传 artifact，对基线给出回归提示（> 15% 变慢仅警告）。
- `reports/m0/perf-baseline.md`（汇总报告）。
### 要实现的行为
1. **客户端采集**：每个机位、每条渲染路径各 3 次，每次 30 s；导出 `{cpu_ms:{p50,p95,p99}, gpu_ms:{p50,p95,p99,available}, fps:{p50,min}, draw_calls, triangles, renderer, adapter, resolution, quality, cam, git_sha, date}`；3 次取中位数并给极差。
2. **目标机型**：RTX 3060（1080p，WebGPU）与 Iris Xe 级核显（720p，WebGL2，Low 档植被）；若有其它机型（如 GTX 1650 Medium）同法记录但不判定。机型/CPU/驱动/浏览器版本/操作系统写入报告。
3. **Rust 微基准**：KCC `kcc_step`（µs）、`kcc_100_players_tick`（ms）、`snapshot_encode_client`（µs/客户端）、`snapshot_decode`（µs）、`ballistics_step`；机型与 `rustc` 版本记录；与 ARCHITECTURE 预算对照（KCC ≤ 20 µs / 3 ms；编码 ≈ 25 µs）。
4. **服务器数字**：从 `reports/m0/loadtest-wt.json`（M0-027）引用 `tick_ms` P50/P99、`aoi_encode_ms`、`bytes_out_per_client`，**不重跑**。
5. 报告结论段：逐条对照 E0-3、E0-5 的数字（达标/不达标/未测），给出 3060 与 Iris Xe 的预算差距与前三个耗时来源（来自叠层的 GPU/CPU 分项或 Babylon Inspector 采样）。
### 接口与数据
- `window.__perf.dump()` schema 来自 M0-020/024（`vegetation` 段已含）；本任务只增 `meta{machine,commit}` 字段（可由 capture 脚本注入）。

## 不做什么
- 不做优化与画质分级（M4-017/018/041）；不做 M1 三机位（M1-069）与四档矩阵（M4-041）。
- 不做视觉回归（M2-050）；不做长时间浸泡（M4-038）。
- 不重新做服务器压测（M0-027 已做）。

## 验收标准
1. `tools/perf/capture.mjs` 在一台机器上一键运行，对 3 个机位 × 所选渲染路径产出 JSON；缺 GPU 时给出清晰报错或自动跳过并在报告标 `未测`。
2. 报告 `reports/m0/perf-baseline.md` 含：**RTX 3060 与 Iris Xe 的帧时间 P50/P99**（或明确 `未测` 与原因）、draw call、fps；三次重复的 P50 极差 ≤ 5%（可重复性判据；超出需注明环境干扰）。
3. E0-5 判定：RTX 3060 / 1080p ≥ 60 fps（P50）、Iris Xe / WebGL2 / 720p ≥ 30 fps（P50）——达标/不达标/未测三选一，附数字。
4. Rust 基准基线：KCC `kcc_step` 中位 µs、`kcc_100_players_tick` ms、`snapshot_encode_client` µs 三个数字写入 STATUS 与 `reports/baselines/*.json`；与预算（20 µs / 3 ms / ≈25 µs）对照结论明确。
5. `tools/perf/README.md` 使他人（无上下文者）能照做：包含准备步骤、命令、预期输出文件、常见问题（省电模式、后台节流、浏览器版本）。
6. `bench` 作业能在 Actions 手动触发并上传 criterion artifact；对一次人为变慢的改动（基准对比测试，可在本地模拟）给出警告。

## 测试要求
- 单元测试：`report.mjs` 聚合/百分位计算（vitest 或 node:test）、`perf-cameras` 机位固定性。
- 集成/无头测试：`capture.mjs` 在无 GPU 的 CI（SwiftShader）上能跑通冒烟（只验证脚本，不记录数字）。
- 性能基准：本任务即基准。
- 回归：基线 JSON 入库，M1-069、M2-052 作为对照。

## 性能预算
对照 ARCHITECTURE §13.5：CPU 帧 ≈ 10.4 ms（High，60 fps）、GPU ≈ 12.0 ms/≤ 14 ms 目标；地形 ≤ 2 ms；draw call ≤ 400（M0-024）；KCC ≤ 20 µs/3 ms、编码 ≈ 25 µs（§6.10）。本任务**不放宽任何数字**，不达标写 PLAN_ISSUES。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 参考硬件（RTX 3060、Iris Xe 级机器，或能远程跑 Playwright 采集的方式）是**人类输入**（NEEDS_HUMAN H-021）；缺失时本任务只能 `partial`。
- 无头 Chromium 启用 WebGPU 的启动参数因平台而异（**未核实**），以所在机器的 Chromium 文档为准并写进 README；不要假设 CI 机器有 GPU。
- 云 VM 的 GPU/热节流会使 P99 抖动，报告要记录环境并给出 3 次重复的极差。
