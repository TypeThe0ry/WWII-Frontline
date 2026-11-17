# 任务总索引（tasks/INDEX.md）

> 状态：规划阶段产物。本文是**全部任务的唯一清单**；执行状态见 `tasks/STATUS.md`；执行规则见 `tasks/README.md`。
> 任务 ID 格式 `M{n}-{三位序号}`；**编号即执行顺序**：同一里程碑内，被依赖者编号必然小于依赖者（已由脚本校验）；跨里程碑依赖只指向更早的里程碑。
> 任务文件路径：`tasks/M{n}-{三位序号}-{slug}.md`（展开方式见 README §2.1）。

## 0. 读表说明

**文档缩写**（引用章节列）：`ARCH` = docs/ARCHITECTURE.md；`GD` = docs/GAMEDESIGN.md；`UI` = docs/UI_SPEC.md；`MAPS` = docs/MAPS.md；`REF` = docs/REFERENCE_MEASUREMENTS.md；`SHOP` = docs/ASSET_SHOPPING_LIST.md；`GAPS` = docs/ASSET_GAPS.md；`LICENSES` = 根目录 LICENSES.md；`MILESTONES` = docs/MILESTONES.md；`BRIEF R{n}` = 总控裁定 §3c 的第 n 条（已体现在各文档，任务以裁定为准）。

**工作流**（stream）：
- `repo`：仓库/CI/工具链
- `sim`：共享模拟 crate
- `net`：protocol 与客户端 net/game 状态
- `server`：对局服务器与 gateway
- `client-render`：客户端渲染/输入/表现
- `ui`：UI 屏幕与组件
- `audio`：音频
- `world`：地形/流式/植被/建筑/剔除
- `assets`：素材管线与素材接入
- `bots`：bot 与 LLM 指挥官
- `voice`：语音（LiveKit）
- `test`：测试/压测/验收
- `ops`：部署/监控/合规/发布
- `data`：/data 数值、史实复核、对标测量
- `maps`：地图管线与地图数据

**规模**：`S` ≤ 约 150 行新代码或等量工作；`M` ≤ 约 350 行；`L` ≤ 约 600 行（**一次会话上限**，超出必须拆分并回写本表，见 README §5）。

**素材需求列**：写需求 ID 前缀（对应 `docs/ASSET_SHOPPING_LIST.md` 的需求 ID）或"无"。含"灰盒可/占位可/缺则静音"的任务**不被素材阻塞**（按 `docs/ASSET_GAPS.md` §3 占位约定先完成）；名为 `assets-m*-integrate-*` 的"素材接入"任务才真正依赖素材到位，素材未 OK 则该任务标 `blocked` 或 `partial`，**不影响其它任务**。

**`[人工]` 标记**：范围列以 `[人工…]` 开头的任务需要人类提供输入（录像、数据下载、史料校对、账号/域名等）；输入缺失时标 `blocked` 并写明所缺条目，继续做不依赖它的任务（见 README §6）。

**数量说明**：下列任务数量高于总控给出的参考范围（M0 25–30、M1 45–55、M2 35–45、M3 50–60、M4 35–45），原因是严格按"一次会话 ≤ 600 行"拆分、且每个 UI 屏幕/素材接入/对标测量/史实复核/验收都独立成任务。偏差已向总控说明；如要压缩，优先合并同工作流相邻的 S/M 任务，**不要**增大单任务规模。

## 1. M0 技术验证（M0-001 – M0-033，共 33 个任务）

| 任务 ID | slug | 标题 | 工作流 | 前置依赖 | 规模 | 引用章节 | 一句话范围 | 关键验收指标 | 素材需求 |
|---|---|---|---|---|---|---|---|---|---|
| M0-001 | `repo-scaffold` | 仓库脚手架与工具链 | repo | — | M | ARCH §4.1; ARCH §4.5 | 建 Cargo workspace（protocol/sim/sim-wasm/server/gateway/bots/headless-client/balance-sim 空壳）、client（Vite+TS+Preact 空壳）、tools/、data/、.gitignore（assets/incoming、assets/processed、docs/measurements/raw）、rust-toolchain、pnpm workspace、开发说明 | `cargo build --workspace` 与 `pnpm -C client build` 零警告通过；目录与 BRIEF §4 布局一致 | 无 |
| M0-002 | `ci-baseline` | GitHub Actions 基线与依赖图检查 | repo | M0-001 | M | ARCH §17.5; ARCH §4.5 | lint/test-rust/test-ts/wasm-build 工作流；基于 cargo metadata 的依赖方向检查脚本（违反 §4.5 即失败）；cargo-deny | 故意加入 sim→tokio 依赖的 PR 被 CI 拒绝；全部 PR 作业墙钟 < 15 min | 无 |
| M0-003 | `font-license-check` | 字体许可证核实与登记（R10） | assets | M0-001 | S | UI §2.3; SHOP §4.1.6; LICENSES §3 | 核实 UI_SPEC §2.3 选定字体与 CJK 子集字体的许可证原文，存档 licenses/texts/，在 LICENSES.md 登记；写子集化流程说明 | 每个字体在 LICENSES.md 有一行且状态为 OK/PENDING；保留字体名（RFN）风险已记录；无法联网核实的写入 PLAN_ISSUES | V-FNT-*（仅登记，不购买） |
| M0-004 | `data-schema-validate` | /data schema 与校验工具 | data | M0-001 | M | ARCH §8.5; ARCH §4.4; ARCH §17.5 | /data/schemas/（weapons/roles/loadouts/vehicles/maps 草案）、tools/data-validate、sim 的 build.rs 嵌入 /data 并计算 DATA_HASH、CI data-validate 作业 | 对缺字段/错单位的样例 JSON 校验必报错；同内容 DATA_HASH 稳定 | 无 |
| M0-005 | `protocol-bitio-messages` | protocol：位流、量化与 M0 消息集 | net | M0-001 | L | ARCH §4.3.1; ARCH §6.8; ARCH §8.2 | BitWriter/BitReader、位置/角度量化、PROTOCOL_VERSION、Codec trait、Hello/Welcome/Reject/Ping/Pong/InputBundle/InputCmd/FireCmd；no_std+alloc | proptest 往返 10^6 样本、量化误差 ≤ 1/64 m 与 0.35°；InputBundle(3 条) ≤ 30 B；无 unsafe | 无 |
| M0-006 | `protocol-snapshot-delta` | protocol：快照编码与基线增量 | net | M0-005 | L | ARCH §6.5; ARCH §8.2; ARCH §19.1 | SnapshotEncoder/Decoder、32-tick 基线环、全量关键帧、field_mask、cargo-fuzz 解码靶 | decode(encode(b,a),a)==b 属性测试；fuzz 10 min 无 panic；近景玩家 delta 记录均值 ≤ 10 B | 无 |
| M0-007 | `sim-core-world` | sim：World/step/确定性底座 | sim | M0-005, M0-004 | L | ARCH §4.3.2; ARCH §8.1; ARCH §6.1 | bevy_ecs（独立使用）、World/step(inputs)、固定顺序 EntityId、Pcg64Mcg+种子公式、libm、state_hash、WorldView | 同 seed+输入两次运行 1 万 tick state_hash 逐 tick 相同；禁用依赖检查通过 | 无 |
| M0-008 | `data-weapons-m0` | 数据：M0 步枪（Kar98k、M1 Garand） | data | M0-004 | S | ARCH §8.5.1; GD §6.2; GD §6.3 | /data/weapons/ 两个 JSON（historical 与 gameplay，状态 initial） | data-validate 通过；全部数值带单位且标 initial | 无 |
| M0-009 | `map-m0-terrain-block` | 地图：M0 单块地形（256 m）数据管线 | maps | M0-001, M0-004 | M | MAPS §7.3; MAPS §1.1; MAPS §1.2; MAPS §6 | tools/map-pipeline 第一版：默认程序化分形高度图；若 assets/incoming/maps/carentan 有 DEM 则走 GDAL 路径；导出 heightmap_1m.r16、meta.json、64 m 区块切片 | 输出 257² 高度图与 meta.json 通过 validate；同输入输出哈希一致 | 无（可选 DEM，见 MAPS §2.1） |
| M0-010 | `sim-terrain-collision` | sim：高度场碰撞 | sim | M0-009, M0-007 | M | ARCH §9.5; ARCH §5.3 | 区块碰撞数据构建 Rapier HeightField；地面检测、坡度限制、射线查询 API | 高度场射线 1 万次 < 5 ms；角色无法攀上 > 35° 坡面 | 无 |
| M0-011 | `sim-kcc-movement` | sim：Rapier 角色控制器与基础移动 | sim | M0-010 | L | ARCH §5.3; ARCH §6.10; GD §9.1; GD §9.2 | Rapier KCC（enhanced-determinism）、走/冲刺/蹲/跳、step_player（供预测复用） | 单次 KCC ≤ 20 µs（criterion，参考机）；100 个 KCC 一 tick ≤ 3 ms；速度与 /data 误差 < 2% | 无 |
| M0-012 | `sim-weapon-rifle` | sim：步枪射击、弹道与装填 | sim | M0-011, M0-008 | L | GD §6.3; GD §6.4; GD §6.5; GD §6.6; ARCH §6.7.3 | 半自动/栓动状态机、散布与后坐、段扫描弹体（重力+阻力）、头/躯干/四肢命中体、伤害衰减、弹匣与装填 | 500 m 下坠与 RK4 数值参考解的误差 < 3%；射速 ≤ cap；单测覆盖状态机 | 无 |
| M0-013 | `sim-damage-death` | sim：生命、死亡与简易重生 | sim | M0-012 | S | GD §7.1; GD §7.2; GD §10.6 | 生命值、死亡（M0 无倒地）、固定出生点计时重生、击杀事件结构 | 躯干致死发数与 /data 一致；重生计时误差 ≤ 1 tick | 无 |
| M0-014 | `sim-wasm-bindings` | sim-wasm：WASM 封装 | sim | M0-011, M0-006, M0-012 | M | ARCH §4.3.3; ARCH §8.3 | wasm-pack 构建、init/step_player/decode_snapshot/apply_snapshot、体积检查脚本、CI wasm-build | wasm-opt 后 ≤ 1.2 MB（br ≤ 400 KB）；Node smoke 测试通过 | 无 |
| M0-015 | `server-skeleton` | server：进程骨架与 30 Hz 主循环 | server | M0-007, M0-005 | M | ARCH §6.1; ARCH §6.10; ARCH §18.3 | 专用 sim 线程绝对时间表循环、有界 channel、tracing JSON 日志、/metrics（tick_ms 直方图）、CLI/环境变量配置 | 空载 10 min 的 tick 间隔抖动 P99 < 1 ms；tick_ms 可被采集 | 无 |
| M0-016 | `server-transport-ws` | server：WebSocket 传输与握手 | server | M0-015 | M | ARCH §6.6.2; ARCH §19.1 | tokio-tungstenite、通道复用帧、Hello/Welcome、握手超时 5 s、缓冲上限、"最新者优先"丢旧快照 | 10 个 WS 客户端握手并保持 10 min；丢旧快照逻辑有单测 | 无 |
| M0-017 | `server-transport-wt` | server：WebTransport 传输 | server | M0-016 | L | ARCH §5.5; ARCH §6.6.1; ARCH §18.2 | wtransport：datagram（C1/C2）、bidi stream（C0）、uni stream（C4）；开发自签证书脚本 | Chrome 本机连入并收发 datagram；同一 Message 层与 WS 通用 | 无 |
| M0-018 | `server-session-input` | server：会话、输入缓冲与校验 | server | M0-017, M0-013 | M | ARCH §6.3; ARCH §15.1 | 玩家会话、8-tick 输入缓冲、seq 去重/窗口/速率校验、缺失输入策略、进出世界 | 乱序/重复/过期输入单测通过；缺失 5 tick 后清零移动 | 无 |
| M0-019 | `server-snapshot-aoi-v0` | server：AOI v0 与快照发送 | server | M0-018, M0-006 | L | ARCH §6.5.4; ARCH §6.9 | 64 m 网格、距离优先级累加器、20 Hz 错相出包、self_state、基线 ack、事件冗余 | 10 客户端下行 P50 ≤ 128 kbps；AOI+编码阶段 ≤ 2.6 ms/tick | 无 |
| M0-020 | `client-render-bootstrap` | client/render：引擎初始化与调试叠层 | client-render | M0-001 | M | ARCH §5.1; ARCH §10.3; ARCH §13.6 | WebGPUEngine→Engine 回退、能力探测、渲染循环与 30 Hz 固定步长解耦、帧时间/GPU timestamp 叠层、ComputeShader 冒烟 | 两条路径均渲染测试场景；叠层显示 GPU/CPU 帧时间；compute 写纹理冒烟通过（§20.1-4） | 无 |
| M0-021 | `client-net-transport` | client/net：传输与时钟同步 | net | M0-014, M0-017 | M | ARCH §6.2; ARCH §6.6; ARCH §4.3.9 | WT 优先 WS 回退的传输抽象、握手、Ping/Pong EWMA、estimated_server_tick、输入提前量 | 80 ms 人工延迟下时钟误差 ≤ ±1 tick；禁用 WT 时自动走 WS | 无 |
| M0-022 | `client-prediction-reconcile` | client/game：输入、预测、对账与插值 | net | M0-021, M0-020 | L | ARCH §6.3; ARCH §6.4; ARCH §8.4 | Pointer Lock 与 InputCmd 采样、未确认输入环、sim-wasm 预测、对账（0.05/1.0 m 阈值（R5）、100 ms 平滑）、100 ms 实体插值缓冲、事件总线 | vitest 回放录制快照：误差 < 0.05 m 不回滚；100 ms 延迟+2% 丢包下静止校正 ≤ 1 次/10 s | 无 |
| M0-023 | `client-terrain-cdlod` | client/render：CDLOD 地形 | client-render | M0-020, M0-009 | L | ARCH §9.2; ARCH §9.8 | 四叉树节点选择、65×65 实例网格、顶点 morph、高度纹理数组、256 m 近景 + 8 km 低精度远景环、程序化着色 | RTX 3060 参考机 1080p 地形 GPU ≤ 2 ms；LOD 切换无可见裂缝；WebGL2 路径同样可渲染 | 无（V-NAT-007 CC0 可选） |
| M0-024 | `client-vegetation-m0` | client/render：植被实例化与 impostor | client-render | M0-023 | M | ARCH §9.6; ARCH §13.5.3 | thin instances 草/树、LOD 与八面体 impostor 烘焙（程序化占位几何）、风动 | 单块地形+植被：RTX 3060 ≥ 60 fps，Iris Xe（WebGL2）≥ 30 fps（§20.1-5）；draw call ≤ 400 | 无 |
| M0-025 | `client-actors-placeholder` | client/render：灰盒角色、第一人称枪与射击反馈 | client-render | M0-022, M0-023 | M | GAPS §3; ARCH §7.1; ARCH §7.2 | 胶囊+头球占位角色（阵营色、PH_ 命名与 socket）、第一人称枪占位、枪口焰/曳光/弹着点（内置粒子）、命中标记 | 开火与枪口焰同帧（本地预测）；节点命名符合 PH_{需求ID}__；10 名远端实体插值无抖动 | 无（灰盒） |
| M0-026 | `bots-m0-basic` | bots：M0 基础 bot | bots | M0-018, M0-012 | L | ARCH §16.1; ARCH §16.3; GD §20.1 | BotBrain trait；高度场网格寻路（无 navmesh）、视锥+LoS 感知、瞄准误差/反应延迟、产出与真人同构的 InputCmd、服务器分片宿主 | 10 bot 对战 5 min 无 panic；bot 思考 ≤ 1 ms/tick；单测断言 bot 输入走同一校验路径 | 无 |
| M0-027 | `test-headless-v0` | headless-client v0 与 10 客户端联测 | test | M0-019, M0-026 | M | ARCH §17.2; ARCH §4.3.7 | 无渲染客户端：握手、合成输入、解码快照、字节分布统计，WT 与 WS 双路径 | 10 headless+10 bot 跑 30 min：tick_ms P99 ≤ 28 ms；每客户端下行 P50 ≤ 128 kbps | 无 |
| M0-028 | `test-browser-net-interop` | 测试：浏览器 WT/WS 互通矩阵 | test | M0-021 | S | ARCH §5.0; ARCH §20.1-1 | Playwright 跑 Chromium/Firefox/WebKit 连接 WT 与 WS，记录支持表与失败原因 | 矩阵记录入 STATUS；Chromium WT+WS 均通过；其余浏览器结果如实登记 | 无 |
| M0-029 | `test-determinism-drift` | 测试：确定性与 WASM 漂移 | test | M0-014, M0-012 | M | ARCH §17.1; ARCH §2.1; ARCH §20.1-3 | native×2、wasm×2 state_hash 比对；native vs wasm 10 min 随机输入回放位置 RMS 漂移 | native/native 与 wasm/wasm 逐 tick state_hash 完全一致（阻断）；native vs wasm RMS < 2 cm/s（信息性，超出写 PLAN_ISSUES） | 无 |
| M0-030 | `test-perf-baseline-m0` | 测试：M0 性能基线与报告 | test | M0-024, M0-027 | M | ARCH §13.5; ARCH §20.1 | 固定机位性能采集脚本（Playwright+叠层导出 JSON）、criterion 基准入 CI（KCC/编码）、参考机测量流程文档 | 报告含 3060/Iris Xe 帧时间 P50/P99、KCC µs、快照编码 µs，写入 STATUS | 无 |
| M0-031 | `hist-params-review-m0` | 史实参数复核：Kar98k 与 M1 Garand（R8） | data | M0-008 | S | REF §6; GD §0.4; GD §6.2 | 按 REF §6.2 来源逐字段复核 historical.* 并填 _source，状态 initial→reviewed；无法核实者标"未核实" | 两把枪 historical 字段 100% 有出处或明确"未核实" | 无 |
| M0-032 | `ref-measure-m0` | 对标测量：M0 批回填 | data | M0-012, M0-011 | M | REF §9.1; REF §7; REF §8 | [人工：录像] 建 tools/ref-analysis 与 docs/measurements/ 模板；录像到位后分析批 M0 项并回填 /data；录像缺失则 blocked | 批 M0 每项有记录与 summary.csv 行；回填后我方复测误差在 REF §7.3 容限内 | 无（需人工录像，REF §9.2） |
| M0-033 | `m0-acceptance` | M0 里程碑验收与 STATUS 汇总 | test | M0-030, M0-029, M0-027, M0-028, M0-025, M0-026, M0-031, M0-032 | M | MILESTONES §1; ARCH §20.1 | 核对 M0 退出标准、走演示脚本、ARCH §20.1 五项风险逐项给结论、汇总 STATUS 与 PLAN_ISSUES | 退出标准全部有实测数字；五项风险各有"通过/不通过+证据" | 无 |

