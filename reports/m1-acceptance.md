# M1 里程碑验收报告（M1-072，部分）

状态：**M1 未达到可关闭条件。** 以下为 E1-1…E1-17 的核对结果（已对照 PR #117–#139 后的最新 `tasks/STATUS.md`、`docs/PLAN_ISSUES.md` 与 `reports/` 实测记录核对；结论列严格按 `tools/ci/check-acceptance.mjs` 使用 `PASS` / `partial` / `FAIL` / `未测`，当前 PASS 4、partial 10、未测 3、FAIL 0）。工作站结果按 PI-109 标注，不替代 RTX 3060 / Iris Xe 参考机（H-021）。

| E# | 标准 | 目标 | 实测 | 方法/命令/报告 | 结论 | 遗留(PI-xxx) |
| --- | --- | --- | --- | --- | --- | --- |
| E1-1 | 真人测试局 | ≥20 min、≥20 真人每队 10 | 无真人测试局（真人数 0）；仅有工作站 headless 20 客户端 + 12 bot 最长 1800 s 联测（#116/#119/#139）、B4-D1 Carentan WT 1800 s（20 headless + 12 bot）及 300 s 进程内 bot soak（#129），均不含真人 **B6-soakpin（#161）** 两轮均为共享工作站（PI-109）Carentan WT 无 netem，20 headless + 12 bot 各 1,800 s，仍为 0 真人。 | `reports/m1-playtest-log.md`；人类 H-054/H-056、`reports/m1-server-tail-b6-soakpin.md` | 未测 | H-054/H-056、M1-067 阻塞、PI-088/090 |
| E1-2 | 服务器 tick | 20 客户端 × 30 min（含 netem），P99 ≤ 28 ms、P50 ≤ 18 ms，无连续两 tick 超 33 ms | 共享 EPYC 7402 工作站（PI-109，非参考机）：既有 WT netem（25±10 ms 单向，2 % 丢包）30 min P99 5.30 ms，max 34.5 ms，超时 1 次（on-CPU 32.3 ms）；#119 三组 1800 s 并行（wt+netem / wt 无损 / ws 无损）P99 5.58 / 5.82 / 5.23 ms，max 26.3 / 25.4 / 42.9 ms，超时 0 / 0 / 2，其中 ws tick 28499 为真实 `phase_step` 计算尖峰（cpu 37.5 ms，runq 0.5 ms）。B2-G（#139）WT 无 netem 1800 s（20 headless + 12 bot，bot-fill 开启，`taskset` 绑核）：P50/P99/max 2.099/5.894/75.676 ms，超时 2 次，最长连续 1 次，>20 ms 的 3 次均由 runq wait 主导；历史 WS tick 28499 计算尖峰在两轮 975 s 场景复跑中未重现。B4-D1 同机 Carentan WT 1800 s（20 headless + 12 bot、绑核、无 netem）P50/P99/max 3.519/9.810/323.875 ms、94 次超时最长连续 2（未达标）；3 对连续超时均由 runq 主导，另有 13 次慢 tick 与日志写入的文件系统等待栈匹配。B2-A（#135）驾驶复测 20 WS + 12 bot，无损/netem 各 300 s：P50 2.786/2.499 ms，P99 9.109/6.305 ms，max 163.964/261.554 ms，超时 8/10 次，最长连续均为 1，两次最大值均落在启用 trace 的快照阶段且阻塞来源未归因。B5-E1 同参数工作站 1800 s 结果为 P50/P99/max 4.189/8.644/126.668 ms、29 次超时（均 runq 主导）、最长连续 2，连续超时门槛仍未达。 **B6-soakpin**：同参数无 netem 两轮均通过；P50/P99/max 4.568/8.767/24.434 与 4.162/8.059/81.440 ms，超时 0/9 次、最长连续 0/1。工作站非参考机（PI-109）。 **B6-carentan-netem（PI-109，非参考机）**：20 WT headless + 12 bot，双方各一坦一轻车的 HQ 夹具，部署后全员重叠 1808.767 s，单向 25±10 ms、实测丢包 2.009%。tick P50/P99/max 3.809/8.027/84.295 ms，7 次超时、最长连续 1，11 个跳过定时槽，退出 0。 | `reports/m1-loadtest-netem.md`、`reports/m1-carentan-wt-b4-d1.md`、`reports/loadtest-1119/{wt-30min,phase-}*.json`、`reports/m1-server-tail-b5-e1.md`、`reports/m1-server-tail-b6-soakpin.md`、`reports/m1-carentan-netem-b6.md` | partial | B4-D1/B5-E1 Carentan WT P50/P99 达标但连续超时门槛未达；该两批未测 Carentan WT netem、独占核及参考机。旧 tick 28499 与 B2-A 快照尖峰未定（PI-109、PI-115、PI-121 续、PI-127D1）、PI-127E1；B6 在无损工况通过连续超时门槛（PI-B6-soakpin-1）；本批已补 netem 工况（PI-B6-carentan-netem-1）。参考机、独占核及正式载具驾驶/交战未测。 |
| E1-3 | 带宽 | P50 ≤ 128、P99 ≤ 400 kbps，上行 ≤ 32 kbps | 工作站 M0 sandbox 各运行（含 2 %/5 % 丢包+抖动、150 ms RTT、WT 30 min、#131 驾驶 netem 与 #139 B2-G WT 1800 s bot-fill）下行 P50 14.2–21.4 / P99 22.3–30.3 kbps，上行 P50 20.4–20.6 kbps，均在预算内；B3-C2 载具 WS netem 90 s（20 客户端 + 12 bot）下行 P50/P99 15.40/28.76 kbps、上行 P50 19.92 kbps。B4-D1 工作站 Carentan WT 1800 s（含 C3、无满编载具）下行 P50/P99 24.904/34.144 kbps、上行 P50/P99 20.448/21.280 kbps；该轮未测满编载具、Carentan netem 和参考机（PI-109）。B5-E1 工作站 Carentan WT 下行 P50/P99 21.992/26.888 kbps，上行 20.576/21.424 kbps。 **B6-soakpin（#161，PI-109）** 同工况两轮下行 P50/P99 22.264/26.856、21.616/26.872 kbps，上行 P50/P99 20.576/21.424、20.560/21.400 kbps，上行最大 22.136/21.624 kbps。 **B6-carentan-netem（PI-109）**：四辆在场夹具，实测丢包率 2.009%。下行 P50/P99 20.352/25.336 kbps，上行 20.456/21.376 kbps，最差客户端下行/上行 P99 27.544/21.456 kbps。 | `reports/m1-loadtest-netem.md`、`reports/loadtest-1119/*.json`、`reports/m1-server-tail-b5-e1.md`、`reports/m1-server-tail-b6-soakpin.md`、`reports/m1-carentan-netem-b6.md` | partial | PI-071（M1-024b）、PI-109、PI-127D1、PI-127E1、PI-B6-carentan-netem-1。参考机、独占核及正式载具驾驶/交战未测。 |
| E1-4 | AOI | 视锥外且 >96 m 敌人下发 = 0（硬边界按 PI-071 的离开阈值） | 服务器 10v10 R3 单测通过。**B2-A（#135）驾驶复测**：工作站 WS 20+12，无损/netem 各 300 s，客户端同 tick 独立审计 368,962/350,168 条敌方记录（其中 >105.6 m 61,945/70,524 条），违规 0/0，服务器计数 0/0；旧预测审计 0/6 条，复现的 6 条均为预测位置偏差；上批 #131 的 13/302,361 条缺逐条日志无法复核。**B2-G（#139）**：工作站 WT 无 netem 1800 s 客户端审计 1,612,038 条敌方记录，异常 0，服务器计数 0。B4-D1 Carentan WT 1800 s 同 tick 配对 717,121/717,121 包，审计 2,682,547 条敌方记录（5,913 条 >105.6 m），0 越界、服务器计数 0；WS/WT 真监听器验证手工 C3 标记不跨队，单测覆盖实墙遮挡。该轮未测 Carentan netem 和参考机；B3-C2 工作站 WS netem 90 s 审核 81,603 条敌方记录，违规 0（PI-071、PI-109）。B5-E1 工作站 Carentan WT 配对 719,320/719,320 个快照，检查 133,596 条敌方记录，0 越界且服务端计数 0。 **B6-soakpin（#161，PI-109）** 同 tick 快照配对为 719,987/719,987 和 719,853/719,853，独立审计敌方记录 182,566 / 259,476 条（>105.6 m 为 3,697 / 3,307 条），客户端违规与服务端计数两轮均为 0。 **B6-carentan-netem（PI-109）**：同 tick 配对 709,320/709,320，独立审计 397,531 条敌方玩家记录（3,937 条 >105.6 m），客户端与服务器均 0 越界。 | `reports/m1-loadtest-netem.md`、`reports/loadtest-1119/ws-drive-aoi-*-300s.json`、`cargo test -p frontline-server --test aoi_audit`、`reports/m1-aoi.json`、`reports/m1-server-tail-b5-e1.md`、`reports/loadtest-1119/carentan-wt-30min-b6-soakpin-run{1,2}.json`、`reports/m1-carentan-netem-b6.md` | partial | PI-071、PI-109、PI-121 及续、PI-123、PI-127D1、PI-127E1、PI-B6-carentan-netem-1。参考机、独占核及正式载具驾驶/交战未测。 独立载具 AOI 未测。 |
| E1-5 | 延迟补偿 | 150 ms RTT 命中率差 ≤ 5% | B3-C2 工作站 sim 级瞄准射击场景（PI-109）更新为亚 tick 插值。0 ms 与 150 ms RTT 各 489/600（81.5%），差 0.0 pp。150 ms RTT、视图延迟估计抖动 ±1 tick 为 481/600（80.2%）。添加载具历史和持续地形遮挡回归。B4-D1 加实墙遮挡与同序号已发射起点诊断；真实 WT 平地夹具开启回溯无损 4/4、实测 RTT 150.23–150.99 ms 4/4，同网况关闭回溯 0/4 次 `HitConfirm`。 | `cargo test -p frontline-sim --test lagcomp_hitrate --release -- --nocapture`；`reports/m1-carentan-wt-b4-d1.md` | partial | 小样本链路对照，非完整 Carentan 命中率验收（PI-072、PI-115 续、PI-127D1） |
| E1-6 | High 性能 | P95 ≤ 16.7 ms、≤1200 draw call、≤4.0 M 三角形 | **RTX 3060 参考机（H-021）未测；此前共享 GPU 的工作站（RTX 3080，PI-109）P95/卡顿无法判定**。工作站占位几何 M1 三机位：draw call 84/205/145、三角形 ≤0.45 M、提高优先级后 CPU P50 8.2–9.3 ms；但帧 P95 16.8–83.3 ms、卡顿帧 6–169/60 s（限 2）已查明来自共享 GPU 被其它任务占满（93–100 %），与植被上传无关（PI-114）。#117 自写 CSM 增动态投射体（士兵/坦克/树 LOD），#120/#122 `taa_vel` 工作站 A/B 受 40–98 % 后台 GPU 负载干扰无结论、默认保持关闭。此前空闲 GPU、参考机、分 pass GPU 预算与 wasm 内存均未测。**B6 独立 GPU1 工作站（PI-109）**：WebGPU/1080p 三机位×3 轮，P95 16.70–16.80 ms、>50 ms 卡顿 0–2 帧/轮、draw call 83–211、三角形 347532–444075；RTX 3060、分 pass GPU 与 wasm 内存仍未测。 | `reports/m1-perf.md`、`reports/m1-perf.json`、`reports/m1-perf-raw/`、`reports/m1-csm.md`、`reports/m1-post.md` | 未测 | PI-109、PI-110、PI-114、PI-117 续二、PI-118 续三、H-021、M1-064（占位几何）、PI-B6-perf-gpu12-1 |
| E1-7 | Low 性能 | ≥30 fps，draw call ≤900/500 | **Iris Xe 参考机未测（H-021）。** **B6 GPU1 绑定工作站（PI-109，部分窗口共享）**：WebGL2/720p 输出、960×540 内部，三机位×3 轮，59.88–60.02 rAF FPS、P95 16.70–16.80 ms、>50 ms 卡顿 0–1 帧/轮、draw call 71–181、三角形 125572–143173。 | `reports/m1-perf.md` | 未测 | PI-109、M1-069b、H-021、PI-B6-perf-gpu12-1 |
| E1-8 | 材质变体 | ≤ 60 | M1-042a/b `variant-count` / `prewarmVariants` 统计 PBR 合法变体 **32 个**（opaque 16 / alpha 8 / depth 4 / shadow 4，Low 20 个；工作站预热 WebGPU 15.8–16.3 ms / WebGL2 69.7–105 ms，0 失败，PI-109 非参考机）。B3-C3（#142）与 B6-variantci 已将自写 CSM（`csm:plain`、`csm:instanced`）与 TAA 速度着色器（`velocity:plain`、`velocity:instanced`、`velocity:terrain`）共 5 项纳入 `VariantRegistry` 统计，并将 `pnpm -C client run variant-count` 接入 `scripts/local-ci.sh ts` 门禁：每后端 **High 37/60、Low 25/40**（含源码 `registerCustom` 一致性断言，8/8 通过）。M1 世界场景运行时仍使用 18 种占位材质（代理量，尚未切换为 PBR） | `scripts/local-ci.sh ts`、`pnpm -C client run variant-count`、`reports/m1-pbr.md`、`reports/m1-perf.md` | partial | PI-094、PI-116、PI-128、PI-B6-variantci-1、M1-042b/M1-069b（全场景 PBR 替换与参考机预热未测，M1-064/H-021） |
| E1-9 | 包体积 | core ≤ 8 MB；首图 Medium ≤ 150 MB | #132 生产 core（含 sim-wasm，184 文件）Brotli 1,150,132 B（原始 4,263,442 B，≤ 8,388,608 B）；B3-C6 已将 `core-size.mjs`（`brotliBytes`）接入 `scripts/local-ci.sh assets` 的 `manifest-budget` 门禁（未含 sim-wasm 实测 834,029 B，9.9 %）并补齐按类别/`_fp`/`_tp` 汇总；M1-035b（B3-C4）仅地形草稿 `.fpak`（30 包 / 380 块）总计 5,714,050 B（`checkBudgets` 5,706,520 B，占 150 MiB 预算 3.63 %），当时完整首图 Medium 包未测。B6-fpakmedium 实测完整 Medium 107,438,594 B / 150,000,000 B，含 sim-wasm 的生产 core Brotli 1,189,937 B / 8,000,000 B（PI-109）。 **B6-acceptdemo（PI-109）** 生产 core 含 sim-WASM，195 文件，Brotli 1,193,738 B（≤8,000,000 B）。 | `reports/m1-boot.json`、`reports/m1-chunk-bake.json`、`assets/manifests/terrain_carentan.json`、`tools/ci/core-size.mjs`、`scripts/local-ci.sh assets`、`reports/m1-fpak-medium.md`、`reports/m1-demo-run.md` | PASS | PI-B6-fpakmedium-1（M1-064 的运行时接线待验收）、PI-125、PI-127C6、PI-127C4 |
| E1-10 | 部署流程 | 键鼠可达；点击到可操作 ≤ 15 s；预取完成率 ≥ 95% | M1-040c（#127）模拟 WS/WT 各 1 例 C0 接收；#132 在工作站 Chromium 验证真实网关 → 本机 M0 WS 入场及无样式键鼠界面，并用真实 WASM＋模拟 C0 验证部署拒绝与权威 `spawn_tick` 显示；M1-026b（#136）补工作站私有 netns 5 % 丢包下真 WS/WT 监听器（实测丢包 5.006%/5.019%，每端 6,000/6,000 探针 0 错）的 C0 HQ 回执及 C4 状态对照。**B6-deploytiming（PI-109，工作站非参考机）**：本机真 `gateway`+`server`（Carentan M1 `assault`）+ 生产 `client/dist` + 30 个 Carentan 地形 `.fpak`（5.71 MB），无头 Chromium 软件渲染 + CDP 节流各 5 次：20 Mbps/50 ms 冷/热点击到可操作 P50 **6.25 / 1.42 s**（max 6.48 / 1.43 s），5 Mbps/100 ms 冷/热 P50 **12.85 / 1.83 s**（max 13.00 / 1.86 s），均 ≤15 s；内陆点 `(650,1470)` `R_near=4` 预取 20/20 轮 **81/81（100%）**，热 OPFS `.fpak` 请求 0/0 B（预取降耗 83.5%–90.9%）。不含 M1-064 实素材整包、M1-067 预发布公网与参考机，主页面部署 UI 未直连流式预取。 **B6-acceptdemo（PI-109）** 真实服务器列表、游客签票和两个 Chromium WS 会话的 HQ C0 部署均通过。战术碰撞夹具绕过部署界面。 | `client/e2e/ui-{lobby,match}.spec.ts`；`client/src/net/connect.test.ts`；`reports/m1-c4-netem.md`；`scripts/measure-deploy.mjs`；`reports/m1-deploy-timing.{md,json}`、`reports/m1-demo-run.md` | partial | PI-073 续、PI-092 续、PI-109、PI-125、PI-B6-deploytiming-1、M1-049b、M1-064/M1-067 阻塞 |
| E1-11 | Garrison + 补给闭环 | 闭环可玩 | sim `e2e_garrison_supply_loop` 通过（取箱→放 Garrison 扣 50→队友 8 s ±1 tick 重生）；M1-060b（#128）13 个真实 WASM 客户端逻辑用例通过，覆盖 8 种 Garrison 拒绝原因及补给箱/弹药箱预判；M1-039c（#129）短场景验证 Support 放箱触发 Officer 建驻点；#132 补无样式放置/座位 DOM 与接线夹具；M1-026b（#136）验证 5 % 丢包下 Garrison 状态 C4 仅向本队广播。B2-E 工作站实测：6 客户端回环 WS 放置 C0 夹具 1 项通过，TS 输入/C0 夹具 5 项通过，真实 WASM 验证三种工具禁建。**B6-browsergarrison（PI-109，非参考机）**：两页真服 + 网关各跑 WS/WT，取箱→权威库存 100→50→驻点建成→队友在 8 m 内重生→备弹 120→90→120；确认至放置回执 WS 33.6 ms、WT 33.3–33.6 ms（每种两次）。 **B6-acceptdemo（PI-109）** 双浏览器 WS 夹具测得补给 100 → 50，重生距驻点 5.61 m，弹药箱补充备弹 120 → 90 → 120。 | M1-019/020、`client/src/game/placement-check.test.ts`、`reports/m1-c4-netem.md`、`crates/server/tests/placement_ws.rs`、`client/src/game/live-placement.test.ts`、`reports/m1-garrison-e2e.md`、`reports/m1-demo-run.md` | partial | PI-061/063、PI-090 续、PI-107 续、PI-109、PI-125、PI-B6-browsergarrison-1；正式地图/主页面 UI、真人演示、bot 完工重生、受信任证书与参考机未测 |
| E1-12 | 坦克 | 3 座位；≥12 组 AP 与手算一致；120 ms RTT 直线校正 ≤1 次/10 s | 13 组 `armor_cases.json` 全一致。同机工作站（PI-109）WS 夹具折返联测 300 s（25±10 ms 单向延迟 + 2 % 丢包）：#131 入/出座各 1 次，59/2,950 次纠正（P99 0.0979 m）、0 硬纠正；B2-A（#135）复测无损 0/2,972、netem 51/2,966 次纠正、0 硬纠正。有头 Chromium 工作站夹具（#134，RTX 3080 WebGL2）短程前进/倒车两轮，各入/出座 1 次，rAF P95 均为 16.7 ms，回环 0/244、netem（单向 60 ms / 客户端 RTT P50 143.12 ms、2 % 丢包）1/244 次平滑纠正（0.1215 m），硬纠正均 0。B3-C7 以 8.04672 m/s 限速与新质量、外廓复测同一短程夹具：回环/netem rAF P95 16.8/16.8 ms，平滑纠正 0/365、1/367（netem 最大同 tick 误差 0.0532 m），0 硬纠正/缺 tick 重锚；netem（单向 60 ms、2% 丢包）客户端 RTT P50/P95 112.90/138.40 ms，约 20 s 含倒车。WT 驾驶、炮手/车长座位与换座、联网载具渲染、正式 Crewman 入场及 120 ms RTT 连续 60 s 直线工况仍未测。 **B6-acceptdemo（PI-109）** 独立 M0 WS 夹具确认驾驶座入/出各一次及 11.17 m 权威位移。 | M1-023、M1-022、`reports/m1-loadtest-netem.md`、`reports/m1-vehicle-browser.md`、`reports/m1-demo-run.md` | partial | PI-070、PI-109、PI-123（续–续六）、PI-124G、M1-064/M1-067 阻塞 |
| E1-13 | 倒地/救起/压制 | 状态机全转移单测；压制被客户端音画消费 | M1-016/017 单测通过（状态机转移 ≥10 条）；B3-C1 补 3 项权威伤害状态回归、灰盒遮挡爆炸压制 ×0.5，完整 sim 夹具 native/WASI 哈希 480/480 tick 一致。M1-063b（#121/#126）网络对局已订阅压制音频，工作站无头 Chromium 离线合成 PCM 在第二次压制后 400 ms 恢复到基准（低频振幅与高低频比均约 1.00002×）；#132 的 HUD 压制逻辑与倒地/阵亡/复生 DOM 通过模拟事件夹具。真实素材、实服倒地/压制 C4 整链与压制音画联验均未测（非 RTX 3060 参考机，PI-109）。**B6 复测补齐以上实服链路**：真实战术服 + Chromium SwiftShader，普通 WS 与 netem 5% 各 1/1。权威 C2 压制到达 HUD 和 WebAudio，C4 倒地、救起、阵亡及复生与 HUD 一致 | M1-014b/015/016/017；`crates/sim/tests/{m1_014b_damage_outcomes,throwables}.rs`；`client/e2e/audio-offline.spec.ts`；`reports/m1-suppress-e2e.md` | PASS | PI-B6-suppress-e2e-1、PI-109。参考机、真人与正式素材联验未测（M1-064/065） |
| E1-14 | 素材管线 | `.fpak` 往返一致；无独立 .glb/.ktx2/.ogg/.opus；缺失报告；无 PENDING 进主分支 | B6 前 `ledger-check` 实测 **241 行：OK 231、PENDING 3、FLAGGED 1、REJECTED 0、INTERNAL 6**；`ingest --metadata-only` 校验 227 条免费素材元数据全部通过；`assets/manifests/missing-report.json`（来源级元数据）全里程碑 168 项，M1 72 项（含非视觉）为 5 real / 7 placeholder / 56 missing / 4 internal-only；`scripts/local-ci.sh assets` 49.0 s 通过（173 单测 + 10 e2e）；B3-C6 补齐 JPEG 贴图解码、客户端音频解包与 OPFS 同步句柄 Worker，工作站无头 Chromium（PI-109）跑通全链路 `.fpak`→Worker 解码/BasisU 转码/Opus 解码与 117.65 MB 冷热基准（0 长任务、0 热启动请求、100 Mbps 下 -87.4 %、200 条目上传 P95 0.1 ms）。B6-fpakmedium 将五项已登记 `OK` 模型、灰盒及合成音效入包。35 包 / 870 条目逐字节往返一致，独立素材扫描通过。构建级缺失报告 M1 共 72 项，5 real / 14 placeholder / 49 missing / 4 internal-only。 **B6-assetlod（#154）** 复测台账 242 行（232 OK / 3 PENDING / 1 FLAGGED / 0 REJECTED / 6 INTERNAL）。工作站（PI-109）合成 B 的 LOD0 为 1,288,944 / 5,294,362 B（24.35%），构建 19.54 s。CC0 WaterBottle 夹具 LOD0 为 1,734,656 / 8,966,928 B（19.35%）。 | `scripts/local-ci.sh assets`、`client/e2e/asset-loader.spec.ts`、`reports/m1-asset-loader.json`、`assets/manifests/missing-report.json`、`LICENSES.md`、`reports/m1-fpak-medium.md`、`reports/m1-002-lod.md` | PASS | PI-032–038/044、PI-108、PI-109、PI-127C6、PI-B6-fpakmedium-1（M1-064 运行时接线待验收） |
| E1-15 | 基础反作弊 | 1.3× 速度、超射速被纠正/拒绝并计数 | M1-027a 夹具 1.3× 移动向量注入后最高 6.34 m/s，超射速 8 发出膛、56 次拒绝。B3-C2 工作站混合 WS 20 客户端 300 s，移动超限计数 1,819、拒绝扳机 183、2 个预期断开（PI-109）。**B6-antiche**：默认 3,600°/s；M0 WS 20/20 × 1,800 s，含 4,500 个合成甩枪峰值，移动/视角纠正、射速拒绝、踢出各 0。混合复测钳制移动 1,890、视角 1,800，拒绝扳机 175，仅 2 个预期断开。起点提示通过 WASM 打包与真 WT 诊断回归，不改变权威弹道。 **B6-antiche（#157，PI-109）** 独立 1.3× 移动注入产生 180 次钳制，最高 6.343 m/s。独立 2× 射速注入接受 20 次、拒绝 20 次。 | M1-027a；PI-074；`reports/m1-anticheat-b6.md` | PASS | 仅合成输入与本地链路（PI-109、PI-B6-antiche-1/2）；真人、参考机和 Carentan 实服浏览器未测，不关闭 M1。 |
| E1-16 | 数据与史实 | historical 100% 有出处或"未核实"；批 M1 回填与复测在容限 | M1-070b（#130）原始比对 14 文件 67 个字段：51 一致、10 不一致、6 未核实；经 #137 与 B3-C7，10 项差异均处置：8 项有依据回填、2 项实用射速明确未核实；结构审计 18 已登记、49 缺口、0 违规（Sherman 的字符串出处不计结构化登记）。**B6-histbackfill（PI-B6-histbackfill-1）**：复核剩余 8 项未核实并回填 2 项一手手册出处（M3 冲锋枪 8.0 in 枪管初速 280 m/s；Panzerfaust 60 整件质量 6.25→6.1235 kg），M1911A1 与 M1 Carbine 实用射速改为显式 `"未核实"`；14 文件 67 个 `historical` 字段 100% 完成结构化 `_source` 登记或显式未核实声明（`source-audit.mjs`：**61 已登记 / 6 声明未核实 / 0 违规**，10 文件 `reviewed`、4 文件保持 `initial`）；Sherman 装甲板厚/倾角与 75 mm M3（M61 APC 标称穿深、M48 HE、M64 WP）/机枪弹种表已完成史实对照并登记 `_meta.deviation_reason`（`DATA_HASH = 0x2637b94dda28a192`）。批 M1 对标录像回填与我方复测仍未做（M1-071 blocked） | `docs/measurements/hist-review-m1.md`、`node tools/data-validate/source-audit.mjs --root data`；M1-071 blocked | partial | PI-124G、PI-B6-histbackfill-1、H-011/025/026 |
| E1-17 | 占位决定 | 范围内占位需求 ID 有"决定"或明确推迟 | `missing-report.json` 中 M1 72 个需求 ID（视觉 39 + 音频 33）来源级为 5 real / 7 placeholder / 56 missing / 4 internal-only，B6-fpakmedium 构建级为 5 real / 14 placeholder / 49 missing / 4 internal-only，台账 `ledger-check` 242 行（OK 232 / PENDING 3 / FLAGGED 1 / REJECTED 0 / INTERNAL 6）；B6-gapsdecide 已在 `docs/ASSET_GAPS.md` §2/§3.8 与 `reports/m1-placeholders.md` 按零采购默认假设逐 ID 完成 72/72 项裁决（5 `real` 免费 CC0 采用 + 36 免费候选采用配程序化占位回退 + 15 纯程序化占位且真素材推迟到 M2 + 16 非 M1 兵种/载具/混响/配乐范围外推迟到 M2，并逐项写明程序化占位规格；另核对 M0 延续 13 项）；运行时仍为占位且人工项 H-047/H-058/H-099/H-100 采用默认假设未获人类最终签字 | `reports/m1-placeholders.md`、`assets/manifests/missing-report.json`、`docs/ASSET_GAPS.md` §2/§3.8、`docs/NEEDS_HUMAN.md` §11.7 | partial | PI-108、PI-B6-gapsdecide-1、M1-064/065、H-047/058/099/100 |

