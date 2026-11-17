# M4-041 测试：四档画质性能矩阵

## 元数据
- 里程碑：M4
- 工作流：test
- 规模：L
- 依赖：M3-049（M3 性能基准）, M4-020, M4-022, M4-014, M4-015
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §13.1; ARCHITECTURE §13.5; ARCHITECTURE §13.4; ARCHITECTURE §13.3; ARCHITECTURE §20.1; NEEDS_HUMAN H-021, H-039

## 背景
在**参考硬件**上实测四档性能并回填预算偏差。硬件矩阵（ARCHITECTURE §13.1 与 H-021）：Iris Xe 级核显（Low，WebGL2，720p 30 fps）、GTX 1650/1060 级（Medium，1080p 输出内部 67%，45–60 fps）、RTX 3060（High 基准，1080p，60 fps）、RTX 4070+（Ultra，1440p，60 fps）。另有 8 GB 内存机器测 Low 内存。
目标（MILESTONES E4-1，P95）：**Low 720p ≥ 30 fps；Medium 1080p 45–60 fps；High 3060 1080p ≥ 60 fps；Ultra 4070 1440p ≥ 60 fps**。缺硬件的档标 `partial` 并写明（不得用其它机器数字顶替，README §5.5）。
预算对照（ARCHITECTURE §13.5）：CPU（High，60 fps）合计 ≈ 10.4 ms（输入 1.5、快照解码 0.6、渲染提交 4.5、流式 1.5、音频 0.5、UI 0.8、GC 1.0）；WebGL2 渲染提交 +2.5 ms；GPU（High，1080p，内部 100%，目标 ≤ 14 ms）合计 ≈ 12.0 ms（Hi-Z 0.5、CSM 2.2、预通道 0.8、GTAO 0.9、froxel 0.8、不透明 4.0、天空雾 0.3、透明 0.7、TAA 0.8、后处理 0.7、其它 0.3）；Medium 预算 ≤ 16 ms（需实测校准，§13.5.2 注）。场景复杂度（§13.5.3）：High draw ≤ 1200、三角形 ≤ 4.0 M；Low draw ≤ 500、三角形 ≤ 0.8 M。内存（§13.3）：JS+WASM Low ≤ 0.9 / Medium ≤ 1.2 / High、Ultra ≤ 1.5 GB；显存估计 Low 1.5 / Med 2.5 / High 4 / Ultra 6 GB（M4-022）。
测试场景：三图（Carentan、Omaha、Foy）× 固定机位序列（战斗压力高密度场景：60 动态角色、12 载具、粒子峰值）× 5 min 脚本巡游；P50/P95/P99 帧时间、GPU 分 pass（timestamp）、CPU 分项（`performance.measure`）、JS 堆与 WASM、显存估计、draw call/三角形。
**本任务需要人类提供硬件与实机运行**（H-021）；执行者准备脚本、采集与汇总。

