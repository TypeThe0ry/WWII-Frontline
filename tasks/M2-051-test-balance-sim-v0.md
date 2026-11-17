# M2-051 测试：balance-sim v0

## 元数据
- 里程碑：M2
- 工作流：test
- 规模：L
- 前置依赖：M2-008, M2-028
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.3; ARCHITECTURE §17.5; ARCHITECTURE §4.5; GAMEDESIGN §20; GAMEDESIGN §21; MILESTONES §3.3

## 背景
ARCHITECTURE §17.3：`balance-sim`：无渲染、无网络，以**缩比对局**（总控裁定：默认 20v20、30 min、简化物理/无渲染/粗粒度导航）跑 bot 对战 N 局（并行进程）；全尺寸 100 人局只做少量抽样（≥ 5 局），输出 CSV：胜率（按地图/阵营）、局时长、武器击杀占比、各兵种 KDA、资源曲线、据点争夺周期。目标：以 REFERENCE_MEASUREMENTS 的黑盒测量校准的"节奏指标"（TTK、冲锋时间等）作为回归基线；改动数值的 PR 必须附带前后对比。**bot 行为 ≠ 真人，只用于发现数值异常（一边倒、不可用武器），不替代真人试玩。**
M2 退出标准 E2-11（已按总控裁定改口径）：缩比对局单核 ≥ 10× 实时，200 局缩比对局可在夜间作业 ≤ 4 h 内完成（CI nightly `balance` job），CSV 字段齐全，同 seed 可复现。依赖方向（ARCH §4.5）：`balance-sim` 允许依赖 `protocol`、`sim`、`bots`，**不得依赖 `server`/`gateway`/`client`**；因此模拟循环必须自行驱动 `sim::World::step` 与 `bots`（不经网络与 AOI 编码）。M2-008 提供 Frontline 规则，M2-028 提供 RuleCommander，M2-029/M2-027 提供 bot 行为。

## 具体内容
### 要创建/修改的文件
- `crates/balance-sim/src/main.rs` —— CLI：`balance-sim run --matches 200 --seed 1 --jobs N --map carentan --mode frontline --out reports/balance/`；`--difficulty`、`--time-limit-s`、`--bots-per-team 50`。
- `crates/balance-sim/src/match_runner.rs` —— 单局运行器：构建 `World`（数据来自 `/data`、地图 `carentan` 灰盒碰撞/据点）、两队各 50 bot、按固定步长 30 Hz 推进直至胜负或时限；无 sleep；确定性（种子 = `base_seed + match_index`，`Pcg64Mcg`）。
- `crates/balance-sim/src/parallel.rs` —— 多进程并行（`std::process::Command` 重入自身子进程，每进程跑一批局，禁用线程内共享 `World`；`sim` 仍单线程）；结果汇总。
- `crates/balance-sim/src/stats.rs`、`csv.rs` —— 统计采集与 CSV 输出。
- `reports/balance/` —— 输出目录（CSV 与摘要 JSON 不进 git；基线摘要 `reports/balance/baseline_m2.json` 进 git）。
- `.github/workflows/balance.yml` —— nightly/手动：200 局缩比对局、上传 CSV artifact。
- `docs/` 不修改；字段说明写入 `crates/balance-sim/README` 不创建，改为 `--help` 与 CSV 头注释。
### 要实现的行为
1. 输出 CSV（至少 5 个文件，字段齐全）：
   - `matches.csv`：`match_id,seed,map,mode,winner,victory_reason,duration_s,lines_a,lines_b,ticks,wall_ms,speedup`
   - `weapon_kills.csv`：`match_id,weapon_id,kills,headshots,damage_dealt`（得出武器击杀占比）
   - `role_kda.csv`：`match_id,team,role,k,d,a,score`
   - `resources.csv`：`match_id,t_s,team,manpower,munitions,fuel`（每分钟）
   - `capture_cycles.csv`：`match_id,line,owner_from,owner_to,t_s`（据点易主周期）