## Bot 行为补测（M1-038c / M1-039c）

`bt_scenarios` 验证了墙后 1,000 tick 零开火、听声搜索不授权开火、视觉记忆到期、Move/Attack/Defend/Hold、倒地放弃输入、5 s 卡住跳跃及补给/救治输入。各 200 次脚本采样中，Recruit / Veteran 识别后首次发出 `FIRE` 的平均延迟为 746.00 / 403.50 ms；未测真人实战延迟。共享 EPYC 7402 工作站（PI-109，非 RTX 3060 参考机）上的 10v10 M1 brain / M0 地形发布构建，8,290 次完整决策 P50 1.48 µs / P95 5.39 µs（`reports/m1-bots-bt.json`）。这些结果不替代 E1-1 真人测试或 E1-2 服务器长跑。

发布构建 soak（`cargo test --release --locked -p frontline-server --test bots_soak tactical_ -- --ignored --nocapture --test-threads=1`），同一共享工作站：

| 场景 | 模拟时长 / 墙钟 | 发射 / 击杀 / 重生 | 部署成功 / 拒绝 / 踢出 | BotHost P95（全部 bot 思考和输入摄入） | tick P99 |
| --- | --- | --- | --- | --- | --- |
| Carentan 1/3、真实据点 5、20 bot | 9,000 tick / 300 s；10.703 s | 47 / 11 / 9 | 31 / 0 / 0 | 0.064 ms | 4.741 ms |
| M0、18 bot + 2 脚本玩家 | 9,000 tick / 300 s；5.435 s | 227 / 55 / 54 | 72 / 0 / 0 | 0.134 ms | 1.217 ms |

