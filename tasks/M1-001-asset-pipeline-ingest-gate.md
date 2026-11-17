# M1-001 素材管线：ingest 与许可证闸门

## 元数据
- 里程碑：M1
- 工作流：assets
- 规模：L
- 前置依赖：M0-002, M0-003
- 素材需求：无（灰盒即可；用 CC0 样例测试）；素材状态：n.a.
- 引用文档：ARCHITECTURE §11.1; ARCHITECTURE §11.2; ARCHITECTURE §11.7; LICENSES.md §1; LICENSES.md §2; LICENSES.md §3; LICENSES.md §4.4; ASSET_GAPS §3.4

## 背景
素材管线（ARCHITECTURE §11.1）共 9 步：ingest → normalize → lod → textures → mesh → collision → manifest → pack(.fpak) → upload。本任务只做前两步（ingest、normalize）与"缺失素材报告"，是整条管线的闸门：**无 LICENSES.md 登记或状态非 `OK` 的素材一律拒绝处理**（D-LIC、LICENSES §1 第 1–2 条）。后续 M1-002（mesh/texture）、M1-003（fpak）、M1-004（audio）、M1-005（manifest）都在本任务的 `work/` 目录与 CLI 骨架上扩展。
关键约束：
- 台账表在 `LICENSES.md` §3，**按表头列名解析、不依赖列序**；`asset_id` 以 `AST-EXAMPLE-` 开头的行必须忽略；状态枚举仅 `OK/PENDING/REJECTED`。
- `raw_path` 形如 `assets/incoming/{需求ID}/{渠道}-{订单号}/…`；`assets/incoming/` 与 `assets/processed/` 在 .gitignore（M0-001）。
- 命名规范（ARCHITECTURE §11.2）：全小写 `snake_case`，前缀 ∈ `veh_ wpn_ chr_ prop_ bld_ veg_ tex_ snd_ vfx_ ui_`。
- 规范化坐标系：单位 m、Y-up、前向 -Z（自定义规范，ARCHITECTURE §11.1 步骤 2）。
- 缺失素材报告：按需求 ID 汇总 `real / placeholder / missing`（ASSET_GAPS §3.4）。

## 具体内容
### 要创建/修改的文件
- `tools/asset-pipeline/package.json`、`tsconfig.json`（Node + TypeScript；CLI 入口 `asset-pipeline <ingest|normalize|report>`）
- `tools/asset-pipeline/src/ledger.ts` —— 解析 LICENSES.md §3 Markdown 表（按列名），导出 `LedgerRow`、`loadLedger(path)`、`lookupByRawPath(path)`
- `tools/asset-pipeline/src/ingest.ts` —— 闸门：遍历 `assets/incoming/**`，对每个文件定位所属台账行；校验状态；计算 sha256；复制到 `work/{asset_id}/raw/`
- `tools/asset-pipeline/src/normalize.ts` —— glTF 读入（`@gltf-transform/core`）→ 单位/朝向统一、重置原点、三角化、合并同材质子网格
- `tools/asset-pipeline/src/naming.ts` —— 命名规范校验
- `tools/asset-pipeline/src/report.ts` —— 缺失素材报告（JSON + 控制台表格）
- `tools/asset-pipeline/test/` —— vitest 用例与 `fixtures/`（含 1 个 CC0 许可的小型 glTF 样例；仓库内放自造的最小三角形/立方体 glTF 即可，不引入外部素材）
- `.github/workflows/` 中新增 `asset-pipeline` 作业（接入 M0-002 的 CI 基线）
### 要实现的行为
1. 台账解析：缺列、重复 `asset_id`、状态非枚举值 → 报错并给出行号。
2. 闸门规则：文件的 `raw_path` 前缀未命中任何台账行 → 失败（退出码 2，信息含文件路径）；命中但 `status != OK` → 失败（退出码 3，`PENDING`/`REJECTED` 区分提示）；D-LIC 渠道（TurboSquid、CGTrader、Sketchfab 非 CC、KitBash3D、ActorCore、A Sound Effect 多厂牌）命中且 `confirmation_path` 为 `—` → 失败（退出码 4）。
3. 检查台账存档文件存在：`license_text_path`、`confirmation_path`（非 `—` 时）；缺失 → 失败（LICENSES §4.4）。
4. 处理动作白名单：`allowed_processing` 代码集（`CONVERT/LOD/TEXCOMP/RETARGET/REBIND/MATPARAM/ASSEMBLE`；音频 `TRANSCODE/LOUDNORM/EDIT/LAYERMIX`）；后续任务每执行一种处理前调用 `assertAllowed(assetId, code)`，超出白名单即拒绝（D-MOD）。本任务提供该函数并在 normalize 中以 `CONVERT` 校验。
5. 命名校验：输出逻辑 ID 不符合 §11.2 前缀/`snake_case` → 失败。
6. normalize：缩放到米（按 glTF 声明的单位或 `--unit-scale` 参数）、轴向转换、原点重置到底部中心（可配置）、三角化、同材质子网格合并；输出 `work/{asset_id}/normalized.gltf`（仅中间产物，不上传）。
7. `report`：读取 `assets/requirements.json`（若无则由 ASSET_SHOPPING_LIST §4.2 的需求 ID 前缀清单生成的最小版本；本任务只需支持"需求 ID → 状态"映射）与 manifest 草稿，输出每个需求 ID 的 `real/placeholder/missing`。
### 接口与数据
```ts
interface LedgerRow { asset_id; req_id; type; status: 'OK'|'PENDING'|'REJECTED'; raw_path; allowed_processing: string[]; confirmation_path: string; /* …其余列同 LICENSES §2 */ }
function ingest(opts:{incoming:string; ledger:string; work:string}): IngestResult   // 抛 GateError{code, file}
function assertAllowed(assetId:string, action:ProcessingCode): void
type ReqStatus = 'real'|'placeholder'|'missing'
```

