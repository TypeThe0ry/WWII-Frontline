# M0 里程碑验收报告（M0-033）

验收日期：2026-11-17。依据：`docs/MILESTONES.md` §1.3 退出标准 E0-1…E0-12、§1.4 演示脚本、`docs/ARCHITECTURE.md` §20.1 五项首要风险、`tasks/M0-033-m0-acceptance.md`。
验收对象：提交 `a9d1dc4`（M0-030）之后的工作区，加上本任务的最小修补（见 §3.3）。
测量环境：**本机 4 核共享云虚机（Intel Xeon @ 2.30 GHz，15.7 GB，Linux 6.18），无 GPU，Chromium 153 软件渲染（SwiftShader）——不是参考机**。凡标"未测"的项，原因都是缺参考机、缺录像、缺 GitHub 实跑或缺受信任证书，**没有任何数字是估计或编造的**。

## 1. 总结论

**M0 状态：`partial`（可带条件关闭）。**

- 代码侧：协议、sim、server（30 Hz、WS+WT、会话、AOI/快照）、bots、headless-client、sim-wasm、客户端（引擎、网络、预测对账、CDLOD 地形、植被、灰盒角色与射击反馈）全部实现，**本机从干净状态全量复验通过**（§3：434 个 Rust 测试、238 个 vitest、各 Playwright 套件、确定性、体积、依赖/纯净度检查、数据校验、两套 pytest 全绿；`cargo fmt`、`clippy -D warnings` 干净）。
- 退出标准 12 项：**通过 5**（E0-1、E0-2、E0-4、E0-9、E0-11）、**部分 5**（E0-3、E0-6、E0-7、E0-8、E0-10）、**需参考机 / 人工 2**（E0-5、E0-12）。无一项"未通过"。
- 判定规则（MILESTONES §1.3）：E0-1～E0-6 中 E0-3、E0-5、E0-6 未能完全通过 → M0 记 `partial`，PLAN_ISSUES 已登记（PI-030、PI-005/014 等），**由 Fable 在检查点 0 裁决**；E0-7～E0-12 的缺口登记为 M1 前置待办（§6）。
- 关闭条件（建议）：①Fable 审阅（H-038）接受本报告；②E0-3 / E0-5 / E0-6 的真 GPU / 参考机复测（H-021）在 **M1 渲染任务（M1-043/044/049）开工前**完成，或 Fable 接受 PI-030 的判据调整；③H-102（GitHub 主分支与 CI 实跑）在 M1 第一周内解除；④E0-12 的录像（H-011/025/026）最迟在 M1 退出前回填。以上都不阻塞 M1 的服务器/网络/数据/素材管线类任务开工。
- 本任务自身状态：**partial**（按判定规则；不得写 done）。

## 2. 退出标准对照（E0-1…E0-12）

