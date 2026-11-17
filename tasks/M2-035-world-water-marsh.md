# M2-035 world：水面与沼泽

## 元数据
- 里程碑：M2
- 工作流：world
- 规模：M
- 依赖：M1-049, M2-005
- 素材需求：V-NAT-008（水面/水体贴图，M4 前可灰盒）；素材状态：placeholder
- 引用文档：MAPS §3.7; MAPS §3.4; MAPS §3.6; MAPS §1.6; ARCHITECTURE §9.8; ARCHITECTURE §10.2; ARCHITECTURE §13.5.2

## 背景
Carentan 的洪泛沼泽（德军放水淹没 Douve 河漫滩）是 M2 地图的天然边界（MAPS §3.4、§3.6）：车辆不可通行，步兵可涉水（东侧浅滩移速 ×0.5【待调参】）。MAPS §3.7 对策：**单一水面网格 + 简化着色（折射仅 High 以上）；浅水可涉水的"逻辑区"与视觉解耦**。MAPS §1.6 规定水系矢量属性：`kind`、`depth_m`、`walkable_depth`（决定可涉水）。ARCHITECTURE §9.8 远景环与天空章节提到海岸线水面，海洋属 M4-014，本任务只做沼泽/河/运河。
M2-005 已产出区块、碰撞、navmesh（含 water_zones 区域数据，若地图 `water.geojson` 已烘焙进区块）；M1-049 提供区块流式。

## 具体内容
### 要创建/修改的文件
- `client/src/world/water/water_surface.ts` —— 单一水面网格（覆盖整个可玩区，按水位平面 + 水体掩码纹理裁剪/混合），顶点/片元着色：浑浊棕绿色、边缘泡沫/岸线淡入、芦苇区域颜色偏移；WGSL 与 GLSL 双份。
- `client/src/render/shaders/wgsl/water.wgsl`、`client/src/render/shaders/glsl/water.frag.glsl` —— 水面着色；折射（采样场景颜色 + 深度吸收）仅 High+，Medium/Low/WebGL2 退化为不透明深度渐变。
- `client/src/world/water/water_mask.ts` —— 由区块的水体数据生成/采样 `waterMask`（R8，覆盖 2048 m 可玩区，分辨率 2 m）。
- `crates/sim/src/terrain/water.rs` —— 逻辑涉水区查询：`water_depth_at(x,z) -> f32`、`wade_speed_mult(depth)`、`vehicle_passable(x,z) -> bool`（沼泽不可通行）；供移动/载具规则调用（若 M1/M2 已有同名模块则只补缺口）。
- `data/maps/carentan/water.json`（如 M2-005 已产出则仅引用）与 `data/movement.json` 中涉水速度条目：`wade_speed_mult_by_depth`（初值 0–0.3 m：×0.85；0.3–0.8 m：×0.5；> 0.8 m 不可通行/溺水规则不在本任务），状态 `initial`【待调参 → REFERENCE_MEASUREMENTS §4.2】。
- `client/src/world/water/water_audio_hooks.ts` —— 涉水时向音频事件总线发 `Footstep{surface:"water"}`（音频实现 M2-046，缺素材静音）。
### 要实现的行为
1. 水面：单 draw call 的水面网格；视觉水位与区块高度图交点形成自然岸线；沼泽深度 `depth_m` 驱动颜色（深/浅）。
2. 涉水逻辑与视觉解耦：涉水速度与可通行性只由 sim 查询（`/data` 驱动）决定，客户端预测与服务器一致；视觉水面不参与碰撞。
3. 载具：进入 `vehicle_passable == false` 区域时被阻挡（或沿用 sim 载具 KCC 对不可通行区的处理——接口在此定义，载具侧接入在 M2-015，若已接入则联调）。
4. 质量档位：High+ 折射；其余档简化着色；WebGL2 路径同样可用。
5. 水面与 froxel 雾（M2-031）、CSM 阴影接收兼容。
### 接口与数据
```rust
pub fn water_depth_at(terrain: &TerrainView, x: f32, z: f32) -> f32;      // m，0 表示干地
pub fn wade_speed_mult(cfg: &MovementCfg, depth_m: f32) -> f32;          // 来自 /data
pub fn vehicle_passable(terrain: &TerrainView, x: f32, z: f32) -> bool;
```

## 不做什么
- 不做海洋、潮线、Gerstner 波、浪花（M4-014）；不做冬季冰面（M4-015）。
- 不做溺水/游泳规则、水下视觉（不在 M2 范围）。
- 不做水面反射 SSR（M4-019）。

## 验收标准
1. 水面 pass GPU 时间 P95 ≤ 0.5 ms（High，RTX 3060，1080p；沼泽机位，`pass.water` timestamp）。
2. 涉水：站在 `depth_m` 分别为 0.2/0.5 m 的测试点，移动速度与 `/data` 配置值误差 < 2%（`cargo test -p sim wade_speed`，复用 M0-011 速度测量方法）。
3. 车辆不可通行：测试载具从干地驶向沼泽，在边界处被阻挡（`vehicle_passable` 单测 + 载具集成测试 1 个，若载具未就绪则单测）。
4. 视觉/逻辑一致：沿 20 条随机直线采样，`waterMask` 判定的"水中"与 `water_depth_at > 0` 的不一致采样点 ≤ 2%（脚本）。
5. WebGPU 与 WebGL2 路径均渲染水面；Medium/Low 无折射纹理分配（调试 RT 统计）。
6. 沼泽机位截图入基线（M2-050）。

## 测试要求
- 单元测试：`cargo test -p sim`：深度插值、阈值边界、速度表；vitest：`waterMask` 采样与坐标变换。
- 集成/无头测试：Playwright 沼泽机位两条路径截图；headless-client 驱动角色步入沼泽记录速度（可并入 M2-049 前的脚本）。
- 性能基准：`pass.water` timestamp 1 000 帧。
- 回归：M0/M1 的 KCC 速度测试保持通过（水外行为不变）。

## 性能预算
- ARCHITECTURE §13.5.2 含于"不透明主通道/透明"；本任务单列 ≤ 0.5 ms（INDEX 验收）；水面三角形 ≤ 20k（单网格，按 §13.5.3 总三角预算 ≤ 4.0 M 的 0.5%）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test -p sim`、`vitest`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（V-NAT-008）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 涉水速度数值（MAPS §3.6 的 ×0.5）属【待调参】，对标测量项 M-xx（REFERENCE_MEASUREMENTS §4.2）可能在 M2-054 回填。
- 本任务触及 `sim`（涉水查询），工作流虽为 world，但 sim 改动必须保持确定性规则（不得引入 HashMap、墙钟，ARCH §4.3.2）；改动面应尽量小。
- 洪泛区范围（T-FLOOD）为人工描摹（H-061），若多边形未完成则用已有区域，记录缺口。
