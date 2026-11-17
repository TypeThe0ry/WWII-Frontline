# M2-054 对标测量：M2 批回填

## 元数据
- 里程碑：M2
- 工作流：data
- 规模：M
- 依赖：M1-071, M2-009, M2-010, M2-014
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §9.1; REFERENCE_MEASUREMENTS §4.7; REFERENCE_MEASUREMENTS §4.10; REFERENCE_MEASUREMENTS §4.11; REFERENCE_MEASUREMENTS §4.12; REFERENCE_MEASUREMENTS §7; REFERENCE_MEASUREMENTS §8; REFERENCE_MEASUREMENTS §1.2

## 背景
**本任务含 `[人工：录像与协助者]`**：需要人类录制并存放私有录像（H-060），执行者做分析与回填。**缺录像则任务标 `blocked`**（README §6），登记 STATUS 顶部"blocked 清单"，追加 `docs/NEEDS_HUMAN.md` 条目，继续做下一个依赖已满足的任务。
批 M2 必测项（REFERENCE_MEASUREMENTS §9.1）：
- 对局：R-02（占点人数上限与饱和人数）、R-06（对局总时长/分阶段计时）、R-07（胜利条件倒计时）、R-11（战线推进规则）、R-13（小队规模/数量上限/兵种名额随人数变化）、R-14（复活后部署选择限制）；占点多人规律（R-01 的 5/10 人，需多人协助或公开视频）；资源 R-08（产出速率随控制据点数）、R-09（上限/初始/与队伍规模关系）全人数规模。
- 建造（§4.7）：B-04–B-06、B-09、B-14–B-16、B-18。
- UI（§4.11）：UI-09–UI-11（弹药/生命 UI 变化延迟、指南针/小队成员标识更新与距离、名牌显示距离与淡入淡出）、UI-13–UI-15（语音/聊天指示、回合开始/结束过场与地图投票时长、通知停留时间与优先级）。
- 音频（§4.12，仅测行为，不提取音频）：AU-03（枪声延迟）、AU-05–AU-08（近距离语音衰减、墙体遮挡衰减、爆炸/炮声可闻距离、载具引擎可闻距离与音高范围）。
- U-12/U-13（压制相关）；G-09–G-11、G-13、G-14（手雷/烟雾类）。
合规边界（§1.2、README §5.7）：只黑盒观测（公开视频/自录视频），**禁止反编译/拆包/内存读取/协议解析/素材提取**；录像与原始数据**不入库**（`.gitignore`：`docs/measurements/raw/`、`assets/incoming/ref-captures/`）；音频录像只做时序分析，不得导出为素材。文档与提交信息中不得出现参考作品名称/截图。

## 具体内容
### 要创建/修改的文件
- `docs/measurements/{ID}.md` —— 每个测量项一份记录（REFERENCE_MEASUREMENTS §8.1 模板：元数据、原始数据逐次重复、统计、公开资料数值事实、我方复测、映射的 `/data` 字段、备注）；不含录像文件与截图。
- `docs/measurements/summary.csv` —— 汇总行（§8.2 列定义）；状态约定 §8.3（`待测/已测/已映射/已复测/locked`）。
- `/tools/ref-analysis/*`（若 M1-071 已建则扩展）—— 帧读取、模板匹配、音频包络等分析脚本（仅本地私有运行，数据路径参数化）。
- `data/**`（按映射）—— 回填 `/data` 字段：`modes/frontline.json:adjacency`、`resources.json:{income_per_min_by_*,cap,initial}`、`squads.json`、`roles/*.json:slots_by_team_size[]`、`spawn.json:role_slot_*`、`match.json:{duration_s,win_countdown_s}`、`capture.json:max_effective_players`、`build/*.json`、`ui/timings.json`、`audio/*.json`；每个字段 `source` 改为 `ref_measurement` 并引用测量 ID，`tuning_status: calibrated`（经复测后）。
- `reports/ref/m2_retest.json` —— 我方复测（REFERENCE_MEASUREMENTS §7.4）与误差。
### 要实现的行为
1. 逐项分析（读帧、读进度条、二分距离法），每项重复 ≥ 5 次（§2.5），记录中位数与离散度；置信度分级 A/B/C。
2. 映射规则（§7.2）：测量值 → `/data` 字段；史实与对标冲突按 §6.1 处理（外形/口径/弹容量/射速以史实为准；手感类以对标）。
3. 同法复测（§7.4）：在我方游戏内按同一方法测量并写入记录；误差须在容限内（§7.3）；超容限则迭代调参（改 `/data`，不改代码）。
4. 缺录像/缺协助者的项：保持 `待测`，不得编造数据；在 summary.csv 与 STATUS 列明缺口；整体缺录像则任务 `blocked`。
5. 自动化回归：把已 `locked` 项的复测纳入 `balance-sim`/headless 回归（§7.5）作为后续基线（可先登记，M3-052 前补齐）。
6. 来源合规声明：每份记录写明录像来源类型（自录/公开视频链接）与无素材提取声明。
### 接口与数据
```
docs/measurements/R-08.md → 元数据{版本、日期、方法码}、原始数据[5+次]、统计{median, MAD}、映射 resources.json:income_per_min_by_sectors、复测{误差%}
```

