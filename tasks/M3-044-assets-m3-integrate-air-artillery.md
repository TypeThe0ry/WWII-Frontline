# M3-044 素材接入：飞机与火炮模型

## 元数据
- 里程碑：M3
- 工作流：assets
- 规模：M
- 前置依赖：M2-047, M3-030, M3-031
- 素材需求：V-VEH-ART-*; V-VEH-AIR-*; V-BLD-009; V-SKY-003；素材状态：real（仅限 OK 项），其余 placeholder
- 引用文档：ASSET_SHOPPING_LIST §4.4.1; ASSET_SHOPPING_LIST §4.4.5; ASSET_GAPS §3.2; ASSET_GAPS §3.5; GAMEDESIGN §15.1; GAMEDESIGN §15.7; GAMEDESIGN §13.6; ARCHITECTURE §11.4

## 背景
M3-030/M3-031 用灰盒完成了飞机过场与炮击表现；火炮阵地与 AT 炮的灰盒在 M3-018/019 的渲染挂接中使用（炮架长方体 + 圆柱炮管 + 盾板盒）。本任务接入**已 `OK`** 的素材：
- V-VEH-ART-001–004：US M2A1 105 mm 榴弹炮、GER leFH 18 10.5 cm、US 57 mm M1 AT 炮、GER 7.5 cm Pak 40（GD §15.1/§15.7）。GAP-V11：若 M2A1 无合适模型，使用默认替代方案（H-069 决定）。
- V-VEH-AIR-001–004：空袭飞机（US：P-47 类；GER：Ju 87 类；运输机 C-47/Ju 52 类用于空投，见 GD §13.3/§13.6）。
- V-BLD-009：火车站建筑（Carentan 地标，MILESTONES §4.5）；V-SKY-003：云层。
规则同 M3-041：D-MOD 允许的加工；仅 `OK` 素材；`.fpak`；Medium ≤ 150 MB。
关键 socket：
- 火炮：`socket_barrel_pivot`（仰角枢轴）、`socket_muzzle`、`socket_shield`（AT 炮盾，7 mm/0°，正面 ±60° 有效）、`socket_trail`（尾架，供 Engineer 重定向）；方位转动部件与底座可分离。炮管仰角范围：榴弹炮 5°–85°；AT 炮 −5°/+15°（US）、−5°/+25°（GER）。
- 飞机：`socket_bomb_01..n`（投弹点）、`socket_gun_l/r`（扫射）、`socket_prop`（螺旋桨旋转）。
- 命名与 M3-029 约定一致，写入 `/data`（细化时登记 PLAN_ISSUES）。

## 具体内容
### 要创建/修改的文件
- `tools/asset-pipeline/recipes/artillery/{需求ID}.json`、`recipes/aircraft/{需求ID}.json` —— LOD、socket 映射、材质合并（≤ 2 套/件）。
- `tools/asset-pipeline/recipes/buildings/V-BLD-009.json` —— 火车站：保留可通行门洞/楼层（ASSET_GAPS §3.2 建筑规则），碰撞盒另由 map 数据提供；室内灯/窗保持。
- `tools/asset-pipeline/recipes/sky/V-SKY-003.json` —— 云层贴图/网格压缩（KTX2）。
- `tools/asset-pipeline/src/artillery_checks.ts`、`aircraft_checks.ts` —— socket/尺寸/枢轴校验。
- `client/src/render/artillery/ArtilleryFactory.ts`、`client/src/render/air/AircraftFactory.ts`（改）—— 真模型覆盖占位。
- `assets/manifest/air_artillery_m3.json`。
### 要实现的行为
1. 读取 LICENSES.md 状态；仅处理 `OK`；其余保持灰盒（飞机十字形盒、火炮盒体）。
2. 校验：炮管仰角枢轴位置与 `/data` 一致（容差 0.05 m）；AT 炮盾板位置与尺寸与 sim 防盾碰撞体一致（容差 0.05 m）；飞机翼展/机长与史实偏差 ≤ 5%。
3. 火车站：门洞与楼层连通性与 `/data/maps/carentan` 的碰撞/导航数据一致（不改变可玩性；若不一致写 PLAN_ISSUES，由 maps 流处理）。
4. 云层：仅 High/Ultra 启用，Medium/Low 关闭（ARCH §13.1 画质分级，具体由渲染设置控制）。
5. 输出缺失素材报告，写入 STATUS。
### 接口与数据
- manifest 字段按 ARCH §11.3；命名遵守 ARCH §11.2。

## 不做什么
- 地面载具（M3-041）；武器/角色（M3-042）；音频（M3-043）。
- 为火炮制作不存在的第一人称视角模型（炮手使用光学瞄准界面，M3-035）。
- 建筑破坏阶段模型（`ruin_stage` 的素材归 M2/M4 建筑任务；若 V-BLD-009 无废墟版，保持单阶段并写 PLAN_ISSUES）。

## 验收标准
1. 每个 `OK` 的需求 ID：manifest `placeholder=false`；`pnpm -C tools/asset-pipeline report --milestone M3` 输出这 10 个 ID（ART ×4、AIR ×4、BLD-009、SKY-003）的状态计数。
2. socket 完整性 100%（`artillery_checks`/`aircraft_checks` 0 失败）。
3. 防盾/炮管尺寸与 `/data` 偏差 ≤ 0.05 m；飞机尺寸偏差 ≤ 5%。
4. Medium 包 ≤ 150 MB，`manifest-budget` 通过。
5. Playwright：每个已接入的火炮/飞机各 1 张固定机位截图无穿模/缺材质（人工审阅记录；无则 `未测`）。
6. 管线对非 `OK` 素材拒绝处理；`git ls-files assets/incoming assets/processed` 为空。

## 测试要求
- 单元：recipe 解析、socket 校验。
- 集成：1 门火炮 + 1 架飞机走完整管线 → 客户端 → 截图；空袭过场脚本（M3-030）中飞机为真模型时到达时刻误差仍 ≤ 200 ms。
- 性能：飞机 3 架同屏 + 火炮 2 门的 draw call 增量写入 STATUS（沿用 M3-049 场景）。
- 回归：M3-048 视觉基线更新（人工审阅）。

## 性能预算
- ARCH §11.4 体积；§13.5.3 draw call/三角形；不得使 M3-049 空袭/炮击场景回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 真正依赖素材：全部非 OK 则 `blocked`（H-070、H-050、H-069）；部分则 `partial`。
- 飞机的"德军标识"仅用 Balkenkreuz（H-024：默认不显示纳粹符号）。
- INDEX 将本任务引用节写作 SHOP §4.4.5；V-BLD-009/V-SKY-003 在 SHOP §4.4.1 地图环境小节，已一并引用。
