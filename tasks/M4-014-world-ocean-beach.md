# M4-014 world：海洋、海岸与滩头

## 元数据
- 里程碑：M4
- 工作流：world
- 规模：L
- 依赖：M2-035（水面与沼泽）, M4-005
- 素材需求：V-NAT-006; V-NAT-008；素材状态：placeholder（V-NAT-006 为免费 CC0，状态 OK 可直接用；V-NAT-008 需补搜，先用程序化）
- 引用文档：ARCHITECTURE §9.8; ARCHITECTURE §9.2; ARCHITECTURE §10.3; ARCHITECTURE §13.5.2; MAPS §4.5; MAPS §4.7; ASSET_SHOPPING_LIST §4.5.1

## 背景
Omaha 北侧大部分为海面，需要大尺度海面、潮线/湿沙、浪花与远景海平线。ARCHITECTURE §9.8 规定：远景环 8 km，海岸线需要水面平面，**简化 Gerstner 波，仅 LOD 大尺度，≤ 0.3 ms**。M2-035 已有水面/沼泽通用实现，本任务扩展到海洋与滩头。
关键约束：
- 潮位为静态（固定 06:30，不做潮汐动画，MAPS §4.7）；潮线/湿沙范围来自 M4-002 的 `coast.geojson`（HWL/LWL）与高度图。
- 材质：潮湿深色沙、砾石（圆润）、混凝土、金属障碍（锈蚀）、泥土路、海面（灰绿）（MAPS §4.5）。V-NAT-006 沙/卵石/海草/潮线材质为 Poly Haven/ambientCG CC0（需先在 LICENSES.md 登记 OK 才能入构建）；V-NAT-008 水材质与泡沫无免费现成方案，需着色器 + 泡沫贴图（补搜，缺则程序化噪声泡沫）。
- 水下浅滩逻辑：深度 ≤ 1.2 m 可涉水（`walkable_depth`，MAPS §1.6），> 1.2 m 不可走/游泳规则以 sim 为准；本任务提供渲染一致性（水面高度与碰撞/可走判定一致）。
- WebGPU 与 WebGL2 路径：海面着色器双份（WGSL / GLSL ES 3.00，ARCHITECTURE §10.3）；Low 档简化为平面 + 法线贴图滚动；SSR 仅 Ultra（M4-019）。
- 视觉预算：High GPU 帧 ≈ 12.0 ms（§13.5.2），海面 ≤ 0.3 ms；总变体 ≤ 60（§10.4）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/ocean/OceanMaterial.ts` —— 海面材质（Gerstner 简化，4 个波分量，距离淡出为平面）。
- `client/src/render/ocean/shaders/ocean.wgsl`、`ocean.glsl` —— 双路径着色器。
- `client/src/world/CoastSystem.ts` —— 读取 `coast.geojson` 与高度图，生成潮线/湿沙 splat 权重与浪花（泡沫）带网格。
- `client/src/world/WaterLevel.ts` —— 全图海平面常量与"水面高度查询"（供音频/UI/脚步材质）。
- `data/render/ocean.json` —— 波参数（振幅/波长/方向/速度，状态 `initial`）与颜色。
- `client/tests/render/ocean.visual.spec.ts`。
### 要实现的行为
1. 海面：按 CDLOD 远景环 LOD4–6 范围渲染平面水，近处叠加 Gerstner 波；到可玩区边缘渐隐入远景海平线。
2. 潮线与湿沙：沿 HWL/LWL 之间做湿度渐变（粗糙度降低、颜色变深），砾石堤/海堤交界做混合；与 splat 统一采样，无接缝。
3. 浪花带：沿海岸线生成泡沫带贴花/网格（低成本），随时间滚动；Low 档仅静态泡沫纹理。
4. 水面与高度图一致：相交处无穿帮（深度偏移+菲涅尔），潮线世界坐标 vs 高度图交点误差 ≤ 0.2 m。
5. 双路径与降级：WebGL2 路径无 compute 依赖；WebGPU 路径也不依赖 compute（纯顶点/片元）。
6. 缺素材：V-NAT-008 缺失时用程序化泡沫噪声，材质命名 `PH_V-NAT-008`，manifest `placeholder: true`。
### 接口与数据
- `interface OceanSystem { init(scene, cfg: OceanConfig): void; update(dt: number, camera): void; setQuality(tier: QualityTier): void; dispose(): void }`；`QualityTier` 使用 M4-017 的定义（若 M4-017 未完成，先按 ARCHITECTURE §13.1 的 Low/Medium/High/Ultra 枚举，不硬编码数值，数值读数据文件）。

## 不做什么
- 不做 SSR 水面反射（M4-019）；不做潮汐动画/天气海况变化。
- 不做障碍/工事渲染（M4-003 数据 + M4-029 素材接入）。
- 不做音频海浪（M4-028）。
- 不做水下渲染/游泳（Backlog）。

## 验收标准
1. 海面渲染 GPU 耗时 ≤ 0.3 ms（High，RTX 3060，1080p；WebGPU timestamp 或 `performance.measure`，固定机位 `omaha_beach_01`）；参考机缺失记 `未测` → partial。
2. 潮线与高度图一致：同一机位截图中潮线世界坐标与 `coast.geojson` HWL 的偏差 ≤ 0.2 m（脚本沿 20 个采样点测量）；无穿帮（视觉回归基线通过）。
3. 材质变体数不超出 ≤ 60（Low ≤ 40）；`pnpm -C client test:variants` 通过。
4. WebGL2 与 WebGPU 两条路径均渲染 Omaha 机位；SSIM ≥ 0.9（同机位，阈值初值）。
5. 缺 V-NAT-008 素材时帧内无报错；`PH_V-NAT-008` 出现在缺失报告。
6. 水面高度查询 API 单测：给定世界点返回 在水下/水面/陆地 三态，100 个采样点与 `coast.geojson` 一致。

## 测试要求
- 单元测试：`vitest`：Gerstner 位移函数数值（与参考解析解误差 < 1e-4）；`WaterLevel.query` 边界。
- 集成/无头测试：Playwright `ocean.visual.spec.ts` 固定机位截图（`omaha_beach_01..03`，两条路径）与基线比较（差异像素 ≤ 0.1%，ARCHITECTURE §17.4）。
- 性能基准：`pnpm -C client bench:ocean` 输出 GPU ms（High/Medium/Low）。
- 回归：新增视觉基线（由 M4-040 汇总）。

## 性能预算
- 海面 ≤ 0.3 ms（ARCHITECTURE §9.8）；不得使 GPU 总预算 12.0 ms（§13.5.2）回归；材质变体 ≤ 60/40（§10.4）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 缺口：V-NAT-008 无免费来源（SHOP §4.5.1）；沙/卵石材质 CC0 仍需登记 LICENSES.md=OK 才可入构建（M4-029 接入）。
- 规模 L 的高风险项：双份着色器 + 视觉回归；若超一次会话，停在可提交稳定点并按 README §5.6 续作 `M4-014b`。
- 未核实：潮位/海况史料（H-077）；本任务只用静态参数，不在 UI 声称史实。
- 海平面与 `hmin` 基准（最低水面≈0 m，MAPS §1.2）必须一致，否则潮线整体偏移。
