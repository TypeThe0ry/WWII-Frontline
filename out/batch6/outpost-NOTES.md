# B6-outpost · M2-011

- 实现 Officer 小队 Outpost、Engineer 维修站和 Anti-Tank 炮位。联通设施建造、出生、补给、弹道/穿深、C0/C4、WASM 和客户端逻辑。放置临时键为 `9`、`0`、`]`。正式视觉、建筑菜单和炮口表现待 Fable 设计。任务卡无新增素材需求。
- 与同时合入的 M2-018 协议语义合并后，设施 C4 使用 `0x10`，握手协议为 v12，数据哈希 `0xad4f99353853fab8`。
- 任务状态 `partial`。参考机、真人浏览器整局和真实地图阶段 4 满载未测（H-021）。正式 Sector、导航连通性、车辆补给源、共享建筑总额和完整角色装具待 M2-006/008/010。设施文件在 Node 验证器中计作 hash-only，另有 sim 类型解析与单测。

## 命令与数字

全部工作站结果按 PI-109 标注为非参考机。

- `CARGO_BUILD_JOBS=8 cargo test -p frontline-sim --features test-util --locked --test outpost -- --nocapture`。3 人合计出生 18 次，第 19 次被拒。Outpost 放置 180 tick、等候 360 tick、保护 90 tick、Overrun 450 tick。建炮后 7 发击毁夹具 Sherman。
- `CARGO_BUILD_JOBS=8 cargo test -p frontline-sim --features test-util --locked repair_station_heal -- --nocapture`。首秒修复 19.99878 HP，200 Supplies 修复总额 1,000 HP。
- `CARGO_BUILD_JOBS=8 cargo bench -p frontline-sim --bench outpost_repair_atgun --features test-util --locked`。规则阶段 1.8885–2.2316 µs/tick，合成 399 发枪弹和 1 发炮弹扫描 1.5697–1.8878 ms/tick。
- `CARGO_BUILD_JOBS=8 scripts/local-ci.sh lint rust determinism wasm ts data`。

卡片逐项证据与偏差见 [M2-011 验收](../../reports/m2-outpost.md) 和 PI-B6-outpost-1。
