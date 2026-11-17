# M0-019 server：AOI v0 与快照发送

## 元数据
- 里程碑：M0
- 工作流：server
- 规模：L
- 前置依赖：M0-018, M0-006
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.5.4; ARCHITECTURE §6.9; ARCHITECTURE §6.10; ARCHITECTURE §6.5.2; ARCHITECTURE §6.6.2

## 背景
把 sim 的世界状态变成每个客户端的快照包：**64 m 网格 AOI + 优先级累加器**（ARCHITECTURE §6.5.4）。M0 的 v0 范围：只做距离/视锥/重要性优先级与预算内选择，**不做敌方信息过滤（R3）与团队通道**——那是 M1-024（AOI v1）。M0 地图仅 256 m（4×4 个 64 m 格），所有实体都在 400 m 内。
算法（§6.5.4）：`rate_hz(E,C) = base(dist) × vis × imp`，`base`：≤32 m → 20 Hz、≤96 → 10、≤192 → 5、≤400 → 2；`vis`：视锥内（±70°）1.0、视锥外 0.5、在 C 后方且 > 96 m 0.35；`imp` 基数 1.0，×2 正在开火/爆炸、×2 近 1 s 内受伤/击杀相关、×1.5 在 C 准星 5° 锥内（小队成员、载具项 M0 不适用）；`rate_hz` 上限 20、下限 2；每 tick `acc[C][E] += rate_hz × dt`；出包选择：按 `acc` 降序取 `acc ≥ 1.0`，另外 `acc ≥ 10` 强制先发（防饥饿）；`PACKET_BUDGET = 1150 B`；放不下则 `continue` 并记 `aoi_starved++`。出包节奏：快照基准 20 Hz，每 3 tick 出 2 包，按 `client_id % 3` 错相（§6.1）。基线：`SnapshotEncoder`（M0-006）用客户端 `ack_snapshot_tick` 提升基线；落后 > 32 tick 发关键帧。事件（`Shot/Impact/HitConfirm/DamageDir`）冗余重发直到 ack 或 3 包后过期，客户端按 `event_id` 去重。`self_state`（§6.5.3）每包必含；`pos_corr` 仅当客户端 `pred_hash` 与权威不匹配时才补发完整位置。
WS 路径降级（§6.6.2）：快照上限 15 Hz，`buffered_amount` 高时丢旧包。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/snapshot/{mod.rs,aoi_grid.rs,priority.rs,scheduler.rs,baseline_book.rs,events_out.rs,self_state.rs}`
- `crates/server/tests/{aoi_priority.rs,snapshot_bandwidth.rs}`、`crates/server/benches/aoi_encode.rs`
### 要实现的行为
1. `AoiGrid`：`cell = floor(pos.xz/64)`；`update(entity,new_pos)` 仅在越格时移动；`query(cell, radius_cells=2)` 取 5×5 邻域（"近场"）加至 400 m 的"远场"；`3×3` 邻格 + 速度外推边距的细节写入注释（§5.7 风险）。
2. 优先级累加器：`PriorityState{acc: BTreeMap<(ClientId,EntityId), f32>}`（**不得**用 `HashMap` 之外的非确定迭代影响出包顺序——server 不受 sim 禁令约束，但选择顺序需稳定以便测试：同分按 `EntityId` 升序）；实体离开候选（> 440 m 迟滞）后连续 3 个出包周期不再发则发 `entity_remove`；新进入实体首包必须全量关键帧。
3. 出包调度：每 tick 对 `(client_id % 3) != (tick % 3)` 的客户端出包（即每客户端 20 Hz）；WS 客户端再降为 15 Hz（每 4 tick 出 3 包的等价实现，或每 2 tick 间隔交替，写明所选方法）；`header.input_slack` 填"输入到达时距执行 tick 的余量"（来自 M0-018 的统计，4 bit）。
4. `self_state` 填充：`ammo_in_mag/ammo_reserve/health/stamina(0)/suppression(0)/stance/deploy_state(0)/cooldown_bits`（来自 `WeaponState`）；`pred_hash` 比较：客户端上报的 `pred_hash`（位置量化后 CRC8）与权威同函数结果，不同才附 `pos_corr` 完整位置。
5. 事件冗余：`Shot`（AOI 内玩家可见）、`Impact`（AOI 内）、`HitConfirm`（仅射手）、`DamageDir`（仅被害者）来自 `StepOutput.events`；每个事件分配递增 `event_id: u16`；对每个客户端维护"未确认事件窗口"，最多在 3 个出包中重发；客户端 ack 通过 `ack_snapshot_tick` 间接确认。
6. 度量：`snapshot_bytes`（直方图）、`net_bytes_out_per_client`（滑动 1 s 窗口 kbps，P50/P99 直方图；同时记录"负载"与"负载 + 58 B/包的线路估计"两个口径，§6.9.1）、`aoi_starved_total`、`snapshot_budget_utilization`、`aoi_encode_ms`（阶段 8+9 耗时）。
7. 编码在 sim 线程内串行执行（M0 规模足够）；若实测超预算再引入 `rayon` encode pool（仅服务器；`sim` 仍单线程，§6.10），并在 PLAN_ISSUES 记录。
### 接口与数据
- `SnapshotScheduler::tick(&mut self, view:&dyn WorldView, events:&[SimEvent], sessions:&mut Sessions, now:Tick)->Vec<(ClientId,Bytes)>`；输出经 `ClientTransport::send(C2, …)`（M0-016/017）。

## 不做什么
- 不做敌方可见性过滤、团队通道 1 Hz、友军 400 m 规则（M1-024）。
- 不做延迟补偿（M1-025）；不做带宽调节器（M2-020）；不做载具记录（M1-010）。
- 不做 C4 可靠事件（M1-010/M1-026）。

## 验收标准
1. 带宽：用进程内合成客户端（10 个，30 Hz 随机走动+开火输入，**不经网络**以隔离编码；另跑一次经 WS loopback）运行 5 min：`net_bytes_out_per_client` **P50 ≤ 128 kbps**（线路估计口径）；同时记录 P99 与按 ARCHITECTURE §6.9.2 典型值 ≈ 54 kbps 的对比。
2. CPU：10 客户端下 AOI+编码阶段 ≤ 2.6 ms/tick（ARCHITECTURE §6.10 阶段 8 的 2.0 + 阶段 9 墙钟 0.6）；另用 `benches/aoi_encode.rs` 合成 100 客户端×60 候选，记录每 tick 耗时并外推。
3. 优先级单测：近处（≤ 32 m）实体平均每秒被发 ≈ 20 次、200 m 外 ≈ 2–5 次（10 s 模拟，±10%）；视锥外降 0.5；饥饿保护：任一候选实体在 ≥ 1 s 内至少被发 2 次（2 Hz 下限，构造极端拥挤场景验证）。
4. 基线与关键帧：丢弃 30% 下行包的网络模拟下，客户端（用 M0-006 的 `SnapshotDecoder` 做进程内客户端）能持续解码，`MissingBaseline` 之后 1 s 内恢复；ack 滞后 > 32 tick 后收到关键帧。
5. 事件冗余：一个 `Shot` 事件在 3 个出包内重发，进程内客户端去重后只应用一次；丢包 30% 下事件最终到达率 ≥ 97%。
6. 错相：20 个客户端在任意连续 3 tick 内各 tick 出包客户端数最大差 ≤ 1（均匀度测试）。
7. `pos_corr`：人为篡改客户端上报的 `pred_hash`，下一包含完整位置；一致时只含 4 bit "ok"。

## 测试要求
- 单元测试：`AoiGrid` 越格更新、5×5 邻域、迟滞 400/440 m、`acc` 取整、预算裁剪（放不下 `continue` 不写半条）。
- 集成/无头测试：验收 1/4/5 的进程内合成；M0-027 的真实 headless-client 联测在更上层复核。
- 性能基准：`aoi_encode`（criterion），基线 JSON 入 `reports/baselines/server-m0.json`。
- 回归：典型场景下的 `snapshot_bytes` 分布快照（容差 ±10%）。

## 性能预算
下行：每玩家均值 ≤ 256 kbps / 峰值 ≤ 512 kbps（硬预算）；M0 压测线 P50 ≤ 128 kbps、P99 ≤ 400 kbps（ARCHITECTURE §6.9.5）。CPU：AOI ≈ 2.0 ms + 编码墙钟 ≈ 0.6 ms/tick（§6.10 阶段 8/9）。上行含传输开销 ≈ 22 kbps ≤ 32 kbps 在 M0-027 复核。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 10 名 bot 与 10 个真人客户端的完整场景（含 bot 在 sim 内）依赖 M0-026；本任务验收用合成客户端，真实联测在 M0-027。
- R3（敌方只下发 ≤400 m 且视锥+30° 或 ≤96 m）**不在本任务实现**；M0 向客户端发送所有近场实体，包括视锥外敌人——M0 为内部技术验证，已在 PLAN_ISSUES 记录以免被误当成最终行为，M1-024 修正。
- `PACKET_BUDGET` 取 `min(1150, wt.max_datagram_size − 1)`；WS 无此限制但仍用 1150 保持一致。
