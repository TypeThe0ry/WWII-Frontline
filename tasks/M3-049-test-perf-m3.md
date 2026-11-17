# M3-049 测试：M3 性能基准

## 元数据
- 里程碑：M3
- 工作流：test
- 规模：M
- 前置依赖：M2-052, M3-029, M3-032, M3-031
- 素材需求：无；素材状态：n.a.
- 引用文档：ARCHITECTURE §13.5; ARCHITECTURE §13.5.1; ARCHITECTURE §13.5.2; ARCHITECTURE §13.5.3; ARCHITECTURE §13.3; ARCHITECTURE §20.1

## 背景
M0-030/M1-069/M2-052 建立了固定机位性能采集脚本（Playwright + 帧时间/GPU timestamp 叠层，M0-020）。M3 加入载具、空袭、炮击、击毁，需要三类新场景的基准，并与 ARCH §13.5 预算对照。预算（High，RTX 3060，1080p，60 fps = 16.67 ms/帧）：
- CPU 主线程合计 ≈ 10.4 ms（输入 1.5、快照解码+插值 0.6、渲染提交 4.5、流式 1.5、音频 0.5、UI 0.8、GC 1.0）。
- GPU ≈ 12.0 ms，目标 ≤ 14 ms（CSM 2.2、不透明 4.0、透明/粒子 0.7、TAA 0.8 …）。
- 场景复杂度（High）：主视图 draw call ≤ 1 200、阴影 draw call ≤ 1 000、可见三角形 ≤ 4.0 M、阴影三角形 ≤ 2.0 M、动态角色 ≤ 60、可见载具 ≤ 12、粒子活跃 ≤ 1 600、材质变体 ≤ 60。
- 内存：JS 堆 + WASM ≤ 1.5 GB（ARCH §13.3）；GPU 显存 High 4 GB。
规则：基准必须可重复——固定种子、固定机位、固定参考机；没有参考机时标 `未测`，不得用别的机器数字顶替后标 done（README §5.5）。

## 具体内容
### 要创建/修改的文件
- `client/tests/perf/m3_scenes.spec.ts` —— 三个场景：1) **vehicle-dense**：12 辆载具（8 辆行驶、4 辆静止）+ 步兵背景；2) **artillery-barrage**：10 次/10 s 炮击 + 烟雾 + 弹坑贴花累积；3) **air-raid**：Bombing（3 架、18 弹）+ Strafing 同帧，另加击毁/残骸 3 辆（M3-032）。每个场景 60 s 采样。
- `client/src/debug/perfExport.ts`（改）—— 导出 JSON：`frame_ms` P50/P95/P99、CPU 分项、GPU pass 分项、draw call、三角形、粒子数、JS 堆、WASM 内存、显存估计。
- `reports/perf/m3-baseline.json` —— 基线（入库的摘要，含机器型号/驱动/浏览器版本/命令）。
- `tools/perf/compare.ts` —— 对比 M2 基线与本次，输出回归表；超过 10% 退化标红。
- `reports/perf/m3-report.md` —— 汇总（摘要，不入 docs）。
### 要实现的行为
1. 每场景在 High（WebGPU）下采集；若有 Medium WebGL2 参考环境，附加采集（可选）。
2. 采集前预热 10 s 排除着色器编译抖动；记录首次着色器编译的卡顿次数（> 50 ms 帧数）。
3. 汇总 CPU/GPU 分项，与 ARCH §13.5.1/§13.5.2 的预算逐项对照，偏差 > 10% 的项列入"预算偏差"表（为 ARCH 回填提供数据，回填由 Fable 做）。
4. 载具密集场景重复运行 3 次取中位数，方差 > 5% 则查因。
5. 内存：连续运行 10 min 空袭/炮击循环，JS 堆增长曲线斜率 ≈ 0（泄漏检查）。
### 接口与数据
- JSON 字段：`{ scene, quality, p50_ms, p95_ms, p99_ms, cpu:{...}, gpu:{...}, draw_calls, tris, particles, js_heap_mb, wasm_mb, hitches_gt50ms }`。

## 不做什么
- 四档画质矩阵与四类参考硬件（M4-041）；Low/WebGL2 优化（M4）。
- 优化代码（发现超预算只写 PLAN_ISSUES 与优化建议，必要时拆后续任务，不在本任务大改）。
- 服务器性能（M3-045）。

## 验收标准
1. 三场景 High P95 帧时间 ≤ 16.7 ms（参考机 RTX 3060，1080p；`reports/perf/m3-baseline.json`）；无参考机则该条记 `未测`、任务至多 `partial`（H-021 测试设备矩阵）。
2. 同屏 12 载具场景：主视图 draw call ≤ 1 200、可见三角形 ≤ 4.0 M、动态角色 ≤ 60、材质变体 ≤ 60（`SceneInstrumentation` 数字）。
3. 粒子峰值 ≤ 1 600；空袭场景 GPU "透明/粒子" ≤ 0.7 ms（timestamp query）。
4. 击毁特效 GPU 耗时 ≤ 0.5 ms（M3-032 的验收口径复测）。
5. 10 min 循环后 JS 堆增长 ≤ 5%（起止对比），WASM 内存无增长；JS 堆 + WASM 峰值 ≤ 1.5 GB。
6. 与 M2 基线对比：同机位（M2 老城机位）帧时间退化 ≤ 10%（`compare.ts` 输出 0 项标红，或标红项有 PLAN_ISSUES）。
7. 报告包含参考机型号、浏览器版本、驱动版本与命令行（复现性）。

## 测试要求
- 单元：`perfExport` 统计函数（P95 计算）、`compare.ts` 阈值判定。
- 集成：Playwright 三场景脚本；CI 中以软件渲染只跑"冒烟（60 帧不崩溃，指标字段齐全）"。
- 性能基准：见上；记录硬件。
- 回归：把基线 JSON 与 M2 基线并存，后续任务用 `compare.ts` 回归检查。

## 性能预算
- ARCH §13.5.1（CPU ≈ 10.4 ms）、§13.5.2（GPU ≤ 14 ms）、§13.5.3（场景复杂度）、§13.3（内存）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 灰盒载具的三角形远低于真模型；真素材接入（M3-041/044）后必须复测，并更新基线。
- 需要 RTX 3060 参考机（H-021）；无则数字仅供趋势参考。
- INDEX 的依赖未含 M3-030（空袭），但 air-raid 场景需要其产出；如未完成则该场景记 `未测`。
