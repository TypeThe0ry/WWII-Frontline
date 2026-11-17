# 进度总览与任务日志（PROGRESS.md）

> 最后更新：2026-11-17　｜　当前里程碑：**M0 技术验证（已验收，partial，等 Fable 审阅）→ 下一个 M1-001**
> 用途：负责人看进度的单一入口。执行者收工时在"任务日志"末尾追加条目；总控每波任务后更新顶部表格与"退出标准对照"。
> 逐任务的完整实测数字在 `tasks/STATUS.md`，计划/架构问题在 `docs/PLAN_ISSUES.md`，需要人处理的事项在 `docs/NEEDS_HUMAN.md`。
> 测量环境说明：除非另注，所有数字来自**本机——4 核共享虚机（Xeon 2.3 GHz）、无 GPU、Chromium 软件渲染（SwiftShader）**，均**不是参考机**；其它执行者并行编译/测试时负载均值常在 4–8。

## 1. 里程碑进度

任务总数 263（与 `tasks/INDEX.md` 一致），统计自 `tasks/STATUS.md`（2026-11-17，M0-033 验收后）。

| 里程碑 | 任务总数 | done | partial | blocked | in-progress | todo |
|---|---|---|---|---|---|---|
| M0 技术验证 | 33 | 19 | 14 | 0 | 0 | 0 |
| M1 垂直切片 | 72 | 0 | 0 | 0 | 0 | 72 |
| M2 完整 Frontline | 55 | 0 | 0 | 0 | 0 | 55 |
| M3 全兵种/载具/Assault | 57 | 0 | 0 | 0 | 0 | 57 |
| M4 三图/公开测试 | 46 | 0 | 0 | 0 | 0 | 46 |

**M0 验收结论（M0-033）：`partial`，可带条件关闭**——退出标准通过 5 / 部分 5 / 需参考机或人工 2 / 未通过 0；等 Fable 审阅检查点 0（H-038）。下一个任务：M1-001。详见 `reports/m0/ACCEPTANCE.md`。

## 2. M0 概况

done：M0-001、003、005、006、007、008、009、010、012、013、014、016、017、018、019、021、026、029、031（19 个）。
partial：M0-002、004、011、015、020、022、023、024、025、027、028、030、032、033（14 个，含 M0-033 自身）。
blocked / in-progress / todo：无。M0 的 33 个任务全部已提交（M0-033 由总控提交）；提交哈希取自 `git log`（提交信息以任务 ID 开头）。验收报告：`reports/m0/ACCEPTANCE.md`。

| ID | 标题 | 状态 | 提交 | 一句话结果 |
|---|---|---|---|---|
| M0-001 | 仓库脚手架与工具链 | done | ca86eea | 8 个 crate + client + tools 脚手架；`-D warnings` 构建 0 警告；内部依赖边 14 条与 ARCH §4.5 逐条相等 |
| M0-002 | GitHub Actions 基线与依赖图检查 | partial | 7360d3b | 5 个 workflow + deny.toml + check-deps/purity 脚本本地全过；**未在 GitHub 实跑**（H-102） |
| M0-003 | 字体许可证核实与登记（R10） | done | 3deb07d | Barlow、Noto Sans SC 为 OK；IBM Plex 两行 PENDING（RFN "Plex"，裁定改族名，PI-009） |
| M0-004 | /data schema 与校验工具 | partial | 2bdf74a | 23 个坏样例全部被拒、校验器测试 42/42；DATA_HASH 稳定性测试通过；CI 上"坏 JSON 的 PR 被拒"未实测 |
| M0-005 | protocol：位流、量化与 M0 消息集 | done | 89aad49 | 10^6 样本往返全等；典型 3 条 InputBundle 29 B（最坏 49 B）；位置量化误差 1/64 m |
| M0-006 | protocol：快照编码与基线增量 | done | 177a6ed | 40 实体快照均值 282 B（≤600）；近景 delta 4.12–6.12 B/记录；fuzz 637 万 + 3770 万次 0 崩溃 |
| M0-007 | sim：World/step/确定性底座 | done | b6e07c4 | 2×10 000 tick 逐 tick 哈希相等；100/100 次扰动被检出；bevy_ecs 锁 =0.16.1 |
| M0-008 | 数据：M0 步枪（Kar98k、M1 Garand） | done | c59dd79 | 与 GAMEDESIGN §6.2–6.6 逐项对照差异 0；Garand 躯干 STK=2（0–800 m），Kar98k ≤200 m 为 1 发 |
| M0-009 | 地图：M0 单块地形（256 m）数据管线 | done | ef41e84 | 流水线 0.3–0.7 s，产物共 2.6 MB，两次运行 37 文件 SHA-256 全同；最大坡度 38.7° |
| M0-010 | sim：高度场碰撞 | done | 7705e34 | 射线中位 1.47 ms（<5 ms）；38.7° 坡 KCC 增高 0.02 m；反量化误差 1.1e-6 m；16 块驻留 0.336 MB |
| M0-011 | sim：Rapier 角色控制器与基础移动 | partial | a54126d | KCC 单次中位 18.6 µs（≤20，余量 7%）；100 人 step 中位 2.14 ms（≤3）；仅差参考机复测 |
| M0-012 | sim：步枪射击、弹道与装填 | done | 5585573 | 500 m 下坠对 RK4 误差 2.0%；射速/装填与数据差 0 tick；4 玩家 5000 tick 哈希全同 |
| M0-013 | sim：生命、死亡与简易重生 | done | a4f7dcf | 重生时刻与公式误差 0 tick；含 9 次击杀/重生的 5000 tick 场景两次哈希全同 |
| M0-014 | sim-wasm：WASM 封装 | done | 151b4b0 | wasm brotli 240 627 B（≤400 KB）；Predictor.step 中位 12.3–13.7 µs；native 与 wasm 3000 tick 逐字节一致 |
| M0-015 | server：进程骨架与 30 Hz 主循环 | partial | aee3f6c | 10 min 空载 tick_ms P99 0.017 ms、overrun 0；tick 间隔抖动 P99 2.83 ms 未达 <1 ms（共享虚机，需参考机） |
| M0-016 | server：WebSocket 传输与握手 | done | ca977df | 10 客户端×10 min 6010 次回显 0 断开；10 059 个畸形帧全部关闭连接；握手往返 P50 0.07 ms |
| M0-017 | server：WebTransport 传输 | done | 8819f08 | 100 客户端×120 s 上下行 240 100/240 100 = 100%；RTT P50 1.35 / P99 5.50 ms；单核 11.3% |
| M0-018 | server：会话、输入缓冲与校验 | done | cfaf153 | 20 客户端×60 s tick P99 0.94 ms、100 客户端×60 s P99 4.0 ms；输入窗口/去重/校验逐项计数；真 World 逐字节对拍 |
| M0-019 | server：AOI v0 与快照发送 | done | 69ba174 | 10 客户端线路 P50 16 kbps、100 客户端 P50 59.7 / P99 91.6 kbps；AOI+编码 100 客户端 P99 1.44 ms（预算 2.6） |
| M0-020 | client/render：引擎初始化与调试叠层 | partial | 09e58e0 | WebGL2 路径 e2e 5/5；1080p 空场景 CPU 帧时间 P99 0.60 ms；WebGPU 真机项与 compute 真 GPU 未测 |
| M0-021 | client/net：传输与时钟同步 | done | 2b8b853 | 时钟同步 5 种子 551/551 采样误差 ≤1 tick；WT 不可用时 3 s 回退 WS（真浏览器回退后 WS 握手 3014 ms）；每帧 net 处理 1.9 µs；e2e 11 例全绿 |
| M0-022 | client/game：输入、预测、对账与插值 | partial | 70ebb4b | 进程内等价模型 RTT 100 ms + 丢 2 %：稳态纠正 0；真实 tc netem 未测（H-022） |
| M0-023 | client/render：CDLOD 地形 | partial | c020a7e | 近地形裂缝最差 9.8e-15 m；heightAt 对 sim 最大误差 6.7e-6 m；GPU 帧时间未测；可见节点数验收口径已改（PI-014） |
| M0-024 | client/render：植被实例化与 impostor | partial | 034b00e | draw call ≤ 50（≤400）；impostor 与 LOD1 面积差 ≤ 17.7 %；重分桶 P99 0.3 ms；帧率需 GPU（H-021） |
| M0-025 | client/render：灰盒角色、第一人称枪与射击反馈 | partial | d16ddea | 开火与枪口焰同帧 30/30；100 士兵 draw call +16（≤40）；插值二阶差分 RMS 1e-5 m；GPU 帧时间需参考机 |
| M0-026 | bots：M0 基础 bot | done | b0dd1ec | 100 bot think P99 0.67 ms（预算 3）；真实服务器 `--bots 100` tick P50 1.86 / P99 3.56 ms；bot 无全知 |
| M0-027 | headless-client v0 与 10 客户端联测 | partial | 6c470e2 | WT 30 min 10+10：tick P99 1.2 ms、下行 P50 21.7 kbps、0 断线；100 客户端+10 bot P99 8.3 ms |
| M0-028 | 测试：浏览器 WT/WS 互通矩阵 | partial | 6cb03c9 | Chromium WT/WS/两种回退全部通过；Firefox/WebKit 如实"未验证"；验收时复跑（真实快照） |
| M0-029 | 测试：确定性与 WASM 漂移 | done | 58b880d | 3 种子×18 000 tick：native×2、wasm×2 各 54 000/54 000 一致；native vs wasm 漂移 0 cm/s |
| M0-030 | 测试：M0 性能基线与报告 | partial | a9d1dc4 | 三机位 draw call 50/33/13；RSS 斜率 13.7→0.6 KB/s；低帧率客户端不被踢；真 GPU 帧时间未测 |
| M0-031 | 史实参数复核：Kar98k 与 M1 Garand（R8） | done | 847c704 | 15 个 historical 字段：13 已核实、2 登记未核实；M1 实用射速 40→24 |
| M0-032 | 对标测量：M0 批回填 | partial | cf2b431 | pytest 69/69，合成数据测回误差远小于容限；我方复测 own_m0 出数；参考作品一侧全部"待测"（缺录像） |
| M0-033 | M0 里程碑验收与 STATUS 汇总 | partial | （总控填） | 退出标准 通过 5 / 部分 5 / 需参考机或人工 2；干净复验全绿；30 min RSS +1.95 %；M0 partial，等 Fable 审阅 |

