# M4-015 world：冬季雪地与天气

## 元数据
- 里程碑：M4
- 工作流：world
- 规模：L
- 依赖：M4-010, M2-036（全图流式与内存预算）
- 素材需求：V-NAT-003; V-SKY-002; V-FX-*（灰盒可）；素材状态：placeholder
- 引用文档：MAPS §5.5; MAPS §5.7; ARCHITECTURE §9.8; ARCHITECTURE §9.6; ARCHITECTURE §10.2; ARCHITECTURE §10.4; ARCHITECTURE §13.5.2; ARCHITECTURE §13.5.3

## 背景
Foy 为 1945-01 的冬季战场：严寒、积雪 20–40 cm（**未核实**）、雾与低云、偶有炮击烟；时段可选黎明 08:00（冷蓝色调）/正午/黄昏 16:30（MAPS §5.5）。本任务实现雪地材质、积雪植被变体、降雪/雾粒子、冬季天空与光照预设。
关键约束：
- 材质：积雪、冻土、结冰路面、雪覆盖木头/树枝、石墙、木板屋（MAPS §5.5）；雪面使用统一 PBR + 噪声，避免大面积高光；SSAO/GTAO 在林内易噪声，Low 关闭（MAPS §5.7）。
- 植被：针叶林（云杉）使用 impostor 与 LOD；树 LOD0 ≈ 2–4k tri、LOD1 ≈ 800 tri、八面体 impostor（ARCHITECTURE §9.6）；积雪通过"材质变体/顶点色/世界法线遮罩"实现，**不得新增大量独立着色器变体**：总变体 ≤ 60（Low ≤ 40），ARCHITECTURE §10.4/§13.5.3。
- 降雪粒子：粒子预算 Low 300 / Medium 800 / High 1600 / Ultra 3000（§13.1 粒子上限；降雪占其一部分）；INDEX 指标：降雪粒子 ≤ 0.7 ms。
- 雾：WebGPU 用 froxel（M2-031），WebGL2 退化为高度雾；Foy 能见度 450–600 m（MAPS §5.6）。
- 天空/IBL：每地图 2–3 个固定时段预设，IBL 立方体 256² KTX2（ARCHITECTURE §9.8）；V-SKY-002 冬季/雪天 HDRI 来自 Poly Haven CC0（具体文件未核实，需 LICENSES.md 登记）。
- 脚印为可选（装饰贴花，可砍）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/snow/SnowSurfaceMaterial.ts` —— 地形 splat 中 `snow/packed_snow/frozen_road` 层的材质扩展（共享主 PBR 着色器的宏位，不新增独立变体）。
- `client/src/render/snow/SnowVegetation.ts` —— 植被积雪变体（顶点色/法线朝上权重）。
- `client/src/render/weather/SnowfallSystem.ts` —— 降雪粒子（GPU instancing/粒子系统；预算按档位读取）。
- `client/src/render/sky/WinterPresets.ts` + `data/render/foy_presets.json` —— 3 个时段预设（颜色、太阳角度、雾密度、曝光），状态 `initial`。
- `client/src/render/weather/WeatherController.ts` —— 天气状态（clear/overcast/snow/fog_snow），由服务器时段/地图配置决定，不做随机变化以保证视觉回归。
- `data/maps/foy/weather.json` —— 默认预设与可选预设。
- `client/tests/render/winter.visual.spec.ts`。
### 要实现的行为
1. 雪地：splat 的积雪权重 + 法线遮罩；湿/冰路面降低粗糙度；沿 `snow_depth_zones.json`（M4-010）渲染雪深视觉（仅视觉，减速规则在 sim）。
2. 植被：云杉 LOD0/LOD1/impostor 的积雪变体；风动物体使用 `motion_mask`（ARCHITECTURE §10.1）。
3. 降雪：粒子按 `render_scale` 无关的世界空间体积生成；Low 档粒子数 ≤ 300 并关闭光照；风向随时间缓变（确定性，种子固定）。
4. 时段预设切换（3 个），IBL 立方体预烘焙与加载；不做实时昼夜变化。
5. 雾：与 M2-031 对接；WebGL2 路径高度雾 + 雾颜色随预设。
6. 灰盒：缺 V-NAT-003/V-FX 素材时用锥体树 + 方片粒子（`PH_V-NAT-003`、`PH_V-FX-*`），不引入外部贴图。
### 接口与数据
- `interface WeatherPreset { id: string; sun_elev_deg: number; sun_azi_deg: number; fog_density: number; visibility_m: number; snowfall_rate: number; exposure: number; ibl_ref: string }`，数值在 `data/` 内，客户端不硬编码（ARCHITECTURE §4.4）。

## 不做什么
- 不做雨/夜间/动态昼夜；不做天气同步协议（所有客户端使用同一静态预设，由地图配置决定）。
- 不改 sim 的雪深减速/陷入规则（若需要，写 PLAN_ISSUES）。
- 不做音频（M4-028）；不做素材接入（M4-029）。
- 不做脚印（可选项，若时间不足不做并在备注说明）。

## 验收标准
1. 降雪粒子系统 GPU+CPU 合计 ≤ 0.7 ms（High，固定机位 `foy_forest_edge_01`，1080p）；Low 档粒子数 ≤ 300，Medium ≤ 800（计数器断言）。
2. 植被材质变体加入后，总材质变体仍 ≤ 60，Low 档 ≤ 40（`pnpm -C client test:variants` 通过）。
3. 三个时段预设下 Foy 机位截图与基线差异 ≤ 0.1% 像素（Playwright 视觉回归，两条路径各自基线）。
4. WebGL2 路径下 Foy 林缘机位 ≥ 30 fps（Low，Iris Xe 720p，P95；缺硬件标 `未测`）。
5. 雾能见度验证：距离 600 m 外的目标对比度 ≤ 5%（脚本在固定机位测量），450 m 处 ≥ 15%（初值，记录实测）。
6. 树 LOD 切换无 popping 肉眼可见跳变（M2-050 视觉回归的 LOD 机位序列 SSIM ≥ 0.98）。

## 测试要求
- 单元测试：`vitest`：粒子预算按档位裁剪；预设 JSON schema 校验；天气控制器确定性（同种子同序列）。
- 集成/无头测试：Playwright 固定机位（`foy_*` 12 个，3 时段）截图；`pnpm -C client bench:weather` 输出每档 ms。
- 性能基准：`bench:weather` 基线文件 `reports/perf/m4-weather.json`。
- 回归：视觉基线并入 M4-040；变体统计并入 M4-021 门禁。

## 性能预算
- 降雪 ≤ 0.7 ms；粒子上限遵守 §13.1；GPU 总预算 12.0 ms（§13.5.2）与变体 ≤ 60/40（§10.4）不得回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 严寒/积雪厚度/气象数值**未核实**（MAPS §5.5），只作为美术参数，不对外声称史实。
- 规模 L：如超一次会话，按 README §5.6 拆 `M4-015b`（脚印/可选项最后做）。
- 冬季角色（V-CHR-US-004/V-CHR-GE-003）为 GAP-V04，与本任务无关，接入在 M4-029。
- 依赖 M4-010 的 `snow_depth_zones.json`；若 Foy 数据因 H-072 仅有占位地形，视觉调参结论标 `partial`。