## 不做什么
- LOD、纹理压缩、碰撞简化：M1-002。
- `.fpak` 打包与 Range 读取：M1-003；音频处理：M1-004。
- manifest.json 生成与体积预算：M1-005。
- 采购、登记 LICENSES.md、许可证核实本身：人类负责人（H-009、H-023、H-049）；本任务只读台账。

## 验收标准
1. 构造 3 个样例（台账行 `PENDING`、台账行 `REJECTED`、完全未登记），`asset-pipeline ingest` 对三者均失败，退出码分别为 3、3、2；有单测断言（`ingest.gate.test.ts`）。
2. 状态 `OK` 的 CC0 样例 glTF 通过 ingest，`work/{asset_id}/raw/` 内文件 sha256 与源文件一致（单测比对）。
3. D-LIC 渠道行且 `confirmation_path = —` 的样例被拒（退出码 4）；改为有效路径后通过。
4. `AST-EXAMPLE-*` 行被忽略：仓库当前模板台账上运行 ingest 不报错、不处理任何文件。
5. normalize 对一个 1 cm 单位、Z-up 的样例立方体输出 1 m 边长、Y-up，包围盒误差 < 1e-4 m（单测）。
6. `report` 对含 2 个需求 ID 的样例输出 JSON：已处理者 `real`、仅有灰盒者 `placeholder`、无任何输入者 `missing`；按需求 ID 汇总计数正确。
7. CI 作业 `asset-pipeline` 墙钟 < 3 min（ARCHITECTURE §17.5 对 manifest-budget 的同量级要求；以 Actions 日志计时）。

## 测试要求
- 单元测试（vitest）：`ledger.parse`（缺列/重复/非法状态/管道符）、`gate`（上面 3+1 种情形）、`naming`（合法与非法 10 例）、`normalize`（单位/朝向/原点）、`assertAllowed`（越权动作被拒）。
- 集成：`pnpm -C tools/asset-pipeline test` 全绿；对仓库真实 LICENSES.md 空模板运行 `ingest` 返回成功且零处理。
- 性能基准：无。
- 回归：fixtures 的 sha256 写入 `test/fixtures/SHA256SUMS`。

## 性能预算
- 无运行期预算；仅 CI 作业 < 3 min（ARCHITECTURE §17.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：glTF normalize 的朝向/单位处理容易膨胀；骨骼/动画资产的 normalize 只需透传，不在本任务做重定向（RETARGET 属后续素材接入任务）。
- `requirements.json` 的最终来源（ASSET_SHOPPING_LIST 需求 ID 表）本任务只做最小版本，缺口登记 PLAN_ISSUES。
- 素材未到位不阻塞本任务（灰盒/样例即可）。