## 3. M0 退出标准对照

依据 `docs/MILESTONES.md` §1.3；逐项证据见 `reports/m0/ACCEPTANCE.md` §2（M0-033，2026-11-17，本机非参考机）。E0-1 ~ E0-6 任一不通过 → M0 记 `partial` 并由 Fable 裁决；E0-7 ~ E0-12 可带条件通过但须登记为 M1 前置待办。**统计：通过 5 / 部分 5 / 需参考机或人工 2 / 未通过 0 → M0 partial。**

| # | 退出标准 | 状态 | 证据 / 缺口 |
|---|---|---|---|
| E0-1 | 10 headless + 10 bot 跑 30 min，`tick_ms` P99 ≤ 28 ms，无连续 2 tick 超 33 ms | **通过** | 验收重跑 30 min（release）：tick P50 0.745 / P99 1.349 / max 9.93 ms，overrun 0，0 断线；`reports/m0/acceptance/loadtest-wt-30min.json` |
| E0-2 | 下行 P50 ≤ 128 kbps / 上行 ≤ 32 kbps | **通过** | 下行 P50 21.9（最差客户端 22.5）、上行 P50 20.5 kbps；100 客户端全员 P50 59.7 / P99 91.6（M0-019） |
| E0-3 | KCC 单次 ≤ 20 µs；100 个 KCC 一 tick ≤ 3 ms | **部分** | 单次 19.27 / 20.18 / 19.94 µs（零余量，落在噪声内，PI-030）；100 人 step 2.24–2.29 ms 达标。参考机复测（H-021）或 Fable 调整判据 |
| E0-4 | native/native、wasm/wasm 逐 tick 哈希一致；native vs wasm 漂移 < 2 cm/s | **通过** | 验收干净重建后 `tools/determinism/run.sh --strict`：3 种子 × 18 000 tick 全一致，漂移 0 cm/s；CI 实跑待 H-102 |
| E0-5 | RTX 3060/1080p ≥ 60 fps；Iris Xe/WebGL2/720p ≥ 30 fps | **需参考机** | GPU 帧时间全部未测（H-021）；CPU 侧：整帧 P50 1.1–1.3 ms、draw call ≤ 50，`perf-regress --quick` 0 项超容差 |
| E0-6 | WebGPU compute 冒烟通过，给出 Hi-Z/froxel/TAAU 结论 | **部分** | SwiftShader 上冒烟与三试验通过，结论"可行，Hi-Z 需自写薄封装"（PI-005）；真 GPU 未测（H-021） |
| E0-7 | Chromium WT/WS 通过；Firefox/WebKit 如实登记；WT 禁用 ≤ 5 s 回退 WS | **部分** | Chromium WT / WS / 两种回退全过（验收复跑，真实快照）；删除 WebTransport 回退 10 ms、UDP 黑洞 3.2 s；Firefox/WebKit/Safari 未验证（H-103）、受信任证书（H-017） |
| E0-8 | 100 ms + 2 % 丢包下静止校正 ≤ 1 次/10 s；本地开火与枪口焰同帧 | **部分** | 进程内等价模型稳态纠正 0、静止 120 s 1 次；开火同帧 30/30；**真实 `tc netem` 未做**（本机无 tc，H-022） |
| E0-9 | `sim-wasm` ≤ 1.2 MB（br ≤ 400 KB） | **通过** | wasm-opt 后 766 470 B；brotli-11 244 676 B |
| E0-10 | lint/test-rust/test-ts/wasm-build 全绿；依赖违规 PR 被拒 | **部分** | 本地等价全绿（fmt/clippy/434 测试/238 vitest/e2e/check-deps/purity）；GitHub 从未实跑（H-102） |
| E0-11 | /data 校验通过；两把枪 historical 100 % 有出处或"未核实" | **通过** | `data:validate` 0 错误、`data:test` 51/51；15 个字段 13 已核实 + 2 登记未核实 |
| E0-12 | 对标测量批 M0 有记录，复测误差在容限 | **需人工（blocked）** | 工具链就绪（pytest 69/69）、我方复测出数；参考作品一侧全部"待测"，缺录像（H-011/H-025/H-026，H-104） |

## 4. 关键实测数字

均为**非参考机**数字（见文首环境说明）。来源列为任务 ID；详细命令与完整数字见 `tasks/STATUS.md` 对应行。

### 4.1 协议与带宽

| 项 | 数字 | 来源 |
|---|---|---|
| InputBundle（3 条走动） | 典型 29 B（≤30），最坏 49 B | M0-005 |
| 量化误差（10^6 随机样本） | 位置 x/y/z 0.015625 m；yaw 10 bit 0.176°；pitch 9 bit 0.176°（8 bit 为 0.353°，故裁定改 9 bit） | M0-005 / PI-001 |
| 编解码速度（release） | InputBundle 编码 141 ns、解码 137 ns | M0-005 |
| 快照记录大小 | 近景 delta 4.12 B（纯走动）/ 6.12 B（边走边转）；远景 4.53 B；关键帧 12.50 B；self_state 7 B | M0-006 |
| 40 实体快照 | 均值 282 B，最大 287 B（≤600）；全关键帧 512 B；20 Hz 下载荷约 45 kbps（未含 58 B/包传输开销） | M0-006 |
| 快照编解码耗时（release） | 60 实体编码 ≈5.6 µs/客户端（预算 25 µs），解码 6.3 µs | M0-006 |
| 丢包鲁棒性 | 30% 下行 + 30% ack 丢包 60 s：0 次 MissingBaseline | M0-006 |
| fuzz | snapshot_decode 637 万次、input_bundle_decode 3770 万次执行，0 崩溃 | M0-006 |

### 4.2 模拟性能

| 项 | 数字 | 来源 |
|---|---|---|
| KCC 单次中位 | 18.6 µs（M0-011）；19.3–19.9 µs（M0-012）；19.75 µs（M0-013）；预算 ≤20，几乎无余量 | M0-011/012/013 |
| 100 人 `World::step` | 中位 2.14 ms（M0-011）→ 2.20–2.24 ms（M0-012）→ 2.265 ms（M0-013）；预算 3 ms | M0-011/012/013 |
| 弹丸阶段 | 20 人 + 400 弹丸×4 子步中位 0.30 ms（预算 2.5 ms） | M0-012 |
| 地形射线 | 10 000 条射线（≤800 m）中位 1.47 ms（<5 ms） | M0-010 |
| 空世界 step | 100 玩家 7.2 µs | M0-013 |
| Predictor.step（Node + wasm） | 10 000 次中位 12.3–13.7 µs，p95 24–29 µs（≤50 µs） | M0-014 |
| 基线文件 | `reports/baselines/sim-m0.json` | M0-011 |

### 4.3 确定性

| 项 | 数字 | 来源 |
|---|---|---|
| native×2 / wasm×2 | 3 种子×18 000 tick，各 54 000/54 000 逐 tick 哈希相等；10 玩家 World×2 同样全等 | M0-029 |
| native vs wasm | 每 tick 哈希与位置逐位相同，漂移 rms 0 / max 0 / 0 cm/s（目标 <2 cm/s） | M0-029 |
| 单次耗时（18 000 tick） | native 0.28–0.35 s；wasm（Node）0.61–0.80 s；10 玩家 World 3.4–4.1 s | M0-029 |
| 故障注入 | 注入非确定性后对比在第 700 tick 失败（检测有效） | M0-029 |
| 地形一致性 | 客户端 `TerrainData.heightAt` 对 sim `height_at` 506 032 点最大误差 6.9e-6 m | M0-029 |

### 4.4 传输与服务器

