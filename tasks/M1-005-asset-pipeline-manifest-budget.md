# M1-005 素材管线：manifest 与体积预算门禁

## 元数据
- 里程碑：M1
- 工作流：assets
- 规模：M
- 前置依赖：M1-002, M1-003
- 素材需求：无（用样例）；素材状态：n.a.
- 引用文档：ARCHITECTURE §11.3; ARCHITECTURE §11.4; ARCHITECTURE §17.5; ASSET_GAPS §3.4

## 背景
管线步骤 7（manifest）与 CI 的 `manifest-budget` 作业（ARCHITECTURE §17.5：PR 改素材时触发，目标 < 3 min）。manifest 格式见 ARCHITECTURE §11.3：`schema/build/game_version/data_hash/assets{…}/bundles{…}/budgets{…}`；每个 asset 的 LOD/贴图条目带 `pak`、`entry`、`offset`、`bytes`、`sha256`；**新增 `placeholder: true/false` 字段**（ASSET_GAPS §3.4）。manifest 自身 `Cache-Control: no-cache`（带 ETag），其它资产 immutable；manifest 进 git：`/assets/manifests/`。
预算（ARCHITECTURE §11.4）：`core ≤ 8 MB`（8388608 B，gzip/br 后 JS+WASM+UI）、首张地图 Medium ≤ 150 MB（157286400 B）、High ≤ 350 MB、Ultra ≤ 600 MB。CI 规则：**任何 PR 使 bundle 体积超预算 1% 以上 → 失败；增量 > 20% 须人工批准**。
真素材只有在 `LICENSES.md` 状态 `OK` 时才能标 `real`（ASSET_GAPS §3.4 第 2 条）。

## 具体内容
### 要创建/修改的文件
- `tools/asset-pipeline/src/manifest.ts` —— 汇总 `work/*/stats.json`、`.fpak` 索引、台账状态 → `assets/manifests/manifest.json`
- `tools/asset-pipeline/src/budget.ts` —— 读 `assets/manifests/budgets.json`（预算源文件，进 git）并对每个 bundle 求和；对比"基线 manifest"（上一次合入主分支的版本）
- `tools/asset-pipeline/src/consistency.ts` —— 校验 manifest 的 `offset/bytes/sha256` 与对应 `.fpak` 索引逐条一致
- `assets/manifests/budgets.json` —— `{core_bytes, map_carentan_medium_bytes, map_carentan_high_bytes, map_carentan_ultra_bytes}`
- `.github/workflows/manifest-budget.yml`（或并入 M0-002 的工作流文件）—— 触发路径 `assets/manifests/**`、`tools/asset-pipeline/**`、`LICENSES.md`
- `tools/asset-pipeline/test/manifest.*.test.ts`、`budget.*.test.ts`
### 要实现的行为
1. manifest 生成：每个逻辑 ID 一项，`kind ∈ model|texture|audio|…`；`tier` 数组（`low|medium|high|ultra`，由 asset-class 决定）；`placeholder` 字段；`license_ref` 指向台账 `asset_id`。
2. `bundles`：支持通配（`ui_*`、`terrain_carentan/*`）；`core` 与 `map_carentan_medium|high|ultra`。区块（M1-035）条目按 `terrain_carentan/b_{bx}_{bz}` 命名，归入对应 bundle。
3. 预算检查：对每个 bundle 计算字节总和（取各条目 `bytes` 之和 + 包索引开销）；`core` 体积由 client 构建产物的 br 压缩大小输入（`--core-size-file client/dist/size-report.json`，M0 的 Vite 构建输出需包含该文件，缺失则 `core` 检查标 `skipped` 并在报告提示）。
4. 门禁判定：`now > budget*1.01` → 退出码 1（失败）；`now - base > base*0.20` 且 PR 未带批准标签 `budget-approved` → 退出码 1；其余通过并输出表格（bundle、字节、预算、占比）。
5. 一致性：manifest 与 fpak 索引任一条目不一致 → 失败（包含条目名与差异）。
6. 占位/真素材：`placeholder:false` 的项必须在台账中 `status=OK`，否则失败。
7. 缺失素材报告（来自 M1-001）合并进 `assets/manifests/missing-report.json`。
### 接口与数据
```ts
interface BundleUsage { bundle:string; bytes:number; budget:number; ratio:number }
function checkBudgets(manifest:Manifest, budgets:Budgets, base?:Manifest): {ok:boolean; usages:BundleUsage[]; failures:string[]}
```
`manifest.json` 字段与 ARCHITECTURE §11.3 示例一致，仅追加 `placeholder`。

## 不做什么
- 上传到 R2/CDN：M1-066/M1-067（ops）。
- 客户端消费 manifest：M1-006。
- 材质变体数门禁（≤ 60，ARCHITECTURE §10.4）：M1-042。
- 真素材的采购/登记：人类。

## 验收标准
1. 构造一个使 `map_carentan_medium` 超预算 2% 的 PR 夹具，`manifest-budget` 退出码非 0 且输出 bundle 名与超出比例（单测 + 夹具）；超 0.9% 的夹具通过。
2. 增量 > 20% 且无 `budget-approved` 标签 → 失败；有标签 → 通过（单测以环境变量模拟标签）。
3. 一致性：手工改动 manifest 中某条目 `offset` +1，`consistency` 失败并定位条目（单测）。
4. `placeholder:false` 但台账为 PENDING 的夹具被拒。
5. manifest 生成对同输入两次运行字节完全一致（排除 `build` 时间戳字段；单测比较其余字段 sha256）。
6. CI 作业 `manifest-budget` 墙钟 < 3 min（Actions 日志；ARCHITECTURE §17.5）。
7. 在仓库当前（仅样例/空）状态下作业通过，并输出 `core` 项 `skipped` 或实际值。

## 测试要求
- 单元测试：通配展开、预算算术（1%/20% 边界各 3 例）、一致性、placeholder 规则。
- 集成：fixtures 全链路 ingest→process→pack→manifest→budget。
- 性能基准：1000 个条目 manifest 生成 + 校验 ≤ 5 s。
- 回归：`assets/manifests/manifest.json` 的 schema 变更须升 `schema` 号并登记 PLAN_ISSUES。

## 性能预算
- 引用 ARCHITECTURE §11.4：core ≤ 8 MB、Medium ≤ 150 MB、High ≤ 350 MB、Ultra ≤ 600 MB；本任务负责"测量并拦截"，不得自行放宽数字。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- "基线 manifest"取自主分支 git 历史；首次无基线时只做绝对预算检查。
- `core` 体积依赖 M0-001 的 Vite 构建与 br 压缩输出，若 M0 未产出 size-report，在 PLAN_ISSUES 登记并先 `skipped`。
- ARCHITECTURE §11.4 的"角色/武器 ≈ 18 MB"需按 D-FP 两套精度重算（§11.8），本任务把重算所需的按类别汇总输出到报告，供 M1-064 使用。
