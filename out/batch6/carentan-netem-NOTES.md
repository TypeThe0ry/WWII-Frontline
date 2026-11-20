# B6-carentan-netem（2026-11-20）

在 PI-109 非参考机工作站完成 `carentan/m1` Assault WT 长跑。私有 netns `lo` 使用 `tc netem delay 25ms 10ms loss 2% seed 26`，确认丢弃 52,118/2,594,297 包，实测丢包率 2.009%。使用 `FRONTLINE_SIM_CPU`、异核网络/日志和 tmpfs。20 headless 与 12 bot，20/20 完成 HQ 部署，全员重叠 1808.767 s。

| 检查 | 实测 |
| --- | --- |
| `scripts/loadtest-m0.sh --transport wt --map carentan/m1 --mode assault --deploy-hq --count 20 --bots 12 --duration 1810 --seed 1 --no-build` | 退出 0。tick P50/P99/max 3.809/8.027/84.295 ms，7 次超时、11 个跳过定时槽，最长连续 1。 |
| 带宽上限 | 下行 P50/P99 20.352/25.336，上行 20.456/21.376 kbps。最差客户端下行/上行 P99 为 27.544/21.456 kbps。 |
| 同 tick AOI | 709,320/709,320 快照配对，397,531 条敌方玩家记录，客户端和服务器均 0 越界。 |
| HQ 载具 | US Sherman/Jeep，GER Panzer IV H/Kübelwagen。182 次采样均为 4 辆在场且有血量。20 个客户端采样中共解码 973 条载具记录。 |
| `FRONTLINE_DETERMINISM_STRICT=1 CARGO_BUILD_JOBS=8 scripts/local-ci.sh determinism lint` | 两组通过。三种子共 54,000 tick 的 native/WASM 哈希一致。重基至 `1f889bf` 后 `lint` 复检通过。 |
| `CARGO_BUILD_JOBS=8 cargo test --locked -p frontline-server --test aoi_audit` | 2 项通过。 |
| `node tools/ci/check-acceptance.mjs` | 0 报告问题，0 未豁免开放任务。 |

临时源码夹具使用可玩线车型参数和里程碑线物理接口。正式车型、有人驾驶与交战、独立载具 AOI、参考机、独占核、外网与真人未测。E1-2/3/4、M1-068 与 M1-072 保持 `partial`。视觉部分待 Fable 设计。人工依赖已补入 H-017/021/054/056，计划偏差见 PI-B6-carentan-netem-1。复现命令见[长跑报告](../../reports/m1-carentan-netem-b6.md)，数据见[实测 JSON](../../reports/loadtest-1119/carentan-wt-30min-b6-netem.json)。
