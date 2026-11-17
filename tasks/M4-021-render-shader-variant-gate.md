# M4-021 render：着色器变体与预热门禁

## 元数据
- 里程碑：M4
- 工作流：client-render
- 规模：S
- 依赖：M1-042（PBR 材质系统与变体管理）, M4-017
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §10.4; ARCHITECTURE §20; ARCHITECTURE §13.5.3; ARCHITECTURE §17.5

## 背景
ARCHITECTURE §10.4：全局统一 PBR 着色器 + 少量宏（`USE_NORMAL_MAP`、`USE_ALPHA_TEST`、`USE_VERTEX_WIND`、`USE_DETAIL`）；**总变体 ≤ 60（硬指标，CI 统计）**；Low 档 ≤ 40（§13.5.3）。预热：加载界面期间用"虚拟网格"遍历所有变体绘制到 1×1 RT，完成后才允许进入部署界面，避免首次出现卡顿（§20 第 4 项）。M1-042 已有材质系统与变体管理；M4 新增海洋、雪地、SSR、粒子等，必须在门禁下保持上限。
需要实现的三件事：
1. **变体计数 CI 门禁**：统计 `(shader, defines)` 唯一组合数，按档位（Low/Medium/High/Ultra × WebGPU/WebGL2）分别计数；超限 PR 失败。
2. **预热覆盖率检查**：运行时记录"实际使用过的变体集合"与"预热过的集合"，两者之差必须为空（覆盖率 100%）。
3. **首次出现卡顿检测**：运行时对"首次使用某变体的帧"监测帧时间尖峰（> 2× 目标帧时间且该帧首次出现变体），在开发/内部构建报警；CI 巡游脚本断言 0 次。

## 具体内容
### 要创建/修改的文件
- `tools/shader-variants/count.ts` —— 静态/运行时枚举变体（Playwright 巡游脚本驱动）并输出 `reports/shader-variants.json`。
- `client/src/render/material/VariantRegistry.ts` —— 唯一入口：所有材质通过它注册变体 key；开发构建在运行时新增未预热变体时 `console.error` 并计数。
- `client/src/render/material/Warmup.ts` —— 更新预热：遍历 registry（含 M4-014/M4-015/M4-019 新增变体）。
- `.github/workflows/ci.yml`（或 `shader-variants.yml`）—— 新作业 `shader-variants`：PR 改 `client/src/render/**` 或 `data/render/**` 时运行。
- `data/render/variant_budget.json` —— `{low:40, medium:60, high:60, ultra:60}`（与 §13.5.3 一致，`initial`）。
- `client/tests/render/variants.spec.ts`。
### 要实现的行为
1. 巡游脚本：Playwright 依次加载三张地图（灰盒）× 四档 × 两条路径的固定机位序列，收集 registry 的"使用过的变体"；输出各档计数。
2. 门禁：任一档计数 > 预算 → 作业失败并打印新增变体的 key 与首个触发材质名。
3. 预热覆盖率：预热完成时记录集合 `W`；巡游结束时使用集合 `U`；断言 `U ⊆ W`。
4. 卡顿检测：为"首次使用变体"事件打 `performance.mark`，与帧时间关联；巡游中尖峰数 = 0。
5. 报告产物上传 CI artifacts；`tasks/STATUS.md` 引用报告路径。
### 接口与数据
- `registerVariant(shaderId: string, defines: string[]): VariantKey`；`getVariantCount(tier?: QualityTier, backend?: 'webgpu'|'webgl2'): number`；`reports/shader-variants.json` 结构：`{generated, by_tier:{low:{webgpu:n,webgl2:n},…}, variants:[{key,first_material}]}`。

## 不做什么
- 不优化具体着色器的 ALU 成本；不新增宏。
- 不做 WebGPU 管线缓存持久化（Backlog）。
- 不做着色器编译耗时遥测上报（M4-036）。

## 验收标准
1. 门禁生效：故意添加第 61 个变体的 PR 在 CI 上失败（用 `pnpm -C client test:variants -- --inject-extra` 本地验证退出码 ≠ 0），并输出新增 key。
2. 当前代码库计数：Low ≤ 40，其余 ≤ 60（`reports/shader-variants.json` 实测数字写入 STATUS）。
3. 预热覆盖率 100%：巡游后 `U \ W = ∅`（运行时记录）。
4. 首次出现卡顿：巡游 5 min（三图×固定机位序列）中"首次变体帧时间 > 2× 目标"事件数 = 0（参考机；CI 软件渲染仅验证流程）。
5. 新增变体后预热时长增量 ≤ 0.5 s/10 个变体（记录，非阻断）。

## 测试要求
- 单元测试：`vitest`：registry 去重、计数、按档位统计；预热集合差集逻辑。
- 集成/无头测试：Playwright 巡游脚本（软件渲染可跑）；注入额外变体的负向用例。
- 性能基准：无。
- 回归：`variant_budget.json` 作为基线，改动须在 PR 说明。

## 性能预算
- 材质变体 ≤ 60（Low ≤ 40）（§10.4/§13.5.3）；门禁作业 ≤ 10 min（CI 总目标 §17.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 若实测变体已超预算（海洋/雪/SSR 叠加），不得放宽门禁：写 PLAN_ISSUES 并由相应任务（M4-014/015/019）收敛变体。
- 驱动更新后缓存失效，首次启动仍会较慢（§20 第 4 项残余风险），门禁只保证"预热后无首次卡顿"。
- 本任务与 M2-050 的视觉回归共用巡游机位；机位清单以 `tests/e2e/camera_poses.json` 为唯一来源。
