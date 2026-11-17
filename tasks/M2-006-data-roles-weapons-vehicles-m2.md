# M2-006 数据：M2 全步兵兵种、武器与基础载具

## 元数据
- 里程碑：M2
- 工作流：data
- 规模：M
- 前置依赖：M1-007, M1-008, M1-009
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §5.7; GAMEDESIGN §5.8; GAMEDESIGN §5.11; GAMEDESIGN §5.12; GAMEDESIGN §5.13; GAMEDESIGN §6.2; GAMEDESIGN §14.3; GAMEDESIGN §4.4; ARCHITECTURE §8.5

## 背景
M1 有 5 个兵种（Officer/Rifleman/Medic/Support/Crewman）与 Sherman。M2 补齐其余步兵兵种 Assault、Automatic Rifleman、Machine Gunner、Anti-Tank、Engineer（共 10 个步兵/车组兵种，MILESTONES §3.2），以及 Panzer IV、Jeep/Kübelwagen、卡车。数值唯一真源是 `/data`（ARCH §4.4）；全部状态 `initial`（R8，史实复核由 M2-053 完成）。ID 约定（GD §0.3）：武器 `us_m1_garand`；loadout `{faction}_{role}_{n}`；载具 `us_m4a1_75`。注意 ARCH §8.5.1 示例用 `m1_garand` 无阵营前缀，**以 GD §0.3 为准**并在 PLAN_ISSUES 记文档不一致。
要点：
- 名额（GD §4.4）：Assault 每小队 2、解锁小队人数 ≥ 2、全队 12；AR 1/≥3/6；MG 1/≥4/6；AT 1/≥3/6；Engineer 1/≥3/6。
- Loadout（GD §5.7–5.13）：us_assault_1（Thompson M1A1 30×6、M1911A1 ×2、Mk2×3 + M18×1）、us_assault_2（M3 Grease Gun）、us_assault_3（M1897 Trench Gun，5 发管式弹仓总 30 发）、ger_assault_1/2/3（MP 40 / StG 44 / MP 40+M39）；us_ar_1/2（BAR 20×8 / 20×10）、ger_ar_1/2/3（StG 44 / FG 42 / StG 44）；us_mg_1/2（M1919A4 250×3 / M1919A6 150×4）、ger_mg_1/2/3（MG 34 / MG 42 50×6 / MG 34 双鼓）；us_at_1/2/3（Bazooka ×4 / ×5 / M7 枪榴弹）、ger_at_1/2/3（Panzerschreck ×3 / Panzerfaust 60 ×2 / ×3）；us_engineer_1/2/3、ger_engineer_1/2/3（含 AP/AT 地雷、Satchel/Geballte Ladung、剪线钳）。解锁等级见 GD 各表。
- 武器史实参数 GD §6.2，游戏性参数 GD §6.3/§6.5/§6.9；载具 GD §14.3（Pz IV H：7.5 cm KwK 40 L/48，AP 28/HE 22/Nebel 6，装填 3.2 s，装甲前上 80/12°，路速 38 km/h ×0.75；Jeep 100 km/h；Kübel 80 km/h；CCKW 72 km/h；Blitz 80 km/h）、成本 GD §14.6（Jeep/Kübel M20/F15、上限 4、冷却 60 s；卡车 M40/F30、上限 3、90 s；中型 M140/U80/F120、240 s、解锁门槛 5 min）。