## 2. M1 垂直切片（M1-001 – M1-072，共 72 个任务）

| 任务 ID | slug | 标题 | 工作流 | 前置依赖 | 规模 | 引用章节 | 一句话范围 | 关键验收指标 | 素材需求 |
|---|---|---|---|---|---|---|---|---|---|
| M1-001 | `asset-pipeline-ingest-gate` | 素材管线：ingest 与许可证闸门 | assets | M0-002, M0-003 | L | ARCH §11.1; ARCH §11.7; LICENSES §1 | tools/asset-pipeline ingest/normalize：无 LICENSES.md 登记或状态非 OK 即拒绝、sha256、命名规范校验、单位与朝向统一、缺失素材报告（real/placeholder/missing） | 对 PENDING/未登记样例必拒绝；缺失报告按需求 ID 汇总；CI 作业 < 3 min | 无（用 CC0 样例测试） |
| M1-002 | `asset-pipeline-mesh-texture` | 素材管线：网格 LOD、纹理与碰撞简化 | assets | M1-001 | L | ARCH §11.1; ARCH §11.4 | meshoptimizer LOD（1/0.5/0.25/0.1）、gltf-transform（dedup/prune/meshopt 量化）、toktx（ETC1S/UASTC，法线 UASTC）、ORM 打包、碰撞网格简化 | CC0 样例处理后体积 ≤ 原 25%；LOD 三角比在 ±5% 内；UV 边界保持 | 无（CC0 样例） |
| M1-003 | `asset-pipeline-fpak` | 素材管线：.fpak 分块容器（D-PAK） | assets | M1-001 | L | ARCH §11 (R11); SHOP §3.4; LICENSES §1 | .fpak 格式（索引、64 m 区块/类别分包、完整性哈希、HTTP Range 读取）、打包器与 TS/Node 读取器；产物不含独立 .glb/.ktx2/.ogg 文件 | 往返字节一致；随机 Range 读取 1000 次全部正确；脚本断言产物目录无独立标准格式文件 | 无 |
| M1-004 | `asset-pipeline-audio-pack` | 素材管线：音频处理与打包 | assets | M1-003 | M | ARCH §12.7; ARCH §11.1; SHOP §3.4 | ffmpeg/opusenc：响度归一、Opus(WebM) 封装进 fpak、AAC 兜底、客户端 decodeAudioData 探测协议 | 样例音频往返一致；响度报告；AAC 兜底路径单测 | 无（样例） |
| M1-005 | `asset-pipeline-manifest-budget` | 素材管线：manifest 与体积预算门禁 | assets | M1-002, M1-003 | M | ARCH §11.3; ARCH §11.4; ARCH §17.5 | manifest.json 生成（含 placeholder 字段）、bundle 预算（core ≤ 8 MB、地图 Medium ≤ 150 MB）、CI manifest-budget 作业（超 1% 失败、超 20% 须人工批准） | 构造超预算 PR 被拒；manifest 与 fpak 索引一致 | 无 |
| M1-006 | `client-asset-loader` | client：fpak 加载、解码 Worker 与缓存 | client-render | M1-003, M0-020 | L | ARCH §11.5; ARCH §11.6; ARCH §9.3 | Range 读取 fpak、Worker 池解码 KTX2(BasisU)/Meshopt、分帧 GPU 上传、Cache API/OPFS、占位回退、stream.progress 事件 | 解码期间主线程长任务（> 50 ms）为 0；OPFS 不可用时降级运行；上传 ≤ 1.5 ms/帧 | 无 |
| M1-007 | `data-roles-loadouts-m1` | 数据：M1 五兵种、loadout 与基础规则 | data | M0-004 | M | GD §5; GD §4.4; GD §3.2; GD §10; GD §12; ARCH §8.5.2 | Officer/Rifleman/Medic/Support/Crewman × US/GER 的 roles 与 loadout（L1–L3）、名额、占领/出生/补给初值（initial）；GER Rifleman 加 Panzerfaust 60 的 m1_only loadout 作反装甲手段（loadout 引用的武器 ID 是否存在由 M1-008 验收校验，本任务不依赖 M1-008） | data-validate 通过；loadout 引用的武器/手雷 ID 格式合法（存在性校验在 M1-008 验收）；名额与 GD §4.4 一致 | 无 |
| M1-008 | `data-weapons-m1` | 数据：M1 武器与投掷物 | data | M0-008 | M | GD §6.2; GD §6.3; GD §6.9 | M1 loadout 用到的全部武器（卡宾枪/冲锋枪/手枪/Mk2/M24/烟雾/Panzerfaust 60）historical 与 gameplay JSON，状态 initial | data-validate 通过；每把武器有 historical._source 占位 | 无 |
| M1-009 | `data-vehicle-sherman` | 数据：M4A1 Sherman（M1 的 1 辆坦克） | data | M0-004 | S | GD §14.3; GD §14.4; ARCH §8.5.3 | us_m4a1_75 车辆 JSON：座位、主炮弹种与弹药、装甲板厚/倾角、模块、成本与刷新 | data-validate 通过；全字段带单位；与 GD §14.3 表一致 | 无 |
| M1-010 | `protocol-messages-m1` | protocol：M1 全消息集 | net | M0-006 | L | ARCH §6.8; ARCH §8.2 | C0（Deploy/SquadOp/Chat/Marker/Report）、C4（Kill/Death/Revive/ScoreDelta/ObjectiveChange/SpawnPointState/PlayerRoster/Notice/MatchPhase）、C2.events（Shot/Impact/HitConfirm/DamageDir/Explosion/Suppress）、载具实体与团队通道编码 | 往返+fuzz 通过；载具 delta 记录 ≤ 24 B；PROTOCOL_VERSION 递增并有变更说明 | 无 |
| M1-011 | `sim-movement-full` | sim：完整移动（姿态、体力、翻越、探身） | sim | M0-011, M1-007 | L | GD §9; GD §9.6 | 趴/蹲/站切换时间、体力、翻越/攀爬、探身、姿态对速度/散布修正、服务器验证接口 | 各姿态速度与 /data 误差 < 2%；翻越时间与表一致；体力耗尽/恢复曲线单测 | 无 |
| M1-012 | `sim-weapon-ballistics-pen` | sim：弹道细化与穿透 | sim | M0-012 | M | GD §6.4; GD §6.7 | 阻力参数化、材质穿透表（木/砖/沙袋/金属）、跳弹最简模型、穿透后衰减 | 木板/沙袋穿透深度与 /data 一致；穿透射线不超 3 段 | 无 |
| M1-013 | `sim-weapon-classes-m1` | sim：冲锋枪、卡宾、手枪与近战 | sim | M1-012, M1-008 | L | GD §6.3; GD §6.5; GD §6.8 | 全自动/点射/半自动、手枪、刺刀/近战判定、后坐模式、战术/空仓装填 | 各武器 cyclic rate 与 /data 误差 ≤ 2%（headless 测量）；近战判定单测 | 无 |
| M1-014 | `sim-damage-hitzones` | sim：命中区与伤害模型 | sim | M1-012, M0-013 | M | GD §6.6; ARCH §6.7.1 | 头/躯干/四肢、距离衰减、姿态命中体、友军伤害开关、击杀归属输入 | 致死发数表（近/中/远）与 /data 一致；命中体与回溯历史格式一致 | 无 |
| M1-015 | `sim-grenades-smoke` | sim：手雷与烟雾 | sim | M1-013, M1-014 | M | GD §6.9 | Mk2/M24 引信与爆炸衰减、烟雾云与 LoS 遮蔽接口、投掷弹道 | 爆炸伤害-距离曲线与 /data 一致；烟雾内 LoS 查询返回遮蔽 | 无 |
| M1-016 | `sim-health-downed-revive` | sim：倒地、救起与击杀归属 | sim | M1-014 | M | GD §7 | 倒地/流血/死亡状态机、Medic 救起、自救绷带、非 Medic 对倒地者规则、击杀归属 | 状态机全转移单测；救起耗时与 /data 误差 ≤ 1 tick | 无 |
| M1-017 | `sim-suppression` | sim：压制 | sim | M1-012 | M | GD §8 | 弹道擦过检测、累积/衰减、效果参数输出、反刷分 | 擦过距离阈值与累积曲线单测；输出参数供客户端音画使用 | 无 |
| M1-018 | `sim-objectives-capture` | sim：据点占领与对局状态机 | sim | M1-007, M0-007 | M | GD §3.2; GD §3.6; GD §2.1; GD §2.2 | 占领速度公式、3 个活动据点、热身/进行/结束状态机、M1 简化胜负 | 5v5 占领速度与公式误差 < 1%；状态机单测 | 无 |
| M1-019 | `sim-spawn-garrison` | sim：HQ 出生与 Garrison | sim | M1-018, M1-016 | L | GD §10.1; GD §10.2; GD §10.6; GD §10.8; GD §2.6 | HQ 出生、Garrison（Officer 放置，消耗 Supplies，R1；本任务用 SupplyProvider 桩，闭环验收在 M1-020）、出生保护、波次/计时、重生请求校验 | 出生保护与计时单测；Garrison 放置规则（距敌/区域）全覆盖 | 无 |
| M1-020 | `sim-supplies-crates` | sim：Supplies 与补给箱 | sim | M1-019 | M | GD §12.1; GD §12.2; GD §12.3; GD §12.6 | Supplies 货币（R6；与 M1-019 的 SupplyProvider 桩对接并完成 Garrison 放置闭环验收）、Supply Crate 搬运/放置、Ammo Crate、补弹规则、反滥用 | 补给箱补弹量与 /data 一致；反滥用单测 | 无 |
| M1-021 | `sim-squads-roles-deploy` | sim：小队、兵种名额与部署校验 | sim | M1-019, M1-007 | L | GD §2.5; GD §4; GD §5 | 小队创建/加入/锁定/踢人/队长移交、编制上限（R2）、兵种名额、loadout 选择、deploy 请求权威校验 | 非法部署请求（满员/名额/不可用出生点）全被拒；编制上限单测 | 无 |
| M1-022 | `sim-vehicle-physics` | sim：载具物理（悬挂、履带、座位） | sim | M0-010, M1-009 | L | GD §14.1; GD §14.7; ARCH §5.3 | raycast 悬挂+履带模型、驾驶输入、座位/进出、碰撞、地形适应 | 路速与 /data（史实×0.75）误差 < 3%；40 辆合成载具一 tick ≤ 2 ms | 无（灰盒） |
| M1-023 | `sim-vehicle-combat` | sim：载具武器与装甲判定 | sim | M1-022, M1-012, M1-014 | L | GD §14.2; GD §14.4; GD §14.5 | 主炮弹（AP/HE）、Panzerfaust 60 发射与 HEAT 判定（M1 反装甲手段）、穿深与装甲判定、基础模块损伤、炮塔/炮管运动、同轴机枪 | 12 组（板厚×倾角×距离）AP 判定与手算一致；炮塔转速与 /data 一致 | 无 |
| M1-024 | `server-aoi-v1` | server：AOI v1（敌友规则、团队通道） | server | M0-019, M1-010 | L | ARCH §6.5.4; ARCH §15.2; BRIEF R3 | 敌方仅 ≤400 m 且（视锥+30° 或 ≤96 m）下发、标记敌人仅图标、友军最远 400 m/最低 2 Hz、1 Hz 团队通道、迟滞、载具优先级 | 20 客户端场景：视锥外 > 96 m 敌人下发数 = 0（审计）；P99 ≤ 400 kbps | 无 |
| M1-025 | `server-lagcomp` | server：延迟补偿 | server | M0-018, M1-014, M1-010 | L | ARCH §6.7; ARCH §15.1 | 16-tick 命中体历史、开火回溯（≤ 8 tick）、出射点校验 ≤ 0.5 m、墙体拒绝、指标 | 150 ms RTT 下 headless 命中率与 0 延迟差 ≤ 5%；回溯夹紧计数可见 | 无 |
| M1-026 | `server-deploy-squad-events` | server：部署/小队处理与可靠事件 | server | M1-021, M1-019, M1-010 | L | ARCH §7.3; ARCH §6.8.1; ARCH §6.8.3; ARCH §19.3 | C0 处理器（Deploy/SquadOp/Chat/Marker）、C4 可靠事件广播、roster 增量、聊天长度/字符过滤、速率限制 | 速率超限消息被丢弃并计数；C4 事件丢包 5% 下仍全部送达（headless） | 无 |
| M1-027 | `server-anticheat-basic` | server：基础校验（速度/射速/视角） | server | M0-018, M1-011 | M | ARCH §15.1 | 速度/加速度/射速/视角角速度/出射点校验、违规计数与日志 | 注入 1.3× 速度输入被纠正且计数增加；射速超限被拒 | 无 |
| M1-028 | `gateway-core` | gateway：服务器列表、游客鉴权与 ticket | server | M1-010 | L | ARCH §4.3.5; ARCH §19.2; ARCH §3.3 | axum：服务器列表、游客 token、join ticket（EdDSA、TTL 60 s、单次）、内部注册/心跳 API、SQLite；含 protocol DTO：ServerInfo / TicketClaims | ticket 过期/重放单测；限速每 IP 10/min | 无 |
| M1-029 | `server-gateway-link` | server：注册、心跳与 ticket 验证 | server | M1-028, M0-018 | S | ARCH §3.3; ARCH §19.2 | server 向 gateway 注册/心跳、验证 join ticket（jti 去重）、局终战绩上报 | 重放 ticket 被拒；5 s 心跳丢失 gateway 标记失效 | 无 |
| M1-030 | `map-carentan-sources-geo` | 地图：Carentan 数据源许可证与锚点对账 | maps | M0-009 | M | MAPS §2; MAPS §0.4; MAPS §7.1 | [人工] 核实各数据源许可证并存档，登记 LICENSES.md；OSM ODbL 衍生层公开与署名策略（R9）；锚点相似变换，meta.json 锚点表 | 每个数据源有许可证快照与状态；锚点 ≥ 8 个且标 confidence_m；人工项列入 NEEDS_HUMAN | 无 |
| M1-031 | `map-carentan-dem-third` | 地图：Carentan 1/3 区域高度图 | maps | M1-030 | M | MAPS §1.2; MAPS §1.3; MAPS §3.8 | [人工：DEM 下载] GDAL 脚本：DEM→1 m 高度图（M1 区域 1180 m 全精度、区外 8 m 远景环）；区外不可通行墙数据 | 锚点高程核对 ≥ 10 点误差 ≤ 1 m；同输入输出哈希一致；DEM 缺失则 blocked | 无 |
| M1-032 | `map-carentan-vectors-osm` | 地图：OSM 矢量提取与清洗 | maps | M1-030 | M | MAPS §1.6; MAPS §2.5; MAPS §6.2 | OSM→roads/water/hedgerows/buildings geojson 初稿、属性规范化、署名文件 | geojson 通过 schema；坐标全在 0..2048；署名文件存在 | 无 |
| M1-033 | `map-carentan-1944-edits-third` | 地图：1944 修正（M1 区域） | maps | M1-031, M1-032 | M | MAPS §1.5; MAPS §3.5; MAPS §3.6 | [人工] 树篱描摹、洪泛区修正多边形、terrain_edits.geojson 与应用脚本 | 区域树篱总长与目标 ±15%；应用后高度差分报告；人工描摹缺失则 partial | 无 |
| M1-034 | `map-carentan-zones-buildings-third` | 地图：Carentan 1/3 关卡标注与建筑足迹 | maps | M1-030, M1-032, M1-033 | L | MAPS §3.8; MAPS §3.6; MAPS §6.2; MAPS §6.3 | zones.json（L1a/L2a/L3a）、spawns.json（M1 HQ/Garrison 候选）、buildings 样式/enterable/levels、核心地标灰盒定义、map-pipeline validate | validate 全通过（据点不在水体/实心建筑、≥3 条入口、HQ 间可通行）；M1 临时 HQ 不写入完整图数据 | 无（地标用灰盒） |
| M1-035 | `map-chunk-bake-splat` | 地图：区块烘焙、splat 与植被分布 | maps | M1-031, M1-033, M1-034, M1-003, M1-005 | L | ARCH §9.1; ARCH §9.5; ARCH §9.6; MAPS §1.1 | 64 m 区块切片：高度/splat 规则（坡度、道路、水边）/植被实例表/碰撞/props 表/区块索引，输出 manifest 条目与 fpak 区块包 | 区块全部生成；单块 ≤ 80 KB；抽样 200 条射线碰撞与视觉高度一致 | 无 |
| M1-036 | `map-navmesh-bake` | 地图：Recast 导航网格烘焙 | maps | M1-035 | M | ARCH §16.2; MAPS §6.2; MAPS §7.1 | Recast 脚本（128 m 瓦片）、off-mesh 链接标注格式、navmesh.bin 与载具网格 | HQ↔据点导航连通；烘焙 < 5 min；参数与 ARCH §16.2 一致 | 无 |
| M1-037 | `bots-navmesh-pathfinding` | bots：navmesh 寻路与避障 | bots | M1-036, M0-026 | L | ARCH §16.2; GD §20.3 | Recast 数据加载、A*+漏斗、局部避障、off-mesh 使用、每 bot 重算 ≤ 2 Hz | 60 bot 路径查询 ≤ 1.0 ms/tick；建筑内多层路径连通测试通过 | 无 |
| M1-038 | `bots-behavior-tree-utility` | bots：行为树与效用 AI | bots | M1-037, M1-014 | L | ARCH §16.3; GD §20.3; GD §20.6 | 节点库（Sequence/Selector/Parallel/Decorator）、战术动作（占掩体/射击/换弹/投掷/救治/补给）、效用目标选择、难度参数 | GD §20.3 逐条行为各有场景测试；树节点 ≤ 80/bot；思考 ≤ 60 µs | 无 |
| M1-039 | `bots-squad-roles-fill` | bots：小队长 bot、兵种选择与填充 | bots | M1-038, M1-021, M1-026 | L | GD §4.7; GD §20.1; GD §20.2; ARCH §16.1 | 小队长 bot、兵种/出生点效用选择、Officer bot 放 Garrison、Support bot 补给、真人加入替换阵亡 bot | 20 人场景 bot 填满；真人加入按规则替换；bot 遵守兵种名额 | 无 |
| M1-040 | `client-game-state-events` | client/game：游戏状态与事件总线类型 | net | M0-022, M1-010 | L | ARCH §8.4; ARCH §7.2; ARCH §4.3.9 | roster/小队/部署状态机/据点/重生点/击杀播报状态、事件总线类型、signals 桥、C4 事件应用 | vitest 覆盖状态机与事件应用；重复事件按 event_id 去重 | 无 |
| M1-041 | `client-vehicle-prediction` | client/net：载具驾驶预测与同步 | net | M1-022, M0-022 | M | ARCH §6.4; GD §14.7 | 驾驶员载具预测与对账、乘员座位同步、远端载具插值 | 120 ms RTT 下载具位置校正 ≤ 1 次/10 s（直线行驶） | 无 |
| M1-042 | `render-pbr-material-system` | render：PBR 材质系统与变体管理 | client-render | M0-020, M1-006 | L | ARCH §10.4; ARCH §13.5.3 | 统一 PBR 着色器（WGSL 与 GLSL 对应）、宏变体、变体计数统计、预热遍历、材质冻结 | 变体 ≤ 60（CI 统计门禁）；预热后新物体首次出现无 > 50 ms 卡顿 | 无（V-NAT-007 CC0 材质） |
| M1-043 | `render-lighting-shadows` | render：光照、CSM 阴影与天空 | client-render | M1-042 | L | ARCH §10.1; ARCH §10.2; ARCH §9.8 | 方向光、CSM 4 级（WebGL2 3 级）、IBL、探针加载、大气 LUT 与云层 | 阴影 GPU ≤ 2.2 ms（High 参考机）；WebGL2 3 级可用 | V-SKY-001（CC0 HDRI） |
| M1-044 | `render-post-taa` | render：TAA、曝光与色调映射 | client-render | M1-043 | M | ARCH §10.1; ARCH §10.2 | Halton 8 相位 TAA、运动向量、自动曝光（直方图/ mip 链）、Bloom、色调映射 | 静止场景帧差 < 阈值（无闪烁）；后处理合计 ≤ 1.5 ms（High） | 无 |
| M1-045 | `render-characters-animation` | render：骨骼角色与动画状态机 | client-render | M1-006, M0-025, M1-042 | L | ARCH §13.5.3; GAPS §3 | GPU 蒙皮、动画状态机（走/跑/蹲/趴/受击/死亡）、LOD、重定向流水线接口、程序化占位动画 | 60 角色蒙皮 ≤ 1.5 ms GPU；占位与真资源切换不改代码 | V-CHR-*; V-ANM-*（占位可） |
| M1-046 | `render-first-person-weapon` | render：第一人称武器 pass | client-render | M1-045 | M | GD §6.10; SHOP §4.0 | 独立 FOV 与深度、不穿墙、socket 挂点、开火/装填动画触发、开镜 | 武器不穿墙（贴墙测试）；FP pass ≤ 0.4 ms | V-WPN-*（FP 占位可） |
| M1-047 | `render-vfx-basic` | render：粒子与贴花基础 | client-render | M1-042 | M | ARCH §13.5.3; GAPS §3.2 | 粒子系统（枪口焰/曳光/烟/爆炸/尘土）、弹孔贴花、粒子预算与池 | 800 活动粒子 ≤ 0.7 ms；粒子超限按优先级丢弃 | V-FX-*（占位可） |
| M1-048 | `render-vehicle-tank` | render：坦克渲染 | client-render | M1-042, M1-041 | M | GD §14.1; ARCH §6.5.2 | 车体/炮塔/炮管插值、履带滚动、乘员位、灰盒几何与 socket | 炮塔朝向与 sim 一致 ≤ 1°；履带速度与车速匹配 | V-VEH-US-003（占位可） |
| M1-049 | `world-chunk-streaming` | world：区块流式加载 | world | M1-006, M1-035 | L | ARCH §9.3; ARCH §9.4; ARCH §9.1 | 状态机、同心环、优先级、预取、上传预算、失败占位、部署倒计时预取 | 快速穿越测试：可见"洞"= 0；上传 ≤ 1.5 ms/帧；R_near 预取 ≤ 1 s（100 Mbps） | 无 |
| M1-050 | `world-terrain-splat-material` | world：地形 splat 材质与宏观色 | world | M1-049, M0-023, M1-042 | M | ARCH §9.2 | 8 层 splat 材质数组、宏观色图、近距 triplanar、区块槽位管理 | 地形材质 GPU ≤ 1.5 ms（High）；陡坡无拉伸（人工截图） | V-NAT-007（CC0） |
| M1-051 | `world-hedgerows-props` | world：树篱、植被与道具 | world | M1-049 | L | MAPS §3.7; ARCH §9.6 | 树篱段 instancing 与线性碰撞盒、植被实例表接入、树 impostor、props 实例表 | M1 区域树篱 > 4 km 且 draw call ≤ 60；碰撞盒与 sim 一致 | V-NAT-001; V-NAT-002; V-NAT-004; V-PRP-*（灰盒可） |
| M1-052 | `world-buildings` | world：建筑与可进入室内 | world | M1-049, M1-034 | L | MAPS §3.6; MAPS §3.7; ARCH §9.7 | 建筑灰盒（含门窗洞）、enterable 室内碰撞、探针放置、基础视锥+距离剔除 | 玩家可由门洞进入并上楼；M1 区域 draw call ≤ 900（WebGL2） | V-BLD-001..003（灰盒可，GAP-V02） |
| M1-053 | `ui-framework-i18n` | UI：框架、设计 token 与 i18n | ui | M0-001, M0-003 | L | UI §2; UI §8; UI §9; ARCH §2 | Preact+signals 外壳、Plotline token、基础组件（§9.1）、i18n（en+zh-CN 键校验 CI）、字体加载与子集、DOM 覆盖层与 Canvas 小画布工具 | i18n 缺键 CI 失败；Preact 更新 ≤ 0.8 ms/帧 | V-FNT-*（OFL 字体，见 LICENSES） |
| M1-054 | `ui-boot-menu` | UI：启动加载与主菜单 | ui | M1-053, M1-006 | M | UI §4.1; UI §4.2 | 启动页（能力探测、分阶段进度、着色器预热）、主菜单 | core 包（JS+WASM+UI，br）≤ 8 MB（CI 门禁）；首屏可交互 ≤ 3 s（100 Mbps） | I-MUS-001（可选） |
| M1-055 | `ui-server-browser-team-select` | UI：服务器浏览器与阵营选择 | ui | M1-054, M1-028 | M | UI §4.3; UI §4.4; GD §2.3; GD §2.4 | 服务器列表/过滤/加入、阵营选择与平衡限制显示 | 列表刷新 ≤ 1 s；阵营失衡时受限选项禁用并给出原因 | 无 |
| M1-056 | `ui-deploy-screen` | UI：部署界面 | ui | M1-055, M1-040, M1-021 | L | UI §4.5; GD §2.5 | 最小 DeployMap 部署地图画布（出生点；M1-058 再重构为共享 MapRenderer）、小队面板、兵种与 loadout、倒计时、预取联动 | 全流程键鼠可达；非法选择被禁用；部署到出生 ≤ 2 次点击 | 无 |
| M1-057 | `ui-hud-core` | UI：核心 HUD | ui | M1-053, M1-040 | L | UI §4.6; UI §8 | 弹药/生命/姿态/罗盘/小队列表/击杀播报/据点条/命中与伤害方向/压制指示 | HUD 更新 P95 ≤ 0.8 ms/帧；信息层级符合 UI §1.2 | 无 |
| M1-058 | `ui-tactical-map` | UI：战术地图 | ui | M1-053, M1-040, M1-035 | L | UI §4.7; UI §8.4 | 重构 M1-056 的 DeployMap 为共享 MapRenderer；全屏 Canvas 地图：底图、据点/出生点/小队、标记、缩放平移 | 地图重绘符合 UI §8.4 频率；100 图标 ≤ 1 ms | 无 |
| M1-059 | `ui-downed-death` | UI：倒地与阵亡界面 | ui | M1-057, M1-016 | M | UI §4.10; GD §7 | 倒地呼救/等待救起/重生按钮/观战、击杀回放信息 | 状态切换无空白帧；重生倒计时与服务器误差 ≤ 1 s | 无 |
| M1-060 | `ui-officer-vehicle-tools` | UI：Officer/Support 放置与载具进出 | ui | M1-057, M1-019, M1-020, M1-041 | M | UI §4.12; UI §4.13; UI §4.14 | Garrison 与箱子放置全息（合法=蓝/不合法=斜纹 warn/资源不足=灰，UI_SPEC 配色）、载具进入/换座 UI | 非法位置显示斜纹 warn 并给原因；换座 ≤ 1 次按键 | 无 |
| M1-061 | `ui-pause-settings` | UI：暂停菜单与基础设置 | ui | M1-057 | M | UI §4.16; UI §4.18; UI §5 | ESC 菜单、音量/灵敏度/按键重绑定基础、画质预设占位、退出对局 | 重绑定持久化；与浏览器保留键冲突有提示（UI §5.5） | 无 |
| M1-062 | `audio-engine-core` | 音频：引擎核心 | audio | M1-006 | L | ARCH §12.1; ARCH §12.4; GAPS §3.3 | AudioContext 图、总线、HRTF Panner、复音与偷声、缺失素材静音占位与日志、编码探测 | 64 并发主线程 < 1 ms/帧；缺失需求 ID 每会话仅 1 条日志 | 无（静音占位） |
| M1-063 | `audio-weapons-world` | 音频：枪声分层、脚步与命中 | audio | M1-062, M1-040 | L | ARCH §12.2; GD §6; REF §4.12 | 枪声近/中/远交叉淡化、声速延迟（> 150 m）、脚步材质、弹着/命中、压制简版滤波、UI 提示音 | 交叉淡化曲线单测；延迟 = d/343 ±1 帧；本地开火音与预测同帧 | A-WPN-*; C-BUL-*; E-FTS-001; H-UI-*（缺则静音） |
| M1-064 | `assets-m1-integrate-visual` | 素材接入：M1 视觉素材 | assets | M1-002, M1-003, M1-046, M1-048, M1-052, M1-051 | M | SHOP §4.2; GAPS §3.5; LICENSES §4 | 将已 OK 的真素材按需求 ID 接入（角色/武器/动画/Sherman/树篱与树/道具/建筑/特效贴图），未到位保持占位并报告 | 每个已 OK 的需求 ID 状态 real；占位清单输出；无 PENDING 素材进入主分支 | V-CHR-US-001; V-CHR-GE-001; V-WPN-US-*; V-WPN-GE-*; V-ANM-*; V-VEH-US-003; V-NAT-*; V-PRP-*; V-BLD-*; V-FX-* |
| M1-065 | `assets-m1-integrate-audio-fonts` | 素材接入：M1 音频与字体 | assets | M1-063, M1-004, M1-053 | M | SHOP §4.2; GAPS §3.5 | 已 OK 的音频（枪声分层、坦克、环境、人声、UI）与字体按需求 ID 打包接入，未到位保持静音占位 | 每个已 OK 需求 ID 状态 real；音频包 ≤ 18 MB（Medium 预算）；解码探测通过 | A-WPN-*; C-BUL-*; D-TNK-001..004; E-GER-001; F-AMB-*; G-VOX-001; G-VOX-002; H-RAD-001; H-UI-002; J-IR-001; V-FNT-* |
| M1-066 | `ops-dev-stack` | 运维：本地开发栈 | ops | M1-029 | S | ARCH §3.2; ARCH §18.1 | docker-compose：gateway+server+静态服务，开发证书脚本，一键启动 | docker compose up 后浏览器可经 WT 进入对局 | 无 |
| M1-067 | `ops-staging-deploy` | 运维：预发布部署 | ops | M1-066, M1-005 | M | ARCH §18.1; ARCH §18.2; ARCH §5.17 | [人工：域名/主机/云账号] 对局主机 Docker、Let's Encrypt 与 SIGHUP 续期、R2 上传、gateway 部署 | 外网浏览器经 WT 连入；证书续期演练通过；缺人工输入则 blocked | 无 |
| M1-068 | `test-loadtest-20` | 测试：20 客户端联测 | test | M0-027, M1-024, M1-025, M1-039 | M | ARCH §17.2; ARCH §6.9 | 20 headless+tc netem（RTT 40–150 ms、2% 丢包）运行 30 min | tick_ms P99 ≤ 28 ms；下行 P50 ≤ 128 kbps、P99 ≤ 400 kbps | 无 |
| M1-069 | `test-perf-m1` | 测试：M1 性能基准 | test | M1-052, M1-051, M1-044, M0-030 | M | ARCH §13.5; ARCH §20.1 | 三个固定机位（开阔/树篱/老城）性能采集，对照 ARCH §13.5 预算表 | High（3060，1080p）P95 帧时间 ≤ 16.7 ms；Low（Iris Xe/WebGL2）≥ 30 fps；报告入 STATUS | 无 |
| M1-070 | `hist-params-review-m1` | 史实参数复核：M1 武器与 Sherman（R8） | data | M1-008, M1-009 | M | REF §6; GD §0.4; GD §6.2; GD §14.3 | 按 REF §6.2 来源逐项复核 M1 武器与 Sherman 的 historical 字段，初始→reviewed | 字段 100% 有出处或标"未核实"；冲突项写 PLAN_ISSUES | 无 |
| M1-071 | `ref-measure-m1` | 对标测量：M1 批回填与我方复测 | data | M0-032, M1-011, M1-017, M1-019, M1-023, M1-015 | M | REF §9.1; REF §7.4 | [人工：录像] 分析批 M1 项（移动/生存/压制/手雷/坦克/建造补给/UI 时序）回填 /data，并做我方复测 | 批 M1 每项有记录；我方复测误差在 REF §7.3 容限；录像缺失则 blocked | 无 |
| M1-072 | `m1-acceptance` | M1 里程碑验收与 STATUS 汇总 | test | M1-050, M1-068, M1-069, M1-064, M1-065, M1-067, M1-070, M1-071, M1-056, M1-059, M1-060, M1-061, M1-058, M1-063, M1-047 | M | MILESTONES §2; ARCH §1.6 | 核对 M1 退出标准、走演示脚本（含 20 真人测试局的组织记录）、汇总 STATUS 与 PLAN_ISSUES、列出仍为占位的需求 ID | 退出标准全部有实测数字；占位需求 ID 清单与 GAPS 决定对应 | 无 |

