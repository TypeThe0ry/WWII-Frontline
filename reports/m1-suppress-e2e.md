# E1-13 倒地、救起、复生与压制端到端验收

日期为 2026-11-20。共享 EPYC 7402 工作站（PI-109，非参考机），无头 Chromium WebGL2 SwiftShader 软件渲染，4 个浏览器页连接本机真实 `frontline-server`。使用 Assault 战术规则、程序生成的平地 FCOL 碰撞夹具及真实客户端 WASM。普通 WS 与独立 Linux 网络命名空间内挂载的 `lo` netem 5% 丢包使用相同断言。

## 验证的链路

- **压制**：浏览器走真实 C1 输入触发 sim 爆炸。权威 C2 `Suppress` 经 WASM 解码及事件总线分送生产 HUD 和 WebAudio。断言 HUD 为 `suppressed`、`data-intensity` 与总线强度按现有映射对应，原生低通和增益 `AudioParam` 同时低于基线。随后 HUD 回到 `clear`，截止频率、增益回到 20,000 Hz、1.0。
- **生存**：权威 C4 仅向受害者发送倒地和阵亡 `Death`，本队接收 `Revive` 和 roster。验证 HUD 状态与 C4 顺序一致，倒地仍有正 C2 血量，阵亡血量为零，随后自动复生到己方 HQ 并恢复满血 HUD。敌方看不到受害者的 `Death` 或 `Revive`。
- **协议兼容**：在现有 `PlayerRoster.flags` 加 `ALIVE = 0x20`，保持协议版本与消息长度。复生时由同一可靠 C4 通道通知 HUD，避免把正血量的倒地或晚到的 C2 快照判成复生。倒地和阵亡沿用现有 `Death.can_be_revived` 区分。

## 实测结果

| 链路 | Playwright | C2 强度 / HUD 强度 | 原生低通 / 增益（压制时单次采样） | C4 阵亡倒计时 / C2 观察 tick | HQ 复生坐标 (x, y, z) m |
| --- | --- | --- | --- | --- | --- |
| 普通 WS | 1/1，1.0 min | 0.17647 / 0.14118 | 12,835.06 Hz / 0.84929 | 18 s / 1296 → 1806 | (31.81, 5.00, 59.56) |
| WS + netem 5% | 1/1，1.3 min | 0.17647 / 0.14118 | 13,533.10 Hz / 0.86601 | 17 s / 1758 → 2258 | (28.84, 5.00, 60.69) |

netem 发出 19,897 包、丢弃 1,061 包，实测为 1,061/20,958（5.06%）。两轮均通过 HUD／AudioParam 恢复断言。两轮的 C4 roster 生存掩码均为 `0 → 32 → 1 → 32 → 1 → 0 → 32`，依次对应未部署、存活、倒地、救起、再倒地、阵亡、复生。两次倒地的 C2 血量字节分别为 100、99，阵亡为 0，复生为 255。`Death → Revive → Death → Death` 事件顺序与 HUD 相符。

本地门槛 `rust`、`lint`、`ts`、`determinism` 通过。TS 为 741 个测试和 8 个变体测试。严格确定性检查的 3 个种子各 18,000/18,000 tick native/WASM 哈希一致，位置误差 0 m。

## 复跑命令

按 `DEVELOPING.md` 的 E1-13 章节构建战术服和 WASM，然后执行 `TMPDIR=<本批临时目录> scripts/suppress-e2e.sh`。Playwright 输出每轮的 `[suppress-loop]` JSON。netem seed 为 26，`tc -s -j qdisc show dev lo` 在私有命名空间退出前记录计数。分母为 qdisc 发出的包与丢弃包之和，含该命名空间内的页面 HTTP 和 WS 流量，WS 仍由 TCP 重传。

## 验收边界

夹具补发尚未接到主页面的 `GADGET` 输入位，部署、普通操作、状态解码、HUD 和音频均复用生产代码。夹具只验证功能接线。正式 Carentan 地形、参考机、正式部署与受信任证书、真人参考录像、实素材混音及音画体验均未测。视觉待 Fable 设计。
