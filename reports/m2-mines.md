# M2-014 Landall 验证补记

## 已验证

- worktree 基于 M2-009 / 协议 v12 基线。`CARGO_BUILD_JOBS=8 scripts/local-ci.sh lint` 通过（cargo-deny、fmt、workspace clippy `-D warnings`、ESLint、TypeScript、依赖与 38 项 CI 工具测试）。
- `CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh rust determinism data` 通过，workspace Rust 与 sim `test-util` 测试通过，data fixtures 21 valid / 47 invalid、92 个 data 文件零错误。
- 严格 determinism：3 个 seed 各 18,000 tick，native/native 与 wasm/wasm 重放相同，native/WASM 哈希逐 tick 相同，最大位置误差 0 m。
- Mine、explosive、launcher、HEAT 定向单测通过。工作站是 PI-109 非参考机。

## 尚未完成

当前代码提供数据定义、规则解析和独立 sim API，但 Minefield、Satchels、LauncherState 与 HeatProjectile 尚未连入 World 的权威 tick、玩家输入、SimEvent 服务端投递或正式武器数据。未声称实现完整可玩闭环，M2-014 状态为 `partial`。M2-012 Engineer 调用方、M2-013 正式武器定义、参考机、参考作品史实核验与真人玩法未测。本任务没有新增素材。
