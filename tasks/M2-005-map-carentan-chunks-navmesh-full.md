# M2-005 地图：全图区块、碰撞与导航烘焙

## 元数据
- 里程碑：M2
- 工作流：maps
- 规模：L
- 前置依赖：M2-004, M1-035, M1-036
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §9.1; ARCHITECTURE §9.5; ARCHITECTURE §16.2; MAPS §6.1; ARCHITECTURE §9.6; ARCHITECTURE §9.7; ARCHITECTURE §11.8

## 背景
M1-035/M1-036 对 1/3 区域做了 64 m 区块切片、splat、植被表、碰撞与 navmesh 烘焙。本任务对全图（2048 m，32 × 32 = **1 024 块**，每块 65 × 65 高度样本，ARCH §9.1）执行全流程，并新增 WebGL2 路径用的预计算遮挡体（ARCH §9.7：WebGL2 无 Hi-Z，仅视锥 + 距离 + 预计算建筑遮挡体）。输出由客户端流式加载（M2-036）、服务器碰撞（M2-008 起）、bot 导航（M2-027）、遮挡剔除（M2-034）使用。
关键数字：
- 视觉区块 `terrain/b_{bx}_{bz}.bin`：高度（delta 滤波 + brotli ≈ 3–4 KB）+ splat（64×64×4 权重，≈ 8–16 KB）+ props 实例表 + 植被种子；每块 30–80 KB，全图 ≈ 30–80 MB 原始（ARCH §9.1）；M1-035 单块 ≤ 80 KB。
- 碰撞 `collision/c_{bx}_{bz}.bin`：`[heights f32/u16][static_mesh_indices][material_ids]`，Meshopt 压缩，与视觉分开（ARCH §9.5）；服务器载入全图 Rapier `HeightField`（2049² f32 = 16.8 MB）。
- 植被实例 6 B：`(x:u16,z:u16,species:u8,scale:u8,rot:u8)`；每块上限 草 4 096 / 灌木 256 / 树 192（ARCH §9.6）。
- navmesh（ARCH §16.2）：`cell_size 0.3 m`、`cell_height 0.2 m`、`agent_radius 0.4 m`、`agent_height 1.8 m`、`max_climb 0.5 m`、`max_slope 45°`，128 m 瓦片；载具网格独立（MAPS §6.2：半径 2.0 m、坡度 25°）。
注意：MAPS §6.2 的 navmesh 参数（坡度 35°、台阶 0.4 m）与 ARCH §16.2 不同，**以 ARCH §16.2 为准**（已裁定，MAPS §6.2 已同步修订）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/chunks.py`、`collision.py`、`navmesh.py`（M1 脚本扩到全图；全图并行分块，多进程）
- `tools/map-pipeline/occluders.py` —— 由 `buildings.geojson` 生成 `occluders.json`：每栋可遮挡建筑的凸包/OBB 简化体（≤ 12 顶点），按 64 m 区块索引
- `data/maps/carentan/chunks/index.json` —— 区块索引（bx,bz → 文件、字节数、sha256、是否含建筑）
- `data/maps/carentan/navmesh.bin`、`navmesh_vehicle.bin`、`offmesh_links.json`（窗户翻越、楼梯、树篱缺口、梯子；来自 M2-003 的 door/窗标注）、`occluders.json`
- 输出 `.fpak` 区块包经 M1-003 的打包器按 64 m 区块/类别分包（D-PAK）；manifest 条目写入 `assets/processed/manifest`（不入 git，清单入 git）
### 要实现的行为
1. 1 024 块全部生成视觉 + 碰撞；单块视觉 ≤ 80 KB；全图原始合计 ≤ 80 MB。
2. 植被：沼泽草/苹果树/橡树（树篱）按 splat 与排除规则（道路、建筑、水体）分布；每块计数不超上限。
3. 碰撞：树篱用线性盒（ARCH/MAPS §3.7）；建筑静态碰撞按区块组织；材质 id 与 M1-012 的穿透材质表对齐。
4. navmesh：步兵网格 + 载具网格；off-mesh links 来自建筑 door/窗点与树篱缺口；洪泛区步兵可涉水（`walkable_depth` 内，区域标记为 `shallow`，供移速修正）、载具网格排除。
5. 连通性：烘焙后脚本验证 HQ ↔ 15 个候选据点步兵连通；载具网格 HQ ↔ L1/L2 候选连通（沿 N13 与市区主路）。
6. `occluders.json`：只包含高度 ≥ 6 m 且足迹 ≥ 60 m² 的建筑；总数与体积记录。
### 接口与数据
- 区块索引与碰撞格式沿用 M1-035；若字段需扩展，向后兼容追加并在 PLAN_ISSUES 记"细化"。
- `occluders.json`：`{ "chunk":[bx,bz], "boxes":[{"id","verts":[[x,y,z]...],"height_m"}] }`。

## 不做什么
- 运行时剔除实现 → M2-034；流式加载与内存预算 → M2-036；水面渲染 → M2-035。
- 动态障碍/tile cache（被毁建筑）→ M3。
- 载具 bot 道路图寻路 → M3（ARCH §16.2）。

## 验收标准
1. `chunks/index.json` 含 1 024 条且每条文件存在、sha256 匹配；最大单块 ≤ 80 KB；全图合计 ≤ 80 MB（`map-pipeline stats` 输出）。
2. 抽样 500 条垂直射线：视觉高度与碰撞高度差 ≤ 0.05 m。
3. navmesh 全图烘焙耗时 ≤ 15 min（记录机器型号）；参数与 ARCH §16.2 一致（脚本打印并断言）。
4. HQ ↔ 15 个候选据点步兵 navmesh 连通 = 15/15；载具网格 HQ ↔ L1/L2 候选连通 = 6/6。
5. 每块植被数 ≤ 草 4 096 / 灌木 256 / 树 192（扫描断言 0 违规）。
6. `occluders.json` 通过 schema，建筑数与 §M2-003 的"高 ≥ 6 m 且 ≥ 60 m²"子集数一致。
7. 同输入两次运行 `chunks/index.json` 与 navmesh 的 sha256 一致。

## 测试要求
- 单元测试：`pytest tools/map-pipeline/tests/test_chunks.py`（块索引映射、边界样本共享：相邻块共享边缘 65 样本值一致）、`test_occluders.py`。
- 集成：加载 navmesh 做 50 对随机点寻路，成功率 100%；`cargo test -p sim collision_load_full_map`（若 sim 已有加载器，M2-008 之前可先写在 `tools` 内）。
- 性能基准：烘焙耗时、内存峰值。
- 回归：区块索引哈希入 `hashes.json`。

## 性能预算
体积：区块总计 ≤ 80 MB 原始（ARCH §9.1）；Carentan Medium 包 ≤ 150 MB（ARCH §11.4，由 M1-005 manifest 门禁）；服务器碰撞 `HeightField` 16.8 MB（ARCH §9.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：若烘焙与遮挡体合计超一次会话，把 `occluders` 拆成续作 M2-005b（README §5.6），不要硬做。
- navmesh 在树篱/洪泛场景的卡死风险（MILESTONES §3.8）：off-mesh 与缺口标注要在 bot 压测（M2-049）监控"无进展计数"。
- 区块文件不得以独立 .glb/.ktx2 暴露，统一进 `.fpak`（ARCH §11.8、R11）。
