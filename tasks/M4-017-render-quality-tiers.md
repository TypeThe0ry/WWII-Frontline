# M4-017 render：四档画质预设

## 元数据
- 里程碑：M4
- 工作流：client-render
- 规模：L
- 依赖：M2-032（TAAU）, M2-030（GTAO/SSAO）, M2-031（体积雾）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §13.1; ARCHITECTURE §13.5.3; ARCHITECTURE §10.3; ARCHITECTURE §10.4; ARCHITECTURE §13.3; UI_SPEC §4.16

## 背景
把 ARCHITECTURE §13.1 的 Low/Medium/High/Ultra 四档参数表落成**数据驱动的预设**（`/data/render/quality_tiers.json`，客户端不硬编码数值，ARCHITECTURE §4.4），并支持运行时热切换。
§13.1 关键参数（逐项照表写入数据，单位见原表）：
- 目标：Low 720p/30 fps（核显/WebGL2）、Medium 1080p 输出内部 67% 45–60 fps、High 1080p 60 fps（基准 RTX 3060）、Ultra 1440p 60 fps。
- 内部渲染比例：Low 67–75% / Medium 67%（动态 60–85%）/ High 100%（动态 75–100%）/ Ultra 100%（动态 67–100%）。
- 超分：Low FSR1 式、Medium/High TAAU、Ultra TAAU 质量模式；AA：Low FXAA/轻 TAA、其余 TAA。
- 阴影：Low 3 级×1024 PCF 1×1 距离 150 m；Medium 4 级×1024–2048 PCF 3×3 250 m；High 4 级×2048/1024 软阴影 350 m；Ultra 4 级×2048–4096 PCSS 近似 500 m。
- AO：关 / SSAO 半分辨率 / GTAO / GTAO 全分辨率。体积雾：高度雾 / froxel 80×45×48 / 160×90×64 / 240×135×96。SSR：仅 Ultra（M4-019 实现）。
- 植被密度/距离：草 35%/20 m、65%/35 m、100%/45 m、130%/60 m；道具密度 40/70/100/120%；纹理最大 mip 512²/1024²/2048²/2048²（建筑 4096²）；粒子上限 300/800/1600/3000；视距 1.5/3/5/8 km；动态灯 2/4/8/12；音频并发 32/48/64/64。
- 场景复杂度预算（§13.5.3）：Low draw call ≤ 500、可见三角形 ≤ 0.8 M、材质变体 ≤ 40；High ≤ 1200、≤ 4.0 M、≤ 60。

## 具体内容
### 要创建/修改的文件
- `data/render/quality_tiers.json` + `/data/schemas/render/quality_tiers.schema.json` —— 四档完整参数（状态 `initial`）。
- `client/src/render/quality/QualityManager.ts` —— 读取预设、应用到各 pass、发 `quality:changed` 事件。
- `client/src/render/quality/applyTier.ts` —— 每个子系统的映射函数（shadow/ao/fog/ssr/veg/prop/mip/particle/lights/viewdist）。
- `client/src/render/quality/capabilities.ts` —— 与 M0-020 的能力探测对接，决定某项在 WebGL2 下是否可用（不可用项标原因，不隐藏，UI_SPEC §4.16.4）。
- `client/tests/render/quality.spec.ts`、`client/tests/render/quality.visual.spec.ts`。
### 要实现的行为
1. `QualityManager.setTier(tier, {persist})`：同步应用全部参数；热切换 ≤ 2 s 内完成（含 shader 预热增量），期间不丢输入。
2. 自定义项：单项覆盖后档位显示"自定义"（UI 在 M4-024）；覆盖项保存到 `localStorage`（try/catch，失败则会话内有效）。
3. WebGL2 约束：无 compute 的项（froxel、TAAU、Hi-Z）自动回退：体积雾→高度雾、TAAU→FSR1 式、阴影 3 级；回退不改写预设档名。
4. 变体守卫：切档不得引入新的材质变体；统计 Low 档变体 ≤ 40，其余 ≤ 60（对接 M4-021 门禁；本任务提供计数接口 `getVariantCount()`）。
5. 预估显存：每档给出估算值（对接 M4-022）。
6. 预设数据校验：缺字段/越界值（如阴影级联数 > 4）在 `data-validate` 中报错。
### 接口与数据
- `type QualityTier = 'low'|'medium'|'high'|'ultra'`；`interface TierParams { render_scale:{min:number,max:number,start:number}; upscaler:'fsr1'|'taau'|'taau_quality'; shadows:{cascades:number,res:number[],pcf:string,dist_m:number}; ao:'off'|'ssao_half'|'gtao'|'gtao_full'; fog:{mode:'height'|'froxel',dims?:[number,number,number]}; ssr:boolean; veg:{grass_density:number,grass_dist_m:number,tree_mode:string}; prop_density:number; max_mip:number; particles_max:number; view_dist_km:number; dyn_lights:number; audio_voices:number }`。

## 不做什么
- 不做动态分辨率控制器与自动推荐（M4-018）。
- 不实现 SSR（M4-019）、WebGL2 优化（M4-020）、变体门禁（M4-021）、显存预算驱逐（M4-022）。
- 不做设置 UI（M4-024）。

## 验收标准
1. 四档截图与参数表一致：Playwright 在固定机位对四档各截图，并读取 `QualityManager.snapshot()`，与 `quality_tiers.json` 逐字段相等（单测 100% 字段覆盖）。
2. 高档参数不影响 Low 的变体数：Low 档 `getVariantCount() ≤ 40`；Medium/High/Ultra ≤ 60（CI 断言）。
3. 热切换 Low↔Ultra 往返 10 次，每次生效 ≤ 2 s（`performance.now()` 测量，P95），无 console error。
4. Low 档场景复杂度：Carentan 固定机位 draw call ≤ 500、可见三角形 ≤ 0.8 M（`renderer.stats` 记录；超出写 PLAN_ISSUES，由 M4-020 优化，本任务只记录）。
5. 校验：故意写入越界预设，`data-validate` 失败（单测）。
6. WebGL2 回退矩阵单测：froxel/TAAU/Hi-Z 在 WebGL2 下回退行为与 ARCHITECTURE §10.3 表一致（逐行断言）。

## 测试要求
- 单元测试：`vitest`：预设 schema、`applyTier` 映射、回退矩阵、`localStorage` 不可用时的行为。
- 集成/无头测试：Playwright 四档 × 3 机位截图（基线并入 M4-040）；`pnpm -C client bench:quality-switch`。
- 性能基准：每档渲染 ms 采集脚本沿用 `bench:frame`，输出 `reports/perf/m4-tiers.json`（非阻断，实测在 M4-041）。
- 回归：预设哈希写入 `DATA_HASH`；视觉基线。

## 性能预算
- ARCHITECTURE §13.5.3 场景复杂度预算与 §10.4 变体上限为本任务硬指标；GPU/CPU 帧预算（§13.5.1/§13.5.2）不得回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L：若超一次会话，按 README §5.6 停在稳定点并拆 `M4-017b`（建议拆分线：渲染 pass 映射 / 数据与校验）。
- Medium GPU 预算（≈ 16 ms）需 M4 实测校准（ARCHITECTURE §13.5.2 注）；本任务只负责参数落地。
- H-039（最低配置决定）可能改变 Low 档目标（默认：未达标则 Low 降为 720p/25–30 fps 并标注"最低体验"），数据表须允许调整，不改代码。