| # | 标准 / 阈值 | 实测值（本机，非参考机） | 来源 / 证据 | 结论 | 缺口与解除方 |
|---|---|---|---|---|---|
| E0-1 | 10 headless + 10 bot × 30 min；`tick_ms` P99 ≤ 28 ms；无连续 2 tick > 33 ms | **P50 0.745 / P99 1.349 / max 9.93 ms**；54 090 tick；overrun 0（连续最大 0）；0 断线、0 解码错误；击杀 564 / 重生 563 | 本任务重跑 `scripts/loadtest-m0.sh --transport wt --duration 1800`（release，服务器绑 CPU 0-1、客户端绑 2-3）→ `reports/m0/acceptance/loadtest-wt-30min.json`；M0-027 的首次 30 min：P99 1.215 ms（`reports/m0/loadtest-wt.json`） | **通过** | 参考机 / 独占机复测为加分项；100 headless + 10 bot（M0-027 矩阵）P99 8.3 ms 仍远低于 28 ms |
| E0-2 | 下行 P50 ≤ 128 kbps；上行 ≤ 32 kbps | 下行 **P50 21.9**（最差客户端 22.5）/ P99 26.9 kbps；上行 **P50 20.5**（最差 20.9）kbps（线路估计口径 = 负载 + 58 B/包；仅负载下行 P50 约 16） | 同上 JSON `checks.down_p50_each_client / up_p50_each_client`；M0-019：100 客户端全员 P50 59.7 / P99 91.6 kbps | **通过** | — |
| E0-3 | KCC 单次 ≤ 20 µs；100 个 KCC 一 tick ≤ 3 ms | 单次中位 **19.27 / 20.18 / 19.94 µs**（三次独立运行，基准自判 PASS/FAIL/PASS；`perf-regress` 内一次 20.81 µs）；100 人 `World::step` 中位 **2.24 / 2.29 / 2.26 ms**（p99 3.2–3.9 ms） | `cargo bench -p frontline-sim --bench kcc`（release，`FRONTLINE_BASELINE_DIR` 指向临时目录，不改基线）；基线 `reports/baselines/sim-m0.json`；PI-030 | **部分** | 单次判据在共享虚机上零余量、落在噪声内；100 人 tick 达标。参考机复测（H-021）或由 Fable 按 PI-030 调整判据；不建议启用 ARCH §5.3 降级 |
| E0-4 | native/native、wasm/wasm 逐 tick `state_hash` 一致；native vs wasm RMS 漂移 < 2 cm/s（信息性） | 3 种子 × 18 000 tick：native×2、wasm×2 各 18 000/18 000 一致，10 玩家 World×2 一致；**native vs wasm 逐 tick 哈希与位置逐位相同，漂移 rms 0 / max 0 / 0 cm/s**；故障注入在第 700 tick 被检出 | 本任务重跑 `tools/determinism/run.sh --strict`（72 s，退出 0；wasm 为干净重建的产物）→ `reports/m0/determinism/summary.json` | **通过** | CI 实跑与墙钟（H-102）；`determinism.yml` 已改为默认严格（PI-031） |
| E0-5 | RTX 3060 / 1080p ≥ 60 fps；Iris Xe / WebGL2 / 720p ≥ 30 fps（P50） | **未测**。仅有软件渲染下可测的部分：固定三机位 draw call 50/33/13（≤400）、植被实例 8 130/4 942/3 197、整帧 CPU P50 1.1–1.3 ms / P99 ≤ 4.2 ms（预算 ≈ 10.4 ms）；本任务 `perf-regress --quick` 71 项指标 0 项超容差 | `reports/m0/perf-baseline.md`、`reports/baselines/perf-m0.json`；本任务 `perf-regress check --quick` 退出 0 | **需参考机** | H-021（复测入口见 `tools/perf/README.md`）。软件渲染帧率（1.4–3.8 fps）不得顶替 |
| E0-6 | Babylon `WebGPUEngine`+`ComputeShader` 冒烟通过；Hi-Z/froxel/TAAU 可行性结论 | SwiftShader WebGPU 上 `__smoke.ok=true`、mismatches 0；Hi-Z 链、froxel（921 600/921 600）、TAAU 占位 pass 三试验通过；结论：**可行，Hi-Z 需自写薄封装**（逐 mip 视图、深度纹理绑定、indirect draw 三个缺口留待 M1 spike）；本任务重跑 e2e 的 WebGPU 用例（render-bootstrap / terrain / vegetation 各一）全部通过 | `reports/m0/compute-feasibility.md`；PI-005；主 Playwright 配置 44 通过 | **部分** | 真 GPU 上未测（H-021）；复测入口 `/?renderer=webgpu&smoke=compute&debug=1`；M1-043/044 开工前各做 ≤ 1 天 spike |
| E0-7 | Chromium：WT 与 WS 均通过；Firefox/WebKit 如实登记；WT 禁用 ≤ 5 s 回退 WS | Chromium 153：**WT 通过**（201 快照/10 s，RTT 中位 1.1 ms）、**WS 通过**（151 快照/10 s）、删除 `WebTransport` → 回退 WS **10 ms**、WT 端口关闭 → 回退 12 ms；UDP 黑洞（QUIC 永不完成）→ 3.2 s 后回退，WS 握手 ≤ 5 s；Reject 不回退、断线可重连。Firefox / WebKit：**"未验证（本机未安装）"**；C2 来源为真实服务器快照（M0-028 要求 M0-019 后复跑，本任务已复跑） | `reports/m0/browser-matrix.{json,md}`（已重生成，`scripts/check-interop-table.mjs` 对 STATUS 行已同步）；`net.spec.ts` 11/11 | **部分** | Firefox / Safari 手工验证 H-103；受信任证书与域名 H-017 |
| E0-8 | 100 ms 延迟 + 2 % 丢包下静止校正 ≤ 1 次/10 s；本地开火与枪口焰同帧 | 进程内等价模型（RTT 100 ms、丢 2 % 双向、60 s 走动）：稳态纠正 0、总 1 次、snaps 0、最大误差 0.034 m；静止 120 s 1 次；**开火与枪口焰同帧 30/30**（vitest + e2e）；演示中真服务器 + 真浏览器（RTT 2 ms）静止/走动纠正 0 | M0-022、M0-025 的 STATUS 行；`client/src/game/reconcile.test.ts`；本任务演示（§4） | **部分** | **`tc netem` 真实注入未做**：本机没有 `tc` / `ip` 命令；需受限网络测试台（H-022）或装有 iproute2 的机器；M1-068 复用 `scripts/loadtest-m0.sh`。软件渲染低帧率场景下纠正数无参考意义 |
| E0-9 | `sim-wasm` ≤ 1.2 MB（br ≤ 400 KB） | wasm-opt 后 **766 470 B**（限 1 200 000）；gzip-9 313 546 B；**brotli-11 244 676 B**（限 400 000） | 本任务干净重建后 `node tools/ci/check-wasm-size.mjs` 退出 0 | **通过** | — |
| E0-10 | CI：lint / test-rust / test-ts / wasm-build 全绿；依赖方向违规 PR 被拒 | 本地等价逐项全绿（§3）；`check-deps` OK（依赖图与 ARCH §4.5 相等）、`check-sim-purity` OK（0 豁免）、`node --test tools/ci/test` 13/13（含"违规依赖被拒"夹具） | §3 | **部分** | GitHub 上从未实跑（无 main 分支、无法开 PR，H-102）：Actions 墙钟与真实红/绿 PR 未取得 |
| E0-11 | /data 校验通过；两把枪 historical 字段 100 % 有出处或"未核实" | `data:validate`：9 个文件、0 错误；`data:test` 51/51；15 个 historical 字段：13 已核实 + 2 登记 `_meta.unverified`（100 % 有出处或"未核实"）；LICENSES 台账无 `PENDING` 素材进入主分支（仅 IBM Plex 两条字体记录为 PENDING，且无字体文件入库，PI-009） | `reports/m0/hist-review-m0.md`（M0-031）；本任务 §3 | **通过** | — |
| E0-12 | 对标测量批 M0 每项有记录；复测误差在容限；无录像则 `blocked` | 工具链就绪：`pytest tools/ref-analysis` 69/69；合成数据测回误差：射速 0.006 %、速度 <0.001 %、FOV 误差 0.016°；批 M0 目录 78 行齐全；我方同法复测 `own_m0.{json,md}` 已出；**参考作品测量值全部"待测"** | M0-032、`docs/measurements/CAPTURE_GUIDE_M0.md`、PI-027 | **需人工（blocked）** | 录像：H-011（EULA）、H-025（设备）、H-026（三个录像会话），汇总 H-104；到位后由 M0-032b 回填；M1 退出前必须完成 |

