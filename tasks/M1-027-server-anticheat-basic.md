# M1-027 server：基础校验（速度/射速/视角）

## 元数据
- 里程碑：M1
- 工作流：server
- 规模：M
- 前置依赖：M0-018, M1-011
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §15.1; ARCHITECTURE §15.4; ARCHITECTURE §6.3; ARCHITECTURE §18.3; GAMEDESIGN §9.6; GAMEDESIGN §19.5

## 背景
服务器权威让大多数作弊（速度、无限弹药、飞天、伤害修改）天然失效；本任务补充**可观测的校验与计数**，满足 E1-15（"注入 1.3× 速度/超射速输入被纠正/拒绝且计数"）。诚实边界（ARCHITECTURE §15.4）：浏览器客户端不可信，无法防内存篡改/外部硬件自瞄；目标是"成本高、危害小、可发现"。统计异常检测（爆头率/反应时间/自瞄特征）与回放在 M2。
M1 校验清单（ARCHITECTURE §15.1 的子集）：
- **输入**：`seq` 单调、`tick` 在窗口内（< 当前 −2 或 > 当前 +10 丢弃）、重复/回放输入丢弃 + 计数；连续异常 → 断开。
- **移动速度**：位移/tick ≤ `sprint_mps × dt × 1.15`（含坡度/载具修正）；加速度上限；服务器权威直接纠正，累计偏差记 `suspicion_move`（由 M1-011 的 `validate.rs` 提供 `check_displacement`）。
- **瞬移/穿墙**：客户端 `pred_hash` 与权威不一致次数（每 10 s 滚动窗口）。
- **视角**：yaw/pitch 角速度 ≤ **8000°/s**【待调参】；单帧突变阈值；钳制 + 计数。
- **射速**：每武器冷却/射速含 **5% 容差**；弹匣扣除；拒绝多余射击（由 M1-013 在 sim 内强制，本任务计数/日志）。
- **出射点**：`origin_hint` 与权威位置偏差 ≤ 0.5 m，使用权威位置（与 M1-025 共享计数 `origin_corrected`）。
- **弹药/伤害**：全由服务器持有，客户端无写权限（本任务做代码审计测试：`InputCmd`/`FireCmd` 中无弹药/伤害字段）。
- **载具**：座位权限、速度/扭矩上限（M1-022 sim 保证；本任务计数违规）。
- **速率**：C0 各消息（见 M1-026）。
- 指标：`suspicion_move`、`input_rejected_total{reason}`、`view_rate_clamped_total`、`fire_rate_rejected_total`；日志含 `player_id`、`tick`、`reason`、数值。

## 具体内容
### 要创建/修改的文件
- `crates/server/src/anticheat/mod.rs` —— `AntiCheat` 汇总器，按玩家维护滚动计数
- `crates/server/src/anticheat/input_check.rs` —— seq/tick 窗口、重放、连续异常断开
- `crates/server/src/anticheat/move_check.rs` —— 调用 `sim::movement::validate::check_displacement`、`pred_hash` 不匹配统计
- `crates/server/src/anticheat/view_check.rs` —— 角速度限幅与计数
- `crates/server/src/anticheat/fire_check.rs` —— 射速异常计数（读取 sim 返回的 `FireRejected` 事件）
- `crates/server/src/anticheat/report.rs` —— 汇总到 `suspicion_*` 指标与结构化日志（tracing JSON）；每 60 s 打印前 5 名可疑玩家
- `data/rules/anticheat.json` —— 阈值（`view_max_dps 8000`、`move_tolerance 1.15`、`fire_tolerance 1.05`、断开阈值）
- `crates/server/tests/anticheat_*.rs`；`crates/headless-client/src/cheats/{speedhack,firerate,aimsnap}.rs`（注入器，仅测试用）
### 要实现的行为
1. 输入摄取阶段按顺序检查：窗口 → 重复 → 视角角速度 → 移动向量模长（≤1）→ 按键合法性。
2. 违规处置：丢弃/钳制并计数；`window_10s` 内同类违规 ≥ N（默认 30）→ 记 `warn`；≥ 3N 或连续 9 tick 全异常 → 断开（`Disconnect{reason}`）。
3. 速度作弊注入：客户端发送 1.3× 速度的移动（位置自报不被信任，因协议不含位置；实质为 `pred_hash` 与权威持续不匹配 + 位移校验）→ 服务器仍按权威速度移动（纠正）并 `suspicion_move++`。
4. 射速注入：以 2× cap 速度发 fire 位，sim 拒绝多余发射，`fire_rate_rejected_total++`。
5. 视角注入：单帧 180° 瞬转，钳制到 `8000°/s × dt`，`view_rate_clamped_total++`。
6. 所有计数按玩家、按局保存；局终输出摘要日志（M2 接复核队列）。
7. 不得误伤：合法高灵敏度玩家（鼠标 3000°/s 峰值）和网络抖动下的乱序输入不触发断开（用真实录制轨迹/合成抖动测误报）。
### 接口与数据
```rust
pub enum Violation { SeqReplay, TickWindow, ViewRate, MoveSpeed, FireRate, OriginHint, PredHash }
impl AntiCheat { pub fn on_input(&mut self, p:PlayerId, cmd:&InputCmd, auth:&PlayerAuthState) -> Verdict; }
pub enum Verdict { Accept, Clamp(Violation), Drop(Violation), Disconnect(Violation) }
```

