# tools/perf — 客户端性能采集（M0-030）

目的：在**参考机**（RTX 3060 / Iris Xe 级核显）上重复、可比地测帧时间；在没有 GPU 的机器上测能客观测的东西（draw call、三角形、植被实例数、各阶段 CPU 耗时、JS 堆、包体积）。**没有参考机时标 `未测`，不得用别的机器的数字顶替。**

## 机位与场景（固定，改动即作废基线）

- 入口 `?scene=vegetation&cam=0|1|2&debug=1`（PI-020：地形 + 植被；`?scene=terrain` 只有地形）。机位在 `client/src/render/perf-cameras.ts`（单元测试钉死坐标）：
  `0` 地面朝北（开阔地形 + 近处植被）、`1` 俯瞰整块（实例最多）、`2` 块边缘朝西（山脊/远景环）。
- 植被种子 `veg_seed=1`（默认）、场景里没有依赖墙钟的动画；`quality=high|low`（Iris Xe / WebGL2 档用 `--quality low`）。
- `?perfcam=` / `?perfclock=fixed`（任务卡原稿）不实现：机位用 `cam`，场景无时间依赖，固定步长由 `?scene=` 下无模拟步进保证。

## 准备（参考机）

1. 安装 Node 22、pnpm、Chromium（Playwright 自带，`pnpm -C client exec playwright install chromium` 仅在没有浏览器的机器上做一次）。
2. 仓库根目录：`pnpm install`；`PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0`；`wasm-pack build crates/sim-wasm --target web --profile wasm-release`；`pnpm -C client sync-map`。
3. 关闭省电模式（Windows：电源计划“高性能”；笔记本接电源；macOS 低电量模式关）、关闭其它占 GPU/CPU 的程序，浏览器窗口不要最小化/被遮挡（采集脚本已加 `--disable-background-timer-throttling` 等，但窗口遮挡仍可能降频）。让机器空闲 2 分钟再测，记录是否有热降频（笔记本务必）。

## 命令

```
# RTX 3060 / 1080p / WebGPU（真 GPU：加 --gpu，不带 SwiftShader 参数）
node tools/perf/capture.mjs --gpu --renderer webgpu --viewport 1920x1080 --machine rtx3060
# Iris Xe 级 / 720p / WebGL2 / Low 档
node tools/perf/capture.mjs --gpu --renderer webgl2 --viewport 1280x720 --quality low --machine irisxe
# 其它机型（如 GTX 1650 Medium）同法记录但不判定
node tools/perf/capture.mjs --gpu --renderer webgl2 --viewport 1920x1080 --machine gtx1650
# 无 GPU 的开发机（软件渲染，只看计数与 CPU 阶段）：默认参数即可，约 8 分钟
node tools/perf/capture.mjs
node tools/perf/report.mjs            # 聚合成 reports/m0/perf/summary.{md,json}
```

默认：3 个机位 × 3 次 × （加载 + 预热 5 s + 采集 30 s）。选项见 `capture.mjs` 文件头。冒烟：`--duration 5 --warmup 2 --repeats 1 --cams 0`。

## 输出

`reports/m0/perf/perf-{machine}-{renderer}-cam{N}.json`：`meta`（机器、CPU、核数、内存、OS、Node、Chromium 版本、命令、commit、日期；**请在 README/报告里手工补 GPU 型号与驱动版本**——浏览器拿不到可靠的驱动版本）、`runs[]`（每次 `window.__perf.dump()` 的完整内容 + `meta{cam, run, load_ms, js_heap_mb, caps}`）、`aggregate`（3 次的中位数与极差 %）。拿不到所请求的渲染路径（无 WebGPU 适配器、页面回退到 WebGL2）时写 `{skipped}`，报告标 `未测`。

`dump()` 字段：`cpu_ms{p50,p99}`（一帧的 JS 时间）、`interval_ms`（帧间隔，= 1/fps）、`gpu_ms`（真 GPU 上是 timestamp-query / 软件渲染下的数字**不是** GPU 时间，只在 `gpu_ms.source` 为 `timestamp-query` 或 `disjoint-timer-query` 且 `--gpu` 时才采信）、`draw_calls`、`triangles`、`vegetation{drawCalls,instances,tris}`、`stages{game_update, terrain_update, vegetation_update, overlay, scene_render, babylon_active_meshes_eval, babylon_render_pass}`（各阶段 CPU ms，窗口统计）。

## 判据

- 重复性：3 次 P50 的极差 ≤ 5 %（`report.mjs` 的 “P50 range” 列）；超出就是机器有干扰（后台程序、热降频、省电），重测并写明环境。
- E0-5：RTX 3060 / 1080p ≥ 60 fps（P50，即帧间隔 P50 ≤ 16.7 ms）；Iris Xe / WebGL2 / 720p ≥ 30 fps（P50 ≤ 33.3 ms）。预算对照 ARCHITECTURE §13.5：CPU ≈ 10.4 ms/帧、GPU ≤ 14 ms。
- 前三个耗时来源：看 `stages`（CPU）与 `gpu_ms`；GPU 分项用 Babylon Inspector / Spector.js / Chrome `chrome://gpu` + Performance 面板补充，记入报告。

## 常见问题

- **帧率被锁在 60 / 30**：浏览器与显示器的垂直同步。真机采集请用 `--headed` 并关闭 VSync（Chromium 启动参数 `--disable-frame-rate-limit --disable-gpu-vsync`，可经 `PLAYWRIGHT_CHROMIUM_ARGS`/改 `chromiumArgs()` 加入），否则 P50 恒为 16.7 ms，看不出余量；报告里要写明。
- **fps 极低（1–4）**：这是软件渲染（SwiftShader）。此时 `interval_ms` 与 `gpu_ms` 不代表任何真机，不得写进 E0-5 的判定。
- **WebGPU 请求被跳过**：Chromium 版本太旧或驱动黑名单；`chrome://gpu` 看 WebGPU 状态；Linux 需 `--enable-features=Vulkan`。
- **后台节流**：窗口被遮挡/最小化，rAF 降到 1 Hz；用 `--headed` 并保持窗口前台。
- **浏览器版本**：写进报告（`meta.chromium`）；不同版本的数字不可直接比较。
- **P99 抖动**：云 VM 与笔记本热降频都会；报告三次的极差，不要挑最好的一次。

## 回归检查（无 GPU 也能跑）

`scripts/perf-regress.mjs`：测量 → 与 `reports/baselines/perf-m0.json` 比较 → 超容差即失败。见该文件头与 `reports/m0/perf-baseline.md`。

```
node scripts/perf-regress.mjs check --quick     # 约 6 分钟（含 cargo bench、客户端采集、40 秒级服务器压测）
node scripts/perf-regress.mjs check             # 完整（约 20 分钟）
node scripts/perf-regress.mjs update-baseline   # 有意改变性能后，人工采纳新基线
node --test "tools/perf/*.test.mjs" "scripts/perf-regress.test.mjs"
```
