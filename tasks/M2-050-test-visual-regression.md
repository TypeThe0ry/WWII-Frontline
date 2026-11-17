# M2-050 测试：视觉回归（Playwright）

## 元数据
- 里程碑：M2
- 工作流：test
- 规模：L
- 前置依赖：M2-032, M2-036, M2-030, M2-031
- 素材需求：无（灰盒即可）；素材状态：n.a.（基线随素材接入更新）
- 引用文档：ARCHITECTURE §17.4; ARCHITECTURE §17.5; ARCHITECTURE §10.3; ARCHITECTURE §10.2; MILESTONES §3.3

## 背景
ARCHITECTURE §17.4：Playwright + Chromium（启用 WebGPU 的软件/GPU 运行器；CI 无 GPU 时使用 SwiftShader/软件 WebGPU（Dawn）或固定 WebGL2 路径，**两条路径分别维护基线**）；固定机位：每张地图 12 个机位 × 3 时段 × 4 画质档（子集），固定随机种子、关闭 TAA 抖动（或固定相位）、固定时间；像素比较（`pixelmatch`，阈值 **0.1% 差异像素** + 感知指标），差异产出报告产物；软件渲染与真 GPU 结果有差异，基线按"运行器类型"分开存储；跨 GPU 厂商差异用夜间 job（若有自托管 GPU runner）。
M2 退出标准 E2-12：12 机位 × 2 路径基线入库，CI 差异阈值 0.1% 像素。M2 子集定义：Carentan，1 个时段（清晨），High 配置（WebGPU）与 Medium 配置（WebGL2）；时段/画质档扩展留 M4（M3-048 追加机位、M4-017 四档）。CI job `e2e-visual`（PR 改 `client/render` 与 nightly，目标 < 20 min，§17.5）。依赖 M2-030/031/032 的 AO/雾/TAAU 固定相位开关与 M2-036 的流式（机位就绪等待"区块全部 Resident 且洞 = 0"）。

## 具体内容
### 要创建/修改的文件
- `client/tests/visual/cameras.json` —— 12 个固定机位（位置/朝向/FOV/时间/天气种子）：老城街道纵深、教堂广场、堤道（N13）、树篱田（西南）、沼泽浅滩、东侧运河、开阔草地、建筑室内（一楼入口）、HQ 区域、据点周边（L3 候选）、烟雾/爆炸特效场景、载具（Panzer IV 灰盒）与步兵同屏。
- `client/tests/visual/visual.spec.ts` —— Playwright 测试：加载 Carentan 灰盒（`?visual=1&seed=…&phase=0`）、等待就绪（`window.__visualReady`：区块 Resident、着色器预热完成、洞 = 0、TAA 历史稳定 N 帧）、截图。
- `client/src/debug/visual_mode.ts` —— 视觉测试模式：固定种子、冻结时间/天气/粒子随机、固定 TAA 相位、禁用动态分辨率、固定 `render_scale`、屏蔽 DOM UI（只测 3D 画面；UI 截图另案）。
- `tools/visual/compare.ts` —— `pixelmatch` 对比 + 感知指标（SSIM）+ HTML 报告（差异图叠加）；阈值：差异像素 > 0.1% 即失败（单机位）。
- `tests/visual/baselines/{webgpu-sw,webgl2-sw,webgpu-gpu}/*.png` —— 基线（按运行器类型分存）；`tests/visual/baselines/README` 不创建，规则写入 STATUS。
- `.github/workflows/e2e-visual.yml` —— PR（改 `client/src/render/**`、`tests/visual/**`）与 nightly；上传差异报告 artifact；基线更新走专门命令 `pnpm visual:update`（需人工审查）。
### 要实现的行为
1. 两条路径基线：`webgpu`（SwiftShader/软件 Dawn 或 GPU runner）与 `webgl2`；CI 默认运行器类型 `sw`；自托管 GPU runner 时另存 `gpu` 基线（H-014 决定）。
2. 12 机位 × 2 路径 = 24 张基线入库；每张 1920×1080（或 1280×720 以减小体积，需在 STATUS 记录）PNG。
3. 稳定性：同一提交连续运行 5 次，所有机位差异像素占比 ≤ 0.02%（抖动余量）；超过则机位被标记为不稳定并修复（固定种子/相位）。
4. 报告：失败时产出 `reports/visual/index.html`，列出机位、差异像素占比、SSIM、差异图；CI artifact 保留 14 天。
5. 基线更新流程：只能通过 `visual:update` 本地生成，提交 PR 时需在描述写明原因；CI 不自动更新基线。
### 接口与数据
```json
{ "id": "cam_oldtown_01", "pos": [900,12,1150], "yawDeg": 135, "pitchDeg": -3, "fov": 80, "timeOfDay": "06:00", "seed": 1944, "tags": ["culling","fog","taau"] }
```

