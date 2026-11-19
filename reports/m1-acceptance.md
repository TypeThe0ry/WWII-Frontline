# M1 里程碑验收报告（M1-072，部分）

状态：**M1 未达到可关闭条件。** 以下为 E1-1…E1-17 的核对结果，数字全部取自 `tasks/STATUS.md` 与 `reports/*.json`，未重新估算、未用别的机器数字顶替。工作站 GPU 可用（总控 2026-11-18 通知），GPU 相关项（E1-6/E1-7/E1-8 及 *b 任务）由工作站会话补测后再更新本表。

| E# | 标准 | 目标 | 实测 | 方法/命令/报告 | 结论 | 遗留(PI-xxx) |
| --- | --- | --- | --- | --- | --- | --- |
| E1-1 | 真人测试局 | ≥20 min、≥20 真人每队 10 | 无真人测试局；仅 headless 20 客户端 + 12 bot 300 s | `reports/m1-playtest-log.md`；人类 H-054/H-056 | 未测 | PI-104 |
| E1-2 | 服务器 tick | P99 ≤ 28 ms，稳态 ≤ 18 ms | 工作站（EPYC 7402，共享）：ws 300 s 四组 P99 4.8–5.6 ms、max ≤29 ms；**WT 30 分钟三组**：无损 P99 5.8 ms、max 201 ms（无 trace）；**+netem（25±10 ms，2 % 丢包）P99 5.30 ms、max 34.5 ms、超时 1 次**；无损 trace P99 5.28 ms、max 153.8 ms。**尖峰已归因**：153.8 ms 那次 on-CPU 仅 2.35 ms、运行队列等待 151.3 ms（机器平均负载 136，我们固定的核 90–99 % 忙），即共享工作站的调度饥饿，不是服务器代码；较安静时最坏是 34.5 ms 的一次真实计算尖峰（on-CPU 32.3 ms）；**阶段计时复测（三组 30 分钟）**：P99 5.2–5.8 ms，max 25–43 ms，>20 ms 的 8 次里 7 次是调度等待，1 次（37.5 ms）是 sim step 内的真实计算，step 慢的 tick 号跨运行重复（内容触发），具体子系统未定位 | `reports/m1-loadtest-netem.md`（尖峰归因节）、`reports/loadtest-1119/wt-30min*.json` | **PASS（P99/稳态）；尖峰为环境调度，需独占核复测确认** | 参考机/独占核 |
| E1-3 | 带宽 | P50 ≤ 128、P99 ≤ 400 kbps，上行 ≤ 32 kbps | 各运行（含 2 %/5 % 丢包+抖动与 150 ms RTT、WT 30 分钟）下行 P50 14–21 / P99 22–30 kbps，上行 P50 20.5 kbps | 同上 | **PASS（工作站）** | — |
| E1-4 | AOI | 视锥外且 >96 m 敌人下发 = 0 | 三层审计均为 0：(1) `aoi_audit.rs::no_enemy_is_sent_outside_the_r3_rules`（20 人 10v10 场景，独立复述 R3 界限）通过；(2) 服务器运行时计数 `aoi_enemy_leak_total` = 0；(3) **新增的客户端独立审计**（`headless-client/src/audit.rs`：按规范复述 R3，用客户端预测位置、24 tick 朝向历史、+4 m/+5° 容差，比服务器宽松，所以报告的泄漏都是真违规）在 20+12 bot 联测里检查：ws 无损 306 006 条敌方记录（其中 >105.6 m 的 48 007 条，真正考验视锥规则）、40 ms±15 ms+5 % 丢包 285 622 条（63 423 条）、150 ms RTT 301 179 条（53 754 条）——**泄漏 0**。共约 89 万条记录、16.5 万条远距记录。联测 verdict 现在把两个泄漏计数作为硬判据。局限：客户端审计的容差意味着落在容差内的细微越界不会被发现（精确界限由 `aoi_audit.rs` 保证）；审计在加入后 3 s、死亡/重生/位置修正 >5 m 后 2 s 内跳过（预测位置不可信，首版没有这个跳过时报告过 40 条“泄漏”，查明是这个启动假象，不是服务器问题） | `reports/loadtest-1119/aoi-ws-*.json`；`cargo test -p frontline-server --test aoi_audit`；`cargo test -p frontline-headless-client`（audit 单测 3 项） | **PASS** | PI-071、PI-121 |
| E1-5 | 延迟补偿 | 150 ms RTT 命中率差 ≤ 5% | 瞄准射击场景（`crates/sim/tests/lagcomp_hitrate.rs`，sim 级，每组 600 发；目标随机速度/方向/折返，射手**按客户端看到的位姿瞄准**并带 ±0.2 m 人为瞄准误差）：**0 ms RTT 命中率 90.7 %（544/600），150 ms RTT 90.7 %（544/600），差 +0.0 个百分点（PASS，标准误约 2 pp）**；150 ms 且视图延迟估计抖动 ±1 tick（33 ms）86.3 %，差 -4.3 pp（仍 ≤5）；对照（关闭延迟补偿）0 ms 36.2 %、150 ms 21.0 %。这是 sim 级检验：网络用“服务器被告知的回溯 tick 数”表示（0 ms = 3 tick 插值，150 ms = 3+2 tick），传输层的丢包/抖动由 `scripts/loadtest-netem.sh` 另行覆盖 | `cargo test -p frontline-sim --test lagcomp_hitrate --release -- --nocapture`；`reports/m1-loadtest-netem.md` | **PASS（sim 级）；真实网络端到端的瞄准对照未做** | PI-115 |
| E1-6 | High 性能 | P95 ≤ 16.7 ms、≤1200 draw call、≤4.0 M 三角形 | **工作站数字（RTX 3080，非 3060 参考机，PI-109）：draw call/三角形通过，CPU 通过，P95/卡顿无法判定**。draw call 84/205/145（PASS）、三角形 ≤0.45 M（PASS）、提高优先级后 CPU P50 8.2–9.3 ms（目标约 10.4，PASS）；帧 P95 16.8–83 ms、卡顿帧 6–169/60 s（限 2）但**已查明来自共享 GPU 被其它任务占满（93–100 %）**，与植被上传无关（对照实验，PI-114）：该项需空闲 GPU/参考机重测，**不记 PASS**。分 pass GPU 预算、wasm 内存未测 | `reports/m1-perf.md`（E1-6 复查节）、`reports/m1-perf.json`、`reports/m1-perf-raw/` | **未判定（工作站被占用）；参考机未测** | H-021、空闲 GPU 复测 |
| E1-7 | Low 性能 | ≥30 fps，draw call ≤900/500 | 未测（无 Iris Xe 级机器，且渲染器没有 Low 档运行路径的 M1 采集；High 档工作站 draw call 84–205 仅供参考） | `reports/m1-perf.md` | 未测 | M1-069b、H-021 |
| E1-8 | 材质变体 | ≤ 60 | 运行时场景实际使用的材质 18 种（≤60，**代理量**：材质数，不是 CI `variant-count` 的着色器变体数；含 M0 灰盒占位）；`variant-count` 的 M1 数字仍未产出 | `reports/m1-perf.md`（材质节） | 部分（代理 PASS；变体数未测） | M1-069b |
| E1-9 | 包体积 | core ≤ 8 MB；首图 Medium ≤ 150 MB | core brotli 682,349 B（≤ 8,388,608 B）；首图未测 | `tools/ci/core-size.mjs` | partial | M1-064 阻塞（素材） |
| E1-10 | 部署流程 | 键鼠可达；点击到可操作 ≤ 15 s；预取完成率 ≥ 95% | M1-040c 在模拟 WS / WT 上各通过 1 例真实 WASM 的 C0 部署结果到 `deploy.state` 测试；键鼠、到可操作的耗时及预取完成率均未测 | `client/src/net/connect.test.ts`；M1-056b/057b/061b | 未测（仅验证接收路径） | PI-092 续 |
| E1-11 | Garrison + 补给闭环 | 闭环可玩 | `e2e_garrison_supply_loop` 通过（取箱→放 Garrison 扣 50→队友 8 s ±1 tick 重生）。M1-060b 的 13 个真实 WASM 客户端逻辑用例通过，覆盖 8 种 Garrison 拒绝原因及补给箱/弹药箱预判。实局客户端、放置耗时和补弹闭环未测 | M1-019/020、`client/src/game/placement-check.test.ts` | partial | PI-107 续 |
| E1-12 | 坦克 | 3 座位；≥12 组 AP 与手算一致；120 ms RTT 直线校正 ≤1 次/10 s | 13 组 `armor_cases.json` 全一致；校正频率未测 | M1-023、M1-022 | partial | PI-070 |
| E1-13 | 倒地/救起/压制 | 状态机全转移单测；压制被客户端音画消费 | 016/017 单测通过（转移 ≥10 条）；网络对局已订阅压制音频。工作站无头 Chromium 的合成 PCM 在第二次压制后 400 ms 恢复到基准：低频振幅与高低频比均约 1.00002×。真实素材、联网压制整链与音画联验未测（非 RTX 3060 参考机，PI-109） | M1-016/017；`client/e2e/audio-offline.spec.ts` | partial | PI-053、PI-124B |
| E1-14 | 素材管线 | `.fpak` 往返一致；无独立 .glb/.ktx2/.ogg/.opus；缺失报告；无 PENDING 进主分支 | 台账 8 行：OK 4、PENDING 3、FLAGGED 1、REJECTED 0；仓库内 2 个 `.ktx2` 为测试夹具；PENDING 对应处理产物未入主分支 | `node tools/asset-pipeline/bin/asset-pipeline.mjs ledger-check --root .` | partial | PI-108 |
| E1-15 | 基础反作弊 | 1.3× 速度、超射速被纠正/拒绝并计数 | 1.3× 注入：权威最高 6.34 m/s，`suspicion_move_total` ≥ 100；超射速 8 发出膛、56 次拒绝；甩枪 0 误报 | M1-027a | partial | PI-074 |
| E1-16 | 数据与史实 | historical 100% 有出处或"未核实"；批 M1 回填与复测在容限 | 070b 逐项在线比对 67 个字段：51 一致、10 不一致、6 未核实；JSON 来源未回填，结构审计仍为 13 已登记、54 缺口。Sherman 装甲/弹种全表验收未做；批 M1 录像为 0，未复测。 | `docs/measurements/hist-review-m1.md`；M1-071 blocked | partial | PI-124G、H-011/025/026 |
| E1-17 | 占位决定 | 范围内占位需求 ID 有"决定"或明确推迟 | 72 个 M1 视觉需求 ID 全为占位，台账 OK 行仅 4 条（字体） | `reports/m1-placeholders.md` | partial | PI-108 |

