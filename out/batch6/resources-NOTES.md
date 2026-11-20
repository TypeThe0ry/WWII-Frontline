# B6-resources · M2-009

整分钟资源结算、原子扣费与退款、节点产出及销毁、Supplies 拆除返还、燃油维持和上限指标已接入。地图读入可选 `resource_sites`。v12 客户端按能力位订阅己方绝对 C2 投影和可靠 C4 变化原因；`ResourceDelta` 使用 C4 `0x11` 避开 M2-011 的 `0x10`，经济数据参与 `DATA_HASH`。

- `cargo test -p frontline-sim --locked --features test-util --test resources_economy` · 12 项。首分钟双队 242 / 98 / 50，节点组合收入 92 / 28 / 16。
- `cargo test -p frontline-server --locked --test resources_wire` · 真实 WS，5v5，推进到 tick 9,000，50 次逐客户端 C2 核对和 150 条 C4 收入匹配。
- `cargo bench -p frontline-sim --locked --bench resources_tick` · PI-109 工作站 100 人、18 节点、3,600 tick，P50 4.600 µs、P99 7.640 µs、最大 34.859 µs。
- `FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism` · native / WASM 3×18,000 tick 哈希和位置一致。

参考机与完整阶段 5 联合预算未测，见 H-021。里程碑基线缺少 M2-008 正式五线地图联验，状态 `partial`。M2-010 消费完工与节点受损 API，M2-015 消费维持注册和低燃料 API。资源 HUD 待 Fable 设计。本任务素材需求 ID 为无。

逐项证据见 `reports/m2-resources.md`。规则与接口决定见 PI-B6-resources-1。

## B6 landall 复验（2026-11-20）

- 基线：`8be586c`（M2-011，协议 v12）。没有额外功能改动；M2-009 保持 `partial`。
- 命令：`CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh lint rust determinism ts data`；全部通过。Rust 全工作区与 test-util sim 用例通过；Vitest 778/778；data fixtures 21 valid / 47 invalid，91 文件 0 错误。
- 命令：Binaryen `version_132` 下 `CARGO_BUILD_JOBS=8 scripts/local-ci.sh wasm`；通过。优化 WASM 1,191,370 B（上限 1,200,000），Brotli 344,816 B（上限 400,000）；native/WASM 回放 3,000 tick 逐 tick 一致。
- 未测：参考机和 M2-008 五线地图联验、参考作品/真人资源规模校准。所有工作站结果均为 PI-109 非参考机。人工进展记于 H-021/H-060。
