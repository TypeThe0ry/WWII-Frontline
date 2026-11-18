# M1 性能基准（M1-069b，工作站数字）

> **这是工作站数字，不是 RTX 3060 参考机（H-021）的数字**，不得据此判定 E1-6/E1-7（PI-109）。场景是占位几何（树篱方盒、建筑挤出体、M0 灰盒士兵、M0 植被散布），不是最终素材（M1-064 阻塞）。

## 机器与方法

GPU NVIDIA GeForce RTX 3080 20 GB（Ampere，PCI 01:00.0，同时跑 Sunshine 串流桌面），驱动 595.58.03；2×EPYC 7402（96 线程）、405 GiB；Ubuntu 26.04；Chromium 153.0.8010.12，WebGPU（timestamp-query 可用），1920x1080，垂直同步开（帧率锁 60 Hz）。机器上还有其它进程占用 GPU/CPU，没有隔离。

场景 `?scene=world&map=carentan&cam=open_field|hedgerow|old_town&actors=demo&entities=60`：Carentan 地形 + `hedgerows.geojson`（32 条，424 段，2.39 km）+ `buildings.geojson`（2839 栋，173 个 128 m 合并网格，56 572 三角形）+ 60 个演示士兵（M0 灰盒）+ 士兵视角相机（机位见 `client/src/render/perf-cameras.ts` 的 `M1_PERF_CAMERAS`，单测钉死）。预热 10 s、采集 60 s、3 轮（`tools/perf/m1-scenes.json`）。`frame_ms` 是页面 rAF 逐帧间隔（P95、>50 ms 的卡顿帧数）。HUD 为 `debug=1` 的叠层，不是完整 HUD。命令：

```
node tools/perf/capture.mjs --m1 --gpu --renderer webgpu --viewport 1920x1080 --machine RTX3080-ws --veg 1 --out-dir reports/m1-perf-raw
node tools/perf/capture.mjs --m1 --gpu --renderer webgpu --viewport 1920x1080 --machine RTX3080-ws --veg 0 --out-dir reports/m1-perf-raw
node tools/perf/m1-report.mjs        # -> reports/m1-perf.json
```

## 结果（`budget.mjs` 的 high 档判定）

植被散布仍是 M0 的**全图均匀散布**（不认道路/建筑/树篱，街上也长草），所以分“植被开”和“植被关”两组；真实 M1 负载介于两者之间，待 M1-051b 余项（按 forests/树篱/建筑避让的散布）完成后再测。

| 组 | 场景 | 帧间隔 P50 ms | P95 ms（3 轮） | 卡顿帧 >50 ms（最差轮） | CPU P50 ms | GPU ms | draw call | 三角形 | JS 堆 MB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 植被开 | open_field | 33.3 | 33.4 / 33.4 / 33.4 | 7 | 27.7 | 0.48 | 402 | 658 110 | 969 |
| 植被开 | hedgerow | 33.3 | 33.5 / 33.4 / 50.0 | 43 | 30.1 | 0.61 | 814 | 590 378 | 993 |
| 植被开 | old_town | 33.3 | 50.0 / 50.1 / 50.0 | 88 | 35.7 | 0.48 | 696 | 604 962 | 983 |
| 植被关 | open_field | 16.7 | 16.8 / 16.8 / 16.8 | 2 | 9.5 | 0.19 | 42 | 45 740 | 102 |
| 植被关 | hedgerow | 16.7 | 16.7 / 16.8 / 16.7 | 9 | 8.6 | 0.21 | 144 | 55 410 | 118 |
| 植被关 | old_town | 16.7 | 16.8 / 16.8 / 16.8 | 6 | 10.2 | 0.23 | 81 | 61 668 | 116 |

判定（工具输出，high 档）：
- **植被开：FAIL**。P95 33–50 ms（> 16.7），卡顿帧 7–88（> 2）。瓶颈是 **CPU**（27–36 ms/帧，GPU 只有 0.5–0.6 ms）：植被每帧 400–800 个 draw、4.5–5.9 万实例。draw call（≤ 1200）与三角形（≤ 4.0 M）**PASS**。
- **植被关：FAIL（边缘）**。P50 恰为 60 Hz 锁的 16.7 ms，P95 16.7–16.8 ms 对 16.7 限值差 0.0–0.1 ms，是垂直同步与 0.1 ms 计时精度造成的“FAIL”，不是真实超标（CPU 仅 8.6–10.2 ms，GPU 0.2 ms）；卡顿帧 2–9（限 2）hedgerow/old_town 超标，机器有 Sunshine 等干扰，不能据此认定是游戏问题。要看余量需关 VSync 重测（本次未做）。draw call PASS、三角形 PASS。
- 重复性：植被开 hedgerow 的 P95 极差 > 10 %（有一轮 50 ms），已在 JSON 的 `variance_flags` 标出。
- **未测**（工具不会放过）：分 pass GPU 预算（csm/opaque/transparent/taa/post，渲染器还没有这些 pass 的计时）、`js_heap_mb+wasm_mb`（wasm 内存未采集；JS 堆本身植被开约 970–990 MB，已占 §13.5 内存预算 1536 MB 的 63 %）、VRAM 估计、E1-8 材质变体数（CI 无 M1 数字）、Low/Iris Xe 档。

## 本次接线（M1-051b / 052b 的已做部分）

- `?scene=world`：地形 + 植被 + 树篱 + 建筑；树篱为 1 个 thin-instance 方盒网格（1 draw call，缺口段不画），建筑按 128 m 格合并为 173 个挤出体网格（墙 + 平屋顶，耳切三角化，0 失败）。`window.__world.stats` 给出数量。
- `cam=open_field|hedgerow|old_town` 相机预设与 `local=x,z,yaw` 演示士兵位置；`tools/perf/capture.mjs --m1` 与 `tools/perf/m1-report.mjs`。

## 仍需做

1. 参考机 RTX 3060 / Iris Xe 实测（H-021）。
2. 植被散布按 forests/树篱/建筑避让并压低 CPU（当前 400–800 draw、4.5–5.9 万实例，CPU 瓶颈）；Low 档密度。
3. 建筑：开洞/楼梯/室内（可进入 70 栋目前是实心体）、屋顶形状、LOD/impostor、`building_collider.rs`；树篱：`hedgerow_collider.rs` 与 sim 集成测试、真实树篱人工数据（现有仅 708 m，验收 > 4 km）。
4. 分 pass GPU 计时、wasm 内存采集、关 VSync 复测、占位比例统计。

## 补充：M0 vegetation 场景（上一版，PR #89）

仍保留在 `reports/m1-perf-raw/perf-RTX3080-ws-{vsync,novsync}-webgpu-cam*.json`；只说明工作站 GPU 路径可用，不是 M1 负载。
