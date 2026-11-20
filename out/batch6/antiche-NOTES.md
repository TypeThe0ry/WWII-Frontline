# B6-antiche NOTES（2026-11-20）

- 线：`b6/antiche` → `claude/serene-wright-lc7k1n`。E1-15 基础反作弊 PASS；M1-027 的真人和 Carentan 实服余项仍 partial。
- 做了什么：视角默认改为每轴 3,600°/s（30 Hz 的 120°/tick），可在 `anticheat.json` 调整。WASM 把步兵主武器预计枪口随 C1 冗余发送；服务端的 `>0.5 m` 计数仅用于诊断，不改枪口、命中或踢出。
- 正常对照：`headless-client --transport ws --anticheat-control --count 20 --duration 1800 --seed 1`，M0，0 bot。20/20 在线 1,800.0318–1,800.0329 s，1,079,914 个 C1 包、4,500 个合成峰值；移动/视角纠正、射速拒绝、踢出、异常断开均 0；另有 198 次普通预测回滚。工作站按 PI-109 标为非参考机。
- 注入复测：`--anticheat-mix --count 20 --duration 300` 仅 2 个预期断开，移动钳制 1,890、视角钳制 1,800、射速拒绝 175。专项 `cargo test --locked -p frontline-server --test anticheat --test lagcomp_transport` 验证 1.3×（180 次钳制，最高 6.343 m/s）、2× 射速（40 拉、20 发、20 拒绝），及 WT 偏离起点提示只作诊断（4/4 命中）。
- 门禁：`CARGO_BUILD_JOBS=8 scripts/local-ci.sh` 的 `lint rust ts data determinism wasm` 相关组通过，native/WASM 3 个种子各 18,000 tick 完全一致。未使用 GitHub Actions 或 GPU。复现和实际计数见 `reports/m1-anticheat-b6.md`，决策见 PI-B6-antiche-1/2。
- 未做：真人高灵敏度轨迹、RTX 3060/Iris Xe 参考机、正式证书和预发布、Carentan 浏览器同图预测未测。未改 UI 或素材；若后续需要视觉告警，待 Fable 设计。