## 3. M2 完整 Frontline 模式（M2-001 – M2-055，共 55 个任务）

| 任务 ID | slug | 标题 | 工作流 | 前置依赖 | 规模 | 引用章节 | 一句话范围 | 关键验收指标 | 素材需求 |
|---|---|---|---|---|---|---|---|---|---|
| M2-001 | `map-carentan-dem-vectors-full` | 地图：Carentan 全图高度图与矢量 | maps | M1-031, M1-032 | L | MAPS §3.2; MAPS §1.2; MAPS §1.6; MAPS §3.5 | [人工：DEM 下载] 扩展到完整 2049² 与 32×32 区块、8 km 远景环；OSM 道路/水系/建筑全图清洗与桥梁标注 | 全图锚点高程误差 ≤ 1 m；geojson 全部通过 schema；哈希可复现 | 无 |
| M2-002 | `map-carentan-1944-edits-full` | 地图：Carentan 1944 修正（全图） | maps | M2-001, M1-033 | L | MAPS §1.5; MAPS §3.5; MAPS §7.1 | [人工] 树篱全图描摹（> 15 km）、洪泛区/沼泽多边形、去现代构筑物、terrain_edits 应用与差分报告 | 树篱总长与目标 ±15%；洪泛区与水体逻辑区一致（validate） | 无 |
| M2-003 | `map-carentan-buildings-landmarks-full` | 地图：建筑样式与地标（全图） | maps | M2-002, M1-034 | M | MAPS §3.4; MAPS §3.6; MAPS §3.7 | buildings.geojson 300–500 栋样式/levels/enterable（可进入 ≤ 25%）、landmarks.json（教堂/车站/广场/桥）灰盒定义与 asset_ref | enterable 占比 ≤ 25%；landmarks 均有 asset_ref 与占位 | V-BLD-*（灰盒可） |
| M2-004 | `map-carentan-zones-frontline` | 地图：Carentan Frontline 关卡标注 | maps | M2-003 | M | MAPS §3.6; MAPS §6.2; GD §3.3; GD §3.7 | zones.json（5 条战线）、spawns.json（HQ/Garrison/Outpost/Airhead/OP 候选）、vehicle_routes.json、artillery 占位阵地、Skirmish 子区 | validate 通过；每据点 ≥ 3 步兵入口；载具路线连通 | 无 |
| M2-005 | `map-carentan-chunks-navmesh-full` | 地图：全图区块、碰撞与导航烘焙 | maps | M2-004, M1-035, M1-036 | L | ARCH §9.1; ARCH §9.5; ARCH §16.2; MAPS §6.1 | 1024 区块视觉/碰撞/splat/植被、步兵与载具 navmesh、预计算 WebGL2 遮挡体（occluders.json） | 全图区块总计 ≤ 80 MB 原始；navmesh 烘焙 ≤ 15 min；HQ↔据点步兵/载具连通 | 无 |
| M2-006 | `data-roles-weapons-vehicles-m2` | 数据：M2 全步兵兵种、武器与基础载具 | data | M1-007, M1-008, M1-009 | M | GD §5.7; GD §5.8; GD §5.11; GD §5.12; GD §5.13; GD §6.2; GD §14.3 | Assault/AR/MG/AT/Engineer roles 与 loadout、对应武器（BAR/M1919/MG42/霰弹/火箭/地雷/炸药）、Panzer IV、Jeep/Kübel、卡车（initial） | data-validate 通过；引用完整；名额与 GD §4.4 一致 | 无 |
| M2-007 | `protocol-messages-m2` | protocol：M2 消息（建造、资源、投票、Ping） | net | M1-010 | S | ARCH §6.8; GD §16.4; GD §19.3 | Build/Resource/Ping/Vote/Marker 扩展消息与事件，版本号递增 | 往返+fuzz 通过 | 无 |
| M2-008 | `sim-frontline-mode` | sim：Frontline 模式规则 | sim | M1-018, M1-019 | L | GD §3.3; GD §3.2; GD §2.2 | 5 条战线拉锯、据点相邻锁定、扇区推进、胜负/超时（Frontline 无加时，GD §3.3.4） | 脚本化 20 min 模拟的据点状态序列与规则一致；战线单测覆盖相邻锁定 | 无 |
| M2-009 | `sim-resources-nodes` | sim：战略资源与资源节点 | sim | M2-008 | L | GD §11; BRIEF R6 | Manpower/Munitions/Fuel 整分钟结算、Engineer 资源节点、消耗与上限、与 Supplies 分离 | 整分钟结算精确到 1 单位；节点数量上限单测 | 无 |
| M2-010 | `sim-build-system` | sim：Engineer 建造系统 | sim | M1-020, M2-009 | L | GD §12.4; GD §12.6 | 建造框架（全息检测、建造时间、HP 增长、位置限制、全队上限 120）、铁丝/障碍/沙袋/散兵坑/MG 巢/资源节点 | 位置限制规则单测 100% 覆盖；nav_critical 连通检查生效 | 无 |
| M2-011 | `sim-outpost-repair-atgun` | sim：Outpost、维修站与反坦克炮 | sim | M2-010, M2-006 | M | GD §10.3; GD §12.4; GD §15.7; BRIEF R1 | Outpost（Officer 放置，R1）、Repair Station、AT 炮阵地（AT 用 Supplies 建造，R7；本任务自建 data/emplacements/ 数据） | Outpost 放置/消耗/冷却规则单测；AT 炮伤害与 /data 一致 | 无 |
| M2-012 | `sim-roles-m2` | sim：M2 兵种特有能力 | sim | M2-006, M1-013, M1-011, M2-010 | L | GD §5.7; GD §5.8; GD §5.11; GD §5.12; GD §5.13 | Assault 冲刺/翻越加成、AR 切射速、MG 部署后坐 ×0.35、AT 发射器、Engineer 工具（剪线/探雷/修理） | 各兵种特有数值与 /data 一致（单测）；MG 部署判定单测 | 无 |
| M2-013 | `sim-weapons-m2` | sim：M2 武器类（自动步枪、机枪、霰弹） | sim | M2-006, M1-013 | M | GD §6.3; GD §6.5 | BAR 双档射速、两脚架/部署、MG 持续射击与过热/换弹链、霰弹弹丸扩散 | cyclic rate 误差 ≤ 2%；霰弹 30 m 命中弹丸数与 /data 一致 | 无 |
| M2-014 | `sim-rockets-mines-explosives` | sim：火箭、地雷与炸药 | sim | M2-013, M1-015, M1-023 | L | GD §6.9; GD §14.4 | Bazooka/Panzerschreck/Panzerfaust（HEAT 弹道）、AP/AT 地雷、Satchel/集束手榴弹、对载具/建筑伤害 | HEAT 对装甲板判定与 §14.4 手算一致；地雷触发规则单测 | 无 |
| M2-015 | `sim-vehicles-m2` | sim：M2 载具（轻型车、卡车、Panzer IV）与刷新 | sim | M1-023, M2-006, M2-009 | L | GD §14.3; GD §14.6; GD §14.9 | Jeep/Kübel、CCKW/Blitz、Panzer IV、载具刷新点与资源成本/上限、载员乘坐与步兵交互 | 载具成本扣资源与上限单测；载具碾压/乘员伤害规则单测 | 无 |
| M2-016 | `sim-vehicle-modules-repair` | sim：载具模块损伤与维修 | sim | M2-015, M2-010, M2-011 | M | GD §14.5; GD §14.8 | 履带/炮塔/发动机/弹药架模块、应急维修、维修站 | 模块损伤→性能降级曲线单测；维修耗时与 /data 一致 | 无 |
| M2-017 | `sim-scoring-xp` | sim：计分与经验 | sim | M1-016, M2-008, M2-010, M2-011, M2-015, M2-016 | M | GD §18 | 得分类别、KDA、助攻、占点/救治/补给得分、经验与等级、局终统计数据 | 得分表逐类单测；等级曲线与 /data 一致 | 无 |
| M2-018 | `sim-server-rules` | sim：自动平衡、TK、投票与挂机 | sim | M2-017, M1-021 | M | GD §19; GD §2.4 | 自动平衡、友伤/TK 惩罚、投票规则、AFK 检测 | 各规则边界单测；TK 惩罚阈值与 /data 一致 | 无 |
| M2-019 | `sim-comms-markers` | sim：标记、Ping 与聊天频道规则 | sim | M1-021, M2-007 | M | GD §16; GD §17.1 | 标记权限、Ping 类型与冷却、频道（all/team/squad/command）规则 | 权限矩阵单测；Ping 速率限制生效 | 无 |
| M2-020 | `server-bandwidth-regulator` | server：带宽调节器 | server | M1-024 | M | ARCH §6.6.3; ARCH §6.9.4 | ack 率/估算带宽驱动的 PACKET_BUDGET 缩放（≥ 40%）、远场数量裁剪、HUD 通知 | 5% 丢包下下行 ≤ 68 kbps（典型）；弱网不饿死 2 Hz 下限 | 无 |
| M2-021 | `server-perf-100` | server：100 玩家 CPU 优化 | server | M2-020, M2-008 | L | ARCH §6.10 | 编码并行池、AOI 局部排序、tick 分段耗时指标、profiling 与热点优化 | 100 headless+bot：tick_ms 稳态 ≤ 18 ms、P99 ≤ 28 ms | 无 |
| M2-022 | `gateway-stats-oauth` | gateway：战绩、设置与可选 OAuth | server | M1-028 | M | ARCH §19.2; ARCH §2 (A8) | [人工：OAuth 应用注册] 战绩/设置持久化、OAuth（Discord/Google，PKCE）、游客升级 | OAuth 流程端到端通过；不保存第三方 access token（审计）；缺 OAuth 应用则仅游客路径可测并标 partial | 无 |
| M2-023 | `gateway-admin-reports` | gateway：管理 API 与举报 | server | M2-022, M1-027 | M | ARCH §15.3; GD §19.6; ARCH §19.3 | 举报入库、封禁/踢出/静音、管理员 token+IP 白名单、审计日志 | 管理操作均有审计行；非白名单 IP 拒绝 | 无 |
| M2-024 | `voice-livekit-deploy` | 语音：LiveKit 自托管与 token | voice | M1-066, M1-028 | M | ARCH §14.1; ARCH §14.3; ARCH §18.1 | LiveKit Docker 配置（关闭 egress/录制）、gateway 签发 token（TTL 15 min）、autoSubscribe=false | 两个浏览器经 LiveKit 互通音频；token 过期被拒 | 无 |
| M2-025 | `voice-subscription-manager` | 语音：服务器订阅集管理 | voice | M2-024, M1-026 | L | ARCH §14.2; GD §16.1 | 按小队/指挥/近距（≤ 50 m）计算订阅集、LiveKit Server API 增量更新、最多 8 路 | 订阅更新延迟 P95 < 300 ms（T8，实测）；100 玩家下 API 错误率 < 1% | 无 |
| M2-026 | `voice-client` | 语音：客户端接入与空间化 | voice | M2-025, M1-062, M1-040 | L | ARCH §14.4; ARCH §12.5; ARCH §14.5 | livekit-client、PTT、近距 HRTF 空间化、无线电滤波、失败降级、不做转写（R10） | 小队语音端到端延迟 < 400 ms（局域网）；LiveKit 不可用时游戏不受影响 | H-RAD-001（可选） |
| M2-027 | `bots-utility-goals-frontline` | bots：Frontline 目标与战术行为 | bots | M1-039, M2-008, M2-011, M2-010, M2-015, M2-019 | L | GD §20.3; ARCH §16.3 | 据点进攻/防守效用、Garrison/Outpost 使用、补给/救治/建造行为（Engineer/Medic/Support bot） | GD §20.3 逐条场景通过；bot 进攻成功率在平衡模拟中非 0 且非 100% | 无 |
| M2-028 | `bots-commander-rule` | bots：规则式指挥官 | bots | M2-027, M2-009, M2-010, M2-015, M2-019 | L | GD §20.4; ARCH §16.1; ARCH §16.4 | RuleCommander：资源分配、小队任务、载具呼叫、优先据点；产出与真人同构指令 | 单局内指挥官产生 ≥ 10 条合法指令；非法指令被校验层拒绝（0 漏过） | 无 |
| M2-029 | `bots-scale-100` | bots：100 bot 规模优化 | bots | M2-028 | M | ARCH §16.5; GD §20.5 | 感知缓存、思考分片、寻路预算、难度参数 | 60 bot ≤ 3.0 ms/tick；100 bot（压测）≤ 5 ms/tick | 无 |
| M2-030 | `render-gtao-ssao` | render：GTAO/SSAO | client-render | M1-044 | M | ARCH §10.2; ARCH §10.3 | WebGPU 半分辨率 GTAO+双边模糊；WebGL2 SSAO；Low 关闭 | GTAO ≤ 0.9 ms（High 参考）；开关前后截图差异报告 | 无 |
| M2-031 | `render-froxel-fog` | render：体积雾 | client-render | M1-043 | M | ARCH §10.2; ARCH §10.3 | WebGPU froxel compute（160×90×64）、Medium 降级、WebGL2 高度雾 | froxel ≤ 0.8 ms（High 参考）；WebGL2 路径可用 | 无 |
| M2-032 | `render-taau` | render：TAAU 时间性超分 | client-render | M1-044 | L | ARCH §10.2; ARCH §13.1; ARCH §13.2 | WebGPU compute TAAU（邻域 clamp、自适应锐化）、WebGL2 的 FSR1 式空间放大+轻 TAA、render_scale 接口 | 67% 渲染比例下与 100% 参考图 SSIM ≥ 0.95（固定机位）；TAAU ≤ 0.8 ms（High 参考） | 无 |
| M2-033 | `render-vfx-full` | render：烟雾、火焰、弹坑与血迹 | client-render | M1-047 | M | ARCH §13.5.3; GAPS §3.2 | 烟雾体积 flipbook、火焰、爆炸尘土、弹坑/血迹贴花、粒子 LOD 与预算 | 1600 活动粒子 ≤ 0.7 ms；贴花上限与回收单测 | V-FX-*; V-FX-005 |
| M2-034 | `world-occlusion-culling` | world：遮挡剔除 | world | M1-052, M2-005 | L | ARCH §9.7; ARCH §10.3 | WebGPU Hi-Z compute + indirect draw；WebGL2 预计算遮挡体 + CPU 低分辨率软光栅 | Hi-Z ≤ 0.5 ms（High）；老城机位 draw call 降低 ≥ 30%；WebGL2 剔除 ≤ 0.4 ms | 无 |
| M2-035 | `world-water-marsh` | world：水面与沼泽 | world | M1-049, M2-005 | M | MAPS §3.7; ARCH §9.8 | 单一水面网格、浅水涉水逻辑区与视觉解耦、运河、折射仅 High+ | 水面 ≤ 0.5 ms；涉水区速度与 /data 一致 | V-NAT-008（M4 前可灰盒） |
| M2-036 | `world-streaming-full-map` | world：全图流式与内存预算 | world | M2-005, M2-034 | M | ARCH §9.3; ARCH §13.3; ARCH §11.6 | 全图区块流式、LRU 与软预算、OPFS 配额、纹理 mip 动态降级 | 15 min 巡游：JS 堆+WASM ≤ 1.2 GB（Medium）；可见"洞"= 0 | 无 |
| M2-037 | `ui-radial-ping` | UI：径向菜单与快捷指令 | ui | M1-057, M2-019 | M | UI §4.8; GD §16.4 | 径向菜单、Ping 与快捷语句、冷却显示 | 菜单打开/选择 ≤ 200 ms；Ping 冷却与服务器一致 | 无 |
| M2-038 | `ui-scoreboard` | UI：记分板 | ui | M1-057, M2-017 | M | UI §4.9; GD §18.3 | Tab 记分板（队伍/小队/兵种/KDA/得分）、举报入口 | 100 行刷新 ≤ 1 ms；排序稳定 | 无 |
| M2-039 | `ui-build-menu` | UI：建造界面 | ui | M1-060, M2-010 | M | UI §4.13; GD §12.4 | Engineer 建造菜单、全息预览、Supplies 余量/上限提示 | 非法位置给出原因；上限/余量实时与服务器一致 | 无 |
| M2-040 | `ui-chat-voice` | UI：文字聊天与语音状态 | ui | M1-057, M2-026, M2-019 | M | UI §4.15; GD §16.2 | 聊天框/频道切换、说话指示、PTT 提示、静音/音量 | 聊天限速每 8 s 5 条（GD §16.2）并给出提示；语音失败时有降级提示 | 无 |
| M2-041 | `ui-settings-full` | UI：完整设置与可访问性基础 | ui | M1-061 | L | UI §4.16; UI §7; UI §5.4 | 图形/音频/控制/辅助功能（色盲、字幕、字号）设置、重绑定完整 | 全部设置持久化；色盲模式覆盖据点/队友/敌方标识 | 无 |
| M2-042 | `ui-result-screen` | UI：结算界面 | ui | M2-038, M2-017 | M | UI §4.17; GD §18.4 | 胜负/统计/MVP/下一局投票入口 | 数据与服务器局终统计一致；退出/再来一局流程可用 | I-MUS-002（可选） |
| M2-043 | `ui-vote-toasts` | UI：投票、举报、换队与系统提示 | ui | M1-057, M2-018 | M | UI §4.18; UI §4.19; GD §19.3 | 投票面板、举报表单、换队/换兵种、toast/事件横幅/教程提示 | 投票结果与服务器一致；toast 不遮挡准星区域（安全框） | 无 |
| M2-044 | `ui-tactical-map-frontline` | UI：战术地图的 Frontline 叠层 | ui | M1-058, M2-008, M2-009, M2-005 | M | UI §4.7; GD §3.3; GD §11 | 战线/扇区锁定、资源节点、Outpost/Garrison 图标、侦察信息分层 | 全图 5 战线渲染正确；重绘频率符合 UI §8.4 | 无 |
| M2-045 | `audio-occlusion-suppression` | 音频：遮挡低通与压制效果 | audio | M1-063, M1-017 | M | ARCH §12.3; GD §8.3 | AudioWorklet 遮挡低通（10 Hz 更新）、压制闪避+耳鸣、爆炸冲击 | Worklet 处理 ≤ 0.3 ms/128 帧；压制效果与服务器 suppression 值同步 | C-BUL-005 |
| M2-046 | `audio-reverb-ambience-vehicles` | 音频：混响总线、环境与载具声 | audio | M1-062 | M | ARCH §12.1; ARCH §12.4; ARCH §12.7 | ConvolverNode 混响（field/forest/town）、环境床、载具引擎循环与参数自动化、爆炸 | 3 套 IR 切换无爆音；载具声并发 ≤ 复音预算 | J-IR-001; J-IR-002; F-AMB-*; D-TNK-* |
| M2-047 | `assets-m2-integrate` | 素材接入：M2 批 | assets | M1-064, M1-065, M2-033, M2-035, M2-046 | M | SHOP §4.3; GAPS §3.5 | 已 OK 素材按需求 ID 接入（城镇模块、桥、废墟、道具、兵种变体、MG34/StG44 声、G-VOX、IR），占位清单更新 | 已 OK 需求 ID 状态 real；core 包仍 ≤ 8 MB；Medium 包 ≤ 150 MB（manifest 门禁） | V-BLD-004; V-BLD-008; V-BLD-010; V-PRP-005..010; V-CHR-US-003; V-WPN-*; V-ANM-003; V-ANM-006; V-ANM-007; A-WPN-012; A-WPN-013; G-VOX-003; G-VOX-004; J-IR-002; V-FX-005 |
| M2-048 | `ops-monitoring` | 运维：监控与日志 | ops | M1-067, M2-021 | M | ARCH §18.3; ARCH §18.4 | Prometheus+Grafana+Loki、ARCH §18.3 全部指标看板与告警、日志限速 | 看板含全部指标；人为制造 tick 超限触发告警 | 无 |
| M2-049 | `test-loadtest-100` | 测试：100 客户端压测（WT 与 WS） | test | M1-068, M2-021, M2-029, M2-020, M2-048 | L | ARCH §17.2; ARCH §6.9.5; ARCH §6.10 | 100 headless×30 min（WT）与 WS 兜底各一轮；netem 矩阵（RTT 40–150、抖动 20、丢包 0/2/5%）；泄漏检测 | tick_ms P99 ≤ 28 ms 且无连续 tick_overrun；下行 P50 ≤ 128 kbps、P99 ≤ 400 kbps；末 10 min RSS 增长 < 2%；WS 阈值 P99 ≤ 40 ms | 无 |
| M2-050 | `test-visual-regression` | 测试：视觉回归（Playwright） | test | M2-032, M2-036, M2-030, M2-031 | L | ARCH §17.4 | 固定机位×时段×画质档子集、固定种子/相位、pixelmatch+感知指标、WebGPU/WebGL2 基线分存、CI e2e-visual 作业 | 基线 12 机位 × 2 路径入库；差异 > 0.1% 像素时作业失败并产出报告 | 无 |
| M2-051 | `test-balance-sim-v0` | 测试：balance-sim v0 | test | M2-008, M2-028 | L | ARCH §17.3; GD §20 | balance-sim crate：无渲染"缩比对局"（默认 20v20、30 min、简化物理/粗粒度导航；单核 ≥ 10× 实时）；全尺寸 100 人局仅抽样（≥ 5 局）、并行局、输出 CSV（胜率/时长/武器占比/兵种 KDA/资源曲线） | 200 局缩比对局可在夜间作业 ≤ 4 h 内完成（单核 ≥ 10× 实时）；输出 CSV 字段齐全；同 seed 结果可复现 | 无 |
| M2-052 | `test-perf-m2` | 测试：M2 性能基准 | test | M2-036, M2-030, M2-031, M2-032, M1-069 | M | ARCH §13.5; ARCH §13.3 | 老城/树篱/沼泽/开阔四机位性能与内存采集，对照预算表 | High（3060）P95 ≤ 16.7 ms；GPU 各 pass 对照 §13.5.2；JS 堆+WASM ≤ 1.5 GB | 无 |
| M2-053 | `hist-params-review-m2` | 史实参数复核：M2 武器与载具（R8） | data | M2-006 | M | REF §6; GD §6.2; GD §14.3 | 按 REF §6.2 来源复核 M2 新增武器与载具 historical 字段 | 字段 100% 有出处或标"未核实" | 无 |
| M2-054 | `ref-measure-m2` | 对标测量：M2 批回填 | data | M1-071, M2-009, M2-010, M2-014 | M | REF §9.1 | [人工：录像与协助者] 分析批 M2 项（对局节奏/资源/建造/UI/音频）回填 /data 与复测 | 批 M2 每项有记录；复测误差在容限；缺录像则 blocked | 无 |
| M2-055 | `m2-acceptance` | M2 里程碑验收与 STATUS 汇总 | test | M2-049, M2-050, M2-051, M2-052, M2-047, M2-053, M2-054, M2-042, M2-043, M2-041, M2-040, M2-038, M2-039, M2-037, M2-044, M2-045, M2-026, M2-023, M2-016, M2-018, M2-012, M2-011, M2-021, M2-029, M2-048, M2-024, M2-025 | M | MILESTONES §3; ARCH §1.6 | 核对 M2 退出标准、走完整 Frontline 对局演示（含 100 人压测报告）、汇总 STATUS 与 PLAN_ISSUES | 退出标准全部有实测数字；压测报告附 STATUS | 无 |

