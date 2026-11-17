# M3-048 测试：M3 视觉回归扩充

## 元数据
- 里程碑：M3
- 工作流：test
- 规模：M
- 前置依赖：M2-050, M3-029, M3-030, M3-031, M3-032
- 素材需求：无；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.4; ARCHITECTURE §17.5; ARCHITECTURE §13.5.3

## 背景
M2-050 建好了 Playwright + Chromium 视觉回归框架（ARCH §17.4）：固定机位、固定随机种子、关闭 TAA 抖动（或固定相位）、固定时间；`pixelmatch` 阈值 0.1% 差异像素 + 感知指标；CI 无 GPU 时使用 SwiftShader/软件 WebGPU（Dawn）或固定 WebGL2；两条路径分别维护基线；基线按"运行器类型"分开存放。M3 增加载具/空袭/炮击/击毁等动态内容，需要扩充机位并保证确定性。
作业：`e2e-visual`（PR 改 client/render 与 nightly）目标 < 20 min（ARCH §17.5）。

## 具体内容
### 要创建/修改的文件
- `client/tests/visual/m3/vehicles.spec.ts` —— 机位：1) 12 辆载具阵列（正面/侧面）；2) Tiger 与 Sherman 近景对比；3) 半履带与 Jeep（含乘员占位）；4) 轻型装甲（M8/Puma）行进中。
- `client/tests/visual/m3/airstrike.spec.ts` —— 机位：5) Bombing 条带飞机过顶；6) Strafing 落点溅射；7) 空投降落伞 + 彩色烟。
- `client/tests/visual/m3/artillery.spec.ts` —— 机位：8) 榴弹炮阵地全貌；9) HE 弹着 + 弹坑贴花；10) 烟雾云团遮蔽。
- `client/tests/visual/m3/destruction.spec.ts` —— 机位：11) 载具起火；12) 残骸；13) 殉爆瞬间（固定到事件后第 N 帧）。
- `client/src/debug/visScene.ts`（改）—— 调试场景 `?vis_scene=vehicles12|airstrike|artillery|destruction&seed=N&t=MS`：冻结游戏时钟、固定相机、固定粒子种子（FX seed 来自事件 id，M3-031）、禁用自动曝光漂移与 TAA 抖动。
- `client/tests/visual/__baselines__/{runner}/m3/*.png` —— 基线（按运行器类型分目录）。
- `.github/workflows/ci.yml`（改）—— `e2e-visual` 纳入 m3 目录。
### 要实现的行为
1. 每个机位以固定相机、固定种子、固定 `t` 渲染，等待 `scene.isReady()` 与 N 帧稳定后截图。
2. 差异阈值：`pixelmatch` 差异像素 ≤ 0.1%（沿用 M2 配置）；粒子类机位可在配置中提高到 0.5% 并说明原因。
3. 不同运行器（软件 WebGPU/WebGL2/真 GPU）分别维护基线，不混用。
4. 基线更新流程：`pnpm test:visual --update` 只在显式参数下运行；PR 中基线变化需附人工审阅说明。
5. 失败时输出 diff 图与报告产物（artifact）。
6. 随机/时间源检查：对每个机位连续运行 3 次，像素差异 ≤ 阈值（稳定性）。
### 接口与数据
- 机位清单文件 `client/tests/visual/m3/shots.json`：`{id, scene, seed, t_ms, camera:{pos,target,fov}}`。

## 不做什么
- 三图 × 12 机位 × 3 时段 × 四档画质（M4-040）。
- HUD/UI 视觉回归（可选 1 机位，已在 M3-033/036 备注；全量归 M4）。
- 修复渲染缺陷（发现后写 PLAN_ISSUES）。

## 验收标准
1. 新增 ≥ 8 个机位基线入库（目标 13 个，见上），每个运行器类型一套；`ls client/tests/visual/__baselines__/*/m3 | wc -l` ≥ 8 × 运行器数。
2. 稳定性：每机位连续 3 次运行的差异像素比 ≤ 0.1%（粒子机位 ≤ 0.5%，已在 `shots.json` 注明）。
3. CI 差异阈值生效：故意改一个材质颜色的 PR 使 `e2e-visual` 失败（验证脚本/PR 记录），回滚后通过。
4. `e2e-visual` 总时长 < 20 min（含 M2 机位 + M3 新机位；超出则拆分并写 PLAN_ISSUES）。
5. 无外部时间依赖：`Date.now()`/`performance.now()` 不进入渲染（vitest 检查调试场景的时钟注入）。
6. 失败时自动上传 diff 图 artifact（检查 workflow 配置）。

## 测试要求
- 单元：`visScene` 参数解析、时钟冻结。
- 集成：Playwright 全量。
- 性能基准：作业耗时记录。
- 回归：M2 基线不应变化（M3 改动不得引入 M2 机位差异；有差异需解释）。

## 性能预算
- CI `e2e-visual` < 20 min（ARCH §17.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 本任务依赖 M3-029/030/031/032（击毁机位 11–13 依赖 M3-032，依赖已补）。
- 软件渲染的粒子差异大；必要时对粒子机位使用"区域掩膜"。
- 真素材接入（M3-041/044）后基线必须更新并人工审阅。
