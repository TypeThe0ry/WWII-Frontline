# M1-064 素材接入：M1 视觉素材

## 元数据
- 里程碑：M1
- 工作流：assets
- 规模：M
- 依赖：M1-002（网格/纹理管线）, M1-003（fpak）, M1-046（FP 武器 pass）, M1-048（坦克渲染）, M1-052（建筑）, M1-051（树篱/植被/道具）
- 素材需求：V-CHR-US-001; V-CHR-GE-001; V-WPN-US-*; V-WPN-GE-*; V-ANM-*; V-VEH-US-003; V-NAT-*; V-PRP-*; V-BLD-*; V-FX-*；素材状态：real / placeholder 混合（逐 ID 以 LICENSES.md 为准）
- 引用文档：ASSET_SHOPPING_LIST §4.2; ASSET_GAPS §3.5; ASSET_GAPS §3.4; LICENSES.md §4; ARCHITECTURE §11.7; ARCHITECTURE §11.8

## 背景
本任务是 M1 里真正依赖素材到位的接入任务（INDEX §0：只有名为 `assets-m*-integrate-*` 的任务依赖素材）。流程（ASSET_GAPS §3.5）：素材到位 → 放入 `assets/incoming/{需求ID}/…` → 在 `LICENSES.md` 登记并状态 `OK` → 管线处理（`.fpak` 打包）→ 同一需求 ID 覆盖占位 → manifest `placeholder` 置 false → 视觉回归/听感检查 → 在 `ASSET_SHOPPING_LIST.md` 勾选"已购"（勾选由人完成）。M1 必须到位的视觉（MILESTONES §2.5）：美军 Thompson/BAR/Garand/M1911 + 德军 Kar98k/MP40/StG44/MG42、美/德夏季步兵各 1、1 辆坦克（Sherman）、Jeep、沙袋/木箱/铁丝、树篱与树、至少 1 套民居（或占位盒体）、枪口焰/烟/爆炸。允许占位：坦克内饰、第一人称高精度武器（GAP-V06）、建筑模块（GAP-V02 灰盒）、树篱（GAP-V03 替代方案）。执行者不得自行去非正规渠道找素材、不得自制、不得 AI 生成（ASSET_GAPS §3.4.4、D-MOD）；PENDING/REJECTED 素材一律不得进入主分支（LICENSES.md §1）。处理后只以 `.fpak` 交付，不暴露标准 `.glb/.ktx2/.ogg`（R11）。

## 具体内容
### 要创建/修改的文件
- `assets/manifests/m1-visual.json`（进 git）—— 需求 ID → `.fpak` 条目映射、`placeholder` 字段、LOD、质量档标签、`license_id`（引用 LICENSES.md 行）。
- `tools/asset-pipeline/recipes/m1-visual/*.json` —— 每个需求 ID 的处理配方：减面比例（LOD 1/0.5/0.25/0.1）、纹理 ORM 打包、KTX2 模式（角色/武器 UASTC，背景 ETC1S）、碰撞简化；配方只含 D-MOD 允许的处理（格式转换、LOD/减面、贴图压缩、重定向、权重传递、材质参数、部件装配）。
- `tools/asset-pipeline/retarget/*` —— 动画重定向到统一骨架（M1-045 的 `Skeleton` 约定）；M1 的 V-ANM-001/002/004/005 剪辑。
- `client/src/render/assets/registry.ts`（修改）—— 需求 ID→资源解析：real 优先，缺失回退占位（不改渲染代码，M1-045/046/048/051/052 已留接口）。
- `tools/asset-pipeline/report.ts`（修改）—— 缺失素材报告：按需求 ID 汇总 `real/placeholder/missing`，按里程碑汇总，输出 `reports/m1-asset-status.md`。
- 视觉回归基线更新：`client/test/visual/baselines/*`（真素材到位后）。
### 要实现的行为
1. 逐需求 ID 核对 `LICENSES.md`：状态非 `OK` 的 ID 保持占位并在报告里标 `blocked(license)`，不得处理。
2. 角色：V-CHR-US-001/GE-001 → 减面至 LOD0 ≤ 45k 三角形，骨架重定向，材质 ORM；武器：第三人称 5k–12k、第一人称占位或 25k–60k（GAP-V06 决定前用第三人称品质，H-045）；坦克：V-VEH-US-003 减面至 ≤ 150k 三角形，保留炮塔/炮管层级与 socket。
3. 环境：V-NAT（树/草/灌木/地表 CC0 材质）、V-PRP（沙袋/木箱/铁丝）、V-BLD（民居，若无则保持灰盒）。
4. 特效贴图：V-FX-001..004（枪口焰/烟/爆炸/弹孔），许可证 PENDING 则保持引擎内置粒子。
5. 每个处理产物进入 manifest，`placeholder: false` 仅当对应 LICENSES 行 `OK`；`manifest-budget` 门禁（M1-005）通过。
6. 视觉验证：对每个换入的角色/武器/坦克做固定机位截图，人工确认可读性（标"需人工"）。
### 接口与数据
```json
{ "id": "V-VEH-US-003", "pak": "veh_us_003.fpak", "entry": "m4a1_sherman", "placeholder": false, "license_id": "LIC-0123", "lods": [150000, 75000, 37000, 15000], "sockets": ["socket_muzzle","socket_seat_driver","socket_seat_gunner","socket_seat_commander"] }
```

