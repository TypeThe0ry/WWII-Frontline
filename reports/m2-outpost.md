# M2-011 Outpost、维修站与反坦克炮验收

2026-11-20 · PI-109 工作站（非参考机）· 30 Hz · 状态 `partial`。

| 卡片 | 工作站结果 |
|---|---|
| 1 · Outpost 放置 | 7 个规则拒绝分支与合法放置覆盖。第 180 tick 完成，移动或失血中断。销毁后第 3,599 tick 拒绝，第 3,600 / 3,601 tick 允许。 |
| 2 · Outpost 生命周期 | 3 名队员完成 6 轮共 18 次出生，第 19 次拒绝。单次等待 360 tick，保护 90 tick。第 18,000 tick 过期。25 m 内封锁，15 m 内无人守卫累计 450 tick 后 Overrun，友军进入后计时归零。 |
| 3 · 小队权限 | 无头测试覆盖队外拒绝、队长阵亡保留、队长移交。C4 只对当前小队下发 Outpost，离队发移除。 |
| 4 · 维修站 | 第 450 tick 建成，库存 200 Supplies。8 m 处的 1000/1600 HP 车辆首秒恢复 19.99878 HP。耗尽夹具共恢复 1,000 HP，毁坏引擎保持 0 HP。弹药每 60 tick 补 1 发并扣 4 Supplies，Support 交箱补 100 Supplies。 |
| 5 · AT 炮弹 | 12 组装甲向量与 M1-023 共享求解器。57 mm P100 92 mm / 每 100 m 3%，Pak 40 为 96 mm / 2.2%。模拟 Sherman 被击毁用 7 发。 |
| 6 · AT 操作 | US 装填 90 / 126 tick，AP/HE 切换 30 tick，水平限制 ±40°，Engineer 重新定向 90 tick。`SimEvent::MuzzleFlash` 带 90 tick 画面时长、150 tick 暴露、25 m 误差。 |
| 7 · 建造限制 | 非 Anti-Tank 建炮被拒，队内第 4 门及同区第 2 门被拒。建炮消耗 200 Supplies、600 tick。维修站队内上限 3。 |
| 8 · CPU | 3 Outposts、6 小队、3 站、3 炮的规则阶段 Criterion 区间为 1.8885–2.2316 µs/tick。阶段 4 的 399 发枪弹 + 1 发 AT 炮弹扫描为 1.5697–1.8878 ms/tick。两组均在工作站的合成夹具低于预算。参考机及真实地图阶段 4 满载未测。 |

## 接口及未测项

- 协议 v12。C0 复用 `PlacementRequest` 及放置结果，C4 新增 `StructureState` (`0x10`) 并沿用 `SpawnPointState`。Outpost 预留 ID 为 `0x8000..=0xfffe`。临时放置键为 `9`、`0`、`]`。炮位入座复用载具交互键，建造协作、重新定向和弹种等动作有 C0 接口。正式菜单及炮口特效待 Fable 设计，真人浏览器整局未测。
- M2-006、M2-008、M2-010 的共用实现尚未合入。临时使用 M1 装具与最小职业数据、最近据点作为 Sector 归属、地面补给箱及设施本地建造上限。正式 Sector、多建筑类型共享总额、导航连通性和车辆补给源待依赖合入后联验。
- 初始游戏数值由数据文件提供。Outpost 用 Rust 构建时类型校验，维修站和 AT 炮由 sim 类型解析及测试检查。设施数据参与 `DATA_HASH = 0xad4f99353853fab8`，Node 检查器对这些文件只计入哈希。
- 无新增外部素材。无头夹具使用程序化平坦碰撞地形与已有 Sherman 数据。

## 复现

```sh
CARGO_BUILD_JOBS=8 cargo test -p frontline-sim --features test-util --locked --test outpost -- --nocapture
CARGO_BUILD_JOBS=8 cargo test -p frontline-sim --features test-util --locked repair_station_heal -- --nocapture
CARGO_BUILD_JOBS=8 cargo bench -p frontline-sim --bench outpost_repair_atgun --features test-util --locked
CARGO_BUILD_JOBS=8 scripts/local-ci.sh lint rust determinism wasm ts data
```
