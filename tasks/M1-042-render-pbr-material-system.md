# M1-042 render：PBR 材质系统与变体管理

## 元数据
- 里程碑：M1
- 工作流：client-render
- 规模：L
- 前置依赖：M0-020, M1-006
- 素材需求：V-NAT-007（CC0 地表材质，可选）；素材状态：placeholder（棋盘格/纯色材质即可，真材质由 M1-050/M1-064 接入）
- 引用文档：ARCHITECTURE §10.4; ARCHITECTURE §13.5.3; ARCHITECTURE §10.3; ARCHITECTURE §5.1

## 背景
M1 起所有可见物体（地形、建筑、角色、载具、植被、道具）统一走一套 metal-rough PBR 着色器。ARCHITECTURE §10.4 的硬指标：**全局统一 PBR 着色器 + 少量宏（`USE_NORMAL_MAP`、`USE_ALPHA_TEST`、`USE_VERTEX_WIND`、`USE_DETAIL`），总变体 ≤ 60（CI 统计）**；预热：加载界面期间用"虚拟网格"遍历所有变体绘制到 1×1 RT，完成后才允许进入部署界面，避免首次出现时卡顿；纹理流送先低 mip 后高 mip（Medium 最大 mip 限制 1024²）。双路径（§10.3）：WebGPU 用 WGSL（`ShaderMaterial`/`NodeMaterial`），WebGL2 用 GLSL ES 3.00 对应实现，同一效果双份着色器需视觉回归。WebGL2 渲染提交开销高 20–30%，因此材质需冻结（`freeze()`）并合批。

## 具体内容
### 要创建/修改的文件
- `client/src/render/materials/pbr.wgsl`、`pbr.glsl`（含宏）—— 统一 PBR：BRDF（GGX + Smith + Schlick）、法线贴图、ORM 打包（R=AO, G=Roughness, B=Metallic）、alpha test、顶点风动、细节法线。
- `client/src/render/materials/MaterialFactory.ts` —— `getMaterial(desc: MaterialDesc): Material`，以宏集合哈希为键缓存；重复请求返回同一实例。
- `client/src/render/materials/variants.ts` —— 变体注册表、计数器、`variantStats()`；超过上限（运行时 60 / Low 40）抛开发期错误。
- `client/src/render/materials/prewarm.ts` —— 预热遍历：对注册表中全部变体各绘 1 次到 1×1 RT，返回 `Promise<PrewarmReport>`（含耗时、变体数、覆盖率）；WebGPU 使用 `createRenderPipelineAsync`，WebGL2 使用 `KHR_parallel_shader_compile`（可用时）。
- `client/src/render/materials/placeholder.ts` —— `PH_{需求ID}` 材质工厂（阵营色、棋盘格，ASSET_GAPS §3.1）。
- `tools/ci/variant-count.ts` —— CI 脚本：静态枚举全部 `MaterialDesc` 组合并断言 ≤ 60。
- `client/test/render/materials.test.ts`（vitest）。
### 要实现的行为
1. 宏组合：4 个宏 × 阶段（opaque/alpha/depth-only/shadow）统计后总数 ≤ 60；Low 档 ≤ 40（通过禁用 `USE_DETAIL` 与法线细节）。
2. 材质冻结：静态物体材质创建后 `freeze()`；动态参数（阵营色、受击闪白）走 uniform/instance attribute，不触发新变体。
3. 预热：启动页（M1-054）调用 `prewarm()`；预热完成前 `ready=false`，部署界面据此等待。
4. mip 分层：纹理先绑定低 mip（≤ 256²），高 mip 异步补齐；Medium 档限制最大 1024²（读取 `quality` 设置，不硬编码）。
5. 双路径一致：同一 `MaterialDesc` 在 WGSL/GLSL 下输出的视觉回归基线（固定机位球体+标准材质）差异：平均 ΔE < 2。
### 接口与数据
```ts
export interface MaterialDesc { shading: 'pbr'; flags: { normalMap?: boolean; alphaTest?: boolean; vertexWind?: boolean; detail?: boolean };
  pass: 'opaque'|'alpha'|'depth'|'shadow'; placeholderId?: string /* PH_{需求ID} */ }
export function variantStats(): { count: number; byPass: Record<string, number> };
export async function prewarm(scene: Scene): Promise<PrewarmReport>;
```

## 不做什么
- 不做光照/阴影/天空/IBL（M1-043）、TAA/曝光（M1-044）。
- 不做地形 splat 材质（M1-050）、角色蒙皮材质变体之外的特殊着色（皮肤/布料不单设变体）。
- 不做 WebGPU 与 WebGL2 之外的降级；不做 SSR/体积雾（M2+）。
- 不做 CI 变体门禁的最终版（M4-021 完善；本任务只提供计数与基础 CI 步骤）。

## 验收标准
1. 全部合法 `MaterialDesc` 组合的变体总数 ≤ 60（`pnpm -C client run variant-count`，CI 步骤失败即阻断）；Low 配置 ≤ 40。
2. 预热完成后，首次出现新物体（遍历全部变体各一个实例）的单帧 CPU 帧时间增量 ≤ 50 ms（Playwright + 叠层导出，参考机）。
3. 预热总耗时：WebGPU ≤ 1.5 s、WebGL2 ≤ 3.0 s（参考机，冷缓存；报告写入 STATUS）。
4. WGSL 与 GLSL 对标准球体基线截图的平均 ΔE < 2（`pnpm -C client test:visual materials`）。
5. 材质冻结后，主视图 1 000 个 mesh 的 CPU 提交时间 ≤ 未冻结的 70%（微基准，WebGL2）。
6. 占位材质命名全部符合 `PH_{需求ID}`（单测遍历场景断言）。

## 测试要求
- 单元测试：宏哈希与缓存命中；变体上限异常；`PH_` 命名；mip 分层状态机。
- 集成/无头测试：Playwright 固定机位渲染 12 种材质球，两条路径分别截图。
- 性能基准：预热耗时与首次出现卡顿测量脚本（复用 M0-030 的叠层导出）。
- 回归：M0-023/M0-024 的地形/植被仍可渲染。

## 性能预算
- ARCHITECTURE §13.5.1：渲染提交 ≤ 4.5 ms（High），WebGL2 +2.5 ms；§13.5.3 材质变体 ≤ 60；§13.5.2 不透明主通道 ≤ 4.0 ms 为后续任务共享预算，本任务不得单独占用 > 0.3 ms 的材质开销。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、CI 作业 `variant-count`）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已列出
- [ ] 文档同步：宏集合若有增减，回写 ARCHITECTURE §10.4 并记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Babylon 的 NodeMaterial 在 WebGPU 与 WebGL2 的宏/变体行为不同，需在 M0-020 的能力探测基础上分别测试；若变体统计难以静态化，改为运行时记录并在 CI 的 Playwright 场景中读取。
- 着色器编译卡顿是 ARCHITECTURE §20 的已知风险，预热覆盖率不足会在 M4-021 门禁中暴露。
