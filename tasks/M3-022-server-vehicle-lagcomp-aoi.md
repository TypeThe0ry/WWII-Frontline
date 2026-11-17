# M3-022 server：载具延迟补偿与 AOI 优先级

## 元数据
- 里程碑：M3
- 工作流：server
- 规模：M
- 前置依赖：M1-025, M3-013, M3-005
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.7.1; ARCHITECTURE §6.5.4; ARCHITECTURE §6.9.3; ARCHITECTURE §6.10; ARCHITECTURE §6.5.2; ARCHITECTURE §15.1

## 背景
M1-025 实现了玩家命中体的 16-tick 历史、开火回溯（≤ 8 tick）、出射点校验 ≤ 0.5 m、墙体拒绝与指标；M1-010 已定义载具实体编码（delta ≤ 24 B）。M3 的载具规模上来（≤ 40 辆、同屏 12 辆可见），需要：①**载具命中体回溯**（车体 OBB + 炮塔 OBB + 履带/装甲区，每条约 120 B，ARCHITECTURE §6.7.1），②**载具 AOI 优先级**（`imp ×1.5` 若 E 是载具，§6.5.4）与密集场景编码预算，③载具模块/座位新字段的增量编码（M3-005）。
关键规则：开火回溯 `rewind_ticks = clamp(server_tick_now − view_tick, 0, 8)`（8 tick ≈ 266 ms）；判定位置以射手权威位置 + 客户端方向为弹道起点（偏差 ≤ 0.5 m），目标使用 `server_tick_now − rewind_ticks` 时刻命中体（亚 tick 线性插值）；"载具/火炮等非即时武器：爆炸按服务器当前时刻计算（无回溯）"（§6.7.3）。载具内存：40 × 16 × 120 ≈ 77 KB，写入成本 ≈ 140 条/tick 约 0.05 ms。载具 delta 记录 24 B（§6.9.1）；密集场景 12 辆 × 10 Hz × 24 B = 23 kbps（§6.9.3），合计 ≈ 116.8 kbps。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/lagcomp/vehicle_history.rs` —— `VehicleHitFrame{tick, hull_obb, turret_obb, gun_pitch, track_boxes[2], armor_zone_map_version}`；环形缓冲 `H = 16`；`hitboxes_at(t, frac)` 线性插值（位置线性、旋转 slerp）。
- `crates/server/src/lagcomp/mod.rs` —— 扩展命中流程：弹丸对"当前世界静态 + 回溯时刻动态命中体（玩家 + 载具）"检测；命中载具后调用 `sim` 的 `resolve_shell_hit`（M3-013）；载具对载具的坦克炮：按 GD 的"坦克炮 ff_mult 0.5"处理；`lagcomp_clamped_vehicle` 指标。
- `crates/server/src/aoi/vehicle.rs` —— 载具优先级：载具 `imp ×1.5`（§6.5.4）；载具在 ≤ 400 m 内始终进候选（友军与敌军均按 R3 过滤：敌方载具仅在 ≤ 400 m 且视锥 + 30° 内或 ≤ 96 m 才发精确状态；被标记载具只发地图图标）；炮塔朝向 8 bit、炮管俯仰 6 bit 随载具 delta。
- `crates/server/src/aoi/budget.rs` —— 100 玩家 + 40 载具场景的包预算：`PACKET_BUDGET` 1150 B；若载具占用过大，优先降低远场玩家数量而非载具频率（载具最低 5 Hz，玩家最低 2 Hz）。
- `crates/server/tests/vehicle_lagcomp.rs`、`crates/headless-client/src/scenarios/tank_duel.rs`。
### 要实现的行为
1. 载具命中体历史写入（每 tick，140 条/tick 上限）；缺失历史（刚刷新）时回退到当前帧并计数。
2. 命中判定：射线先对静态世界（当前），再对回溯时刻的玩家与载具命中体取最近命中；载具命中产生 `HitConfirm{zone}`（仅射手）。
3. 墙体拒绝：沿用 M1-025 的 `lagcomp_wall_reject`：回溯命中与当前时刻 LoS 冲突则拒绝（阈值【待调参】）。
4. AOI：载具优先级公式 `rate_hz = base(dist)×vis×imp`，`imp ×1.5`；载具在近场 10 Hz、远场 5 Hz；首包全量关键帧；超 440 m 离开迟滞。
5. 编码预算：密集场景（12 可见载具）载具 delta 总字节 ≤ 2 880 B/s（§6.9.3）；若超限，调节器按 §6.6.3 缩放。
6. 度量：`vehicle_hit_rate`、`lagcomp_clamped_vehicle_total`、`aoi_vehicle_starved_total`、`vehicle_delta_bytes`（Prometheus）。
### 接口与数据
- 复用 M1-025 的 `LagCompHistory`，新增 `VehicleHistory`；`sim` 暴露 `WorldView::vehicle_hit_frame(id)`（`crates/sim`，仅只读）。

## 不做什么
- 载具物理/装甲规则 → M3-013–015（sim 内）；渲染 → M3-029；
- 压测与全功能带宽验证 → M3-045；火炮落点回溯（无回溯，见 §6.7.3）。

## 验收标准
1. **150 ms RTT 下载具命中率与 0 延迟基线差 ≤ 5%**：`headless-client` 坦克对射场景（`tank_duel`）各 1 000 发（0 ms 与 150 ms RTT，`tc netem`），命中率差 ≤ 5 个百分点（报告 `reports/m3-vehicle-lagcomp.json`）。
2. 载具回溯精度：构造移动载具（20 km/h），回溯 8 tick 的命中点位置误差 ≤ 0.1 m（单测）。
3. 载具 delta：24 B 上限（全字段变化）；密集场景（12 载具）载具分量 ≤ 23 kbps，与 ARCHITECTURE §6.9.3 一致（`bytes_by_component` 指标断言）。
4. 100 玩家 + 40 载具场景：`bytes_out_per_client` P99 ≤ 400 kbps（报告，M3-045 复核）；载具起饿计数 `aoi_vehicle_starved_total` = 0（载具 ≥ 5 Hz）。
5. 载具历史写入 ≤ 0.1 ms/tick（40 辆，`criterion`）；AOI+编码阶段不使 §6.10 阶段 8–9 总和超过 2.6 ms。
6. R3 审计：敌方载具超出视锥且 > 96 m 时不进入该客户端快照（单测）；被标记载具只经图标事件。

## 测试要求
- 单元测试：历史环写读、插值、越界保护、优先级排序、预算降级。
- 集成/无头：`tank_duel`（0/150 ms RTT；2% 丢包）；100 客户端 + 40 载具 5 分钟冒烟（完整 30 min 在 M3-045）。
- 性能基准：`cargo bench -p server vehicle_history`、`aoi_encode`。
- 回归：协议黄金样例与 AOI 指标基线更新。

## 性能预算
ARCHITECTURE §6.10：阶段 3 载具 2.0 ms（sim 内）；阶段 7 历史缓冲写入 0.1 ms；阶段 8 AOI 2.0 ms；阶段 9 编码 0.6 ms（墙钟）；§6.9.3 密集下行 ≈ 116.8 kbps（验收 P99 ≤ 400 kbps）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规则只能在 `sim`（ARCHITECTURE §4.3.4）：`server` 只负责历史缓冲与调度，判定函数调用 `sim`。
- 150 ms RTT 在 8 tick 回溯上限内；RTT > 266 ms 的玩家受 `lagcomp_clamped` 限制，不属本验收。
- 压测与最终带宽结论在 M3-045；本任务仅冒烟，不得声称满足 E3-6。