混合组第 30 tick 填到每队 9 bot + 1 脚本玩家，共发 18,000 个 C1 输入包、处理 1 个 C0。测试为进程内模拟，无真实传输或真人；Carentan 仅激活一个据点、使用烘焙 navmesh 与解压后的 380 个 `FCOL` 地形块，不等于多据点完整对局。碰撞夹具用 `map_pipeline chunks-m1` 生成，zlib 解压 `c_*.fcol` 到 `data/maps/carentan/m1/chunks/c_*.bin`；生成物不进仓库。E1-1 不改判定。

## C3 渲染接线验证（PI-128）

每个渲染后端的门禁计数为 High 37/60、Low 25/40。Carentan 的 WebGL2 软件渲染功能检查在 `hedgerow` 机位读到 424 个全图非缺口段及 12 个驻留树篱网格，运行时注册 5 项自写 CSM/TAA 着色器变体。独立流式夹具分别以 8 和 10 m/s 模拟移动 1,000 m，两组均发现 0 个未覆盖像素，最大上传批次均为 720,000 B。驻留网格是对象计数，E1-6 和 E1-7 的帧时间、draw call、GPU 内存与参考机验收本轮未采（PI-109）。M1-064 的实素材、生产地形区块上传链、真实车辆 feed 和 M1-067 的预发布流程仍未验收。