## 不做什么
- 不录制视频、不组织测试局（人类 H-060、H-066）；不下载/提取/分析参考作品的任何素材文件。
- 不做 M3 批测量（M3-052）；不做 M1 批（M1-071 已做）。
- 不改玩法代码或 docs 章节；只改 `/data` 与 `docs/measurements/`。
- 不上传录像到任何第三方；不提交 `raw/`。

## 验收标准
1. 批 M2 清单（上文全部 ID）在 `summary.csv` 中 100% 有记录：状态为 `已映射/已复测/locked` 或明确 `待测（缺录像：原因）`；脚本 `tools/ref-analysis/check_batch.py --batch M2` 输出覆盖表（缺一项即列出）。
2. 每个已测项：重复次数 ≥ 5，记录含中位数、离散度、版本/日期、置信度；缺字段的记录 0 份（脚本校验模板）。
3. 复测误差：已映射项的我方复测误差在 §7.3 容限内的比例 = 100%（超容限项必须已迭代调参并再测；记录前后值）。
4. `/data` 回填：映射到的字段 `source=ref_measurement` 且引用测量 ID（`data-validate` 引用完整性通过，0 个悬空 ID）。
5. 合规：仓库扫描无录像/原始帧（`git ls-files` 无 `.mp4/.mkv/.webm` 与 `raw/`）；文档/提交信息 grep 参考作品名称 0 处。
6. 缺录像时：任务 `blocked`，STATUS 顶部 blocked 清单有"所缺条目 — 日期 — 谁来解除（H-060/H-027/H-066）"，且 NEEDS_HUMAN 有追加条目。

## 测试要求
- 单元测试：分析脚本（已知合成视频/音频，如固定速度进度条）测量误差 ≤ 容限；`check_batch.py`。
- 集成/无头测试：我方复测使用 headless 或 Playwright 脚本（例：资源产出：运行服务器 5 min 读取资源曲线回归斜率）。
- 性能基准：无。
- 回归：改 `/data` 后 `cargo test --workspace`、`balance-sim` 小规模冒烟（20 局）无异常。

## 性能预算
- 无（数据与分析任务）；`data-validate` 作业 < 2 min。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`、`data-validate`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人类依赖：H-060（录制批 M2）、H-027（第二账号/协助者）、H-066（社区渠道与 M2 测试员，占点多人规律）、H-028（私有存储）、H-011（合法获取参考作品并确认 EULA）、H-034（分析人员与抽样复核，建议每批 ≥ 10% 置信度 A 项抽查）。
- 依赖任务 M2-009/M2-010/M2-014 提供资源/建造/爆炸物的我方实现，才能同法复测对应项（R-08/R-09、B-*、G-*）。
- 参考作品版本漂移（H-029）：记录版本号，必要时抽样复测（§7.6）。
- 对局类测量（R-06/R-07）需长局录像或公开视频，成本高；允许以公开视频为源，须记录视频来源类型而不提交。
