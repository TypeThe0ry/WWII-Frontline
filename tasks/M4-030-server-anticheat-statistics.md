# M4-030 server：统计异常检测与回放

## 元数据
- 里程碑：M4
- 工作流：server
- 规模：L
- 前置依赖：M1-027, M2-023
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §15.3; ARCHITECTURE §15.4; ARCHITECTURE §15.1; ARCHITECTURE §18.3; ARCHITECTURE §19.2; GAMEDESIGN §19.5; GAMEDESIGN §19.6

## 背景
首发反作弊 = 服务器权威 + AOI + 统计检测 + 举报；**统计检测只进人工复核队列，不自动封禁**（ARCHITECTURE §15.3；MILESTONES §5.8）。诚实边界：浏览器端无法做内核级保护，外部硬件自瞄无法检测，高水平玩家与作弊者无法 100% 区分（§15.4）。
ARCHITECTURE §15.3 要求：
- 每玩家滚动窗口（最近 10 分钟、本局）指标：爆头率、命中率、首发命中率、反应时间分布（敌人进入视锥到首发命中）、转向速度峰值、"锁定"迹象（准星与目标质心相对运动方差极小的持续时间）、同一枪线穿墙命中次数、移动模式异常。
- 评分：对每指标计算相对同分位基线的 z-score；组合分 `suspicion = Σ w_i * clamp(z_i, 0, 5)`；超过 `T_review` 进入人工复核队列。
- 日志：保存可疑玩家的 10 分钟输入回放（`InputCmd` 流 + 权威状态哈希），体积约 `30 Hz × 11 B × 600 s ≈ 200 KB/玩家`。
- 举报：`Report` → 入库 → 与评分合并优先级；管理员工具（gateway 管理 API）：查询玩家、查看回放、封禁、踢出、静音；所有操作写审计日志。
- 指标（§18.3）：`suspicion_score`（分布）、`reports_total`、复核队列长度。
规则只在 `sim`，server 不实现游戏规则（ARCHITECTURE §4.3.4）：统计模块只读取事件流（`SimEvent::Hit/Shot/Kill`）与 `InputCmd`，不改变模拟。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/anticheat/mod.rs`、`window.rs`（滚动窗口）、`metrics.rs`（指标计算）、`score.rs`（z-score 与组合分）、`replay.rs`（环形输入记录）。
- `data/anticheat/baselines.json` + `weights.json` —— 基线分位（初值由 bot/内测数据拟合，状态 `initial`）、权重 `w_i`、`T_review`。
- `crates/server/src/anticheat/review_queue.rs` + gateway 内部 API `POST /internal/review/enqueue`、`GET /admin/review?limit=`（沿用 M2-023 管理 API 鉴权/审计）。
- `crates/headless-client/src/scenarios/aimbot_inject.rs` —— 合成自瞄输入注入（线性吸附/瞬时转向/低方差跟踪）。
- `crates/server/tests/anticheat_stats.rs`；`crates/server/benches/anticheat.rs`。
### 要实现的行为
1. 每 tick 在 sim 线程**只追加**轻量记录（固定大小环形缓冲，不分配），指标计算在独立 tokio 任务每 5 s 批处理，避免占用 §6.10 阶段 1–8 预算。
2. 指标：爆头率、命中率、首发命中率、反应时间 P10/P50、转向角速度峰值（°/s）、锁定持续时间占比、穿墙命中数、移动异常分（沿用 M1-027 的 `suspicion_move`）。
3. 评分：`suspicion = Σ w_i * clamp(z_i, 0, 5)`；`z_i` 以 `baselines.json` 的分位基线（按武器类别与距离分桶）计算；超过 `T_review` 且样本量足够（最小开火次数 ≥ 60 发）才入队。
4. 回放：每玩家保留最近 10 min 输入环（`InputCmd` + 每秒权威状态哈希），入队时序列化为 `.replay`（≤ 200 KB/玩家，超限丢弃最早部分）。
5. 复核队列 API：按 `suspicion` 与举报数排序；管理员取回放；所有操作审计日志（保留 1 年，§18.4）；**首发不自动封禁**（代码层无 ban 调用路径，单测断言）。
6. 指标导出：`suspicion_score` 直方图、`review_queue_len`、`replay_bytes_total`。
### 接口与数据
- `pub struct SuspicionReport { player_id, score: f32, top_signals: [(Signal, f32); 3], samples: u32, window_s: u32 }`；`enum Signal { HeadshotRate, HitRate, FirstShotHit, ReactionTime, TurnPeak, LockOn, WallbangCount, MoveAnomaly }`。

## 不做什么
- 不做自动封禁、设备指纹、客户端完整性检测（浏览器无法可靠做到，§15.4）。
- 不做管理员前端界面（沿用 M2-023 API；UI 可选后续）。
- 不改 `sim` 规则或延迟补偿；不做语音相关检测。
- 不做 IP 信誉库。

## 验收标准
1. **注入合成自瞄输入被评分排入前 1%**：`headless-client aimbot_inject` 在 100 人对局（99 正常 bot + 1 注入，重复 20 局）中，被注入者 `suspicion` 排名 ≤ 第 1 名/100（即前 1%）的比例 ≥ 90%（`cargo test -p server aimbot_ranks_top1pct`）。
2. **正常 bot 误报率 < 1%**：200 局全 bot 对局（`balance-sim` 或 headless），`suspicion > T_review` 的 bot 占比 < 1%（报告 `reports/anticheat/m4-fp.json`，分母 ≥ 10 000 bot·局）。
3. **回放 ≤ 200 KB/玩家**：10 min 回放序列化后字节数 ≤ 204 800，100 玩家总内存 ≤ 20 MB（`cargo test -p server replay_size`）。
4. 性能：sim 线程侧追加记录成本 ≤ 0.05 ms/tick（100 玩家，`cargo bench -p server anticheat_append`）；批处理任务 CPU 占用 ≤ 1% 单核。
5. 不自动封禁：全代码库 grep 断言 `anticheat` 模块不调用任何 ban/kick API（单测）；复核队列接口鉴权/审计测试通过。
6. 指标可采集：`/metrics` 中出现 `suspicion_score_bucket`、`review_queue_len`。

## 测试要求
- 单元测试：z-score 计算、权重与截断、窗口滚动、样本量不足不入队、回放环溢出。
- 集成/无头测试：`aimbot_inject` 三类注入（吸附/瞬转/低方差锁定）；全 bot 误报测试；回放取回与反序列化往返。
- 性能基准：`cargo bench -p server anticheat_*`，基线文件 `reports/perf/anticheat.json`。
- 回归：基线与权重文件哈希入 `DATA_HASH`。

## 性能预算
- sim 线程阶段 1–8 预算不得回归（§6.10：合计 ≈ 16.4 ms，目标 ≤ 18 ms）；本任务追加 ≤ 0.05 ms/tick，批处理在 net/encode 线程池外的 tokio 任务。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `cargo bench`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：建议拆分线①指标+评分 ②回放+复核队列；若超会话按 README §5.6 建 `M4-030b`。
- 基线来自 bot 与小样本内测，**真人基线缺失**：误报率对真人高水平玩家未知，必须只进复核队列；真人数据在封闭测试（M4-045）后复拟合（写 PLAN_ISSUES）。
- 隐私：回放含输入，属个人数据；保留期与删除遵守 §19.4/H-074（M4-035 实现清理）。
- 合成注入≠真实外挂，"前 1%"只证明检测链路有效，不保证对真实自瞄的召回率；对外沟通须诚实说明（§15.4）。
