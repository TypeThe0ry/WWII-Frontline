# M4-031 gateway：对局编排与匹配（matchd）

## 元数据
- 里程碑：M4
- 工作流：server
- 规模：L
- 依赖：M1-029（server-gateway 注册/心跳）, M4-012, M4-011
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §3.3; ARCHITECTURE §18.1; ARCHITECTURE §5.17; ARCHITECTURE §3.2; ARCHITECTURE §19.2; GAMEDESIGN §2.3; GAMEDESIGN §19.7

## 背景
ARCHITECTURE §3.3 一局的生命周期：
1. gateway 判断需要新局（人数不足/无空位）→ 调用某台对局主机上的 `matchd`（轻量守护，或 gateway 通过 Docker API/systemd 启动进程）执行 `server --map carentan --mode frontline --match-id <uuid>`；
2. `server` 加载 `/data` 与地图碰撞/导航，向 gateway 注册（`POST /internal/matches/register`，含公钥指纹、端口、`data_hash`）；
3. 玩家经 gateway 获取 join ticket，连 `server`；
4. 局终 → 上报战绩 → 关闭连接 → 进程退出；gateway 回收端口；
5. 崩溃：gateway **心跳 5 s 丢失** → 标记局失效，玩家回到服务器浏览器；**首发不做对局迁移/断线续局**。
容量（§18.1）：对局主机 1–2 台，每机 2 局并发（M2 实测确认），4 局 × 100 = 400 并发上限（含 bot）；对局进程 = 一局（100 人）。
本任务落地 `matchd` 守护进程与 gateway 的编排策略：**地图/模式轮换**、**bot 填充配置**、**崩溃处理**、**限流**（容量/并发上限）。模式集合：Frontline（Carentan、Foy）、Assault（Omaha、Foy、Carentan 测试配置）、Skirmish（子区，小规模 16v16–25v25，M4-012/013）。
安全：`matchd` 只接受 gateway 签名的启动指令；对局进程以非 root、资源限制（cgroup/`ulimit`）运行；API 密钥不进日志。

## 具体内容
### 要创建/修改的文件
- `crates/matchd/`（新 crate；需在 workspace 登记并过 ARCHITECTURE §4.5 依赖方向检查：只依赖 `protocol` 的 ID 类型与 `tokio`、`serde`，不依赖 `sim`）—— 进程启动/回收、端口池、日志转发、`/internal/matchd/*` 接口。
- `crates/gateway/src/orchestrator.rs` —— 需求预测（空位/排队）、轮换策略、与 `matchd` 的 RPC。
- `data/matchd/rotation.json` —— 地图/模式轮换表、各模式 `bot_fill`（目标人数、难度）、`max_concurrent_matches_per_host`、冷却（初值，`initial`）。
- `deploy/matchd.service`（systemd）、`deploy/docker-compose.matchd.yml`（开发）。
- `crates/gateway/tests/orchestrator.rs`、`tests/e2e/matchd_8games.sh`。
### 要实现的行为
1. 启动：gateway 判断需要新局 → 选主机（容量未满）→ `matchd` 分配端口 → 拉起 `server` 进程（`--map --mode --match-id --bots N`）→ 等待注册；注册超时 30 s 视为失败并重试 ≤ 2 次。
2. 轮换：按 `rotation.json` 顺序/加权选择下一局的地图与模式；连续两局同图时 Assault 攻守互换（GAMEDESIGN §3.4）；Skirmish 子区轮换；支持管理员手动指定。
3. bot 填充：把 `bot_fill` 传给 `server`（真人优先让位逻辑在 server/sim 内，不在 matchd）。
4. 崩溃：心跳 5 s 丢失 → gateway 标记 `Crashed`，列表中移除，写事件日志与指标 `match_crashed_total`；`matchd` 回收端口与残留进程；玩家客户端回到服务器浏览器（客户端行为由 gateway 返回的状态码触发，UI 已存在）。
5. 限流：`max_concurrent_matches_per_host`、全局 `max_matches`（公开测试按 H-087 批准规模）；超限时新局排队而非拒绝服务。
6. 局终：回收、统计上报；空服务器超时关闭（`idle_timeout_s`，默认 300 s【待调参】）。
### 接口与数据
- `POST /internal/matchd/start {map_id, mode, match_id, bots, seed}` → `{port, pid}`；`POST /internal/matchd/stop {match_id}`；`GET /internal/matchd/status`；鉴权：HMAC 签名头 + 内网白名单。
- 指标：`match_started_total{mode,map}`、`match_crashed_total`、`match_startup_seconds`、`matchd_ports_in_use`。

## 不做什么
- 不做对局迁移/断线续局（ARCHITECTURE §3.3）；不做跨区域合服（ARCHITECTURE §1.4）。
- 不做生产部署与证书（M4-033）；不做告警规则（M4-034）。
- 不做玩家社交匹配（好友/段位）。
- 不在 `matchd` 中实现任何游戏规则。

## 验收标准
1. **无人工干预连续跑 8 局**：脚本 `tests/e2e/matchd_8games.sh` 驱动 `headless-client` + bot，覆盖 ≥ 3 张地图与 3 种模式，8 局全部正常开局、结束、回收；失败 0（报告列出每局 map/mode/时长）。
2. **进程崩溃后玩家回到列表 ≤ 10 s**：故障注入（`kill -9` server 进程），从崩溃到 gateway 列表移除并客户端收到失效通知 ≤ 10 s（5 s 心跳判定 + 5 s 传播），测 10 次 P95 ≤ 10 s。
3. 启动耗时：`match_startup_seconds`（请求 → 注册完成）P95 ≤ 20 s（含 Omaha/Foy 地图加载，记录；超限写 PLAN_ISSUES）。
4. 端口回收：运行 20 局后无泄漏端口/僵尸进程（`ss -lntu`/`ps` 检查，数量回到基线）。
5. 限流：把 `max_concurrent_matches_per_host` 设 2，请求第 3 局时排队且不影响运行中局（单测/e2e）。
6. 安全：未签名/过期签名的 `start` 请求被拒（4 个负向单测）；`matchd` 子进程以非 root 运行（e2e 检查 uid）。
7. 轮换：随机种子固定时 100 局序列与预期表一致；连续同图 Assault 攻守互换（单测）。

## 测试要求
- 单元测试：轮换器、端口池、容量与排队、HMAC 校验、心跳状态机。
- 集成/无头测试：`matchd_8games.sh`；崩溃注入；netem 不适用。
- 性能基准：`matchd` 自身 CPU/内存 ≤ 1% 单核 / ≤ 50 MB（`/metrics`）。
- 回归：`rotation.json` 入 `DATA_HASH`？不入（运维数据）；单独 schema 校验。

## 性能预算
- 单机 2 局并发的资源（§18.1/§6.10：单局 ≈ 1.0–1.5 核等效）不得因 `matchd` 增加；`matchd` ≤ 50 MB 内存。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / e2e 脚本）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `matchd` 是 ARCHITECTURE §3.3/§4.1 的新增进程/crate（原文写"轻量守护，或简化为 gateway 通过 Docker API/systemd 启动进程"）：若新增 crate 违反 §4.5 依赖方向，退而采用"gateway 直接调用 systemd/Docker"并写 PLAN_ISSUES。
- INDEX 规模 L：可拆 `M4-031b`（轮换/限流）；核心（启动/回收/崩溃）优先。
- 规模/成本批准由 H-087 决定；容量上限数值等待 M2 压测数据。
