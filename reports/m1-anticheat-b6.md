# B6-antiche：E1-15 基础反作弊（2026-11-20）

环境：共享 EPYC 7402 工作站，PI-109 **非参考机**。本批只验证逻辑与本地网络路径，没有使用 GPU 或 GitHub Actions。

## 阈值与起点提示

- `data/rules/anticheat.json` 的 `view_max_deg_s` 默认 **3,600°/s**，按 yaw（最短环绕差）和 pitch 分别检查；30 Hz 正常连续输入的上限为 **120°/tick**。旧默认 8,000°/s 在单 tick 不可触发；新值比 3,000°/s 脚本峰值高 20%。数据 schema 和运行时都拒绝 ≥5,400°/s。没有更改累计违规门槛、权威移动或武器冷却。
- `FireCmd.origin_hint` 用世界 XYZ 的 14-bit 坐标，0.25 m/码，基点 `(0, -128, 0)` m，全零表示不可用。单轴舍入误差 ≤0.125 m；不可用或超界不夹到边界。它不改变原有 wire 位宽，也不复用 `Shot`/`Impact` 的较小坐标域。
- 浏览器使用 WASM 当次步兵主武器预测的枪口，随 C1 输入打包，提示随 `seq` 冗余、确认后停止、回放不重写。未定位、缺碰撞、非主武器或乘车时不填有效提示。浏览器当前默认只装载 M0 碰撞，连 Carentan 时省略提示；真实 Carentan 场景与正式证书浏览器未测（H-017）。
- 服务器只对同序号、确实发射的枪口比较，偏差 **>0.5 m** 才递增 `origin_corrected_total`。提示不影响权威起点、弹道、射击许可或踢出。重复、过晚、缺失和未发射的提示不计。合成 Carentan 坐标（含负高度）和真实 WT 输送分别通过回归，不当作实服联测。

## 专项回归与网络对照

| 工况 | 实测 |
| --- | --- |
| 服务端默认视角回归 | 180° 单 tick 瞬转钳到 119.9982°；1,000 条合成合法转向中，含 3,000°/s 峰值，视角钳制和踢出各 0。另送 334 条过时序号重包，全部按重复包处理。 |
| 1.3× 移动输入 | 900 tick、每 5 tick 注入一次；180 次钳制，最大水平移动速度 6.343 m/s。 |
| 2× 射速 | Garand 40 次扳机 / 10 s，20 发出膛、20 次拒绝；为隔离冷却判定，测试扩大弹匣，不是生产弹匣容量。 |
| 真 WS 混合注入 | M0、6 客户端、30 s、0 bot。移动钳制 270、视角钳制 180、射速拒绝 29；仅 2 个预期踢出（原因 1、2 各一次），客户端退出码 0。 |
| 真 WS 混合注入（20 人复测） | M0、20 客户端、300 s、0 bot，20/20 入场。移动钳制 1,890、视角钳制 1,800、射速拒绝 175；仅 2 个预期踢出（原因 1、2 各一次），解码错误 0，客户端退出码 0。 |
| 真 WT 起点提示 | 4 局平地夹具均保留 `HitConfirm`（4/4）；两个偏离提示各 +1 诊断，两个无提示各 0，均无踢出。配套服务端夹具也比较同 Seed 的 `Shot`、projectile、state hash，均与无提示对照相同。 |
| 真 WT 短时正常对照 | M0、6 客户端、90 s、0 bot；私有回环 `netem delay 25ms 10ms loss 2%`，`tc -s` 记录丢包 772 个。69 个合成高灵敏度峰值，视角/移动钳制、射速拒绝、踢出、客户端断开各 0。3 次输入停顿、52 次普通预测回滚，客户端退出码 0。 |
| 输入校验微基准 | Criterion 30 样本，单输入 55.235 ns（54.953–55.592 ns），100 玩家各 3 条新鲜输入 17.690 µs（17.502–17.897 µs）。为进程内桥接摄入/resolve，不含传输、世界步进或参考机。 |

专项命令（已准备 M0 数据）：

```sh
export CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cargo test --locked -p frontline-server --test anticheat --test lagcomp_transport -- --nocapture
cargo test --locked -p frontline-headless-client --lib -p frontline-protocol
cargo build --release --locked -p frontline-server -p frontline-headless-client \
  --features frontline-headless-client/anticheat-test
cargo bench --locked -p frontline-server --bench anticheat_on_input -- --sample-size 30
# WS_PORT 取自独立测试服日志，OUT_DIR 为已创建的测试输出目录：
target/release/headless-client --url "127.0.0.1:$WS_PORT" --transport ws \
  --count 20 --duration 300 --seed 1 --anticheat-mix --report "$OUT_DIR/mix.json"
```

## 30 分钟正常对照方法

真实本地 WS、M0 sandbox、20 名脚本客户端、0 bot、种子 1。偶数 ID 输出 3,000°/s 峰值的 yaw/pitch 合成鼠标样本，奇数 ID 使用原路线转向；两组都用合法移动和最少间隔 2 秒的扳机节奏。不是人类输入录像。此对照不发送 `FireCmd`，因此不能用它的起点计数验证提示链路。使用新阈值的 release 二进制开始长跑；后续改动只涉及该对照没有发送的起点提示、测试和文档。

```sh
PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0
CARGO_BUILD_JOBS=8 cargo build --release --locked -p frontline-server -p frontline-headless-client
# 终端 1：独立新服；记录它日志中的 WS 和 metrics 端口。
target/release/frontline-server --data-dir "$PWD/data" --map m0 --mode sandbox --bots 0 \
  --ws-port 0 --metrics-port 0 --wt-port 0 --dev-cert --duration 1850 --report "$OUT_DIR/server.json"
# 终端 2：WS_PORT / METRICS_PORT 取自新服日志。
curl -fsS "http://127.0.0.1:$METRICS_PORT/metrics" > "$OUT_DIR/metrics-before.txt"
target/release/headless-client --url "127.0.0.1:$WS_PORT" --transport ws --data-dir "$PWD/data" \
  --map m0 --count 20 --duration 1800 --seed 1 --anticheat-control --report "$OUT_DIR/headless.json"
curl -fsS "http://127.0.0.1:$METRICS_PORT/metrics" > "$OUT_DIR/metrics-after.txt"
```

判据：20/20 入场且每个连接 ≥1,800 s，10 个偶数客户端生成非零 `flick_peaks`，0 客户端异常断开或解码错误；`view_rate_clamped_total`、`suspicion_move_total`、`fire_rate_rejected_total` 与 `session_kicked_total` 的前后差均为 0。C1 冗余重复包、时序丢弃与网络停顿是另一类统计，`pos_corr` 预测回滚也不是反作弊处罚。

## 本地门禁

- `CARGO_BUILD_JOBS=8 scripts/local-ci.sh lint data determinism wasm ts` 对应组通过（实跑分组）；TS 97 文件、739 用例通过，含真实 WASM C1 提示打包、去重与缺状态省略回归。
- 确定性 3 个种子 × 18,000 tick：native 重复、WASM 重复、跨端状态哈希均逐 tick 相同，位置 RMS / max 均为 0 m；WASM 组另测 3,000 tick，80 B `PlayerAuthState`、33 B `WeaponState` 均逐 tick 比特一致。
- WASM 体积 1,161,592 B（上限 1,200,000 B），Brotli 336,239 B（上限 400,000 B）。