| 项 | 数字 | 来源 |
|---|---|---|
| 空载 10 min（30 Hz） | tick_ms P50 0.006 / P99 0.017 / max 5.34 ms；overrun 0；tick 间隔误差 P99 2.83 ms（绑定 WS 端口后复测 0.79 ms，tick_ms P99 0.013 ms） | M0-015 / M0-016 |
| WS 稳定性 | 10 客户端×10 min，6010 次回显，0 意外断开；握手 Hello→Welcome P50 0.07 ms（debug） | M0-016 |
| WS 防护 | 10 059 个畸形帧全部关闭连接、0 panic；5 s 握手超时；同 IP 第 5 连接被拒 | M0-016 |
| WT 压测（release） | 100 客户端×120 s，上行与下行各 240 100/240 100（100.0000%），0 意外断开；回显 RTT P50 1.35 / P99 5.50 / max 17 ms；握手 P50 51 ms；服务器 11.3% 单核 | M0-017 |
| 浏览器 | 无头 Chromium 153 经开发证书指纹连入 WT：10 datagram 收 10 个回包 | M0-017 |
| 说明 | M0-017 压测时 sim 内无玩家，tick 数字不含 sim 开销 | M0-017 |

### 4.5 渲染（无 GPU，软件渲染）

| 项 | 数字 | 来源 |
|---|---|---|
| 1080p 空场景（WebGL2/SwiftShader，600 帧） | CPU 帧时间 P50 0.30 ms、P99 0.60 ms（≤4 ms）；WebGPU(SwiftShader+shim) P99 0.50 ms | M0-020 |
| WebGL2 回退 | `navigator.gpu=undefined` 时 219 ms 内就绪（≤2 s） | M0-020 |
| compute 冒烟（SwiftShader WebGPU） | ok=true、mismatches=0、782 ms；froxel 921600/921600 | M0-020 |
| CDLOD 选择 | `selectNodes` 1000 随机位姿中位 0.077–0.082 ms（<0.3 ms）；可见节点中位 27（15–46） | M0-023 |
| 接缝 | 128 对相邻节点×10 000 采样最差高度差 9.8e-15 m；去掉邻接掩码对照 1.67 m；GPU 洞像素 0 | M0-023 |
| 纹理与加载 | 近远景高度纹理 2.13 MB（<4 MB）；地形 fetch+解码+建树+首次选择 48–100 ms；1 个 draw call | M0-023 |
| 未测 | 所有 GPU 帧时间（RTX 3060 / Iris Xe）：参考机 | M0-020/023 |

### 4.6 体积与数据

| 项 | 数字 | 来源 |
|---|---|---|
| sim-wasm | 优化后 746 965 B；gzip-9 308 281 B；brotli-11 240 627 B（门限 1 200 000 / 400 000） | M0-014 |
| M0 地形产物 | 共 2.6 MB（不入库，`python3 -m map_pipeline m0 --seed 1` 重建）；碰撞区块 16 块驻留 0.336 MB | M0-009 / M0-010 |
| DATA_HASH | 随数据变化（M0-004 空集 0x141cfc9842c4b0e3；M0-008 后 0x8e4ea0ee2af4bf8f；M0-029 时 0xeb4ef8d3b81fc45c），客户端不得写死字面量 | M0-004/008/029 |
| 客户端包 | 脚手架阶段 dist 0.75 kB js（无实质意义）；真实首包体积未测 | M0-001 |

### 4.7 验收复测（M0-033，干净重建后，release，本机非参考机）

| 项 | 数字 | 来源 |
|---|---|---|
| 30 min WT，10 headless + 10 bot | tick P50 0.745 / P99 1.349 / max 9.93 ms；下行 P50 21.9 / 上行 P50 20.5 kbps；0 断线；击杀 564 / 重生 563 | `reports/m0/acceptance/loadtest-wt-30min.json` |
| 服务器 RSS（同一次运行） | 12.1 → 14.6 MB；最后 10 min **+1.951 %**（验收线 2 %）；斜率 1.29 → 0.71 → 0.48 KB/s 递减 | 同上 `rss_samples` |
| KCC 单次 / 100 人 step | 19.27 / 20.18 / 19.94 µs；2.24–2.29 ms | `cargo bench -p frontline-sim --bench kcc` ×3 |
| 确定性（干净重建 wasm） | 3 种子 × 18 000 tick native/wasm 全一致，漂移 0 | `tools/determinism/run.sh --strict` |
| 测试规模 | Rust 434 通过 / 0 失败 / 10 忽略；vitest 238；Playwright 主 44 + net 11 + game 6 + interop 4；pytest 21 + 69 | `reports/m0/ACCEPTANCE.md` §3 |

## 5. 已生效裁定摘要

总控已做出的裁定（原文暂存于总控的执行者守则，此处为仓库内的正式记录；PI 条目的"裁决"栏已同步）。

| 编号 / 主题 | 裁定 | 影响的任务 |
|---|---|---|
| 包名 | crate 名带 `frontline-` 前缀；任务里的 `-p sim`/`-p protocol` 一律理解为 `-p frontline-sim`/`-p frontline-protocol` | 全部 Rust 任务 |
| PI-001 | protocol 细化全部采纳；仅 pitch 量化由 8 bit 改 9 bit | M0-005/006/014/018 |
| PI-002 / PI-003 | 数据字段名以 `data/schemas/*.schema.json` 与 `data/README.md` 映射表为准；新增字段先加可选 schema 字段再同步 `data_defs.rs` | M0-008/011/013/032 |
| PI-004 | sim 依赖 serde / serde_json / sha2 批准 | M0-007/014 |
| PI-005 | Babylon WebGPU compute 用 `client/src/render` 内自写薄封装；逐 mip 视图、深度纹理绑定、indirect draw 留到用到时验证 | M0-020/030、M1-043/044、M2-032/050 |
| PI-006 | 固定步长取 1000/30 ms | M0-022 |
| PI-007 | 保留 `part_mult.pelvis` 与 `fx{...}`；pelvis 缺省回退躯干倍率 | M0-012/025 |
| PI-008 | 地形二进制不入库，干净检出先运行 `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0`；约定 x 东、z 北、行=z；新增可选字段保留（该条原误编为第二个 PI-007） | M0-010/023/004 |
| PI-009 | 方案 A：IBM Plex 子集/WOFF2 改族名 `Frontline UI Sans`/`Frontline UI Mono`；Noto Sans SC 子集 name 表不得含 "Source"；LICENSES 增处理码 `SUBSET`、`RENAME` | M1-053/054 |
| PI-010 | 快照：单一基线 tick、仅基线里出现过的实体发 delta、远景不发 pitch、位置差分阈值 32/256/2048 步、基线环 32 tick；AOI 须让近/中景实体每包出现 | M0-019 等 |
| PI-011 | `bevy_ecs` 锁 `=0.16.1`、不开 `std`；升级前必跑 `node tools/ci/check-deps.mjs`；sim 测试用 `--features test-util`；sim 内遍历用 BTreeMap | M0-007 之后全部 sim 任务 |
| PI-012 | weapon `historical._source` 用结构化数组；`_meta.unverified[]`、`deviation_reason{}`；`historical` 只描述史实、gameplay 不因复核而改 | M1-070、M2-053、M3-051 |
| PI-013 | `rapier3d =0.36.1`（含 enhanced-determinism）只作碰撞世界；高度/射线/LOS 自写；缺地形二进制时测试跳过，`FRONTLINE_REQUIRE_TERRAIN=1` 则失败 | M0-011/012/014/022/029 |
| PI-014 | CDLOD：每实例邻接掩码 + 2:1 平衡；高度纹理 RG8；近远景融合带；**可见节点验收改为 ≤ 450（删去下限 150）** | M0-023/030、M1-049/050 |
| PI-015 | 移动数值在 role `movement` 块；解析 snap（Rapier 自带 snap 超预算）；加减速数据化：`accel_time_s` 0.25、`decel_time_s` 0.20【待调参】 | M0-011/012/014/022 |
| PI-016 | 武器：`SimEvent` 含 Shot/DryFire/Impact/Hit；`WeaponState` 33 B；半隐式欧拉 4 子步弹道保持不变；M0 无输入缓冲 | M0-013/014/015/018/022 |
| PI-017 | 伤害/重生规则在 `data/rules/respawn.json`；死者保留实体但不在 `players()`；**M0-018 载入地图后必须按 spawns.json 的 HQ 点对每队调用 `World::set_team_spawn`**；不要在客户端写死 DATA_HASH | M0-018/019 |
| PI-018 | sim-wasm：client 不把 pkg 当 workspace 依赖，用手写类型与 `LAYOUT_*` 常量；wasm 模块由 M0-022 注入；构建命令 `wasm-pack build crates/sim-wasm --target web --profile wasm-release` | M0-022/029/030 |
| PI-019 | 确定性：入口 `tools/determinism/run.sh [--strict]`；**CI 默认开严格模式**（下一个改 `determinism.yml` 的执行者落实）；`Predictor.heightAt` 已暴露 | M0-022/030 |
| 传输约定 | 帧格式 `[ch u8][len u16 LE][payload]`；WT 用 `wtransport =0.7.2`，流帧无通道字节；`Welcome.team`、`Command::ClientGone`、`--dev-echo` 由 M0-018 收尾 | M0-016/017/018/021 |
| 测试约定 | 时序敏感测试不断言墙钟内精确 tick 数/零超时；Playwright 无头 WT 页面须由真实 localhost HTTP 服务器托管 | 全部 |
| 性能约定 | 改动 sim 后跑 `cargo bench -p frontline-sim --bench kcc` 并在回报中给数字（KCC 余量≈1%） | M0-018 起 |
| 空碰撞世界 | `CollisionData::empty()` 下玩家不动；服务器启动须载入 `data/maps/m0` | M0-018 |
| WebGPU 本机限制 | 本机仅 SwiftShader，e2e 经 `client/e2e/webgpu-swiftshader-shim.js` 离屏；真 GPU 项标"需在参考机上测（H-021）" | M0-020/023/024/030 |
| 磁盘约定 | cargo 命令前 `export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`；可用空间 <5 GB 时先清理；不另建 target 目录 | 全部 |
| 进度文档 | 执行者收工追加本文件"任务日志"；新 PI 若已有对应裁定直接填"裁决"并标 resolved | 全部 |

