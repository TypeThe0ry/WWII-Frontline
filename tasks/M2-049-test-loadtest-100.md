# M2-049 测试：100 客户端压测（WT 与 WS）

## 元数据
- 里程碑：M2
- 工作流：test
- 规模：L
- 前置依赖：M1-068, M2-021, M2-029, M2-020, M2-048
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.2; ARCHITECTURE §6.9.5; ARCHITECTURE §6.10; ARCHITECTURE §6.6.3; ARCHITECTURE §18.3; MILESTONES §3.3

## 背景
M2 的核心退出标准 E2-1/E2-2/E2-3/E2-15（MILESTONES §3.3）。ARCHITECTURE §17.2：`headless-client`（复用 `protocol`/`sim`）连接真实 `server`，模拟 100 客户端：合成输入（路径点行走 + 随机开火）、解码快照，校验 ①下行字节分布（P50/P99，§6.9 验收线）；②对账误差；③延迟与丢包注入（`tc netem`：RTT 40–150 ms、抖动 20 ms、丢包 0/2/5%）；④连接保持 60 分钟。通过标准（M2）：100 客户端 × 30 分钟，`tick_ms` P99 ≤ 28 ms，`tick_overrun` 无连续，末 10 分钟 RSS 增长 < 2%；稳态 ≤ 18 ms（§6.10）；WS 兜底路径同样压测，阈值放宽：P99 ≤ 40 ms tick、快照 15 Hz。
带宽验收（§6.9.5）：每客户端下行 P50 ≤ 128 kbps、P99 ≤ 400 kbps，5% 丢包典型 ≤ 68 kbps，上行 ≤ 32 kbps（不含语音）。带宽调节器（M2-020，§6.6.3）：`PACKET_BUDGET` 缩放 ≥ 40%，弱网下 2 Hz 下限不被饿死（E2-15）。M1-068 已做 20 客户端联测；M2-021 做 100 玩家 CPU 优化；M2-029 做 bot 规模；M2-048 提供监控看板（作为观察工具与告警验证）。"100 headless（含 bot）"：真人客户端数 + 服务器 bot 合计 100，压测须覆盖"100 headless 真连接"与"混合（例 60 headless + 40 bot）"两种配置。

## 具体内容
### 要创建/修改的文件
- `crates/headless-client/src/{main.rs,scenario.rs,metrics.rs,netem.rs}`（扩展 M1-068）—— 100 并发客户端、场景脚本（部署→行走据点→开火→死亡重生循环）、每客户端指标（下行字节、对账误差、RTT、快照间隔）、WT/WS 切换、汇总 JSON。
- `tools/loadtest/run_100.sh`、`netem_matrix.sh`、`leak_check.sh` —— 一键运行脚本：启动 server+gateway（docker compose）、应用 `tc netem` 矩阵（RTT 40/80/150 ms × 抖动 20 ms × 丢包 0/2/5%）、30 min 运行、RSS 采样（`/proc` 或 `/metrics`）、报告生成。
- `tools/loadtest/analyze.py` —— 解析 Prometheus 导出与 headless 汇总，生成 `reports/m2-load.json` 与 `reports/m2-load.md`（P50/P99/最大 tick、连续 overrun、RSS 曲线斜率、带宽分布）。
- `.github/workflows/loadtest.yml`（修改）—— nightly 15 min 版本（ARCH §17.5：`loadtest` 100 客户端 15 分钟，< 30 min）；30 min 完整版手动触发。
- `crates/headless-client/tests/smoke_100.rs` —— 5 min 冒烟（PR 不跑，手动）。
### 要实现的行为
1. 配置 A（WT）：100 headless × 30 min；配置 B（WS 兜底，禁用 WT）：100 headless × 30 min；配置 C（混合 60 headless + 40 server bot）：30 min。
2. 场景含：小队创建、Garrison/Outpost/建造（M2-010/011）、载具搭乘（M2-015，如已就绪）、Frontline 据点占领，使 tick 压力接近真实（40 载具上限的压测在 M3-045）。
3. netem 矩阵：RTT {40, 80, 150} × 丢包 {0, 2, 5}% 各运行 ≥ 5 min（可用 20 客户端缩小，主 30 min 运行用 80 ms/2% 一档）；记录 `PACKET_BUDGET` 缩放系数与 2 Hz 下限（被饿死计数 `aoi_starved_total`）。
4. 泄漏检测：末 10 min server RSS 线性回归斜率换算增长 < 2%；headless-client 自身泄漏不计入。
5. 结果归档到 `reports/m2-load.json`（含机器型号、命令、git 哈希、DATA_HASH、PROTOCOL_VERSION），摘要写入 STATUS。
### 接口与数据
```json
{ "config": "wt|ws|mixed", "clients": 100, "duration_s": 1800, "tick_ms": {"p50":0,"p99":0,"max":0,"overrun_consecutive_max":0}, "down_kbps": {"p50":0,"p99":0}, "up_kbps": {"p99":0}, "rss_growth_last10min_pct": 0.0, "netem": {"rtt_ms":80,"loss_pct":2,"budget_scale_min":0.0} }
```

