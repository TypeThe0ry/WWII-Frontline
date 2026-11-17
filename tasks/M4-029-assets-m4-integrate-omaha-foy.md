# M4-029 素材接入：M4 Omaha/Foy

## 元数据
- 里程碑：M4
- 工作流：assets
- 规模：M
- 前置依赖：M3-041, M4-003, M4-008, M4-014, M4-015
- 素材需求：V-BLD-005..007; V-NAT-003; V-NAT-006; V-NAT-008; V-SKY-002; V-CHR-US-004; V-CHR-GE-003; F-AMB-002; F-AMB-004; I-PD-001；素材状态：real（仅限 LICENSES.md=OK 者）/ placeholder（其余）
- 引用文档：ASSET_SHOPPING_LIST §4.5; ASSET_GAPS §3.5; ARCHITECTURE §11.4; ARCHITECTURE §11.7; ARCHITECTURE §11.8; ASSET_GAPS §3.4

## 背景
这是 M4 唯一**真正依赖素材到位**的任务（README §5.3）。素材未 OK 则任务 `blocked`/`partial`，**不影响其它任务**。流程（ASSET_GAPS §3.5 替换流程）：素材放入 `assets/incoming/{需求ID}/…` → 在 `LICENSES.md` 登记并状态 OK → 管线处理（`.fpak` 打包）→ 同一需求 ID 覆盖占位 → manifest `placeholder` 置 false → 视觉回归/听感检查 → 在 SHOP 勾选"已购"。
需求清单与现状（SHOP §4.5）：
- V-BLD-005 阿登村舍（GAP-V02，无直接候选）；V-BLD-006 大西洋壁垒碉堡（CGTrader 候选，**需书面确认，PENDING**）；V-BLD-007 滩头障碍（CGTrader/Flipped Normals/RenderHub，PENDING）。
- V-NAT-003 冬季云杉（Poly Haven CC0 基础 OK；付费 PENDING）；V-NAT-006 海滩材质（Poly Haven/ambientCG CC0，OK）；V-NAT-008 水材质（无免费来源，需补搜）；V-SKY-002 冬季 HDRI（Poly Haven CC0，具体文件未核实）。
- V-CHR-US-004、V-CHR-GE-003 冬季角色：GAP-V04，**未发现**；依赖 H-069 决定（夏季角色 + 冬季服装模块需确认 D-MOD）。
- F-AMB-002、F-AMB-004：Freesound CC0（OK）。I-PD-001 公共领域二战录音：**暂缓，PENDING**（PD 判断复杂，禁止直接使用）。
体积门禁（ARCHITECTURE §11.4）：Medium 首图 ≤ 150 MB、High ≤ 350 MB、Ultra ≤ 600 MB（每图，manifest 门禁；超预算 1% 即失败，> 20% 增量须人工批准）。
`.fpak` 是唯一下发格式（R11）：CDN 上不得出现可独立下载的标准 .glb/.ktx2/.ogg 文件。

## 具体内容
### 要创建/修改的文件
- `assets/incoming/**`（人工放入，不入 git）；`assets/manifests/{omaha_dog_green,foy}.json` —— 每个需求 ID → `fpak/entry/lod/placeholder:false`。
- `tools/asset-pipeline/profiles/m4_omaha_foy.yaml` —— 每需求 ID 的处理配置（LOD 目标、KTX2 档位、碰撞简化、socket 命名）。
- `client/src/world/assets/ReplacementMap.ts` —— 需求 ID → 资产句柄；未真实 → 占位。
- `data/props/{beach,winter}.json` —— 校对尺寸/碰撞与真素材实际边界框一致（偏差 ≤ 5%，否则调整数据而非缩放素材失真）。
- `reports/assets/m4-missing.json` —— 缺失素材报告（real/placeholder/missing）。
- 更新 `docs/ASSET_SHOPPING_LIST.md` 的"已购"勾选**不由本任务改**（人类勾选）；本任务在回报中列出状态。
### 要实现的行为
1. 对每个需求 ID：检查 `LICENSES.md` 状态；`OK` 才处理，`PENDING/REJECTED/未登记` 保持占位并写入缺失报告（管线 `ingest` 会机械拒绝）。
2. 处理：转换、LOD/减面、KTX2 压缩、碰撞简化（D-MOD 允许范围内），打包 `.fpak`（按 64 m 区块与类别）。
3. 接入：障碍/碉堡实例表（M4-003）与森林/村舍点位（M4-008）改指向真资产；材质接入 M4-014/M4-015 的 PBR 宏位；冬季角色走 M3-042 的换装/重定向流程（若 GAP-V04 无素材则保持占位）。
4. Poly Haven/ambientCG 材质登记署名（CC0 无需署名但记录来源）；`credits.json` 与 manifest 一致（H-088）。
5. 体积门禁：生成 Medium/High/Ultra 的每图体积报告；超预算时降 LOD/纹理档并重打包，直到达标。
6. 视觉回归与听感检查：重拍固定机位（交给 M4-040 汇总）；环境声接入后做简要听感检查（交 M4-028 清单）。
### 接口与数据
- manifest 条目：`{"req_id":"V-BLD-007","lod":[…],"pak":"props_b_12_07.fpak","entry":"hedgehog_lod0","placeholder":false,"license_ref":"LIC-0123"}`；`placeholder:true` 的条目必须保持灰盒命名 `PH_{req_id}`。

