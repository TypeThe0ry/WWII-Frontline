# M1-068 测试：20 客户端联测

## 元数据
- 里程碑：M1
- 工作流：test
- 规模：M
- 前置依赖：M0-027, M1-024, M1-025, M1-039
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.2; ARCHITECTURE §6.9; ARCHITECTURE §6.10; ARCHITECTURE §6.7; ARCHITECTURE §15.2

## 背景
M1 退出标准 E1-2/E1-3/E1-4/E1-5 的服务器侧实测。ARCHITECTURE §17.2：`headless-client` 连接真实 `server`，合成输入（路径点行走 + 随机开火）、解码快照、校验下行字节分布（P50/P99，§6.9 验收线）、对账误差、延迟与丢包注入（`tc netem`：RTT 40–150 ms、抖动 20 ms、丢包 0/2/5%，M1 取 RTT 40–150 ms、2% 丢包）。预算：服务器 `tick_ms` 稳态 ≤ 18 ms、P99 ≤ 28 ms、**不得连续 2 tick 超 33 ms**（§6.10）；每客户端下行 P50 ≤ 128 kbps、P99 ≤ 400 kbps，上行 ≤ 32 kbps（不含语音；§6.9.5）。AOI 信息控制（R3/§15.2）：视锥外且 > 96 m 的敌人下发数 = 0（E1-4）。延迟补偿（§6.7）：150 ms RTT 下 headless 命中率与 0 延迟基线差 ≤ 5%（E1-5）。M0-027 已有 10 headless + 10 bot 的 30 min 联测；M1 升级为 20 客户端（含 M1 的 5 兵种/坦克/占点逻辑下的合成行为）。

## 具体内容
### 要创建/修改的文件
- `crates/headless-client/src/scenarios/m1_loadtest20.rs` —— 场景：20 个 headless 客户端（每队 10，部分走小队部署 `DeployRequest`，兵种轮换含 Officer/Medic/Support/Crewman），路径点行走+随机开火+占点；其中 1 队 3 人驾驶 Sherman 的脚本；bot 填满空位（M1-039）。
- `crates/headless-client/src/netem.rs` —— 封装 `tc netem` 配置/还原（需 root；非 root 时跳过并标 `未测`），RTT 40–150 ms 均匀分布、抖动 20 ms、丢包 2%。
- `crates/headless-client/src/audit.rs` —— AOI 审计：客户端记录收到的敌方实体精确状态，与自身视锥/距离比对，输出违规计数（视锥外且 > 96 m 的敌人，R3）。
- `crates/headless-client/src/hitrate.rs` —— 命中率对照：固定射手/目标脚本，在 0 ms 与 150 ms RTT 下各跑 N 发，输出命中率差。
- `tools/loadtest/run-m1-20.sh`、`tools/loadtest/report.py` —— 一键运行与报告生成（`reports/m1-loadtest-20.json` 与 `.md`，含 tick_ms 直方图、带宽分布、AOI 审计、命中率）。
- `.github/workflows/loadtest.yml`（nightly，可选）—— 缩短到 5 min 的冒烟版本。
### 要实现的行为
1. 30 min 运行：20 客户端 + 填充 bot 稳定在线，无断线退出；`/metrics` 每 5 s 抓取。
2. 采集：`server_tick_ms` 直方图、`tick_overrun_total`、`net_bytes_out_per_client` P50/P99、上行 kbps、`lagcomp_clamped_total`、AOI 审计违规、RSS 曲线。
3. 命中率对照：150 ms RTT vs 0 ms，各 ≥ 2 000 发。
4. 两路径：WT 为主；WS 兜底另跑 10 min 冒烟（阈值 M2-049 放宽，M1 仅记录）。
5. 报告对照预算表自动给出 PASS/FAIL，失败项列出原因与建议。
### 接口与数据
```text
cargo run -p headless-client -- scenario m1-loadtest20 --clients 20 --duration 1800 --netem rtt=40-150,jitter=20,loss=2 --out reports/m1-loadtest-20.json
```
报告字段：`tick_ms{p50,p99,max}`、`overrun_consecutive_max`、`down_kbps{p50,p99}`、`up_kbps{p50,p99}`、`aoi_violations`、`hitrate{rtt0,rtt150,delta}`、`rss_mb{start,end}`。

## 不做什么
- 不做 100 客户端压测与 WS 兜底全阈值（M2-049）、泄漏检测 < 2%（M2-049）。
- 不做服务器性能优化（超预算时写 PLAN_ISSUES 并拆新任务，不在本任务改架构）。
- 不做真人测试局（M1-072 组织记录，H-054/H-056）。
- 不做浏览器侧渲染性能（M1-069）。

## 验收标准
1. 20 客户端 × 30 min：`tick_ms` P99 ≤ 28 ms，稳态（中位数）≤ 18 ms，无连续 2 tick 超 33 ms（`/metrics` 直方图；INDEX 指标；E1-2）。
2. 下行带宽：每客户端 P50 ≤ 128 kbps、P99 ≤ 400 kbps（E1-3；§6.9.5）；上行 ≤ 32 kbps（P99）。
3. AOI 审计：视锥外且 > 96 m 的敌人精确状态下发数 = 0（E1-4；30 min 全程累计）。
4. 延迟补偿：150 ms RTT 下命中率与 0 延迟基线差 ≤ 5%（绝对百分点，每组 ≥ 2 000 发；E1-5）。
5. 30 min 内无客户端因协议/校验错误被服务器断开（`disconnect_reason` 计数 = 0，除脚本主动退出）。
6. 服务器 RSS 在后 10 min 增长 < 5%（M1 宽松阈值，记录；严格 < 2% 在 M2-049）。
7. 无 netem 权限时：条目 1–3 仍在无损网络下测并标注"netem 条件未测"，任务至多 `partial`。

## 测试要求
- 单元测试：`audit.rs` 视锥/距离判定；报告生成器 PASS/FAIL 逻辑。
- 集成/无头测试：本任务本身即集成测试；另提供 2 min 冒烟版进 CI（`loadtest-smoke`）。
- 性能基准：`reports/m1-loadtest-20.json` 入 `reports/`，STATUS 记录数字与命令。
- 回归：M0-027 的 10 客户端场景保持通过。

## 性能预算
- ARCHITECTURE §6.10：`tick_ms` 稳态 ≤ 18 ms、P99 ≤ 28 ms，不连续 2 tick 超 33 ms；§6.9.5：下行均值 ≤ 256 kbps（M1 验收线 P50 ≤ 128 kbps）、峰值 ≤ 512 kbps，上行 ≤ 32 kbps；§16.5 bot ≈ 3.0 ms/tick。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过；`cargo clippy -D warnings` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"：记录运行机型）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：偏差与预算超标写 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 压测机必须与"参考机"区分记录（CPU 型号、核数）；云容器的 CPU steal 会污染 tick 数据，需记录 `steal%`。
- 载具与 bot 同时在线时 §6.10 阶段 3/6 为风险点；如超标，先查 profile 再决定是否拆 `server-perf` 专项（MILESTONES §2.10 检查点 3）。
- 命中率对照对 RNG/散布种子敏感，使用固定 `spread_seed` 并取 ≥ 3 次重复的平均。
