# M1 八步演示运行记录

- 运行日期 2026-11-20。检出提交 `3a7af8f`。命令 `scripts/demo-m1.sh`。
- PI-109 工作站（非参考机）。本机 HTTP/WS、无头 Chromium 软件 WebGL2，使用生成的战术碰撞图和独立 M0 驾驶夹具。
- 脚本退出码 `0`。下表的 PASS 对应本地子项。

| §2.4 | 演示步骤 | 本地检查 | 待人工演示 |
|---|---|---|---|
| 1 | 启动与主菜单 | PASS — Chromium 153.0.8010.12 实际探测软件 WebGL2，启动页进入主菜单。 生产 core（含 WASM）core: 195 files, raw 4414287 B, brotli 1193738 B (max 8000000); data 13375 B。 | 未测（需人）：预发布域名、受信任 HTTPS 与正式包下载过程（H-017）。 |
| 2 | 服务器列表、游客入局、HQ 部署 | PASS — 服务器列表显示已注册对局。2 个 Chromium 游客取得真实签票并通过 WS 入场，US 小队 Officer/Support 的 HQ 部署获 C0 确认和存活快照。 | 未测（需人）：20 人测试服、地图画布键鼠部署、倒计时内 Carentan 实图预取演示（H-054/H-057）。 |
| 3 | 移动、射击与 Garrison 重生 | PASS — 步行到达放置点，队友快照确认蹲下与站起，Garand 与 Thompson 均消耗权威弹药（Thompson 30 发），Garrison 扣除 50 补给并激活，重生距驻点 5.61 m。 | 未测（需人）：翻越、全息绿/红和实图真人队友操作（H-057）。 |
| 4 | Support 补给与补弹 | PASS — Support 在 HQ 取箱并搬运，放置补给箱和弹药箱。Officer 备弹 120 → 90 → 120。 | 未测（需人）：实图真人搬运、放置与补弹（H-057）。 |
| 5 | Medic 与压制 | PASS — sim 夹具通过受伤/倒地/救起和压制规则检查。 | 未测（需人）：浏览器内真人 Medic 救援与压制音效/屏幕效果（H-057）。 |
| 6 | Sherman 驾驶 | PASS — 独立 M0 WS 驾驶，权威入座 → 行驶 → 离座。driver seat 0 on vehicle 128: 87 snapshots, 11.17 m authoritative displacement, then exited。 | 未测（需人）：三人乘员、实图路线、游戏内双方交战（H-054/H-057）。 |
| 7 | 据点规则 | PASS — sim 对局夹具通过预热、据点推进与胜负条件检查。 | 未测（需人）：L1a/L2a/L3a 真人拉锯及战术地图显示（H-054/H-057）。 |
| 8 | 指标、素材报告与结算 | PASS — HTTP /metrics 返回 4416 个 tick 样本。主动停服触发结果上报，gateway SQLite 确认 ended=1、4418 ticks、1 个据点。 | 未测（需人）：预发布监控看板及真人对局自然结束后的结算展示（H-057）。 |

本地子项 8/8 取得通过证据。完整 M1 退出标准见 [E1-1～E1-17](m1-acceptance.md)。现场材料见 [STATUS](../tasks/STATUS.md)、[占位决定](m1-placeholders.md) 和 [构建缺失清单](../assets/manifests/missing-report.json)。
