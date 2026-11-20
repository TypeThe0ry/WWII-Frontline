# WWII-Frontline 双线合流评估

**结论：不要直接把 `work/playable` 合进 M1。** 保留里程碑线的网关、安全和验收基线，在隔离的集成分支分批引入可玩线功能。先确定数据和联机契约，再接对局和客户端；两种场景都回归后才考虑切换。本文只分析，不实际合流。

## 1. 快照与规模（2026-11-20）

| 引用 | 固定提交 | 自共同祖先独有提交 | 自共同祖先改动路径 | 增行 / 删行 |
|---|---|---:|---:|---:|
| `origin/claude/serene-wright-lc7k1n`（MS，已 fetch） | `0e5b41f907ecc736f1abe5cee730e2c6fb0a3f2a` | 163 | 1,091 | +192,526 / −1,464 |
| `work/playable`（P，本地） | `196a3de9185a0e377c83ccddc6befb1d01c28817` | 211 | 1,837 | +98,853 / −1,401 |
| 唯一 `merge-base --all` | `873e2d700f75e0116224ccd1b79d91c17665b729`（2026-11-17，M1-002 WIP） | — | — | — |

`rev-list --left-right --count MS...P` 包含合并提交；另用 `--no-merges --cherry-pick` 的结果为 **162 / 164**，也不能当成待移植的功能数。两侧从祖先的路径并集 **2,758**，交集 **170**（`diff --no-renames`）；行数包含 JSON、报告、生成物等，不等于代码量或工时。

文档落地 rebase 时 MS 前移到 `f4f18a7`（仅既有 E1-10 文档跟进）。与固定 P 重算，独有提交变为 **164 / 211**，**131 条冲突路径完全不变**。其余详细统计仍以表中固定快照为准。

## 2. 两线各自要保住的功能

| 领域 | MS 基线应保留 | P 能力应引入 |
|---|---|---|
| 入局与运维 | `crates/gateway` 真实游客鉴权、服务器列表、注册/心跳/结果、签发票据；`crates/server/src/gateway_link` 验签与席位链。`dev.sh`、`deploy/dev/` 全栈启动；`deploy/staging/runbook.md` 是模板，不是已上线。签票有本机压测（`reports/m1-gateway-ticket-b6.md`）。 | `scripts/dev/{play,status,stability}.sh`、P4 部署/HUD、同页 WS 断线续战；只读状态查询面向单台比赛服，不等于网关列表或票据，P 的 `crates/gateway/src/main.rs` 仍为空壳。不要用试玩直连取代正式入场。 |
| 权威对局 | `data/maps/carentan/` 的 M1 Carentan/Assault；服务端 AOI/带宽、时延补偿、反作弊、倒地压制、补给驻点和坦克验收夹具。不把 WS、WT、短场景和真人证据混算。 | `data/maps/carentan1/` 的五段有序战线（Capture Lock、后方出生、区段归属/Garrison、bot 任务重算）；十兵种、双座轻车、工兵工事，含场景、HUD/战术地图和权威快照，见 `out/{frontline-mode,content,ux}/NOTES.md`。P1 Frontline Lite 与 Carentan 有序战线是不同入口/验收。 |
| 地图、资产与客户端 | Carentan M1 地形与分包、C3 流式、FPAK/OPFS、CSM/TAA、无样式 UI 和资产门禁（`assets/manifests/`、`reports/m1-fpak-medium.md`）；Medium 是本地构建结果，不代表每项素材都已接入实局。 | 试玩 UI 与内容接线、士兵批绘、Willys/Kübel 和工兵公开模型（`client/src/render/{actors,vehicles}/`、`reports/b6-jeepeng-models.md`）。这些模型是可再生占位，不是采购资产或视觉验收。保持免费资产台账；`INTERNAL` 不进入公开产物。 |
| 可复用证据 | `scripts/local-ci.sh` 及 `reports/m1-acceptance.md`。时点上仍为 **3 PASS / 11 partial / 3 未测**，M1 未关闭。已有例证：本机真网关部署计时、WS/WT 双页 Garrison、Carentan WT 无损 30 分钟两轮；不是预发布 HTTPS、netem 完整覆盖或真人验收。 | `out/stability/NOTES.md` 的固定 v10 构建记录：`release-05` 的 P1，本机 WS/Chromium SwiftShader，3×30 分钟，每轮 8 个自动化浏览器＋8 bot；含续连、坦克/驻点分工。属于该构建的记录，非本次复跑；其 `release-05/` 原始样本不在固定 tip 的 Git 树。Carentan 有序战线有独立 bot 平衡与浏览器冒烟，不能替代 M1 验收。 |

