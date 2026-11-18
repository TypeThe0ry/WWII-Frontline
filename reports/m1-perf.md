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

植被散布已改为认地图（PI-110，本次修复，见下节）：不长在建筑/道路/树篱/水上，树集中在 forests 里，草稀疏到 0.4、4×4 个 64 m 区块共用一组网格。下表“植被开”是修复后的数字；“植被关”未重测（场景代码未变）。修复前的数字（同机器、同方法）：P95 33–50 ms、CPU P50 27.7/30.1/35.7 ms（open_field/hedgerow/old_town）、draw call 402/814/696、JS 堆 969–993 MB、卡顿帧 7/43/88。

| 组 | 场景 | 帧间隔 P50 ms | P95 ms（3 轮） | 卡顿帧 >50 ms（最差轮） | CPU P50 ms | GPU ms | draw call | 三角形 | JS 堆 MB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 植被开 | open_field | 16.7 | 16.8 / 33.4 / 50.0 | 95 | 14.2 | 0.40 | 84 | 449 163 | 287 |
| 植被开 | hedgerow | 16.7 | 50.1 / 100.0 / 50.0 | 238 | 17.9 | 0.37 | 205 | 352 351 | 306 |
| 植被开 | old_town | 16.7 | 33.4 / 33.4 / 16.8 | 70 | 14.6 | 0.33 | 145 | 344 578 | 298 |
| 植被关 | open_field | 16.7 | 16.8 / 16.8 / 16.8 | 2 | 9.5 | 0.19 | 42 | 45 740 | 102 |
| 植被关 | hedgerow | 16.7 | 16.7 / 16.8 / 16.7 | 9 | 8.6 | 0.21 | 144 | 55 410 | 118 |
| 植被关 | old_town | 16.7 | 16.8 / 16.8 / 16.8 | 6 | 10.2 | 0.23 | 81 | 61 668 | 116 |

判定（工具输出，high 档）：
- **植被开：FAIL（但已大幅改善）**。CPU P50 从 27.7–35.7 ms 降到 **14.2 / 17.9 / 14.6 ms**（目标 §13.5 约 10.4 ms，仍未达到）；draw call 402/814/696 → **84/205/145**（预算 ≤1200，PASS）；三角形 ≤0.45 M（≤4.0 M，PASS）；JS 堆 970–990 → **287–306 MB**；P50 帧间隔恒为 16.7 ms（VSync 锁 60 Hz）。**P95 仍超**：每场景 3 轮里最好 16.8 ms、最差 50–100 ms；**卡顿帧 >50 ms 为 70–238（限 2），本次没有消除**，各轮差异很大（old_town 第 3 轮仅 3 个），而工作站此时的平均负载约 100（其它会话在跑 Codex 等），所以这些尖峰有一部分来自机器干扰，但不能因此判 PASS；需要在安静机器/参考机上重测，并另查植被上传与 GC 的尖峰。
- **植被关：FAIL（边缘，上一版数字）**。P50 恰为 60 Hz 锁的 16.7 ms，P95 16.7–16.8 ms 对 16.7 限值差 0.0–0.1 ms（VSync 与 0.1 ms 计时精度造成），卡顿帧 2–9；CPU 8.6–10.2 ms，GPU 0.2 ms。
- 重复性：植被开三个场景的 P95 极差远超 10 %，`variance_flags` 已标出。
- 材质（E1-8 的代理量）：运行时场景里实际被网格使用的材质 **18 种**（含 M0 灰盒士兵/武器/特效、地形外的 12 种地图相关材质；≤60），这是材质数而不是 CI `variant-count` 的着色器变体数。
- **未测**（工具不会放过）：分 pass GPU 预算（csm/opaque/transparent/taa/post，渲染器还没有这些 pass 的计时）、`js_heap_mb+wasm_mb`（wasm 内存未采集；JS 堆本身约 0.3 GB）、VRAM 估计、`variant-count` 着色器变体数、Low/Iris Xe 档。

## 植被修复（PI-110，本 PR）

- **瓶颈**：Babylon 每个 draw 的 CPU 成本（WebGPU 下约 30 µs/draw：`babylon_active_meshes_eval` 21–24 ms，占 scene_render 的 70 %），而 GPU 只要 0.5 ms。M0 是“每 64 m 区块 × 物种 × LOD 一个网格”，Carentan 2048 m 方块有上千个网格、400–800 个 draw。
- **做法**：(1) `Vegetation` 新增 `groupChunks`：实例表仍是 64 m 区块，但 g×g 个区块共用一组 LOD 网格（world 场景用 4，即 256 m）：draw 400–800 → 84–205，`vegetation_update` 约 2 ms → 0.2 ms；(2) `world/veg-mask.ts`：把 buildings/roads/hedgerows/water/forests 栅格到 1 m 的掩码，散布避开建筑（+1.5 m）、道路（半宽 +0.4 m）、树篱（半宽 +0.6 m）、水（+0.5 m），树在 forests 内概率 0.9、林外 ×0.12；(3) 草的保留概率 ×0.4（`veg_grass`，[tbd]），草实例 2.2 M → 0.6 M，内存随之下降。M0 的 `?scene=vegetation` 行为不变（默认 groupChunks=1、无掩码）。
- 测试：`veg-mask.test.ts`（4 项）、`scatter.test.ts` 掩码 3 项、`vegetation.test.ts` 分组 1 项。

## 本次接线（M1-051b / 052b 的已做部分）

- `?scene=world`：地形 + 植被 + 树篱 + 建筑；树篱为 1 个 thin-instance 方盒网格（1 draw call，缺口段不画），建筑按 128 m 格合并为 173 个挤出体网格（墙 + 平屋顶，耳切三角化，0 失败）。`window.__world.stats` 给出数量。
- `cam=open_field|hedgerow|old_town` 相机预设与 `local=x,z,yaw` 演示士兵位置；`tools/perf/capture.mjs --m1` 与 `tools/perf/m1-report.mjs`。

## 仍需做

1. 参考机 RTX 3060 / Iris Xe 实测（H-021）。
2. 植被：CPU P50 仍 12–18 ms（目标约 10.4）且卡顿帧未消除：继续降 `babylon_active_meshes_eval`（场景网格/实体部件数）、查植被缓冲上传与 GC 尖峰；Low 档密度；树篱旁的树、树篱本身的植被。
3. 建筑：开洞/楼梯/室内（可进入 70 栋目前是实心体）、屋顶形状、LOD/impostor、`building_collider.rs`；树篱：`hedgerow_collider.rs` 与 sim 集成测试、真实树篱人工数据（现有仅 708 m，验收 > 4 km）。
4. 分 pass GPU 计时、wasm 内存采集、关 VSync 复测、占位比例统计。

## 补充：M0 vegetation 场景（上一版，PR #89）

仍保留在 `reports/m1-perf-raw/perf-RTX3080-ws-{vsync,novsync}-webgpu-cam*.json`；只说明工作站 GPU 路径可用，不是 M1 负载。
