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

## E1-6 卡顿与 CPU 复查（2026-11-19，PI-114）

在 `nice -n -15`（提高优先级，其余条件相同）下重采 M1 三机位，植被开，3 轮×60 s（`reports/m1-perf-raw/perf-RTX3080-ws-nice-webgpu-*.json`）：

| 场景 | CPU P50 ms（3 轮） | 帧 P95 ms（3 轮） | 卡顿帧 >50 ms（3 轮） |
| --- | --- | --- | --- |
| open_field | 8.2 / 8.3 / 8.7 | 33.3 / 49.9 / 66.6 | 33 / 90 / 148 |
| hedgerow | 9.3 / 8.6 / 9.0 | 66.7 / 83.3 / 16.8 | 168 / 169 / 21 |
| old_town | 8.7 / 8.6 / 9.2 | 16.8 / 16.8 / 33.4 | 6 / 28 / 102 |

- **CPU P50 8.2–9.3 ms，达到 §13.5 的约 10.4 ms 目标。** 之前（不提优先级）的 12–18 ms 里有一部分是工作站的 CPU 抢占（平均负载 60–100/96 线程）。
- **卡顿不是本项目代码造成的，是工作站 GPU 被其它任务占着。** 证据：(1) 采集时 `nvidia-smi` 四张卡都在 93–100 % 利用率（别的会话的训练任务，Chrome 的 GPU 进程与 Sunshine/训练任务共用一张卡），我们自己的 GPU 时间只有 0.2–0.5 ms/帧；(2) 主线程 CPU 只有 8–9 ms 而帧间隔却出现 >50 ms，即在等 GPU/呈现；(3) **对照实验**：old_town 植被开，把植被重分桶/上传完全关掉（`veg_budget=0`，不再写缓冲）与默认（0.45 ms）交替各跑两次 45 s：卡顿帧 100 / 113（两个“坏窗口”）和 3 / 2（两个“好窗口”），**与植被上传无关，只和时间窗口（机器当时的占用）有关**；(4) 同一配置不同轮之间卡顿帧从 6 到 169，波动远大于任何配置差异。
- 所以 **E1-6 的 P95 与卡顿帧在这台共享 GPU 的工作站上无法判定**；需要空闲 GPU（或参考机）重测。我不把它记成 PASS。可复现命令：`nice -n -15 node tools/perf/capture.mjs --m1 ...`；`--extra "veg_budget=0"` 传页面参数。

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

## B6-perf-gpu12 GPU1 绑卡复测（2026-11-20）

**PI-109 工作站数据。RTX 3060 和 Iris Xe 参考机仍未测（H-021）。** 同一共享工作站的 NVIDIA GeForce RTX 3080 20 GB（GPU1，PCI `0000:23:00.0`，驱动 `595.58.03`），2× EPYC 7402（96 线程），Chromium Headless Shell `153.0.8010.12`。两个档位各在启动前用 `nvidia-smi` 检查三次，GPU1 均为 0%，且 `pmon` 无进程。启动时用 `DRI_PRIME=pci-0000_23_00_0!` 和 GPU1 的 `CUDA_VISIBLE_DEVICES`，并在私有挂载命名空间内屏蔽其余三张卡的 NVIDIA 与 DRM 设备节点。采样同时用独立进程记录各卡实时利用率、GPU1 显存占用、宿主 1 分钟负载、Chromium GPU 进程所在卡和其他 GPU1 进程。完整记录在 `reports/m1-perf-raw/perf-RTX3080-ws-b6-gpu1-load.csv`。

沿用上文 M1 方法。Carentan `scene=world`，三处固定机位、每处 3 轮，60 个演示实体、植被开启，`debug=1`，每轮预热 10 s、采集 60 s，保留 Chromium 默认帧率限制。High 用 WebGPU，输出和内部均为 1920×1080。Low 用 WebGL2/ANGLE Vulkan，输出 1280×720，`?quality=low` 沿用已有地形 LOD 和植被配置，新增固定 75% 内部比例，实测画布 960×540。三处均使用现有占位场景，沿用历史命令的缺省阴影及后处理设置（自写 CSM、TAA、完整 HUD 未启用）。使用冻结时的采集源码提交 `26fbc46`。High 的采集器 `dirty` 标记来自确定性检查更新了历史 `reports/m0/determinism/summary.json`，该产物随后撤回；无运行时代码改动。渲染后端、分辨率和档位同时变化，因此本组数据不能单独量化某一项的收益。

P95 和卡顿是每轮完整的页面 rAF 间隔统计，FPS 为该轮 rAF 样本数除以实测采集时长。draw call 和三角形按既有工具取每轮末帧快照，表内为三轮中位数。卡顿定义为帧间隔严格大于 50 ms。原始 `vegetation.instances` 是末相机的可见网格快照，第一人称独立武器相机排在世界相机后，因此该字段的 `0` 不表示世界植被关闭；主表使用既有逐帧场景 draw call / active-index 计数。