## 4. M3 全兵种、全载具、Assault、指挥官能力、火炮（M3-001 – M3-054，共 54 个任务）

| 任务 ID | slug | 标题 | 工作流 | 前置依赖 | 规模 | 引用章节 | 一句话范围 | 关键验收指标 | 素材需求 |
|---|---|---|---|---|---|---|---|---|---|
| M3-001 | `data-roles-weapons-m3` | 数据：M3 全兵种补全与剩余武器 | data | M2-006 | M | GD §5.4; GD §5.14; GD §5.16; GD §5.17; GD §6.2; GD §6.3 | Commander/Tank Commander/Spotter/Sniper roles 与 loadout（GD loadout 表中出现的全部等级，至 L8；XP 公式虽到 L10，不发明 L9/L10 loadout）、G43/FG42/MG34/M1903A4/Kar98k+ZF41/M3 等剩余武器（initial） | 14 兵种 × 2 阵营 loadout 全部通过 data-validate；引用完整 | 无 |
| M3-002 | `data-vehicles-m3` | 数据：M3 其余载具（10 辆） | data | M2-006 | M | GD §14.3; GD §14.6 | 其余 10 辆载具（M8/M5A1/M4A3(76)W/Jumbo/M3 半履带/Puma/Luchs/Panther/Tiger/251）JSON，装甲/弹种/座位/成本（initial） | data-validate 通过；与 GD §14.3 表逐项一致 | 无 |
| M3-003 | `data-commander-artillery` | 数据：指挥官能力与火炮 | data | M3-001 | S | GD §13.2; GD §15.1; GD §15.5; GD §15.7 | 8 项能力（资源/冷却/延迟/范围）、榴弹炮/反坦克炮射表与散布（initial） | data-validate 通过；数值与 GD §13.2 表一致 | 无 |
| M3-004 | `map-carentan-artillery-routes-assault` | 地图：Carentan 炮兵阵地、载具路线与 Assault 测试配置 | maps | M2-004 | M | MAPS §6.2; GD §15.1; GD §3.4 | artillery.json（HQ 附近预设阵地与观测点，R7）、vehicle_routes.json 补全、zones.json 内 Assault 测试用 assault_stages（仅用于 M3 模式验证） | validate 通过；阵地到据点射程覆盖报告 | 无 |
| M3-005 | `protocol-messages-m3` | protocol：M3 消息（能力、炮兵、标记、载具座位） | net | M2-007 | M | ARCH §6.8; GD §13; GD §15 | CommanderOrder 细化、AbilityExecuted、炮兵指令/弹着事件、Spotter 标记事件、座位/武器位同步 | 往返+fuzz 通过；新增事件带宽计入 ARCH §6.9 复核 | 无 |
| M3-006 | `sim-commander-orders` | sim：指挥官席位、资源池与指令校验 | sim | M3-003, M3-005, M2-009 | L | GD §13.1; GD §13.10; GD §5.4 | Commander 席位唯一性、权限、能力通用流程（资源/冷却/延迟/授权/取消）、Command 频道权限 | 非 Commander 的能力指令全部被拒；资源不足/冷却中被拒（单测） | 无 |
| M3-007 | `sim-ability-drops` | sim：Supply Drop 与 Ammo Drop | sim | M3-006, M1-020 | M | GD §13.3 | 空投落点散布、箱子生成/寿命、与补给系统衔接 | 散布半径与 /data 一致（统计 1000 次）；箱子寿命准确 | 无 |
| M3-008 | `sim-ability-air-strikes` | sim：Bombing/Strafing/Precision | sim | M3-006, M2-014 | L | GD §13.4; GD §13.5; GD §13.6 | 飞机实体（无碰撞）、条带落弹、延迟/预警、伤害与载具/建筑结算 | 条带尺寸/弹数与 GD §13.2 一致；友军误伤规则单测 | 无 |
| M3-009 | `sim-ability-recon-rally-airhead` | sim：Recon Flight、Rally 与 Airhead | sim | M3-006, M1-019 | M | GD §13.7; GD §13.8; GD §10.4 | 侦察揭示（标记事件，R3）、Rally 范围 buff、Airhead 临时重生点（生命周期/容量） | Recon 揭示仅下发图标；Rally buff 起止与 /data 一致；Airhead 重生计时单测 | 无 |
| M3-010 | `sim-spotter-op-recon` | sim：Spotter 标记、OP 与暴露规则 | sim | M3-001, M2-019 | L | GD §10.7; GD §17; BRIEF R3 | OP 放置/出生（Spotter，R1）、精确标记、暴露规则、Sniper 隐蔽 | 暴露规则矩阵单测；被标记敌人只产生图标事件（审计） | 无 |
| M3-011 | `sim-sniper-weapons` | sim：狙击步枪与屏息 | sim | M3-001, M2-013 | M | GD §6.3; GD §6.5; GD §6.10 | 高倍镜、屏息、栓动弹道、枪口焰暴露 | 屏息时间/晃动与 /data 一致；800 m 命中判定与解析弹道一致 | 无 |
| M3-012 | `sim-roles-m3` | sim：Commander/Tank Commander/Spotter/Sniper 特有规则 | sim | M3-001, M2-012, M3-006, M3-010, M3-011 | M | GD §5.4; GD §5.14; GD §5.16; GD §5.17 | 车长位与载具认领的规则骨架（座位/运输细节在 M3-016/017 接入，此处只提供资格与接口）、望远镜、Spotter 与 Sniper 共享 OP 出生、兵种名额与等级 | 14 兵种名额表全部与 GD §4.4 一致（单测） | 无 |
| M3-013 | `sim-vehicle-armor-full` | sim：全弹种穿深与装甲判定 | sim | M1-023, M3-002 | L | GD §14.2; GD §14.4 | AP（含 APCBC/APC）/HE/HEAT/烟雾（GD §14.2；APCR 无数据，不在首发范围）、倾角/跳弹/过穿、侧裙与防盾弱点、全部载具装甲数据接入 | 40 组（弹种×车辆×部位×距离）判定与手算一致；Tiger 前装甲对 75 mm AP 的判定符合 /data | 无 |
| M3-014 | `sim-vehicles-light` | sim：轻型/侦察载具 | sim | M3-013 | M | GD §14.3 | M8/M5A1/Puma/Luchs：自动炮连发、多机枪位、高速悬挂参数 | 各车最高速与 /data（×0.75）误差 < 3%；Luchs 连发速率误差 ≤ 2% | 无 |
| M3-015 | `sim-vehicles-medium-heavy` | sim：中/重型坦克 | sim | M3-013 | M | GD §14.3; GD §14.5 | M4A3(76)W/Jumbo/Panther/Tiger：质量/转向/爬坡参数、装填时间、模块损伤差异 | 重型坦克原地转向/爬坡参数单测；装填时间误差 ≤ 1 tick | 无 |
| M3-016 | `sim-vehicle-transport-seats` | sim：运输载具座位与乘员射击 | sim | M2-015, M3-002 | M | GD §14.3; GD §14.7; GD §14.9 | 半履带/卡车座位、乘客可射击规则、补给卡车、封闭车厢限制 | 座位权限与可射击矩阵单测 | 无 |
| M3-017 | `sim-vehicle-call` | sim：载具呼叫与认领 | sim | M3-006, M2-015, M3-016 | M | GD §13.9; GD §14.6 | Commander 呼叫载具、HQ 刷新点出现（3 s）、TC 认领、成本与全队上限；Armor Crew 人数在认领时校验（GD §14.6），呼叫后 60 s 无人认领则回收并退还全部资源（GD §13.9） | 成本/上限/冷却单测；出现位置冲突处理 | 无 |
| M3-018 | `sim-artillery-howitzer` | sim：榴弹炮阵地 | sim | M3-003, M3-004, M3-006, M2-019 | L | GD §15.1; GD §15.3; GD §15.4; GD §15.5; GD §15.6 | 预设阵地（R7）、操作员流程、距离-仰角换算、弹种/弹药库存、散布与暴露/反制 | 射表落点误差与 /data 散布一致（统计 1000 发）；弹药库存规则单测 | 无 |
| M3-019 | `sim-artillery-crew-atgun` | sim：炮组操作与反坦克炮 | sim | M3-018, M2-011 | M | GD §15.3; GD §15.7 | 多操作员角色分工、炮位权限、AT 炮完整规则（回转/装填/护盾） | 权限矩阵单测；AT 炮对装甲判定与 §14.4 一致 | 无 |
| M3-020 | `sim-assault-mode` | sim：Assault 模式规则 | sim | M2-008, M3-004 | L | GD §3.4; GD §3.7 | 逐段推进、阶段目标/时限、攻守方出生与重生规则、胜负 | 用 Carentan 测试配置脚本化跑通 4 阶段；阶段超时/推进单测 | 无 |
| M3-021 | `sim-destruction-dynamic-nav` | sim：建筑破坏阶段与动态导航障碍 | sim | M2-014, M2-005 | M | MAPS §6.2; ARCH §16.2 | 建筑 ruin_stage 切换、碰撞更新、navmesh tile cache 动态障碍 | 建筑被摧毁后碰撞/导航在 1 s 内更新；不破坏确定性（state_hash 一致） | 无 |
| M3-022 | `server-vehicle-lagcomp-aoi` | server：载具延迟补偿与 AOI 优先级 | server | M1-025, M3-013, M3-005 | M | ARCH §6.7.1; ARCH §6.5.4 | 载具 OBB/炮塔回溯命中体、载具 AOI 优先级、100 玩家+40 载具编码预算 | 载具命中回溯在 150 ms RTT 下命中率与 0 延迟差 ≤ 5%；载具带宽符合 ARCH §6.9.3 | 无 |
| M3-023 | `bots-vehicle-driving` | bots：载具驾驶与乘员 bot | bots | M2-029, M3-016, M3-004 | L | ARCH §16.2; GD §20.3 | 道路图（路点）寻路、驾驶/炮手/车长 bot、乘员协同 | bot 坦克沿路线到据点成功率 ≥ 90%（10 次）；卡住自恢复 | 无 |
| M3-024 | `bots-spotter-sniper-ai` | bots：Spotter 与 Sniper bot | bots | M2-029, M3-010, M3-011 | M | GD §20.3; GD §17 | OP 选址与放置、标记敌人、狙击位选择与转移 | bot Spotter 每局放置 OP ≥ 1；狙击 bot 非全知（视锥/识别概率） | 无 |
| M3-025 | `bots-artillery-tank-crew` | bots：炮组与车长 bot | bots | M3-023, M3-019, M3-024 | M | GD §20.3; GD §15 | 炮兵 bot 响应观测点请求、车长选目标与指挥 | 炮兵 bot 在观测点请求后 ≤ 30 s 开火；射击命令全部合法 | 无 |
| M3-026 | `bots-commander-abilities` | bots：规则指挥官使用全部能力 | bots | M2-028, M3-008, M3-009, M3-007, M3-017, M3-018 | L | GD §20.4; ARCH §16.4 | 能力选择/落点/时机效用、载具呼叫策略、炮兵目标 | 一局内全部 8 种能力至少各被使用 1 次；0 条非法指令漏过校验 | 无 |
| M3-027 | `bots-llm-commander` | bots：LLM 指挥官（可选 feature） | bots | M3-026 | L | ARCH §16.4; BRIEF R4 | [人工：API key 与官方价格复核] CommanderBrain/LlmCommander、战况 JSON（仅己方可见信息）、动作枚举 schema 校验、60 s 间隔、CostBudget（≤ 0.5 美元/局，价格走配置）、超时/非法/超限回退 RuleCommander | 注入非法 JSON/超时/超限均回退且 llm_fallback_total 增加；成本记账与实际 token 一致；默认关闭 | 无 |
| M3-028 | `bots-assault-mode` | bots：Assault 模式 bot 行为 | bots | M3-026, M3-020 | M | GD §3.4; GD §20.4 | 攻/守阶段目标、集结与推进、阶段间重组 | Assault 测试配置下 bot 攻方完成阶段率在平衡模拟中非 0 且非 100% | 无 |
| M3-029 | `render-vehicles-all` | render：全部载具外观与交互 | client-render | M1-048, M3-014, M3-015, M3-016 | L | GD §14.1; ARCH §13.5.3 | 全部载具渲染（灰盒+socket）、座位乘员位、炮口焰与履带/轮式表现、LOD、模块损伤视觉 | 12 辆可见载具 ≤ 4.0 M 三角形预算内；炮塔/炮管 socket 与 sim 一致 | V-VEH-US-*; V-VEH-GE-*（灰盒可） |
| M3-030 | `render-aircraft-flyby` | render：空袭飞机与落弹表现 | client-render | M2-033, M3-008 | M | GD §13.4; GD §13.5; GAPS §3.2 | 飞机过场（灰盒十字形）、投弹/扫射表现、条带预警、与服务器 ETA 同步 | 飞机到达时刻与服务器 eta 误差 ≤ 200 ms；粒子预算内 | V-VEH-AIR-*（灰盒可） |
| M3-031 | `render-artillery-impacts` | render：炮击落点与战场环境表现 | client-render | M2-033, M3-018 | M | GD §15.5; ARCH §13.5.3 | 弹着爆炸/尘柱/弹坑贴花、远处炮击闪光、烟雾遮蔽表现 | 同屏 10 次爆炸 ≤ 0.7 ms 粒子；弹坑贴花回收生效 | V-FX-* |
| M3-032 | `render-vehicle-destruction` | render：载具击毁与残骸 | client-render | M3-029, M2-033, M2-016 | M | GD §14.5; MAPS §3.7 | 击毁燃烧、炮塔抛飞（可选）、残骸实体、模块损伤烟火 | 残骸数量上限与回收；击毁特效 ≤ 0.5 ms | V-PRP-010 |
| M3-033 | `ui-commander` | UI：指挥官界面 | ui | M2-044, M3-006, M3-007, M3-008, M3-009 | L | UI §4.11; GD §13.10 | 资源总览、能力面板、地图上选点（条带方向/半径预览）、载具呼叫、Command 频道 | 全部 8 项能力可下达；非法落点给出原因；操作 ≤ 3 次点击 | 无 |
| M3-034 | `ui-spotter-op-tools` | UI：Spotter 与 OP 工具 | ui | M1-060, M3-010 | M | UI §4.12; GD §17.1 | 望远镜精确标记、OP 放置全息、标记生命周期显示 | 标记延迟与服务器一致；OP 非法位置给原因 | 无 |
| M3-035 | `ui-vehicle-artillery` | UI：载具与火炮操作界面 | ui | M1-060, M3-018, M3-017 | L | UI §4.14; GD §15.3; GD §14.7 | 载具座位 HUD（弹种/装填/炮塔指示/损伤）、炮位界面（方位/仰角/弹种/库存）、车辆刷新点选择 | 炮位射表输入与服务器落点一致；HUD 更新 ≤ 0.8 ms | 无 |
| M3-036 | `ui-hud-vehicle-assault` | UI：载具 HUD 细节与 Assault HUD | ui | M1-057, M3-020, M3-013 | M | UI §4.6; GD §3.4 | 车内/车外 HUD 切换、模块损伤示意、Assault 阶段条/计时/目标 | 阶段切换提示与服务器 MatchPhase 同步；无遮挡准星 | 无 |
| M3-037 | `audio-vehicles-heavy` | 音频：全部载具与重武器声 | audio | M2-046, M3-014 | M | ARCH §12.1; ARCH §12.4 | 各载具引擎/履带/炮塔/主炮/被击中声事件映射与参数自动化 | 同屏 12 载具音频并发在复音预算内；缺失素材静音占位 | D-TNK-*; D-VEH-001; B-HVY-001 |
| M3-038 | `audio-explosions-artillery-air` | 音频：爆炸、炮击与空袭 | audio | M2-045, M3-008, M3-018 | M | ARCH §12.2; ARCH §12.3 | 炮声近/远与声速延迟、炮击床、空袭飞机/炸弹声、爆炸冲击 | 炮声延迟 = d/343；远炮走卷积总线（听感检查记录） | B-ART-*; B-AIR-*; B-EXP-*; D-AIR-001 |
| M3-039 | `audio-voice-lines-radio` | 音频：战场人声与无线电 | audio | M2-026, M1-062 | M | ARCH §12.5; GD §16.1 | 受击/倒地/喊话事件映射、指挥口令、无线电噪声与开关声 | 人声事件同类 50 ms 内合并规则生效；无线电滤波与频道一致 | G-VOX-*; H-RAD-001 |
| M3-040 | `voice-command-vehicle-channels` | 语音：指挥与车组频道 | voice | M2-025, M3-006, M3-016 | M | ARCH §14.1; ARCH §14.2; GD §16.1 | Command 频道订阅、车组频道、频道优先级与 8 路上限 | 车内 3 人互通且不泄漏给车外；Command 仅 Commander+Officer | 无 |
| M3-041 | `assets-m3-integrate-vehicles` | 素材接入：M3 载具 | assets | M2-047, M3-029 | M | SHOP §4.4.5; GAPS §3.5 | 已 OK 的载具模型（含 GAP-V01 的替代方案）按需求 ID 接入并设 socket，占位清单更新 | 已 OK 需求 ID 状态 real；Medium 包仍 ≤ 150 MB（manifest 门禁） | V-VEH-US-*; V-VEH-GE-* |
| M3-042 | `assets-m3-integrate-weapons-chars` | 素材接入：M3 武器与角色 | assets | M2-047, M3-011 | M | SHOP §4.4.2; SHOP §4.4.3; GAPS §3.5 | 已 OK 的剩余武器、伞兵/坦克兵角色、指挥官动画接入 | 已 OK 需求 ID 状态 real；占位清单更新 | V-WPN-US-*; V-WPN-GE-*; V-CHR-GE-002; V-CHR-GE-004; V-ANM-010 |
| M3-043 | `assets-m3-integrate-audio` | 素材接入：M3 音频 | assets | M2-047, M3-037, M3-038, M3-039 | M | SHOP §4.4.6; GAPS §3.5 | 已 OK 音频接入（重武器/炮/空袭/地雷/Foley/配乐/IR；需求 ID：I-MUS-*、J-IR-002），缺失保持静音 | 已 OK 需求 ID 状态 real；音频总量符合预算 | A-WPN-*; B-*; D-*; E-ACT-001; F-AMB-*; I-MUS-*; J-IR-002 |
| M3-044 | `assets-m3-integrate-air-artillery` | 素材接入：飞机与火炮模型 | assets | M2-047, M3-030, M3-031 | M | SHOP §4.4.5; GAPS §3.5 | 已 OK 的飞机/榴弹炮/反坦克炮模型接入与 socket | 已 OK 需求 ID 状态 real | V-VEH-ART-*; V-VEH-AIR-*; V-BLD-009; V-SKY-003 |
| M3-045 | `test-loadtest-100-full` | 测试：M3 全功能 100 客户端压测 | test | M2-049, M3-022, M3-026, M3-028, M3-021 | M | ARCH §17.2; ARCH §6.9.3 | 40 载具+全部能力+Assault 模式下 100 客户端 30 min（WT 与 WS） | tick_ms P99 ≤ 28 ms；密集场景下行 P99 ≤ 400 kbps；无泄漏 | 无 |
| M3-046 | `test-balance-sim-m3` | 测试：M3 平衡模拟 | test | M2-051, M3-026, M3-028, M3-023 | L | ARCH §17.3; GD §21 | balance-sim 缩比对局（默认 20v20、30 min）200 局×（Frontline/Assault），另加全尺寸 100 人抽样 ≥ 5 局、输出胜率/时长/武器击杀占比/兵种 KDA/载具击杀比 | Frontline 阵营胜率 40–60%、Assault 攻方胜率 35–65%；无武器击杀占比 > 25% 或 < 0.5%（如越界写入调参清单） | 无 |
| M3-047 | `test-balance-tuning-pass` | 数据：M3 首轮数值调参 | data | M3-046 | M | GD §21; REF §7.3 | 依据平衡模拟与对标测量调整 /data 初值（保留变更日志与前后对比） | 调参后重跑平衡模拟，越界项清零或登记 PLAN_ISSUES；每处改动附前后对比 | 无 |
| M3-048 | `test-visual-regression-m3` | 测试：M3 视觉回归扩充 | test | M2-050, M3-029, M3-030, M3-031, M3-032 | M | ARCH §17.4 | 增加载具/空袭/炮击机位，更新基线 | 新增 ≥ 8 机位基线入库；CI 差异阈值生效 | 无 |
| M3-049 | `test-perf-m3` | 测试：M3 性能基准 | test | M2-052, M3-029, M3-032, M3-031, M3-030 | M | ARCH §13.5 | 载具密集/炮击/空袭三场景性能采集 | High P95 ≤ 16.7 ms；同屏 12 载具 draw call ≤ 1 200 | 无 |
| M3-050 | `ops-ci-nightly-jobs` | 运维：夜间作业 | ops | M2-049, M2-051, M2-050 | S | ARCH §17.5 | loadtest/balance/e2e-visual 夜间作业、报告归档与失败通知 | CI 夜间作业配置完成且首次运行成功、报告可下载；连续 3 晚稳定性在 M3-054 验收时复核 | 无 |
| M3-051 | `hist-params-review-vehicles` | 史实参数复核：全部载具与剩余武器（R8） | data | M3-002, M3-001 | L | REF §6; GD §14.3; GD §14.4; GD §6.2 | 按 REF §6.2 来源逐项复核 /data 全部载具（装甲/火炮/质量/路速）与剩余武器，初始→reviewed | 全部载具与武器 historical 字段 100% 有出处或标"未核实"；冲突项登记 PLAN_ISSUES | 无 |
| M3-052 | `ref-measure-m3` | 对标测量：M3 批回填（1/4：载具机动与炮塔 + Assault 时间窗） | data | M2-054, M3-013, M3-018, M3-008 | M | REF §9.1 | [人工：录像] 分析批 M3 的 V-01–V-14（载具机动/转向/爬坡/炮塔/装填/弹种切换）与 R-12 共 15 项回填与复测；其余 52 项由 M3-052b/c/d 续作（批 M3 共 67 项，按测量类别均分） | 15 项每项有记录；复测误差在容限；缺录像则 blocked | 无 |
| M3-052b | `ref-measure-m3b` | 对标测量：M3 批回填（2/4：载具火力、乘员与损伤） | data | M3-052 | M | REF §9.1; REF §4.6 | [人工：录像] 续 M3-052：分析批 M3 的 V-15–V-27（弹种初速与下坠、后坐、机枪、换座、上下车、视野、车组、碾压、装甲受击、维修、成本、残骸）回填与复测 | 13 项每项有记录与 summary.csv 行；复测误差在 REF §7.3 容限；缺录像则 blocked | 无 |
| M3-052c | `ref-measure-m3c` | 对标测量：M3 批回填（3/4：指挥官能力与音频时序） | data | M3-052b | M | REF §9.1; REF §4.8; REF §4.12 | [人工：录像] 续 M3-052：分析批 M3 的 C-01–C-15（指挥官能力延迟/持续/冷却/范围/成本等）与 AU-09–AU-12 回填与复测 | 19 项每项有记录与 summary.csv 行；复测误差在容限；缺录像则 blocked | 无 |
| M3-052d | `ref-measure-m3d` | 对标测量：M3 批回填（4/4：火炮、狙击/特殊武器、压制复核） | data | M3-052c, M3-011 | M | REF §9.1; REF §4.9 | [人工：录像] 续 M3-052：分析批 M3 的 A-01–A-13（火炮）、W-13/W-27–W-29/W-36（狙击/特殊武器补测）、U-02/U-13（压制复核）回填与复测 | 20 项每项有记录与 summary.csv 行；复测误差在容限（致死发数精确相等）；缺录像则 blocked | 无 |
| M3-053 | `ui-i18n-zh-complete` | UI：简体中文文案与 CJK 字体完整性 | ui | M3-033, M3-035, M3-034, M3-036 | M | UI App.A; ARCH §2 (A7) | 补全全部 zh-CN 文案、CJK 字体子集覆盖检查、长文本溢出检查 | 两种语言键 100% 对齐（CI）；常用 3 500 字覆盖；最长文本不溢出（视觉回归） | V-FNT-* |
| M3-054 | `m3-acceptance` | M3 里程碑验收与 STATUS 汇总 | test | M3-045, M3-047, M3-048, M3-049, M3-050, M3-051, M3-052, M3-053, M3-041, M3-042, M3-043, M3-044, M3-027, M3-025, M3-024, M3-040, M3-032, M3-012, M3-007, M3-052b, M3-052c, M3-052d | M | MILESTONES §4; ARCH §1.6 | 核对 M3 退出标准、演示脚本（14 兵种/全载具/8 能力/Assault/火炮）、汇总 STATUS 与 PLAN_ISSUES | 退出标准全部有实测数字；LLM 指挥官成本与回退演示记录 | 无 |

