# M1-052 world：建筑与可进入室内

## 元数据
- 里程碑：M1
- 工作流：world
- 规模：L
- 依赖：M1-049, M1-034（建筑足迹/enterable/levels 数据）
- 素材需求：V-BLD-001..003（灰盒可，GAP-V02）；素材状态：placeholder（盒体+屋顶斜面，门窗洞保留）
- 引用文档：MAPS §3.6; MAPS §3.7; MAPS §1.6; MAPS §6.2; ARCHITECTURE §9.7; ARCHITECTURE §9.5; ASSET_GAPS §3.2

## 背景
Carentan 老城是室内战场景。MAPS §3.7：老城核心 ~800×600 m，建筑 300–500 栋；**可进入建筑 ≤ 25%**，其余为"实心"外壳（低面数外壳 + 合并）；按 64 m 区块流式；建筑 LOD0/1/2；WebGL2 无 Hi-Z → 预计算建筑遮挡体（M2-034）。`buildings.geojson` 属性：`style`、`levels`、`roof`、`enterable`、`destructible`、`asset_ref`、`ruin_stage`（MAPS §6.2）。ASSET_GAPS §3.2：建筑占位 = 盒体 + 屋顶斜面；**门/窗洞按玩法需要保留**（可进入的建筑必须有可通行的门洞与楼层）；碰撞用墙/地板/屋顶盒（不可用单个大盒）；室内可玩性不得因占位被改变。碰撞（ARCHITECTURE §9.5）：服务器载入静态建筑碰撞（凸分解或三角网，按区块组织）；客户端 WASM 只载 R_phys+预取圈。本任务完成：建筑渲染（外壳+可进入室内灰盒）、碰撞、探针放置、基础视锥+距离剔除。

## 具体内容
### 要创建/修改的文件
- `client/src/render/world/Buildings.ts` —— 区块建筑实例化：外壳（合并网格，按风格/LOD）、可进入建筑的内部（地板/墙/楼梯盒）、屋顶；按区块合批。
- `client/src/render/world/BuildingProcedural.ts` —— 灰盒生成器：`footprint + levels + roof + doors/windows`（读 `/data/maps/carentan/buildings.json`）→ 带门窗洞的墙体盒组合；节点命名 `PH_V-BLD-001__{id}`。
- `client/src/render/world/BuildingLod.ts` —— LOD0/1/2 与距离阈值（【待调参】：60/150/400 m）。
- `client/src/render/world/Culling.ts` —— 基础视锥 + 距离剔除（Hi-Z 为 M2-034）；按区块分组剔除。
- `crates/sim/src/world/building_collider.rs`（修改/新增）—— 建筑碰撞：墙/地板/屋顶盒、门洞可通行、楼梯斜坡（满足 KCC step/坡度）；与渲染同源。
- `tools/map-pipeline/` 如需：输出建筑灰盒数据（M1-034 的产物，若缺字段在此补一个导出脚本）。
- 光照探针放置接口：`client/src/render/lighting/ProbePlacement.ts`（读取 M1-035 的探针点；室内每层至少 1 个探针）。
- 测试：`client/test/world/buildings.test.ts`、`crates/sim/tests/building_walk.rs`、Playwright 机位。
### 要实现的行为
1. 外壳与内部分离：不可进入建筑只生成合并外壳（无内部）；可进入建筑生成地板/墙/楼梯与门窗洞，窗洞尺寸满足翻越规则（M1-011）。
2. 碰撞与渲染同源：门洞宽 ≥ 1.0 m、高 ≥ 2.0 m（玩法数据常量放 `/data`），楼梯坡度 ≤ 35° 或台阶 ≤ 0.4 m（与 KCC 参数一致，ARCHITECTURE §5.3 与 MAPS §6.2 navmesh 参数）。
3. 探针放置：每栋可进入建筑每层 ≥ 1 个探针；室内光照用探针 + ≤ 8 动态点光（ARCHITECTURE §10.2）。
4. 基础剔除：视锥 + 距离 + 区块级；远距（> 400 m）仅外壳 LOD2。
5. WebGL2 合批：同风格建筑 thin instances/合并网格，冻结材质（M1-042）。
6. 可进入建筑占比统计输出（≤ 25% 的校验脚本，来源 M1-034 数据）。
### 接口与数据
```ts
export interface BuildingDef { id: string; footprint: [number,number][]; levels: number; roof: 'gable'|'hip'|'flat'; style: string; enterable: boolean; openings: { type: 'door'|'window'; wall: number; offset: number; w: number; h: number; sill: number }[] }
export class Buildings { onChunkResident(bx: number, bz: number, defs: BuildingDef[]): void; onChunkEvict(bx: number, bz: number): void }
```
碰撞数据格式沿用 `collision/c_{bx}_{bz}.bin`（ARCHITECTURE §9.5）。

## 不做什么
- 不做建筑真素材接入（V-BLD-001..003，M1-064；GAP-V02 决定由人类 H-047）。
- 不做可摧毁建筑/受损阶段（M3）、遮挡剔除（M2-034）、可进入建筑的 navmesh 标注（M1-036/M1-034）。
- 不做室内家具/道具（M2 起）。
- 不做建筑内音频混响（M2-046）。

## 验收标准
1. 玩家可由门洞进入并上楼：选 ≥ 3 栋 `enterable` 建筑，headless 脚本 `building_walk` 驱动角色从户外经门洞到二层（sim 碰撞 + KCC，≥ 3 栋全部成功）。
2. M1 区域 draw call ≤ 900（WebGL2 Medium，"老城"机位，叠层 `draw_calls`；INDEX 指标）。
3. 老城机位 GPU 帧时间：High（3060）≤ 16.7 ms P95；该项仅记录，权威验收在 M1-069。
4. 渲染与碰撞对齐：随机 100 栋建筑的墙体渲染 AABB 与碰撞盒偏差 ≤ 0.1 m（`building_align` 单测）。
5. 可进入建筑占比 ≤ 25%（脚本统计 M1 区域数据；超出写 PLAN_ISSUES）。
6. 每栋可进入建筑每层 ≥ 1 个探针（数据校验脚本）。

## 测试要求
- 单元测试：灰盒生成（门窗洞）、LOD 选择、剔除、探针放置。
- 集成/无头测试：`crates/sim` 的 `building_walk`；Playwright 老城/室内机位截图基线。
- 性能基准：draw call/三角形/GPU 耗时导出到 `reports/m1-buildings.json`。
- 回归：M1-051 树篱 draw call 指标保持；M0 地形无回归。

## 性能预算
- ARCHITECTURE §13.5.3：主视图 draw call ≤ 1 200（High）/ ≤ 900（WebGL2 Medium）/ ≤ 500（Low）；可见三角形 ≤ 4.0 M；§9.7 WebGL2 的遮挡体延后到 M2-034。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、`cargo test -p sim`）；`clippy`/`eslint`/`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID 已列出
- [ ] 文档同步：建筑开口数据格式细化写 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：碰撞与渲染灰盒生成若超一次会话，拆 `M1-052b`（探针放置与剔除）。
- 灰盒的门窗洞必须与 navmesh（M1-036 off-mesh 链接）一致，否则 bot 无法入内；与 M1-037 的多层连通测试联动。
- 室内性能（draw call 与探针光照）是 E1-6/E1-7 的主要风险（MILESTONES §2.8）。
