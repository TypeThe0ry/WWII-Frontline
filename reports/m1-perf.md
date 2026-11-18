# M1 性能基准（M1-069a，部分）

状态：**全部实测项 未测**。本次只交付判定工具与场景定义；没有参考机（RTX 3060 / Iris Xe，H-021），本机无 GPU，且 M1-051b/052b（树篱、建筑渲染接线）尚未完成，三机位无法在真实场景中采集。

| 判据 | 状态 |
| --- | --- |
| E1-6 High P95 ≤ 16.7 ms、draw call ≤ 1200、三角形 ≤ 4.0 M | 未测 |
| E1-7 Low P95 ≤ 33.3 ms、draw call ≤ 900 / 500 | 未测 |
| E1-8 材质变体 ≤ 60 | 未测（CI `variant-count` 尚无 M1 数字） |
| GPU 分 pass 预算、内存、卡顿帧 | 未测 |

工具：`tools/perf/budget.mjs`（阈值判定，`tools/ci/test/perf-budget.test.mjs` 8 项通过）、`tools/perf/m1-scenes.json`（三机位占位坐标，`_meta.status: initial`）。

占位比例：未统计（M1-064 素材整合被阻塞，所有 M1 视觉需求 ID 均为占位）；真素材接入后必须复测。