| 档位 | 机位 | rAF P95 ms（3 轮） | rAF FPS（3 轮） | >50 ms（3 轮） | draw call | 三角形 |
| --- | --- | --- | --- | --- | ---: | ---: |
| High | open_field | 16.70 / 16.70 / 16.70 | 59.78 / 59.85 / 59.80 | 2 / 0 / 2 | 83 | 444,075 |
| High | hedgerow | 16.70 / 16.70 / 16.80 | 59.78 / 59.78 / 59.01 | 2 / 2 / 1 | 211 | 359,827 |
| High | old_town | 16.80 / 16.70 / 16.80 | 59.17 / 59.90 / 59.87 | 0 / 1 / 1 | 148 | 347,532 |
| Low | open_field | 16.70 / 16.70 / 16.80 | 59.88 / 59.97 / 60.02 | 1 / 0 / 0 | 71 | 143,173 |
| Low | hedgerow | 16.70 / 16.70 / 16.70 | 60.00 / 60.02 / 60.02 | 0 / 0 / 0 | 181 | 131,244 |
| Low | old_town | 16.70 / 16.70 / 16.70 | 60.02 / 60.02 / 60.02 | 0 / 0 / 0 | 116 | 125,572 |

- **High**：采集窗口内 216 个主机负载点。GPU1 利用率 25/38/45%（最小/中位/最大），显存 638/656/672 MiB；GPU0 / GPU2 / GPU3 利用率分别为 27/41/70% / 3/100/100% / 0/14/50%，宿主 loadavg(1m) 29.72/39.00/60.21。`pmon` 216 个点观察到 Chromium GPU 进程，均在 GPU1；未见其他 GPU1 进程。
- **Low**：采集窗口内 221 个主机负载点。GPU1 利用率 2/23/57%（最小/中位/最大），显存 506/510/832 MiB；GPU0 / GPU2 / GPU3 利用率分别为 0/42/82% / 67/100/100% / 0/30/55%，宿主 loadavg(1m) 20.23/26.68/63.40。`pmon` 221 个点观察到 Chromium GPU 进程，均在 GPU1；91/221 个点同时观察到非本批的 GPU1 进程；不能视为全程独占。

Low 的非独占窗口：hedgerow 第 1 轮 1/25 点、hedgerow 第 2 轮 1/25 点、hedgerow 第 3 轮 20/24 点、old_town 第 1 轮 22/25 点、old_town 第 2 轮 23/25 点、old_town 第 3 轮 24/24 点。其余 Low 轮未观察到其他 GPU1 进程。共用窗口原样保留，不将无归因的负载解读为本项目性能。

对照数值预算（不是 E1 参考机判定）：High 9/9 轮的 draw call 与三角形低于上限；P95 按 0.001 ms 去除浮点尾差后有 3/9 轮 >16.7 ms，有 0/9 轮超过 2 个卡顿帧。Low 有 9/9 轮达到 30 FPS，draw call 9/9 轮同时低于 900 和 500；有 0/9 轮 P95 >33.3 ms、0/9 轮超过 2 个卡顿帧。不同批次的宿主负载与客户端版本并非受控变量，不能将这些差异单独归因于 GPU 隔离，也不能把 rAF 的约 60 Hz 当作实屏呈现验证。

复现命令（本批先运行 `wasm-pack build crates/sim-wasm --target web --profile wasm-release`，再用 `pnpm -C client sync-map carentan` 同步地图和 sim-wasm；以下命令在上文的 GPU1 私有设备视图中运行，使用本机正常 ICD 搜索路径）：

```bash
export TMPDIR=/tmp/pg12 DRI_PRIME='pci-0000_23_00_0!'
export CUDA_VISIBLE_DEVICES=GPU-b64d67e4-4cc7-dcf0-5aa2-a16bd85586e3
node tools/perf/capture.mjs --m1 --gpu --renderer webgpu --viewport 1920x1080 --quality high --machine RTX3080-ws-b6-gpu1-high --veg 1 --out-dir reports/m1-perf-raw
node tools/perf/capture.mjs --m1 --gpu --renderer webgl2 --angle-vulkan --viewport 1280x720 --quality low --machine RTX3080-ws-b6-gpu1-low --veg 1 --out-dir reports/m1-perf-raw
```

本次聚合采用 `tools/perf/m1-report.mjs` 的 `rowFromCapture` 与 `tools/perf/budget.mjs`，结果追加于 `reports/m1-perf.json` 的 `B6-perf-gpu12`；六个采集文件保留在 `reports/m1-perf-raw/`。GPU 绑卡与负载记录不能替代参考机、最终素材、完整画质、分 pass GPU 计时及 wasm 内存验收；这些均未测。

旧版 `m1-report.mjs` 命令只重建历史的 WebGPU/植被 A/B；如单独复跑旧版聚合，请用 `--out` 输出到临时文件，不要覆盖本报告的追加数据。