## 6. 卡在人这边的事项

| # | 事项 | 影响 | 对应 H- 编号 |
|---|---|---|---|
| 1 | **GitHub 上没有 main 分支、无法开 PR**，CI 从未实跑 | M0-002、M0-004、M0-014、M0-029 的 CI 验收项（真实 PR 红绿对、Actions 墙钟、"坏 JSON 的 PR 被拒"）；E0-10 | **H-102**（新增；仓库与 Actions 额度另见 H-014） |
| 2 | **需要一台带独立显卡的参考机复测** | M0-020（WebGPU 帧时间/像素/compute 真 GPU）、M0-023（GPU 帧时间、Iris Xe）、M0-011（KCC 两项耗时）、M0-015（tick 抖动 P99 <1 ms）；E0-3、E0-5、E0-6 | H-021（测试设备矩阵）；当前影响见 NEEDS_HUMAN §0.1 的"开发启动后的新增事项" |
| 3 | **对标录像**（批 M0：步枪与基础移动） | M0-032、E0-12；GAMEDESIGN §21 的"待测"项校准 | H-011（EULA 判断）、H-025（录制设备）、H-026（批 M0 录制） |
| 4 | **受信任 TLS 证书与域名** | 跨浏览器 WT 互通与受限网络测试（M0-028）；本地开发证书仅 13 天且仅限 `serverCertificateHashes` | H-017 |
| 5 | **素材**：当前全部为灰盒占位 | 不影响 M0；M1 素材接入任务依赖 | H-098、H-099、H-100、H-101（免费优先方案下的补搜/下载/购买决定） |
| 6 | **Fable 的 M0 审阅**（M0 验收结论 partial，PI-030 待裁决） | M0 关闭 | H-038 |
| 7 | **Firefox / Safari 手工互通、受限网络 / `tc netem` 测试台** | E0-7 的非 Chromium 格、E0-8 的真实网络注入 | H-103、H-022 |

### 参考机复测入口

- 页面：`/?renderer=webgpu&smoke=compute&debug=1`，加载后在控制台读 `window.__smoke`（compute 冒烟结果：`ok`、`mismatches`、耗时、Hi-Z/froxel/TAAU 三试验）与 `window.__perf.dump()`（固定 16 字段的性能转储，含 `gpu_ms`）。
- 地形：`/?scene=terrain&cam=0|1|2&debug=1`，叠层 GPU timer；机位目前为占位预设（M0-030 定义正式机位）。
- WebGL2 路径：`/?renderer=webgl2`。
- sim 微基准：`cargo bench -p frontline-sim --bench kcc`（M0-011 的 KCC 与 100 人 step）；server 空载抖动按 M0-015 的 10 min 空载流程复测。
- 结果文件提交到 `reports/`，并在 STATUS 对应行与本文件追加说明。

## 7. 任务日志

按完成（提交）顺序。格式：`### 任务 ID 标题 — 状态 — 日期 — 提交`。

### M0-001 仓库脚手架与工具链 — done — 2026-11-17 — 提交 ca86eea
- 做了什么：建立 cargo workspace（protocol/sim/sim-wasm/server/gateway/bots/headless-client/balance-sim）、client（vite + TypeScript + Babylon 8.56.2）、tools 与 `check-layout`。
- 关键实测：`-D warnings` 构建、clippy 均退出 0；内部依赖边 14 条与 ARCH §4.5 允许集合逐条相等；8 个 smoke 测试通过。
- 遗留：BRIEF 文档不存在，目录按 ARCH §4.1；TypeScript 取 6.0.3（typescript-eslint 限制）；Babylon 取 8.x（最新大版本已是 9.x）。
- 影响：固定工具链版本（Rust 1.97.0、Node 22.23.4、pnpm 10.28.0）。

### M0-002 GitHub Actions 基线与依赖图检查 — partial — 2026-11-17 — 提交 7360d3b
- 做了什么：5 个 workflow（lint/test-rust/test-ts/wasm-build/data-validate 占位）、`deny.toml`、`tools/ci/{dep-matrix,check-deps,check-sim-purity}.mjs` 及 11 个 node:test。
- 关键实测：本地逐 step 通过；沙盒副本加 sim→tokio、gateway→sim 均使 check-deps exit 1。
- 遗留：**未在 GitHub 实跑**（真实 PR 红绿对、Actions 墙钟未取得）；`cargo deny` advisories 本机无法访问 RustSec；deny 许可证清单待 H-037。
- 影响：E0-10 悬空；后续 workflow 的墙钟均无法验证（H-102）。

### M0-005 protocol：位流、量化与 M0 消息集 — done — 2026-11-17 — 提交 89aad49
- 做了什么：位流读写、量化、C0 与输入消息（Hello/Welcome/Ping/Pong/InputCmd/InputBundle），golden bytes 与 fuzz 冒烟。
- 关键实测：10^6 样本往返全等；位置量化误差 1/64 m；典型 3 条 InputBundle 29 B、最坏 49 B；编码 141 ns/解码 137 ns；2×10^5 随机串 + 全部截断 + 全部单比特翻转 0 panic。
- 遗留：pitch 8 bit 误差 0.353° 略超 0.35°（PI-001 裁定改 9 bit，由 M0-006 落实）。
- 影响：Welcome 增加 `data_hash`/`client_tick_offset`；InputBundle 差分位宽成为 M0-018/021/022 的接口。

### M0-004 /data schema 与校验工具 — partial — 2026-11-17 — 提交 2bdf74a
- 做了什么：JSON Schema（weapon/role/loadout/vehicle/map_meta/spawns）、`tools/data-validate` CLI、sim 编译期数据载入与 `DATA_HASH`。
- 关键实测：23 个 invalid 样例全部被拒（退出码 1 且含 `_expect_error`），校验器测试 42/42；`data_hash_stable` 通过；校验全流程约 6 s（<2 min）；build.rs 增量约 0.26 s。
- 遗留：CI 墙钟与"坏 JSON 的 PR 被拒"需 GitHub 实跑（H-102）；PI-002/003/004。
- 影响：数据字段命名以 schema 为准；DATA_HASH 随数据变化。

### M0-020 client/render：引擎初始化与调试叠层 — partial — 2026-11-17 — 提交 09e58e0
- 做了什么：Babylon 引擎初始化（WebGPU 优先、WebGL2 回退）、调试叠层与 `__perf.dump()`（16 字段）、`FixedStepper`、compute 冒烟（Hi-Z/froxel/TAAU 试验）。
- 关键实测：e2e 5/5（WebGL2）；1080p 空场景 CPU 帧时间 P50 0.30 / P99 0.60 ms；SwiftShader WebGPU compute 冒烟 ok、mismatches 0；回退 219 ms 内就绪。
- 遗留：WebGPU 真机帧时间/像素/compute 未测（H-021）；PI-005（需自写薄封装）、PI-006（步长 1000/30）。
- 影响：本机无头 WebGPU 无法 present，e2e 用离屏 shim；像素校验走 WebGL2。

### M0-008 数据：M0 步枪（Kar98k、M1 Garand） — done — 2026-11-17 — 提交 c59dd79
- 做了什么：两把枪的数据文件，STK 快照测试；schema 追加可选 `part_mult.pelvis` 与 `fx`。
- 关键实测：与 GAMEDESIGN §6.2–6.6 逐项对照差异 0；躯干 STK：Garand 0–800 m 均 2 发，Kar98k ≤200 m 1 发、201 m 起 2 发；DATA_HASH 变为 0x8e4ea0ee2af4bf8f。
- 遗留：当时 Rust 侧在隔离副本验证（M0-007 在途）；PI-007。
- 影响：`pelvis` 缺省回退躯干倍率（M0-012 落实）。

### M0-009 地图：M0 单块地形（256 m）数据管线 — done — 2026-11-17 — 提交 ef41e84
- 做了什么：确定性程序化地形管线（高度图、渲染/碰撞区块、manifest）。
- 关键实测：流水线 0.3–0.7 s；产物共 2.6 MB（1 m 高度图 132 098 B、远景 8 m 2 101 250 B、16+16 个区块文件）；两次运行 37 个文件 SHA-256 全同；最大坡度 38.7°，<5° 平地占 88.5%；21 个 pytest 通过 1 跳过。
- 遗留：DEM（GDAL）路径本机未装，未实跑；二进制不入库（PI-008）。
- 影响：干净检出必须先运行地图流水线；坐标约定 x 东、z 北。

