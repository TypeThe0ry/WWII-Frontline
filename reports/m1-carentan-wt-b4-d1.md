# B4-D1：Carentan M1 服务端联测（2026-11-19）

## 工况与复现

- EPYC 7402 共享工作站（PI-109，**不是 RTX 3060 参考机**），同机私有 `unshare -n` loopback，未加 netem。Release 服务端和 headless，`taskset` 分别限定在 CPU `24,29,41` 与 `6,10`；运行时逐线程核对了 `/proc/*/task/*/status`。未独占核，其他宿主任务仍可运行在这些核上。
- `carentan/m1`、Assault、20 个真实 WebTransport headless 连接 + 12 个固定服务端 bot（非 bot-fill），用 C0 请求 HQ 出生。无网关、真人、满编载具或 M1-064 正式素材。已加载 **380** 块 Carentan FCOL；联测二进制使用 B4-D2 (#150) 合入前的建筑灰盒碰撞，headless 预测仍只使用地形。合成客户端在 L3a/L3b 间巡逻并按现有生成器运动/开火，不是脚本化瞄准命中率试验；这轮没有提交 C0 标记或 `FireCmd` 起点提示。
- 地图烘焙基于仓库的 `.r16` 与 M1 切片矢量数据。只在暂存副本中解开 `c_*.fcol`（zlib）得到服务器/客户端现有 loader 接受的原始 `.bin`，并以既有 L3a/L3b 坐标补 headless 巡逻点；不提交生成文件或修改正式地图。脚本加载 `carentan/m1` 的父图 `meta.json`，使用切片 `zones.json`、`spawns.json`、navmesh 和现有地形。

```sh
D=$(mktemp -d)
mkdir -p "$D/data/maps" "$D/reports" "$D/soak"
cp -a data/maps/carentan "$D/data/maps/"
M="$D/data/maps/carentan"
PYTHONPATH=tools/map-pipeline python3 -m map_pipeline chunks-m1 \
  --repo "$D" --out "$M" --dest "$M/m1/chunks" --workers 6
python3 - "$M/m1" <<'PY'
import json, sys, zlib
from pathlib import Path
m = Path(sys.argv[1])
for p in (m / 'chunks').glob('c_*.fcol'):
    raw = zlib.decompress(p.read_bytes())
    assert raw[:4] == b'FCOL' and len(raw) == 16 + 65 * 65 * 2 + 64 * 64, p
    p.with_suffix('.bin').write_bytes(raw)
spawns = json.loads((m / 'spawns.json').read_text())
points = [p for line in json.loads((m / 'zones.json').read_text())['lines'] for p in line['points']]
spawns['patrol_points'] = [{'pos': p['pos']} for p in points if p['id'] in ('L3a', 'L3b')]
(m / 'spawns.json').write_text(json.dumps(spawns))
PY
cargo build --locked --release -p frontline-server -p frontline-headless-client
LOADTEST_SERVER_CPUS=24,29,41 LOADTEST_CLIENT_CPUS=6,10 TMPDIR="$D/soak" \
  FRONTLINE_DATA_DIR="$D/data" FRONTLINE_MATCH_ID=0a9003ec-f0b5-4ac1-a09a-81d05072d401 \
  FRONTLINE_TICK_TRACE=1 FRONTLINE_STEP_TRACE=1 FRONTLINE_LOG_RATE_LIMIT=10000 \
  unshare -n bash -c 'ip link set lo up; unset GATEWAY_URL FRONTLINE_DEV_NO_GATEWAY; \
    exec scripts/loadtest-m0.sh --transport wt --map carentan/m1 --mode assault --deploy-hq \
    --count 20 --bots 12 --duration 1800 --seed 1 --no-build --out "$TMPDIR/m1-wt.json"'
```

## 30 分钟 WT 结果

| 项 | 实测 |
| --- | --- |
| 在线与收包 | 20/20 WT 加入、20/20 HQ 部署、每客户端在线 1800.031–1800.042 s；预热后至标称结束前 178 次 scrape 均为 US 10 + GER 10 + 12 bot。0 加入失败 / 0 断线 / 0 解码错 / 0 缺基线。 |
| 服务端 tick | 54,104 tick，P50 / P99 / max **3.519 / 9.810 / 323.875 ms**；94 次超时、216 个跳过的定时槽。最长连续超时 **2**（共 3 对），超过允许的 1，脚本退出 **1**，**不是 tick 验收 PASS**。AOI 编码 P99 在 8 ms 桶。 |
| 合成客户端带宽 | 下行 P50/P99 **24.904 / 34.144 kbps**（最差客户端 P50/P99 26.176/35.512），上行 P50/P99 **20.448 / 21.280 kbps**，按 headless 的线速估算。RSS 末段 593 s 增长 0.395%。 |
| C2/AOI | **717,121/717,121** 个快照与同 tick 服务器位姿配对。客户端解码后独立审计 **2,682,547** 条敌方记录，其中 5,913 条超过 105.6 m 近界；0 条越界，服务器 R3 越界计数 0。审计检查 R3 距离/视锥边界，不从网络包反推数值遮挡。 |
| C3 | 20/20 收到队伍数据，合计 **35,856** 包（每人 1,792–1,793 包）；未发送手工标记，标记路径另见真 WS/WT 集成测试。 |
| 正常合成输入对照 | `session_kicked_total=0`、136 次输入停顿、移动/角速度异动计数各 0、272 次射速拒绝（不能据此推断人类误报）。有普通重复、未来 tick 和 ring overwrite，原始计数见 JSON。 |

### `tick_slow` / `tick_overrun` 归因

- 日志包含 288 条 `tick_slow`、94 条 `tick_overrun`，其中 175 tick >20 ms。按整 tick `schedstat` 的严格多数分类：**321** 次由运行队列等待主导（71 次超时），**56** 次有未计入 CPU/runq 的多数时间（23 次超时），**5** 次混合，**0** 次由 CPU 执行主导。阶段耗时都是墙钟，不能直接当作计算耗时。
- 三对连续超时位于 tick 60/61、33727/33728、51155/51156；全部由 runq 等待主导。示例 tick 33728：墙钟 102.121 ms，CPU 3.014 ms，runq 99.014 ms。最长 tick 11923 的快照阶段 321.099 ms、CPU 3.950 ms、runq 0.486 ms；该 tick 未取得现场等待栈，不能断言其根因。
- 在后段用另一核采样服务器 sim 线程 `/proc/.../{wchan,stack}`，按日志时间窗匹配到 **13** 个慢 tick（8 次 slow、5 次超时）阻塞在 `ext4`/`jbd2` 的文件写入路径。stdout 与 stderr 均写到同一个 `server.log`，快照路径同步写 `FL_AOI_TRACE` 的位姿记录；例如 tick 43716 墙钟 **99.855 ms**、快照 **97.445 ms**、CPU **3.152 ms**、runq **0.834 ms**，窗内 11 个 `do_get_write_access` 采样。它验证了这批快照尖峰中的日志 I/O 等待，不证明未采样的 319.439 ms 余量有同一根因。
- 汇总和前五慢 tick：`reports/loadtest-1119/carentan-wt-30min-b4-d1.json`。原始 traces、`affinity.txt`、二进制 SHA-256、Prometheus scrape 和内核等待栈保留在工作站 `/data/scratch/batch/b4/D1/`。正式复测应分开做 trace 审计与不向共享盘同步写 trace 的计时对照，并在独占 CPU 或参考机上重复；Carentan 30 分钟 netem、满编载具、正式素材尚未验收。

## 其它服务端验证

- **C3**：同一局真 WS/WT 监听器的集成测试，WS 队长经 C0 放标记，WS 自己和 WT 同队同小队成员收到相同 C3 扩展；WT 对手只收到自己阵营 roster，未声明 capability 的同小队 WS 连接仍收到原成员格式。另有权限、跨队/跨小队隔离、TTL/5 m 量化、cap、编码兼容及离线回收用例。手工标记不是 Spotter 侦察结果，浏览器 HUD 尚未接入。
- **遮挡与校验**：AOI 的现有缓存以单块实墙完整遮住目标盒作为建筑剔除条件；掩体边缘露出和友军处理有回归。延迟补偿按沿射线的实墙穿越厚度及遮挡持续窗口阻止历史命中，薄墙/斜射、当前与历史视线均有对照。另将真实地图的既有壁面回归指向本次烘焙目录，确认 380 块地形、11,600 块灰盒和石材/混凝土壁面实际命中。合入 B4-D2 (#150) 室内数据后，独立复跑同一壁面回归：380 块地形、11,740 块灰盒、石材/混凝土壁面命中；未将该短测当作新 30 分钟联测。`FireCmd` 起点提示按真实同序号发射统计；只诊断超 0.5 m 的差异，不改变权威枪口、命中或断开决定。当前临时编码只覆盖 M0 事件坐标域，Carentan 枪口多数不可表示；soak 不发送此字段，0 不能作为网络起点提示验收。
- **WT 延迟补偿对照**：`crates/server/tests/lagcomp_transport.rs` 使用独立平地夹具和真实 C1/C2 链路，每局向 C2 所见的横移目标发一枪。无损回环开启回溯 **4/4** 次 `HitConfirm`；私有回环 netem 75 ms/向、C0 实测 RTT **150.23–150.99 ms**，开启回溯 **4/4**、去掉回溯请求 **0/4**，服务端命中计数与去重后 C2 事件相等。原始记录 `D1/lagcomp-netem.log`。这是小样本路径回归，不是 Carentan 命中率验收。
- **网关**：本机 release 200 并发签票原夹具 200/200 唯一票，HTTP P95 **53.269 ms**；三轮加 `/metrics` 采样的兼容夹具均为 200/200，P95 **56.865 / 58.365 / 61.166 ms**，约 **3,405 / 3,279 / 3,132 张/s**。SQLite pool acquire 平均 **29.343 / 31.132 / 30.682 ms**，事务平均 **0.168 / 0.165 / 0.174 ms**（均 200 样本/轮）；P95 桶分别是 **(40, 50] / (50, 75] / (50, 75] ms** 与 **(0.1, 0.5] ms**（三轮）。此工况的主要耗时在等待连接，50 ms 线未达；未更改并发写入模型，未声称提速。进程级 gateway+WS 验证注册与签票入场，重放/过期/篡改各拒绝 1 次，最后观测到的心跳后 **5.017 s** 下线。兼容性失败不占用签票游标；验签后的身份元数据随会话保存与清理，`gateway_player` 非权威槽位，`is_admin` 不授予权限。仍缺重连令牌、完整战绩/阶段上报和预发布账号验证（M1-067）。