## D2 占位角色与室内逻辑（PI-128 B4-D2）

Carentan 灰盒烘焙覆盖 70/1,408 栋（5.0%）、其中 54 栋有坡道，输出 146 个楼层探针点。SwiftShader + 真实 WASM 验证 1 栋跨门平面进/出、墙体阻挡与渲染遮挡；原生 KCC 另实测 3 栋进门、出门、实墙阻挡及登二层。软件 WebGL2 / 离屏 WebGPU 各验证 60 个占位角色；这些是功能证据，不是 WebGL2 Medium 老城 draw-call 或帧时间数据。占位 Humanoid 骨架与状态机不等同于真材质、剪辑或蒙皮。D2 未采 GPU 渲染性能（PI-109，工作站检查见共享 GPU 利用率 ≥30%，RTX 3060/Iris Xe 未测），不更改 E1-6/E1-7/E1-17 结论。真实素材与预发布入局仍受 M1-064/M1-067 阻塞。

## 演示脚本（MILESTONES §2.4）

以下为 B6-acceptdemo 之前的记录，本批复跑见文末。

8 步均**未执行**：预发布域名与 HTTPS（H-015/016/017/020 待人工，M1-067 阻塞）、完整客户端 M1 闭环（无样式骨架与部分夹具已做，M1 真服部署/战斗闭环未做）、20 人真人测试服（H-054/056）、坦克联网渲染与老城实景素材（*b 仅完成占位渲染，实景与联网接线未完，M1-064 阻塞）都不具备。没有任何一步记为通过。

