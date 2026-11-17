# M2-020 server：带宽调节器

## 元数据
- 里程碑：M2
- 工作流：server
- 规模：M
- 前置依赖：M1-024
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.6.3; ARCHITECTURE §6.9.4; ARCHITECTURE §6.5.4; ARCHITECTURE §6.6.2; ARCHITECTURE §18.3

## 背景
M1-024 实现了 AOI v1：网格候选、R3 敌我过滤、优先级累加器 `acc[C][E] += rate_hz·dt`、出包选择（`budget_bytes = PACKET_BUDGET(C)`，默认 1 150 B 载荷；`acc ≥ 10` 的强制先发以保 2 Hz 下限；仍有 `acc ≥ 2` 未发则 `aoi_starved++`）。本任务实现**带宽调节器**（ARCH §6.6.3）：服务器为每客户端维护 EWMA 发送字节、客户端 ack 率、估算 RTT；若 **ack 率 < 90%** 或估算可用带宽（QUIC 拥塞窗口 / WS 的 `buffered_amount`）不足，则：① `PACKET_BUDGET` 线性缩放（**最低 40%**）；② 远场 T3 频率不得低于 2 Hz → 优先裁剪 T2/T3 的**数量**而不是降频；③ 通知 HUD。
预算背景（ARCH §6.9）：典型 54.4 kbps、5% 丢包 ≈ 68 kbps（基线 ack 滞后使差分位宽 +25%）、密集 ≈ 117 kbps；硬预算均值 ≤ 256 kbps、峰值 ≤ 512 kbps；M2 验收线 P50 ≤ 128 kbps、P99 ≤ 400 kbps。WS 兜底（§6.6.2）：快照上限 15 Hz；`buffered_amount > 8 KB` 丢旧快照仅留最新。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/net/regulator.rs` —— `BandwidthRegulator`：`on_send(bytes)`、`on_ack(tick)`、`on_rtt(ms)`、`scale() -> f32`（0.4..=1.0）、`budget(base) -> usize`、`shed_far(count) -> usize`
- `crates/server/src/net/aoi.rs` —— 接入：出包预算用 `regulator.budget()`；T2/T3 候选数量裁剪规则（先 T3 后 T2，**保持被保留实体 2 Hz**）
- `crates/server/src/net/transport_stats.rs` —— 从 `wtransport` 取拥塞窗口/RTT；WS 路径用 `buffered_amount`
- `crates/server/src/metrics.rs` —— 新增 `net_budget_scale`（直方图）、`net_regulated_clients`（gauge）、`net_far_shed_total`
- 配置：`data/server_net.json`（EWMA 系数、阈值 90%、最低 40%、恢复斜率）——不属游戏性数值，放服务器配置而非 `/data` 规则
### 要实现的行为
1. 每 tick 更新 EWMA（发送速率，α 取配置；初值取 `PACKET_BUDGET`），每 10 个出包周期（≈ 0.5 s）重算 `scale`。
2. 触发降级：`ack_rate < 0.90` 或 `est_bw < 当前发送速率 ×1.2` → `scale -= step_down`（线性，≥ 0.4）；恢复：连续 2 s ack ≥ 95% 才 `scale += step_up`（防抖，迟滞）。
3. 预算应用：`budget_bytes = PACKET_BUDGET × scale`；固定成本（header + self_state + events + team/world）不得被裁剪——不够时先砍实体。
4. 远场裁剪：在预算内按 acc 排序时，T3 实体按"数量上限"裁到 `n_max(scale)`，**不得把 T3 的 `rate_hz` 降到 2 Hz 以下**；被裁的实体视为"离开候选"，走进入/离开迟滞（400 m / 440 m）而不是一直饿着。
5. 通知 HUD：scale < 0.7 时经 C4 `Notice{code:NET_DEGRADED,args:[scale%,0,0]}`（M1-010 已有 `Notice`），恢复时发 `NET_RECOVERED`；每客户端至多 1 条/10 s。
6. WS 路径：快照目标率 15 Hz；`buffered_amount > 8 KB` → 丢弃旧快照（计数 `ws_snapshot_dropped_total`）。
### 接口与数据
- `pub struct RegulatorCfg{ ack_low:f32=0.90, min_scale:f32=0.40, step_down:f32, step_up:f32, ewma_alpha:f32 }`；`scale` 与 `aoi_starved_total`、`snapshot_budget_utilization`（ARCH §18.3）一并导出。

## 不做什么
- AOI 过滤规则/优先级公式本身（M1-024，不得改 R3）；100 人 CPU 优化 → M2-021；压测编排 → M2-049。
- 客户端 HUD 呈现 → UI 任务（M2-043 的系统提示）；协议新增字段（沿用现有 `Notice`）。
- 语音带宽（LiveKit 独立路径）。

## 验收标准
1. 5% 丢包（`tc netem loss 5% delay 40ms 20ms`）、典型场景（ARCH §6.9.2 的 50 可见玩家）下行均值 ≤ 68 kbps（`net_bytes_out_per_client` 均值，30 min headless，连接 20 个）。
2. 弱网（loss 5% + RTT 150 ms + 带宽限 80 kbps）下 `scale` 稳定在 [0.40, 1.0]，不低于 0.40（指标最小值断言）；恢复到无损后 ≤ 10 s 回到 ≥ 0.95。
3. 2 Hz 下限不被饿死：在 `scale = 0.40` 下，被保留的 T3 实体（192–400 m）相邻两次更新间隔 P99 ≤ 600 ms（headless 记录每实体更新时间戳）；`aoi_starved_total` 速率 ≤ 0.5/min。
4. 无损网络下 `scale` 始终 = 1.0，且与 M1-024 基线逐字节输出一致（回归测试，避免误降级）。
5. 密集场景（ARCH §6.9.3，150 事件/s）P99 下行 ≤ 400 kbps（验收线，30 min）。
6. HUD 通知限频：同客户端 10 s 内 ≤ 1 条（测试）。
7. 调节器 CPU：100 客户端 ≤ 0.1 ms/tick（`cargo bench -p server regulator`）。

## 测试要求
- 单元测试：`scale` 状态机（降/升/迟滞/下限）、EWMA、裁剪数量函数；proptest：任意 ack 序列 `scale ∈ [0.4,1]`。
- 集成/无头：`headless-client` + `tc netem` 矩阵（loss 0/2/5%，RTT 40/100/150 ms，WS 与 WT 各一）；断言上面的验收数字。
- 性能基准：`cargo bench -p server regulator`。
- 回归：M1-024 的 20 客户端场景 P99 ≤ 400 kbps 不回归。

## 性能预算
下行：均值 ≤ 256 kbps、峰值 ≤ 512 kbps（硬预算，ARCH §5.6）；M2 验收线 P50 ≤ 128 kbps / P99 ≤ 400 kbps（ARCH §6.9.5）；CPU 并入 ARCH §6.10 阶段 8–9（AOI 2.0 ms / 编码 0.6 ms 墙钟），本任务 ≤ 0.1 ms。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `tc netem` 需要 root/容器能力；无权限时用用户态丢包注入（headless-client 的 `--loss`）并把 netem 项记 `未测`/`partial`。
- WebTransport 拥塞窗口读取依赖 `wtransport` 暴露的统计，若缺失则仅用 ack 率 + RTT 驱动（登记 PLAN_ISSUES）。
- 只改"预算/数量"，不得改变 R3 的敌方不下发规则。