## 5. M4 Omaha Beach、Foy、Skirmish、画质与优化、公开测试（M4-001 – M4-046，共 46 个任务）

| 任务 ID | slug | 标题 | 工作流 | 前置依赖 | 规模 | 引用章节 | 一句话范围 | 关键验收指标 | 素材需求 |
|---|---|---|---|---|---|---|---|---|---|
| M4-001 | `map-omaha-sources-geo` | 地图：Omaha 数据源、许可证与锚点对账 | maps | M1-030 | M | MAPS §4.3; MAPS §2; MAPS §0.4 | [人工] 核实海岸 LiDAR/测深/历史图许可证并登记，锚点相似变换，meta.json | 每个数据源有许可证快照与状态；锚点 ≥ 8 个；人工项列入 NEEDS_HUMAN | 无 |
| M4-002 | `map-omaha-dem-vectors` | 地图：Omaha 高度图、海岸与矢量 | maps | M4-001, M2-001 | L | MAPS §4.2; MAPS §4.5; MAPS §1.2 | [人工：DEM/测深下载] 沙滩/悬崖/海岸线高度图、潮线、道路/建筑/村镇矢量、远景环 | 锚点高程误差 ≤ 1 m；海岸线与历史图对账报告 | 无 |
| M4-003 | `map-omaha-obstacles-wn` | 地图：滩头障碍与防御工事摆放 | maps | M4-002 | M | MAPS §4.4; MAPS §4.6 | [人工：潮位与 WN 位置史料校对] 障碍带、WN 工事、铁丝网、地雷区点位与 landmarks.json | 每个 WN/障碍组有史料出处或"未核实"标注；点位均在可玩区 | V-BLD-006; V-BLD-007（灰盒可） |
| M4-004 | `map-omaha-zones-assault` | 地图：Omaha Assault 关卡标注 | maps | M4-003, M3-004 | M | MAPS §4.6; GD §3.4; GD §3.7 | zones.json（assault_stages）、spawns.json（含登陆艇 boat 类出生）、artillery.json、载具路线 | validate 通过；每阶段目标可达；攻守出生点互不可视 | 无 |
| M4-005 | `map-omaha-chunks-navmesh` | 地图：Omaha 区块、碰撞与导航烘焙 | maps | M4-004, M2-005 | M | ARCH §9.1; ARCH §16.2; MAPS §6.1 | 区块切片、splat 规则（沙/卵石/草）、碰撞、navmesh、occluders | 滩头→崖顶导航连通；区块体积 ≤ Carentan 同量级 | 无 |
| M4-006 | `map-foy-sources-license` | 地图：Foy 数据源许可证与锚点对账 | maps | M1-030 | M | MAPS §5.3; MAPS §2; MAPS §7.2 | [人工] 瓦隆 LiDAR 等许可证核实存档与登记，锚点相似变换，meta.json | 许可证状态明确（OK/PENDING）；PENDING 不进主分支 | 无 |
| M4-007 | `map-foy-dem-vectors` | 地图：Foy 高度图与矢量 | maps | M4-006, M2-001 | L | MAPS §5.2; MAPS §5.5; MAPS §1.2 | [人工：DEM 下载] 1 m 高度图、森林/道路/村庄矢量、去现代构筑物、远景环 | 锚点高程误差 ≤ 1 m；哈希可复现 | 无 |
| M4-008 | `map-foy-forests-foxholes` | 地图：Foy 森林与散兵坑布置 | maps | M4-007 | M | MAPS §5.4; MAPS §5.6 | [人工] 森林密度/树种分布、预设散兵坑与工事点位、雪地 splat 规则 | 森林区植被实例上限符合 ARCH §9.6；点位通过 validate | V-BLD-005; V-NAT-003（灰盒可） |
| M4-009 | `map-foy-zones` | 地图：Foy 关卡标注（Frontline 与 Assault） | maps | M4-008, M2-004 | M | MAPS §5.6; GD §3.3; GD §3.4; GD §3.7 | zones.json（Frontline 战线与 Assault 阶段（美军进攻，1945-01-13 506th PIR 攻 Foy，MAPS §5.6））、spawns.json、artillery、载具路线 | validate 通过；两种模式各有完整配置 | 无 |
| M4-010 | `map-foy-chunks-navmesh` | 地图：Foy 区块、碰撞与导航烘焙 | maps | M4-009, M2-005 | M | ARCH §9.1; ARCH §16.2; MAPS §6.1 | 区块切片、碰撞、步兵/载具 navmesh、occluders | 村庄↔森林导航连通；烘焙 ≤ 15 min | 无 |
| M4-011 | `sim-landing-craft-spawn` | sim：登陆艇出生与滩头规则 | sim | M4-004, M3-020, M1-019 | M | GD §10.1; MAPS §4.6; GD §3.4 | boat 出生点类型（航行/抵滩/卸载）、登陆艇生命周期、滩头出生保护 | 登陆艇到岸时刻与配置一致（±1 s）；出生保护单测 | V-VEH-*（灰盒可） |
| M4-012 | `sim-skirmish-mode` | sim：Skirmish 模式规则 | sim | M2-008 | M | GD §3.5; GD §3.7 | 单据点小规模规则、Skirmish 子区、胜负/回合 | 脚本化模拟一局胜负正确；与 Frontline 共用 sim 代码路径（无分叉） | 无 |
| M4-013 | `map-skirmish-areas` | 地图：Skirmish 子区配置 | maps | M2-004, M4-009, M4-012, M4-004, M4-005 | S | MAPS §6.2; GD §3.5 | Carentan 与 Foy 的 skirmish_areas、HQ/Garrison 子区出生点 | validate 通过；子区出生互不可视 | 无 |
| M4-014 | `world-ocean-beach` | world：海洋、海岸与滩头 | world | M2-035, M4-005 | L | ARCH §9.8; MAPS §4.5 | Gerstner 简化海面（大尺度）、潮线/湿沙材质、浪花、远景海平线、水下浅滩逻辑 | 海面 ≤ 0.3 ms（High）；潮线与高度图一致无穿帮 | V-NAT-006; V-NAT-008 |
| M4-015 | `world-winter-snow-weather` | world：冬季雪地与天气 | world | M4-010, M2-036 | L | MAPS §5.5; ARCH §9.8 | 雪地材质（splat+积雪遮盖）、云杉植被积雪、降雪/雾粒子、冬季天空/光照预设、脚印（可选） | 降雪粒子 ≤ 0.7 ms；植被材质变体仍 ≤ 60 | V-NAT-003; V-SKY-002; V-FX-* |
| M4-016 | `bots-skirmish-omaha-foy` | bots：新地图与新模式 bot 适配 | bots | M3-028, M4-011, M4-012, M4-005, M4-010 | M | GD §20.3; GD §20.4 | 登陆艇/滩头/森林/雪地行为参数、Skirmish 目标、新地图 off-mesh 链接 | 三图×三模式各完成 1 局 bot 对局无卡死（> 30 s 无进展计数 = 0） | 无 |
| M4-017 | `render-quality-tiers` | render：四档画质预设 | client-render | M2-032, M2-030, M2-031 | L | ARCH §13.1; ARCH §13.5.3 | Low/Medium/High/Ultra 参数表落地（阴影/AO/雾/植被/粒子/mip/视距/动态灯）、预设数据化、热切换 | 四档截图与参数表一致；高档参数不影响 Low 的变体数（≤ 40） | 无 |
| M4-018 | `render-dynamic-resolution-autotune` | render：动态分辨率与自动推荐 | client-render | M4-017 | M | ARCH §13.2; ARCH §13.6 | GPU 时间驱动 render_scale、5 s 基准场景与档位推荐、降档提示 | 压力场景下帧时间回到目标 ≤ 3 s；推荐不自动降档（单测） | 无 |
| M4-019 | `render-ssr-ultra` | render：Ultra 屏幕空间反射 | client-render | M4-017 | M | ARCH §10.2; ARCH §13.1 | Hi-Z SSR 仅水面/金属车体、IBL 回退 | Ultra 4070 级 SSR ≤ 1.0 ms；关闭时无残留开销 | 无 |
| M4-020 | `render-webgl2-low-path` | render：Low 档与 WebGL2 路径优化 | client-render | M4-017, M2-034 | L | ARCH §10.3; ARCH §13.5.3; ARCH §13.1 | WebGL2 三级阴影、合批/冻结材质、遮挡体剔除、FSR1 式放大、粒子/植被/动态灯削减 | Iris Xe 720p WebGL2 ≥ 30 fps（P95）；draw call ≤ 500；可见三角形 ≤ 0.8 M | 无 |
| M4-021 | `render-shader-variant-gate` | render：着色器变体与预热门禁 | client-render | M1-042, M4-017 | S | ARCH §10.4; ARCH §20 (4) | 变体计数 CI 门禁（≤ 60/Low ≤ 40）、预热覆盖率检查、首次出现卡顿检测 | 门禁生效；预热覆盖率 100%（运行时记录） | 无 |
| M4-022 | `render-vram-mip-budgets` | render：显存与纹理 mip 预算 | client-render | M4-017, M2-036 | M | ARCH §13.3; ARCH §13.4; ARCH §9.3 | 自估计显存预算、按档最大 mip、LRU 驱逐与 mip 动态降级 | 15 min 巡游显存估计 ≤ 档位预算（Low 1.5/Med 2.5/High 4/Ultra 6 GB）；无"洞" | 无 |
| M4-023 | `ui-modes-maps-polish` | UI：模式/地图选择与加载界面更新 | ui | M1-055, M4-012, M4-011 | M | UI §4.3; UI §4.4; UI §4.1 | 服务器浏览器的模式/地图过滤、三图加载页与简报、登陆艇出生界面 | 三种模式×三张图的进入流程全部可达；加载页进度准确 | 无 |
| M4-024 | `ui-quality-benchmark-settings` | UI：画质设置与基准向导 | ui | M2-041, M4-018 | M | UI §4.16; ARCH §13.6 | 四档选择、动态分辨率开关、首次运行 5 s 基准与推荐、高级项 | 档位切换 ≤ 2 s 生效；推荐结果与档位表一致 | 无 |
| M4-025 | `ui-accessibility-audit` | UI：可访问性审计与修复 | ui | M2-041 | M | UI §7; UI §10 | 色盲模式、字幕/声音可视化、字号/布局、减少晃动、对比度审计与修复 | UI 文本对比度 ≥ 4.5:1（脚本检查）；全部交互键盘可达 | 无 |
| M4-026 | `ui-responsive-ultrawide` | UI：缩放、安全区与超宽屏测试 | ui | M1-057, M2-041 | S | UI §6.1; UI §6.4; UI §6.5 | 测试矩阵（16:9/21:9/1366×768…必测；16:10、32:9 为可选，UI_SPEC §6 无专门规则）截图与修复 | UI §6.5 矩阵全部通过；HUD 不越出安全框 | 无 |
| M4-027 | `ui-tutorial-onboarding` | UI：新手引导 | ui | M2-043, M1-056 | M | UI §4.19; GD §2 | 首次进入的操作提示（移动/射击/部署/小队/占点）、可跳过、可在设置重开 | 新玩家首局 5 分钟内完成部署+占点引导；提示不遮挡准星 | 无 |
| M4-028 | `audio-map-tuning` | 音频：分地图混响与环境调校 | audio | M2-046, M4-014, M4-015 | M | ARCH §12.2; ARCH §12.7 | 海滩/林地/雪地的混响 IR 与环境床选择、尾音类型映射 | 三图各通过听感检查清单（记录）；音频包体积符合各档预算 | F-AMB-002; F-AMB-004; J-IR-* |
| M4-029 | `assets-m4-integrate-omaha-foy` | 素材接入：M4 Omaha/Foy | assets | M3-041, M4-003, M4-008, M4-014, M4-015 | M | SHOP §4.5; GAPS §3.5 | 已 OK 的碉堡/障碍/冬季建筑与植被、冬季角色、海滩材质、天空与环境声接入 | 已 OK 需求 ID 状态 real；High 包 ≤ 350 MB、Ultra ≤ 600 MB（manifest 门禁） | V-BLD-005..007; V-NAT-003; V-NAT-006; V-NAT-008; V-SKY-002; V-CHR-US-004; V-CHR-GE-003; F-AMB-002; F-AMB-004; I-PD-001 |
| M4-030 | `server-anticheat-statistics` | server：统计异常检测与回放 | server | M1-027, M2-023 | L | ARCH §15.3; ARCH §15.4 | 滚动窗口指标（爆头率/命中率/反应时间/转向峰值）、z-score 评分、10 min 输入回放记录、复核队列 API | 注入合成自瞄输入（headless）被评分排入前 1%；正常 bot 误报率 < 1%；回放 ≤ 200 KB/玩家 | 无 |
| M4-031 | `gateway-matchmaking` | gateway：对局编排与匹配 | server | M1-029, M4-012, M4-011 | L | ARCH §3.3; ARCH §18.1; ARCH §5.17 | gateway 内置主机代理（不新增 crate）启动/回收对局进程、地图/模式轮换、bot 填充配置、崩溃心跳处理 | 无人工干预连续跑 8 局；进程崩溃后玩家回到列表 ≤ 10 s | 无 |
| M4-032 | `server-security-hardening` | 安全：加固与审计 | server | M4-030, M2-023 | M | ARCH §19; ARCH §17.1 | 解码 fuzz 长跑、CSP/SRI、速率限制与连接限制审计、依赖审计（cargo-deny/npm audit）、字符串过滤与 UI 注入检查 | fuzz 累计 ≥ 1 h 无 panic；CSP 无 unsafe-inline；npm/cargo 审计无高危未处理 | 无 |
| M4-033 | `ops-prod-deploy` | 运维：生产部署 | ops | M1-067, M4-031, M2-048 | M | ARCH §18.1; ARCH §18.2; ARCH §5.17 | [人工：生产主机/域名/CDN] 生产对局主机、gateway、LiveKit、R2+CDN、证书监控（到期前 14 天告警） | 外网全流程可玩；证书续期演练；缺人工输入则 blocked | 无 |
| M4-034 | `ops-alerts-runbook-release` | 运维：告警、运行手册与发布回滚 | ops | M2-048, M4-033 | M | ARCH §18.3; ARCH §18.5 | 告警规则落地、事故运行手册、蓝绿发布与回滚演练、协议/数据哈希兼容流程 | 回滚演练 ≤ 10 min；告警规则均有触发演练记录 | 无 |
| M4-035 | `ops-privacy-gdpr` | 运维：隐私与合规（数据导出/删除） | ops | M2-022, M4-033 | M | ARCH §19.4; ARCH §19.2 | [人工：法务审阅] 数据导出/删除端点、隐私政策与本地存储说明草稿、聊天/IP 保留期实现 | 导出/删除端点端到端通过；保留期清理作业运行记录 | 无 |
| M4-036 | `ops-telemetry-optin` | 运维：匿名遥测（首次同意） | ops | M4-035, M4-017 | S | ARCH §18.3; ARCH §19.4 | 帧时间分布/画质档/传输类型/着色器编译耗时的匿名上报，默认关闭或首次明确同意 | 未同意时零上报（抓包验证）；上报不含 IP/麦克风/硬件指纹 | 无 |
| M4-037 | `ops-public-test-prep` | 运维：公开测试准备 | ops | M4-033, M4-034, M4-035 | M | ARCH §18.1; MILESTONES §6 | 容量与成本方案、落地页与状态页、反馈入口、已知问题清单、发布公告草稿（不含参考作品名称/截图） | 容量方案与 100 人压测数据对应；公告通过 BRIEF §1 知识产权自查 | 无 |
| M4-038 | `test-loadtest-soak` | 测试：三图 60 分钟浸泡压测 | test | M3-045, M4-031, M4-014, M4-015 | M | ARCH §17.2; ARCH §18.3 | 100 headless×60 min×三张图（WT 与 WS），含重连与断线注入 | tick_ms P99 ≤ 28 ms；连续 overrun = 0；RSS 末 10 min 增长 < 2%；连接保持率 ≥ 99% | 无 |
| M4-039 | `test-balance-all-maps` | 测试：三图三模式平衡模拟 | test | M3-047, M4-016 | M | ARCH §17.3; GD §21 | 三图×可用模式 各 200 局缩比对局、对比报告、调参清单 | Frontline 各图阵营胜率 40–60%、Assault 攻方胜率 35–65%（缩比对局口径，全尺寸抽样 ≥ 5 局）；越界项登记 | 无 |
| M4-040 | `test-visual-regression-all` | 测试：三图视觉回归与四档画质 | test | M3-048, M4-014, M4-015, M4-017 | M | ARCH §17.4 | 三图×12 机位×3 时段×四档子集基线 | 基线入库；CI 差异阈值生效；夜间 job 报告 | 无 |
| M4-041 | `test-perf-matrix` | 测试：四档画质性能矩阵 | test | M3-049, M4-020, M4-022, M4-014, M4-015 | L | ARCH §13.1; ARCH §13.5; ARCH §13.4 | [人工：参考硬件] 在 Iris Xe/GTX 1650/RTX 3060/RTX 4070 上测三图性能与内存；回填 ARCH §13.5 预算偏差 | Low 720p ≥ 30 fps；Medium 1080p 45–60 fps；High 3060 1080p ≥ 60 fps；Ultra 4070 1440p ≥ 60 fps（P95）；缺硬件项标 partial | 无 |
| M4-042 | `test-browser-matrix` | 测试：浏览器兼容矩阵 | test | M0-028, M4-020, M4-017 | M | ARCH §5.0; ARCH §20 (6) | Chromium/Edge/Firefox/Safari 的 WebGPU/WebGL2、WT/WS、音频编码探测、语音的实测矩阵 | 矩阵入 STATUS；Chromium 全功能通过；其余浏览器问题登记并标优先级 | 无 |
| M4-043 | `hist-params-final-sweep` | 史实参数最终清扫（R8） | data | M3-051, M3-047 | S | REF §6; GD §0.4 | 全部 /data 条目的 historical 状态统计，剩余 initial/"未核实"逐项处理或登记 | /data 中无状态为 initial 的 historical 字段（或有 PLAN_ISSUES 编号） | 无 |
| M4-044 | `ref-measure-m4-recheck` | 对标测量：复测与版本漂移处理 | data | M3-052, M4-043, M3-052b, M3-052c, M3-052d | M | REF §7.4; REF §7.6; REF §9.1 | [人工：参考作品新版本抽样录像] locked 项抽样复测、我方复测、漂移说明 | 抽样项误差在容限；漂移项更新或登记 | 无 |
| M4-045 | `test-closed-beta` | 测试：封闭测试执行与反馈汇总 | test | M4-037, M4-038, M4-040, M4-041, M4-042 | M | MILESTONES §6; ARCH §20 | [人工：招募测试者] 邀请制测试局、问卷/崩溃/性能遥测汇总、缺陷分级 | 完成 ≥ 3 局 ≥ 60 人的测试局；缺陷清单分级入 PLAN_ISSUES | 无 |
| M4-046 | `m4-acceptance` | M4 里程碑验收与 STATUS 汇总 | test | M4-045, M4-039, M4-044, M4-029, M4-028, M4-024, M4-025, M4-026, M4-027, M4-023, M4-019, M4-018, M4-021, M4-036, M4-032, M4-013, M4-030, M4-031, M4-033, M4-034, M4-035, M4-037 | M | MILESTONES §5; ARCH §1.6 | 核对 M4 退出标准、三图×三模式演示、汇总 STATUS 与 PLAN_ISSUES，给出公开测试 Go/No-Go 清单 | 退出标准全部有实测数字；Go/No-Go 清单每项有证据 | 无 |

