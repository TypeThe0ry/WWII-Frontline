# M2-021 CPU100 批次记录

日期：2026-11-20。结论：`partial`，没有生成 `reports/m2-perf100.json`，因为本批未运行定义该数字的 tick_100 性能基准或 100 人负载。

## 已实现

- `/metrics` 新增 `server_tick_stage_ms{stage}`，记录现有六段墙钟：`inputs`、`sim_step`、`post_step`、`bots`、`aoi_snapshot`、`send`。该分组是当前 sim loop 的六个粗粒度边界，不等同 ARCH §6.10 的 11 个预算阶段。
- 超限判定抽为 `OverrunGuard`：工作时长严格大于 tick 周期才计超限；3 次连续超限进入降频条件，正常 tick 清除连续计数。现有快照调度器在第三次超限后降到 15 Hz，保持 60 tick，即 30 Hz 下约 2 s。
- tick schedule 继续使用绝对 deadline，错过的 slot 跳过而不追赶。无协议、sim 规则或数据变更。

## 验证与机器

- `CARGO_BUILD_JOBS=8 cargo test -p frontline-server --lib`：149 passed、5 ignored；workspace Rust 门禁同样通过。
- `CARGO_BUILD_JOBS=8 cargo clippy -p frontline-server --all-targets -- -D warnings`：通过。
- `CARGO_BUILD_JOBS=8 cargo test -p frontline-server tick::overrun --lib`：2 passed。
- `CARGO_BUILD_JOBS=8 cargo test -p frontline-server --lib metrics::tests::exposes_required_series_with_enough_buckets`：1 passed。
- `cargo fmt --all --check`：通过（格式化后复验）。
- `scripts/local-ci.sh determinism`：通过；原生/WASM 3 个 seed 各 18,000 tick 完全一致，terrain-vs-sim Vitest 1 项通过。
- `CARGO_BUILD_JOBS=8 scripts/local-ci.sh rust`：通过，包括 workspace 测试及 `frontline-sim --features test-util`。
- `CARGO_BUILD_JOBS=8 scripts/local-ci.sh lint`：通过，包括 `cargo-deny`、workspace clippy `-D warnings`、ESLint、TypeScript、依赖边检查及 38 个 CI 工具测试。
- 新增超限集成断言：`degradation_guard_halves_to_15hz_for_2_seconds_then_recovers` 通过，验证 60 tick 降频窗和恢复后 20 Hz；sim-thread 40 ms 注入恢复单测通过。
- PI-109 工作站：AMD EPYC 7402 双路，每路 24 核；`lscpu` 报告 CPU max 3349.6089 MHz；Linux `7.0.0-14-generic`；`rustc 1.97.0 (2d8144b78 2026-07-07)`。低于 ≥3.5 GHz 基准要求，且为共享工作站，属于非参考机。性能数值未测。

## 未完成 / 未测

- 100 headless、0 bot 与 40 headless + 60 bot 各 30 min；tick P50/P99、连续超限扫描与末 10 min RSS 增长：未测。H-067 未批准租机，本批不启动该压测。
- 11 个 ARCH 阶段的独立 P50、3 线程并行快照编码、AOI 局部排序、`cargo bench -p server tick_100` 前后对比、前五热点 profiling/优化、600 tick 优化前后 `state_hash` 对照、M1-068 20 人回归：未测/未完成。
- M2-008 仍为 todo，当前测量也不具备任务卡要求的完整规则负载。

后续依赖：先完成 11 阶段拆分与编码/AOI 优化，再由获批的参考机执行 M2-021 性能矩阵；完整 100 客户端 WT/WS 与 netem 长跑仍由 M2-049 负责。
