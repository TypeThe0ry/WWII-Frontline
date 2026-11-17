# M1-024 server：AOI v1（敌友规则、团队通道）

## 元数据
- 里程碑：M1
- 工作流：server
- 规模：L
- 依赖：M0-019, M1-010
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.5.4; ARCHITECTURE §15.2; ARCHITECTURE §6.9; ARCHITECTURE §6.10; ARCHITECTURE §18.3; GAMEDESIGN §17.2; GAMEDESIGN §17.4; BRIEF R3

## 背景
M0-019 的 AOI v0：64 m 网格、距离优先级累加器、20 Hz 错相出包（`client_id % 3`）、self_state、基线 ack、事件冗余。v1 加入**反透视的信息控制**与**团队通道**，规则以 **R3** 为准（ARCHITECTURE §6.5.4/§15.2 与 GAMEDESIGN §17.2 已统一）：
- **敌方**实体仅在 **≤ 400 m 且（位于玩家视锥 + 30° 余量内 或 ≤ 96 m）** 时才进入候选并下发精确状态；视锥外且 > 96 m 的敌人**直接剔除**（§6.5.4 的 `vis` 降频系数只用于友军）。被友军标记/侦察暴露的敌人**只下发地图图标**（团队通道，1 Hz，位置量化到 5 m，带有效期）。
- **友军**：最远 400 m、最低 2 Hz；更远的友军只走团队通道图标。
- **团队通道（C3，1 Hz）**：向同阵营每人发送队友 `(x:11b, z:11b, role:4b, state:3b, facing:3b)` = 32 bit；己方小队成员（≤ 6）用高精度 `(x:16b, z:16b, y:8b, …)`，每 0.5 s 走普通快照路径（GD §17.4：同小队 10 Hz 高优先级、其他小队长 1 Hz 图标）。
- **优先级**：`rate_hz = base(d) × vis × imp`，`base`：≤32 m 20 Hz、≤96 m 10、≤192 m 5、≤400 m 2；`vis`：视锥内(±70°) 1.0、视锥外 0.5、后方且 >96 m 0.35（仅友军）；`imp` 基数 1.0，开火/爆炸中 ×2、小队成员 ×2、**载具 ×1.5**、1 s 内受伤/击杀相关 ×2、准星 5° 锥内 ×1.5；上限 20、下限 2 Hz。
- **迟滞**：进入半径 400 m、离开 440 m；离开后连续 3 个出包周期不再发；新进入实体首包必须全量关键帧。
- **LoS 剔除**：对"完全被厚墙隔绝且 > 20 m"的敌人，每客户端每 0.25 s 检查一次并缓存【待调参】。
- 包预算 `PACKET_BUDGET` 默认载荷 1150 B；`aoi_starved` 指标；载具 delta 记录 24 B（M1-010）。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/aoi/candidates.rs` —— 候选集：5×5 近场 + 400 m 远场；敌/友分流
- `crates/server/src/aoi/enemy_filter.rs` —— 视锥+30° / 96 m 规则、迟滞、LoS 缓存
- `crates/server/src/aoi/priority.rs` —— 优先级与累加器（友军 `vis`、`imp` 乘子含载具）
- `crates/server/src/aoi/team_channel.rs` —— 1 Hz 团队通道构建；标记敌人图标（5 m 量化、有效期）
- `crates/server/src/aoi/audit.rs` —— 审计：每次出包记录"视锥外且 >96 m 的敌人下发数"，统计计数 `aoi_enemy_leak_total`
- `crates/server/src/aoi/mod.rs` —— 接口并入 M0 的快照发送循环
- `data/rules/aoi.json` —— `enemy_max_m 400`、`enemy_close_m 96`、`frustum_half_deg 70`、`frustum_margin_deg 30`、`hysteresis_leave_m 440`、`los_check_s 0.25` 等常量（单位后缀）
- `crates/server/tests/aoi_*.rs`；`crates/headless-client/src/scenarios/aoi_audit.rs`
### 要实现的行为
1. 每个出包 tick 对每个客户端构建候选：同阵营（友军）与敌对阵营分别处理；敌方过滤见上；视锥以客户端最近一条已执行 `InputCmd` 的 yaw/pitch 为朝向，使用 3D 夹角判定（`acos(dot) ≤ 70°+30°`）。
2. 敌方快速转身的缓冲：视锥判定使用"当前朝向与 100 ms 前朝向的并集"（缓解突现，ARCHITECTURE §15.2 潜在代价），并计入 `frustum_union_used`（细化，见备注）。
3. 被标记/暴露的敌人：由 `MarkerPlace`/Spotter 逻辑（M2）产生的标记事件接入；M1 先提供接口与单测夹具；图标 1 Hz、位置 5 m 量化，不含精确状态字段。
4. 团队通道：每秒向每个客户端发送一包 `TeamState`（≤ 49 人 ≈ 200 B）；小队成员走快照高精度路径。
5. 载具：优先级 ×1.5，含炮塔朝向；载具乘员不单独下发乘员位置（乘员位随载具），敌方载具同样遵守敌方过滤。
6. 审计：服务器在 `aoi_audit` 模式下对**每个**下发的敌人复核规则；任何违规计 `aoi_enemy_leak_total`（应恒为 0）并 warn 日志；非审计模式下以 1% 采样复核。
7. 带宽调节：沿用 M0（`PACKET_BUDGET` 线性缩放最低 40%，T3 数量优先裁剪）。
8. 指标：`aoi_starved_total`、`snapshot_budget_utilization`、`net_bytes_out_per_client`（ARCHITECTURE §18.3）。
### 接口与数据
```rust
pub struct AoiParams { pub enemy_max_m: f32, pub enemy_close_m: f32, pub frustum_half_deg: f32, pub margin_deg: f32 }
pub fn select_for_client(c:&ClientCtx, w:&dyn WorldView, p:&AoiParams, acc:&mut Accumulators) -> Selected{ friends, enemies, vehicles, team_tick:bool }
```

## 不做什么
- 100 客户端规模与带宽调节器细化：M2；
- 友军高精度路径之外的 Commander 全队 1 Hz 视图：M3；
- 标记（Spotter/Recon Flight）的产生逻辑：M3（本任务只留接口）；
- 协议字段定义：M1-010；
- 延迟补偿：M1-025；客户端消费与实体生命周期：M1-040。

## 验收标准
1. **20 客户端场景**（`aoi_audit`，10v10，随机移动 + 持续转身 10 min）：视锥+30° 之外且 > 96 m 的敌人**下发数 = 0**（审计日志 `aoi_enemy_leak_total == 0`，贴 STATUS）。
2. 友军 400 m 内各客户端收到的友军更新频率下限 ≥ 2 Hz（统计 P5 ≥ 1.9 Hz）；> 400 m 友军零精确记录，仅出现在 1 Hz 团队通道。
3. 迟滞：实体在 399 m 进入、441 m 才离开；在 400±5 m 往复 1 分钟内进入/离开切换 ≤ 2 次（单测 + 场景）。
4. 团队通道：49 人一次编码 ≤ 200 B，发送间隔 1.0 s ±1 tick，扩散含 role/state 位正确（解码往返单测）。
5. 带宽：20 客户端（含 netem RTT 40–150 ms、2% 丢包）30 min，每客户端下行 P50 ≤ 128 kbps、**P99 ≤ 400 kbps**（`/metrics` `net_bytes_out_per_client`；ARCHITECTURE §6.9.5）。
6. 性能：AOI + 编码阶段 20 客户端稳态 ≤ 1.0 ms/tick，且任何情形不超过 ARCHITECTURE §6.10 阶段 8+9 的 2.6 ms/tick（`server_tick_ms` 分段计时）。
7. 载具：载具优先级 ×1.5 使 100 m 内载具更新频率 ≥ 同距离玩家的 1.4 倍（单测）；载具记录 ≤ 24 B（统计）。
8. 回归：M0 的 10 客户端 P50 ≤ 128 kbps 不回归（`test-headless-v0`）。

## 测试要求
- 单元测试：视锥夹角边界（99.9°/100.1°）、96 m 近距边界（95.9/96.1 m）、迟滞、LoS 缓存失效、5 m 量化、团队通道往返。
- 集成/无头：`aoi_audit` 20 headless 客户端 30 min；netem 条件脚本入库 `scripts/netem/m1.sh`。
- 性能基准：criterion `aoi_select`（60 候选/客户端）。
- 回归：带宽/泄漏基线写入 `reports/m1-aoi.json`。

## 性能预算
- 引用 ARCHITECTURE §6.9.5（下行 P50 ≤ 128、P99 ≤ 400 kbps；硬预算均值 ≤ 256、峰值 ≤ 512）与 §6.10 阶段 8（AOI 2.0 ms）+ 阶段 9（编码 0.6 ms 墙钟）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- "视锥"的精确定义（半角 70°+30° 是否过宽；与客户端实际 FOV 的关系）文档未写死；本任务取 ARCHITECTURE §6.5.4 的 ±70° 加 30° 余量，写入 `/data/rules/aoi.json` 并登记 PLAN_ISSUES；审计与过滤必须使用同一常量。
- 转身缓冲（朝向并集）是对 ARCHITECTURE §15.2"预测相机转速预留"的细化，登记 PLAN_ISSUES。
- INDEX 验收"视锥外 > 96 m 敌人下发数 = 0"即 E1-4（MILESTONES §2.3）。
- 规模 L；若超出，将团队通道与 LoS 缓存拆为 `M1-024b`。