## 不做什么
- 不采购素材、不付款、不发邮件（H-080、H-009/H-050，人工）。
- 不自制/重画/AI 生成任何素材（A2、D-MOD）；不从非正规渠道下载。
- 不改 `LICENSES.md` 状态（人类登记）。
- 不做载具/武器等 M3 素材（M3-041/042 已覆盖）。

## 验收标准
1. **已 OK 的需求 ID 状态为 real**：`asset-pipeline report` 中，LICENSES.md=OK 的 ID 全部 `real`；PENDING/未登记的 ID 全部 `placeholder` 或 `missing`；二者之间无错配（脚本断言 0 错配）。
2. **High 包 ≤ 350 MB、Ultra ≤ 600 MB**（manifest 门禁）：Omaha 与 Foy 各自 Medium ≤ 150 MB / High ≤ 350 MB / Ultra ≤ 600 MB，`manifest-budget` 作业通过；记录实测字节数。
3. CDN 构建产物扫描：`dist/` 与上传清单中 `.glb|.gltf|.ktx2|.ogg|.wav|.mp3` 文件数 = 0（`tools/asset-pipeline/check-no-loose-assets.sh`）。
4. 真素材碰撞/尺寸：每个 `real` 资产的包围盒与 `/data/props` 记录偏差 ≤ 5%（脚本），否则更新数据并在 PR 说明。
5. 授权可追溯：每个 `real` 资产在 `LICENSES.md` 有条目；`credits.json` 与 manifest 条目一一对应（CC-BY 署名无遗漏，数量差 = 0）。
6. 缺失素材报告列出全部 10 个需求 ID 的最终状态，并写入 `tasks/STATUS.md`；任务状态：全部 OK 才 `done`，否则 `partial` 并列出缺口与对应 H-080。

## 测试要求
- 单元测试：`pytest/vitest tools/asset-pipeline`：`ingest` 对 PENDING 素材报错；`placeholder` 字段推断；体积计算。
- 集成/无头测试：Playwright 加载 Omaha/Foy 机位确认无 404、无白模缺失（占位项仅限报告列出的 ID）。
- 性能基准：加载时间（Medium，本地缓存冷/热）记录；纹理解码 Worker 时间不超过 M1-006 基线 20%。
- 回归：视觉基线更新（M4-040）；`manifest` 哈希。

## 性能预算
- 体积：§11.4（Medium ≤ 150 / High ≤ 350 / Ultra ≤ 600 MB）；主视图 draw call/三角形不得超 §13.5.3；植被每块实例上限 §9.6。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest` / `vitest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖 NEEDS_HUMAN H-080（M4 采购批）、H-069（GAP-V04 冬季角色决定）、H-050（卖家回复与台账更新）、H-088（PENDING 清零）。H-080 无默认假设；缺则 `blocked` 并在 STATUS 登记，其它任务继续。
- 冬季角色 GAP-V04 市场上"未发现"：可能长期占位；MILESTONES 允许"推迟到 Mx"但须在 ASSET_GAPS 记决定（人类）。
- I-PD-001 暂缓：不得凭搜索结果使用（PD 判断需 NARA/LOC 或律师确认）。
