# M2-052 测试：M2 性能基准

## 元数据
- 里程碑：M2
- 工作流：test
- 规模：M
- 前置依赖：M2-036, M2-030, M2-031, M2-032, M1-069
- 素材需求：无（灰盒即可）；素材状态：n.a.（灰盒数字；素材接入后复测）
- 引用文档：ARCHITECTURE §13.5; ARCHITECTURE §13.5.1; ARCHITECTURE §13.5.2; ARCHITECTURE §13.5.3; ARCHITECTURE §13.3; ARCHITECTURE §13.4

## 背景
M1-069 在三个固定机位（开阔/树篱/老城）采集过性能基准。M2 增加 GTAO（M2-030）、froxel 雾（M2-031）、TAAU（M2-032）、遮挡剔除（M2-034）、水面（M2-035）、全图流式（M2-036）、完整粒子（M2-033）。退出标准 E2-7（MILESTONES §3.3）：RTX 3060 / 1080p 老城/树篱/沼泽/开阔四机位 P95 帧时间 ≤ 16.7 ms；JS 堆 + WASM ≤ 1.5 GB（Medium ≤ 1.2 GB）；老城机位 draw call 因剔除下降 ≥ 30%。
预算表（ARCH §13.5）：CPU（主线程 High）合计约 10.4 ms（输入/game 1.5、快照解码+插值 0.6、渲染提交 4.5、流式 1.5、音频 0.5、UI 0.8、GC/杂项 1.0），WebGL2 渲染提交 +2.5 ms；GPU（High，1080p 内部 100%，≤ 14 ms）合计约 12.0 ms：Hi-Z+实例剔除 0.5、CSM 2.2、深度预通道 0.8、GTAO 0.9、froxel 0.8、不透明主通道 4.0、天空/雾合成 0.3、透明/粒子 0.7、TAA/TAAU 0.8、Bloom/曝光/色调映射 0.7、呈现+其它 0.3。场景复杂度（§13.5.3）：主视图 draw call ≤ 1 200（WebGPU High）/ ≤ 900（WebGL2 Medium）/ ≤ 500（Low）；可见三角形 ≤ 4.0 M/2.0 M/0.8 M；动态角色 ≤ 60；活动粒子 ≤ 1 600；材质变体 ≤ 60。

## 具体内容
### 要创建/修改的文件
- `client/tests/perf/scenes.json` —— 四个固定机位（老城、树篱、沼泽、开阔）+ 压力附加场景：60 动态角色、12 载具、1 600 粒子（灰盒，固定种子）。
- `client/tests/perf/perf.spec.ts` —— Playwright 性能采集：预热 10 s，采样 60 s；CPU 帧时间（`requestAnimationFrame` 间隔 + 主线程分段 mark）、GPU timestamp（逐 pass）、draw call/三角形/粒子/材质变体数（Babylon 统计 + 调试叠层）、内存（`performance.memory`、WASM memory、自估计显存）。
- `client/src/debug/perf_probe.ts` —— 统一采集接口（沿用 M1-069，必要时扩展 pass 标签：`pass.hiz_cull/csm/depth/gtao/froxel/main/sky/transparent/taau/post`）。
- `tools/perf/report.ts` —— 汇总生成 `reports/perf/m2_perf.json` 与 `m2_perf.md`（对照 §13.5 预算表逐项 ✓/✗）。
- `reports/perf/m1_baseline.json`（读取，M1-069 产出）—— 对比回归。
### 要实现的行为
1. 采集四机位 × {WebGPU High（RTX 3060，1080p，内部 100%）, WebGPU Medium（67% TAAU）, WebGL2 Medium} 三配置；每配置 60 s、固定机位、固定种子。
2. 输出：帧时间 P50/P95/P99、CPU 分段、GPU 各 pass、draw call、三角形、粒子、材质变体、JS 堆 + WASM 峰值、显存估计；与 §13.5 预算逐项对比，超预算项高亮。
3. 剔除对比：老城机位 `?cull=off/on` 两次采样，输出 draw call 差与帧时间差（E2-7）。
4. 15 min 巡游内存：与 M2-036 的 `camera-tour` 复用，记录峰值；Medium ≤ 1.2 GB，High ≤ 1.5 GB。
5. 与 M1 基线对比：开阔/树篱/老城三机位帧时间回归 ≤ +10%（M2 新增 pass 导致的增量需在表中归因）。
### 接口与数据
```json
{ "scene": "oldtown", "config": "webgpu-high", "frame_ms": {"p50":0,"p95":0,"p99":0}, "cpu_ms": {"submit":0,"stream":0}, "gpu_ms": {"hiz_cull":0,"csm":0,"gtao":0,"froxel":0,"taau":0}, "draw_calls": 0, "tris": 0, "particles": 0, "variants": 0, "heap_mb": 0, "wasm_mb": 0, "vram_est_mb": 0 }
```

