# M3-042 素材接入：M3 武器与角色

## 元数据
- 里程碑：M3
- 工作流：assets
- 规模：M
- 前置依赖：M2-047, M3-011
- 素材需求：V-WPN-US-*; V-WPN-GE-*; V-CHR-GE-002; V-CHR-GE-004; V-ANM-010；素材状态：real（仅限 OK 项），其余 placeholder
- 引用文档：ASSET_SHOPPING_LIST §4.4.2; ASSET_SHOPPING_LIST §4.4.3; ASSET_SHOPPING_LIST §4.4.4; ASSET_GAPS §3.5; ARCHITECTURE §11.1; ARCHITECTURE §11.2; ARCHITECTURE §11.4; MILESTONES §4.5

## 背景
M3 补齐剩余武器（M3 范围内：US V-WPN-US-004/006/009；GER V-WPN-GE-002/003/006/009/012–014，见 MILESTONES §4.5）、伞兵/坦克兵角色（V-CHR-GE-002/004）与指挥官动画（V-ANM-010）。M2-047 已跑通 M2 的接入；M3-011 完成了狙击步枪 sim。本任务把**已 `OK`** 的素材接入，未 OK 保持灰盒占位。
关键约束：
- D-FP：第一人称武器 25k–60k 三角形【待调参】，第三人称 5k–12k；每把武器两套精度（占位同形，ASSET_GAPS §3.2）。
- socket：武器 `socket_muzzle`、`socket_ejection`、`socket_magazine`；角色手部握持点；挂点命名按 `/data` 与 M3-029 约定，不得自创冲突命名。
- D-MOD：允许格式转换、LOD、贴图压缩、重定向动画、换骨架绑定权重传递、材质调整、已购部件组合装配；不允许从零建模/雕刻/重画/AI 生成。
- 只用 `LICENSES.md` 状态 `OK` 的素材；`.fpak` 打包；Medium ≤ 150 MB（ARCH §11.4：角色/武器 US+GER ≈ 18 MB 的估算）。
- GAP-V05（FJR/伞兵替代）按 H-069 的决定；德军符号默认不显示卐字/SS 符文（H-024）。

## 具体内容
### 要创建/修改的文件
- `tools/asset-pipeline/recipes/weapons/{需求ID}.json` —— 第一/第三人称两套 LOD 目标、挂点映射、材质合并（每把 ≤ 2 套材质）。
- `tools/asset-pipeline/recipes/characters/{需求ID}.json` —— 伞兵/坦克兵：减面（LOD0 ≤ 22k 三角形，沿用 M1 角色标准；若 M1 配方不同以其为准）、骨架重定向到统一骨架（`rig_humanoid_v1`，沿用 M1）、服装部件组合。
- `tools/asset-pipeline/recipes/animations/V-ANM-010.json` —— 指挥官动画（望远镜、挥手、指示地图等）重定向与剪辑切分。
- `tools/asset-pipeline/src/weapon_checks.ts`、`character_checks.ts` —— 校验 socket、骨骼、动画剪辑命名。
- `assets/manifest/weapons_chars_m3.json`（沿用 M2-047 的 manifest 方式）。
- `client/src/game/weaponFactory.ts`、`characterFactory.ts`（改）—— 真素材覆盖占位（同一需求 ID，不改调用方）。
### 要实现的行为
1. 列出目标需求 ID 与 `LICENSES.md` 状态；`OK` 者执行配方，未 OK 者保持灰盒。
2. 武器：生成 FP/TP 两套；校验 `socket_muzzle` 与 `/data` 枪口偏移一致（容差 0.02 m）；弹匣/供弹挂点齐全（MG34 鼓/弹链、FG42 侧置弹匣、G43 弹匣）。
3. 角色：伞兵（GER，V-CHR-GE-002）与坦克兵（GER，V-CHR-GE-004）换装挂接 Rifleman/Crewman/Tank Commander loadout 的外观槽位；重定向动画库不得出现漂浮/穿模（Playwright 抽检 10 帧）。
4. 指挥官动画：V-ANM-010 挂接 Commander 兵种的 idle/看地图/指示动作；动画状态机沿用 M1 的 AnimGraph。
5. 输出缺失素材报告并写入 STATUS；占位保留 `PH_{需求ID}` 命名。
6. 更新 manifest `placeholder=false`；校验体积预算。
### 接口与数据
- manifest 字段按 ARCH §11.3；节点命名遵守 ARCH §11.2。

## 不做什么
- 载具（M3-041）、飞机/火炮（M3-044）、音频（M3-043）。
- 冬季角色（GAP-V04，M4）；自制/改造服装之外的贴图重绘（禁止）。
- 武器数值与 sim 行为（M3-001/M3-011）。

## 验收标准
1. 每个 `OK` 的需求 ID：manifest `placeholder=false`，`pnpm -C tools/asset-pipeline report --milestone M3` 输出 real/placeholder/missing 计数并写入 STATUS。
2. FP 武器 LOD0 三角形在 25k–60k 内、TP 在 5k–12k 内（脚本校验表；越界者列入 STATUS 并写 PLAN_ISSUES）。
3. 枪口 socket 位置与 `/data` 偏差 ≤ 0.02 m（`weapon_checks` 0 失败）。
4. 角色 LOD0 三角形与骨骼数符合 M1 标准；动画剪辑命名与 AnimGraph 引用 100% 解析（0 缺失剪辑）。
5. Medium 包体积 ≤ 150 MB，`manifest-budget` 作业通过。
6. 管线对非 `OK` 素材拒绝（非零退出码）；`git ls-files assets/incoming assets/processed` 为空。
7. Playwright 抽检：每个新武器第一人称与第三人称各 1 张截图无穿模/缺材质（人工审阅记录，状态可为 `未测` → partial）。

## 测试要求
- 单元：配方解析、socket 校验、命名规则。
- 集成：1 把武器 + 1 个角色走完整管线 → 客户端加载 → 截图。
- 性能：单资源 `.fpak` 解码耗时；加载不引起 > 8 ms 卡帧（流式分帧上传，ARCH §13.5.1 流式 1.5 ms）。
- 回归：M3-048 视觉基线在真素材接入后更新（人工审阅）。

## 性能预算
- ARCH §11.4 体积；§13.5.3 动态角色 ≤ 60、骨骼动画 ≤ 60；不得使 §13.5.1 CPU 预算回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 真正依赖素材：全部非 OK 则 `blocked`（H-070 采购、H-050 卖家确认、H-069 GAP 决定）；部分则 `partial`。
- INDEX 中武器前缀写作 `V-WPN-US-*; V-WPN-GE-*`，具体 ID 以 MILESTONES §4.5 与 SHOP §4.4.3 为准。
- 武器 LOD 三角形区间来自 BRIEF D-FP，标【待调参】。