## Bot 行为补测（M1-038c / M1-039c）

`bt_scenarios` 验证了墙后 1,000 tick 零开火、听声搜索不授权开火、视觉记忆到期、Move/Attack/Defend/Hold、倒地放弃输入、5 s 卡住跳跃及补给/救治输入。各 200 次脚本采样中，Recruit / Veteran 识别后首次发出 `FIRE` 的平均延迟为 746.00 / 403.50 ms；未测真人实战延迟。共享 EPYC 7402 工作站（PI-109，非 RTX 3060 参考机）上的 10v10 M1 brain / M0 地形发布构建，8,290 次完整决策 P50 1.48 µs / P95 5.39 µs（`reports/m1-bots-bt.json`）。这些结果不替代 E1-1 真人测试或 E1-2 服务器长跑。

发布构建 soak（`cargo test --release --locked -p frontline-server --test bots_soak tactical_ -- --ignored --nocapture --test-threads=1`），同一共享工作站：

| 场景 | 模拟时长 / 墙钟 | 发射 / 击杀 / 重生 | 部署成功 / 拒绝 / 踢出 | BotHost P95（全部 bot 思考和输入摄入） | tick P99 |
| --- | --- | --- | --- | --- | --- |
| Carentan 1/3、真实据点 5、20 bot | 9,000 tick / 300 s；10.703 s | 47 / 11 / 9 | 31 / 0 / 0 | 0.064 ms | 4.741 ms |
| M0、18 bot + 2 脚本玩家 | 9,000 tick / 300 s；5.435 s | 227 / 55 / 54 | 72 / 0 / 0 | 0.134 ms | 1.217 ms |

混合组第 30 tick 填到每队 9 bot + 1 脚本玩家，共发 18,000 个 C1 输入包、处理 1 个 C0。测试为进程内模拟，无真实传输或真人；Carentan 仅激活一个据点、使用烘焙 navmesh 与解压后的 380 个 `FCOL` 地形块，不等于多据点完整对局。碰撞夹具用 `map_pipeline chunks-m1` 生成，zlib 解压 `c_*.fcol` 到 `data/maps/carentan/m1/chunks/c_*.bin`；生成物不进仓库。E1-1 不改判定。

## 演示脚本（MILESTONES §2.4）

8 步均**未执行**：预发布域名与 HTTPS（H-015/016/017/020 待人工）、真实客户端界面（TSX 未做）、20 人测试服、坦克与老城场景渲染（GPU 接线 *b 任务未做）都不具备。没有任何一步记为通过。

## 结论

M1 的服务端与模拟层（tick、带宽、倒地/压制、补给闭环、载具装甲判定、反作弊计数）在无损 headless 条件下达标或有数字支撑；客户端渲染、UI、素材、部署、真人测试局、录像对标均未完成。不建议关闭 M1，待 Fable 审阅检查点 1 裁决。