## 6. 统计

| 里程碑 | 任务数 | ID 范围 |
|---|---|---|
| M0 | 33 | M0-001 – M0-033 |
| M1 | 72 | M1-001 – M1-072 |
| M2 | 55 | M2-001 – M2-055 |
| M3 | 54 | M3-001 – M3-054 |
| M4 | 46 | M4-001 – M4-046 |
| 合计 | 260 | — |

规模分布：S 16 / M 158 / L 86。

按工作流 × 里程碑：

| 工作流 | M0 | M1 | M2 | M3 | M4 | 合计 |
|---|---|---|---|---|---|---|
| repo | 2 | 0 | 0 | 0 | 0 | 2 |
| sim | 6 | 13 | 12 | 16 | 2 | 49 |
| net | 4 | 3 | 1 | 1 | 0 | 9 |
| server | 5 | 6 | 4 | 1 | 3 | 19 |
| client-render | 4 | 8 | 4 | 4 | 6 | 26 |
| ui | 0 | 9 | 8 | 5 | 5 | 27 |
| audio | 0 | 2 | 2 | 3 | 1 | 8 |
| world | 0 | 4 | 3 | 0 | 2 | 9 |
| assets | 1 | 7 | 1 | 4 | 1 | 14 |
| bots | 1 | 3 | 3 | 6 | 1 | 14 |
| voice | 0 | 0 | 3 | 1 | 0 | 4 |
| test | 5 | 3 | 5 | 5 | 7 | 25 |
| ops | 0 | 2 | 1 | 1 | 5 | 9 |
| data | 4 | 5 | 3 | 6 | 2 | 20 |
| maps | 1 | 7 | 5 | 1 | 11 | 25 |