### M0-003 字体许可证核实与登记（R10） — done — 2026-11-17 — 提交 3deb07d
- 做了什么：核实 4 个字体并登记台账，保存上游许可证文本，写 `font-subsetting.md`。
- 关键实测：Barlow、Noto Sans SC 为 OK；IBM Plex Sans/Mono 为 PENDING（RFN "Plex"）；Noto Sans SC 可变 TTF 17 772 300 B。
- 遗留：LICENSES §5 字体行仍写"未核实"（未改）。
- 影响：PI-009 裁定方案 A（改族名），由 M1-053 落实。

### M0-007 sim：World/step/确定性底座 — done — 2026-11-17 — 提交 b6e07c4
- 做了什么：ECS World、实体 ID（带代数）、PRNG、状态哈希、固定步长调度。
- 关键实测：2 次 10 000 tick 逐 tick 哈希 10 000/10 000 相等；100 次扰动 100/100 在 ≤2 tick 内检出；sim 依赖图无 tokio/rayon/wasm-bindgen/getrandom；100 玩家空世界 step ≈2.78 µs。
- 遗留：`bevy_ecs` 只能锁 =0.16.1 且不开 `std`（PI-011）；determinism 测试须带 `--features test-util`。
- 影响：后续 sim 任务沿用 World 接口；`los/height_at/team_resources` 为占位，分别由 M0-010/013 落实。

### M0-031 史实参数复核：Kar98k 与 M1 Garand（R8） — done — 2026-11-17 — 提交 847c704
- 做了什么：两把枪 historical 字段逐项核实，写 `reports/m0/hist-review-m0.md`；`_source` 改结构化数组。
- 关键实测：15 字段中 13 已核实、2 未核实（Kar98k 初速、射速）；一手来源 TM 9-1275、FM 23-5、TM-E 30-451；M1 实用射速 40→24 发/分并新增 `effective_range_m=460`；gameplay 数值未改。
- 遗留：Kar98k 一手来源未找到（M1-070 补）。
- 影响：PI-012 格式沿用到后续武器/载具数据。

### M0-006 protocol：快照编码与基线增量 — done — 2026-11-17 — 提交 177a6ed
- 做了什么：快照编码/解码、基线 delta、预算打包、属性与 fuzz 测试；落实 PI-001（pitch 9 bit）。
- 关键实测：近景 delta 4.12–6.12 B/记录，远景 4.53 B，关键帧 12.50 B；40 实体均值 282 B（最大 287 B，预算 600 B）；基线环 32 tick；30% 丢包 60 s 0 次 MissingBaseline；fuzz 637 万 + 3770 万次执行 0 崩溃。
- 遗留：PI-010（单一基线 tick、位置差分阈值 32/256/2048 步等细化）。
- 影响：M0-019 的 AOI 须让近/中景实体每包都出现，否则反复发 12.5 B 关键帧。

### M0-010 sim：高度场碰撞 — done — 2026-11-17 — 提交 7705e34
- 做了什么：碰撞区块解析、高度场查询、射线/LOS、Rapier 碰撞世界。
- 关键实测：射线（10 000 条，≤800 m）中位 1.47 ms；坡度 36–55° 增高 0.02 m；反量化最大差 1.1e-6 m；解析 10^5 例模糊测试无 panic；16 块驻留 0.336 MB；自写射线与 Rapier 3000 条中 2999 条一致（另 1 条 Rapier 漏检）。
- 遗留：native↔wasm 逐位一致性待 M0-014/029 比对（后已一致）；cargo-deny 未跑；PI-013。
- 影响：地形二进制缺失时测试跳过，验收须设 `FRONTLINE_REQUIRE_TERRAIN=1`。

### M0-015 server：进程骨架与 30 Hz 主循环 — partial — 2026-11-17 — 提交 aee3f6c
- 做了什么：固定 30 Hz sim 线程、`/metrics`、JSON 日志、优雅关闭。
- 关键实测：10 min 空载 17 995 tick，tick_ms P99 0.017 ms、overrun 0；tick 间隔误差 P99 2.83 ms；SIGTERM 退出码 0 且 <100 ms；100 玩家空世界 step P99 <1 ms（debug）。
- 遗留：抖动 P99 <1 ms 未达标（共享虚机负载；M0-016 回归复测 0.79 ms），需参考机复测（H-021）；`--pin-sim-core`、`--bots` 仅接受参数。
- 影响：端口绑定与会话由 M0-016/017/018 接手。

### M0-011 sim：Rapier 角色控制器与基础移动 — partial — 2026-11-17 — 提交 a54126d
- 做了什么：KCC 移动（姿态、冲刺、跳跃、坡度限制）、数据化参数、预测一致性。
- 关键实测：KCC 单次中位 18.6 µs（预算 ≤20，余量约 7%）；100 人 `World::step` 中位 2.14 ms（≤3）；各模式稳态速度误差 <0.01%；>35° 坡 5 s 增高 ≤0.040 m；10 000 步重放与另一线程逐步位相同；World 与 PredictSim 3000 tick 位相同。
- 遗留：仅差参考机复测 KCC 两项耗时；PI-015（Rapier 自带 snap 约 41 µs 超预算，改解析 snap）。
- 影响：`PlayerAuthState` 39 B 布局供 M0-014/022；空碰撞世界下玩家不动，M0-018 须载入真实地形；后续 sim 任务须盯 KCC 余量。

### M0-012 sim：步枪射击、弹道与装填 — done — 2026-11-17 — 提交 5585573
- 做了什么：射击/装填状态机、散布与后坐、半隐式欧拉弹道、命中体与伤害；落实 PI-015 加减速。
- 关键实测：500 m 下坠对 RK4 误差 Garand 2.0%/Kar98k 1.7%；射速（Garand 20 发/10 s）与装填时长（2.800/3.400 s）与数据差 0 tick；盲射 10 000 发全在 3.5° 内；躯干伤害与公式差 <0.01 HP；确定性 5000 tick 全同；弹丸阶段 0.30 ms；104 项 sim 测试全过（`FRONTLINE_REQUIRE_TERRAIN=1`）。
- 遗留：命中体尺寸/眼高/移动散布阈值为初值【待测 → M-15】；M0 无输入缓冲；PI-016。
- 影响：`WeaponState`（33 B）与新 API 供 M0-014/015；KCC 余量降至 19.3–19.9 µs。

### M0-016 server：WebSocket 传输与握手 — done — 2026-11-17 — 提交 ca977df
- 做了什么：帧格式、握手、每通道队列、限速与大小限制、负载不敏感的冒烟测试。
- 关键实测：10 客户端×10 min 6010 次回显 0 断开；畸形帧 10 059 个全部关闭连接；C2 丢旧保新、C0 100/100 保序；Hello→Welcome P50 0.07 ms；M0-015 回归 interval_error P99 0.79 ms。
- 遗留：`Welcome.team` 暂为 `player_id & 1` 占位、`Command::ClientGone` 暂同 RemovePlayer（M0-018 接手）；未测真实网络 CPU。
- 影响：新增依赖 tokio-tungstenite 0.26；修正 `skeleton_smoke` 的负载敏感断言，确立"时序测试不断言墙钟精确值"的约定。

### M0-023 client/render：CDLOD 地形 — partial — 2026-11-17 — 提交 c020a7e
- 做了什么：四叉树选择、无裂缝 morph（邻接掩码）、远景环融合、调试模式、`pnpm -C client sync-map`。
- 关键实测：selectNodes 中位 0.077–0.082 ms；接缝最差 9.8e-15 m（无掩码对照 1.67 m）；heightAt 对 sim 最大 6.7e-6 m；高度纹理 2.13 MB；加载 48–100 ms；1 个 draw call；e2e 12/12。
- 遗留：GPU 帧时间未测（H-021）；WebGPU 路径无像素校验；可见节点中位 27，低于原验收下限 150（PI-014，总控裁定删去下限）。
- 影响：M0-030 需定义三个机位并接管 `cam=` 预设；M0-024/025 用 `Terrain.heightAt`。

### M0-013 sim：生命、死亡与简易重生 — done — 2026-11-17 — 提交 a4f7dcf
- 做了什么：伤害结算接生命值、死亡与波次重生（规则在 `data/rules/respawn.json`）。
- 关键实测：躯干 STK 与数据推导一致；重生时刻与公式误差 0 tick（7 s→15 s、16 s→30 s 等）；含 9 次击杀/9 次重生的 5000 tick 两次哈希全同；20 人 3000 tick 24 击杀 = 24 死亡；KCC 中位 19.75 µs（余量≈1%）、100 人 step 2.265 ms。
- 遗留：PI-017。
- 影响：**M0-018 载入地图后必须对每队调用 `World::set_team_spawn`**；新增数据文件会改变 DATA_HASH，客户端不得写死哈希；119 项 sim 测试全过。

### M0-014 sim-wasm：WASM 封装 — done — 2026-11-17 — 提交 151b4b0
- 做了什么：wasm-bindgen 封装（Predictor、SnapshotDecoder、消息函数）、体积门禁、native-vs-wasm 对比、手写 TS 类型。
- 关键实测：brotli 240 627 B / gzip 308 281 B / 优化后 746 965 B；`Predictor.step` 中位 12.3–13.7 µs；1000 次 decode 内存增长 0 B；非法输入全部抛异常且可恢复；native 与 wasm 3000 tick 每 tick 72 B 状态逐字节相同。
- 遗留：CI 墙钟未取得（本机冷编译约 1 m 10 s）；`SelfState` 的 pos_corr 对账由 M0-022 接上；PI-018。
- 影响：client 不把 pkg 作为 workspace 依赖，M0-022 注入 wasm 模块；M0-029 复用 trace 示例。

