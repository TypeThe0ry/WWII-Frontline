# M1-017 sim：压制

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：M
- 依赖：M1-012
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §8.1; GAMEDESIGN §8.2; GAMEDESIGN §8.3; GAMEDESIGN §8.4; GAMEDESIGN §6.5; GAMEDESIGN §9.1; ARCHITECTURE §12.3; ARCHITECTURE §6.5.3

## 背景
压制是"纯玩家体验效果 + bot 行为输入"，**不改变伤害**（GD §8）。服务器权威计算每名玩家的压制值 `S ∈ [0,100]`，经 `self_state.suppression(4b)` 与 `Suppress{source_dir,intensity}` 事件下发；客户端把它转成音画效果（M1-057/M1-063，AudioWorklet 闪避见 ARCHITECTURE §12.3）；bot 据 S 切换状态（M1-038）。核心公式（均【待调参 / 待测 → REFERENCE_MEASUREMENTS §suppression】）：
- 触发：对每颗**敌方未命中**子弹，求弹丸路径与目标头部上 0.2 m 到脚下 0.2 m 的胶囊轴线最近距离 `d`；`d ≤ d_max=5.0 m` 时 `s = caliber_weight × (1 − d/d_max)^1.2 × bullet_speed_factor`；`S += s`。`caliber_weight`：手枪/9 mm/.45 ACP 0.8；.30 Carbine/7.92×33 1.5；7.92×57/.30-06 步枪 2.2；MG 2.8；.50 BMG 6.0；`bullet_speed_factor = min(1, v/500)`。爆炸 `S += 40×(1−r/R_supp)²`，`R_supp=15 m`（炮弹 25 m），被墙隔挡 ×0.5。友军子弹不压制（友军爆炸会）。每 tick 每玩家最多处理 12 发。
- 累积/衰减：受压后 0.5 s 窗口内不衰减；之后 `dS/dt = −(6 + 0.12·S)`；姿态衰减修正 趴 ×1.3、蹲 ×1.15、站 ×1.0、掩体后 ×1.4（对射线被障碍阻挡）。示例：MG42 d=2 m 每发 s ≈ 1.5，持续扫射 1.5 s 到 ≈ 35，4 s 到 80+。
- 效果输出 `e = S/100`：画面径向模糊 `6·e` px、镜头晃动 `0.6°·e @12 Hz`（**同时叠加到视角与射击射线**，对命中有实际影响）、`supp_spread = 1 + 1.5·e`（乘到盲射与 ADS 散布，M1-013）、去饱和 `1−0.55e`、暗角 `0.6e`、移速 `×(1−0.10e)`、ADS 时间 `×(1+0.30e)`、听觉 −3 dB·e、"被钉死"：`S ≥ 85` ADS 失效 0.5 s。
- 反刷分：压制本身不给分；被压制者 5 s 内被队友击杀，压制者 +3 分（`suppression_assist`，计分 M2，本任务只产出事件）。
- bot 阈值：`S ≥ 30` 找掩体，`S ≥ 60` 后撤/匍匐。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/suppression/detect.rs` —— 弹丸擦过检测（段到胶囊轴线最近距离，**在 M1-012 弹丸步进中调用**，对附近 ≤ 5 m 的玩家，使用 AOI 网格/空间查询，避免 O(N×弹)）
- `crates/sim/src/suppression/state.rs` —— `SuppressionState{S, window_until}`、累积与衰减
- `crates/sim/src/suppression/effects.rs` —— `SuppressionEffects{move_mult, ads_mult, spread_mult, shake_deg, desat, vignette, blur_px, pinned}`（纯函数 `effects(S)`）
- `crates/sim/src/suppression/anti_farm.rs` —— 5 s 窗口内的压制者记录与 `SuppressionAssist` 事件
- `data/combat/suppression.json`（+ schema；全部系数）
- `crates/sim/tests/suppression_*.rs`；`crates/sim/benches/suppression.rs`
### 要实现的行为
1. 检测：每颗未命中的敌方子弹，每个子步对 ≤ 5 m 范围内的敌对玩家求最近距离；同一发子弹对同一玩家只触发一次；同 tick 多弹合并（求和后一次更新，且 ≤12 发/玩家/tick）。
2. 累积/衰减：按公式；`S` clamp [0,100]；窗口计时用 tick（0.5 s = 15 tick）。
3. 效果输出：`effects(S)`；`pinned`：`S ≥ 85` 时 ADS 输入在 0.5 s 内被忽略（服务器忽略 `aim` 位）。
4. 对射击的影响：服务器在 M1-013 的散布公式中读取 `supp_spread` 与晃动角，晃动由确定性噪声（`Rng::for_(tick, player, SUPPRESS_SHAKE)`）生成，客户端预测使用相同公式。
5. 事件：对被压制者产出 `Suppress{source_dir:u8, intensity:u8}`（仅该玩家，且节流 ≤ 10 Hz）；`self_state.suppression` 取 `round(S/100×15)`（4 bit）。
6. 爆炸压制：接收 M1-015 的爆炸输入，按公式叠加（含墙隔挡）。
7. 反刷分：记录每名玩家最近 5 s 内压制他的敌人；该玩家被击杀时对压制者产出 `SuppressionAssist` 事件（每次击杀每压制者最多 1 次）。
### 接口与数据
```rust
pub fn on_projectile_pass(sim:&mut Suppression, shot:&ProjectileSeg, targets:&TargetIndex) -> SmallVec<[SuppressHit;8]>;
pub fn step_suppression(s:&mut SuppressionState, dt:f32, stance:Stance, in_cover:bool, tick:Tick);
pub fn effects(s:f32) -> SuppressionEffects;
```
`/data/combat/suppression.json`：`d_max_m 5.0`、`exponent 1.2`、`caliber_weight{…}`、`decay{base 6, per_s 0.12}`、`stance_mult{prone 1.3,…}`、`window_s 0.5`、`pinned_s 85`。

## 不做什么
- 客户端音画效果（径向模糊、去饱和、嗡声、AudioWorklet 闪避）：M1-057/M1-063（本任务提供参数输出）；
- bot 对压制的行为反应：M1-038；
- 计分（+3）与记分板：M2；
- Rally（累积 ×0.7）：M3；
- 载具炮弹近失压制（25 m）：随 M1-023 的爆炸事件调用本任务接口。

## 验收标准
1. 擦过距离阈值：`d=5.0 m` 触发 s=0；`d=5.01 m` 不触发；`d=0` 时 `s = caliber_weight × speed_factor`（单测；7.92×57，v=740 ⇒ s=2.2）。
2. 累积曲线：MG 类 d=2 m 每发 s≈1.5（2.8×0.6^1.2=1.52，误差 < 2%）；1200 rpm 连续 1.5 s 的 S ≈ 35±5（含衰减，`cargo test suppression_mg42`）。
3. 衰减：S=50、站姿、窗口过后 1 s 的衰减量 = 6+0.12×50 = 12（±2%）；趴姿同条件 ≈ 15.6（×1.3）；掩体后 ×1.4（单测）。
4. 窗口：受压后 0.5 s（15 tick）内 S 不下降，第 16 tick 起下降（单测）。
5. 效果输出：S=100 时 `move_mult=0.90`、`ads_mult=1.30`、`spread_mult=2.5`、`desat=0.45`（1−0.55）、`vignette=0.6`；S=0 全为恒等（单测，与 GD §8.3 逐项对齐）。
6. `pinned`：S=84 不触发、S=85 触发；0.5 s 内 `aim` 位被忽略，第 0.5 s 后恢复（单测）。
7. 友军子弹不压制；友军爆炸压制（单测 2 例）；同一发子弹只触发一次（单测）。
8. 反刷分：被压制者 4.9 s 后被队友击杀产出 1 个 `SuppressionAssist`，5.1 s 后 0 个（单测）。
9. 性能：50 名射手同时开火、100 个玩家的擦过检测一 tick ≤ 0.3 ms（`cargo bench -p sim suppression`）。

## 测试要求
- 单元测试：公式/阈值/窗口/姿态修正/事件节流/`effects`；`self_state` 4 bit 量化往返。
- 集成：headless 射手与目标，`Suppress` 事件到达且强度随距离单调。
- 性能基准：`suppression` criterion 基线。
- 回归：确定性哈希（晃动噪声）。

## 性能预算
- 并入 ARCHITECTURE §6.10 阶段 4（≤ 2.5 ms）；本任务 ≤ 0.3 ms/tick（最坏）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 擦过检测必须与 M1-012 的子步积分共享几何，避免重复遍历弹体；若性能不达标，把检测降为每 tick 一次（弹段端点）。
- 压制晃动叠加到射击射线会使服务器与预测同时依赖 `Rng::for_`，两端 seed 公式必须一致（ARCHITECTURE §4.3.2）。
- 数值全部【待调参】，批 M1 压制测量 U-01–U-11 由 M1-071 回填（需人工录像，H-053）。
