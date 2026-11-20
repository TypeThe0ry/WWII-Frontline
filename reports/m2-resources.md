# M2-009 战略资源与资源节点验收

2026-11-20，PI-109 工作站（EPYC 7402，非参考机）。资源顺序均为 Manpower / Munitions / Fuel。

| 验收项 | 工作站结果 |
|---|---|
| 1 · 整分钟发放 | Tick 1,799 为 200 / 80 / 40。Tick 1,800 双方均为 242 / 98 / 50，发出 6 条收入事件和一份双队快照。 |
| 2 · 两 Sector 加节点 | 双方各 2 Manpower、1 Munitions、1 Fuel 节点，收入各 92 / 28 / 16，首分钟池 292 / 108 / 56。 |
| 3 · 上限和溢出 | 上限 1,500 / 800 / 400。双队满池同分钟丢弃 84 / 36 / 20 单位，次 tick 增量归零。 |
| 4 · 数量限制 | 同 Sector 第二个同类节点、全队第四个同类节点被拒。三类各 3 个达到总上限 9，第十个被拒。 |
| 5 · 易手与战利品 | 真正的据点占领状态更新在同 tick 清除 3/3 已建节点，双方当分钟收入按新所有权结算。攻占方只获得一次 +50 Manpower。 |
| 6 · Overrun、拆除和伤害 | 899 tick 后己方进入，计时从 899 清零。再次无人防守持续 900 tick 时销毁。拆除生成同队 125 Supplies 地面箱。100 枪弹伤害降 2 HP，2,500 爆炸伤害销毁，不返 Supplies。 |
| 7 · 扣除和退款 | 一项不足时三项原值保持，负成本和无效比例拒绝。同 tick 事件按调用顺序输出。151 / 11 / 31 的 50% 退款为 75 / 5 / 15。随机扣费及退款序列通过边界性质测试。 |
| 8 · 燃料维持 | 轻、中、重各 1 台产生 −6 Fuel。入账和扣除后为 44。额外维持将 Fuel 扣到 0 并置低燃料，撤销额外注册后下分钟回到 4。 |
| 9 · 资源阶段 ≤ 0.1 ms/tick | 100 人、18 节点、3,600 tick，P50 4.600 µs、P99 7.640 µs、最大 34.859 µs。两个结算 tick 为 8.570 / 34.859 µs。参考机及与 M2-008 的阶段 5 联合预算未测（H-021）。 |

- `GameMode::Frontline` 和 `Assault` 启用，`Skirmish` 和 `Sandbox` 的资源池及规则保持关闭。资源、节点和注册表参与启用模式的 `state_hash`，五点夹具的首分钟快照为 `0x9fe58dd08d955880`。现有确定性测试严格比较 3×18,000 tick，native / WASM 的逐 tick 哈希与位置均完全一致。合流 M2-011 后协议为 v12，最终数据哈希待本轮确定性门槛复核。
- 服务器可从 `zones.json` 候选点载入资源点，安装时与对应 Strongpoint ID 关联。五线候选选择、扇区地图点位和真实 Carentan 联验依赖 M2-008，未测。Engineer 建造、可命中的结构几何与工兵权限由 M2-010 接入，车辆生成和限速由 M2-015 接入。视觉待 Fable。本卡素材需求为无。

## 5v5 联网

真实回环 WebSocket 上，5 US + 5 GER 使用加速模拟时钟推进到 tick 9,000，逐客户端核对 5 次整分钟投影，共 50 次 C2 检查和 150 条己方 C4 收入事件。夹具给 US 两个 Sector、GER 一个 Sector，分别以 42 / 18 / 10 和 36 / 15 / 8 入账，避免阵营串发在同值下被漏检。

| Tick | US | GER |
|---:|---|---|
| 1,800 | 242 / 98 / 50 | 236 / 95 / 48 |
| 3,600 | 284 / 116 / 60 | 272 / 110 / 56 |
| 5,400 | 326 / 134 / 70 | 308 / 125 / 64 |
| 7,200 | 368 / 152 / 80 | 344 / 140 / 72 |
| 9,000 | 410 / 170 / 90 | 380 / 155 / 80 |

测试用裸 WS 客户端连接真实 `WsListener`，合并后复跑耗时 2.61 s。合并 M2-019 之前还用生产 `headless-client` 收集同样的五个整分钟样本，10 个客户端共 0 解码错误、0 意外断开，耗时 3.00 s。正式回归使用现有 WS 测试夹具，遵守跨 crate 依赖矩阵。真实 WASM 模块另外用 1 个 `WorldState` 和 1 个有符号 `ResourceDelta` 字节夹具验证解码，得到 242 / 98 / 50 和 −6 Fuel。

## 复现

```sh
pnpm install --frozen-lockfile
export CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1
scripts/local-ci.sh rust lint ts data determinism wasm
cargo test -p frontline-server --locked --test resources_wire
cargo bench -p frontline-sim --locked --bench resources_tick
```

`rust` 运行 workspace tests 和 `test-util` sim 套件。`resources_economy` 的易手用例使用 `test-util` 预置捕获进度，由现有据点规则在真实 tick 完成占领。微基准用计时探针只采资源系统本体，每个活人位于所有节点的检测半径外，从而测到每个节点均扫描 100 人的路径。
