# M3-045 测试：M3 全功能 100 客户端压测

## 元数据
- 里程碑：M3
- 工作流：test
- 规模：M
- 前置依赖：M2-049, M3-022, M3-026, M3-028, M3-021
- 素材需求：无；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.2; ARCHITECTURE §6.9.3; ARCHITECTURE §6.9.5; ARCHITECTURE §6.10; ARCHITECTURE §18.3; MILESTONES §4.3 (E3-6)

## 背景
M2-049 做了 100 客户端压测（含 WT 与 WS）。M3 加入 40 辆载具、全部指挥官能力、Assault 模式、动态导航障碍与载具 AOI 优先级（M3-022）。本任务在全功能下复测容量，对应退出标准 E3-6。
判据（ARCH §17.2/§6.9）：
- 服务器 `tick_ms` P99 ≤ 28 ms（稳态目标 ≤ 18 ms，ARCH §6.10 合计约 16.4 ms）；`tick_overrun` 无连续超限；末 10 分钟 RSS 增长 < 2%。
- 下行 `bytes_out_per_client`：P50 ≤ 128 kbps；密集场景 P99 ≤ 400 kbps（硬预算 峰值 ≤ 512 kbps，推导值密集 ≈ 117 kbps、含载具 12 辆 × 10 Hz × 24 B）。
- 上行（不含语音）≤ 32 kbps（推导 ≈ 22 kbps）。
- WS 兜底路径指标放宽：tick P99 ≤ 40 ms、快照 15 Hz（ARCH §17.2）。
- 网络条件注入：RTT 40–150 ms、抖动 20 ms、丢包 0/2/5%（`tc netem`）。
- 载具（ARCH §6.10 阶段 3）≤ 40 辆，预算 2.0 ms；阶段 4 弹体 ≤ 400 段，预算 2.5 ms；阶段 5 规则（含指挥官能力）0.7 ms。

## 具体内容
### 要创建/修改的文件
- `crates/headless-client/src/scenarios/m3_full.rs` —— 脚本化场景：100 个合成客户端（含 2 个 Commander 席位、若干 Armor Crew、炮组、Spotter），按计划驱动：每 90 s 轮转触发 8 种能力；40 辆载具由 Commander 呼叫并由合成驾驶沿路点行驶；2 门榴弹炮周期开火；Spotter 周期标记。
- `crates/headless-client/src/scenarios/assault_m3.rs` —— Assault 测试配置场景（M3-004 配置）。
- `crates/headless-client/src/netem.rs` —— 按配置文件调用 `tc netem`（容器内需 `NET_ADMIN`）。
- `tools/loadtest/run_m3_full.sh` —— 启动 server + 100 客户端，采集 `/metrics`，产出报告。
- `reports/m3-loadtest/`（目录，进 git 只存摘要 JSON；原始日志不入库）：`wt_frontline.json`、`wt_assault.json`、`ws_assault.json`。
- `.github/workflows/nightly.yml`（读取，不在此改；由 M3-050 接入）。
### 要实现的行为
1. 运行矩阵（各 30 min）：A) WT + Frontline 全功能；B) WT + Assault 测试配置；C) WS + Assault（降级指标）。
2. 合成载具驾驶使用 server 提供的 `vehicle_routes.json`（M3-004）；不要求绕过障碍的智能，卡住允许，但需统计。
3. 能力触发：8 种能力在 30 min 内各 ≥ 10 次；非法指令注入每分钟 5 条，必须全部被服务器拒绝且不影响 tick。
4. 采集：`tick_ms` 直方图、`tick_overrun_total`、各阶段耗时、`bytes_out_per_client`（P50/P99）、RSS、`conn_alive_ratio`、重连次数。
5. 密集场景：每 5 min 一次 60 s 聚集（全员 + 10 载具 ≤ 150 m 半径）测下行 P99。
6. 判定脚本输出 PASS/FAIL 表，写 `reports/m3-loadtest/summary.md`（不入 docs）。
### 接口与数据
- 报告 JSON 字段：`{ run, transport, duration_s, tick_ms:{p50,p99,max}, overrun_consecutive_max, rss_growth_last10m_pct, down_kbps:{p50,p99}, up_kbps:{p50,p99}, vehicles_max, abilities:{name:count} }`。

## 不做什么
- 60 分钟浸泡与三图压测（M4-038）；真浏览器客户端压测。
- 优化 server 性能本身（超预算则写 PLAN_ISSUES，并按 §5.5 标 partial，不在本任务重构）。
- LLM 指挥官的压测（默认关闭；若开启另测，不计入本任务）。

## 验收标准
1. 运行 A、B：`tick_ms` P99 ≤ 28 ms；`tick_overrun` 连续次数 = 0；末 10 分钟 RSS 增长 < 2%（`/metrics`，报告 JSON）。
2. 运行 C（WS）：tick P99 ≤ 40 ms、快照 15 Hz 下连接保持率 ≥ 99%。
3. 下行：运行 A、B 的 `bytes_out_per_client` P50 ≤ 128 kbps；密集 60 s 窗口 P99 ≤ 400 kbps（硬预算 512 kbps 不得超）。
4. 上行 P99 ≤ 32 kbps。
5. 40 辆载具同时存在的时段 ≥ 5 min（`vehicles_max ≥ 40` 的时间累计）。
6. 8 种能力各 ≥ 10 次被服务器接受；注入的非法指令 100% 被拒（拒绝计数 = 注入计数）。
7. 5% 丢包条件下重连成功率 = 100%，无 panic（`grep -c panic server.log = 0`）。

## 测试要求
- 单元：判定脚本对样例报告的 PASS/FAIL 逻辑。
- 集成/无头：上述三运行矩阵；netem 配置 RTT 40/100/150 ms、抖动 20 ms、丢包 0/2/5% 各至少一个 5 min 子段。
- 性能基准：记录参考机型号与命令；无参考机则报告标注"非参考机，partial"。
- 回归：对比 M2-049 报告，tick P99 退化 > 15% 则写 PLAN_ISSUES。

## 性能预算
- ARCH §6.9.5（下行/上行预算）、§6.10（tick 预算 33.3 ms，稳态 ≤ 18 ms）、§17.2。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 租用服务器/成本需人批准（H-067）；无高频参考主机时数字仅作参考，状态 partial。
- 运行总时长 ≈ 3 × 30 min + 准备；若 CI 不可行，在专用主机上手动运行并归档 JSON。
- 容量数据需回写 ARCH §18.1（由 Fable 在检查点 3 处理，非本任务）。