2. 同 seed 同参数两次运行，`matches.csv`（除 `wall_ms/speedup` 列）与全部其它 CSV 逐字节相同；`state_hash` 序列写入 `matches.csv` 的 `final_hash` 列。
3. 汇总：胜率（按阵营）、局时长分布、武器击杀占比（> 25% 或 < 0.5% 标红，供 M3-046 判定）、兵种 KDA、节奏指标（平均 TTK、首次接触时间）。
4. 速度记录：每局 `speedup = 模拟时长 / 墙钟`；汇总输出分位数；若 `speedup < 目标`，在摘要显式告警。
5. 失败容错：单局 panic/超时（> 墙钟 N 分钟）被隔离并记录，不拖垮整批。
### 接口与数据
```rust
pub struct RunConfig { pub matches: u32, pub base_seed: u64, pub jobs: u32, pub map: String, pub mode: Mode, pub bots_per_team: u16, pub time_limit_s: u32, pub difficulty: Difficulty }
pub fn run_match(cfg: &RunConfig, index: u32) -> MatchResult; // 纯函数式，确定性
```

## 不做什么
- 不做 Assault/Skirmish 与多地图（M3-046、M4）；不做武器伤害调参本身（只产出数据，调参写入 `/data` 的 PR 另做，附前后对比）。
- 不做服务器网络/AOI 路径的模拟；不做载具驾驶 bot（M3-023），M2 局内载具仅按 bot 现有行为。
- 不替代真人试玩；不做真人平衡结论。
- 不在 `balance-sim` 内实现游戏规则（只调用 `sim`）。

## 验收标准
1. 速度与吞吐（E2-11）：缩比对局（默认 20v20、30 min）单核 `speedup` 中位数 ≥ 10×（实测，记录机型）；nightly runner（≥ 8 逻辑核）上 `balance-sim run --matches 200`（缩比）总墙钟 ≤ 4 h。另跑全尺寸 100 人局抽样 ≥ 5 局，记录 `speedup`（仅记录，不设阈值）。若缩比口径未达标，写 PLAN_ISSUES 并含根因分解（sim/bots/规则耗时占比），如实记入 STATUS。
2. 可复现：同 seed 运行两次 `--matches 20`，除 `wall_ms/speedup` 外 CSV 逐字节一致（`diff`，0 差异）。
3. CSV 字段齐全：5 个文件的列集合与上述一致（`tests/csv_schema.rs`），无空列、无 NaN。
4. 容错：注入 1 局 panic（测试开关），其余 19 局完成，`matches.csv` 有该局错误记录。
5. 不依赖违规：`cargo metadata` 依赖图检查通过（`balance-sim` 仅依赖 `protocol/sim/bots`，ARCH §4.5 脚本）。
6. 200 局输出的摘要包含：双方胜率、局时长 P50/P90、武器击杀占比最高/最低 3 项、各兵种 KDA；`baseline_m2.json` 入库作为回归基线（**仅记录，不判通过/失败**）。

## 测试要求
- 单元测试：CSV 写出与回读；统计聚合（已知小样本）；种子派生；并行结果合并顺序无关。
- 集成/无头测试：`tests/short_match.rs`：2 bot 小队 5 分钟局确定性；`--matches 4 --jobs 2` 冒烟。
- 性能基准：`cargo bench -p balance-sim step_100`（100 bot 单 tick 总耗时，对照 ARCH §6.10 阶段 2–6）。
- 回归：与 `reports/balance/baseline_m2.json` 比较的脚本 `tools/balance/compare.py`（输出差异表，M3-046 使用）。

## 性能预算
- 缩比对局单核 ≥ 10× 实时；200 局缩比对局夜间作业 ≤ 4 h（总控裁定，替代原"200 局 < 40 min"）。参考：服务器预算 §6.10 单 tick 约 16.4 ms 含网络/AOI/编码；去掉网络后阶段 1–6 约 11.5 ms，对应单核 ≈ 2.9× 实时（粗算）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test -p balance-sim`、nightly `balance` 作业）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **口径已裁定（缩比对局）**：原"> 10× 实时"与"200 局 ≤ 40 min"在 100 人全尺寸局下不可同时达到（以下为推导）：一局 90 min × 200 = 18 000 模拟分钟，≤ 40 min 墙钟意味着聚合吞吐 ≥ 450× 实时；8 核并行时每进程需 ≈ 56× 实时，而 100 bot 的 `sim` 单 tick 成本（KCC 3 ms + 弹道 2.5 ms + bot 3–5 ms + 规则 0.7 ms ≈ 10 ms）仅给出约 3× 实时/核。总控已裁定采用缩比对局口径（见背景）；任务内仍须实测并如实上报。
- 远程分批执行时需 CPU 隔离；nightly runner 规格决定可行性（H-014）。
- bot 对战结果受 bot 难度与行为影响大，结果只作异常检测。
