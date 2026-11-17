# M1-069 测试：M1 性能基准

## 元数据
- 里程碑：M1
- 工作流：test
- 规模：M
- 前置依赖：M1-052, M1-051, M1-044, M0-030
- 素材需求：无（灰盒即可）；素材状态：n.a.（占位下测量，并记录占位比例）
- 引用文档：ARCHITECTURE §13.5; ARCHITECTURE §20.1; ARCHITECTURE §13.3; ARCHITECTURE §13.1; MILESTONES §2.3

## 背景
M1 客户端性能退出标准 E1-6/E1-7/E1-8：High（RTX 3060，1080p）P95 帧时间 ≤ 16.7 ms，主视图 draw call ≤ 1 200，可见三角形 ≤ 4.0 M；Low（Iris Xe / WebGL2 / 720p）≥ 30 fps（P95），draw call ≤ 900（WebGL2 Medium 预算）/ ≤ 500（Low 预算，若测 Low 设置）；材质变体 ≤ 60。预算表（ARCHITECTURE §13.5）：CPU 主线程 16.67 ms 帧内合计 ≈ 10.4 ms；GPU（High，1080p）合计 ≈ 12.0 ms，各 pass：Hi-Z 0.5（M2）、CSM 2.2、深度预通道 0.8、GTAO 0.9（M2）、froxel 0.8（M2）、不透明 4.0、天空/雾 0.3、透明/粒子 0.7、TAA 0.8、Bloom/曝光/色调映射 0.7、呈现 0.3；场景复杂度（§13.5.3）：动态角色 ≤ 60、粒子 ≤ 1 600 等。M0-030 已建立"固定机位性能采集脚本（Playwright+叠层导出 JSON）"与参考机测量流程。M1 的三个固定机位（INDEX）：开阔、树篱、老城；测量时包含 60 个角色（或 headless 实体回放）、粒子、UI 全开。内存预算（§13.3）：JS 堆 + WASM ≤ 1.5 GB，显存 High 4 GB。ARCHITECTURE §20.1 的 M0 五项风险已在 M0 验收；M1 复测与渲染相关项（着色器卡顿、WebGL2 draw call）。

## 具体内容
### 要创建/修改的文件
- `tools/perf/m1-scenes.json` —— 三个固定机位（位置/朝向/时间/天气）+ 场景脚本（60 实体回放、爆炸/枪战脚本、HUD 全开）；机位 ID：`open_field`、`hedgerow`、`old_town`。
- `tools/perf/run-perf.mjs`（修改 M0-030 脚本）—— 每机位预热 10 s + 采集 60 s，导出帧时间序列、CPU/GPU 分项（叠层 GPU timestamp）、draw call、三角形、粒子数、JS 堆、WASM 内存、GPU 内存估计、着色器编译事件。
- `tools/perf/compare-budget.ts` —— 与 ARCHITECTURE §13.5 预算表比对，输出表格与 PASS/FAIL；超预算的 pass 列出超额量。
- `client/src/render/debug/perf-overlay.ts`（修改）—— 导出 JSON 的稳定 schema；P50/P95/P99；GPU timestamp 缺失（不支持）时标记。
- `reports/m1-perf.md`、`reports/m1-perf.json` —— 报告（含机型、浏览器版本、驱动、命令行）。
- `docs` 外产物：STATUS 记录 + PLAN_ISSUES 条目（超预算项）。
### 要实现的行为
1. 参考机测量：High（RTX 3060，1080p，WebGPU）与 Low（Iris Xe，WebGL2，720p）各自三机位。
2. 对照 §13.5.2 的 GPU 分 pass 表与 §13.5.3 复杂度表；M2 才有的 pass（Hi-Z/GTAO/froxel）标"不适用（M2）"。
3. 采集占位比例：报告写明当前使用占位的需求 ID 数与比例（占位三角形远小于真素材，真素材接入后需复测——写入备注）。
4. 重复 3 轮取中位数，记录方差；有意义差异（> 10%）标红。
5. 着色器卡顿检测：记录采集期间 > 50 ms 的帧，关联编译事件。
6. 输出 E1-6/E1-7/E1-8 的结论段落供 M1-072 引用。
### 接口与数据
```json
{ "machine": "RTX3060-1080p", "scene": "old_town", "frame_ms": {"p50": 0, "p95": 0, "p99": 0}, "draw_calls": 0, "tris": 0, "gpu_ms": {"csm": 0, "opaque": 0, "taa": 0, "post": 0}, "js_heap_mb": 0, "wasm_mb": 0, "hitches_gt_50ms": 0 }
```

## 不做什么
- 不做优化本身（超预算写 PLAN_ISSUES 并建议新任务，不在此任务大改渲染）。
- 不做 M2 的 Hi-Z/GTAO/froxel/TAAU 预算（M2-052）、动态分辨率/画质四档完整调校（M4）。
- 不做服务器侧性能（M1-068）。
- 不用别的机器数字顶替后标 `done`（README §5.5）。

## 验收标准
1. High（RTX 3060，1080p，WebGPU）三机位 P95 帧时间 ≤ 16.7 ms；主视图 draw call ≤ 1 200；可见三角形 ≤ 4.0 M（`reports/m1-perf.json`；E1-6）。
2. Low（Iris Xe，WebGL2，720p）三机位 P95 ≥ 30 fps（帧时间 ≤ 33.3 ms）；WebGL2 Medium 设置下 draw call ≤ 900；Low 设置下 ≤ 500（E1-7）。
3. 材质变体 ≤ 60（CI `variant-count` 当次结果；E1-8）。
4. GPU 各已实现 pass 不超出 §13.5.2：CSM ≤ 2.2 ms、不透明 ≤ 4.0 ms、透明/粒子 ≤ 0.7 ms、TAA ≤ 0.8 ms、Bloom/曝光/色调映射 ≤ 0.7 ms（High，GPU timestamp；任一超标写 PLAN_ISSUES）。
5. 内存：JS 堆 + WASM ≤ 1.5 GB、估计显存 ≤ 4 GB（High；`performance.measureUserAgentSpecificMemory` 或 `performance.memory` 与自估计）。
6. 60 s 采集内 > 50 ms 的卡顿帧数 ≤ 2（预热后）。
7. 参考机不可用时：对应条目标 `未测`，任务 `partial`，报告写明已用机型与差距（README §5.5）。

## 测试要求
- 单元测试：`compare-budget.ts` 的阈值判定；JSON schema 校验。
- 集成/无头测试：Playwright 采集脚本 CI 冒烟（非参考机，仅检查脚本可运行，不判 PASS）。
- 性能基准：本任务即基准；基线文件 `reports/m1-perf.json` 入库（摘要，不含原始 trace 大文件）。
- 回归：M0-030 的报告格式向后兼容。

## 性能预算
- ARCHITECTURE §13.5.1 CPU 总计 ≈ 10.4 ms（余量 6.3）；§13.5.2 GPU ≈ 12.0 ms（≤ 14 ms 目标）；§13.5.3 复杂度表；§13.3 内存预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID 已列出（占位比例）
- [ ] 文档同步：超预算项登记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 参考机（RTX 3060 / Iris Xe）需人类提供或租用（NEEDS_HUMAN H-021 测试设备矩阵）；缺失则 `partial`。
- 灰盒占位的三角形/纹理远低于真素材，真素材（M1-064）接入后必须复测一次；结果以复测为准。
- WebGL2 路径无 Hi-Z（M2-034），老城机位 draw call 是主要风险（MILESTONES §2.8）。
