# M4-010 地图：Foy 区块、碰撞与导航烘焙

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：M
- 依赖：M4-009, M2-005
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §9.1; ARCHITECTURE §9.5; ARCHITECTURE §16.2; MAPS §6.1; MAPS §6.2; MAPS §5.7

## 背景
把 Foy 的高度图、矢量、森林与工事打成运行时格式：1 024 个 64 m 区块、splat（雪/冻土/结冰路/针叶落叶/石材/泥）、碰撞、步兵与载具 navmesh、遮挡体。流程与 M4-005 对称，复用 `tools/map-pipeline` 阶段。
关键约束：
- 区块与碰撞格式见 ARCHITECTURE §9.1/§9.5；服务器全图 HeightField 2049² f32 = 16.8 MB。
- navmesh 参数（ARCHITECTURE §16.2）：`cell_size 0.3 m、cell_height 0.2 m、agent_radius 0.4 m、agent_height 1.8 m、max_climb 0.5 m、max_slope 45°`，128 m 瓦片；载具网格独立（半径 2.0 m、坡度 25°）。
- 森林内部：树干为静态圆柱碰撞（ARCHITECTURE §9.6），navmesh 需把树干作为障碍；密林中 off-mesh 链接较少，路径需穿行林间空隙。
- 村庄：建筑可进入占比受限；窗口/楼梯 off-mesh 标注。
- 雪地：splat 规则输入"积雪遮盖权重"（供 M4-015 材质使用），但此处只生成权重，不渲染。
- 烘焙耗时 ≤ 15 min（INDEX 指标）。

## 具体内容
### 要创建/修改的文件
- `data/maps/foy/chunks/`（清单+哈希）、`splat_{0,1}.png`、`navmesh.bin`、`navmesh_vehicle.bin`、`offmesh_links.json`、`occluders.json`。
- `tools/map-pipeline/map_pipeline/stages/{splat_winter,navbake}.py` —— 增加冬季 splat 规则与树干障碍注入。
- `reports/m4-foy-chunks.json`。
### 要实现的行为
1. 切片 1 024 块并生成 terrain/collision 文件与 manifest。
2. 生成 splat：层含 snow、packed_snow/frozen_road、frozen_soil、conifer_litter、leaf_litter、stone、mud；规则由坡度、森林多边形、道路类别、水边决定；输出各层占比。
3. 烘焙步兵与载具 navmesh，树干与围栏作为障碍；校验：村庄（L4）↔ 森林（L2）双向步兵连通；N30 与 L1c 载具连通；牧场对载具可行但标记"雪深减速"区域（元数据，供 sim/bot 使用）。
4. 生成 off-mesh（窗户翻越、围栏、沟渠跨越）链接。
5. 生成 WebGL2 遮挡体，建筑为主；森林用密度遮蔽近似不建遮挡体。
6. 体积报告：与 Carentan/Omaha 对比。
### 接口与数据
- 同 M4-005 的区块 manifest/`navmesh.meta.json` 约定；新增 `snow_depth_zones.json`（`polygon`、`depth_m`、`vehicle_speed_mul` 【待调参】），数据在 `/data`，客户端/服务器均不硬编码。

## 不做什么
- 不渲染雪地/天气（M4-015）；不做 bot 行为参数（M4-016）。
- 不做动态 tile cache 更新（沿用 M3-021）。
- 不做 Omaha 数据（M4-005）。

## 验收标准
1. 1 024 个区块文件齐全，`map-pipeline validate-chunks foy` 缺失/哈希错误 = 0。
2. 村庄 L4 候选点 ↔ 森林 L2 候选点：步兵路径双向存在，`findPath` 成功率 100%；N30/L1c 载具路径存在。
3. 导航烘焙总耗时 ≤ 15 min（记录机器/CPU）。
4. 单块 terrain 文件 ≤ 80 KB，超限块数 = 0（森林块含植被种子，重点核对）。
5. 区块总体积不超过 Carentan 全图体积的 120%（记录字节数）。
6. 服务器加载 Foy 全图碰撞 + navmesh 后 RSS 增量 ≤ 40 MB（`cargo test -p server load_map_foy -- --nocapture`）。
7. 树干障碍注入后，随机 1 万对森林内点对路径失败率 ≤ 2%（排除被围死点；报告）。

## 测试要求
- 单元测试：相邻块边缘高度连续；splat 每像素权重和 = 255；树干障碍注入的 navmesh 洞数与树数一致（采样）。
- 集成/无头测试：`cargo test -p bots navmesh_connectivity_foy`；`headless-client` 在 Foy 行走 L2→L4 路线 60 s 无卡死/穿地。
- 性能基准：`cargo bench -p bots navmesh_query` 在 Foy 数据上的中位数记录。
- 回归：`hashes.json`、`DATA_HASH`。

## 性能预算
- 导航烘焙 ≤ 15 min；体积落在 ARCHITECTURE §11.4 子项内；无新增服务器 CPU 预算（§6.10）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 若 M4-006/007 因许可证 PENDING 只有占位地形，则本任务只能对占位数据烘焙，连通性与体积验收记 `partial`。
- navmesh 参数 MAPS §6.2（35°/0.4 m）与 ARCHITECTURE §16.2（45°/0.5 m）不一致，以 ARCHITECTURE 为准（同 M4-005 备注，PLAN_ISSUES 已/将登记）。
- 密林 navmesh 多边形数量与内存需关注；超限时降低细节并记录。