## 结论

M1 的服务端与模拟层（tick、带宽、AOI 同 tick 审计、sim 级延迟补偿、倒地/压制状态机、补给闭环、载具装甲判定、反作弊计数、无样式 UI/音频/驾驶夹具）已有工作站实测或单测支撑；但参考机性能（E1-6/E1-7，H-021）、真实网络端到端延迟补偿、完整 M1 场景长跑、素材运行时接入（M1-064）、预发布部署（M1-067）、真人测试局（E1-1，H-054/056）与对标录像（M1-071，H-011/025/026）均未完成。不建议关闭 M1，待 Fable 审阅检查点 1 裁决。

### B6-assetlod（2026-11-20）

M1-002 合成 B 的 LOD0 实测 1,288,944 / 5,294,362 B（24.35%），全流程 19.54 s。双跑 4 级网格、碰撞和统计文件的 SHA-256 均一致。此项计入 E1-14 的构建期证据，E1-9/E1-14 的整包与运行时验收沿用上表。详见 `reports/m1-002-lod.md`、PI-B6-assetlod-1（工作站，PI-109）。

真实 CC0 WaterBottle（4×2048² PNG）的 LOD0 为 1,734,656 / 8,966,928 B（19.35%）。登记该测试夹具后，`ledger-check` 为 242 行，OK 232、PENDING 3、FLAGGED 1、REJECTED 0、INTERNAL 6。

