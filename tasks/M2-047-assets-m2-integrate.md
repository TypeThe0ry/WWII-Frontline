# M2-047 素材接入：M2 批

## 元数据
- 里程碑：M2
- 工作流：assets
- 规模：M
- 前置依赖：M1-064, M1-065, M2-033, M2-035, M2-046
- 素材需求：V-BLD-004; V-BLD-008; V-BLD-010; V-PRP-005..010; V-CHR-US-003; V-WPN-*（M2 批：V-WPN-US-002/007/011/012、V-WPN-GE-010）; V-ANM-003; V-ANM-006; V-ANM-007; A-WPN-012; A-WPN-013; G-VOX-003; G-VOX-004; J-IR-002; V-FX-005；素材状态：以 `LICENSES.md` 为准（OK 才接入，其余保持 placeholder）
- 引用文档：ASSET_SHOPPING_LIST §4.3; ASSET_GAPS §3.5; ASSET_GAPS §3.4; ARCHITECTURE §11.1; ARCHITECTURE §11.4; ARCHITECTURE §11.7; ARCHITECTURE §11.8

## 背景
这是 M2 唯一真正依赖素材的任务（README §5.3）。M2 其余任务全部在灰盒占位下完成。素材准入唯一入口是根目录 `LICENSES.md`：**只有状态 `OK` 的素材可进入构建**，`PENDING/REJECTED/未登记` 一律不得处理、不得上传；管线 `asset-pipeline ingest` 会机械拒绝（ARCH §11.7，M1 的 `asset-pipeline-ingest-gate`）。D-LIC：付费素材需卖家书面确认；D-PAK：处理后只以 `.fpak` 分块容器交付（R11、ARCH §11.8）；D-MOD：允许格式转换/LOD/贴图压缩/动画重定向/部件组装，不允许从零建模、雕刻、重画贴图、AI 生成。
ASSET_SHOPPING_LIST §4.3 的 M2 批：城镇街道模块（V-BLD-004，GAP-V02）、桥梁（V-BLD-008，GAP-V13）、废墟（V-BLD-010）、野战无线电/帐篷/散兵坑/电线杆围栏/残骸（V-PRP-005–010，GAP-V12）、美军兵种变体（V-CHR-US-003，GAP-V18，需确认 D-MOD）、M1 Carbine/M1903A4/M18/刺刀/Panzerfaust、动画（V-ANM-003 翻越、006 救治、007 建造；GAP-V17）、StG44/MG34 枪声（A-WPN-012/013；GAP-A01）、战场人声（G-VOX-003/004）、远距炮声 IR（J-IR-002）、血迹贴花（V-FX-005，GAP-V15）。替换流程见 ASSET_GAPS §3.5。包体积门禁：core ≤ 8 MB、Carentan Medium 包 ≤ 150 MB（ARCH §11.4，E2-8）。

## 具体内容
### 要创建/修改的文件
- `assets/incoming/{需求ID}/…` —— 人工放入的原始素材（`.gitignore`，不提交）。
- `tools/asset-pipeline/src/ingest.ts`、`process/*.ts`、`pack.ts`（沿用 M1，必要时扩展）—— 对每个需求 ID 执行 ingest → 处理（glTF/KTX2/Meshopt/音频 Opus+AAC）→ `.fpak` 打包 → 写 manifest。
- `assets/manifest/m2.manifest.json`（提交）—— 每需求 ID 的 `status: real|placeholder|missing`、`placeholder` 字段、`pak/entry/offset/bytes/sha256[0:8]`、体积、许可证行 ID。
- `reports/assets/m2_missing_report.md` —— 缺失素材报告（real/placeholder/missing 汇总，附对应 GAP 与 H- 编号）。
- `client/src/render/asset_registry.ts`、`client/src/audio/asset_registry.ts`（修改）—— 按需求 ID 解析：优先 `real`，否则回落占位（`PH_{需求ID}`）；挂点（socket）命名遵守既有约定，不得发明新名。
- `docs/ASSET_SHOPPING_LIST.md` 的"已购"勾选由人类/总控更新，本任务**不改 docs**，只在 STATUS 记录。
### 要实现的行为
1. 对 `LICENSES.md` 中每个 M2 需求 ID：状态 `OK` → 接入并置 `real`；`PENDING/REJECTED/无条目` → 保持占位并在报告列出，**不得处理或打包**。
2. 接入验证：几何尺寸/碰撞与 `/data` 一致（真素材换占位不改玩法）；socket 齐全（武器 `socket_muzzle/ejection/magazine`、载具/道具按既有约定）；材质变体总数仍 ≤ 60（ARCH §10.4）；音频三层（近/中/远）缺哪层按"该层静音"。
3. 打包：按 64 m 区块（建筑/道具）与类别（`wpn/char/snd/fx`）分 `.fpak`；Opus 为主、AAC 兜底；`.glb/.ktx2/.ogg` 不独立输出。
4. 体积门禁：运行 `manifest-budget`，core ≤ 8 MB（gzip/br 后 JS+WASM+UI）、Carentan Medium ≤ 150 MB（含全部 M2 已接入素材）；超限则按 D-MOD 允许手段（降 LOD/mip、压缩）调整，仍超限写 PLAN_ISSUES，不得放宽。
5. 回归：替换后跑视觉回归（M2-050 基线更新需人工确认）与听感检查清单（音频，记录到 STATUS）。
### 接口与数据
```json
{ "id": "V-BLD-010", "status": "real|placeholder|missing", "placeholder": false, "license_row": "LIC-0xx", "pak": "bld_carentan_0.fpak", "entries": [{"logical_id":"…","offset":0,"bytes":0,"sha256_8":"…"}] }
```