## 7. 需要人类输入的任务（潜在 blocked）

| 任务 ID | slug | 所需人工输入（摘自范围列） |
|---|---|---|
| M0-032 | `ref-measure-m0` | [人工：录像] |
| M1-030 | `map-carentan-sources-geo` | [人工] |
| M1-031 | `map-carentan-dem-third` | [人工：DEM 下载] |
| M1-033 | `map-carentan-1944-edits-third` | [人工] |
| M1-067 | `ops-staging-deploy` | [人工：域名/主机/云账号] |
| M1-071 | `ref-measure-m1` | [人工：录像] |
| M2-001 | `map-carentan-dem-vectors-full` | [人工：DEM 下载] |
| M2-002 | `map-carentan-1944-edits-full` | [人工] |
| M2-022 | `gateway-stats-oauth` | [人工：OAuth 应用注册] |
| M2-054 | `ref-measure-m2` | [人工：录像与协助者] |
| M3-027 | `bots-llm-commander` | [人工：API key 与官方价格复核] |
| M3-052 | `ref-measure-m3` | [人工：录像] |
| M3-052b | `ref-measure-m3b` | [人工：录像] |
| M3-052c | `ref-measure-m3c` | [人工：录像] |
| M3-052d | `ref-measure-m3d` | [人工：录像] |
| M4-001 | `map-omaha-sources-geo` | [人工] |
| M4-002 | `map-omaha-dem-vectors` | [人工：DEM/测深下载] |
| M4-003 | `map-omaha-obstacles-wn` | [人工：潮位与 WN 位置史料校对] |
| M4-006 | `map-foy-sources-license` | [人工] |
| M4-007 | `map-foy-dem-vectors` | [人工：DEM 下载] |
| M4-008 | `map-foy-forests-foxholes` | [人工] |
| M4-033 | `ops-prod-deploy` | [人工：生产主机/域名/CDN] |
| M4-035 | `ops-privacy-gdpr` | [人工：法务审阅] |
| M4-041 | `test-perf-matrix` | [人工：参考硬件] |
| M4-044 | `ref-measure-m4-recheck` | [人工：参考作品新版本抽样录像] |
| M4-045 | `test-closed-beta` | [人工：招募测试者] |

