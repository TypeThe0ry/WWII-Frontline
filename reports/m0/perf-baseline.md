# M0 性能基线（M0-030）

生成：2026-11-17。本机：4 核共享云虚机（Intel Xeon @ 2.30 GHz，15.7 GB，Linux 6.18），Node 22.23，Chromium 153.0.8010.12（Playwright，**SwiftShader 软件渲染**，无 GPU）。**这不是参考机**：所有"真 GPU 帧时间"一律 `未测（需参考机，H-021）`，不用本机数字顶替。

数据文件：`reports/baselines/perf-m0.json`（71 个指标，回归脚本的基线）、`reports/m0/perf/`（客户端采集的原始 JSON 与 `summary.md/json`）、`reports/baselines/{sim-m0,sim-m0-weapon,server-m0,bots-m0}.json`（Rust 微基准，各任务写入）、`reports/m0/loadtest-wt.json`（M0-027，未重跑）、`reports/m0/loadtest-wt-10min-after-rss-fix.json`（本任务，RSS 修正后重跑）。

## 1. E0-3 / E0-5 判定

| 项 | 判据 | 结果 | 判定 |
| --- | --- | --- | --- |
| E0-3 KCC 单次 | ≤ 20 µs | 中位 **19.22 µs**（本机） | 达标但余量 4 %；**非参考机，partial**（H-021：参考机复测） |
| E0-3 100 个 KCC 一 tick | ≤ 3 ms | 中位 **2.24 ms** | 达标（同上） |
| E0-5 RTX 3060 / 1080p / WebGPU | P50 ≥ 60 fps | — | **未测（需参考机，H-021）** |
| E0-5 Iris Xe / 720p / WebGL2 / Low | P50 ≥ 30 fps | — | **未测（需参考机，H-021）** |

软件渲染（SwiftShader）下的帧间隔（0.27–0.59 s，1.4–3.8 fps）只反映 CPU 模拟 GPU 的速度，**不得**用于 E0-5。

## 2. 客户端（软件渲染下客观可测的部分）

入口 `?renderer=webgl2&scene=vegetation&cam=0|1|2&debug=1&quality=high`，视口 640×360，预热 5 s、统计窗口清零后采集 30 s，每机位 3 次（`node tools/perf/capture.mjs --machine xeon4-swiftshader`，约 7 分钟）。三次取中位数，极差 = (max−min)/中位数。

| 机位 | draw call | 三角形 | 植被实例 | 植被 draw call | JS 堆 | 帧间隔 P50（软件渲染，不作判据） | P50 三次极差 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 地面朝北 | 50 | 450 341 | 8 130 | 49 | 58.4 MB | 588 ms | 2.9 % |
| 1 俯瞰 | 33 | 9 884 | 4 942 | 32 | 58.4 MB | 312 ms | 2.5 % |
| 2 块边缘朝西 | 13 | 318 492 | 3 197 | 12 | 58.5 MB | 271 ms | **7.9 %**（> 5 %，共享虚机干扰，见下） |

draw call 全部 ≤ 400（M0-024 预算），三机位三次采集的 draw call / 三角形 / 实例数**逐次完全一致**（固定机位 + 固定植被种子，确定性）。

每帧 CPU 各阶段（P50 / P99，ms，三次中位数；计时器分辨率 0.1 ms）：

| 机位 | game_update | terrain_update | vegetation_update | overlay | scene_render（含下两项） | ├ babylon 活动网格评估 | └ babylon 渲染遍历 | 整帧 CPU |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 0.0 / 0.1 | 0.3 / 0.6 | 0.1 / 0.2 | 0.2 / 0.5 | 0.7 / 3.1 | 0.2 / 0.4 | 0.4 / 2.2 | 1.3 / 3.6 |
| 1 | 0.0 / 0.1 | 0.3 / 1.1 | 0.1 / 1.1 | 0.2 / 1.4 | 0.6 / 3.5 | 0.2 / 1.3 | 0.3 / 3.2 | 1.2 / 4.2 |
| 2 | 0.0 / 0.1 | 0.3 / 1.0 | 0.1 / 2.5 | 0.2 / 0.7 | 0.5 / 2.0 | 0.2 / 0.4 | 0.2 / 1.5 | 1.1 / 3.7 |

