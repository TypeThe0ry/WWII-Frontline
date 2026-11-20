# M2-017 计分与经验（B6-scoring，2026-11-20）

实测均在 EPYC 7402 共享工作站完成（PI-109，非参考机）。此项不改变 M1 退出验收。

| 任务卡验收 | 本地结果 |
| --- | --- |
| 1. 数据驱动得分 | `score_table` 逐条覆盖 47 项，含 8 项禁用的 M3 条目；禁用条目不发分。数据树 `79` 项通过校验。 |
| 2. 防刷 | 同一对手滚动 60 秒第 4 次 Combat 为 0；据点 90 秒内重复占领 Offense 为 0；滚动 60 秒前线/普通 Garrison、Outpost、Ammo Crate 分别限 20/15/10/30 点；另验 AFK、120 秒重复救起与 10 秒助攻边界。 |
| 3. 等级 | 本职等级 L2/L5/L10 阈值 250/2,500/11,250 XP；账号 L10/L50/L100 阈值 10,800/137,200/394,015 XP，来自整数公式。验证阈值前后 1 XP。 |
| 4. 经验 | 1,000 分完整参赛胜/负/平 = 1,200/1,150/1,170 XP。5 分钟、70% 参与率分别测试前一 tick 与阈值 tick。 |
| 5. 摘要 | 字段清单及 JSON 往返断言；队/小队排行、各兵种得分与经验、分项及支援/载具计数。旧 spool 仍可重放。资源曲线及缺失前置的统计只预留接口，未声称有实局样本。 |
| 6. 计分预算 | 见本文性能实测，参考机未测（H-021）。 |

## 范围与口径

- M1 已接入击杀归属、被击倒/击毙、救起与友军治疗、压制助攻、据点进度和易手、Garrison 建造/完成/队友部署、Ammo Crate 使用。步兵 K/D/A 统计的击杀包括有效倒地或直接 KIA，补杀不加第二个 K，死亡数只计 KIA；防刷门槛屏蔽 Combat 点数，不改变统计。命中为命中敌方步兵的投射物事件数，显示比例封顶 100%。
- Garrison 的前线标签按放置时所在扇区算，反复分分别按玩家/理由限额；计分显示不向对方泄露事件。匿名局内会话分开保留，槽位复用不继承旧分或已放置物的奖励。停止前的绝对 tick 数与 Live/Overtime 时长分开保存。停服若无自然赛果，无胜负 XP；线上的 `rows` 继续兼容，新增 `score_summary`。
- M2-008/010/011/015/016 生产者未落地；Assault 分段、资源节点、火炮/工事、Outpost、修理、载具伤害/击毁与车组归属仍缺完整实局输入。资源曲线等待 M2-009，AFK 由 M2-018 驱动（没有组件时默认 false）。补给箱消耗归属、标记击杀和载具里程未接入实时生产者。字段与计分接口在，实局未测。
- M3 保留禁用项。未接 M2-038/M2-042 视觉、战绩持久化与跨局 XP、兵种解锁；视觉待 Fable 设计。无新增资产。

## 加速集成对账

`cargo test --release --locked -p frontline-server --lib score_audit_5v5_plus_bots_36000_ticks -- --ignored --nocapture`：合成场地，5v5 脚本员加 2 个真实服务端 bot，预热切换后推进 36,000 tick（20 分钟 sim 时间，不是 20 分钟网络墙钟）。服务端实际弹道/伤害路径产生 145 条 `ScoreDelta`，净分 1,267；每人四类及总分与 `MatchSummary` 精确一致。

## 计分微基准

`CARGO_BUILD_JOBS=8 taskset -c 36 cargo bench --locked -p frontline-sim --bench score_tick`：100 玩家、50 个混合事件/tick、3 个据点的周期结算；预热 600 tick 后测 36,000 tick。计分引擎本体（不含 ECS/网络）壁钟 P50/P95/P99/max 为 **4.750/6.970/13.930/626.574 µs**，合计 **6,947,322** 分与账本一致。P99 低于 50 µs；单次峰值超出，不把共享宿主的 wall-time 尾部当作纯 CPU，未验证参考机逐 tick 上界。

变基到 `e785de3` 后同命令复测，P50/P95/P99/max **5.660/6.470/8.510/290.177 µs**，仍精确对账 **6,947,322** 分；同样不作参考机通过判定。