## 不做什么
- 不做多时段与四档画质基线（M4-017/M3-048 追加）；不做跨 GPU 厂商矩阵（夜间 GPU runner 视 H-014）。
- 不做 UI/HUD 截图回归（UI 有各自的 Playwright 用例）。
- 不做性能采集（M2-052）。
- 不自动更新基线。

## 验收标准
1. 基线入库：12 机位 × 2 路径 = 24 张基线 PNG 已提交，`cameras.json` 12 条（脚本检查文件数 = 24）；仓库增量体积 ≤ 60 MB（`du`，若超标改用 720p 并记录）。
2. 稳定性：同一提交、同一运行器连续 5 次，24 张全部差异像素 ≤ 0.02%（`pnpm visual:stability`）。
3. 故意回归检测：人为修改一个着色器常数（如雾密度 ×1.2）后 `e2e-visual` 作业失败，且报告指出受影响机位（演练记录，差异 > 0.1% 的机位 ≥ 1）。
4. 阈值生效：`compare.ts` 单测——差异像素占比 0.09% 通过、0.11% 失败；SSIM 同时输出。
5. CI 时长：`e2e-visual` 作业墙钟 < 20 min（ARCH §17.5），记录实际分钟数。
6. 基线按运行器类型分存：`webgpu-sw`/`webgl2-sw` 目录结构单测；运行器类型错配时作业给出明确错误而非误报差异。
7. 视觉测试模式下确定性：关闭 TAA 抖动或固定相位后，同机位两次截图逐像素完全相等的机位比例 ≥ 90%（其余需在报告列出非确定性来源）。

## 测试要求
- 单元测试：`compare.ts`（阈值、尺寸不一致报错）；`visual_mode` 种子/相位固定；`cameras.json` schema。
- 集成/无头测试：本任务即 e2e；Chromium 启用 `--enable-unsafe-webgpu`（软件/GPU 视运行器）与 `--use-gl=swiftshader`；记录 Chromium 版本（版本升级会改变基线，需在 STATUS 记录并锁定版本）。
- 性能基准：作业时长；单机位截图耗时。
- 回归：M2-030/031/032/034/035/033 各自的机位已覆盖（`tags` 与这些任务的验收对照）。

## 性能预算
- CI 目标时长 < 20 min（ARCH §17.5）；不影响产品运行时预算。基线仓库体积 ≤ 60 MB。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（Playwright `e2e-visual`、`vitest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 100 人场景下基线不稳（MILESTONES §3.8）：视觉测试只用静态灰盒场景，不含 bot/网络。
- 软件 WebGPU 渲染 Compute 效果（froxel/TAAU/GTAO）可能极慢或不可用；若 SwiftShader 不支持，WebGPU 基线只能在 GPU runner 生成（H-014 自托管 runner 决定），此时 `webgpu-sw` 基线标 `未测`，任务 `partial`。
- 素材接入（M2-047）后基线需整体更新并人工确认。
- 基线 PNG 入 git 会增大仓库，必要时改用 git-lfs（需人类批准，写 PLAN_ISSUES）。