## 不做什么
- 不采购、不下载、不登记素材（人类：H-058 GAP 决定、H-059 M2 采购批、H-050 卖家回复）；不修改 `LICENSES.md` 状态。
- 不自制/AI 生成任何缺失素材；不用其它层顶替缺失音频层。
- 不接入 M3 素材（M3-041…044）；不做 M1 素材（M1-064/065 已做）。

## 验收标准
1. 台账一致性：对 M2 清单每个需求 ID，`manifest` 状态与 `LICENSES.md` 状态 100% 一致（`OK`→`real`；非 OK→`placeholder/missing`；脚本 `asset-pipeline verify-m2`，不一致即失败）。
2. 门禁：`manifest-budget` 作业通过——core ≤ 8 MB；Carentan Medium 包 ≤ 150 MB（报告记录实际字节）。
3. 已 OK 的需求 ID 在缺失素材报告中状态为 `real`，且无任何 `PENDING` 素材进入 `.fpak`（扫描 manifest 与许可证行，0 例违规）。
4. 玩法不回归：替换后 `cargo test`（`/data` 校验）与 `vitest` 全绿；真素材尺寸与 `/data` 尺寸偏差 ≤ 2%（脚本逐个对比包围盒）。
5. 材质变体总数 ≤ 60（CI 统计，ARCH §10.4）；Medium 视觉回归基线更新项已人工确认并记录。
6. 音频：已接入的音频条目 `decodeAudioData` 探测通过（Opus；AAC 兜底路径单测）；缺失层静音，听感检查清单记录于 STATUS。
7. `.fpak` 完整性：随机抽 20 条条目 Range 读取，`sha256[0:8]` 全部匹配；无独立 `.glb/.ktx2/.ogg` 产物（扫描 `assets/processed`，0 个）。

## 测试要求
- 单元测试：ingest 闸门（构造 PENDING 条目必拒）；manifest 生成/校验；尺寸比对。
- 集成/无头测试：Playwright 加载 M2 manifest 后进入 Carentan 机位无 404；缺失报告生成。
- 性能基准：加载体积/时间报告（Medium 包下载 ≤ 150 MB）；`manifest-budget`。
- 回归：视觉回归（M2-050 基线）、听感检查清单。

## 性能预算
- ARCHITECTURE §11.4：core ≤ 8 MB；Carentan Medium ≤ 150 MB / High ≤ 350 MB / Ultra ≤ 600 MB；§12.7 音频 Medium ≈ 18 MB；§13.4 显存各项（建筑/道具活跃集、角色/武器）不得超预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test`、`vitest`、`manifest-budget`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **阻塞规则**：若 M2 批素材整体未到位（`LICENSES.md` 无 OK 行），本任务标 `blocked`，在 STATUS 顶部登记"所缺条目 — 日期 — 谁来解除（H-058/H-059/H-050）"，并追加 `docs/NEEDS_HUMAN.md` 条目；其它 M2 任务不受影响。只部分 OK 时标 `partial`，只接入 OK 项。
- MG34（A-WPN-013）无实录来源（GAP-A01）；用 MG42 变调/混音需人工接受史实偏差（D-MOD、H-058），未批准不得接入。
- 兵种变体（V-CHR-US-003）需用基础身体 + 配件拼装，依赖 D-MOD 确认（H-002）。
- 若人类未决定 GAP（H-058），保持灰盒，M2 验收时在 ASSET_GAPS 对应 GAP 写"推迟到 Mx"（ASSET_GAPS §3.4 第 3 条）。
