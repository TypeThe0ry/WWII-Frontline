# B6-gwticket NOTES

- 工作树：`.wt/gwticket`，分支 `b6/gwticket`，里程碑线 `claude/serene-wright-lc7k1n`。
- 日期：2026-11-20。

## 做了什么

- B5-E1 原版 200 并发重跑七轮：P50/P95 28.430/49.357 ms，P95 ≤50 ms 仅 4/7 轮。原报告约 52 ms 对应 P95，不是 P50。
- 500 并发基线：SQLite 排队均值中位数 55.576 ms，事务 0.068 ms，Ed25519 0.040 ms。比较单连接直交接和四连接池后，采用长驻连接及最多 32 个已就绪请求的同昵称游标批写。保留改名、补档、拒绝、持久化与签票安全语义；提交后签票，不等凑批。
- 压测夹具新增并发参数、P99、可选分段指标、进程 PID 与清理。定版 100/200/500 并发各七对，P50 4.748/12.414/25.333 ms，P99 6.219/15.710/29.740 ms（轮次中位数）；500 并发最慢轮 P99 41.436 ms。默认线程数补跑 100/500 各三轮，500 并发 P50/P99 28.780/33.802 ms。定版 7,400/7,400 请求成功，各轮零重复票。工作站非参考机，PI-109。
- 结果写入 `reports/m1-gateway-ticket-b6.{md,json}`，追加 M1-028、E1-1/E1-10 入局前置、PI-B6-gwticket-1、H-017/H-021。

## 验证

- `CARGO_BUILD_JOBS=8 cargo build --release --locked -p frontline-gateway`。
- `TOKIO_WORKER_THREADS=8 python3 scripts/gateway-ticket-load.py <binary> --protocol-version 4 --concurrency <100|200|500> --report-only --metrics`；成对交替运行。参见报告命令与逐轮数据。
- `cargo test -p frontline-gateway --locked`：28/28 通过；`cargo clippy --locked -p frontline-gateway --all-targets -- -D warnings` 通过。
- `CARGO_BUILD_JOBS=8 scripts/local-ci.sh rust determinism`：`pass=[rust determinism] fail=[] skipped=[]`。原生和 WASM 在种子 1/2/3 各 18,000 tick 的状态哈希与位置逐 tick 一致；terrain/WASM 对比 1/1 通过。
- 对齐含 browsergarrison 的基线 `b9fc244` 后，`cargo test --locked -p frontline-sim --features test-util --test spawn_garrison` 14/14；`FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism` 通过，三个种子仍逐 tick 一致。
- 真网关、真比赛服进程联测 `process::real_gateway_process_ticket_and_staleness` 1/1：正常入局，重放/过期/签名错误各拒绝 1 次，断心跳下线 5.000 s，结果提交后正常下线。
- `cargo fmt --all --check`、`git diff --check`、`node tools/ci/check-acceptance.mjs` 均通过。

## 没做与遗留

- 参考机、受信任 HTTPS 预发布、持续负载及混合改名压力、独立 HTTP 报文解析与 DEBUG 写日志 I/O 均未测。M1-028 仍为 `partial`；没有关闭真人/部署 E1。未涉及 UI 或素材；视觉如有后续，待 Fable 设计。
- 无 GPU、无 GitHub Actions；不触碰工作站服务。压测夹具只清理自己启动并记录 PID 的网关。
- 对齐已合入的 variantci、assetlod、browsergarrison，`PLAN_ISSUES.md`、`NEEDS_HUMAN.md` 和 `m1-acceptance.md` 保留双方追加。