## 具体内容
### 要创建/修改的文件
- `data/roles/{assault,automatic_rifleman,machine_gunner,anti_tank,engineer}.json` —— 字段按 ARCH §8.5.2：`max_per_squad/max_per_team`、`unlock.squad_size_min`、`movement`、`abilities[]`（`sprint_stamina_mult:0.8`、`vault_time_mult:0.75` 对 Assault；`bipod_recoil_mult:0.35` 对 MG；见 GD §5.1）、`faction_overrides.loadout_ids`
- `data/loadouts/{us,ger}_{role}_{n}.json` —— 上述全部 loadout（含解锁等级 `unlock_condition`）
- `data/weapons/` —— 新增：`us_bar_m1918a2`、`us_m1919a4`、`us_m1919a6`、`us_m1897`、`us_bazooka_m1a1`、`ger_mg34`、`ger_mg42`、`ger_stg44`、`ger_fg42`、`ger_panzerschreck`、`us_m3_grease`、`us_m1_garand`（若 M1 未含）等；投掷/爆炸物与地雷 `us_satchel`、`ger_geballte_ladung`、`us_ap_mine_m2a4`、`ger_s_mine_35`、`us_at_mine_m1a1`、`ger_tellermine_43`、`us_m7_grenade_launcher`
- `data/vehicles/{ger_pz4h,us_jeep,us_jeep_mg,ger_kubel,ger_kubel_mg,us_cckw_transport,us_cckw_supply,ger_blitz_transport,ger_blitz_supply}.json` —— 座位、装甲区（GD §14.3.3）、模块 HP（GD §14.5.1）、`spawn_cost`、`respawn_cooldown_s`、`cap`
- `data/schema/*.schema.json` 增补 `weapon.class` 新取值：`lmg`、`shotgun`、`launcher`、`mine`、`explosive`
### 要实现的行为
1. 全部数值取自 GD 表；游戏性字段在 `_meta.tunable` 登记；史实字段带 `historical._source`（写"待 M2-053 复核"或具体出处），状态 `status:"initial"`。
2. 单位后缀规范（`_m`、`_mps`、`_rpm`、`_kg`、`_s`、`_ms`）；载具速度用 `max_speed_kph`（GD §0.2 载具为 km/h）。
3. 引用完整性：loadout 的 `primary/secondary/gadget` 均指向存在的武器；role 的 loadout 均存在；名额与 GD §4.4 逐字段一致。
4. `data_hash` 随内容变化（ARCH §4.4）；不要在客户端代码里放数值。
### 接口与数据
- 沿用 M1-007/008/009 的 schema；新增字段只在末尾追加并保持向后兼容；MG `deploy:{time_s:0.8,recoil_mult:0.35,spread_mult:0.35}`；BAR `fire_modes_rpm:[350,550]`。

## 不做什么
- 兵种能力与武器行为的实现 → M2-012/M2-013/M2-014/M2-015（本任务只写数据）。
- AT 炮（57 mm M1 / Pak 40）数据 → M2-011（见其备注）；Tank Commander/Spotter/Sniper/Commander 兵种与其余载具 → M3。
- 史实复核 → M2-053；对标测量回填 → M2-054。

## 验收标准
1. `cargo run -p data-validate`（或 M1 建立的校验命令）通过：5 个新兵种 + 全部 loadout + 武器 + 载具 JSON 0 错误。
2. 引用完整性：脚本扫描 loadout→weapon、role→loadout、vehicle→weapon 的悬空引用数 = 0。
3. 名额字段与 GD §4.4 表逐项一致（测试读取 GD 表的内联期望值 `tests/data_roles_caps.rs`）。
4. 武器射速字段与 GD §6.2/§6.3：BAR 350/550 rpm、MG42 1200 rpm、MG34 850 rpm、M1919A4 500 rpm 等逐项断言。
5. Pz IV H：装甲 80/12°（前上）、50/12°（前下）、AP 28 / HE 22 / Nebel 6、装填 3.2 s 全部断言。
6. 全部新增 JSON 均有 `status:"initial"`，`_meta.tunable` 非空。

## 测试要求
- 单元测试：`cargo test -p sim data_roles_caps data_weapons_refs data_vehicles_schema`。
- 集成：`sim` 的 `build.rs` 嵌入新数据后 `DATA_HASH` 变化且稳定（两次构建相同）。
- 性能基准：无。
- 回归：更新 `data_hash` 快照测试。

## 性能预算
无（数据任务）；`GameData` 嵌入体积对 `sim-wasm` ≤ 1.2 MB 的预算（ARCH §4.3.3）需检查不回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- GD 史实参数出自记忆未联网复核（R8）；任何与公开史料冲突的数值先照 GD 写入并保持 `initial`。
- MG 持续射击的过热/换弹链参数 GD 没有给出数字：只写 GD 已有的换弹秒数（M1919A4 6.5 s、MG 34 5.5/6.0 s、MG 42 6.0/7.0 s），首发**无过热机制**（总控裁定，GD 已写明）：数据中保留 `overheat` 字段、默认 `enabled=false`。