**统计：通过 5 / 部分 5 / 需参考机或人工 2 / 未通过 0。**

## 3. 全量检查结果

### 3.1 干净状态复验流程

`df -h` 19 GB 可用 → `cargo clean`（释放 5.4 GB）→ 删除 `data/maps/m0` 下全部二进制与 `client/public/maps` → 按 DEVELOPING.md 重生成：

| 步骤 | 命令 | 结果 | 耗时 |
|---|---|---|---|
| 地形重生成 + 校验 | `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0` + `validate` | `validate: ok`；`heightmap_1m.r16` md5 与重生成前一致（`afd3fae2…`，确定性） | 0.7 s |
| sim-wasm（wasm-release） | `wasm-pack build crates/sim-wasm --target web --profile wasm-release` | 成功 | 45 s |
| 同步到客户端 | `pnpm -C client sync-map` | 20 个地图文件 + wasm 包 + 占位数据 | 1 s |

### 3.2 检查矩阵

| # | 检查 | 命令（要点） | 结果 | 耗时 |
|---|---|---|---|---|
| 1 | rustfmt | `cargo fmt --check` | 首次**失败**（`crates/server/src/net/conn.rs` 一处，M0-030 已报）→ 已修，复跑退出 0 | 1 s |
| 2 | clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 警告，退出 0 | 36 s |
| 3 | Rust 测试 | `FRONTLINE_REQUIRE_TERRAIN=1 cargo test --workspace --features frontline-sim/test-util`（debug，含干净构建） | **434 通过 / 0 失败 / 10 忽略**（忽略项为 release-only 性能与长跑测试，另行如下） | 253 s |
| 4 | release 性能测试 | `cargo test --release -p frontline-server --test step_overhead` | 2/2 通过（`hundred_players_empty_world_p99_under_1ms` 等） | 18 s（含构建） |
| 5 | release 构建 | `cargo build --release -p frontline-server -p frontline-headless-client` | 成功 | 89 s |
| 6 | KCC / World::step 基准 | `cargo bench -p frontline-sim --bench kcc` ×3 | 见 E0-3（19.27 / 20.18 / 19.94 µs；2.24–2.29 ms） | — |
| 7 | 依赖矩阵 | `node tools/ci/check-deps.mjs` | OK | <1 s |
| 8 | sim 纯净度 | `node tools/ci/check-sim-purity.mjs` | OK（0 豁免） | <1 s |
| 9 | 目录布局 | `node tools/ci/check-layout.mjs` | OK（24 目录、4 文件） | <1 s |
| 10 | CI 脚本测试 | `node --test tools/ci/test/*.test.mjs` | 13/13 | 1 s |
| 11 | 数据校验 | `pnpm data:validate` / `pnpm data:test` | 0 错误（9 文件，2 个无 schema 仅哈希）/ 51/51 | 10 s |
| 12 | 客户端 lint / typecheck | `pnpm -C client lint` / `typecheck` | 退出 0 / 0 | 4 s / 5 s |
| 13 | 客户端单测 | `pnpm -C client test` | 34 文件 238 用例通过 | 7 s |
| 14 | 客户端构建 | `pnpm -C client build` | 成功（>500 kB chunk 警告，brotli 总 JS 470 KB，见 perf-baseline） | 4 s |
| 15 | Playwright 主配置 | `pnpm -C client test:e2e` | 首次**整体失败**（vitest 文件被误收，见 §3.3）→ 修正后 **44 通过 / 8 跳过（SAVE_SHOTS 截图刷新等自带 skip 条件）/ 0 失败** | 463 s |
| 16 | Playwright net | `FRONTLINE_REQUIRE_SERVER=1 … -c playwright.net.config.ts` | 11/11 | 17 s |
| 17 | Playwright game（含 low-fps） | `… -c playwright.game.config.ts` | 6/6（含"≤5 fps 客户端 60 s 不被踢"） | 127 s |
| 18 | Playwright interop | `scripts/run-interop.sh` | Chromium 4/4；Firefox / WebKit 6 项"未验证（未安装）"；`check-interop-table.mjs` 对 STATUS 同步 | 43 s |
| 19 | 确定性 | `bash tools/determinism/run.sh --strict` | 3 种子 × 18 000 tick 全一致，漂移 0（E0-4） | 72 s |
| 20 | wasm 体积 | `node tools/ci/check-wasm-size.mjs` | OK（E0-9） | 2 s |
| 21 | 性能回归 | `node scripts/perf-regress.mjs check --quick` | 退出 0；71 项指标 0 超容差，6 项（Rust 基准，quick 模式跳过）未测 | 200 s |
| 22 | Python 测试 | `cd tools/map-pipeline && python3 -m pytest` / `cd tools/ref-analysis && python3 -m pytest` | 21 通过 1 跳过 / 69 通过 | 2 s / 28 s |
| 23 | 30 min WT 压测 | `scripts/loadtest-m0.sh --transport wt --duration 1800` | ALL OK（E0-1、E0-2、RSS 见 §5） | 1800 s |
| 24 | 演示脚本 | §4 | 见 §4 | 64 s |