### M0-029 测试：确定性与 WASM 漂移 — done — 2026-11-17 — 提交 58b880d
- 做了什么：3 种子×18 000 tick 随机输入的 native×2/wasm×2/native-vs-wasm 套件、故障注入、漂移统计与 `determinism.yml`。
- 关键实测：各 54 000/54 000 tick 一致；native vs wasm 漂移 0（目标 <2 cm/s）；故障注入第 700 tick 检出；`summary.json` 重跑逐字节相同；`terrain-vs-sim.test.ts` 506 032 点最大 6.9e-6 m。
- 遗留：CI 未实跑（墙钟 <6 min 未取得）；PI-019。
- 影响：裁定 CI 默认开严格模式（下一个改 `determinism.yml` 的执行者落实）；输入运行时生成、不入库。

### M0-017 server：WebTransport 传输 — done — 2026-11-17 — 提交 8819f08
- 做了什么：基于 wtransport 的 WT 传输（datagram 走 C1/C2，可靠流走 C0/C4），开发证书工具，与 WS 共用连接逻辑。
- 关键实测：100/100 datagram 回显保序；100 客户端×120 s 上下行 100.0000%，RTT P50 1.35 / P99 5.50 ms，11.3% 单核；无头 Chromium 153 收发 10/10；`transport_matrix` 18 项全过（9 用例×WS/WT）；开发证书 13 天 ECDSA P-256。
- 遗留：Firefox/Safari 互通与受信任证书未做（M0-028、H-017）；`--dev-echo` 脚手架待 M0-018 移除或限定测试；C5 bulk 流无测试。
- 影响：新增依赖 wtransport =0.7.2（未跑 `cargo deny`）；`ClientHandle::max_datagram_size()` 供 M0-019；无头 Chromium 页面须由真实 localhost HTTP 托管。

### M0-028 测试：浏览器 WT/WS 互通矩阵 — partial — 2026-11-17 — 提交 （总控填）
- 做了什么：在 M0-021 的联调基础设施上新增 `client/e2e/interop/`（矩阵汇总 fixtures + vitest、server 包装、interop.spec.ts：每浏览器 × WT/WS 的 10 s 探针与回退用例）、`scripts/run-interop.sh`、`scripts/check-interop-table.mjs`（JSON 与 STATUS 表比对）、`.github/workflows/interop.yml`（手动 + nightly）；Firefox/WebKit 未安装时记"未验证"。
- 关键实测：Chromium 153.0.8010.12 WT 与 WS 各 199 包/10 s、0 断线、RTT 中位约 1 ms（loopback，C2 来源为 --dev-echo）；删除 WebTransport 回退 WS 约 10 ms，WT 端口关闭回退约 13 ms（≤5000 ms）；M0-021 的 net.spec 11/11 重跑通过。
- 遗留：Firefox / WebKit / Safari 无法在本机验证（H-103，复测步骤见 DEVELOPING.md）；快照数来自 dev-echo，M0-019 后须复跑；PI-021 登记了对 §5.0 的实测核对。
- 影响：`net-support.ts` 的 `startServer` 增加可选 `echo`；vitest include 增加 `e2e/interop/*.test.ts`；若 M0-018/019 移除 `--dev-echo`，探针自动改记 `snapshotSource: server`，须有真实 C2 快照才能通过。

<!-- 执行者在此之后追加（只追加；写入前重新读取文件末尾；格式同上；不改别人的条目） -->

### M0-018 server：会话、输入缓冲与校验 — done — 2026-11-17 — 提交 （待总控填）
- 做了什么：启动时载入 `data/maps/m0` 的 16 个碰撞区块与 spawns.json（每队 `set_team_spawn`、`set_map_center`），缺图即报错并给出重生成命令；握手时真实分配队伍（人少者优先、平局 US）并先于 Welcome 发 `Command::Join`；`validate_input` 单一入口 + 8 槽输入环 + 缺失输入状态机 + 30 连违规踢出 + C0 Ping 5/s；`ClientGone` 幂等；PlayerId 释放后 1 s 内不复用；`--dev-echo` 降为隐藏测试脚手架。
- 关键实测（release，本机 4 核、客户端同机）：20 客户端×30 Hz×60 s 的 `server_tick_ms` P50 0.243 / P99 0.939 ms（预算 28），100 客户端 P50 1.00 / P99 4.01 ms，`input_queue_full_total`=0；输入摄取+校验 100 玩家 P99 0.026 ms（预算 0.8）；4 玩家 90 tick 合成输入与离线 World 逐字节一致；10^5 条 fuzz 无 panic。
- 遗留：8000°/s 转速上限在 30 Hz/16 位角下不可触发（PI-022，M1 改滑动窗口）；缺失输入清边沿位是对 §6.3 的细化；Hello 无昵称；M0-019 从 `SimBridge`/`PlayerInputState` 读 ack 与 pred_hash。
- 影响：M0-019 在 `sim_thread::tick_body` 的 `aoi_and_snapshot/send` 钩子里接 `Sessions::handle_of(..).send`；M0-026 的 bot 必须走 `PlayerInput::ingest`/`validate_input`；服务器二进制现需地形（或 `--empty-world`/`--dev-echo`）；`logging::CURRENT_TICK` 进程全局，同进程多服务器的测试需串行。

### M0-022 client/game：输入、预测、对账与插值 — partial — 2026-11-17 — 提交 （待总控填）
- 做了什么：键鼠采样与 Pointer Lock（`client/src/input`）、30 Hz 预测 + 未确认环（128）+ 对账（0.05 m / 1.0 m，平滑 100 ms τ=40 ms）+ 100 ms 缓冲插值 + 事件解码（`client/src/game`）、`?server=` 联网入口（`net-game.ts`）、`sync-map` 追加 16 块碰撞与 wasm 包；测试以进程内假服务器为主（同一 wasm Predictor 作权威端，含延迟/抖动/丢包/乱序）。
- 关键实测（本机非参考机）：正常路径零纠正（走动 60 s、快照延迟 100 ms 丢 2 %：稳态 0 次、snaps 0；启动 1 次 initial sync）；注入 0.3 m 偏差每 10 s：稳态 5 次纠正、snaps 0、最大可见偏移 0.189 m；预测步 P50 0.010 ms，回滚+重放 8–32 条 0.03–0.07 ms；60 实体每帧 P99 0.088 ms（预算 2.1）；插值误差 P99 3.0 %；InputBundle 3 条 27.1 B（≈20 kbps 含 58 B 线路开销）。
- 遗留：实机 `tc netem` 验收（需 M0-019 真实快照）未测；`pred_hash`/`pos_corr`/`ack_snapshot_tick` 语义由本任务暂定（PI-023，M0-019 须照此实现）；Shift=walk 与 UI_SPEC 冲突；事件位置码米制暂定。
- 影响：M0-025 用 `window.__netGame.game.state`（本地位姿、实体 SoA、signals）与事件总线接渲染；M0-019 必须读 PI-023；M0-027 的 headless 客户端可复用 `GameLoop` 的 `NetPort`；e2e：`pnpm -C client exec playwright test -c playwright.game.config.ts`。


### M0-019 server：AOI v0 与快照发送 — done — 2026-11-17 — 提交 （待总控填）
- 做了什么：`crates/server/src/snapshot/`（64 m 网格、优先级累加器与 R3 候选规则、每客户端基线/ack/`pred_hash` 记账、事件 3 包冗余、`self_state`、调度器），接入 `sim_thread` 的 `aoi_and_snapshot`/`send`；20 Hz 按 `player_id % 3` 错相、WS 15 Hz、连续 3 次 tick 超时全体降到 15 Hz（90 tick）；`pred_hash` 的唯一 Rust 实现放在 protocol（与客户端 TS 黄金值对拍）；新增指标 `snapshot_bytes`、`snapshot_budget_utilization`、`aoi_starved_total`、`aoi_encode_ms`、`net_bytes_out_per_client_kbps{kind}` 等。R3 按总控裁定在 M0 就实现。
- 关键实测（release，非参考机）：10 客户端×5 min 线路估计 P50 16.0 / P99 18.8 kbps（预算 128 / 400）；100 客户端全员收快照 P50 59.7 / P99 91.6 kbps、平均包 336 B；AOI+编码每 tick 10 客户端 P99 0.036 ms、100 客户端 P50 0.79 / P99 1.44 ms（预算 2.6）；30 % 丢包事件送达 98.1 %；每实体记录 11–13 B，关键帧占记录 65–85 %（PI-010 的"近景每包出现"提示被实测推翻：开启反而 +20 % 字节，默认关，见 PI-024）。
- 遗留：C3 团队通道、载具记录、带宽调节器未做（M1/M2）；`pred_hash` 应改由 sim-wasm 导出（M0-025）；实机 tc netem 对账可现在复测（M0-022 遗留）；Firefox/WebKit 仍未验证（H-103）；`bots_soak::bots_smoke_20s` 在 M0-026 进行中失败（非本任务）。
- 影响：`client/e2e/net-support.ts` 默认不再起 `--dev-echo`，浏览器矩阵 C2 来源已是 `server`；`SimLoop::{snapshot_stats, snapshot_degraded}` 与 `SnapshotScheduler`（可被 M0-027 headless 对拍）；M0-025 须按 PI-024 第 4 条解释 `flags`/`anim`。