对照预算（ARCHITECTURE §13.5）：CPU 帧 ≈ 10.4 ms（High，60 fps）——本机三机位 P50 1.1–1.3 ms / P99 ≤ 4.2 ms，**CPU 侧远低于预算**（M0 场景无角色、无网络；`game_update` 为 0，因为 `?scene=vegetation` 不连服务器）；地形更新 0.3 ms（预算 ≤ 2 ms）。GPU 侧（≤ 14 ms）未测。**前三个耗时来源**（CPU）：`scene_render` 0.5–0.7 ms（其中渲染遍历 0.2–0.4 ms）> `terrain_update` 0.3 ms > `overlay` 0.2 ms；GPU 分项只能在参考机上用 `gpu_ms` / Babylon Inspector 得到。

局限：①`gpu_ms` 在软件渲染下读到的是 `disjoint-timer-query` 对 SwiftShader 的计时（≈ 帧间隔），不是 GPU 时间；②cam 2 的 P50 极差 7.9 % 超过任务卡 5 %：同一虚机上别的租户负载不可控（见 M0-027 同样说明），其余两个机位 ≤ 3 %；③JS 堆来自 `performance.memory`（Chromium 非标准接口）。WebGPU 路径在本机只能通过 SwiftShader 适配器 + 离屏输出垫片编译/绘制，无像素校验，不采集（`capture.mjs --renderer webgpu` 可用，数字同样不可作 GPU 判据）。

## 3. Rust 微基准（本机；`cargo bench`，release，各自写 `reports/baselines/*.json`）

| 指标 | 实测（中位） | 预算 | 备注 |
| --- | --- | --- | --- |
| `kcc_step` | 19.22 µs | ≤ 20 µs | 余量 4 %，非参考机 |
| `kcc_100_players_tick` | 2.24 ms | ≤ 3 ms | |
| `ballistics_step`（400 弹丸 × 4 子步，阶段增量） | 0.312 ms | ≤ 2.5 ms | |
| 20 人连续开火 `World::step` | 0.092 ms | — | |
| AOI+编码，100 客户端，每 tick | P50 0.80 ms（P99 见 server-m0.json） | ≤ 2.6 ms | |
| 快照编码，每客户端每包（`snapshot_encode_client`） | 12.9 µs（100 客户端）/ 3.07 µs（10 客户端） | ≈ 25 µs | 含 AOI 选择，≈ 预算一半 |
| 快照解码（`snapshot_decode`） | **未测**：仓库没有这个基准（任务卡的 `cargo bench -p protocol snapshot_decode` 不存在，`crates/protocol/benches/` 为空）；解码在客户端 wasm 内，端到端解码错误数 0 见 M0-027 | — | 留给 M1-069 补基准 |
| bot think（60 bot，每 tick） | 0.069 ms | ≤ 3 ms | |

## 4. 服务器（来自 M0-027 的 `loadtest-wt.json`，未重跑；以及本任务 RSS 修正后的 10 min 重跑）

| 场景 | tick P50 / P99 (ms) | 下行 P50 (kbps) | 上行 P50 (kbps) | RSS 末 5 min 增长 |
| --- | --- | --- | --- | --- |
| M0-027：WT 30 min，10 headless + 10 bot（修正前） | 0.655 / 1.215 | 21.7 | 20.5 | 7.8 %（窗口 10 min，≈ 2.1–2.3 KB/s） |
| 本任务：WT 10 min，10 headless + 10 bot（修正后） | 0.719 / 1.415 | 21.8 | 20.4 | **1.6 %**（≈ 0.8–1.0 KB/s） |
| 基线（回归脚本用）：WT 120 s，40 headless + 10 bot | 1.96 / 3.52 | 38.4 | — | 斜率 1.98 KB/s（含起步阶段，30–120 s 窗口） |

预算：tick P99 ≤ 28 ms、下行 P50 ≤ 128 kbps、上行 ≤ 32 kbps——全部达标（同机共享虚机，非参考机）。

## 5. 包体积（`pnpm -C client build`，`client/dist`）

| 项 | 原始 | gzip -9 | brotli |
| --- | --- | --- | --- |
| JS（全部 chunk，含按需加载的调试/地形/植被/角色） | 2 132 940 B | 543 158 B | 470 431 B |
| sim-wasm | 766 470 B | 313 546 B | 244 676 B |
| HTML | 3 012 B | 774 B | 613 B |
| 地图资产（`dist/maps`：高度图 + 16 个碰撞块 + meta） | 2 437 322 B | — | — |

无独立 CSS 文件。预算（ARCHITECTURE §17.5 manifest 体积门禁 1 %）作用于后续的 manifest，M0 尚无。

## 6. M0-030 内修正的三个问题与数字