两线共用许多概念和文件（Carentan、部署、坦克、Garrison、WS/WT、音效等），但实现、地图 ID、场景及取证口径不是等价物。工作站记录均按 **PI-109 非参考机** 使用。

## 3. Git 试算与完整冲突清单

Git 2.53.0，方向 **MS ← P**。以下命令只写模拟树对象及标准输出，不改 worktree、索引或分支，不创建合并提交。默认配置与显式开启重命名、提高阈值的结果逐字节相同（机器格式取 `-z`，应按 NUL 拆分，勿把消息数当路径数）。

```sh
MS=0e5b41f907ecc736f1abe5cee730e2c6fb0a3f2a
P=196a3de9185a0e377c83ccddc6befb1d01c28817
git merge-base --all "$MS" "$P"
git rev-list --left-right --count "$MS...$P"
git -c merge.renames=true -c merge.renameLimit=10000 \
  merge-tree --write-tree --name-only --messages -z "$MS" "$P"
```

退出 **1（有冲突）**，模拟树 `4d6888a025607d51ba812e986d56fc5a3891d3ae`。**131 个冲突路径**：130 个文本文件含 **412** 个 `<<<<<<< ` 起始标记，另 1 个二进制 (`snapshot_0.bin`，Git 同时报 binary 和 contents，只计一次)。最大几个：`sim/src/schedule.rs` 20、`sim/src/projectile.rs` 17、`server/src/sim_thread.rs` 16、`protocol/src/msg/c0.rs`/`bots/src/soldier.rs` 各 15。**另有 39 个双改路径被自动合并**，必须单独做语义审查，尤其 `Cargo.lock`、`pnpm-lock.yaml`、`.github/workflows/lint.yml`、`client/src/net/wasm/layout.ts`、`crates/protocol/src/msg/input.rs`、`crates/server/src/session.rs`、`crates/sim/src/step.rs`，以及武器/地图 schema。整体带入 P 也可能让新增的资产、生成物和部署约定同时生效，即使 Git 不报冲突。

<details>
<summary>根目录：3 个</summary>

```text
DEVELOPING.md
LICENSES.md
deny.toml
```

</details>

<details>
<summary>client：40 个</summary>

```text
client/e2e/actors.spec.ts
client/package.json
client/playwright.config.ts
client/scripts/sync-map.mjs
client/src/actors-boot.ts
client/src/audio/audio.test.ts
client/src/game/event-bus.ts
client/src/game/events.test.ts
client/src/game/events.ts
client/src/game/game-loop.ts
client/src/game/net-game.ts
client/src/game/placement.test.ts
client/src/game/placement.ts
client/src/game/snapshot-apply.ts
client/src/input/keymap.ts
client/src/input/pointer-lock.ts
client/src/input/sampler.ts
client/src/main.ts
client/src/net/codec.test.ts
client/src/net/codec.ts
client/src/net/connect.ts
client/src/net/wasm/types.ts
client/src/node-test-shims.d.ts
client/src/render/actors/actor-data.ts
client/src/render/actors/actor-manager.ts
client/src/render/actors/placeholder-character.ts
client/src/render/assets/index.ts
client/src/render/fx/first-person-camera.ts
client/src/render/fx/impact-dust.ts
client/src/render/fx/index.ts
client/src/render/fx/tracer.ts
client/src/render/lighting/lighting.test.ts
client/src/render/terrain/terrain-material.glsl.ts
client/src/render/terrain/terrain-material.wgsl.ts
client/src/render/terrain/terrain-scene.ts
client/src/render/terrain/terrain.ts
client/src/render/vegetation/scatter.ts
client/src/render/vegetation/vegetation-scene.ts
client/src/render/vegetation/vegetation.ts
client/vite.config.ts
```

