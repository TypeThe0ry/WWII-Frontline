# B6-scoring / M2-017（2026-11-20）

- 在 sim 实现数据驱动计分、按 tick 防刷与滚动限额、兵种归属、分项统计、对局与兵种 XP。C4 仍是 v4 的相同消息长度，限制队伍内传输；WASM / client 同步解码。结果回投保留旧 `rows` 并携带 `score_summary`，旧 spool 兼容。
- 47 条表测试（8 项 M3 禁用）；加速 36,000 tick、5v5 + 2 bot、145 条增量、净分 1,267，逐人逐类无差异。`cargo bench --locked -p frontline-sim --bench score_tick` 的 P50/P95/P99/max 为 4.750/6.970/13.930/626.574 µs。详细命令、负载范围见 [报告](../../reports/m2-scoring-b6.md)。
- 工作站（PI-109，非参考机）；参考机、对标和真人调参未测。前置 M2 的真实生产者、历史快照、跨局 XP 与解锁仍未接入。无新增素材；计分板及结算页视觉待 Fable 设计。
- 首轮 Carentan WS 长跑发现生产战术服未启动 Warmup，虽 10/10 部署但未计分。已补接线并从生产构造器复验完整 36,000 tick 的加速对账；覆盖排除 bot 的 2 人提前开赛与 Sandbox 原行为。
- 同队复用槽位先清旧分；网关不以仿真槽位归属账号。新版 spool 放 `scoring-v1`，新版可读旧格式，旧版不得接管新子目录。保持停服回投，不含自动轮转。数据哈希滚动部署限制见 `deploy/staging/runbook.md`；预发布、受信任 TLS 未测。
- `scripts/loadtest-m0.sh --transport ws --map carentan/m1 --mode assault --deploy-hq --count 10 --bots 2 --duration 1225 --seed 1 --no-build` 复验退出 0；36,773 个服务端 tick，10/10 部署，0 断线/解码错误，36,000 个计分 tick 后自然结算（US 胜），12 条参与记录合计 1,560 净分。未采集 C4 每条网络增量；145 条/1,267 分的逐条对账是独立的加速审计。
- 并行任务带宽调节合入后重跑加速审计，数字不变；计分微基准 P50/P95/P99/max 为 5.660/6.470/8.510/290.177 µs，6,947,322 分对账一致。峰值超过 50 µs，并非参考机预算通过。
- 变基到 `d4887f4` 后继承协议 v5，`DATA_HASH=0x29aaf80610f4e251`。接入 M2-018 权威 AFK，换队保留账本，平衡读取实分；v4 长跑与 v5 门槛分别记录。
- 最终 `scripts/local-ci.sh` 六组均绿（lint/rust/data/determinism/wasm；重建 v5 WASM 后单独复跑 ts：766 项，见报告）。v5 Carentan WS 65 s 冒烟 10/10 部署、0 断线/解码错误，1,993 计分 tick/12 条记录，主动停服无自然赛果。严格确定性 3×18,000 tick 哈希及位置相同；未跑 GitHub Actions。
