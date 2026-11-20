# B6-scoring / M2-017（2026-11-20）

- 在 sim 实现数据驱动计分、按 tick 防刷与滚动限额、兵种归属、分项统计、对局与兵种 XP。C4 仍是 v4 的相同消息长度，限制队伍内传输；WASM / client 同步解码。结果回投保留旧 `rows` 并携带 `score_summary`，旧 spool 兼容。
- 47 条表测试（8 项 M3 禁用）；加速 36,000 tick、5v5 + 2 bot、145 条增量、净分 1,267，逐人逐类无差异。`cargo bench --locked -p frontline-sim --bench score_tick` 的 P50/P95/P99/max 为 4.750/6.970/13.930/626.574 µs。详细命令、负载范围见 [报告](../../reports/m2-scoring-b6.md)。
- 工作站（PI-109，非参考机）；参考机、对标和真人调参未测。前置 M2 的真实生产者、历史快照、跨局 XP 与解锁仍未接入。无新增素材；计分板及结算页视觉待 Fable 设计。
