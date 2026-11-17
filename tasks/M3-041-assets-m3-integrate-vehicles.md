# M3-041 素材接入：M3 载具

## 元数据
- 里程碑：M3
- 工作流：assets
- 规模：M
- 前置依赖：M2-047, M3-029
- 素材需求：V-VEH-US-001/002/004/005/006; V-VEH-GE-001/002/004/005/006/007；素材状态：real（仅限 LICENSES.md 状态为 OK 的 ID），其余 placeholder
- 引用文档：ASSET_SHOPPING_LIST §4.4.5; ASSET_GAPS §3.5; ASSET_GAPS §2.1; ARCHITECTURE §11.1; ARCHITECTURE §11.4; ARCHITECTURE §11.7; ARCHITECTURE §11.8; BRIEF D-MOD（见 tasks/README §5.2）

## 背景
M3-029 已用灰盒渲染全部载具并定义了 socket 约定；M2-047 是 M2 的素材接入任务，已把素材管线（`tools/asset-pipeline`）跑通。本任务把**已购买且在 `LICENSES.md` 状态为 `OK`** 的 M3 载具模型接入：US（M8、M5A1、M4A3(76)W/Jumbo、CCKW、M3 半履带，对应 V-VEH-US-001/002/004/005/006）与 GER（Puma、Luchs、Pz IV、Panther、Tiger、Blitz、251 等，V-VEH-GE-001…007）。
规则：
- 只处理 `LICENSES.md` 状态 `OK` 的素材；`PENDING/REJECTED/未登记` 保持占位且不得进入主分支（README §5.2；管线 ingest-gate 会机械拒绝）。
- 允许的加工（D-MOD）：格式转换、LOD/减面、贴图压缩（KTX2）、重定向、材质参数调整、把已购部件组合装配；**不允许**从零建模、雕刻新部件、重画贴图、AI 生成。
- 处理后只以 `.fpak` 分块容器下发（D-PAK/R11），不提交 `assets/incoming/` 与 `assets/processed/`。
- 内饰：GAP-V01 默认方案是**简化内饰盒**（不做高精度内饰）。
- 包体预算（ARCH §11.4）：Carentan Medium ≤ 150 MB（其中载具约 16 MB / 6 种的估算）；manifest 门禁：超预算 1% 失败，增量 > 20% 需人工批准。
- 若售方模型尺寸/比例与 `/data` 史实尺寸不符，只做等比缩放（≤ ±5%），超出写 PLAN_ISSUES 并保留占位。

## 具体内容
### 要创建/修改的文件
- `tools/asset-pipeline/recipes/vehicles/{需求ID}.json` —— 每个需求 ID 一份处理配方：源文件、LOD 目标三角形（LOD0/1/2）、节点重命名映射到 socket 名（`socket_turret/gun_pivot/muzzle/coax/hull_mg/roof_mg/seat_*`）、炮塔/炮管骨骼或父子关系、贴图集合并（≤ 2 套材质/车）、阵营标识贴花层。
- `tools/asset-pipeline/src/vehicle_checks.ts` —— 校验脚本：socket 完整性、尺寸与 `/data` 对比、LOD 层数、材质数。
- `assets/manifest/vehicles.json`（或沿用 M2-047 的 manifest 路径）—— 每个需求 ID 条目 `placeholder:false`、`license_ref`、`fpak_chunk`、体积。
- `client/src/render/vehicles/VehicleFactory.ts`（改）—— 加载真模型并回退到灰盒（同一需求 ID 覆盖占位，不改调用方）。
- `docs/ASSET_SHOPPING_LIST.md` 的"已购"勾选由人类负责人做；本任务只输出清单，不改文档。
### 要实现的行为
1. 读取 LICENSES.md，列出 12 个目标需求 ID 的状态；仅为 `OK` 者执行配方。
2. 处理流水线：gltf-transform（清理/合并/Meshopt 压缩）→ 减面生成 LOD → toktx（KTX2，颜色 ETC1S/UASTC 按需，法线 UASTC）→ `.fpak` 打包。
3. socket 映射校验：每辆装甲车必须有 `socket_turret`、`socket_gun_pivot`、`socket_muzzle`，可旋转部件的枢轴位置与 `/data` 一致（容差 0.05 m）。
4. 输出缺失素材报告：每个需求 ID `real/placeholder/missing`，并写入 STATUS。
5. 未 OK 的需求 ID 保持灰盒，并在报告中给出缺口原因（对应 GAP-V01/V10 等）。
6. 载具真模型到位后，M3-029 的 draw call/三角形/材质变体验收必须重测。
### 接口与数据
- manifest 字段按 ARCH §11.3 格式；`placeholder` 字段由管线写入；命名按 ARCH §11.2。

## 不做什么
- 飞机、榴弹炮、AT 炮、火车站、云层（M3-044）。
- 武器与角色（M3-042）；音频（M3-043）。
- 自制/补建模型（禁止，A2/D-MOD）；内饰高精度（GAP-V01 默认简化盒）。
- 购买/授权决策（人类：H-070、H-069、H-050）。

## 验收标准
1. 每个已 `OK` 的需求 ID 在 manifest 中 `placeholder=false`；报告给出 12 个目标 ID 的 real/placeholder/missing 计数（命令 `pnpm -C tools/asset-pipeline report --milestone M3`）。
2. Medium 包总体积 ≤ 150 MB，manifest-budget 门禁通过（`manifest-budget` CI 作业）；载具子包体积写入 STATUS。
3. 每辆装甲车 socket 完整性检查 100% 通过（`vehicle_checks` 0 失败）。
4. 尺寸与 `/data` 的长/宽/高偏差 ≤ 5%（脚本输出表）。
5. 12 辆载具同屏场景（M3-029 机位）重测：draw call ≤ 1 200、可见三角形 ≤ 4.0 M（与 M3-049 口径一致；无参考机则写"未测"并保持 partial）。
6. 管线对未 `OK` 的素材返回非零退出码并拒绝处理（对一个故意设为 PENDING 的用例验证）。
7. `assets/incoming/` 与 `assets/processed/` 无任何文件被 git 追踪（`git ls-files assets/incoming assets/processed` 为空）。

## 测试要求
- 单元：配方解析、socket 重命名映射、尺寸校验。
- 集成：对 1 个样例模型走完整管线（ingest-gate → .fpak → 客户端加载 → Playwright 截图）。
- 性能：加载时间（单车 `.fpak` 解码 ≤ 200 ms，Playwright 记录）。
- 回归：M3-048 视觉基线需在真模型接入后更新（人工审阅）。

## 性能预算
- ARCH §11.4（Medium ≤ 150 MB）；§13.5.3 draw call/三角形；载具 LOD0 三角形不设单车硬上限，但每车 LOD 表写入 manifest 并在 M3-049 汇总。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 本任务真正依赖素材：全部相关 ID 非 `OK` 时标 `blocked`（所缺条目：H-070 M3 采购批、H-050 卖家书面确认；登记 STATUS 顶部）；部分 OK 时标 `partial`。
- GAP-V01（内饰）、GAP-V10（Puma 等）的最终决定见 H-069；默认替代方案生效时须在 STATUS 写明。
- CGTrader 等"慎用"渠道的模型必须有卖家书面确认（D-LIC）；无确认保持 PENDING。