任务卡写的 `python3 -m pytest tools/map-pipeline tools/ref-analysis` 单次调用会因两个 `tests/` 目录同名与 `ref_analysis` 包路径而在收集阶段失败（`ModuleNotFoundError: ref_analysis`），**须分目录运行**（上表 #22），这不是测试失败。

### 3.3 验收中发现并做的最小修补（详见 PI-031）

1. `crates/server/src/net/conn.rs`：一处遗留 rustfmt 差异，已格式化（仅换行）。
2. `client/playwright.config.ts`：加 `testIgnore: ['**/*.test.ts']`。自 M0-028 起主 Playwright 配置会把 vitest 文件 `e2e/interop/fixtures.test.ts` 当成用例载入并整体报错，导致 `pnpm -C client test:e2e` 退出 1；修后见上表 #15。
3. `.github/workflows/determinism.yml`：PI-019 裁定的"CI 默认严格模式"落实（`DETERMINISM_STRICT=0` 或手动触发取消勾选才降为警告）。
4. `DEVELOPING.md`：本地演示推荐 URL 的 `scene=empty` 更正为 `empty=1`（代码只认 `empty=1`）。
5. 重生成的跟踪产物：`reports/m0/browser-matrix.{json,md}`、`reports/m0/M0-021-net-e2e.json`、`reports/m0/determinism/summary.json`（数据哈希与 wasm 哈希随当前数据/重建更新）。
6. 未改任何功能代码与测试断言；无删除、跳过或放宽。

### 3.4 新发现的问题

- **PI-030**：KCC 单次 ≤ 20 µs 在本机零余量（§2 E0-3）。
- 无新的功能缺陷。演示中确认：Garand 半自动，按住左键只发一枪（需松开再按）、冲刺中不能开火——均为设计行为，不是缺陷。

## 4. 演示脚本（MILESTONES §1.4）

走法：自写的一次性脚本（Playwright + 程序化 Vite，不入库），服务器 `target/release/frontline-server --bots 10`，随机端口（WS 45575 / WT 48114 / metrics 36497 一类，每次运行不同），开发证书指纹固定（`certhash`），全部进程由脚本自己启动并按句柄结束，未碰 7000/7001/9100/5173 上的演示服务器。日志：`reports/m0/acceptance/demo-log.txt`；截图同目录。