</details>

<details>
<summary>crates/bots：5 个</summary>

```text
crates/bots/src/ctx.rs
crates/bots/src/soldier.rs
crates/bots/tests/common/mod.rs
crates/bots/tests/golden.rs
crates/bots/tests/nav.rs
```

</details>

<details>
<summary>crates/protocol：14 个</summary>

```text
crates/protocol/fuzz/Cargo.toml
crates/protocol/src/channel.rs
crates/protocol/src/lib.rs
crates/protocol/src/msg/c0.rs
crates/protocol/src/msg/c4.rs
crates/protocol/src/msg/mod.rs
crates/protocol/src/snapshot/decoder.rs
crates/protocol/src/snapshot/entity.rs
crates/protocol/src/snapshot/events.rs
crates/protocol/src/snapshot/mod.rs
crates/protocol/src/snapshot/self_state.rs
crates/protocol/tests/golden/hello.hex
crates/protocol/tests/golden/snapshot_key.hex
crates/protocol/tests/snapshot_prop.rs
```

</details>

<details>
<summary>crates/server：15 个</summary>

```text
crates/server/Cargo.toml
crates/server/src/bot_host.rs
crates/server/src/config.rs
crates/server/src/main.rs
crates/server/src/net/conn.rs
crates/server/src/net/transport.rs
crates/server/src/sim_bridge.rs
crates/server/src/sim_thread.rs
crates/server/src/snapshot/priority.rs
crates/server/src/snapshot/scheduler.rs
crates/server/src/snapshot/self_state.rs
crates/server/src/validate.rs
crates/server/src/world_loader.rs
crates/server/tests/aoi_priority.rs
crates/server/tests/bots_validation.rs
```

</details>

<details>
<summary>crates/sim-wasm：8 个</summary>

```text
crates/sim-wasm/src/messages.rs
crates/sim-wasm/src/predictor.rs
crates/sim-wasm/src/snapshot.rs
crates/sim-wasm/tests/determinism_native.rs
crates/sim-wasm/tests/golden/layout.json
crates/sim-wasm/tests/golden/native.json
crates/sim-wasm/tests/golden/snapshot_0.bin
crates/sim-wasm/tests/node-smoke.mjs
```

</details>

<details>
<summary>crates/sim：15 个</summary>

```text
crates/sim/build.rs
crates/sim/src/collision/heightfield.rs
crates/sim/src/collision/mod.rs
crates/sim/src/data.rs
crates/sim/src/data_defs.rs
crates/sim/src/health.rs
crates/sim/src/lib.rs
crates/sim/src/movement/mod.rs
crates/sim/src/projectile.rs
crates/sim/src/schedule.rs
crates/sim/src/weapon/mod.rs
crates/sim/src/world.rs
crates/sim/tests/damage_death.rs
crates/sim/tests/determinism_basic.rs
crates/sim/tests/weapon_duel.rs
```

</details>

<details>
<summary>data：19 个</summary>

```text
data/loadouts/ger_medic_1.json
data/loadouts/ger_officer_1.json
data/loadouts/ger_rifleman_1.json
data/loadouts/ger_support_1.json
data/loadouts/us_medic_1.json
data/loadouts/us_officer_1.json
data/loadouts/us_rifleman_1.json
data/loadouts/us_support_1.json
data/roles/crewman.json
data/roles/medic.json
data/roles/officer.json
data/roles/support.json
data/schemas/loadout.schema.json
data/schemas/vehicle.schema.json
data/vehicles/us_m4a1_75.json
data/weapons/ger_mp40.json
data/weapons/us_m1911a1.json
data/weapons/us_m1_carbine.json
data/weapons/us_thompson_m1a1.json
```

</details>

<details>
<summary>docs：3 个</summary>

```text
docs/ASSET_GAPS.md
docs/NEEDS_HUMAN.md
docs/PLAN_ISSUES.md
```

</details>

<details>
<summary>tasks：1 个</summary>