### M0-025 client/render：灰盒角色、第一人称枪与射击反馈 — partial — 2026-11-17 — 提交 （待总控填）
- 做了什么：`client/src/render/actors`（ActorManager、实例化灰盒角色/枪、命名与 `window.__placeholders`）、`render/fx`（枪口焰、曳光、尘土、第一人称相机与枪）、`ui/{crosshair,hit-marker}`、`actors-boot.ts` 接线（`?server=` 用真实 GameState，`?actors=demo` 无服务器）；键位改为 Left Shift 冲刺 / CapsLock 调试慢走；sim-wasm 导出 `predHash`，client 删除 TS 版。
- 关键实测：30 次开火同帧差 0；10 bot 60 s 渲染二阶差分 RMS 1e-5 m、0 瞬移；10 / 100 个士兵相对 0 个：draw call 均 +16、三角形 +6 520 / +65 200；真实服务器（含快照与 8 个 bot）冒烟通过，同队两页互见；vitest 226 通过。
- 遗留：CPU/GPU 帧时间待参考机（H-021）；tc netem 本机无 tc 未测；软件渲染下带植被场景仅 1–4 帧/s，联网时出现纠正（环境所致）。
- 影响：PI-025（灰盒尺寸在 data/placeholders，改变 DATA_HASH；占位名统一 `__`）；M0-030 可用 `?actors=demo&entities=N`；服务器与 wasm 需同源重建。


### M0-026 bots：M0 基础 bot — done — 2026-11-17 — 提交 （待总控填）
- 做了什么：`crates/bots`（`BotBrain` trait、`SoldierBrain` 状态机 Patrol/Engage/Reload/Dead、4 m 栅格 + 3/2 加权 A*、视锥 120°/距离衰减识别概率/LoS 感知、瞄准〔弹道下坠、目标速度提前量、高斯误差、扣后坐〕、卡住检测）；`crates/server/src/bot_host.rs`（`--bots N`：bot 占 PlayerId 127 倒数、队伍交替，每 tick 经 `SimBridge::ingest_cmd` 即真人同一条 `validate_input` 路径注入）；`data/rules/bots.json` + `bots.schema.json` + 校验器新种类，`GameData::rules.bots`；`server_bots`、`bot_think_ms`、`bot_path_ms` 指标；`WorldView::slope_deg_at`（只读追加）。
- 关键实测（release，本机非参考机）：10 bot×5 min soak 0 panic/0 出界、52 击杀（命中率 21.7 %）、全体 bot step P99 0.032 ms（验收 ≤1）；`think_10/60/100` P99 0.025 / 0.355 / 0.665 ms（预算 1.0 / 3.0 / —）；100 bot 每 tick 路径查询 P99 0.13 ms，单次 A* 中位 23 µs；真实 30 Hz 的二进制 `--bots 10` tick P50 0.22 / P99 0.48 ms、`--bots 100` P50 1.86 / P99 3.56 ms（0 超限）；100 次反应延迟 min 0.400 s（≥0.35）；同种子两次 5000 tick 逐 tick 输入哈希一致。
- 遗留：`step_overhead` 的 debug 100 玩家 P99<1 ms 在共享负载下失败（非本任务，bots=0）；M0 无听觉/掩体/手雷/补给（弹药打光的 bot 退回巡逻）；盲射距离取 12 m 而非任务卡的 80 m（实测依据见 PI-026）；难度档仅 Regular 接线（三档数据已在表里）。
- 影响：**DATA_HASH 改变**（新增 `data/rules/bots.json`；sim-wasm 已重建并同步 `client/public/wasm`）；`sim_thread` 新增 `build_loop_with_bots`/`spawn_with_bots`/`SimLoop::{bot_host,tally}`/`EventTally`（`SimThreadConfig` 签名不变）；`Sessions::reserve_bot`；M0-027 可直接 `--bots 10` 做 10 headless + 10 bot 联测；M1-036/038 替换 `grid_nav`/`SoldierBrain`，保持 `BotBrain`/`BotHost`；黄金哈希 `crates/bots/tests/golden.rs` 在任何 sim 规则/数据调参后需有意更新。

### M0-032 对标测量：M0 批回填 — partial — 2026-11-17 — 提交 （待总控填）
- 做了什么：A 部分全部完成——`tools/ref-analysis/`（ffprobe/抽帧、音频峰值法、火光帧法、网格/像素标定与跟踪、FOV 拟合、Laplacian 与光流压制曲线、统计与置信度、§7.3 容限、记录与 `summary.csv`、JSON spec 一键生成记录）、`docs/measurements/`（模板、25 列表头、README、一页录像清单 `CAPTURE_GUIDE_M0.md`）、我方同法复测 `crates/sim/tests/own_remeasure.rs`（导出 `own_m0.{json,md}`）与回归脚手架 `ref_regression.rs`。B 部分（录像分析与回填）因没有任何录像未做。
- 关键实测：`pytest` 69/69；合成 600 rpm 音频测回 600.04 rpm（误差 0.006 %，验收 ≤1 %）、4.4 m/s 合成视频测回 4.4000 m/s（<0.001 %，验收 ±3 %）、78° FOV 测回 78.016°；我方 Garand 120 rpm/换弹 2.8–3.4 s/开镜 0.30 s、躯干 2 发致死，步兵 2.0/4.4/6.4 m/s（全部在 own_m0.md）。参考作品测量值全部仍为"待测"。
- 遗留 / 需人工：缺批 M0 录像（H-011 / H-025 / H-026，汇总为 NEEDS_HUMAN H-104），录像到位后由 M0-032b 按 `docs/measurements/CAPTURE_GUIDE_M0.md` 回填；PI-027（`_meta.ref` 字段推迟、口径细化、sim 不可测项、docs/measurements 里有其它任务截图）。
- 影响：M0-033 验收时对标值"待测"是预期状态；M1-011/013 实现匍匐/逐发装填后补测 own_m0 里的"不可测"项；M4-044 复用同一工具链。

### M0-027 headless-client v0 与 10 客户端联测 — partial — 2026-11-17 — 提交 （待总控填）
- 做了什么：`crates/headless-client`（库 + 二进制）：WS/WT 真实传输（同机各绑 127.x.y.z 避开每 IP 4 连接）、握手、30 Hz 路点行走/点射/跳蹲合成输入、`SnapshotDecoder` 解码、`PredictSim` 预测 + PI-023 对账（死亡冻结、重生传送）、按秒带宽窗口（线路估计 +58 B/包与纯负载并列）、JSON 报告；`scripts/loadtest-m0.sh`（自起服务器、随机端口、绑核、每 10 s 抓 /metrics 与 RSS、只结束自己的 PID）、`scripts/loadtest-m0-matrix.sh`、`scripts/summarize-loadtest.mjs`；服务器仅追加 `Report.events`；`step_overhead` 计时断言改为 release 才启用。
- 关键实测（release，4 核云主机，服务器/客户端绑不同核，非参考机）：WT 30 min，10 headless+10 bot：tick P50 0.655 / P99 1.215 ms，超限 0，下行 P50 21.7 kbps（预算 128）/ P99 26.8，上行 20.5，0 断线 0 解码错误，击杀 536；WS 10 min：tick P99 1.248，下行 P50 16.9。矩阵 120 s：WT 100 客户端+10 bot tick P50 5.1 / P99 8.3 ms、下行 P50 68 kbps，客户端进程 34 % 单核；20 客户端 P99 1.8–2.5 ms。
- 遗留：服务器 RSS 在 WT 30 min 最后 10 min 增长 7.8 %（验收 <2 %，WS 无此现象，绝对量 1.2 MB，原因未查，PI-028 第 5 点）；`loadtest_smoke`（60 s×3，`--ignored`）已写未长跑；初始同步量化偏移使约 30–50 % 快照仍带容差内 `pos_corr`（+≈1 kbps，PI-028 第 3 点）；演示服务器曾被我误用 `pkill` 按名杀掉，之后只按自己的 PID 结束。
- 影响：M1-068/M2-049 复用 `scripts/loadtest-m0.sh`；M0-030/033 以 `reports/m0/loadtest-wt.json` 为对照基线；浏览器客户端须同样做死亡冻结预测（PI-028）；`target/release/frontline-server` 被我的 release 构建覆盖过（含 `Report.events`）。

