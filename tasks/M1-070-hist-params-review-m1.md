# M1-070 史实参数复核：M1 武器与 Sherman（R8）

## 元数据
- 里程碑：M1
- 工作流：data
- 规模：M
- 前置依赖：M1-008, M1-009
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：REFERENCE_MEASUREMENTS §6; GAMEDESIGN §0.4; GAMEDESIGN §6.2; GAMEDESIGN §14.3

## 背景
总控裁定 R8：GAMEDESIGN 中的史实参数出自记忆、未联网逐项复核，须按 REFERENCE_MEASUREMENTS §6 的来源逐项复核 `/data` 中的武器与载具参数；**复核前相关数值状态一律为 `initial`**（GD §0.4）。M0-031 已复核 Kar98k 与 M1 Garand；本任务复核 M1 其余武器（M1-008 产出的全部武器 JSON：M1 Carbine、Thompson M1A1、M1911A1、Mk2 手雷、M18 烟雾、MP 40、StG 44、P38、M24 Stielhandgranate、Nb.Hgr.39 烟雾、Panzerfaust 60 等，**以 `/data/weapons/` 当前文件为准**）与 Sherman（`us_m4a1_75`）的 `historical` 字段。取舍规则（REF §6.1）：外形/口径/弹容量/全重/枪管长以史实为准；理论射速、初速、有效射程以史实为第一来源，允许有限度偏离但必须写 `deviation_reason`；手感类时序与游戏性数值以对标测量为准（M1-071，不属本任务）；载具最高速/战斗全重/火炮口径/弹种以史实为初值，加速曲线/转向速率/装填时间以对标为准。游戏最高速 = 史实路速 × 0.75（GD §14.3）。来源（REF §6.2，核实日期 2026-11-17）：美军 TM 9-1275（M1 Garand）、FM 23-5、TM 9-1005-222-12、TM 9-1205（M1919）、TM 9-2200、HyperWar TM 索引、TM-E 30-451（德军手册，可能不完整）；德军条令（HDv/Merkblatt）**未核实到可靠链接**；Hogg 等出版物需馆藏；博物馆馆藏（Royal Armouries、Smithsonian、IWM）未检索到具体条目。REF §6.2 注：来源页内数字**未下载全文核对**，引用具体数值前必须打开原文核对页码并把页码写入记录。

## 具体内容
### 要创建/修改的文件
- `data/weapons/*.json`（M1-008 产物）—— 逐字段填 `historical._source`（来源 ID + 页码/章节）、`historical.deviation_reason`（如游戏性偏离）、状态 `initial → reviewed`；无法核实的字段写 `"_source": "未核实"`，状态保持 `initial`。
- `data/vehicles/us_m4a1_75.json`（M1-009 产物）—— 同上：乘员、质量、主炮口径/弹种/弹药、装甲厚度/倾角、路速；穿深为"游戏标称值"（GD §0.4）需标注。
- `docs/measurements/hist-review-m1.md`（新增，记录文件，不含受版权保护的大段摘抄，只记录数值事实、来源 ID、页码）—— 字段级对照表：字段 | 原值(initial) | 来源 | 页码 | 复核结果(一致/修订/未核实) | 备注。
- `/data/schemas`（如需）—— 增补 `deviation_reason`、`source_refs[]` 字段（REF §6.3 建议字段：`caliber`、`cartridge`、`mag_capacity`、`cyclic_rpm`、`muzzle_velocity_mps`、`mass_kg`、`length_mm`、`source_refs[]`）。
- `docs/PLAN_ISSUES.md`（追加）—— 与 GAMEDESIGN §6.2/§14.3 表冲突的项（逐条）。
### 要实现的行为
1. 逐武器逐字段对照 GD §6.2 与来源；记录页码；一致→`reviewed`，不一致→按 REF §6.1 规则取值并写 `deviation_reason`，同时在 PLAN_ISSUES 写冲突条目（不改 GAMEDESIGN 文档本身，由 Fable 回写）。
2. Sherman：核对 GD §14.3 表（乘员 5、游戏座位 3、75 mm M3 AP 30/HE 24/WP 6 弹药数、装填 3.0 s、同轴 M1919 1000、车体 M1919 400、车顶 M2HB 200、路速 38 km/h、质量 30.3 t）与装甲表（车体前上 51/56°、前下 51/45°、侧 38/0、后 38/0、顶 19/90、炮塔前 76/30、侧 51/25、后 51/25、顶 25/90）；弹药数与装填时间属游戏性，不要求史实来源，标 `gameplay` 来源。
3. 不得编造：不可联网核实的来源（德军条令、Hogg）写"未核实"，**不用记忆填充数值并标 reviewed**。
4. 版权纪律：只记录数值事实，不复制手册原文段落；不把转录文本放入仓库（NEEDS_HUMAN H-031 默认）。
5. `data-validate` 通过；`DATA_HASH` 变化需随提交说明（数据变更 = 版本变更，ARCHITECTURE §18.5）。
### 接口与数据
```json
"historical": { "cartridge": ".45 ACP", "mag_capacity": 30, "cyclic_rpm": 700, "muzzle_velocity_mps": 280, "mass_kg": 4.8, "length_mm": 813,
  "source_refs": [{"id": "TM9-2200", "page": "待核对填写"}], "_source": "…", "deviation_reason": null, "review_status": "reviewed" }
```
（数值仅为结构示例，不作为复核结果。）

