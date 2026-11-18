# M1 性能基准（M1-069b，工作站数字）

> **这是工作站数字，不是 RTX 3060 参考机（H-021）的数字**，不得据此判定 E1-6/E1-7（PI-109）。

## 结论

- `open_field` / `hedgerow` / `old_town`：**全部 未测**。客户端没有这三个场景入口和 60 实体的采集脚本；`tools/perf/m1-scenes.json` 机位仍是占位坐标；树篱/建筑渲染接线（M1-051b/052b）未完成。没有用别的场景顶替。
- E1-6 / E1-7 / E1-8 / 分 pass 预算 / 内存：**未测**。
- 补充：工作站 GPU + WebGPU 路径已打通，并在 M0 的 `?scene=vegetation` 三机位上采了 3 次 × 30 s（见下）。该场景没有实体、HUD、树篱、建筑，**不代表 M1 负载**。

## 机器

GPU NVIDIA GeForce RTX 3080（20 GB，Ampere，PCI 01:00.0，同时跑 Sunshine 串流桌面），驱动 595.58.03；2×EPYC 7402（96 线程）、405 GiB；Ubuntu 26.04、Linux 7.0.0-14；Chromium 153.0.8010.12、WebGPU（timestamp-query 可用）、1920x1080。采集时机器上还有其它进程占 GPU（nvidia-smi 显示各卡有 4–18 GB 已用），没有隔离。

## 补充数据：M0 vegetation 场景（WebGPU，3 次/机位）

垂直同步开（帧率锁 60，P50 恒 16.7 ms，看不出余量）：

| 机位 | 帧间隔 P50 ms | P50 极差 % | 帧间隔 P99 ms（3 次） | CPU P50 ms | GPU ms（timestamp-query，3 次） | draw call | 三角形 | JS 堆 MB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| cam0 | 16.7 | 0 | [17.9, 17.9, 17.9] | 2.9 | [0.36, 0.37, 0.36] | 50 | 450341 | 70.4 |
| cam1 | 16.7 | 0 | [18.2, 17.2, 17.5] | 3.3 | [0.27, 0.26, 0.27] | 33 | 9884 | 65.6 |
| cam2 | 16.7 | 0 | [19, 17.4, 19.6] | 2.6 | [0.21, 0.2, 0.3] | 13 | 318492 | 65.5 |

垂直同步关（`--novsync`，P50 极差超 5 % 的重复性判据：不满足，原因见下）：

| 机位 | 帧间隔 P50 ms | P50 极差 % | 帧间隔 P99 ms（3 次） | CPU P50 ms | GPU ms（timestamp-query，3 次） | draw call | 三角形 | JS 堆 MB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| cam0 | 2.3 | 21.7 | [13.3, 8.5, 17.5] | 2 | [0.35, 0.36, 0.35] | 50 | 450341 | 63.5 |
| cam1 | 2.7 | 25.9 | [7.7, 24.3, 10.4] | 2.1 | [0.31, 0.27, 0.28] | 33 | 9884 | 73.4 |
| cam2 | 3.2 | 78.1 | [19.1, 9, 22.4] | 2.1 | [0.21, 0.22, 0.21] | 13 | 318492 | 70.9 |

注意：
- 关 VSync 后 3 次 P50 极差 22–78 %，超过 README 的 5 % 判据；机器有 Sunshine 与其它任务干扰，且帧间隔已到 2–3 ms 量级，精度粗（0.1 ms 取整）。该组只能说明有数倍余量，不能当作基线。
- GPU 时间 0.2–0.4 ms/帧（timestamp-query），远低于 §13.5 的 14 ms 预算；该场景三角形 ≤ 0.45 M、draw call ≤ 50。
- RTX 3080 比 RTX 3060 强约 2 倍以上，所以这些数字对 3060 的外推只是下界方向的参考。

## 复现

```
node tools/perf/capture.mjs --gpu --renderer webgpu --viewport 1920x1080 --machine RTX3080-ws-vsync --out-dir reports/m1-perf-raw
node tools/perf/capture.mjs --gpu --renderer webgpu --viewport 1920x1080 --novsync --machine RTX3080-ws-novsync --out-dir reports/m1-perf-raw
```
原始 JSON：`reports/m1-perf-raw/`（commit 见本 PR）。`capture.mjs` 本次改动：`playwright` 解析失败时回退 `@playwright/test`；Linux `--gpu` 加 `--use-angle=vulkan --ignore-gpu-blocklist`（否则页面拿不到 WebGPU 适配器）；新增 `--novsync`。

## 仍需做（M1-069b 余项）

1. 参考机 RTX 3060 / Iris Xe 实测（H-021）。
2. M1 三机位入口、60 实体 + 60 s 脚本、`reports/m1-perf.json` 的 `budget.mjs` 行格式（`frame_ms.p95` 等）——待 M1-051b/052b 渲染接线后。
3. 占位比例统计（M1-064 素材接入后）。
