# M1-025 server：延迟补偿

## 元数据
- 里程碑：M1
- 工作流：server
- 规模：L
- 依赖：M0-018, M1-014, M1-010
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.7; ARCHITECTURE §15.1; ARCHITECTURE §6.8.2; ARCHITECTURE §6.10; ARCHITECTURE §18.3; GAMEDESIGN §6.4

## 背景
服务器权威 + 客户端预测下，射击要对**射手视角时间**的目标位置判定（ARCHITECTURE §6.7）。M1-014 已提供 `LagCompHistory` 数据结构（16 tick 环、4 胶囊/玩家、车体 OBB+炮塔 OBB）；本任务把它接入服务器主循环并实现开火回溯。规则（T2：上限 250 ms ≈ 8 tick）：
- 历史：每 tick `record_lag_comp_history()`（主循环步骤 4），玩家 4 胶囊 + 世界位置 + 朝向（约 40 B/条），载具 OBB（约 120 B/条）；总内存 ≈ 64 KB + 77 KB；写入成本 ≈ 0.05 ms/tick。
- `FireCmd{seq, client_tick, view_tick, weapon, origin_hint, dir_hint, spread_seed}`；`view_tick` = 客户端渲染的**插值时间**对应 tick（≈ `estimated_server_tick − 3`，100 ms 插值缓冲）。
- 回溯：`rewind_ticks = clamp(server_tick_now − view_tick, 0, 8)`；超出部分**不补偿**，记录 `lagcomp_clamped`。
- 判定位置：**射手权威位置（服务器当前）+ 客户端上报方向**为起点，校验 `origin_hint` 与权威位置偏差 ≤ **0.5 m**（超出则使用权威位置并计数）；目标使用 `server_tick_now − rewind_ticks` 时刻的命中体，亚 tick 线性插值。
- 弹道（§6.7.3）：弹体按段扫描逐 tick 推进；第 n tick 目标命中体取 `view_time + n·dt`（不超过当前时间），到达当前 tick 后切换为实时碰撞；"打得准的人看到命中即命中"。
- **被害者保护**：若回溯命中与当前时刻视线检查 `LoS(shooter_now, target_now_pos)` 冲突（目标已在厚墙后 > 0.3 s 前进入掩体）→ **拒绝命中**，记 `lagcomp_wall_reject`【待调参】。
- 载具/火炮等非即时武器与爆炸：按服务器当前时刻计算，**不回溯**。
- `FireCmd` 随 `InputBundle` 冗余发送最近 2 次，服务器按 `seq` 去重；连续 9 tick 无输入则判断连接疑似中断（≥ 300 ms）。
- 指标（ARCHITECTURE §18.3）：`lagcomp_clamped_total`、`lagcomp_wall_reject_total`。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/lagcomp/history.rs` —— 持有 `sim::LagCompHistory`，在 tick 循环调用 `record`
- `crates/server/src/lagcomp/rewind.rs` —— `resolve_fire(cmd, shooter_state, now) -> FireResolution`（回溯 tick、起点校验、目标命中体视图）
- `crates/server/src/lagcomp/wall_guard.rs` —— 被害者保护 LoS 检查
- `crates/server/src/lagcomp/metrics.rs` —— 指标注册
- `crates/sim/src/lagcomp.rs`（小改）—— 暴露 `hitboxes_at(t, frac)` 与弹丸"按射手视角时间"推进的接口 `ProjectileClock::{ViewTime, Realtime}`
- `crates/server/tests/lagcomp_*.rs`；`crates/headless-client/src/scenarios/lagcomp_hit_rate.rs`
- `scripts/netem/rtt150.sh`（tc netem 条件）
### 要实现的行为
1. 每 tick 记录历史；环长 16；越界 `tick` 查询返回最近有效帧并计数。
2. 开火事件到达：计算 `rewind_ticks`；`>8` 夹紧并 `lagcomp_clamped++`；`<0`（未来）夹紧为 0。
3. 起点校验：`|origin_hint − auth_pos| > 0.5 m` → 采用权威起点并 `origin_corrected++`（不拒绝开火）。
4. 子弹生成：弹体携带 `view_time`；每 tick 目标命中体取 `view_time + n·dt`；超过"当前时间"后改实时；伤害结算走 M1-014。
5. 墙体拒绝：命中确认前以**当前**目标位置和射手当前位置做 LoS；被厚墙隔断且目标已掩体 > 0.3 s（以历史比对）→ 拒绝并 `lagcomp_wall_reject++`；阈值在 `/data/rules/lagcomp.json`。
6. 非即时武器/爆炸/载具：绕过回溯（`ProjectileClock::Realtime`）。
7. 命中回执：`HitConfirm`（仅射手）和 `DamageDir`（仅被害者）经 C2.events；结算仍由服务器。
8. 观测：`/metrics` 暴露夹紧/拒绝/起点校正计数，日志按 5 s 汇总。
### 接口与数据
```rust
pub struct FireResolution { pub rewind_ticks: u8, pub origin: Vec3, pub view_time: f32, pub clamped: bool, pub origin_corrected: bool }
pub fn resolve_fire(cmd:&FireCmdDecoded, shooter:&PlayerSnapshot, now:Tick, hist:&LagCompHistory) -> FireResolution;
```

## 不做什么
- 命中体几何与伤害公式：M1-014；弹道/穿透：M1-012；
- 客户端预测与"提前量"瞄准提示：M1-040/M1-057；
- 反作弊统计与回放：M1-027（基础）/M2；
- 载具开火回溯（不回溯）、载具命中体精细化：载具 OBB 仅占位；
- 带宽/AOI：M1-024。

## 验收标准
1. **150 ms RTT**（netem 单向 75 ms、抖动 ±10 ms）下 headless 命中率与 **0 延迟基线**差 ≤ 5%（绝对差，各 ≥ 2000 次射击样本；移动靶以固定脚本匀速往返；`lagcomp_hit_rate` 场景输出两条件命中率，写 STATUS）。
2. 无补偿对照：同条件关闭回溯时命中率下降 ≥ 15%（证明机制有效；记录数字，不作为失败条件但需写入）。
3. 夹紧：构造 `view_tick` 落后 12 tick 的开火，实际使用 8 tick 回溯，`lagcomp_clamped_total` 递增 1（单测 + `/metrics` 断言）。
4. 起点校验：偏差 0.49 m 采用客户端提示、0.51 m 采用权威位置并计数（单测）。
5. 墙体拒绝：目标于 0.5 s 前进入厚墙后，客户端（旧视图）射击被拒且 `lagcomp_wall_reject_total` 递增；目标 0.1 s 前刚入掩体则允许（2 例）。
6. 载具与爆炸不回溯（单测断言 `rewind_ticks==0`）。
7. 历史写入 ≤ 0.1 ms/tick（100 玩家 + 40 载具）；`resolve_fire` ≤ 20 µs/次（criterion）。
8. 30 min、20 客户端、150 ms RTT 的 `server_tick_ms` P99 ≤ 28 ms（并入 `test-loadtest-20` 前的先行数据，STATUS 记录）。

## 测试要求
- 单元测试：回溯夹紧、亚 tick 插值、环绕、起点校验、LoS 保护、`ProjectileClock` 切换。
- 集成/无头：`lagcomp_hit_rate`（0 ms vs 150 ms vs 150 ms 无补偿）；netem 脚本。
- 性能基准：criterion `resolve_fire`、`record_history`。
- 回归：指标名称与 ARCHITECTURE §18.3 一致。

## 性能预算
- 引用 ARCHITECTURE §6.10 阶段 7（历史写入 ≈ 0.1 ms）与阶段 4（回溯命中体检测并入 2.5 ms）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- ARCHITECTURE §6.7.2 里"8 tick = 266 ms"与 T2 250 ms 上限并存，实现取 8 tick（文档已定）；"0.3 s"墙拒绝阈值【待调参】。
- 命中率对比需要稳定的合成靶运动，headless 脚本必须确定性（固定种子），否则 5% 差难以复现。
- 游戏规则不得写入 `server`（ARCHITECTURE §4.3.4）：本任务只做"时间选择与校验"，命中结算在 `sim`。
- 规模 L；若超出，拆 `M1-025b`（墙体保护 + 载具历史）。
