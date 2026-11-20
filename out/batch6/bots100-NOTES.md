# B6-bots100 landall 复验

- 基线：`d537011`（M2-009 resources，协议 v12）。
- 合并冲突语义保留：data validator 同时保留 economy、resource_node、bots_difficulty、scoring；sim build 保留 ResourceNode 与 BotDifficulty typed validation；fixtures 两项测试均保留；metrics 同时保留 resource overflow labels 和 bots metrics module。
- 命令：`CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh lint rust determinism ts data`，全部通过。工作区 Rust 与 test-util sim 测试通过；3 个 seed 各 18,000 tick 的 native/WASM 严格对照哈希一致、最大位置误差 0 m；Vitest 778/778；data fixtures 22 valid / 48 invalid，92 文件 0 错误。
- determinism 产物：协议 v12，`DATA_HASH=0x4fd0b9149eac3b4e`（以当前 `reports/m0/determinism/summary.json` 核对）。此次没有重跑 100 bot 性能、逐 bot 延迟或 10 min soak；引用的 bot 性能数字来自既有报告，均是 PI-109 非参考机。
- 复验：rebase 后 `lint rust determinism data` 通过；首次 TS 并行运行中 Vitest worker 因 Node `SIGTRAP` 异常退出，单独重跑 `scripts/local-ci.sh ts` 后 99 个文件、778 项全部通过。`DATA_HASH`/协议版本为当前构建结果。
- 状态仍为 `partial`：逐 bot 延迟标准只有 23/98 配对在 ±10%；M2-027/028 和 H-021 参考机未测。未改变测试阈值。