```text
tasks/STATUS.md
```

</details>

<details>
<summary>tools：8 个</summary>

```text
tools/data-validate/lib.mjs
tools/data-validate/rules/ranges.mjs
tools/data-validate/rules/tunable.mjs
tools/data-validate/rules/units.mjs
tools/data-validate/test/cli.test.mjs
tools/map-pipeline/README.md
tools/map-pipeline/map_pipeline/carentan.py
tools/map-pipeline/map_pipeline/cli.py
```

</details>

## 4. 协议、数据和确定性的断点

### 4.1 实际是 v4 ↔ v10；v8 是历史检查点

- `crates/protocol/src/lib.rs`：共同祖先 **v1**，MS **v4**（`15df2e8` 的自车座位状态改动）；P 的 `349e376`/`911ee9a` 是 **v8**，`170d592` 是 **v9**，`550a381` 整合为 **v10**，本次 P tip 仍是 v10。不要按 v8 设计当前合流；如果还有历史试玩客户端，v8 也必须列入拒绝测试。最早 v8 提交 `349e376` 的 `native.json` 仍有 v7，`def2ae9` 才钉回 v8；当前 tips 代码与该 JSON 的版本值一致。
- 这不是只改一个版本号。`crates/protocol/src/{channel.rs,msg/,snapshot/}` 的例子（MS → P）：
  - **C0**：`Welcome` 新增 16 B `resume_ticket` 及 `resumed`，28 → 45 B；`DeployRequest/Result` 由 `0x10/0x11` 改成 `0x07/0x08`，请求体同长度但字段、顺序不同。放置消息也改编号和格式；MS 的小队/聊天等控制消息不能当作仍存在。
  - **C1/C2**：输入 bit 14 从保留位变为 `FIRE_MODE`，座位 aux 含义随线演进。普通玩家的 keyframe 也新增 `has_vehicle` 位；车辆 delta、自状态（不含校正时 56 → 58 bit）和事件枚举均不兼容，事件 tag 7 从 Marker 变为 Down。
  - **C3/C4**：MS 不可靠 C3 队伍状态不能径直换成 P 的可靠 C4 物流/小队状态；C4 中 `0x01` 从 Kill 改为 MatchPhase、`0x09` 从 MatchPhase 改为 DeathView。`Hello.caps` bit 2 从 `CAP_TEAM_MARKERS` 改为 `CAP_DEATH_VIEWS`，不是兼容开关。对照消息 ID、字段顺序/宽度、枚举和布局，不能从一侧套用整份 codec；必须走真实 wire 而不只测 TS 类型。
- 两线都在 `crates/server/src/net/handshake.rs::check_hello` 比较 Hello 的版本和数据哈希，版本先于哈希；两端原版 Hello 的编码可互解，静态推导 v4/v10 交叉会在入场票校验/分配席位前收到 `VersionMismatch` 并关闭；双方已有本端构造错版本/错哈希的 `ws_handshake.rs` 测试，本次没有跑跨版本实物互连。浏览器会检查 `Welcome.dataHash`，明确拒绝/哈希不符不触发 WT→WS 回退。MS 的 `Hello.ticket` 是网关签发的入场票；P 的可恢复模式把非空 `Hello.ticket` 当作进程内续连凭据。两凭据的目的、寿命、撤销、席位占用和遥测不能混用。建议制定明确的 admission/resume 意图区分，首次入场只用已验签票，续连凭据绑定现有授权会话，状态查询不占席。不得把试玩匿名直连接入正式队列。
- **建议**为最终新 wire 契约分配新版本（例如 v11，实施时查有没有已占用版本），不承诺滚动期间 v4/v8/v10 互通。独立区服和客户端版本并行；发布和回滚都按同一构建的网页 JS、WASM、服、网关配置和内容清单成套操作，防旧缓存和旧票连错服。

### 4.2 `DATA_HASH` 与地图身份是不同检查