1. **低帧率客户端被误踢**：见 PLAN_ISSUES PI-029。真服务器 + Chromium，主线程每帧阻塞 280 / 330 / 700 ms（帧率 3.5 / 3 / 1.4）60 s（700 ms 档 50 s）全部不被踢（`client/e2e/low-fps.spec.ts`）；修正前复现了根因条件：RTT EWMA 升到 1062 ms、输入提前量升到 16 tick（服务器窗口 10 tick）。
2. **WT 服务器 RSS 增长**：massif（20 个 WT 客户端 150 s）存活堆 6.89 → 7.36 MB（≈ 3.6 KB/s），增长点为 `SnapshotScheduler` 的诊断样本向量；修正后同条件存活堆 6.84 → 6.89 MB（≈ 0.3 KB/s，即退出报告用的 tick 样本向量，之后改为定容蓄水池）。100 个 WT 客户端 200 s 的 RSS 线性斜率：**13.7 → 0.6–0.7 KB/s**；10 + 10 bot 10 min：**≈ 2.2 → ≈ 0.9 KB/s**（`server_rss_growth_pct` 7.8 % → 1.6 %，验收线 2 %）。残留约 0.8 KB/s：存活堆（massif）已平，其中 0.24 KB/s 是退出报告样本池的填充（32 768 个样本，约 18 min 后停止），其余未能进一步归因（分配器/线程栈层面，不是存活对象），建议 M0-033 做一次 30 min 复测确认斜率归零。
3. **浏览器死亡/重生**：`client/src/game/death-respawn.test.ts`：修正前死亡 5 s 内本地模拟多走 33.7 m、整段 53 次纠正；修正后死亡期间 0 m、总纠正 6 次（启动同步 1 + 死亡 1 + 重生 1 + 重生后 3 次小纠正，均在重生后 2 s 内），重生时的纠正是 1 次 snap。

## 7. 回归脚本

```
node scripts/perf-regress.mjs check [--quick] [--skip client,bundle,rust,server]   # 测量 + 与 reports/baselines/perf-m0.json 比较；超容差退出码 1
node scripts/perf-regress.mjs measure --out FILE        # 只测量
node scripts/perf-regress.mjs compare --current FILE [--strict]
node scripts/perf-regress.mjs update-baseline [--from FILE]   # 有意改变性能后，人工采纳新基线
node --test "tools/perf/*.test.mjs" "scripts/perf-regress.test.mjs"
```

完整测量实测约 10 分钟（598 s：客户端 7 分钟 + 打包 + 4 个 Rust 基准 + 2 分钟服务器压测），`--quick` 实测 167 s（客户端每机位 1 次 × 12 s；跳过 4 个 Rust 基准里的 3 个，故 6 个指标"未测"并跳过）；`--quick` 的容差判定与完整模式相同。容差写在基线 JSON 的 `rules`：计数类（draw call、三角形、植被实例）±2 %；包体积 +5 %；CPU 阶段耗时 +100 % + 1 ms（共享虚机 + 0.1 ms 计时器，只拦数量级退化）；Rust 微基准 +25 %（亚 0.1 ms 的 +60 %）；服务器 tick +100 % + 0.5 ms，下行带宽 ±15 %；RSS 斜率基线 + 2 KB/s（健康服务器在 75–120 s 窗口里因起步台阶读到 2–2.8 KB/s；修正前 40 客户端约 5.5 KB/s，由 100 客户端的 13.7 KB/s 外推、未单独测）。人为变慢的演示（把 `kcc_step` ×1.5、JS 包 +10 %、RSS 斜率 6.5 KB/s 写进当前测量）→ 3 项 FAIL，退出码 1。`.github/workflows/bench.yml`（手动 + nightly，不进 PR 必跑）跑其中的 Rust 部分并上传 `target/perf-current.json`。真 GPU 帧时间不在回归脚本里（`gpu_unmeasured` 字段占位），在参考机上用 `tools/perf/capture.mjs`（流程见 `tools/perf/README.md`）。

## 8. 参考机复测入口（H-021）

`tools/perf/README.md`：RTX 3060 / 1080p / WebGPU：`node tools/perf/capture.mjs --gpu --renderer webgpu --viewport 1920x1080 --machine rtx3060`；Iris Xe / 720p / WebGL2 / Low：`--gpu --renderer webgl2 --viewport 1280x720 --quality low --machine irisxe`；然后 `node tools/perf/report.mjs`。把 `reports/m0/perf/` 下的 JSON 与 GPU 型号 / 驱动版本一并交回，由本报告 §1 补判定。同一台机器上的 `cargo bench -p frontline-sim --bench kcc` 与服务器压测复测 E0-3。
