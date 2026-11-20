# B6-pingchat / M2-019

- 实现 C0 Ping/Chat/Notice、C2 标记与过期，以及现有 C3 地图标记投影。标记权限、配额、5 m 网格、1 Hz 频控、LOS 识别、击倒好友时的 Need Medic 和受信任的静音上报由服务器驱动。
- 聊天支持 200 个 Unicode 标量及 NFKC。控制符拒绝，Command 同队且要求指挥角色。举报可以保存前后各 20 条接收者可见的聊天内容，采集后半段最多 60 s。
- `cargo bench -p frontline-sim --bench comms_tick`（PI-109 工作站，非参考机）在 200 标记 / 100 玩家夹具中测得稳定路径 P95 0.76 µs，12 次更新扇出 P95 9.09 µs，缓存过期路径 P95 3.82 µs。穷举扫描 P95 55.82 µs。
- 10 个本机无头 WebSocket 会话。矩阵 700/700，M2-019 详细边界及偏差见 `reports/m2-019-pingchat.md`。
- 参考机、真人、正式证书及当前无席位的角色实局未测。视觉待 Fable 设计。无素材需求和缺失 ID。实时管理员面板沿用 100 条上限。
- 合入 M2-018（基线 `d4887f4`）后，`CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh wasm determinism lint ts data` 中 `wasm`、`determinism`、`data` 通过。修复换队订阅与 v11 Hello 断言后，`scripts/local-ci.sh lint ts` 复跑全绿，客户端 99 文件、772 项测试通过。数据校验检查 80 文件，0 错误。
- 严格确定性 3 × 18,000 tick，原生与 WASM 每 tick 的哈希及位置逐位一致。WASM smoke 另有 3,000 tick 逐位一致。协议 v11，合并数据哈希 `0xfa5af0f0d3f5a999`。用仓库锁定的 Binaryen 132 复跑 `wasm` 和严格 `determinism` 均通过。WASM 优化后 1,189,992 B（上限 1,200,000 B），Brotli 343,064 B（上限 400,000 B）。
- C3 保留旧地图的取整口径。换队清除旧队投影。新增服务器规则禁言回归，`cargo test --locked -p frontline-server --test c0_handler` 8/8 通过。
- `CARGO_BUILD_JOBS=8 scripts/local-ci.sh rust` 全绿，包含 workspace 和 `frontline-sim --features test-util`，合计 1,456 项通过、24 项跳过。最终服务端专项复跑覆盖库测试、C0、C3、10 会话 Ping/Chat 和 2 组 M2-018 联测，共 157 项通过，4 项手工测试跳过。
