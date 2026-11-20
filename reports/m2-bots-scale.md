# M2-029 Bots 规模化工作站验收

日期：2026-11-20。以下本地性能数字按 PI-109 标为工作站、非参考机；CPU 参考设备仍未测（H-021）。基准固定 M0 碰撞地图、100 bot 种子 `0x0295ca1e`，不能代替 M2-027 Frontline 目标、M2-028 指挥官或正式地图整局验收。两项前置依赖当前仍为 `todo`。

## 实现

- `BotRuntime` 每 tick 重建共享感知索引并计划所有到期 bot。预测决策成本使用复用的最大堆，高成本优先，成本相同时优先较早延期者、再按 PlayerId 升序。初始估计 60 µs；完成思考后更新为带 25% 余量的衰减高水位，单 bot 估计上限 120 µs。超时的下一次预测调至 120 µs。准入前校验剩余 3 ms（≤60 个视图玩家）或 5 ms（更大视图）tick 预算；未入选的工作不调用决策并顺延。实测计时截止和操作系统抢占意味着这仍是协作式预算，不是硬实时保证。
- 路径预算器以 1.0 ms/tick 为默认值，持有可恢复 A* 搜索状态；难度初值移至 `data/bots/difficulty.json`，分队及逐 bot 配置，纳入 `DATA_HASH`。客户端与服务端共同数据哈希为 `0x61bd21f4bfc09fe2`；协议仍为 v11。
- LoS/识别仍由每 bot 执行；开火前再次验证当前敌方可见性。指标为 `bots_tick_ms`、`bots_path_ms`、`bots_perception_ms`、`bots_tactical_ms`、`bots_deferred_total`、`bots_think_timeout_total` 和 `bots_stuck_total`。

## 工作站结果

命令：`CARGO_BUILD_JOBS=8 FRONTLINE_REQUIRE_TERRAIN=1 cargo bench --locked -p frontline-bots --bench bots_tick -- --quick`。450 tick 热身，每档 900 个 tick 样本，不含 sim step 或服务器网络传输。

| Bot 数 | Tick P50 | Tick P99 | Tick 最大值 | 路径 P99 | Deferred / timeout |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 60 | 0.041 ms | 0.239 ms | 0.468 ms | 0.200 ms | 0 / 0 |
| 100 | 0.074 ms | 0.331 ms | 0.515 ms | 0.262 ms | 0 / 0 |

两档的本机结果都低于卡片的数值预算，但不计为参考机 E2-4 通过。100 个同时换目标的可恢复路径查询在 5 tick 内全部完成，路径阶段最大 0.992181 ms；用确定性每 tick 两查询上限时完成于 50 tick。该 burst 的计时依赖共享工作站环境。

固定种子 100 bot 对局双跑各 3,000 tick，600,000 条输入经夹具合法性检查，每次均记录 413 发射、116 击杀，末状态哈希均为 `a226709036fb4111`。服务器验证夹具额外执行 300,000 条输入，均被同一 bridge 校验链接受，拒绝计数为 0。

5 分钟、9,000 tick 发现敌人到开火对照：基线 832 对有效样本，优化运行 808；池化中位数均为 18 tick（600 ms），优化运行 3 次决策超时、0 次 LoS 违规开火。**逐 bot 中位数标准未通过**：98 个可配对 bot 里仅 23 个处于 ±10%，配对 bot 绝对相对偏差中位数 38.9%。池化相同不能替代逐 bot 标准；调度相位有意变更，且单种子短样本量较小，需在控制相位的专门 A/B 复验后裁决。

10 分钟、18,000 tick 长跑共 2,494 发射、5 次决策超时、5 次 B-21 卡住恢复；各 bot 卡住次数最大 2，达到但未超过卡片上限。合成灰盒 LoS 阳性/遮挡对照分别产生 6/0 次开火。

命令：`CARGO_BUILD_JOBS=8 cargo test --release --locked -p frontline-bots --test scale_100 -- --include-ignored --nocapture --test-threads=1`。完整基准样本见 `reports/bench/bots_tick_baseline.json`。

## 未测与结论

- RTX 3060 参考服务器 CPU、独占负载及 E2-4 参考硬件验收：未测（H-021）；100 真人/载具/弹体服务器满载和租用硬件：未测（H-067）。
- M2-027、M2-028 前置仍为 todo，未能在 Frontline 目标与规则指挥官的完整玩法链上测 100 bot。
- 逐 bot 识别到开火偏差超出 10% 限值，故 M2-029 为 `partial`。总体 tick 工作站数据不消除该失败，也不代表 M2 里程碑完成。详见 PI-B6-bots100-1。
