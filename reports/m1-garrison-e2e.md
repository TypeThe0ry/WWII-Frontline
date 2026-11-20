# E1-11：Garrison 与补给浏览器闭环（B6-browsergarrison）

2026-11-20，EPYC 7402 工作站（PI-109，非参考机）。Playwright Chromium headless shell 153.0.8010.12，关闭硬件 GPU，无场景渲染。真实本机 `frontline-gateway`、`frontline-server` 和 Vite；无 mock C0/C1/C2/C4、无 sim 调试传送或修改库存。两名玩家分别使用 WS 或强制 WT，WT 使用自签开发证书的 SHA-256 固定，无 WS 回退。

## 覆盖范围与断言来源

- 两个互动页面分别控制同一 US 小队的 Officer 与 Support，经服务器列表、访客身份和真实签票加入。另有三个不部署的会话（两个 GER、一个 US）；用于满足真实自动分队及 Support 小队至少三人的限制。占位会话走独立回环源地址上的 WS 代理，不修改服务器每 IP 上限，也不代理两名被测玩家。
- 使用生成的 FCOL 平台/落差、HQ 和据点，套用 M1 战术地图加载与规则；不是正式 Carentan 地图。浏览器夹具复用 `startNetGame`、WASM、`createDeployActions`、输入采样和 `LivePlacement`，通过真实网络推进。绕过的是尚未完整接入正式页面的部署和放置 DOM、指针锁与渲染，不替换玩法或网络层。
- Support 等 HQ Depot 生成库存，按住 USE 取箱，C0 从 `NotCarrying` 变为可放置；搬出 HQ 保护区后放箱。Officer 放置前先收到 `NoSupplies`，Support 尝试使用驻点工具收到 `WrongRole`。Officer 的预览报价 50；实际成功提交的服务端日志按玩家及驻点 ID 核对，同队落地箱库存 **100 → 50**。队友从 C4 收到同一驻点的 `Building → Active`。
- Officer 坠落死亡，再通过 C0 选择该 Garrison。Officer 的 C2 生命值恢复为正，Support 的独立 C2 解码确认其权威出生坐标在驻点 8 m 内。Support 在队友旁放 Ammo Crate；队友打空弹匣、装填及按住 USE，C2 权威备弹 **120 → 90 → 120**。

## 放置实测

以下每种传输各两个闭环样本，不是吞吐、分位数或传输性能比较。选槽到回执含预览、自动确认及输入调度；确认到回执从 `Mouse0` 提交意图开始，止于 `LivePlacement` 收到成功 C0 回执。建造段是 Support 浏览器收到 `Building` 到 `Active` 的时差，含自动测试开始按住 USE 的间隔。

| 传输 / 轮次 | 选槽到成功回执 | 确认到成功回执 | Building → Active | C2 出生距驻点 |
| --- | ---: | ---: | ---: | ---: |
| WS / 1 | 62.9 ms | 33.6 ms | 20.100 s | 7.789 m |
| WT / 1 | 60.9 ms | 33.3 ms | 20.100 s | 0.988 m |
| WS / 2 | 63.1 ms | 33.6 ms | 20.067 s | 0.988 m |
| WT / 2 | 62.7 ms | 33.6 ms | 20.199 s | 7.758 m |

## 复跑

```sh
CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo build --locked -p frontline-gateway -p frontline-server
CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism
pnpm -C client exec playwright test -c playwright.garrison.config.ts --fully-parallel --workers=2
CARGO_BUILD_JOBS=8 scripts/local-ci.sh ts
CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -p frontline-sim --features test-util --test spawn_garrison --test supply_ammo
CARGO_BUILD_JOBS=8 FRONTLINE_C4_SOAK_SECS=10 scripts/loadtest-netem.sh --netem 'loss 5% seed 26' --c4 wt
```

- Playwright：两轮各 2/2。确定性：三种子各 18,000 tick，原生/WASM 各重复两次，逐 tick 哈希和位置完全一致。TS：96 文件、734 项通过。sim 驻点/补给：14 + 8 项通过。WT C4 短程丢包回归：配置 5% 丢包，实测 5.259%；100 条编号通知/客户端，驻点状态己方 3 条、敌方 0 条（独立 C4 回归，不是浏览器闭环在 netem 下复跑）。
- 真实浏览器用例发现 sim 仅发建成及摧毁事件，缺初始 `Building`。现由成功创建驻点的 sim 路径入队，通过既有同队 C4 广播，不改 wire 格式。服务端实际扣费审计为 DEBUG 日志，不向客户端公开库存。
- E1-11 保持 `partial`：完成本机双浏览器逻辑闭环；正式 Carentan 资源、主游戏页面的完整部署/放置/交互 UI、真人演示、bot 完工后重生、受信任证书与参考机未测。未新增美术素材或变更现有视觉。