## 不做什么
- 不做 M3 全功能（40 载具 + 全部能力 + Assault）压测（M3-045）；不做 LLM 指挥官压测。
- 不做语音压测（LiveKit 容量单独验证，M2-025 的 T8）；headless 不含语音。
- 不做服务器优化本身（M2-021）；本任务发现超预算时写 PLAN_ISSUES 并回报，不自行改 sim 规则。
- 不做生产环境压测（M4）。

## 验收标准
1. WT 配置 A：100 headless × 30 min，`tick_ms` P99 ≤ 28 ms、稳态均值 ≤ 18 ms、最大连续 `tick_overrun` = 0（`/metrics` 直方图 + `reports/m2-load.json`）。
2. 内存：末 10 min server RSS 增长 < 2%（`leak_check.sh` 线性回归）。
3. 带宽（配置 A，无丢包）：每客户端下行 P50 ≤ 128 kbps、P99 ≤ 400 kbps；上行 P99 ≤ 32 kbps（不含语音）；5% 丢包档典型场景下行中位 ≤ 68 kbps（§6.9.4）。
4. WS 配置 B：100 客户端 × 30 min：`tick_ms` P99 ≤ 40 ms，快照频率 ≥ 15 Hz（客户端测得），连接 0 次非预期断开。
5. 带宽调节器（E2-15）：netem 弱网档（150 ms/5%）下 `PACKET_BUDGET` 缩放系数 ≥ 0.4（记录最小值），任何客户端 2 Hz 下限不被饿死（`aoi_starved_total` 在下限实体上增量 = 0）。
6. 配置 C（混合）：`tick_ms` P99 ≤ 28 ms，bot 阶段耗时 ≤ 5 ms/tick（`bots_tick_ms`，E2-4）。
7. 对账：headless 统计的预测误差 > 0.05 m 触发回滚次数/客户端/min ≤ 阈值（记录基线，若 > 10 次/min 写 PLAN_ISSUES，R5）。
8. 报告：`reports/m2-load.{json,md}` 含上述全部数字及机器型号/命令；并在监控看板（M2-048）有对应曲线截图记录路径。

## 测试要求
- 单元测试：headless 场景状态机；指标聚合（分位数算法 vs 已知分布）；netem 命令生成（参数校验）。
- 集成/无头测试：本任务本身即集成测试；先 5 min 冒烟（20→100 渐进，每 30 s +10）再正式 30 min。
- 性能基准：`reports/m2-load.json` 作为后续回归基线（M3-045 对比）。
- 回归：M1-068 的 20 客户端联测继续通过（P99 ≤ 28 ms）。

## 性能预算
- ARCHITECTURE §6.10：稳态 ≤ 18 ms/tick，P99 ≤ 28 ms，绝不连续 2 tick 超 33 ms；§6.9.5：下行 P50 ≤ 128 / P99 ≤ 400 kbps，上行 ≤ 32 kbps；§17.2 WS 阈值 P99 ≤ 40 ms；容量估算（§6.10 末）单局 ≈ 1.0–1.5 核等效，以本压测校准。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`、压测脚本）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 需要参考服务器（≥ 4 物理核、≥ 3.5 GHz，ARCH §6.10 基准硬件）与独立的压测机；租用与成本需人类批准（H-067、H-016）。无参考机则相关条目 `未测`，任务至多 `partial`，不得用开发机数字顶替。
- 压测机与 server 分机运行，否则 CPU 争用污染 tick 数据；`tc netem` 需 root，在容器内运行时注意权限。
- 若 P99 超标：先看 M2-021 的分段指标定位阶段（AOI/编码/bot），写 PLAN_ISSUES，Fable 可降 `PACKET_BUDGET`/AOI 参数（MILESTONES §3.8）。
- WT 压测需受信任证书或 `serverCertificateHashes`；headless 使用 `wtransport` 客户端，不涉及浏览器限制。
