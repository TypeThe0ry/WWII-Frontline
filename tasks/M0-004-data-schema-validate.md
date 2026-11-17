# M0-004 /data schema 与校验工具

## 元数据
- 里程碑：M0
- 工作流：data
- 规模：M
- 依赖：M0-001
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §8.5; ARCHITECTURE §4.4; ARCHITECTURE §17.5; GAMEDESIGN §0.2; GAMEDESIGN §0.3; GAMEDESIGN §6.1

## 背景
`/data` 是全部游戏数值的唯一真源（ARCHITECTURE §4.4；GAMEDESIGN §0.2）。`sim` 在编译期用 `build.rs` 嵌入 `/data` 并计算 `DATA_HASH`（SHA-256 前 8 字节），握手时 `Hello.data_hash` 必须与服务器一致，否则拒绝（ARCHITECTURE §8.2）；CI 有 `data-validate` 作业（<2 min，§17.5）。本任务建立 schema、校验工具、嵌入与哈希机制，使 M0-008（武器）、M0-011（角色）、M0-013（规则）等数据任务有地方落地、有人把关。
通则（ARCHITECTURE §8.5）：JSON；`id` 为稳定 snake_case slug；名称字段为 i18n key（`name_key`）；**所有数值带单位后缀**（`_m`、`_ms`、`_mps`、`_rpm`、`_kg`、`_deg`、`_hp`…）；`【待调参】` 字段登记在 `_meta.tunable`；历史参数（`historical`）与游戏性参数（`gameplay`）分块；每个文件带 `_meta.status ∈ {initial, reviewed, measured}`（R8：复核前一律 `initial`）。ID 约定取 GAMEDESIGN §0.3（`us_m1_garand`、`ger_kar98k`、loadout `us_rifleman_1`）——ARCHITECTURE §8.5.1 示例写 `m1_garand`，二者冲突时以 GAMEDESIGN §0.3 为准并在 PLAN_ISSUES 记一条"文档冲突"。

## 具体内容
### 要创建/修改的文件
- `data/schemas/{weapon,role,loadout,vehicle,map_meta,spawns}.schema.json` —— JSON Schema 2020-12。weapon 必含（M0 范围，后续任务只追加可选字段）：`id, name_key, faction, class, historical{caliber,cartridge,magazine_capacity,magazine_type,muzzle_velocity_mps,rate_of_fire_rpm_practical,mass_kg,_source}`、`gameplay{damage_torso_hp, part_mult{head,neck,torso,limb}, falloff[{dist_m,mult}], fire{mode, cap_rpm | bolt_cycle_ms}, reload{tactical_ms,empty_ms,per_round_ms?}, ads_ms, spread{hip_deg,ads_deg}, recoil{vkick_deg,hsigma_deg,recover_deg_s,bloom_per_shot_deg,bloom_decay_deg_s}, ballistics{muzzle_velocity_mps,drag_k_per_m,gravity_mps2}, pen_power, mag_reserve}`、`_meta{status,tunable[],calibration_ref}`。role 按 ARCHITECTURE §8.5.2（`movement{walk_mps,run_mps,sprint_mps,crouch_mps,prone_mps,stamina_max}`、`health`、`faction_overrides.*.loadout_ids`），并增加可选 `capsule{radius_m,height_stand_m,height_crouch_m}`（M0-011 使用）。loadout 按 §8.5.3。vehicle 按 §8.5.4（M0 无实例，只要 schema 可校验示例）。map_meta/spawns 对应 MAPS §6.2 的 `frontline.map.meta/1`、`frontline.map.spawns/1`。
- `data/schemas/fixtures/valid/*.json`、`data/schemas/fixtures/invalid/*.json`（每个 invalid 文件在 `_expect_error` 字段写期望的错误子串，如 `"missing: historical._source"`）。
- `tools/data-validate/`（Node，`ajv` 2020 + 自写规则）：`index.mjs`、`rules/{units,tunable,refs,ranges}.mjs`、`test/*.test.mjs`。
- `crates/sim/build.rs`、`crates/sim/src/data.rs`（`GameData{weapons,roles,loadouts,vehicles,hash}`，ARCHITECTURE §8.1；`pub const DATA_HASH: u64`）。
- `.github/workflows/data-validate.yml`（接入 M0-002 的占位作业，触发路径 `data/**`、`tools/data-validate/**`）。
### 要实现的行为
1. **单位规则**：键名以 `_m/_ms/_mps/_rpm/_kg/_deg/_hp/_s/_deg_s/_per_m` 等白名单后缀结尾才算带单位；对数值字段无单位后缀、或后缀与 schema 声明不符（例如 `muzzle_velocity_kph`）报错。
2. **引用完整性**：loadout 的 `primary/secondary/gadget` 必须指向已存在的 weapon id；role 的 `loadout_ids` 必须存在。
3. **tunable 规则**：`_meta.tunable` 中列出的路径必须真实存在；`status` 缺失报错；`historical` 内每个数值必须有 `_source` 或 `"未核实"` 标记（规则在 M0-031 才对 `status=reviewed` 强制）。
4. **范围规则**：如 `cap_rpm ∈ [1,2000]`、`falloff` 距离严格递增且 `mult ∈ (0,1]`、`muzzle_velocity_mps ∈ [50,1500]`。
5. **DATA_HASH**：`build.rs` 遍历 `/data` 下除 `schemas/` 外全部 `.json`，按相对路径排序，`serde_json` 解析后以**规范化形式**（键排序、无空白）重新序列化，拼接后取 SHA-256 前 8 字节小端为 `u64`；纯格式化（缩进/换行）改动不改变哈希，任何值改动必变。`build.rs` 同时用 `serde(deny_unknown_fields)` 的类型化结构做构建期解析，解析失败即编译失败；完整 JSON Schema 校验由 Node 工具负责（二者分工写入 `data/README.md`）。
### 接口与数据
- CLI：`node tools/data-validate/index.mjs [--root data] [--json]`，错误一行一条 `path:jsonpointer: message`，有错 `exit 1`。
- 数值字段命名以本任务的 schema 为准；M0-008/011/013 若需新字段，**先改 schema**（可选字段追加），并在 PLAN_ISSUES 记"细化"。