## B6-gwticket 补测（E1-1 / E1-10 的入局前置）

M1-028 的工作站网关签票（PI-109，非参考机）各七轮，100／500 并发 P50 4.748／25.333 ms、P99 6.219／29.740 ms，500 并发最慢轮 P99 41.436 ms。200 并发的 P95 7/7 轮 ≤50 ms。详见 [B6 签票报告](m1-gateway-ticket-b6.md) 与 PI-B6-gwticket-1。仅验证 loopback 签票，不代表部署耗时或真人入局；E1-1／E1-10 状态不变，参考机和受信任 HTTPS 未测。

### B6-deploytiming（2026-11-20）

工作站无头 Chromium（SwiftShader WebGL2，PI-109 非参考机）经真实本机 `frontline-gateway` + `frontline-server`（`carentan/m1` `assault`）与生产 `client/dist` + 30 个 Carentan 地形 `.fpak`（5,714,050 B）完成两档 CDP 节流 × 冷/热 OPFS 各 5 次测量：20 Mbps / 50 ms RTT 冷/热点击到可操作 P50 **6,249.2 / 1,419.4 ms**，5 Mbps / 100 ms RTT 冷/热 P50 **12,850.8 / 1,832.6 ms**（最慢样本 12,997.6 ms ≤ 15 s）；`R_near=4` 预取 20/20 轮 **81/81（100%）**，热 OPFS `.fpak` HTTP 请求 0 次（0 B）。同步修复 `ManifestIndex` 漏索引地形 `chunk` 字段。详见 `reports/m1-deploy-timing.md`、`reports/m1-deploy-timing.json`、PI-B6-deploytiming-1；因缺参考机、M1-064 实素材整包与 M1-067 预发布部署，E1-10 保持 `partial`。