### M0-030 测试：M0 性能基线与报告（含三项总控交办修正） — partial — 2026-11-17 — 提交 （待总控填）
- 做了什么：`tools/perf/{capture,report,lib}.mjs` + README（参考机流程）、客户端帧阶段 CPU 计时（`__perf.dump().stages`，schema 2）与固定机位 `render/perf-cameras.ts`、`scripts/perf-regress.mjs`（测量 + 与 `reports/baselines/perf-m0.json` 比较，超容差退出码 1）、`.github/workflows/bench.yml`（手动 + nightly）、`reports/m0/perf-baseline.md`；三项交办修正：①低帧率客户端被误踢（服务器 `too_far` 不再计入 30 连踢，> now+300 的伪造 tick 新增 `far_future` 计入；客户端输入提前量封顶 9 tick）；②WT 服务器 RSS 线性增长（根因：调度器诊断样本向量 + 退出报告样本向量无人读取地累积，已关闭/改为定容蓄水池）；③浏览器客户端死亡冻结预测、重生按全新一命重放（与 headless 一致）。
- 关键实测（非参考机，软件渲染）：draw call 50/33/13、三角形 450 341/9 884/318 492、植被实例 8 130/4 942/3 197（三次逐次一致）；整帧 CPU P50 1.1–1.3 ms；kcc_step 19.22 µs、100 人 tick 2.24 ms；包体积 JS brotli 470 KB、wasm brotli 245 KB。RSS：100 个 WT 客户端斜率 13.7 → 0.6 KB/s，10+10 bot 10 min 增长 7.8 % → 1.6 %（验收线 2 %）。浏览器死亡/重生：死亡期间本地多走 33.7 m、53 次纠正 → 0 m、6 次。60 s 低帧率（≈3 fps）客户端不被踢。
- 遗留 / 需人工：RTX 3060 / Iris Xe 的帧时间与 E0-5 全部未测（H-021，复测命令见 `tools/perf/README.md`）；cam2 的 P50 极差 7.9 %（共享虚机）；RSS 残留约 0.8 KB/s（存活堆已平，未归因，建议 M0-033 做 30 min 复测）；`snapshot_decode` 基准不存在；演示事故的直接触发条件属推断（见 PI-029）。
- 影响：`__perf.dump()` schema 1 → 2（新增 `stages`，`__perf.reset()`）；`input_rejected_total` 新增 `far_future` 标签；服务器退出报告的 tick 分位数改为蓄水池估计（最大值精确）；M1-068/M2-049 可复用 `client/e2e/low-fps.spec.ts` 与 `perf-regress.mjs`；4 个 Rust 基准支持 `FRONTLINE_BASELINE_DIR`；演示服务器仍是旧二进制，需重启才有修正。

### M0-033 M0 里程碑验收与 STATUS 汇总 — partial — 2026-11-17 — 提交 （待总控填）
- 做了什么：不写功能代码。`cargo clean` + 地形重生成（md5 与之前一致）+ wasm-release 重建后，全量重跑 fmt / clippy / `cargo test --workspace` / release 性能测试 / check-deps·purity·layout / data 校验 / client lint·typecheck·vitest·build / 各 Playwright 套件 / 确定性 `--strict` / wasm 体积 / `perf-regress --quick` / 两套 pytest；重跑 30 min WT 压测（M0-030 交接的 RSS 复测）；用真浏览器 + 真服务器走完演示脚本（`reports/m0/acceptance/` 截图与日志）；逐条判定 E0-1…E0-12，写 `reports/m0/ACCEPTANCE.md`。
- 关键实测：退出标准 通过 5 / 部分 5 / 需参考机或人工 2 / 未通过 0；Rust 434 通过 0 失败；30 min WT tick P99 1.349 ms、下行 P50 21.9 kbps、0 断线；**RSS 最后 10 min +1.951 %（险过 2 % 线），斜率 1.29→0.71→0.48 KB/s 递减**；KCC 三次 19.27/20.18/19.94 µs（零余量，PI-030）；确定性 native/wasm 逐位一致。
- 验收内最小修补（PI-031）：`conn.rs` rustfmt 差异；Playwright 主配置误把 vitest 文件 `e2e/interop/fixtures.test.ts` 当用例（自 M0-028 起 `pnpm -C client test:e2e` 整体失败，已 `testIgnore`）；`determinism.yml` 默认严格（PI-019）；DEVELOPING 示例 URL。重生成的跟踪产物：browser-matrix、M0-021-net-e2e.json、determinism/summary.json；STATUS 的 M0-028 行表格数字同步。
- 遗留 / 需人工：H-038（Fable 审阅）、H-021（参考机：E0-3/5/6 真 GPU）、H-102（CI 实跑）、H-011/025/026（录像，H-104）、H-017、H-103、H-022（`tc netem`：本机无 tc，演示步 3 无法执行）；PI-030 待 Fable 裁决；浏览器内未手工复现"被击杀后重生"（headless 564 击杀/563 重生与单测覆盖）。任务卡写的 `pytest tools/map-pipeline tools/ref-analysis` 单次调用不可用，须分目录运行。
- 影响：M1-001 可开工；M1 渲染任务（M1-043/044/049）前必须拿到参考机数据；M1-068 的 RSS 判据建议改为斜率口径并跑 ≥ 1 h；对 M1 的 10 条建议见 `reports/m0/ACCEPTANCE.md` §9。

### M1-001 素材管线：ingest 与许可证闸门 — partial — 2026-11-17 — 提交 （待总控填）
- 做了什么：新建 `tools/asset-pipeline`（TypeScript，CLI `ingest|normalize|report|ledger-check|gen-requirements`）：按列名解析 LICENSES.md §3（缺列/重复/非法状态/管道符均报行号，忽略 AST-EXAMPLE 行）；ingest 闸门（未登记 → 退出码 2、非 OK/FLAGGED → 3、D-LIC 渠道缺确认 → 4、存档文件缺失 → 5，全部通过才复制并校验 sha256）；`assertAllowed`（D-MOD 白名单，越权 → 8）；命名校验（§11.2）；normalize（单位→米、Z-up→Y-up/-Z 前向、原点置底部中心、三角化、同材质合并；骨骼/动画资产只套单位+轴向外壳节点）；缺失素材报告与 `assets/requirements.json`（168 个需求 ID，由 ASSET_SHOPPING_LIST §4 生成）；`.github/workflows/asset-pipeline.yml`。
- 关键实测：vitest 7 文件 / 80 项通过（全套本机 0.5 s）；3 个样例退出码 3/3/2；立方体 1 cm Z-up → 1 m 边长（误差 < 1e-4）；真实 LICENSES.md（4 行：OK 2、PENDING 2）+ 无 incoming → 成功、零文件；M1 范围 72 个需求 ID：real 0 / placeholder 1 / missing 71；CI 作业各命令本机顺序 3.9 s。新增依赖 `@gltf-transform/core 4.5.1`（MIT，传递依赖 property-graph MIT）、devDep `@types/node`；client vitest 238/238 无回归。
- 遗留 / 需人工：验收 7（Actions 墙钟）待 H-102；PI-032（D-LIC 与 LICENSES §6 的免费例外、FLAGGED 视同 PENDING、空 allowed_processing=全放行、源朝向默认约定、requirements.json 最小版本）待 Fable 确认；无需人类采购。
- 影响：M1-002/003/004/005 在 `work/{asset_id}/` 目录布局（`raw/`、`ingest.json`、`normalized.gltf/.bin/.json`）与 `assertAllowed(assetId, code, ledger?)` 上扩展；运行用 Node type stripping（22.18+），或 `pnpm -C tools/asset-pipeline build`。

### M1-002 素材管线：网格 LOD、纹理与碰撞简化 — partial — 2026-11-17 — 提交见 git log

- 做了什么：`process` 命令（`processMesh`）：ORM 打包与 NPOT 补边、KTX2（ETC1S/UASTC，法线 RRRG 两通道）、meshoptimizer LOD 链（锁 UV 接缝）、gltf-transform 量化 + meshopt 压缩、碰撞简化（trimesh/convex）、`stats.json`；`_fp`/`_tp` 两套独立链；`check-size-ratio` 子命令；`test:e2e`。
- 关键实测：20k 三角球体 LOD 比例偏差 0 %；包围盒误差 < 4e-6；碰撞 400 三角 / ≤ 32 块；全流程 8.6–16.1 s；体积比样例 A 18.3 %、样例 B 31.5 %。
- 遗留 / 需人工：真实 CC0 样例未入库；体积比对法线图占比敏感；`toktx` 换为 WASM 编码器等取舍见 PI-033，待 Fable 确认；CI 墙钟 H-102。
- 影响：M1-003 读 `work/{asset_id}/lod{N}{_fp|_tp}.glb`、`collision.glb`、`stats.json`；M1-006 需按 RRRG 还原法线。

## 8. 如何维护本文件

- **执行者**：收工前，在"任务日志"末尾追加本任务条目（`### 任务 ID 标题 — 状态 — 日期 — 提交 abc1234`，3–6 行：做了什么、关键实测数字、遗留问题/需人工事项、对后续任务的影响）；写入前重新读取文件末尾（并行执行者可能同时追加），只追加、不改别人的条目。数字必须来自实测，查不到写"未记录"。同时更新 `tasks/STATUS.md` 自己那一行。
- **总控**：每波任务后更新本文件顶部的里程碑表、"M0 概况"表、"退出标准对照"、"关键实测数字"与"卡在人这边的事项"，并同步 `tasks/STATUS.md` 顶部摘要；新裁定追加到"已生效裁定摘要"；定期整理（合并重复、把已解决的人工事项划掉）。
- **不在本文件做的事**：不复制 STATUS 的完整实测长文；不记录架构变更（走 `docs/PLAN_ISSUES.md`）；不改任务范围（走 `tasks/INDEX.md`）。
