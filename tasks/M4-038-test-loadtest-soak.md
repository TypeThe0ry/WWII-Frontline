# M4-038 测试：三图 60 分钟浸泡压测

## 元数据
- 里程碑：M4
- 工作流：test
- 规模：M
- 前置依赖：M3-045, M4-031, M4-014, M4-015
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.2; ARCHITECTURE §18.3; ARCHITECTURE §6.10; ARCHITECTURE §6.9; ARCHITECTURE §5.6

## 背景
在 Carentan、Omaha、Foy 三张图上各做一次 **100 headless 客户端 × 60 min** 浸泡，传输分别为 WebTransport 与 WebSocket 兜底（共 3 图 × 2 传输 = 6 次运行，每次 60 min，含重连与断线注入）。ARCHITECTURE §17.2 通过标准（M2 起）：100 客户端 × 30 分钟，服务器 `tick_ms` P99 ≤ 28 ms，`tick_overrun` 无连续，末 10 分钟 RSS 增长 < 2%；**WS 兜底路径同样跑，降级指标放宽：P99 ≤ 40 ms tick、快照 15 Hz**；`headless-client` 注入：延迟与丢包（`tc netem`：RTT 40–150 ms、抖动 20 ms、丢包 0/2/5%）。带宽验收线（§6.9/§5.6）：每玩家下行均值 ≤ 256 kbps、峰值 ≤ 512 kbps，上行 ≤ 32 kbps（不含语音）。服务器 CPU 预算（§6.10）：稳态 ≤ 18 ms/tick，P99 ≤ 28 ms，绝不连续 2 tick 超 33 ms。
M4 版本包含新地图（Omaha 海滩开阔视距、Foy 森林密集）与新模式（Assault/Skirmish）、gateway 编排（M4-031）、统计检测（M4-030）后的总体回归。
指标来源：`/metrics`（`server_tick_ms` 直方图、`server_tick_overrun_total`、`net_bytes_out_per_client`、`net_rtt_ms`、RSS）。

## 具体内容
### 要创建/修改的文件
- `crates/headless-client/src/scenarios/soak.rs` —— 浸泡场景（路径点行走 + 随机开火 + 部署/重生 + 小队行为，含断线重连、延迟/丢包轮换）。
- `tests/loadtest/soak_matrix.sh` —— 3 图 × 2 传输 的编排（经 gateway 编排或直接启动 server），收集 `/metrics` 快照（每 10 s）。
- `tests/loadtest/analyze.py` —— 读取指标时序，计算 P50/P99、连续 overrun、RSS 斜率、连接保持率、带宽分布，输出 `reports/loadtest/m4-soak-{map}-{transport}.json` 与汇总 `m4-soak-summary.md`。
- `tests/loadtest/netem_profiles.sh` —— RTT/抖动/丢包轮换配置。
- CI：nightly job `loadtest-soak`（手动触发或夜间，参考机 runner；非每 PR）。
### 要实现的行为
1. 每次运行：100 headless（模式按地图默认：Carentan Frontline、Omaha Assault、Foy Frontline 或 Assault 之一，并在备注记录），时长 60 min；前 5 min 为热身不计入 P99。
2. 断线/重连注入：每 10 min 随机 5% 客户端断线 30–90 s 后重连（GAMEDESIGN §2.8 的 90 s 窗口内重连应保留角色；超 90 s 应被移除）。
3. 网络条件轮换：0%/2%/5% 丢包与 RTT 40–150 ms 的 10 min 段落轮换（WT 与 WS 同样）。
4. 记录并检查：`tick_ms` P99、`tick_overrun` 最大连续次数、末 10 min RSS 增长、连接保持率（未被服务器/客户端异常断开的占比）、下行带宽 P50/P99/峰值、快照频率（WS ≥ 15 Hz）、bot 卡死指标。
5. 失败处理：任一指标越界即该次运行失败，保留 `flamegraph`/`tracing` 日志片段（50 MB 内）与复现命令；写 PLAN_ISSUES。
6. 汇总：6 次运行的表格与结论；与 M3-045/M2-049 的数据对比（退化 > 10% 须说明）。
### 接口与数据
- `soak.rs` CLI：`headless-client soak --map <id> --mode <mode> --clients 100 --duration 3600 --transport wt|ws --netem <profile> --metrics-url …`；输出 JSON 报告 schema 固定（`pass`/`fail`、各指标、阈值）。

## 不做什么
- 不做优化修复本身（发现问题写 PLAN_ISSUES，由对应任务修复；本任务只测）。
- 不做浏览器真实客户端压测（用 headless-client）。
- 不做公开测试规模（> 100 人/局或多局并发）的容量压测（M4-037 方案引用）。
- 不做语音压测（沿用 M2-049）。

## 验收标准
1. **tick_ms P99 ≤ 28 ms**（WT 运行；WS 运行阈值放宽至 ≤ 40 ms），三图各 60 min，指标取自 `/metrics` 直方图（热身后）。
2. **连续 overrun = 0**：`server_tick_overrun_total` 的最长连续超限序列 = 0（偶发单次允许记录，速率 ≤ 0.5/min 告警线；若有则须全部为非连续）。
3. **RSS 增长 < 2%**（末 10 min 对比 50–60 min 窗口起点），6 次运行均满足。
4. **连接保持率 ≥ 99%**（排除注入的主动断线客户端的重连失败：重连成功率 ≥ 99%）。
5. 带宽：下行 P50 ≤ 256 kbps、峰值 P99 ≤ 512 kbps 每玩家（§5.6/§6.9）；上行 ≤ 32 kbps；超限项逐项记录。
6. WS 兜底：快照频率 ≥ 15 Hz，`tick_ms` P99 ≤ 40 ms。
7. 汇总报告覆盖 6 次运行，全部 `pass`；任一 `fail` 须有 PLAN_ISSUES 编号且任务状态为 `partial`。

## 测试要求
- 单元测试：`analyze.py` 对合成时序（含注入连续 overrun、RSS 线性增长）正确判定。
- 集成/无头测试：先做 5 min 冒烟（3 图 × 2 传输）再跑 60 min 全量；netem 配置需要 root，在参考主机上执行（缺权限则标 `未测`）。
- 性能基准：见上；基线文件 `reports/loadtest/baseline-m4.json`。
- 回归：把 6 次运行的 P99 记入基线，后续 PR 的夜间 loadtest（M3-050）以此对比。

## 性能预算
- 服务器 CPU：稳态 ≤ 18 ms/tick、P99 ≤ 28 ms（§6.10）；带宽：均值 ≤ 256 kbps、峰值 ≤ 512 kbps 下行、上行 ≤ 32 kbps（§5.6）；WS 兜底 P99 ≤ 40 ms（§17.2）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`、`pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 需要参考主机（≥ 4 物理核，T3 基准硬件）与 root（netem）；执行者云容器通常不具备，缺失则 `未测`/`partial`，**不得用其它机器的数字顶替**（README §5.5）。
- 6 × 60 min = 6 h 挂钟时间，规划会话时用 `run_in_background` 与检查点，避免占用会话。
- Foy/Omaha 数据若仅为占位地形，性能结论仅供参考，须在真数据接入后重跑关键组（写 PLAN_ISSUES）。