## 具体内容
### 要创建/修改的文件
- `tests/perf/matrix/run.ts` —— Playwright（或 `puppeteer`/`chrome --remote-debugging`）驱动的巡游与采集；输出 `reports/perf/m4-{hardware_id}-{tier}-{map}.json`。
- `tests/perf/matrix/hardware.json` —— 硬件登记模板（CPU/GPU/RAM/OS/浏览器版本/驱动/分辨率/DPR/电源模式）。
- `tests/perf/matrix/summarize.py` —— 汇总表与预算对照（P95 vs 目标；分 pass 偏差），生成 `reports/perf/m4-matrix-summary.md`。
- `tests/perf/matrix/README.md` —— 人类操作步骤（如何在目标机运行、如何回传 JSON、电源与后台程序要求）。
- 反馈：对 `ARCHITECTURE §13.5` 的偏差汇总 `reports/perf/m4-budget-deviation.md`（**由 Fable 回写文档**，执行者不改 docs，按 README §5.1 写 PLAN_ISSUES）。
### 要实现的行为
1. 采集器：每帧记录帧间隔、CPU 分项、GPU timestamp（可用时）；每 1 s 记录内存与显存估计；5 min 巡游；剔除前 30 s 预热。
2. 目标机运行：每台 × 每档 × 每图 3 次，取中位；记录浏览器版本与是否 WebGPU/WebGL2。
3. 汇总：P50/P95/P99 帧时间、fps、预算偏差表（实测 vs §13.5.1/§13.5.2 分项）、draw/三角形、内存。
4. 缺硬件：对应档位在报告中标 `未测`，STATUS 对应项 `partial`。
5. 瓶颈归因：对未达标项给出 top 3 pass/CPU 项与建议（PLAN_ISSUES），不在本任务内修复。
6. 可复现：固定种子、固定巡游、固定 `render_scale` 起始值；动态分辨率开/关各跑一次（开：记录 `scale` 曲线）。
### 接口与数据
- 报告 JSON：`{"hw":"…","tier":"high","map":"…","backend":"webgpu","fps_p50":…,"fps_p95":…,"frame_ms_p95":…,"gpu_ms_by_pass":{…},"cpu_ms":{…},"js_heap_mb":…,"vram_est_mb":…,"draw":…,"tris":…}`。

## 不做什么
- 不做优化修复（交给 M4-020/M4-022 的后续或 PLAN_ISSUES）。
- 不做浏览器兼容矩阵（M4-042）；不做服务器性能（M4-038）。
- 不做遥测采集（M4-036，封闭测试补充）。
- 不改 ARCHITECTURE 预算数字（只提出偏差建议，README §5.1）。

## 验收标准
1. **Low 720p ≥ 30 fps（P95）**：Iris Xe 级机器 WebGL2，三图均满足 `frame_ms_p95 ≤ 33.3`。
2. **Medium 1080p 45–60 fps（P95）**：GTX 1650/1060 级，输出 1080p，三图均 `fps_p95 ≥ 45`（上限 60 为封顶）。
3. **High RTX 3060 1080p ≥ 60 fps（P95）**：`frame_ms_p95 ≤ 16.7`。
4. **Ultra RTX 4070 级 1440p ≥ 60 fps（P95）**：`frame_ms_p95 ≤ 16.7`。
5. 内存：JS 堆 + WASM 峰值 Low ≤ 0.9 / Medium ≤ 1.2 / High、Ultra ≤ 1.5 GB；显存估计 ≤ 档位预算（引用 M4-022 的估计，实测以 `chrome://gpu` 或工具辅助，不可得则 `未测`）。
6. 场景复杂度：各档 draw/三角形 ≤ §13.5.3 预算；超出项列表 + PLAN_ISSUES 编号。
7. 缺硬件项标 `partial` 并写明缺失机型；`m4-matrix-summary.md` 对 4 档 × 3 图矩阵每格有"值/未测"。

## 测试要求
- 单元测试：`summarize.py` 对合成数据的分位计算与预算对照（pytest）。
- 集成/无头测试：脚本在 CI 软件渲染下做 30 s 冒烟（验证流程，不验证数字）。
- 性能基准：本任务即基准；基线 `reports/perf/baseline-m4.json` 供后续对比。
- 回归：与 M3-049/M2-052 的基线对比，退化 > 10% 须说明。

## 性能预算
- 见"背景"：§13.5.1 CPU、§13.5.2 GPU、§13.5.3 复杂度、§13.3 内存；本任务是**验证**这些预算而非新增。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`pytest` / Playwright 冒烟）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- **人工：参考硬件**（H-021 设备矩阵；H-039 最低配置决定，默认"按实测取能达标的最低档，未达标则 Low 降为 720p/25–30 fps 并标注最低体验"）。无硬件则 `blocked`/`partial`。
- 灰盒阶段的数字低估真实素材负载；真素材接入（M4-029）后需重测至少 High/Low 两档，结果写入 STATUS 并标注版本。
- 规模 L（含人工协调）：若超会话，拆 `M4-041b`（Ultra/低内存机补测）。