| 步 | 内容 | 结果 | 证据 |
|---|---|---|---|
| 1 | 启动服务器（`--bots 10`，`--map m0 --mode sandbox` 即默认值）；Chrome 打开客户端，叠层显示 FPS / GPU ms / RTT / 传输 | **通过**：叠层 `FPS 60.0 · GPU 9.1 ms（disjoint-timer-query，软件渲染）· net wt rtt 2 ms jitter 5.9 ms · entities 10`（10 个 bot）；传输 `wt`（`session.kind='wt'`） | `step2-wt-play.png`、`demo-log.txt` |
| 2 | 10 bot 巡逻交火；本地玩家走、冲刺、蹲、跳、开火 Garand，看到 bot 与命中标记，被击杀后重生 | **通过（含说明）**：真浏览器经 WT：走 2.5 s 前进 10.5 m（≈4.2 m/s）、冲刺 2.5 s 前进 15.9 m（≈6.4 m/s）、跳起 ≥ 0.43 m、蹲 1.2 s、开火 1 发（弹匣 8→7、`fireLocal` 0→1）；纠正 0 次；远处可见 bot 的灰盒轮廓。**被击杀后重生**未在浏览器里手工复现（本地玩家未被 bot 击中）；该路径由 headless 30 min（击杀 564 / 重生 563、被动死亡后冻结预测）与 `death-respawn.test.ts` 覆盖。真实弹道下坠在 sim 测试里验证（500 m 下坠对 RK4 误差 2 %），浏览器里近景 bot 太远/无弹着点可视化未单独截图 | `step2-wt-fire.png`、`demo-log.txt` |
| 3 | `tc netem` 注入 100 ms + 2 % 丢包，叠层显示校正次数 | **无法执行**：本机没有 `tc` / `ip`（iproute2 未安装、无网络安装）。叠层的"reconcile corrections"行已在显示（演示里恒为 0）。等价证据：进程内链路模型（RTT 100 ms / 丢 2 %）与 headless 对账 60 s 稳态 0 次纠正（M0-022 / M0-027） | `demo-log.txt` |
| 4 | 禁用 WebTransport → 自动回退 WebSocket，叠层传输变为 WS | **通过**：删除页面的 `WebTransport` 后 `transport=auto` 在 841 ms（自导航起）内连上，`session.kind='ws'`，叠层 `net ws rtt 4 ms`；真实"UDP 黑洞"回退 3.2 s（e2e） | `step4-ws-fallback.png` |
| 5 | 强制 WebGL2 重载，同场景渲染成功，叠层显示帧时间 | **通过**：地形 + 植被场景（`scene=vegetation&renderer=webgl2`）软件渲染 2.8–3.2 fps，CPU 帧 P50 4.3–4.5 ms，draw 20–25、三角形 33 万–35 万；注意软件渲染帧率不是 E0-5 数字 | `step5-webgl2-terrain-veg.png` |
| 6 | 10 个 headless-client 并展示 `/metrics` | **通过**：10 headless + 10 bot 的 30 min 运行里每 10 s 抓取 `/metrics`（tick 直方图、每客户端带宽直方图、`server_players/bots`），汇总在 `loadtest-wt-30min.json`；演示服务器的 `/metrics` 全文抓取见 `step6-metrics.txt`（252 行，含 `server_tick_ms`、`net_bytes_out_per_client_kbps`、`aoi_encode_ms`、`server_bots 10`、`session_kicked_total 0`） | `loadtest-wt-30min.json`、`step6-metrics.txt` |
| 7 | 对照 STATUS 的 E0-1…E0-12 | **通过**：见 §2，`tasks/STATUS.md` 摘要同步 | 本报告 |

## 5. 30 分钟 WT RSS 复测（M0-030 交接项）

场景：release，10 headless（WT）+ 10 bot，1800 s，服务器绑 CPU 0-1，客户端绑 2-3；每 10 s 取服务器 RSS（181 个样本）。

| 项 | 数值 |
|---|---|
| RSS 起点 → 终点 | 12.10 MB（t=0）→ 14.63 MB（t=1802 s）；预热后 t=120 s 13.43 MB |
| **最后 10 min 增长（窗口 591 s）** | **+1.951 %（14.012 → 14.285 MB）**，验收线 2 % → **通过，但余量只有 0.05 个百分点** |
| 线性斜率（最小二乘） | 0–10 min 1.29 KB/s；10–20 min 0.71 KB/s；**20–30 min 0.48 KB/s**；全程 0.73 KB/s |
| 曲线形态 | 每 2 min 的增量：+1 328（起步）、+184、+96、+132、+84、+96、+84、+88、+68、+66、+56、+60、+60、+56 KB——**单调递减、对数型而非线性**，没有出现恒定斜率的泄漏迹象 |
| 同一次运行的其它判据 | tick P99 1.349 ms、下行 P50 21.9 kbps、0 断线、0 解码错误、`ALL OK` |