## 不做什么
- 不写任何具体武器/角色数据（M0-008、M0-011、M0-013）；不写 `structures/abilities` schema（M1/M3）。
- 不做 `status` 的史实复核（M0-031）；不做对标测量字段 `ref_*`（M0-032 追加可选字段）。
- 不在 `sim` 内使用 `data` 做任何游戏逻辑（M0-007 起）。

## 验收标准
1. `data/schemas/fixtures/invalid/` 至少 10 个样例（缺字段、错单位、范围越界、引用不存在、falloff 非递增、tunable 路径不存在…），校验工具对**每个**样例退出码 1 且错误含 `_expect_error` 子串；`valid/` 样例全部通过。
2. 对 `data/schemas/fixtures/valid/*` 之外的真实 `/data`，工具退出码 0（M0 此时可只含 fixtures 与 README）。
3. 单元测试 `cargo test -p sim data_hash_stable`：同内容两次构建 `DATA_HASH` 相等；改动缩进后相等；改动一个数值后不等。
4. `cargo build -p sim` 对一个缺必填字段的 `/data` 文件编译失败（用临时文件演示，不入库）。
5. `data-validate` CI 作业墙钟 < 2 min（ARCHITECTURE §17.5），且仅在 `data/**` 变更时触发。

## 测试要求
- 单元测试：Node 工具的每条规则至少 1 正 1 反；`cargo test -p sim data_*`。
- 集成/无头测试：CI 中故意提交坏 JSON 的 PR 被 `data-validate` 拒绝一次（记录运行链接或日志）。
- 性能基准：无。
- 回归：`DATA_HASH` 的当前值写入 STATUS（后续任务改 `/data` 时哈希会变，属预期）。

## 性能预算
`data-validate` 作业 < 2 min（ARCHITECTURE §17.5）；`build.rs` 对 /data 的处理增加 `cargo build -p sim` 墙钟 < 3 s。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `sim` 的 `build.rs` 引入 `serde`/`serde_json`/`sha2` 作为 build-dependency 与 dependency：属于 ARCHITECTURE §4.3.2 允许的"serde（仅数据解析）"；`sha2` 若不在 INDEX 列出，需在 PLAN_ISSUES 登记一条依赖说明。
- ARCHITECTURE §8.5.1 与 GAMEDESIGN §6.1/§6.3 的字段名并不一致（`drag_k` vs `k_drag`；`damage_head` 绝对值 vs `part_mult`）。本任务统一为 `_per_m`/`part_mult` 写法（GAMEDESIGN §6.4/§6.6 的口径），并把对照表写进 `data/README.md`。
- `DATA_HASH` 输入范围含 `data/maps/**`：地图数据变更会使握手哈希变化，这是 ARCHITECTURE §18.5 "数据变更 = 版本变更"的预期行为。
