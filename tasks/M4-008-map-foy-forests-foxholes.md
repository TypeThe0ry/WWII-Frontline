# M4-008 地图：Foy 森林与散兵坑布置

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：M
- 依赖：M4-007
- 素材需求：V-BLD-005; V-NAT-003（灰盒可）；素材状态：placeholder
- 引用文档：MAPS §5.4; MAPS §5.6; MAPS §5.7; ARCHITECTURE §9.6; ARCHITECTURE §13.5.3; NEEDS_HUMAN H-078, H-079

## 背景
Foy 的玩法核心是"林—空地—村"：森林内部视距短（< 40 m），林缘散兵坑俯瞰 300–400 m 的开阔雪地牧场（杀戮区），村庄为近战核心（MAPS §5.6）。本任务布置森林密度/树种分布、预设散兵坑与工事、雪地 splat 规则输入，并给出植被实例上限校验。
关键约束：
- 植被实例表：每 64 m 区块 6 B/实例 `(x:u16,z:u16,species:u8,scale:u8,rot:u8)`；每块上限：草 4096（仅 R_near）、灌木 256、树 192（高密度森林区上限）（ARCHITECTURE §9.6）。
- 散兵坑：沿林内北缘分布，间距 ~10–25 m，单/双人坑，木盖/原木，积雪（MAPS §5.4 精度 B；具体坑位需人工，H-078/H-079）；战时坑位与现存凹坑有出入，**不得把现代纪念碑位置写入游戏**。
- 视线管理：牧场每 80 m 至少一处可蹲伏矮掩体（田埂、围栏、弹坑、雪堆、倒木）；N30 直线用路旁树列/车辆残骸/路障打断；雾能见度 450–600 m（MAPS §5.6）。
- 雪地载具：雪深 > 0.4 m 减速、陷入概率【待调参】（规则在 sim，非本任务）。
- 性能：针叶林树数以十万计 → 八面体 impostor、LOD；Low 档减半（MAPS §5.7）。

## 具体内容
### 要创建/修改的文件
- `data/maps/foy/forests.geojson` —— 细化密度分区与树种比例（更新 M4-007 产物）。
- `data/maps/foy/vegetation_seeds.json` —— 每区块植被种子与密度参数（实例由运行时/烘焙按种子生成）。
- `data/maps/foy/fortifications.json` —— 散兵坑（`type: foxhole|log_dugout`，`pos`，`facing_deg`，`capacity:1|2`，`cover_value`）、围栏/田埂/倒木/路障点、`cover_points`。
- `data/maps/foy/landmarks.json` —— Foy 十字路口、村教堂/小礼拜堂（位置未核实则 `verified=false`）、N30、Bois Jacques 林缘。
- `data/props/winter.json` —— 冬季道具类型表（尺寸/碰撞/掩体等级）。
- `tools/map-pipeline/map_pipeline/stages/{place_foxholes,vegetation_density}.py`。
### 要实现的行为
1. 依据 `forests.geojson` 与坡度/路径排除生成每块植被预算表；任一块树数 > 192、灌木 > 256 时自动降密并记录。
2. 沿林缘北缘线按 10–25 m 间距（固定种子抖动）生成散兵坑；L2a/L2b/L2c（MAPS §5.6）附近密度更高；`log_dugout` 位于 L2c (700,800) 附近，抗炮击（掩体值高）。
3. 牧场 80 m 栅格检查：每格是否存在 ≤ 1.2 m 高掩体，缺口自动补点并列报告。
4. 输出视线报告：从 L2 线向 L4 线的长视线（> 300 m）射线抽样的遮挡比例。
5. 灰盒：树用锥体/圆柱（`PH_V-NAT-003__*`）、农舍用盒体 + 屋顶斜面（`PH_V-BLD-005__*`），保留门洞与楼层以便可进入建筑玩法不变（ASSET_GAPS §3.2）。
6. 记录需要的人工审核：森林密度分布、坑位（H-079）。
### 接口与数据
- `fortifications.json` 条目：`{"id":"fh_0132","type":"foxhole","pos":[x,y],"facing_deg":..,"capacity":1,"cover":0.6,"asset_req":"V-BLD-005"}`（`cover` 取值范围与枪弹/爆炸减伤规则在 sim 数据中定义，不在此硬编码）。

## 不做什么
- 不渲染雪、天气、积雪植被材质（M4-015）。
- 不写 zones/spawns（M4-009）、不切区块与烘焙 navmesh（M4-010）。
- 不实现散兵坑的 sim 掩体规则（沿用既有掩体/姿态规则；若缺则写 PLAN_ISSUES）。
- 不购买素材（H-080），不下载 Poly Haven 之外的免费资源入库（入库走 LICENSES.md 流程，M4-029）。

## 验收标准
1. 全图 1 024 块中，任一块 树实例 ≤ 192、灌木 ≤ 256、草 ≤ 4096（ARCHITECTURE §9.6）；脚本统计超限块数 = 0。
2. `map-pipeline validate foy` 通过：散兵坑/工事坐标 100% 在 0..2048、0 个落入水体/实心建筑/坡度 > 35° 区域。
3. 牧场区（L3 范围）按 80 m 栅格，掩体覆盖率 = 100%（报告缺口列表为空）。
4. 散兵坑线长度与数量记录；相邻坑间距 5–95 分位在 10–25 m 内（报告）。
5. 全图预估树实例总数记录于报告，并给出 Medium/Low 档（Low 减半）的实例预算，用于 M4-015 的 ≤ 60 变体与粒子/植被预算核对。
6. 每个坑位/地标标注 `verified` 字段；`verified=false` 项在报告中汇总，且 `docs/NEEDS_HUMAN.md` 有 H-079 追加条目。

## 测试要求
- 单元测试：植被预算降密函数（超限输入→输出 ≤ 上限，保持空间分布单调）；坑位生成固定种子可重复；间距统计函数。
- 集成测试：`pytest test_foy_fortifications.py`、`map-pipeline validate foy`。
- 性能基准：树实例总数/每块最大值上报（输入 M4-015、M4-041）。
- 回归：`hashes.json`。

## 性能预算
- 植被每块上限遵守 ARCHITECTURE §9.6；渲染预算（主视图 draw call、可见三角形）遵守 §13.5.3，本任务不得产生超限的数据。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖 H-078（坑位需人工，Easy Company 纪念碑不入游戏）、H-079（森林/散兵坑审核，默认"游戏性优先于史实精度；instancing + 距离分级"）。
- V-BLD-005（阿登村舍）为 GAP-V02，V-NAT-003 免费基础为 Poly Haven CC0（状态 OK）+ 付费 PENDING；素材未到位用灰盒，真接入在 M4-029。
- 针叶林密度与"林内完全看不见"的平衡需 M4-039 与试玩验证。