### B6-fpakmedium（2026-11-20）

E1-9 和 E1-14 的 Medium 构建门禁通过。运行时素材摆放、混音、美术和参考机验收仍待验证。见 `reports/m1-fpak-medium.md`、PI-B6-fpakmedium-1。

### B6-gapsdecide（2026-11-20，E1-17）

已在 `docs/ASSET_GAPS.md` §2（34 个 GAP）与 §3.8（M1 72 个需求 ID）及 `reports/m1-placeholders.md` 按 `docs/ASSET_SHOPPING_LIST.md` §8/§9、`docs/MILESTONES.md` §2.1/§2.2 与 `PLAYABLE_SCOPE.md` 零采购默认假设完成 72/72 项逐 ID 裁决：5 项来源级 `real`（`V-NAT-002`/`004`/`005`、`V-PRP-004`/`006`）免费 CC0 采用、36 项（视觉 12 + 音频 24）免费候选采用配程序化占位回退、15 项（视觉 15）纯程序化占位且真素材推迟到 M2、16 项（视觉 7 + 音频 9）非 M1 垂直切片范围推迟到 M2，并逐项写明程序化几何/位姿/粒子/8 边形弹孔环或 48 kHz 0.1 s 静音 `AudioBuffer` 占位规格；TT 已购 6 个 UE 包保持 `INTERNAL` 不入主分支，需人类购买/外包/手动下载项仅登记 `docs/NEEDS_HUMAN.md`（H-047/H-049/H-058/§11.7），未编造已采购。因运行时接线与人工签字未闭环，E1-17 结论保持 `partial`（PI-B6-gapsdecide-1）。

### B6-deploytiming-land（2026-11-20，E1-10）

核对里程碑线已收录 #158 的部署测量脚本、`ManifestIndex` 回归和 PI-109 工作站结果（PI-B6-deploytiming-land-1）。本轮部署耗时未重测。参考机、实素材整包、预发布环境与主页面 UI 直连预取仍待验收，E1-10 为 `partial`。

### B6-linemerge-plan（2026-11-20，分析，不新增验收结论）

