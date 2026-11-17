# M3-011 sim：狙击步枪与屏息

## 元数据
- 里程碑：M3
- 工作流：sim
- 规模：M
- 依赖：M3-001, M2-013
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.3; GAMEDESIGN §6.4; GAMEDESIGN §6.5; GAMEDESIGN §6.10; GAMEDESIGN §17.3; GAMEDESIGN §5.17

## 背景
M0-012 已实现栓动步枪状态机、散布/后坐与段扫描弹道；M2-013 已有 BAR/MG 的部署与摆动（sway）。本任务补齐 Sniper 兵种的武器行为：带倍率瞄准镜（高倍镜）、**屏息**、**瞄具零点分划**、栓动单发/夹装装填、枪口暴露。武器与 loadout 数据来自 M3-001：US M1903A4（M73 2.5×，5 发 ×12）、M1C Garand（M81 2.2×）、GER Kar98k+Zf39（4×）、Kar98k+ZF41（1.5×）、G43+ZF4（4×）。
关键参数（GD §6.3/§6.5，全部从 `/data/weapons` 读取，不得硬编码）：M1903A4 栓动 cycle 1.6 s、dmg 110、F0/F1/min 200/800/0.85、eff 800 m、ADS 0.35 s、pen 230；Kar98k Zf39 cycle 1.6 s、dmg 110、ADS 0.40；Zf41 cycle 1.55 s、dmg 105、ADS 0.34；G43 ZF4 cap 110 rpm、dmg 78、ADS 0.40。ADS 散布 0.03°（栓动狙击）。弹道：.30-06 `k_drag = 0.00075`、初速 853 m/s；7.92×57 `k = 0.00068`、初速约 760 m/s（以 /data 为准）。
**屏息**（GD §6.5）：按住屏息键 4 s 使 sway 清零，之后 2 s 内 sway 加倍并强制恢复（黑屏晕眩 0.5 s 前不得连续屏息），体力 100 中扣 30。Spotter 在 100 m 内时 Sniper sway ×0.8（来自 M3-010 的修正因子）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/weapon/scope.rs` —— `ScopeState{zoom, zero_m, reticle_zero_cycle}`：零点 100/200/300/400/500/600 m 分划循环（仅带镜枪有全程分划，机械瞄具仅 100/300，GD §6.4）；分划补偿角 `θ = atan(下坠/距离)` 由弹道表预计算（`ballistic_table(weapon, max_range)`）。
- `crates/sim/src/weapon/breath.rs` —— 屏息状态机 `Idle → Holding(4 s) → Recover(2 s, sway×2) → Cooldown(blackout 0.5 s)`；体力扣 30；与 M1-011 的体力系统对接；被打断规则（受伤/移动/开火后重置）。
- `crates/sim/src/weapon/sway.rs`（M2-013 已有）—— 加入狙击 sway 参数：幅度、频率取 /data，Spotter 协同因子 0.8 乘入。
- `crates/sim/src/weapon/bolt.rs`（M0-012 已有）—— 补齐栓动：单发装填 0.65 s/发、夹装 3.0–3.2 s 二选一（GD §6.3 规则）、开镜时从机械瞄具到倍镜 FOV 缩放。
- `crates/sim/tests/sniper.rs`、`crates/sim/benches/bolt_ballistic.rs`。
### 要实现的行为
1. 倍率瞄准：ADS 时 FOV 按 `sight.zoom` 缩放；服务器按 `ads_time_s` 判定"准星稳定"时刻，未稳定前开火按盲射/过渡散布。
2. 零点分划：按键循环零点；开火时枪管俯仰补偿用零点对应的预计算角；零点切换需 0.2 s 动画不在规则里，只记录当前零点用于校验（服务器权威：`zero_m` 随输入 `aux` 同步）。
3. 屏息：持续按键且体力 ≥ 30 才能开始；4 s 内 sway 为 0；之后 2 s 强制恢复、sway ×2；结束后 0.5 s 黑屏晕眩不可再屏息；体力 −30。
4. 暴露：开火后 `concealment` 恢复 ×1.0（M3-010 提供接口 `on_shot(player)`）；Scope Glint 事件钩子（开镜且正对敌 ±25° ≤ 400 m，M3-010 消费）。
5. 命中：沿用 M0-012/M1-014 的段扫描与衰减；Zf39/ZF4 等带镜 G43 的半自动 cap 110 rpm。
6. 数据驱动：所有数值来自 `/data/weapons/*`，`GameData.hash` 参与客户端预测校验。
### 接口与数据
- `ScopeState` 与 `BreathState` 作为 `PlayerAuthState` 的附加字段（`sim-wasm` 预测子集需包含，`client-predict` feature）；不得增加 `self_state` 超过 14 B 预算：`breath(3b)` + `zero_idx(3b)` 复用现有 `cooldown_bits(8b)` 的保留位（ARCHITECTURE §6.5.3），位宽取值写 PLAN_ISSUES 细化。

## 不做什么
- 暴露/迷彩/OP → M3-010；bot 狙击 AI → M3-024；
- 狙击镜 UI 视觉与屏息音效 → UI/音频任务；武器模型 → M3-042；
- 其它武器类（自动/机枪）已在 M2-013。

## 验收标准
1. 800 m 处命中判定与解析弹道一致：对 M1903A4（853 m/s，k=0.00075）与 Kar98k Zf39，`criterion`/单测中数值积分得到的下坠与解析 `x(t)` 的误差 < 3%（沿用 M0-012 方法），分划补偿后落点偏差 ≤ 0.3 m（零距离对应射程内）。
2. 屏息：持续按住时 sway 振幅在 0–4 s 为 0；4–6 s 为基准 ×2；0.5 s 黑屏内再按无效；体力扣 30（误差 ≤ 1）；计时误差 ≤ 1 tick。
3. Spotter 100 m 内：sway ×0.8（误差 < 1%）；101 m 外 ×1.0。
4. 栓动节奏：M1903A4 实际射速 ≤ 60/1.6 = 37.5 rpm（误差 ≤ 2%）；夹装 3.0–3.2 s、单发 0.65 s/发（单测）。
5. 零点循环 100→600 m 共 6 档、机械瞄具 2 档；每档补偿角与预计算表误差 < 0.01°。
6. 带镜 ADS 时间：Zf39 0.40 s、ZF41 0.34 s、M73 0.35 s（±1 tick）。
7. 单次狙击射击处理（含弹道）≤ 0.05 ms；与 `cargo bench -p sim bolt_ballistic` 的基线比较回归 ≤ 10%。

## 测试要求
- 单元测试：屏息状态机全转移、体力不足、被打断；零点分划；bolt 装填二选一；ADS 稳定时间。
- 集成/无头：headless 狙击手对 800 m 靶 100 次射击，命中率与解析期望一致（预期在无散布下 100% 命中分划正确时）。
- 性能基准：`criterion` 基线 `bolt_ballistic`。
- 回归：确定性哈希（含预测哈希）更新；`sim-wasm` 体积检查。

## 性能预算
ARCHITECTURE §6.10 阶段 4（武器与弹道 2.5 ms，≤ 400 活动弹体段）：单发 ≤ 5 µs/段；本任务不得使阶段 4 总预算回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD §6.5 "屏息"一句话说明歧义较多（"黑屏晕眩 0.5 s 前不得连续屏息"），本任务按上方状态机解释并写 PLAN_ISSUES；狙击专项的手感对标项为 REFERENCE_MEASUREMENTS W-27–W-29（M3-052）。
- 800 m 弹道时间 ≈ 1.4 s 量级，须配合 M1-025 延迟补偿"只回溯目标命中盒"的规则（GD §6.4），本任务不改服务器。
- 预测 WASM 与原生按容差对账（R5），狙击 sway 随机种子须走 `Rng::for_(tick, entity, purpose)`。