## 确定性和兼容性

协议 v4，`DATA_HASH=0xc13bfe523d8e0c80`；相同规则由 native/server、WASM/client 的握手共享。严格对照 3 个种子各 18,000 tick，native/WASM 哈希全部一致、位置差 0 m；另有 Live 据点得分回放与计分摘要对账。该 M0 严格矩阵不等同于 native/WASM 完整 M2 比赛的跨平台回放。

## 生产开赛与结算回归

- 首次 Carentan WS 探针（10 客户端、2 bot、标称 1,210 s）生成 36,474 个服务端 tick，10/10 HQ 部署，0 断线、0 解码错误，但 `score_summary` 时长为 0、玩家为空。它只证明网络跑通：加载战术地图后没有从 Idle 启动 Warmup，因此不算计分验收。
- 生产构造器现在对有据点的非 Sandbox 战术图启动 Warmup。`tactical_warmup_counts_connections_not_bots` 覆盖默认 1,800 tick 热身、bot 排除、真人离开与替换、第二个真人会话入场提前开赛、首个计分 tick，以及 Sandbox 保持 Idle。加速审计也改走同一构造器，按 `/data` 原时限自然结束；新基线重跑仍为 36,000 计分 tick / 145 增量 / 1,267 净分。
- C4 回归验证同 tick 同队槽位复用：旧队友先收 `LEFT`，再收新行；新连接只收到新名单，对方不收到该槽位。网关管理统计只沿用旧格式的聚合，不将新的仿真槽位归到票据账号。真实进程回归确认完整摘要落在 `scoring-v1`，新进程能重放该摘要和旧格式、回投内容不变，损坏的文件保留待处置。自然比赛在 sim 中结算；提交给网关仍发生在停服，无新增对局轮转。
- 由于发现只筛协议版本，部署新 `DATA_HASH` 时客户端/WASM 与比赛服须同批，旧哈希对局须从给新客户端的发现列表撤下；旧进程回滚时不要把新版 spool 挪回根目录。预发布与受信任证书未测（H-017）。

## Carentan WS 长跑复验

工作站同机私有 netns loopback，未加 netem；Release 服务端/客户端分别绑定 CPU `24,29,41` / `6,10`，没有独占核，不连接网关。用 [B4-D1](m1-carentan-wt-b4-d1.md) 的临时 Carentan M1 图（380 块原始碰撞、L3a/L3b 巡逻点）。此长跑使用 Warmup 修复后二进制，早于带宽调节基线的变基；变基后的代码用上述加速审计及本地门槛另行复验。

```sh
CARGO_BUILD_JOBS=8 cargo build --release --locked -p frontline-server -p frontline-headless-client
LOADTEST_SERVER_CPUS=24,29,41 LOADTEST_CLIENT_CPUS=6,10 \
  FRONTLINE_DATA_DIR="$D/data" TMPDIR="$D/soak" \
  unshare -n bash -euc 'ip link set lo up; unset GATEWAY_URL FRONTLINE_DEV_NO_GATEWAY; \
    exec scripts/loadtest-m0.sh --transport ws --map carentan/m1 --mode assault --deploy-hq \
      --count 10 --bots 2 --duration 1225 --seed 1 --no-build --sample-interval 10 --out "$TMPDIR/summary.json"'
```

| 检查项 | 结果 |
| --- | --- |
| 会话 | 10/10 接入并完成 HQ 部署，US/GER 各 5 个 headless，另有 2 bot。每连接 1,225.032–1,225.037 s，0 断线，0 解码错误。 |
| 服务端 | 36,773 tick，运行 1,226.167 s；tick P50/P99/max 1.463/4.335/135.233 ms，2 次超时，最长连续 1；脚本退出 0。 |
| 计分 | 36,000 个 Live tick 后按时限自然结算（`time_limit_lead`，US 胜）；12 条参与者记录的四类之和全部等于各自总分，US/GER 净分 1,032/528，合计 **1,560**。 |

本轮 headless 未逐条记录 C4 分数流，不声称网络路径逐条对账；逐条对账见加速审计，C4 单独验证编码、队伍隔离和槽位复用。不是参考机测试、真人评分校准或生产网关部署。