## 不做什么
- 不做优化（发现超标写 PLAN_ISSUES，由对应任务/Fable 处理）；不做 Low 档/Iris Xe 基准（M4-020/M4-023）。
- 不做 100 人网络下的帧时间（M2-049 为服务器侧）。
- 不做素材接入后的复测（M2-047 之后若素材到位，另行记录于 STATUS 备注）。

## 验收标准
1. 四机位（老城/树篱/沼泽/开阔）WebGPU High（RTX 3060，1080p）P95 帧时间 ≤ 16.7 ms（E2-7；`reports/perf/m2_perf.json`）。
2. GPU 各 pass（High）对照 §13.5.2：Hi-Z+剔除 ≤ 0.5、GTAO ≤ 0.9、froxel ≤ 0.8、TAAU ≤ 0.8、粒子 ≤ 0.7、阴影 ≤ 2.2、不透明主通道 ≤ 4.0 ms（P95）；GPU 合计 ≤ 14 ms。
3. 老城机位 draw call：剔除开启比关闭降低 ≥ 30%，并 ≤ 1 200（High）。
4. 内存：High 配置 15 min 巡游 JS 堆 + WASM 峰值 ≤ 1.5 GB；Medium ≤ 1.2 GB（`performance.memory`，Chromium）。
5. 复杂度：压力附加场景下可见三角形 ≤ 4.0 M、材质变体 ≤ 60、活动粒子 ≤ 1 600（统计）。
6. CPU 主线程（High）：分段合计 ≤ 10.4 ms 均值（§13.5.1），渲染提交 ≤ 4.5 ms；WebGL2 路径渲染提交 ≤ 7.0 ms（4.5+2.5）。
7. 与 M1 基线对比：三机位帧时间回归 ≤ +10% 或在表中给出新增 pass 的归因。
8. 报告含机器型号、驱动、Chromium 版本、命令与 git 哈希；缺参考机的项标 `未测`（任务至多 `partial`）。

## 测试要求
- 单元测试：`tools/perf/report.ts` 的预算对比逻辑（构造超预算数据应标 ✗）；分位数计算。
- 集成/无头测试：Playwright 在真实 GPU 机上运行（CI 无 GPU，人工/自托管 runner 触发）；软件渲染运行仅用于脚本冒烟，数字不入验收。
- 性能基准：本任务即基准；基线 JSON 入 `reports/perf/` 供 M3-049 对比。
- 回归：M1-069 脚本不得破坏。

## 性能预算
- 全部引用 ARCHITECTURE §13.5.1（CPU 10.4 ms）、§13.5.2（GPU 12.0 ms/14 ms 上限）、§13.5.3（复杂度）、§13.3（内存）、§13.4（显存 High 合计 ≈ 2 940 MB，余量 ≈ 1 060 MB）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、perf 脚本）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 需要参考硬件：RTX 3060（High 基准）与 GTX 1650/1060（Medium）；测试设备矩阵见 H-021。无参考机则对应数字写 `未测`，**不得用其它机器的数字顶替后标 done**（README §5.5）。
- 灰盒几何远轻于真素材，M2-047 接入后三角形/显存会显著上升，需复测并更新（至少在 M2 验收前对已 OK 的素材复测一次）。
- GPU timestamp query 需浏览器特性支持（`timestamp-query`）；不可用时用帧间隔估计并标注"估计"。