## 不做什么
- 爆头率/反应时间/自瞄统计检测、10 分钟输入回放、人工复核队列、封禁管理 API：M2；
- 反透视（AOI 控制）：M1-024；
- 命中回溯合理性（lagcomp 夹紧）：M1-025；
- 客户端侧任何"反作弊"措施（不可靠，ARCHITECTURE §15.4）；
- 指挥官能力/部署校验：规则在 sim，计数并入 `deploy_requests_total`（M1-026）。

## 验收标准
1. 注入 **1.3× 速度**输入（`speedhack` 注入器，持续 30 s）：服务器权威速度不超过 `sprint×1.0`（对比位置轨迹最大速度 ≤ 6.4 m/s×1.02），`suspicion_move` 计数在 30 s 内 ≥ 100（`/metrics` 断言）。
2. **射速超限**被拒：`firerate` 注入器以 2× cap 开火，服务器实际发射数 ≤ `cap×1.05×T`（T=10 s），`fire_rate_rejected_total` ≥ 实际超限发数的 90%（单测 + headless）。
3. 视角瞬转 180° 单帧被钳制至 ≤ `8000°/s × (1/30) ≈ 266.7°`/tick 内的合法值，`view_rate_clamped_total` 递增；合法 3000°/s 甩枪 0 误报（1000 条合成轨迹，误报率 = 0）。
4. 乱序/重复/过期输入：seq 重放 100 次全部丢弃（`input_rejected_total{reason="replay"}`=100）；窗口外（−3、+11 tick）丢弃；窗口内（−2、+10）接受（边界单测）。
5. 持续异常断开：连续 9 tick 全部无效输入或 `3N` 违规 → 收到 `Disconnect`；正常玩家 30 min 20 客户端场景中被误断开数 = 0。
6. 出射点 0.51 m 偏差使用权威位置并计数（与 M1-025 共享，单测）。
7. 审计：`cargo test no_client_authoritative_fields` 断言 `InputCmd/FireCmd` 不含位置/弹药/伤害字段（反射式检查 protocol 结构）。
8. 性能：校验链 ≤ 5 µs/条输入；100 名玩家全部 3 条输入 ≤ 0.8 ms（ARCHITECTURE §6.10 阶段 1 含校验，`cargo bench`）。

## 测试要求
- 单元测试：各校验边界、滚动窗口、断开阈值、误报用合成人类轨迹。
- 集成/无头：三个注入器 + 正常客户端混合 20 客户端 5 min。
- 性能基准：criterion `anticheat_on_input`。
- 回归：阈值入 `/data`，改动需 PLAN_ISSUES 记录。

## 性能预算
- 引用 ARCHITECTURE §6.10 阶段 1：输入摄取与校验（含 anti-cheat）100 玩家 ≤ 0.8 ms/tick。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 因协议不下发位置，"速度作弊"的实际表现是预测与权威分歧；校验价值在于计数与告警，权威本身已免疫。
- 阈值（8000°/s、1.15、5%）均【待调参】，误报率比漏报更需控制；M2 以真实对局数据重新标定。
- 注入器放在 `headless-client`，不得出现在发布的服务器二进制中（`cfg(feature = "cheat-injectors")`）。