结论：M0-030 修掉的两处累积（调度器诊断样本、退出报告样本）使 WT 路径的 RSS 不再线性增长；残留约 0.5 KB/s 且在继续衰减（与 M0-030 估计的"0.24 KB/s 为报告样本池填充 + 分配器预热"一致，样本池约 18 min 后停止填充）。**按 2 % / 10 min 的验收口径通过**；该口径对 1.95 % 的结果很敏感，建议 M1-068 把门禁改成"最后 10 min 的最小二乘斜率 ≤ 1 KB/s 且逐 10 min 递减"，并在 ≥ 1 小时的 soak 里确认斜率归零（见 §7 建议 8）。原始曲线：`reports/m0/acceptance/loadtest-wt-30min.json` 的 `rss_samples`。

## 6. ARCHITECTURE §20.1 五项首要风险

| # | 风险 | 结论 | 证据 | 是否建议改架构 |
|---|---|---|---|---|
| 1 | `wtransport` datagram 稳定性与浏览器互通（M0-017 / M0-028） | **带条件通过** | 100 WT 客户端 × 120 s：上/下行 240 100/240 100（100 %）、0 意外断开、RTT P50 1.35 / P99 5.50 ms（M0-017）；30 min 10+10 无断线；Chromium WT / WS / 回退全过（§2 E0-7）。**条件**：Firefox / Safari 未验证（H-103）；受信任证书未取得（H-017，目前只能用 13 天开发证书指纹） | 否。WS 兜底已验证（3.2 s 内回退、握手 ≤ 5 s），WT 作为加速路径的策略站得住 |
| 2 | Rapier `enhanced-determinism` KCC ≤ 20 µs（M0-011） | **带条件通过** | 中位 19.3–20.8 µs，100 人 step 2.24–2.29 ms（预算 3）；确定性全过；服务器 tick P99 1.2–8.3 ms 远低于 28 ms。条件：参考机复测或调整判据（PI-030） | 暂不建议启用 §5.3 降级；若参考机上稳定 > 20 µs，优先做 KCC 局部优化而不是降级 |
| 3 | WASM 预测与原生漂移 < 2 cm/s（M0-029） | **通过** | 3 种子 × 18 000 tick native/wasm 逐位一致，漂移 0 cm/s（本任务干净重建后复测）；浏览器真服务器对账演示 0 次纠正 | 否。R5"不把 native vs wasm 比特一致当阻断项"的容限设计保留，CI 默认严格已落实（可随时降级为警告） |
| 4 | Babylon `WebGPUEngine` + `ComputeShader` 下 Hi-Z / froxel / TAAU 可行性（M0-020） | **带条件通过** | SwiftShader 上冒烟通过，三试验通过；需自写薄封装（逐 mip 视图、深度纹理绑定、indirect draw 三缺口，PI-005）；真 GPU 未测 | 否，但 **compute 路线按 PI-005 走"自写薄封装"**，且 M1-043/044 开工前必须先做 spike 与参考机冒烟 |
| 5 | 单块地形 + 植被帧率（M0-024 / M0-030） | **未测（需参考机）；CPU 侧通过** | CPU 整帧 P50 1.1–1.3 ms（预算 ≈ 10.4 ms）、draw call ≤ 50（≤ 400）、地形选择 0.08 ms、重分桶 P99 0.3 ms；GPU 帧时间未测（H-021） | 暂无证据支持改架构；若参考机上 RTX 3060 / 1080p < 60 fps，先走 §5.3/§13.5 的画质档位降级 |

## 7. Fable 检查点 0 材料（MILESTONES §1.10）

1. **五项风险结论**：见 §6。
2. **预算表是否被支持（ARCH §6.10 / §6.9.5 / §13.5 / §4.3.3）**：支持，且余量很大——tick P99 1.2–1.35 ms（10+10，预算 28）、8.3 ms（100 headless + 10 bot）；下行 P50 22 kbps（10+10，预算 128）、100 客户端 P50 60–68 kbps；AOI + 编码 100 客户端 P99 1.44 ms（预算 2.6）；100 人 step 2.24 ms（预算 3，余量 25 %）；bot think 100 个 P99 0.67 ms（预算 3）；wasm 245 KB brotli（预算 400）。**唯一紧的是 KCC 单次**（PI-030）。无需重新分配预算；GPU 侧（§13.5）无数据。
3. **任务粒度校准**：仓库没有逐任务的"会话数"记录，无法给出数字，不编造。能从 STATUS 看到的信号：体量明显超一次会话的是 M0-018/019/022/025/027/030（STATUS 行实测文字均 >2 000 字，且 M0-030 还带三项交办修正）；多次回访的是 M0-011（KCC 复测）、M0-028（M0-019 后须复跑）、M0-027→M0-030（RSS/误踢）。建议 M1 的 L 级任务在任务卡里预先拆出"实现 / 性能与长跑验收"两段，并让长跑验收任务晚于其依赖的实现任务。
4. **对标测量工具链**：可用（`pytest` 69/69，合成数据测回误差远小于容限；批 M0 78 行目录；我方复测已出）。**M1 开工前不必须补录像**（只缺参考作品一侧的数字，我方数值全部为 `initial`/待调参），但 **M1 退出前必须回填**；若 M1-011/013 的手感调参想以参考数字为准，应在这两个任务开始前拿到批 M0 录像。
5. **M0 发现的新依赖 / INDEX 缺口**：`rapier3d =0.36.1`（enhanced-determinism）、`wtransport =0.7.2`、`bevy_ecs =0.16.1`（锁版本，PI-011）、sim 的 `serde/serde_json/sha2`（PI-004）；工具：wasm-pack 0.15.1、wasm-bindgen-cli 0.2.130、binaryen version_132、（可选）valgrind；INDEX 未列的产出：`data/rules/*.json` 与 `rules.schema.json`（PI-017）、`data/placeholders/`（PI-025）、`data/rules/bots.json`（PI-026）、`scripts/perf-regress.mjs` 与 `reports/baselines/perf-m0.json`（M0-030）、`M0-032b`（录像到位后的回填任务，STATUS 已引用但 INDEX 没有）。建议先改 INDEX 再改 STATUS。