## 不做什么
- 不做对标测量回填（M1-071）；不改游戏性字段（伤害/散布/冷却等）。
- 不复核 M2/M3 武器与载具（M2/M3 的 `hist-params-review-*`）。
- 不改 GAMEDESIGN/ARCHITECTURE 文档结论（写 PLAN_ISSUES，Fable 回写）。
- 不做 Kar98k/Garand（M0-031 已做）；不做穿深与装甲的"实验数据"复核（只标注"游戏标称值"）。

## 验收标准
1. M1 范围内每把武器的 `historical.*` 字段 100% 有出处（来源 ID + 页码）或明确标"未核实"（脚本 `tools/data-validate --check-sources m1` 统计缺口数 = 0；INDEX 指标）。
2. Sherman 的 `historical`、装甲板厚/倾角、主炮口径/弹种字段 100% 有出处或"未核实"（同脚本）。
3. 与 GD §6.2/§14.3 数值不一致的项逐条写入 PLAN_ISSUES（条目数与 `hist-review-m1.md` 中"修订"行数一致，脚本核对）。
4. 状态流转正确：所有已核实字段的武器 `review_status: reviewed`；含"未核实"字段的武器保持 `initial`（脚本断言，无"带未核实字段却 reviewed"）。
5. `cargo test -p sim data_` 与 `data-validate` 通过；回填后 `DATA_HASH` 变化已在 STATUS 记录新旧值。
6. 记录文件中无大段手册原文（人工抽查 + 脚本检测单段 > 200 字符引用，数量 = 0）。
7. 无法联网访问来源时：该条目标 `未核实`，任务 `partial`，PLAN_ISSUES 写明缺口。

## 测试要求
- 单元测试：`data-validate` 的 source 检查规则（缺 `_source`/缺页码/reviewed 与未核实矛盾均报错）。
- 集成/无头测试：依赖 `/data` 的 sim 测试（致死发数、射速）在数值修订后重新通过；若有字段修订则更新对应断言来源（不得把参考数字硬编码进测试）。
- 性能基准：无。
- 回归：`determinism` 作业（DATA_HASH 变化后重跑）。

## 性能预算
- 无（数据任务）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`、`data-validate`）；`cargo clippy -D warnings` 干净
- [ ] 性能预算：无
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：冲突项写入 PLAN_ISSUES（不改 GAMEDESIGN 章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 德军条令与出版物来源在线不可得；多数德军武器字段可能留 `未核实`（NEEDS_HUMAN H-031 来源合法性、H-062 史实抽样复核）。
- 美军 TM 的转录站点可能带编辑错误（REF §6.2 注）；至少用两个来源交叉核对关键数字（口径、初速、射速、质量）。
- 穿深为游戏标称值，不得标为"史实复核通过"。