## 不做什么
- 不做 M2 批素材（城镇模块、桥、废墟、兵种变体，M2-047）、冬季角色（M3+）。
- 不做素材采购/付款/卖家确认（人类，H-049、H-050）；不修改 LICENSES.md 状态（人类登记；执行者只读与校验）。
- 不自制/修改素材几何（D-MOD 边界之外）；不提交 `assets/incoming/` 与 `assets/processed/`。
- 不做音频与字体接入（M1-065）。

## 验收标准
1. 每个状态为 `OK` 的 M1 视觉需求 ID 在 manifest 中 `placeholder: false`，且渲染中使用真资源（Playwright 断言节点不含 `PH_` 前缀；脚本 `asset-status` 输出 real 计数 = LICENSES `OK` 计数）。
2. 占位清单输出：`reports/m1-asset-status.md` 列出全部 M1 视觉需求 ID 的 `real/placeholder/missing` 与原因，且与 ASSET_SHOPPING_LIST §4.2 的 ID 一一对应（脚本比对，缺项 = 0）。
3. 无 PENDING/REJECTED 素材进入主分支：`asset-pipeline ingest` 在含 PENDING 条目的测试夹具上返回非零（CI 测试 `ingest_gate_pending`），且仓库 `git ls-files` 中无 `assets/incoming/**`、`assets/processed/**`。
4. 产物目录无独立 `.glb/.ktx2/.ogg/.opus`：`find <upload_dir> -name '*.glb' -o -name '*.ktx2' -o -name '*.ogg' -o -name '*.opus'` 结果为空（E1-14）。
5. Medium 首图包体积 ≤ 150 MB（`manifest-budget` 作业，含占位；E1-9）。
6. 单个坦克 ≤ 150k 三角形、单个角色 LOD0 ≤ 45k、第三人称武器 ≤ 12k（manifest 校验脚本）。
7. 视觉回归基线更新后，Playwright 固定机位（角色/武器/坦克/树篱/民居）通过；缺素材的 ID 在报告里有对应 GAP 决定或"推迟到 Mx"（ASSET_GAPS §3.4.3；E1-17）。

## 测试要求
- 单元测试：配方解析；manifest 字段校验；ID→资源解析与回退。
- 集成/无头测试：`pnpm asset:build m1-visual`（带 CC0 示例与 PENDING 夹具）；Playwright 视觉。
- 性能基准：加载 M1 首图 Medium 包耗时（100 Mbps）记入 `reports/m1-asset-status.md`。
- 回归：M1-045/046/048/051/052 的占位路径在素材缺失时继续工作。

## 性能预算
- ARCHITECTURE §11.4：首图 Medium ≤ 150 MB（角色/武器 US+GER ≈ 18 MB、载具 ≈ 16 MB、建筑/道具 ≈ 40 MB 等估算构成）；§13.5.3 可见三角形/动态角色预算不得回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、CI `manifest-budget`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：LICENSES 校验结果（§4.4：PENDING/REJECTED 条数、缺存档文件）贴入回报
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 素材未 OK 时本任务标 `blocked` 或 `partial`，**不影响其它任务**（INDEX §0）；在 STATUS 顶部 blocked 清单登记"所缺条目—日期—人类"，并在 `docs/NEEDS_HUMAN.md` 追加条目（H-049 M1 采购批、H-050 卖家回复处理、H-047 GAP 决定）。
- 书面确认未到的"慎用"渠道素材（CGTrader/TurboSquid 等）保持 PENDING（D-LIC）。
- 重定向与权重传递属 D-MOD 允许范围，但需 H-002 确认边界；未确认前只做格式转换与减面。
