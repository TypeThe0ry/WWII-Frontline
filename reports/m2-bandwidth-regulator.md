# M2-020 带宽调节器（B6-bwreg，2026-11-20）

**状态：`partial`。** 以下实测均为 PI-109 共享 EPYC 7402 工作站，非参考机；不使用 GPU。没有购买、素材或真人依赖。真实链路与调度器合成负载的数字分别列出，不将其合并成完整场景验收。

| 卡内验收 | 工作站结果 |
| --- | --- |
| 1. 典型 50 可见玩家、5% 丢包、20 连接、30 min，均值 ≤68 kbps | Carentan WT 20 headless + 40 bot，设定双向 5% / 40±20 ms，1,800.001 s；35,880 个 C2 `wire` 估算样本均值 21.428 kbps。每位接收者恰有 50 个可见玩家及 4/10/14/22 各档分布的完整工况未测。见 [传输夹具](m2-020-bwreg-fixtures.md)。 |
| 2. 5% 丢包、150 ms RTT、80 kbps，最低 0.40，恢复至 ≥0.95 ≤10 s | 单客户端 M0 WT、WS：设定双向 75 ms / 5%、下行 80 kbit/s；`le=0.399` 零样本，两路都进入 40% 档；撤除后分别 +9.056 s / +8.041 s 首次采到 `net_regulated_clients=0`（单客户端等价于 scale=1）。按设定条件和采样上界报告，非实测精确 RTT 或转换时间。合成测试逐 tick 断言 0.40 下限，解除容量限制后 149 tick（4.967 s）到满档。 |
| 3. 保留 T3 更新 P99 ≤600 ms、饥饿 ≤0.5/min | 合成静态 50 候选（4/10/14/22），连续 60 s 固定 0.40。保留 8 个 T3，解码 952 个间隔、P99 500 ms；饥饿和 R3 泄漏均为 0，14 个被裁 T3 的首次回包为全量。[Carentan WT 诊断复跑](m2-020-bwreg-t3.md)：单 headless、上行丢包 17.87%、下行 0%，按逐包纳入掩码确认持续保留的友军步兵 T3，299 个到达间隔 P99 503.1 ms，无间隔超过 600 ms，饥饿 0/min。WS 及非友军步兵的同口径间隔未测。 |
| 4. 无损保持 1.0，与 M1-024 逐字节一致 | 相同输入逐 tick 比对不启用控制器的 M1 调度路径：WT 400 包、WS 300 包一致，档位从未下降。额外 M0 真链路清网短测 WT/WS 分别采到 32/24 次满档，无降档。 |
| 5. 密集 150 事件/s、30 min，P99 ≤400 kbps | 合成 100 玩家、20 解码客户端、30 分钟仿真时间，精确注入 270,000 个 Impact（150/s）；36,000 个 1 s C2 `wire` 估算的 P99 为 88.304 kbps。真实 Carentan WS 20 headless + 80 bot，1,800.002 s，均值 26.042 kbps，35,615 个样本的 P99 落在 (32, 54] kbps 桶内。实服同时具备规定事件率、载具组合与候选分布未测。 |
| 6. HUD 通知至多 1 条/10 s | 单测观察两轮降档/恢复，相邻通知 ≥300 tick；另用实际 WS/WT 监听器注入确认暂停，恢复确认后两路均正确解码 C4 通知。客户端保留通用事件出口；HUD 视觉待 Fable 设计。 |
| 7. 100 客户端 ≤0.1 ms/tick | Criterion 100 样本估计区间 2.3762–2.4050 µs/tick（点估计 2.3893 µs/tick），包含 ACK/RTT、EWMA、档位更新、字节与数量预算、通知判断；不含传输锁、AOI 选择/编码。参考机未测（H-021）。 |

### 行为与指标

- 两轮 Carentan 长跑均未通过 tick 预算：WT/WS 全程分别 90/479 次超时，最长连续 2/8 次。它们用于观察共享工作站的传输夹具，不用于归因调节器 CPU 成本或判定 M1 合格。

- M1 的 R3、距离迟滞、LoS 与更新频率公式不变。只在降档时对通过过滤的远场集合实施数量限制：T3 向下取整，合计剩余名额给 T2；同档优先保留已入选实体。被裁者不积攒饥饿分，通过既有离开通知移出，恢复有效候选时首包为全量。`aoi_starved_total` 统计 `acc ≥ 2` 且写不进包的记录，不把正常延后一包或裁剪当作饥饿。
- 编码先写头、自身状态和事件，再对实体区应用 40–100% 软预算；传输物理硬上限仍生效。48 事件压力测试中固定字段超过 460 B，事件全保留、实体让位。现有 C3 团队/世界发送不受调节器缩放；WS 阻塞只合并未写 C2，写端还限制恢复后的补发速度。WS 双向字节口径沿用现有指标，不声称读取了浏览器或代理的缓冲字节。
- 指标：`net_budget_scale`、`net_regulated_clients`、`net_far_shed_total`、`ws_snapshot_dropped_total`，并保留 `snapshot_budget_utilization`（可能 >1，固定字段超软预算）。协议 v4 与 `DATA_HASH=0x2637b94dda28a192` 不变；`NET_DEGRADED` / `NET_RECOVERED` 是既有 Notice 中的新语义编号，不新增字段。

### 复现

```bash
export CARGO_BUILD_JOBS=8
scripts/local-ci.sh ts data
FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism
scripts/local-ci.sh rust lint
cargo test -p frontline-server --test bandwidth_regulator -- --nocapture
cargo test -p frontline-server --test bandwidth_notice
cargo test --release -p frontline-server --test bandwidth_regulator dense_150_events_per_second_scheduler_soak -- --ignored --nocapture
cargo bench --locked -p frontline-server --bench regulator -- regulator --noplot
```

严格确定性：3×18,000 ticks，native/WASM 每 tick 哈希与位置全部相同。真实传输复现见 [M2-020 netem 夹具](../scripts/netem/m2-020/README.md)。
