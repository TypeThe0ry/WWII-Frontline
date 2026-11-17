# M4-003 地图：滩头障碍与防御工事摆放

## 元数据
- 里程碑：M4
- 工作流：maps
- 规模：M
- 依赖：M4-002
- 素材需求：V-BLD-006; V-BLD-007（灰盒可）；素材状态：placeholder
- 引用文档：MAPS §4.4; MAPS §4.6; MAPS §4.7; MAPS §6.2; ASSET_GAPS §3.2; NEEDS_HUMAN H-077, H-079

## 背景
Omaha 的视觉与战术特征由"滩头障碍带 + 砾石堤/海堤 + WN 抵抗巢"构成。本任务把 MAPS §4.4 的真实地标清单落成 `landmarks.json`、道具实例表与工事点位。
关键约束：
- 障碍带：高潮线向海 40–250 m 的三排（Rommel's asparagus、Element C、水雷柱、刺猬），还原精度 B（密度/排布）；实例数可达数千 → GPU instancing + 距离分级（MAPS §4.7）。
- 砾石堤与海堤：沿 HWL 向陆 5–20 m，宽 10–15 m，精度 A（轮廓，来自 LiDAR）；海堤中途设断口（排水沟/台阶/物资堆）打断直线视线。
- WN：MAPS §4.4 仅给"候选"（WN 72/71/70/68/66，±200–250 m），**编号与位置在资料间矛盾**，以 Taylor《Omaha Beachhead》/Cross-Channel Attack 为准，由人类裁定（H-077）。游戏内名称仅用 [G] 原创描述名，真实 WN 编号仅内部标注。
- 视线管理：沉没/搁浅登陆艇与车辆残骸至少每 60 m 一个；弹坑；烟幕由游戏逻辑触发（MAPS §4.6）。
- 坐标一律经 M4-001 的相似变换；设计据点非真实地标，允许移动 ≤ 60 m；真实地标不得移动。

## 具体内容
### 要创建/修改的文件
- `data/maps/omaha_dog_green/landmarks.json` —— 真实地标与 WN（`kind: wn|seawall|draw|church|…`，`fidelity`，`state_1944`，`refs[]`，`verified: bool`）。
- `data/maps/omaha_dog_green/props_beach.json` —— 障碍/残骸/铁丝网/地雷区实例表（`type`、`pos`、`yaw`、`row`、`placeholder_id`）。
- `data/maps/omaha_dog_green/fortifications.json` —— WN 内的 Tobruk/炮位/堑壕/碉堡点位与射界朝向、`cover_points`（B-03 用）。
- `tools/map-pipeline/map_pipeline/stages/place_obstacles.py` —— 按排布规则从 HWL 偏移生成障碍带（固定种子，可重复）。
### 要实现的行为
1. 以 HWL 折线为基准，按三排规则生成障碍实例：排距、间距、类型比例写在 `tools/map-pipeline/maps/omaha_dog_green/obstacles.yaml`（参数【待调参】）；每个实例标注 `blocks_vehicle`、`explosive_clearable`（供 M3 破坏/Engineer 清障规则引用）。
2. 沿 `coast.geojson` 摆放残骸（每 60 m ≥ 1 个）、海堤断口、弹坑贴花点。
3. 为每个 WN/障碍组写 `refs[]`（史料出处）；无出处者 `verified=false` 并在 `notes` 写"未核实"。
4. 校验：所有点位在 0..2048、不在水体深处（水深 > 1.2 m 的点不放步兵工事）、不落入实心建筑；WN 周围为每个工事生成 ≥ 3 个步兵入口（MAPS §6.3）。
5. 灰盒：障碍用盒/圆柱占位（尺寸取 `/data` 道具定义，`PH_V-BLD-007__*` 命名）；WN 用盒体 + 屋顶斜面（`PH_V-BLD-006__*`），保留射击孔高度。
### 接口与数据
- `props_beach.json` 条目：`{"id":"hedgehog_0412","type":"czech_hedgehog","pos":[x,y],"yaw_deg":..,"row":2,"asset_req":"V-BLD-007"}`。
- 道具类型表（尺寸、碰撞、掩体等级）放 `data/props/beach.json`，客户端不硬编码。

## 不做什么
- 不写 zones/spawns/artillery（M4-004）。
- 不做障碍的运行时渲染与实例化（world 任务，见 M4-014/M4-029）。
- 不裁定 WN 编号（人类 H-077）；不购买素材（H-080）。
- 不实现障碍被爆破的 sim 规则（沿用 M2-014/M3-021 的破坏机制，本任务只标注属性）。

## 验收标准
1. 每个 WN 与每个障碍组（排）在 `landmarks.json`/`props_beach.json` 中有 `refs[]` 或 `verified=false` + "未核实"备注；脚本检查缺项数 = 0。
2. `map-pipeline validate omaha_dog_green` 通过：点位 100% 位于 0..2048、0 个落入实心建筑、0 个在深水（> 1.2 m）。
3. 障碍实例总数记录于报告；按三排规则生成，重复运行输出哈希一致（固定种子）。
4. 沿 HWL 方向每 60 m 窗口内残骸/掩体数 ≥ 1（脚本沿海岸线滑窗检查，缺口列表为空）。
5. 海堤直线段连续长度 ≤ 120 m（含断口），脚本报告最长连续段。

## 测试要求
- 单元测试：`place_obstacles` 在合成直线 HWL 上的排距/间距与参数一致（误差 < 1 cm）；种子相同输出相同。
- 集成测试：`map-pipeline validate` 对 Omaha 全数据；`pytest test_omaha_props.py`。
- 性能基准：实例总量上报；M4-014/M4-029 渲染预算依据该数字（ARCHITECTURE §13.5.3 draw call 上限）。
- 回归：写入 `hashes.json`。

## 性能预算
- 障碍带实例数可达数千：须能以按区块"一个 draw call/物种/LOD"渲染（ARCHITECTURE §9.6 thin instances）；本任务只提供数据并报告实例总数与每 64 m 区块最大实例数。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖 NEEDS_HUMAN H-077（WN 裁定、潮位）与 H-079（障碍带/工事审美审核）。默认假设：H-077 以 MAPS §4.4 候选位置继续，不在游戏内展示可证伪的精确史实标注；H-079 默认"游戏性优先于史实精度"。
- V-BLD-006/007 的候选素材许可证为 PENDING（需卖家书面确认，D-LIC）；未 OK 前保持占位，真实接入在 M4-029。
- Tobruk 专用模型"未发现"（SHOP §4.5.1）；用碉堡变体或灰盒代替并记入缺口报告。