## 8. 遗留问题清单

### 8.1 partial 任务各自缺什么

| 任务 | 缺什么 | 解除方 |
|---|---|---|
| M0-002 CI 基线 | GitHub 上实跑（真实红/绿 PR、Actions 墙钟）；`cargo deny` advisories 未验证 | H-102 |
| M0-004 数据校验 | CI 墙钟与"坏 JSON 的 PR 被拒"需 GitHub 实跑 | H-102 |
| M0-011 KCC | 单次 ≤ 20 µs 需参考机复测（本机 19.3–20.8，PI-030） | H-021 / Fable |
| M0-015 server 骨架 | tick 间隔抖动 P99 < 1 ms 在共享虚机 2.83 ms（复测 0.79 ms）；30 min 压测里 `tick_interval_error` P99 0.018 ms | H-021（独占机） |
| M0-020 渲染引导 | WebGPU 真机帧时间 / 像素 / compute 真 GPU 未测 | H-021 |
| M0-022 预测对账 | 真实 `tc netem` 下的对账手感未测（用进程内等价模型与 headless 替代） | H-022 / M1-068 |
| M0-023 CDLOD 地形 | GPU 帧时间（RTX 3060 / Iris Xe）未测；WebGPU 无像素校验 | H-021 |
| M0-024 植被 | 帧率判据需 GPU（软件渲染下不可比） | H-021 |
| M0-025 灰盒角色与射击反馈 | CPU 帧增量 ≤ 0.5 ms 与 GPU 帧时间在软件渲染下无意义，需参考机 | H-021 |
| M0-027 headless | 本任务已复测 30 min：RSS 1.95 %（通过），遗留的"初始同步量化偏移使约 30–50 % 快照带 `pos_corr`（+≈1 kbps）"未处理（PI-028 第 3 点） | M1-040 / M1-068 |
| M0-028 互通矩阵 | Firefox / WebKit / Safari 本机无构建，如实"未验证"；受信任证书 | H-103、H-017 |
| M0-030 性能基线 | RTX 3060 / Iris Xe 帧时间未测；cam2 P50 极差 7.9 %（共享虚机）；`snapshot_decode` 基准不存在（留给 M1-069） | H-021 |
| M0-032 对标测量 | 参考作品一侧的数字全部"待测"（缺录像） | H-011 / H-025 / H-026（H-104）→ M0-032b |
| M0-033 本任务 | 因上述 E0-3/5/6 未完全通过，按判定规则记 partial，等 Fable 审阅 | H-038 |

### 8.2 PLAN_ISSUES 状态

PI-001…PI-019、PI-021、PI-029、PI-031 为 resolved；PI-020、PI-022…PI-028 已由总控按守则"采纳"（文件里状态栏仍为 open，建议总控统一回写为 accepted）；**PI-030（KCC 判据）open，待 Fable 裁决**。

### 8.3 需要人类负责人做的事（NEEDS_HUMAN）

| 编号 | 事项 | 卡住什么 |
|---|---|---|
| **H-038** | 触发 Fable 的 M0 结束审阅（本报告 + PI-030） | M0 关闭 |
| **H-021** | 测试设备矩阵：一台 RTX 3060 级独显机 + 一台 Iris Xe 级核显机（或可远程跑 Playwright 采集脚本的方式） | E0-5、E0-6 真 GPU、E0-3、M0-011/015/020/023/024/025/030 |
| **H-102** | GitHub `main` 分支 + 能开 PR，使 CI 实跑 | E0-10；M0-002/004 |
| **H-011 / H-025 / H-026（汇总 H-104）** | 合法获取参考作品并判断 EULA、录制设备、批 M0 三个录像会话 | E0-12、M0-032b |
| **H-017** | 域名与受信任 TLS 证书 | 跨浏览器 WT 互通、生产 WT |
| **H-103** | 在真实 Firefox / Safari 上按 DEVELOPING.md 手工验证互通 | E0-7 的 Firefox / Safari 格 |
| **H-022** | 受限网络 / `tc netem` 测试台 | E0-8 的真实网络注入 |
| **H-039** | 基于 M0 数据决定最低配置 / 最低支持设备（M1 开工前） | M1 画质档位 |

