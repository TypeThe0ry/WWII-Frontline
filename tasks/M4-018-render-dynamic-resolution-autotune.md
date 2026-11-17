# M4-018 render：动态分辨率与自动推荐

## 元数据
- 里程碑：M4
- 工作流：client-render
- 规模：M
- 前置依赖：M4-017
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §13.2; ARCHITECTURE §13.6; ARCHITECTURE §13.1; UI_SPEC §4.16; UI_SPEC §4.1

## 背景
ARCHITECTURE §13.2 规定动态分辨率与自适应：
- 目标帧时间 `T_target`（30 fps → 33.3 ms；60 fps → 16.7 ms）。每 **0.5 s** 用 GPU 时间（WebGPU timestamp query，若可用；否则用帧间隔估计）计算：`scale += clamp((T_target*0.9 - T_gpu)/T_target, -0.05, +0.02)`，限幅于档位区间（Low 67–75%、Medium 60–85%、High 75–100%、Ultra 67–100%，§13.1）。
- 若**连续 10 s** 低于目标且已触底 → **提示**降档；**不自动降档**（避免抖动，只推荐）。
- 首次运行：**5 秒基准场景**（固定镜头）→ 推荐档位；用户可改。
§13.6 能力探测：`navigator.gpu.requestAdapter()` → `adapter.info`；限制项（`maxTextureDimension2D`、`maxStorageBufferBindingSize`）；WebGL2 `MAX_TEXTURE_SIZE` 与扩展；`navigator.deviceMemory`（Chromium 上报最大 8）；`hardwareConcurrency`；5 s 基准。结果**仅用于推荐，不上传**（隐私，§19）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/quality/DynamicResolution.ts` —— 控制器（纯函数 `step(state, t_gpu, cfg)` + 薄封装）。
- `client/src/render/quality/GpuTimer.ts` —— WebGPU timestamp query 封装；不可用时帧间隔估计（带平滑）。
- `client/src/render/quality/Benchmark.ts` —— 5 s 固定镜头基准（固定机位、固定相位抖动、固定种子），输出 `BenchResult{avg_ms,p95_ms,fps}`。
- `client/src/render/quality/recommend.ts` —— 能力 + 基准 → 推荐档位（规则表读自 `data/render/recommend_rules.json`）。
- `data/render/recommend_rules.json` —— 阈值表（`initial`）：例如 P95 ≤ 阈值 → 档位；iGPU 判定词表；deviceMemory 规则。
- `client/tests/render/dynres.spec.ts`。
### 要实现的行为
1. 控制器按 §13.2 公式每 0.5 s 调整 `render_scale`，并保证单次变化幅度（−5% / +2%）与区间限幅；切场景/暂停时冻结 1 s。
2. 触底检测：连续 10 s `T_gpu > T_target` 且 `scale == min` → 发 `quality:suggest_downgrade` 事件（UI 提示在 M4-024），**绝不自动改档位**（单测断言）。
3. 基准：进入固定场景 5 s（预热 1 s 不计入），收集帧时间（GPU timestamp 优先）；输出推荐档位及依据（能力 + 实测）。
4. 推荐是建议，不写入设置，除非用户确认；基准结果仅本地保存（`localStorage` try/catch），不上传（M4-036 遥测另有同意流程）。
5. 切后台（`visibilityState !== 'visible'`）暂停统计，避免误降。
6. `render_scale` 变化须与 TAAU/FSR1 路径兼容（输出分辨率不变，内部分辨率变），并避免每帧重建 RT：量化到 5% 档（8 个离散值）后复用缓存的 RT。
### 接口与数据
- `interface DynResState { scale: number; lowStreakMs: number; atFloor: boolean }`；`function step(s: DynResState, tGpuMs: number, targetMs: number, band: [number,number], dtMs: number): DynResState`；事件 `quality:suggest_downgrade{tier}`、`quality:recommendation{tier,reason}`。

## 不做什么
- 不做设置界面与基准向导 UI（M4-024）。
- 不上传基准/遥测（M4-036）。
- 不做自动降档（设计明确禁止）。
- 不改四档参数表（M4-017）。

## 验收标准
1. 压力场景帧时间回到目标 ≤ 3 s：在 High 档构造 +40% GPU 负载（额外全屏 pass 的合成压力），`render_scale` 调整后 `T_gpu ≤ 0.9*T_target` 的恢复时间 ≤ 3 s（Playwright 脚本 `pnpm -C client bench:dynres`，参考机；缺硬件标 `未测`）。
2. 推荐不自动降档：`vitest dynres.spec.ts` 断言触底 10 s 后只发建议事件，`QualityManager.tier` 不变。
3. 公式单测：给定 `(T_gpu, T_target)` 序列，输出 `scale` 与手算值误差 < 1e-6；限幅（−0.05/+0.02、区间上下限）逐边界断言。
4. 基准确定性：同机器连续 3 次 5 s 基准的平均帧时间相对偏差 ≤ 10%（需机器；缺则 `未测`）。
5. RT 缓存：连续 60 s 抖动负载下 `render_scale` 离散值 ≤ 8 种，RT 重建次数 ≤ 8（计数器断言）。
6. 切后台再回来 5 s 内不触发降档建议（单测）。

## 测试要求
- 单元测试：控制器纯函数；`recommend` 规则表（合成能力对象，覆盖 iGPU/dGPU/低内存/无 WebGPU）。
- 集成/无头测试：Playwright 在 WebGPU 与 WebGL2 路径各跑一次 5 s 基准（CI 软件渲染仅验证流程，不验证数字）。
- 性能基准：控制器自身开销 ≤ 0.05 ms/帧（`performance.measure`）。
- 回归：无视觉基线变化（`render_scale` 固定时输出与 M4-017 一致）。

## 性能预算
- 控制器 CPU ≤ 0.05 ms/帧；timestamp query 读回异步，不得造成 GPU 管线停顿；总预算 §13.5.1 的"GC/杂项 1.0 ms"内。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- WebGPU timestamp query 需要 `timestamp-query` feature，且部分浏览器量化精度降低（侧信道缓解），不可用时回退帧间隔估计，精度较差；两者结果均须记录来源。
- GPU 信息可能被浏览器隐藏（UI_SPEC §4.1.1），推荐规则必须在信息缺失时退化为仅基于基准。
- 硬件依赖：参考机（H-021）缺失时，验收 1/4 记 `未测`，任务 `partial`。