| 固定版（以 `crates/sim-wasm/tests/golden/native.json` 核对） | `DATA_HASH`（u64） | seed 1 × 18,000 predictor 终点 golden |
|---|---|---|
| MS v4 | `0x2637b94dda28a192` / `2753873441184063890` | `0x5bea1351d8223cd6` |
| P v10 | `0x547179ff523f16c3` / `6084778709057738435` | `0xf2b111d21ef632ed` |

`crates/sim/{build.rs,src/data_hash.rs}` 从 `/data` 非 `schemas/` 的 JSON 路径和值做规范化 SHA-256，再取首 8 字节的 little-endian u64。不是只 hash 数值配置：换路径、增删 JSON、`_meta` 等字段变化也会换全局值；单纯重排键或空白不换。v8 自身也不止一个数据集：`def2ae9` 为 `4966131997518681532`、`911ee9a` 为 `17575066828767126413`。所以即使统一协议号，也不能把异版 server/native/WASM 混在一起。不要照抄任一旧 hash。以批准的数据树生成，并核对服务端与实际下发给浏览器的 WASM；`client/public/data` 若有镜像，另验同步。还有三处握手 hash 覆盖不到的契约：

- **武器索引**：MS 按路径顺序嵌入武器，P 的 `crates/sim/build.rs` 使用 `/data` 之外的 `crates/sim/weapon-index.json`（前端也读），单改映射可能改变武器 ID 的 wire 含义却不换数据 hash。
- **实际地图**：运行时地图来自 `world_loader`，握手值是编译期 hash，不直接覆盖 `.r16`、navmesh `.bin` 等字节。`config.rs` 的 map id 1 已由 MS 的 `carentan/m1` 变成 P 的 `p1`（`carentan1=2`），mode id 1 由 `frontline` 变成 `frontline-lite`（P 的有序 `frontline=4`）。不能只保留数字；逐场景核对地图/切片/模式和 manifest/FPAK 校验值。
- **客户端 ABI**：`client/src/net/wasm/layout.ts` 的 `AUTH_STATE_BYTES` 为 80 → 39、`SELF_LEN` 为 15 → 13；JS 与同版 WASM 一致不能由 Hello 证明。构建和部署都需要成套版本与真实快照测试。

### 4.3 不能用重钉 golden 消掉未解释的变化

两边 sim 的输入、系统顺序、伤害/移动/载具、spawn、碰撞与种子消耗各自演进。不同数据 hash、predictor 终点、布局 JSON 和二进制 snapshot 不是冲突解决的任选项。先保存旧线的同种子轨迹、golden 和构建身份，定位第一处预期或非预期的差异。只有在单位/字段、序列化和 gameplay 回归可解释后，才从合流结果重生成 `hello.hex`、`snapshot_key.hex`、`sim-wasm/tests/golden/{native.json,layout.json,snapshot_0.bin}` 和 TS/WASM 布局映射。不能从模拟冲突树直接执行/发布。

保留 `determinism_basic.rs` 的多线程和注入扰动回归，以及 `determinism_native.rs` 的黄金断言。切换门槛建议用 `FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism`：3 种子 × 18,000 tick，native 与 WASM 各双跑，逐 tick 比较 hash **和位置**；默认组允许跨目标漂移只告警，不应作为合流发布的唯一门槛。再用真实 Carentan 场景补录权威输入的复放，覆盖倒地/压制、建造、破坏、区段易手、换座/续连等，基础 predictor 绿不等于这些系统行为已验证。

## 5. 建议次序和回退线（本批不执行）

