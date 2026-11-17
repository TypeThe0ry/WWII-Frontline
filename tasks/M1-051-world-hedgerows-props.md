# M1-051 world：树篱、植被与道具

## 元数据
- 里程碑：M1
- 工作流：world
- 规模：L
- 依赖：M1-049
- 素材需求：V-NAT-001; V-NAT-002; V-NAT-004; V-PRP-*（灰盒可，GAP-V03）；素材状态：placeholder（长条盒/圆柱）
- 引用文档：MAPS §3.7; MAPS §1.6; MAPS §6.2; ARCHITECTURE §9.6; ARCHITECTURE §9.5; ASSET_GAPS §3.2

## 背景
树篱（bocage）是 Carentan 的战术核心：掩体与视线分割（MAPS §1.6、§3.6）。MAPS §3.7：树篱用样条 + GPU instancing 的"树篱段"，2 种变体 + 缺口件，碰撞用线性盒；西南区树篱总长度 > 15 km（全图），M1 区域 > 4 km。`hedgerows.geojson` 属性：`height_m`（1.5–2.5）、`mound_width_m`、`gap`（缺口）、`vehicle_blocking`（MAPS §6.2）。植被（§9.6）：区块实例表 `(x:u16, z:u16, species:u8, scale:u8, rot:u8)` = 6 B；每块上限草 4 096（仅 R_near）、灌木 256、树 192；树 LOD0 2–4k tri（近 30 m）→ LOD1 ≈ 800 tri（< 100 m）→ 八面体 impostor（12×12 视角，每视角 128²，atlas 1536² KTX2）；全部 Thin Instances，按区块一个 draw call/物种/LOD。占位（ASSET_GAPS §3.2）：树篱/植被 = 长条盒/圆柱，尺寸取玩法数据；碰撞与掩体判定必须按真实玩法数据；道具/工事（沙袋、木箱、铁丝）= 盒/圆柱，沙袋墙保留射击孔高度。

## 具体内容
### 要创建/修改的文件
- `client/src/render/world/Hedgerows.ts` —— 从区块数据中的树篱段（折线 → 段实例：位置/朝向/长度/高度/土堤宽）生成 thin instances；2 种变体 + 缺口件；按区块一个 draw call。
- `client/src/render/world/Vegetation.ts` —— 接入区块植被实例表（草/灌木/树），LOD 与 impostor（M0-024 已有基础，本任务接入区块流式）。
- `client/src/render/world/Props.ts` —— props 实例表（沙袋、木箱、铁丝、障碍、路标…）灰盒与真模型的统一接口。
- `client/src/render/world/ImpostorBake.ts` —— 八面体 impostor 烘焙（运行时或离线工具接口；M0-024 若已有则复用）。
- `crates/sim/src/world/hedgerow_collider.rs`（修改/新增）—— 树篱线性碰撞盒（长度 × 土堤宽 × 高度）加载（来自区块碰撞数据）。
- `client/src/render/world/hedgerow-lod.ts` —— 远处合并/降级策略（> 150 m【待调参】用单条低面数带）。
- 测试：`client/test/world/hedgerows.test.ts`、`crates/sim/tests/hedgerow_collision.rs`。
### 要实现的行为
1. 树篱段：按 `hedgerows.geojson` 派生（M1-035 已烘焙为区块实例表）；高度/土堤宽来自数据；缺口件处无碰撞/无掩体。
2. 碰撞盒与 sim 一致：客户端渲染的树篱位置、尺寸与 sim 碰撞盒的偏差 ≤ 0.1 m（同一数据源，单测断言）。
3. 植被：按区块渲染草（仅 R_near）、灌木、树；树 3 档 LOD + impostor；风动顶点着色（`USE_VERTEX_WIND` 宏，M1-042）。
4. Props：实例表驱动；灰盒按 `/data` 尺寸；沙袋墙保留射击孔高度。
5. 预算驱动：Low 档植被密度减半（草 ×0.5），树篱不减。
6. 区块卸载时释放实例缓冲（与 M1-049 状态机钩子配合）。
### 接口与数据
```ts
export interface HedgeSegment { x: number; z: number; yaw: number; len: number; height: number; moundWidth: number; gap: boolean; vehicleBlocking: boolean }
export class Hedgerows { onChunkResident(bx: number, bz: number, segs: HedgeSegment[]): void; onChunkEvict(bx: number, bz: number): void }
```
植被实例表格式按 ARCHITECTURE §9.6；props 表字段 `(x, z, y_off, yaw, prop_id, scale)`（以 M1-035 实际输出为准）。

## 不做什么
- 不做建筑（M1-052）、地形材质（M1-050）、水面/沼泽（M2-035）。
- 不做树篱的真素材接入（V-NAT-001..004，M1-064）。
- 不做可摧毁树篱/建筑（M3 起）、遮挡剔除（M2-034）。
- 不做 bocage 扫描资产（GAP-V03，由人类决定 H-047）。
- 不做树篱对弹道穿透的规则（sim 的 M1-012 材质表负责）。

## 验收标准
1. M1 区域树篱总长 > 4 km，同屏 draw call（含树篱+植被+props）≤ 60（Playwright "树篱"机位，`draw_calls` 叠层导出；INDEX 指标）。
2. 碰撞盒与渲染对齐：随机 200 个树篱段，渲染 AABB 与 sim 碰撞盒偏差 ≤ 0.1 m（`hedgerow_align` 单测）。
3. 玩家无法穿越未标缺口的树篱，可通过缺口（sim 集成测试 `hedgerow_collision`）。
4. 树 LOD 切换无明显 pop：LOD 过渡距离差 ≥ 10 m 且 impostor 截图 ΔE < 5（固定机位）。
5. 区块驱逐后实例缓冲全部释放：反复穿越 5 min，JS 堆 + GPU 缓冲增长 < 5%（叠层内存采样）。
6. 植被每块实例数不超过上限（草 4 096、灌木 256、树 192，加载器断言）。

## 测试要求
- 单元测试：段生成数学、缺口处理、实例表解码、LOD 选择。
- 集成/无头测试：sim 碰撞穿越测试；Playwright 机位截图基线。
- 性能基准：draw call 与 GPU 耗时导出到 `reports/m1-hedgerows.json`。
- 回归：M0-024 植被验收（draw call ≤ 400）保持。

## 性能预算
- ARCHITECTURE §13.5.3：主视图 draw call ≤ 1 200（High）/ ≤ 900（WebGL2 Medium）；本任务（树篱+植被+props）≤ 60（INDEX）；§9.6 植被上限见上。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、`cargo test -p sim`）；`clippy`/`eslint`/`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_V-NAT-001__*` 等）；缺失素材需求 ID 已列出
- [ ] 文档同步：区块实例表字段细化写 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：树篱（含碰撞）与植被/props 若超一次会话，拆 `M1-051b`（植被/impostor/props）。
- 树篱总长 > 4 km 的前提是 M1-033 人工描摹完成；缺口径时验收 1 记 `partial` 并写 PLAN_ISSUES。
- 树篱真素材存在 GAP-V03（替代方案：自行用树篱纹理+盒体组合不算新建模，需 H-002 确认 D-MOD 边界）。
