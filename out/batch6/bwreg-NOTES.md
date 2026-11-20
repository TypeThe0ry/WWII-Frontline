# B6-bwreg / M2-020（里程碑线）

- 实现：每客户端 C2 档位 40–100%，QUIC 窗口/RTT、WS 待写/写阻塞、确认率、恢复迟滞；T3/T2 数量裁剪，不修改 R3 或频率公式；固定字段优先，WS 最新快照优先，C4 状态通知限频与 Prometheus 指标。协议 v4 和游戏数据哈希 `0x2637b94dda28a192` 不变，配置在 `crates/server/config/server_net.json`。
- PI-109 工作站（非参考机）：无损逐包对照 700/700；40% 档合成 T3 P99 500 ms、0 饥饿/min；100 客户端微基准 2.389 µs/tick；合成 150 事件/s、30 分钟 C2 估算 P99 88.304 kbps。真实弱网撤除后 WT/WS +9.056/+8.041 s 首次采到满档。真实 WS/WT 的 C4 降档和恢复已解码对照。
- 真 Carentan WT 40% 档诊断：逐包纳入掩码与接收对齐，持续保留的友军 T3 共 299 个到达间隔，P99 503.1 ms、饥饿 0/min（上行丢包 17.87%、下行 0%）。见 `reports/m2-020-bwreg-t3.md`。
- 两轮 Carentan 20 连接各 1,800 s：5% WT 夹具 21.428 kbps、密集 WS 夹具 26.042 kbps（估算 C2 下行均值，两轮 P99 均落在 (32, 54] kbps 桶内）。全程无断连、解码失败、缺失基线；共享机 tick 超时分别 90/479 次，最长连续 2/8 次，不视为 tick 预算达标。8 个短矩阵用例及数据见 `reports/m2-020-bwreg-fixtures.md`。
- 命令：`CARGO_BUILD_JOBS=8 scripts/local-ci.sh ts data`；`FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism`（3×18,000 tick native/WASM 位一致）；`scripts/local-ci.sh rust lint`；`cargo bench --locked -p frontline-server --bench regulator -- regulator --noplot`。真实链路命令见 `scripts/netem/m2-020/README.md`。未使用 GitHub Actions。
- 余项：实服严格 §6.9.2 的逐客户端 50 可见及分层，实服严格 §6.9.3 的 150 事件/s 与载具混合，以及参考机未测（H-021；物理网络 H-022）。无新素材。HUD 视觉待 Fable 设计。详见 `reports/m2-bandwidth-regulator.md`；M2-020 为 `partial`，不改变 M1 判定。
