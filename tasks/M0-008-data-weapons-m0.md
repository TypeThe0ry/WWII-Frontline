# M0-008 数据：M0 步枪（Kar98k、M1 Garand）

## 元数据
- 里程碑：M0
- 工作流：data
- 规模：S
- 依赖：M0-004
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §8.5.1; GAMEDESIGN §6.2; GAMEDESIGN §6.3; GAMEDESIGN §6.4; GAMEDESIGN §6.5; GAMEDESIGN §6.6

## 背景
M0 的射击只涉及两把步枪：美军 M1 Garand（半自动、8 发 en-bloc 弹夹）与德军 Kar98k（栓动、5 发弹夹）。M0-012 的射击/弹道/伤害全部读取这里的数据；客户端不硬编码数值（ARCHITECTURE §4.4）。史实参数（`historical`）出自 GAMEDESIGN §6.2，其可信度见 GAMEDESIGN §0.4：**作者记忆整理、未联网复核**，因此本任务写入的数值状态一律 `initial`，由 M0-031 按 REFERENCE_MEASUREMENTS §6.2 复核；游戏性参数（`gameplay`）来自 GAMEDESIGN §6.3–§6.6 的初值，全部【待调参】，其中射速/致死发数/后坐还【待测】于 REFERENCE_MEASUREMENTS（M0-032 回填）。
ID 约定 GAMEDESIGN §0.3：`us_m1_garand`、`ger_kar98k`（ARCHITECTURE §8.5.1 示例的 `m1_garand` 视为示例）。字段名以 M0-004 的 `weapon.schema.json` 为准。

## 具体内容
### 要创建/修改的文件
- `data/weapons/us_m1_garand.json`
- `data/weapons/ger_kar98k.json`
### 要实现的行为
按 GAMEDESIGN 填写初值（单位后缀见 M0-004；下表是需落地的数字，字段映射由执行者对照 schema 完成）：

| 字段 | us_m1_garand | ger_kar98k | 来源 |
|---|---|---|---|
| class / fire | `rifle_semi`，半自动，游戏射速 cap 120 rpm | `rifle_bolt`，栓动循环 1500 ms | GD §6.3 |
| historical.cartridge | .30-06 Springfield（7.62×63 mm） | 7.92×57 mm Mauser（s.S.） | GD §6.2 |
| 弹容量/供弹 | 8 发 en-bloc 弹夹 | 5 发弹夹（栓动） | GD §6.2 |
| 初速 | 853 m/s | 760 m/s | GD §6.2（约） |
| 实用射速 | 40 rpm | 15 rpm | GD §6.2 |
| 全重 | 4.3 kg | 3.9 kg | GD §6.2 |
| damage_torso | 70 HP | 100 HP | GD §6.3 |
| falloff（起点/终点/保留比例） | 100 m / 600 m / 0.72 | 200 m / 800 m / 0.80 | GD §6.3 |
| 战术/空仓装填 | 3400 ms / 2800 ms | 夹装 3000 ms（空仓同），逐发 650 ms/发 | GD §6.3 |
| 开镜 | 300 ms | 300 ms | GD §6.3 |
| pen_power | 220 | 230 | GD §6.3 |
| drag_k_per_m | 0.00075 | 0.00068 | GD §6.4 |
| 后坐 vkick/hσ/回落 | 1.6° / 0.25° / 12°/s | 3.2° / 0.20° / 10°/s | GD §6.5 |
| 盲射/ADS 散布 | 3.5° / 0.15° | 4.0° / 0.08° | GD §6.5 |
| bloom 每发/衰减 | 0.40° / 3.0°/s | 0 / — | GD §6.5 |
| part_mult | 头 3.0、颈 2.0、躯干 1.0、盆骨 0.9、四肢 0.65 | 同 | GD §6.6 |
| mag_reserve | 6（弹夹数，初值） | 6（初值，与 loadout 设计一致后再调） | ARCH §8.5.1 示例；M1-007 会重新设定 |

1. `falloff` 写成分段数组 `[{dist_m:0,mult:1.0},{dist_m:F0,mult:1.0},{dist_m:F1,mult:min}]`，F1 之后保持 `min`（GD §6.6）。
2. 每个文件的 `_meta.tunable` 列出全部 `gameplay.*` 数值路径；`_meta.status="initial"`；`_meta.calibration_ref` 指向 `REFERENCE_MEASUREMENTS §4.1 W-01…W-37`（逐项对应：射速 W-01/W-02/W-13、装填 W-03/W-04/W-05、开镜 W-07、后坐 W-15–W-18、扩散 W-19–W-21、初速/下坠 W-22–W-25、伤害 W-37）。
3. `historical._source` 写 `"待 M0-031 复核（初值取自 GAMEDESIGN §6.2）"`，不得伪造出处。
4. 增加 `fx` 块：`muzzle_flash_ms: 50`、`tracer: false`（步枪无曳光，GD §6.4；M0-025 的调试曳光走客户端调试开关，不改数据）。
5. 单位全部带后缀；不得出现 GAMEDESIGN 之外的新数值。若 schema 缺字段，先按 M0-004 的流程补可选字段。
### 接口与数据
- 无代码接口；`sim` 通过 `GameData.weapons` 读取（M0-012）。

## 不做什么
- 不创建其它武器（M1-008）；不做 loadout/role（M0-011 建 role，M0-012 建 loadout）。
- 不做史实复核（M0-031）；不做对标测量回填（M0-032）。
- 不在代码里复制这些数值。

## 验收标准
1. `node tools/data-validate/index.mjs` 对 `data/weapons/` 两个文件退出码 0。
2. 两个文件的 `_meta.status` 均为 `initial`；`grep -c "_source" data/weapons/*.json` 每个文件 ≥ 1。
3. 上表每个数字与 GAMEDESIGN §6.2–§6.6 逐项一致（用一个临时脚本或人工逐项勾选，结果写入 STATUS："差异 0 项"）。
4. 衍生检查：用 `falloff` 与 `part_mult` 计算躯干 STK——Garand 在 0/300/600/800 m 均为 2 发；Kar98k 在 ≤ 200 m 为 1 发，> 200 m 为 2 发（脚本输出写入 STATUS，与 GAMEDESIGN §6.6 TTK 基线表一致）。
5. 重复运行 `cargo build -p sim`，`DATA_HASH` 变化且两次构建相同。

## 测试要求
- 单元测试：`cargo test -p sim data_weapons_m0_load`（两把枪可被 `GameData` 解析，字段值等于 JSON）。
- 集成/无头测试：无。
- 性能基准：无。
- 回归：把 STK 衍生表作为数据快照测试，防止后续调参无意破坏 GAMEDESIGN §6.6 的"2 发倒地"原则。

## 性能预算
无。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- ARCHITECTURE §8.5.1 示例 Garand 的 `muzzle_velocity_mps: 850` 与 GAMEDESIGN §6.2 的"约 853 m/s"不同；取 GAMEDESIGN，M0-031 以史料为准最终定值。
- GAMEDESIGN §6.3 对 Garand 弹夹抛出规则（`partial_clip_policy`）属 M1-013 范围，本任务只存战术/空仓装填时间。
- 对标测量（射速、后坐、致死发数）由 H-026（录像）驱动，缺录像时数值保持 `initial`。