[双线合流评估](../docs/LINE_RECONCILE.md) 固定 MS `0e5b41f`／P `196a3de`（协议 v4／v10，v8 为 P 历史版）。预估 131 个冲突路径；保留现有网关和 M1 证据，不用 P1 本机试玩长跑代替 M1 实服矩阵。MS 工作站（PI-109，非参考机）`scripts/local-ci.sh determinism` 通过，配套严格比较 3×18,000 tick 的 native/WASM 哈希和位置完全一致。合流候选、跨版本实物互连、参考机、预发布与真人复测均未测，原 E1 判定不变（PI-B6-linemerge-plan-1）。

## B6-acceptdemo 本机八步复跑（2026-11-20）

`scripts/demo-m1.sh` 退出 0，[八步报告](m1-demo-run.md)逐步记录 8/8 本地子项。浏览器使用 HTTP/WS 和生成战术碰撞图，驾驶使用独立 M0 WS 夹具，Medic 与据点胜负使用 sim 用例。主动停服前 `/metrics` 采到 4,416 个 tick 样本，网关数据库保存了 4,418 tick 的结束记录。正式服、受信任 HTTPS、20 真人、实图演示录像、自然结算界面、参考机性能和对标复测仍未测。M1-072 为 `partial`，M1 保持开放，见 PI-B6-acceptdemo-1。

### B6-suppress-e2e（2026-11-20，E1-13）

工作站（PI-109，非参考机）普通 WS 和 netem 5% 各通过 1 轮，后者实测丢弃 1,061/20,958 包。HUD 压制强度、原生音频截止频率与增益变化、C4 生存事件及 HUD 恢复均有断言。详见 `reports/m1-suppress-e2e.md`。

### B6-uilogic（M1-053b，2026-11-20）

工作站（PI-109，非参考机）无头 Chromium：画廊初始/滚动后 62/65 个元素，8 人小队 HUD 74、空 HUD 宿主 1；四者各 60 静止帧 0 次 Paint，阳性对照分别 4/6/4/4 次。100 个独立 signal/帧的合成提交耗时均值 0.456 ms、P95 0.600 ms。完整子集中文首屏 78,786 B Brotli（预算 409,600 B）；无样式画廊键盘、焦点恢复与按需分片通过无头验证。[测量数据](m1-ui-framework.json)；[批次记录](../out/batch6/uilogic-NOTES.md)。不代表整局 CPU 或参考机 E1-6/E1-7，未改变 M1 退出判定；视觉待 Fable。

### B6-gwstats（2026-11-20，M2-022）

沿用 M1 的 server→gateway 结算回投，PI-109 工作站（非参考机）验证了真实本机进程直接回投（61 tick），以及网关离线时落盘、重启后重放（91 tick）。两条结果均可按对局 ID 查询。使用 M0 空世界夹具，真人自然结算与参考机未测。详见 [M2 网关报告](m2-gateway-stats.md) 和 PI-B6-gwstats-1。

### B6-gwadmin（2026-11-20，M2-023）

网关回归覆盖原有游客登录、签票、注册、心跳和结果接口。管理 API、举报与验收边界见 `reports/m2-gateway-admin.md` 和 PI-B6-gwadmin-1。

### B6-bwreg（2026-11-20，M2-020）

PI-109 工作站的无损 M1 调度路径对照通过（WT 400 包 + WS 300 包逐字节一致），合成 AOI 与实际 WT/WS 弱网结果见 [M2 带宽调节器报告](m2-bandwidth-regulator.md)。不以 M2 探针替代 M1 正式工况，不改变 E1 或 M1 判定。

### B6-balance（2026-11-20，M2-018）

M2 服务器规则采用协议 v5 与 v2 连接票据。规则、回归证据与试验边界见 [M2 服务器规则验收](m2-server-rules.md)（PI-B6-balance-1）；工作站结果不替代 M1 的参考机、真人或公网验收，M1 判定不变。

### B6-pingchat（2026-11-20，M2-019）

PI-109 工作站 10 个 WebSocket 无头会话通过 Ping 与文字聊天分组验证。权限矩阵共 700 项，工作站预算和未测项见 [M2-019 报告](m2-019-pingchat.md)。原 M1 判定沿用。

### B6-scoring（2026-11-20，M2-017）

计分与经验及 C4、网关摘要已接入；5v5 + 2 bot 加速 36,000 tick 对账一致。详见 [M2 计分报告](m2-scoring-b6.md)。工作站非参考机（PI-109），M2 缺失生产者和参考机未测，M2-017 保持 `partial`；不改变 M1 退出判定。

Carentan 本机 1,225 s WS 复验 10/10 HQ 部署、0 断线和 0 解码错；36,000 Live tick 后自然结算，12 条参与记录合计 1,560 净分。该 5v5 + 2 bot 长跑无网关、无真人，未取代 M1 的正式验收工况（PI-B6-scoring-2）。

并入 M2-019 后，v11 短跑及加速对账均复验通过（PI-B6-scoring-4）；M1 退出判定不变。

### B6-outpost（2026-11-20，M2-011）

设施规则复用 M1 出生、补给、弹道和装甲路径。PI-109 工作站验证第 18 次小队出生成功、第 19 次被拒，以及 7 发无头击毁 Sherman。M2 验收与依赖偏差见 [M2-011 验收](m2-outpost.md) 及 PI-B6-outpost-1。
### B6-resources（2026-11-20，M2-009）

战略资源、节点规则和客户端投影的 PI-109 工作站结果见 [M2 资源验收](m2-resources.md) 与 PI-B6-resources-1。参考机及 Carentan 五线地图联验未测，M2-009 记 `partial`。