## 8. 素材接入任务（真正依赖素材到位）

| 任务 ID | slug | 依赖的素材需求 ID |
|---|---|---|
| M1-064 | `assets-m1-integrate-visual` | V-CHR-US-001; V-CHR-GE-001; V-WPN-US-*; V-WPN-GE-*; V-ANM-*; V-VEH-US-003; V-NAT-*; V-PRP-*; V-BLD-*; V-FX-* |
| M1-065 | `assets-m1-integrate-audio-fonts` | A-WPN-*; C-BUL-*; D-TNK-001..004; E-GER-001; F-AMB-*; G-VOX-001; G-VOX-002; H-RAD-001; H-UI-002; J-IR-001; V-FNT-* |
| M2-047 | `assets-m2-integrate` | V-BLD-004; V-BLD-008; V-BLD-010; V-PRP-005..010; V-CHR-US-003; V-WPN-*; V-ANM-003; V-ANM-006; V-ANM-007; A-WPN-012; A-WPN-013; G-VOX-003; G-VOX-004; J-IR-002; V-FX-005 |
| M3-041 | `assets-m3-integrate-vehicles` | V-VEH-US-*; V-VEH-GE-* |
| M3-042 | `assets-m3-integrate-weapons-chars` | V-WPN-US-*; V-WPN-GE-*; V-CHR-GE-002; V-CHR-GE-004; V-ANM-010 |
| M3-043 | `assets-m3-integrate-audio` | A-WPN-*; B-*; D-*; E-ACT-001; F-AMB-* |
| M3-044 | `assets-m3-integrate-air-artillery` | V-VEH-ART-*; V-VEH-AIR-*; V-BLD-009; V-SKY-003 |
| M4-029 | `assets-m4-integrate-omaha-foy` | V-BLD-005..007; V-NAT-003; V-NAT-006; V-NAT-008; V-SKY-002; V-CHR-US-004; V-CHR-GE-003; F-AMB-002; F-AMB-004; I-PD-001 |
