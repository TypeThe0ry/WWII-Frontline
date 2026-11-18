# M1 里程碑验收报告（M1-072，部分）

状态：**M1 未达到可关闭条件。** 以下为 E1-1…E1-17 的核对结果，数字全部取自 `tasks/STATUS.md` 与 `reports/*.json`，未重新估算、未用别的机器数字顶替。工作站 GPU 可用（总控 2026-11-18 通知），GPU 相关项（E1-6/E1-7/E1-8 及 *b 任务）由工作站会话补测后再更新本表。

| E# | 标准 | 目标 | 实测 | 方法/命令/报告 | 结论 | 遗留(PI-xxx) |
| --- | --- | --- | --- | --- | --- | --- |
| E1-1 | 真人测试局 | ≥20 min、≥20 真人每队 10 | 无真人测试局；仅 headless 20 客户端 + 12 bot 300 s | `reports/m1-playtest-log.md`；人类 H-054/H-056 | 未测 | PI-104 |
| E1-2 | 服务器 tick | P99 ≤ 28 ms，稳态 ≤ 18 ms | 20 客户端 + 12 bot：P50 1.626 / P99 3.54 / max 38.256 ms，超时 1 次（连续 1） | `scripts/loadtest-m0.sh --transport ws --count 20 --bots 12 --duration 300`；`reports/m1-loadtest-20-ws-lossless.json` | partial | M1-068b |
| E1-3 | 带宽 | P50 ≤ 128、P99 ≤ 400 kbps，上行 ≤ 32 kbps | 下行 P50 15.96 / P99 26.6 kbps，上行 P50 20.5 kbps（无损、同机） | 同上 | partial | M1-068b（netem 未测） |
| E1-4 | AOI | 视锥外且 >96 m 敌人下发 = 0 | AOI 审计未做；aoi_encode P99 ≤ 0.5 ms | `audit.rs` 未实现 | 未测 | M1-068b |
| E1-5 | 延迟补偿 | 150 ms RTT 命中率差 ≤ 5% | 未测（无 netem、无对照场景） | M1-068b | 未测 | M1-068b |
| E1-6 | High 性能 | P95 ≤ 16.7 ms、≤1200 draw call、≤4.0 M 三角形 | 未测（判定工具已就绪；三机位未采集） | `tools/perf/budget.mjs`、`reports/m1-perf.md` | 未测 | M1-069b（工作站） |
| E1-7 | Low 性能 | ≥30 fps，draw call ≤900/500 | 未测（无 Iris Xe 级机器） | 同上 | 未测 | M1-069b、H-021 |
| E1-8 | 材质变体 | ≤ 60 | 未测（M1 无变体计数结果） | CI `variant-count` | 未测 | M1-069b |
| E1-9 | 包体积 | core ≤ 8 MB；首图 Medium ≤ 150 MB | core brotli 682,349 B（≤ 8,388,608 B）；首图未测 | `tools/ci/core-size.mjs` | partial | M1-064 阻塞（素材） |
| E1-10 | 部署流程 | 键鼠可达；点击到可操作 ≤ 15 s；预取完成率 ≥ 95% | 未测（TSX 组件与 Playwright 未做） | M1-056b/057b/061b | 未测 | — |
| E1-11 | Garrison + 补给闭环 | 闭环可玩 | `e2e_garrison_supply_loop` 通过（取箱→放 Garrison 扣 50→队友 8 s ±1 tick 重生）；真实客户端未测 | M1-019/020 | partial | — |
| E1-12 | 坦克 | 3 座位；≥12 组 AP 与手算一致；120 ms RTT 直线校正 ≤1 次/10 s | 13 组 `armor_cases.json` 全一致；校正频率未测 | M1-023、M1-022 | partial | PI-070 |
| E1-13 | 倒地/救起/压制 | 状态机全转移单测；压制被客户端音画消费 | 016/017 单测通过（转移 ≥10 条）；客户端消费未接 | M1-016、M1-017 | partial | PI-053 |
| E1-14 | 素材管线 | `.fpak` 往返一致；无独立 .glb/.ktx2/.ogg/.opus；缺失报告；无 PENDING 进主分支 | 台账 8 行：OK 4、PENDING 3、FLAGGED 1、REJECTED 0；仓库内 2 个 `.ktx2` 为测试夹具；PENDING 对应处理产物未入主分支 | `node tools/asset-pipeline/bin/asset-pipeline.mjs ledger-check --root .` | partial | PI-108 |
| E1-15 | 基础反作弊 | 1.3× 速度、超射速被纠正/拒绝并计数 | 1.3× 注入：权威最高 6.34 m/s，`suspicion_move_total` ≥ 100；超射速 8 发出膛、56 次拒绝；甩枪 0 误报 | M1-027a | partial | PI-074 |
| E1-16 | 数据与史实 | historical 100% 有出处或"未核实"；批 M1 回填与复测在容限 | 67 字段：13 有出处、54 标"未核实"（均保持 initial）；批 M1 录像为 0 | `docs/measurements/hist-review-m1.md`；M1-071 blocked | partial | M1-070b、H-011/025/026 |
| E1-17 | 占位决定 | 范围内占位需求 ID 有"决定"或明确推迟 | 72 个 M1 视觉需求 ID 全为占位，台账 OK 行仅 4 条（字体） | `reports/m1-placeholders.md` | partial | PI-108 |

## 演示脚本（MILESTONES §2.4）

8 步均**未执行**：预发布域名与 HTTPS（H-015/016/017/020 待人工）、真实客户端界面（TSX 未做）、20 人测试服、坦克与老城场景渲染（GPU 接线 *b 任务未做）都不具备。没有任何一步记为通过。

## 结论

M1 的服务端与模拟层（tick、带宽、倒地/压制、补给闭环、载具装甲判定、反作弊计数）在无损 headless 条件下达标或有数字支撑；客户端渲染、UI、素材、部署、真人测试局、录像对标均未完成。不建议关闭 M1，待 Fable 审阅检查点 1 裁决。