### 8.4 缺失素材需求 ID（全部灰盒占位）

V-CHR-US-001、V-CHR-GE-001、V-WPN-US-001、V-WPN-GE-001、V-FX-001、V-FX-006、V-NAT-002、V-NAT-004、V-NAT-007（CC0 可选）；音频全部缺失（M0 无音频）。`window.__placeholders` 在演示里列出了当前用到的占位 ID。

## 9. 对 M1 的建议（按优先级，≤ 10 条）

1. **先让 CI 在 GitHub 上跑起来（H-102）**：E0-10 与 M0-002/004 的最后缺口；之后把 `pnpm -C client test:e2e`、`playwright.{net,game,interop}.config.ts`、determinism（默认严格）各列为独立作业。
2. **取得参考机（H-021）并在 M1 渲染任务前完成三件事**：E0-5 帧率、KCC 复测（PI-030）、`?smoke=compute` 真 GPU 冒烟；否则 M1-043/044/049 在没有预算校准的情况下开工。
3. **Hi-Z / 深度纹理绑定 / indirect draw / TAAU 接入各做 ≤ 1 天 spike**（PI-005 的三个缺口），再开 M1-043/044。
4. **KCC 余量**：M1 内安排一个低风险优化（静止玩家跳过 ground probe、复用接触缓存）并保持黄金哈希；同时请 Fable 裁决 PI-030 的判据（服务器预算以 100 人 tick 为准）。
5. **录像（H-011/025/026）在 M1-011/013 手感调参前到位**，M0-032b 回填；最迟 M1 退出前。
6. **真实弱网验收**：在有 `tc`/iproute2 的机器或受限网络台（H-022）上，用 `scripts/loadtest-m0.sh` + netem 跑 E0-8（100 ms + 2 % 丢包）与 WT 阻断场景，纳入 M1-068。
7. **受信任证书与域名（H-017）+ Firefox / Safari 手工互通（H-103）**，决定 WT 是否对非 Chromium 浏览器默认启用。
8. **长时间 soak 门禁**：把 RSS 判据改为"最后 10 min 最小二乘斜率 ≤ 1 KB/s 且逐 10 min 递减"，并跑 ≥ 1 h（M0 的 1.95 % 是险过）；顺带查清 0.5 KB/s 残留（分配器 / 线程栈层面）。
9. **补 `snapshot_decode` 基准（M1-069）并处理初始同步的量化偏移**（约 30–50 % 快照带 `pos_corr`，PI-028 第 3 点），为 M1-040（自动重连）的对账打底；输入窗口"窗口外丢弃但不计违规"的语义（PI-029）在改成滑动窗口时保持。
10. **任务粒度**：M1 的 L 级任务拆成"实现 / 长跑与性能验收"两段；每个任务卡要求记录会话数，便于检查点 1 校准；INDEX 补上 M0-032b、`data/rules`、`data/placeholders`、perf-regress 的归属（见 §7 第 5 项）。

## 10. 复验命令清单（下次验收照抄）

```sh
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 FRONTLINE_REQUIRE_TERRAIN=1
PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0
wasm-pack build crates/sim-wasm --target web --profile wasm-release && pnpm -C client sync-map
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --features frontline-sim/test-util
cargo build --release -p frontline-server -p frontline-headless-client
cargo test --release -p frontline-server --test step_overhead && cargo bench -p frontline-sim --bench kcc
node tools/ci/check-deps.mjs && node tools/ci/check-sim-purity.mjs && node tools/ci/check-layout.mjs && node --test tools/ci/test/*.test.mjs
pnpm data:validate && pnpm data:test
pnpm -C client lint && pnpm -C client typecheck && pnpm -C client test && pnpm -C client build
pnpm -C client test:e2e
FRONTLINE_REQUIRE_SERVER=1 pnpm -C client exec playwright test -c playwright.net.config.ts
FRONTLINE_REQUIRE_SERVER=1 pnpm -C client exec playwright test -c playwright.game.config.ts
scripts/run-interop.sh
bash tools/determinism/run.sh --strict && node tools/ci/check-wasm-size.mjs
node scripts/perf-regress.mjs check --quick
(cd tools/map-pipeline && python3 -m pytest) && (cd tools/ref-analysis && python3 -m pytest)
scripts/loadtest-m0.sh --transport wt --duration 1800 --no-build --out reports/m0/acceptance/loadtest-wt-30min.json
```

Playwright 的 net / game 配置依赖 `target/debug` 的 `frontline-server` 与 `gen_dev_cert`（`cargo test --workspace` 会生成）。