1. **锁定两套可重建基线。** 从 MS 拉隔离集成分支；固定两边 SHA、依赖锁、工具链、地图/素材清单，按上述脚本各跑基线，保存二进制和日志。`--cherry-pick` 的去重计数不是移植范围；按功能取相依提交，历史报告及台账双方都保留，不将其改写为新构建已通过。开始实施时重新 fetch/re-pin 和模拟，本文列表不是未来分支的审批。
2. **先冻结数据与模式边界。** 并存 M1 Carentan/Assault 和 P1/Carentan 有序战线；选定 schema、数值/史实、地图 ID 和角色/载具映射，按阶段引入 sim 的 schema/loader → 对局规则/工事轻车 → bot。以 MS 默认模式稳定为约束，不因接入新内容改写现有 M1 的默认场景。共同修改的 sim 调度和快照逐项查，不取整边。每组先过数据、sim 和确定性。
3. **把协议、服、WASM/前端 codec 当同一迁移单元。** 允许用夹具验证未接 HUD 的路径；保护 MS 网关签票、票据校验、注册/结果和 AOI，明确续连授权与过期语义；逐条补 v10 wire 能力、新版 handshake、C0/C4、keyframe/delta 和 client recovery。关住玩法入口，直到生产构建初次入局和续连都过关。分离旧版与新版 URL/服务器池，禁止靠客户端降级尝试旧协议。
4. **接入客户端与打包，再开灰度。** 合并配置、事件和无样式逻辑，再接渲染/占位；保留 MS 资源流式和现有脚本的默认门禁，检查工作流没有恢复自动触发。统一 ledger，检查专用文件、生成物、依赖许可证和包预算；不要用整边目录覆盖。视觉待 Fable 设计。客户端、网关和战术服同版经真实链路运行两种模式后，才考虑正式切换。
5. **回退门槛。** 版本/数据接受错误、会话或席位串用、跨队状态泄漏、玩法回归、无法解释的 trace 漂移、持续 tick 超时或整包门禁失败即停止放量；恢复旧 HTML/JS/WASM、静态内容、兼容的网关配置和旧服池，放完旧场再切。演练数据库迁移的回退/保留策略，不能假定旧进程能读新 schema 或接管新进程的会话票。

### 所需验证清单（合流候选，均未测）

| 门槛 | 必须覆盖 |
|---|---|
| 合并卫生、类型和数据 | `CARGO_BUILD_JOBS=8 scripts/local-ci.sh lint rust ts data map`；再查锁文件、schema、地图 pipeline 可重建、公开构建没有 `INTERNAL`、license 台账/免费来源与任务索引；Actions 不触发。 |
| 确定性、WASM 与协议 | `FRONTLINE_DETERMINISM_STRICT=1 CARGO_BUILD_JOBS=8 scripts/local-ci.sh determinism wasm`，单独回放两线代表地图的权威 trace；大小与内存布局；协议黄金/属性测试、server/headless/TS codec 互对、二进制 snapshot 和快照回滚。旧 v4/v8/v9/v10 ↔ 新版正反交叉的拒绝路径、新版匹配 hash 成功/错 hash 失败。 |
| 网关到实服 | `crates/{gateway,server}/tests/` 的 `gateway`、`gateway_link`、`session_flow`、`transport_matrix`、`ws_handshake`、`wt_loopback`；窗口过期、重复/旧凭据拒绝、只读状态探测不占席、断线恢复同一玩家、满员/席位回收、WT 和 WS 回退、网关重启与结果幂等；验证轮换/回退后旧票与旧网页处理可解释。 |
| 游戏闭环 | M1 真 WS/WT 与浏览器：部署、权威出生、延迟补偿/反作弊、AOI/重发、倒地与压制、补给/Garrison、坦克多座；另跑 P1 Lite 和 Carentan 有序战线专项（`client/e2e/playwright.frontline.config.ts`、`frontline.spec.ts`）、十兵种、工事、两轻车及有序锁、地图/HUD 对应和恢复后首个有效快照，角色权限跨队不串。 |
| 资产与入局 | 生产包上跑 `scripts/local-ci.sh assets` 所需 Medium 先决产物；FPAK/OPFS 冷热、预取完成率、点击到可操作，地形/角色/轻车接线先用无头软件渲染，Windows 和 WT/WS 浏览器矩阵另验。 |
| 持久和参考环境 | 复跑两线的 30 分钟矩阵：MS Carentan 20 客户端、WS/WT、无损和 netem，P1 3×30 min 可玩混合负载；新增 Carentan 有序战线的同版多客户端夜跑，比较 tick、连续超时、带宽、内存与恢复，明确客户端数、bot 数、地图、协议、数据 hash。参考机/预发布受信任 TLS 以及 20 人真人与录像仍未测（H-017/021/054/056）；工作站按 PI-109 不替代正式验收。 |
