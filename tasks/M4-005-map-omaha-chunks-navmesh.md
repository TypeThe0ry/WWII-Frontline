# M4-005 地图：Omaha 区块、碰撞与导航烘焙

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：M
- 依赖：M4-004, M2-005（Carentan 全图区块/碰撞/navmesh 烘焙流程，复用）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §9.1; ARCHITECTURE §9.5; ARCHITECTURE §16.2; ARCHITECTURE §11.4; MAPS §6.1; MAPS §6.2; MAPS §4.7

## 背景
把 M4-002～M4-004 的原始数据打成运行时格式：64 m 区块（1 024 块）、splat、碰撞、Recast navmesh、WebGL2 遮挡体。
关键数字：
- 区块文件 `terrain/b_{bx}_{bz}.bin`：高度 65² u16 + splat 64×64×4 + props 实例表 + 植被种子，约 30–80 KB/块（ARCHITECTURE §9.1）。
- 碰撞 `collision/c_{bx}_{bz}.bin`：高度场 + 静态网格索引 + 材质 id（ARCHITECTURE §9.5）；服务器载入全图 HeightField 2049² f32 = 16.8 MB。
- navmesh（ARCHITECTURE §16.2、MAPS §6.2）：步兵 `cell_size 0.3 m、cell_height 0.2 m、agent_radius 0.4 m、agent_height 1.8 m、max_climb 0.5 m、max_slope 45°`（MAPS §6.2 另给 35°/台阶 0.4 m 草案：以 ARCHITECTURE 为准，差异写 PLAN_ISSUES）；载具独立网格（半径 2.0 m、坡度 25°）；128 m 瓦片。
- splat 规则（沙/卵石/草/湿沙/泥土/混凝土/悬崖岩/道路）：按坡度、距 HWL/LWL、`coast.geojson` 类型与道路规则生成（MAPS §1.1 T-SPLAT，规则 AI + 人工微调）。
- 水体：近岸水下区域标记为"可涉水深度 ≤ 1.2 m 可走，> 1.2 m 不可走"（`walkable_depth`，MAPS §1.6）。
- 悬崖：1 m 高度图 + 专用碰撞（可选悬崖网格）；导航标注不可通行（MAPS §4.7）。

## 具体内容
### 要创建/修改的文件
- `data/maps/omaha_dog_green/chunks/` —— 区块索引与产物清单（大文件不进 git，仅清单 + 哈希；产物上传对象存储）。
- `data/maps/omaha_dog_green/splat_{0,1}.png` —— 材质权重，2048×2048。
- `data/maps/omaha_dog_green/navmesh.bin`、`navmesh_vehicle.bin` —— Recast 烘焙结果。
- `data/maps/omaha_dog_green/offmesh_links.json` —— 海堤台阶、悬崖攀爬点、残骸翻越点。
- `data/maps/omaha_dog_green/occluders.json` —— WebGL2 遮挡体（OBB，≤ 24 个大平面/块，ARCHITECTURE §9.7）。
- `tools/map-pipeline/map_pipeline/stages/{chunk,splat,collision,navbake,occluders}.py` —— 复用 Carentan 阶段，增加海岸/潮线 splat 规则与"水深不可走"。
### 要实现的行为
1. 切片 1 024 块；每块生成 terrain/collision 文件与 manifest 条目（尺寸、哈希、依赖）。
2. 按 splat 规则生成两张 RGBA 权重图；输出 `reports/m4-omaha-splat.json`（各层面积占比）。
3. 烘焙步兵与载具 navmesh；验证滩头→崖顶连通：US boat 到岸点 → L1…L5 各候选点至少一条步兵路径；D-1/D-3 载具路径连通（不要求其它路径）。
4. 标注 off-mesh links 并在 navmesh 中生成 link；记录数量。
5. 把海岸 `walkable_depth` 写入碰撞材质 id；深水区域碰撞上不可站立。
6. 汇总体积报告：全图区块总字节，Medium/High 加载子集估算，与 Carentan 对比。
### 接口与数据
- 区块 manifest 条目：`{"bx":int,"bz":int,"terrain":{"path","bytes","sha256"},"collision":{...}}`；格式与 Carentan 一致，不得引入新字段而不更新 schema。
- navmesh 元数据 `navmesh.meta.json`：烘焙参数、tile 数、poly 数、耗时、输入哈希。

## 不做什么
- 不渲染（海面/潮线材质在 M4-014）。
- 不放置 bot 行为参数（M4-016）。
- 不做动态障碍/破坏更新（沿用 M3-021 tile cache，本任务只保证 tile 切分兼容）。
- 不做 Foy 数据（M4-010）。

## 验收标准
1. 1 024 个区块文件齐全；`map-pipeline validate-chunks omaha_dog_green` 缺失/哈希错误数 = 0。
2. 区块总体积（terrain + collision）≤ Carentan 同量级：不超过 Carentan 全图体积的 120%（记录两者字节数）。
3. 连通性：boat 到岸点 → 每个 Assault 段目标点的步兵路径存在（Detour `findPath` 成功率 100%）；D-1、D-3 载具路径存在；失败点列表为空。
4. 导航烘焙耗时 ≤ 15 min（记录机器；与 Foy 同目标）。
5. 单个 64 m 区块 terrain 文件 ≤ 80 KB（ARCHITECTURE §9.1），超限块数 = 0；否则列出并写 PLAN_ISSUES。
6. 服务器加载 Omaha 全图碰撞 + navmesh 后内存增量 ≤ 40 MB（`cargo test -p server load_map_omaha -- --nocapture` 记录 RSS）。

## 测试要求
- 单元测试：`pytest` 对 chunk 切片边界连续性（相邻块边缘高度样本相等）；splat 权重和 = 255（每像素）。
- 集成/无头测试：`cargo test -p bots navmesh_connectivity_omaha`（沿用 M1-036 连通性测试）；`headless-client` 在 Omaha 上行走 boat 点→L3 路线 60 s 无穿地/卡死。
- 性能基准：navmesh 烘焙耗时、查询 1 万次随机路径中位数（沿用 `cargo bench -p bots navmesh_query`）。
- 回归：区块哈希写 `hashes.json`；`DATA_HASH` 更新。

## 性能预算
- 服务器 CPU 无新增预算；区块体积落在 ARCHITECTURE §11.4 各档预算内（地形约 30 MB 级，Medium 首图 ≤ 150 MB 的子项）。
- 导航烘焙 ≤ 15 min（INDEX 指标，与 M4-010 一致）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- navmesh 参数 MAPS §6.2（坡 35°/台阶 0.4 m）与 ARCHITECTURE §16.2（45°/0.5 m）不一致：以 ARCHITECTURE 为准，并把 35° 作为 bot 偏好（GAMEDESIGN 步兵坡度限制 35°，KCC 约束）；记 PLAN_ISSUES。
- 滩头开阔区 navmesh 多边形数量大，注意 tile 内存；超限时降低 `cell_size` 精度并记录。
- 悬崖/冲沟的 > 35° 区域为"步兵专属路线"需确认 KCC 与 navmesh 一致（避免 bot 卡死，M4-016 验证）。
