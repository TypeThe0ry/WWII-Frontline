# 任务状态表（tasks/STATUS.md）

> 执行者每完成/推进/阻塞一个任务，必须更新本表对应行与顶部摘要，并随该任务的提交一起提交。
> 状态枚举：`todo` / `in-progress` / `done` / `partial` / `blocked`（定义见 `tasks/README.md` §4）。
> 任务清单与 `tasks/INDEX.md` 一致；新增/拆分任务须**先改 INDEX 再改本表**。

## 摘要

- **当前里程碑**：M0 技术验证
- **下一个任务**：`M0-001` repo-scaffold（依赖已满足、编号最小的 todo 任务）
- **blocked 清单**：（无）
  - 格式：`任务 ID — 所缺条目 — 登记日期 — 谁来解除（人类/素材/其它任务）`
- **partial 清单**：（无）
  - 格式：`任务 ID — 已完成部分 — 遗留项 — 对应 PLAN_ISSUES 编号`
- **最近更新**：（执行者填写：日期 / 任务 ID）

## 列说明

- **实测结果**：验收标准要求的**实测数字**（如"tick_ms P99 = 21.4 ms，100 客户端×30 min"），附测量命令或报告路径；不得只写"通过"。
- **遗留问题**：未解决的偏差/风险，写 `docs/PLAN_ISSUES.md` 条目编号；无则留空。
- **完成提交**：完成该任务的 commit 短哈希（多个提交用逗号分隔）。

## M0 技术验证（M0-001 – M0-033）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M0-001 | 仓库脚手架与工具链 | done | 1) `RUSTFLAGS="-D warnings" cargo build --workspace` 退出 0、警告 0；2) `cargo clippy --workspace --all-targets -- -D warnings` 退出 0；`cargo fmt --check`、`cargo test --workspace` 通过（8 个 smoke 测试）；3) `pnpm -C client build` 退出 0 无 warning（dist 0.75 kB js），`tsc --noEmit` 0，`pnpm lint` 0；4) `cargo metadata` 内部依赖边 14 条，与 ARCHITECTURE §4.5 允许集合逐条比对完全相等（sim→protocol；sim-wasm→sim,protocol；bots→sim,protocol；server→sim,protocol,bots；gateway→protocol；headless-client→sim,protocol；balance-sim→sim,protocol,bots；protocol 无）；5) `node tools/ci/check-layout.mjs` 退出 0（24 目录+4 文件）；6) `git check-ignore` 三路径均被忽略；7) 两 lockfile 已生成待入库，`@babylonjs/core` 精确 8.56.2。版本：Rust 1.97.0、Node 22.23.4、pnpm 10.28.0、vite 8.3.8、TS 6.0.3、vitest 5.1.0、eslint 10.15.0、preact 11.2.0、@preact/signals 2.11.4。 | BRIEF 文档不存在，目录清单按 ARCHITECTURE §4.1；check-layout 不要求 gitignore 的 assets/incoming、processed（新增 assets/.gitkeep）；TS 取 6.0.3（typescript-eslint 8.73 要求 <6.1，故未用 7.x）；Babylon 取 8.x 最新（最新大版本已是 9.x） | |
| M0-002 | GitHub Actions 基线与依赖图检查 | partial | | 已建 lint/test-rust/test-ts/wasm-build/data-validate(占位) 5 个 workflow、deny.toml、tools/ci/{dep-matrix,check-deps,check-sim-purity}.mjs + fixtures + 11 个 node:test（全过）。本地逐 step 验证：fmt/clippy/eslint/tsc/vitest/check-*/cargo deny(bans+licenses+sources) 见下；check-deps 对当前仓库 exit 0；真实 cargo metadata 沙盒副本中加 sim→tokio、gateway→sim 均 exit 1 且日志含 `sim -> tokio`（protocol→sim 被 cargo 自身环检测拒绝，改用 fixture 验证）。purity 扫描违规 exit 1，purity-allow 通过并列出。 **未在 GitHub 上实跑**：验收 1 的真实 PR 红绿对、验收 5 的 Actions 墙钟（空/热缓存）未取得；`cargo deny` 的 advisories 检查本机无法访问 github.com/RustSec，未验证（CI 上首次运行需确认）。 本机并行他人代码导致：`cargo fmt --check`（protocol 在途）、eslint/vitest（client/src/render 在途）暂失败，非本任务范围。 遗留：缓存选 Swatinem/rust-cache 而非 sccache（H-014 前无远端存储）；deny 许可证清单待 H-037；wasm-build 在 sim-wasm 无 wasm-bindgen 前跳过（M0-014 去掉）；cargo-deny/第三方 action 版本未在 GitHub 上验证可用。 | |
| M0-003 | 字体许可证核实与登记（R10） | done | 4 字体各 1 行：Barlow、Noto Sans SC 为 OK（RFN 分别为无、Source，族名不冲突）；IBM Plex Sans/Mono 为 PENDING（RFN "Plex"，子集化须改名，见 PI-009）。licenses/texts 4 文件 4377-4456 字节；Noto Sans SC 可变 TTF 实测 17,772,300 字节；font-subsetting.md 已写 | PI-009 待裁决；LICENSES §5 字体行仍写"未核实"（未改） | |
| M0-004 | /data schema 与校验工具 | partial | 1) invalid fixtures 23 个（覆盖 weapon/role/loadout/vehicle/map_meta/spawns；缺字段、错单位 kph、无单位、范围越界、引用不存在、falloff 非递增、tunable 路径不存在等），`pnpm -C tools/data-validate test` 42/42 通过，其中对每个 invalid 样例单独起 CLI 子进程：退出码 1 且输出含 `_expect_error`；8 个 valid 样例全过（`node tools/data-validate/index.mjs --fixtures`）；2) 真实 `/data` 退出码 0（`--root data`，M0 仅含 schemas+fixtures+README）；3) `cargo test -p frontline-sim data_`（注意包名是 frontline-sim）7/7 通过，含 `data_hash_stable`（同内容两次/打乱文件序/改缩进与键序相等；改一个值或改文件名不等）与 `data_hash_matches_tree`；4) 在 scratchpad 副本中放入缺 `historical._source` 的 weapon 文件，`cargo build -p frontline-sim` 退出 101 `data validation failed: weapons/us_fixture_rifle.json: missing field `_source``（未入库）；5) CI 墙钟：本机等价步骤（validator 测试 4.9 s + fixtures + validate <0.5 s）约 6 s，远小于 2 min；仅 `data/**`、`tools/data-validate/**` 触发（.github/workflows/data-validate.yml 已接入）。构建增量：build.rs 重跑 ≈0.26 s（<3 s）。`cargo fmt --check` / `clippy --workspace --all-targets -D warnings` / `cargo test --workspace` 通过；wasm32 目标可编译。**当前 DATA_HASH = 0x141cfc9842c4b0e3**（/data 无数据文件时的空集哈希；M0-008 起会变，属预期）。 | 未在真实 GitHub Actions 上跑：CI 作业墙钟与“故意提交坏 JSON 的 PR 被拒绝一次”需总控推送后在 GitHub 上记录（故标 partial）；PI-002（字段命名文档冲突）、PI-003（schema 细化）、PI-004（serde/serde_json/sha2 依赖、包名 frontline-sim）；rules/*.json 与 maps/*/map.json 暂无 schema，仅进哈希；.github/workflows/data-validate.yml 由 M0-002 创建占位，此处已改写；根 package.json 追加 data:validate / data:test；pnpm-lock 新增 tools/data-validate（ajv 8.20.0） | |
| M0-005 | protocol：位流、量化与 M0 消息集 | done | 包名 `frontline-protocol`（`-p frontline-protocol`）。1) `--release -- --ignored roundtrip_1m` 10^6 样本（InputCmd/InputBundle/Hello/Welcome/Ping/Pong 混合）全部往返相等，1.5 s；`PROPTEST_CASES=10000` 快速版通过；2) 10^6 随机样本最大量化误差：x/z/y 均 0.015625 m（=1/64）；yaw 10 bit 0.176°、8 bit 0.703°；pitch 8 bit 0.3530°（略大于 0.35°，见遗留）、9 bit 0.176°、16 bit 0.0014°、i16 0.0014°；3) 典型 3 条走动 InputBundle = 29 B（≤30），最坏全字段突变 49 B；4) fuzz_smoke：2×10^5 随机串 + 全部前缀截断 + 全部单比特翻转，0 panic；ticket=257 的 Hello 被拒（TooLong）；5) `grep -R unsafe crates/protocol/src` 为空（由 workspace lint forbid 保证，已验证加 unsafe 即编译失败）；`cargo build --target wasm32-unknown-unknown` 通过；`cargo tree` 仅 arrayvec 0.7.8。性能（--release，仅记录）：InputBundle 编码 141 ns、解码 137 ns。`cargo fmt --check`、`cargo clippy -p frontline-protocol --all-targets -D warnings`、`cargo test -p frontline-protocol` 通过（13 单元 + 6 limits + 6 roundtrip；golden bytes 8 个）。未做 criterion 基准（可选项，用 ignored 测试 bench_codec 代替）。 | PI-001：Welcome 加字段、InputBundle 差分位宽、origin_hint 含义待 M1、pitch 8 bit 误差 0.353° vs 0.35° | |
| M0-006 | protocol：快照编码与基线增量 | done | 先落实 PI-001 裁定：新增 `PITCH_BITS_EVENT=9`（Shot.dir_pitch；M0 唯一的 8 bit pitch），limits 测试更新；InputBundle 无需改（pitch 为 i16），重测典型 3 条 = 29 B、最坏 49 B，golden 不变；PI-001 已标 resolved 并追加裁定。`crates/protocol/src/snapshot/*` + tests/{snapshot_prop,snapshot_size}.rs + fuzz/。 1) 属性测试（--release，各 10^5 次，随机 1..60 实体、关键帧/增量/移除/新增混合）全部通过：关键帧路径 `decode(encode(b,None))==b`；增量路径 `decode(encode(b,Some(a)),Some(a))==b`（近景精确，远景 yaw 8 bit 量化且 pitch 取基线，测试按此预期；约 258 万 delta + 50 万 keyframe 记录）；debug 下默认 10^4。 2) cargo-fuzz（nightly 1.102.0-nightly，本机新装 cargo-fuzz 0.13.2）：`snapshot_decode` 601 s、637 万次执行，`input_bundle_decode` 331 s、3770 万次，均 0 crash/panic/OOM（artifacts 为空）；日志 reports/m0/M0-006-fuzz-{snapshot_decode,input_bundle_decode}.log。 3) 字节数（60 s 内存回放，4 m/s 直线，20 Hz，ack 间隔≤2 tick）：近景 delta 纯走动 4.12 B/记录、边走边转 6.12 B（目标 ≤10）；远景 4.53 B（目标 9）；ack 晚 3 个快照（≈100 ms RTT）仍 6.12 B；关键帧 12.50 B（≤13）；self_state 7 B（含 pos_corr 13 B，≤14）；事件 Shot 98 bit ≈ 12.3 B/条。 4) 40 实体（近 4+中 10 用近景格式 + 远 26，delta，含 self_state 与 5 条 Shot）平均 282 B、最大 287 B（≤600）；40 实体全关键帧 512 B；预算 60/100/333/700/1150 B 下 `add_entity` 返回 `Ok(false)` 且已写内容可解码（含 127 记录上限）。对照 ARCHITECTURE §6.9：20 Hz × 282 B ≈ 45 kbps 载荷（未含 58 B/包传输开销），均值预算 256 kbps 内。 5) 基线环：ack 落后 32 tick 仍为 delta，33 tick 全部关键帧（断言记录类型）；30% 下行丢包 + 30% ack 丢包 60 s：849 解码/351 丢，0 次 MissingBaseline，最终与服务器一致。 性能（--release，`--ignored bench_snapshot`）：snapshot_encode_client（60 实体，30 近 + self）≈ 5.6 µs/客户端（含建输入 6.2 µs；预算 25 µs），snapshot_decode 6.3 µs。检查：`cargo fmt --check`、`cargo clippy -p frontline-protocol --all-targets -D warnings`、`cargo test -p frontline-protocol`（20 单元 + 6 limits + 6 roundtrip + 7 snapshot_prop + 4 snapshot_size，含 3 个新 golden/2 个快照 golden）通过；wasm32 构建通过；无新增依赖（fuzz 工程独立 workspace，依赖 libfuzzer-sys）。 | PI-010：快照位宽/基线语义细化（单一基线 tick、记录类型、位置大小类阈值改为 32/256/2048 步、远景不发 pitch）；M0-019 应让近/中景实体每包都出现，否则多发关键帧（12.5 B）；CI 只需跑 fuzz 60 s 冒烟（`cargo +nightly fuzz run <target> -- -max_total_time=60`）；criterion 未用，以 ignored 测试代替。 | |
| M0-007 | sim：World/step/确定性底座 | done | 1) `cargo test -p frontline-sim --features test-util` 的 `determinism_basic`：2 次 10 000 tick（各在独立线程、其间扰动堆分配）逐 tick 相等 10000/10000，最终 hash golden=0x700dc1b825c9ae38；2) 100 次扰动（50 次改 match_seed、50 次翻转某玩家某 tick 一个按钮位）在 ≤2 tick 内 hash 出现差异 100/100；3) check-sim-purity OK（0 waiver）、check-deps OK、`cargo tree -p frontline-sim --target all -e normal` 无 tokio/rayon/wasm-bindgen/getrandom；4) wasm32 构建（含 `--features client-predict`）通过；5) 乱序输入：debug 构建 panic（should_panic 单测）、release 构建结果与已排序一致（`--release` 单测）均通过；6) 槽位复用代数+1、旧 id 查询 None 单测通过。基准 `cargo bench -p frontline-sim`（release，本机软件环境，仅记录）：world_step_empty 10 玩家 ≈0.31 µs/step、100 玩家 ≈2.78 µs/step。fmt、clippy `-D warnings`（`--no-deps --all-targets --features test-util`）通过；lib 单测 17 + determinism 5 全过。 | bevy_ecs 只能锁 =0.16.1 且不开 `std`，否则 wasm32 传递依赖 wasm-bindgen/tokio 使 check-deps 失败（PI-011）；determinism 测试需 `--features test-util`（CI 须带该参数）；`WorldView::entity_state` 暂返回 sim 自有 `EntityStateView`（protocol 尚无导出的 EntityState）；los/height_at/team_resources 为占位（M0-010/013）；`cargo clippy -p frontline-sim` 不带 `--no-deps` 时会 lint 并行中的 protocol（M0-006）而失败，非本任务。 | |
| M0-008 | 数据：M0 步枪（Kar98k、M1 Garand） | done | 1) `node tools/data-validate/index.mjs --root data` 退出 0（checked 2 files, 0 error）；`--fixtures` 8 valid/23 invalid 通过；`pnpm -C tools/data-validate test` 42/42（连跑 3 次全过；改 schema 后第一次曾见 1 项失败，未复现、原因未查明）；2) 两文件 `_meta.status=initial`，`grep -c _source` 各 1；3) 逐项对照 GAMEDESIGN §6.2–§6.6：差异 0 项（由 `crates/sim/tests/data_weapons_m0.rs::data_weapons_m0_load` 断言全部数值）；4) 躯干 STK（HP 100）：Garand 0/100/200/300/600/800 m 均 2 发（每发 70→50.4 HP）；Kar98k 0/100/200 m 为 1 发，201 m 起 2 发（201 m 99.97 HP；300 m 96.7；800 m 80.0），与 §6.6 一致，已作为快照测试 `data_weapons_m0_stk_snapshot`；5) DATA_HASH 本任务后 = **0x8e4ea0ee2af4bf8f**（此前空集 0x141cfc98…），清理后重建两次相同。测试 2/2 通过；fmt/clippy -D warnings 在隔离副本通过。 | 当前工作区 `cargo test -p frontline-sim` 无法编译（M0-007 进行中：rng.rs/schedule.rs 报错，非本任务代码），故 Rust 侧在 scratchpad 隔离副本（同一份 data/、data*.rs、build.rs、测试）验证；M0-007 稳定后总控应重跑一次 `cargo test -p frontline-sim data_weapons_m0`。schema 追加可选字段 `part_mult.pelvis`、顶层 `fx{muzzle_flash_ms,tracer}` 并同步 data_defs.rs（PI-007）；pelvis 缺省回退规则交 M0-012；步枪 `caliber`/`cartridge` 拆分写法与 `magazine_type` 取值（en_bloc_clip / stripper_clip_internal_box）为本任务自定。同时完成 PI-002/003/004 裁决回填与 ARCH §8.5 变更说明。 | |
| M0-009 | 地图：M0 单块地形（256 m）数据管线 | done | 1) `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1` 全流水线 0.3–0.7 s；heightmap_1m 132098 B、far_8m 2101250 B、16 个 b_*.bin 各 8450 B、16 个 c_*.bin 各 12562 B（与任务公式一致），共 2.6 MB；hmin/hmax=1/11 m（实际 1.71–10.20 m）。2) `validate data/maps/m0` 退出 0；`node tools/data-validate/index.mjs` 0 错误；`pnpm data:test` 42 通过；`cargo test -p frontline-sim data_` 通过。3) pytest 21 通过 1 skip（合成 GeoTIFF 用例，本机无 GDAL）；test_determinism 两次运行 37 个文件 SHA-256 全同。4) 区块边界 65 样本逐一相等；远景中心与近景在 8 m 抽样点差 0 量化步。5) 最大坡度 38.7°，>35° 的土坎沿 z 向连续 9 个样本、沿 x 向多段 ≥8；<5° 平地占 88.5%，含 64×64 m 的两块平台（出生点所在）。SHA-256：meta.json 32bc28fc917371778a24d1f59af37f251c64a38da3b9bb7bbe08b8303a542f90、spawns.json 75ea6d44925402d05a61ffbf93ebb91a668d4fa26bd438d748dd040f60cca4b7、heightmap_1m.r16 76fe2d58af3bc864c8cf7d2889d29ec8c5685cf968aa5b7e3554be6bddd44756、heightmap_far_8m.r16 1c958f6f179088893cd10b260cc4de67c73425884977a54e717b936bf85b1d30（完整值见 data/maps/m0/manifest.json 与本地 sha256sum）。产物：meta/spawns/manifest.json 入库；*.r16 与 chunks/ 已 .gitignore，重生成命令见 DEVELOPING.md。DEM 路径（GDAL）本机未装，仅清晰报错路径与窗口计算有单测，未实跑。 | PI-007 | |
| M0-010 | sim：高度场碰撞 | done | 机型：Xeon 2.3 GHz 云主机（4 核，release）。1) `cargo bench -p frontline-sim --bench collision`：collision_raycast 10 000 条种子射线（≤800 m，M0 地形）中位数 1.47 ms（<5 ms；最坏情形 6 m 高水平射线 3.14 ms；ground_probe 约 52 ns/次）；2) `slope_limit_35deg`（Rapier KCC，胶囊 r0.35/h1.8，4.4 m/s×5 s，max_slope_climb=35°）：36/40/45/55° 坡增高 0.02 m（≤0.5）；10/20/25/30° 坡增高 ≥100% 坡高；M0 真实路堤 38.7° 增高 0.02 m，其缓侧（≈19°）可爬 2.5 m；3) 反量化：1000 点 height_at 与 r16 双线性参考最大差 1.1e-6 m（≤1e-4）；4) 接缝：1000 个交点邻域样本 1 cm 差均 ≤ 全局最大样本步长×1 cm+1e-4，缺块 None；5) `collision_parse_fuzz` 10^5 例（31684 Ok/68316 Err）无 panic，合法解析 ≤21 KB（先验长度再分配，无法在 forbid(unsafe) 下装计数分配器，按设计保证）；6) 同数据构建两次/克隆/乱序插入：Rapier 碰撞体句柄顺序与 `CollisionData::state_hash` 全等。另：自写射线与 Rapier cast_ray 3000 条中 2999 条一致（1 条 Rapier 漏检，暴力步进裁定自写正确）；M0 全 16 块驻留 0.336 MB（≤5 MB）；wasm32 构建、check-deps、check-sim-purity、fmt、clippy -D warnings 通过；sim 测试（带 test-util）全过。CI：test-rust.yml 增加 `cargo test -p frontline-sim --features test-util --locked` 一步。 | PI-013（Rapier =0.36.1 + enhanced-determinism；查询自写；`from_bytes(bx,bz,..)`、`height_at->Option`）。地形二进制缺失时测试/基准打印重建命令后跳过（`FRONTLINE_REQUIRE_TERRAIN=1` 则失败）。native↔wasm 逐位一致性待 M0-014/029 比对；未用 criterion；cargo-deny 本机未安装未实跑。 | |
| M0-011 | sim：Rapier 角色控制器与基础移动 | partial | 机型：本机 Xeon 2.3 GHz 云主机 4 核、**非参考机**、测量时系统负载均值≈6.7（release，enhanced-determinism，Instant 自制 harness，非 criterion）。1) `cargo bench -p frontline-sim --bench kcc`：kcc_step 中位数 18.6 µs（均值 22.8，p99 96.3；预算 ≤20，余量仅≈7%）；Rapier 自带 snap_to_ground 使其 ≈41 µs，故改为解析 snap（PI-015）；2) kcc_100_players_tick（100 人随机走/跳/蹲/冲刺，M0 地形，整个 World::step）中位数 2.14 ms（≤3），p99 2.63 ms；基线 reports/baselines/sim-m0.json；3) `speed_matches_data` 平地 3 s：walk 2.000/run 4.400/sprint 6.400/蹲跑 2.400/蹲走 1.500 m/s，相对误差均 <0.01%（另测对角不超速、后退冲刺按跑速）；4) 跳跃峰值 0.5987 m（数据 0.6，误差 0.22%），空中 20 tick，落地 grounded=true；5) 坡度：>35°（36/38.7/45/55/70°）跑 5 s 高度增益 ≤0.040 m（M0 38.7° 堤坝 0.033 m）；≤30° 可上，水平速度占比 10°97%/20°88%/25°82%/30°75%（34° 69%）；M0 缓坡爬升 2.5 m、均速 4.23 m/s；6) 确定性：`movement_replay_10k` 10 000 步（含随机跳/蹲/冲刺）与另一线程重建碰撞世界的结果逐步位相同，golden 轨迹哈希 0x04275f5ff85febd8；`movement_world_vs_predict` World 与 PredictSim 3 000 tick 位相同（合成与 M0 地形）。加速/减速：即时（1 tick）；站↔蹲 250 ms=7.5 tick，第 8 tick 完成；空中控制 0.3/tick。`cargo test -p frontline-sim --features test-util`（FRONTLINE_REQUIRE_TERRAIN=1）全过；clippy -D warnings、fmt、wasm32（含 client-predict）、check-deps、check-sim-purity、data-validate（47/47）通过。`determinism_basic` golden 因 state_hash 折入移动状态而重钉为 0x63ac47f15c2331d8。 | 仅差参考机复测 KCC 两项耗时（本机非参考机且余量小，记 partial，见 PI-015）；空碰撞下玩家不动（server 用 `CollisionData::empty()` 时）；`aux` bit0=走路键、其余位待 M0-012；平滑加速待 REF M-24–M-26；PlayerAuthState 字节布局 39 B 供 M0-014/022 | |
| M0-012 | sim：步枪射击、弹道与装填 | todo | | | |
| M0-013 | sim：生命、死亡与简易重生 | todo | | | |
| M0-014 | sim-wasm：WASM 封装 | todo | | | |
| M0-015 | server：进程骨架与 30 Hz 主循环 | partial | 验收 2-6 通过，验收 1 抖动项未达标。10 min 空载（release，4 核 Firecracker 虚机，Xeon 2.3GHz，内核 6.18.44，期间 load avg 4-8，其它执行者并行编译/测试）：17995 tick，server_tick_ms P50 0.006 / P99 0.017 / max 5.34 ms，tick_overrun_total=0，consecutive_max=0；tick_interval_error P50 0.0003 / P99 2.83 / max 64 ms（跳过 6 个时间槽）。100 玩家空世界 step P99<1 ms（debug 构建，集成测试 step_overhead）；SIGTERM 退出码 0 且 <100 ms；日志 JSON + 限速、假时钟（50 ms tick 后无追赶/无漂移，连续 3 次超限 max=3）单测通过；/metrics 桶数 13。 | 抖动 P99 < 1 ms 未达标：共享虚机负载所致，需在参考机/独占物理机复测（H-021），故 partial。--pin-sim-core 仅接受参数（M1-067）；--bots 仅告警、server_bots 恒 0（M0-026）；WS/WT 端口未绑定（M0-016/017）；apply_inputs 仅用本 tick 新到输入（M0-018）；无新增 PLAN_ISSUES。 | |
| M0-016 | server：WebSocket 传输与握手 | todo | | | |
| M0-017 | server：WebTransport 传输 | todo | | | |
| M0-018 | server：会话、输入缓冲与校验 | todo | | | |
| M0-019 | server：AOI v0 与快照发送 | todo | | | |
| M0-020 | client/render：引擎初始化与调试叠层 | partial | 2026-11-17 无 GPU 环境（4 核 Xeon 2.3 GHz，Chromium 软件渲染）实测：①验收1 `pnpm -C client test:e2e` 5/5 通过：`?renderer=webgl2` 5 点（中心+4 角）与清屏色差>30 且 60 帧后变化≤6，无 console error/pageerror；WebGPU 真机项**未测**。②验收2 `__perf.dump()` 固定 16 字段 schema 已测（vitest+e2e）；WebGPU(SwiftShader)路径 `gpu_ms.source=timestamp-query` 有值，**数值为软件适配器耗时，真 GPU 未测**；WebGL2/SwiftShader 无可用计时则回退 `interval-estimate`（SwiftShader 上 EXT_disjoint_timer_query 可用但读数~0.0006 ms，不可信）。③验收3 compute 冒烟在 SwiftShader WebGPU 上 `__smoke.ok=true, mismatches=0, 782 ms`，Hi-Z/froxel(921600/921600)/TAAU 三试验均通过；真 GPU **未测**；结论见 reports/m0/compute-feasibility.md。④验收4 FixedStepper vitest 通过（100 ms→3 步；1000 ms→5 步且 clamped+1；1 h 随机 dt 偏差≤1）。⑤验收5 `navigator.gpu=undefined` auto→WebGL2，页面导航起 219 ms 内就绪（≤2 s），叠层显示 `fallback: navigator.gpu is undefined`。⑥验收6 1920×1080 空场景 60 s（600 帧窗口，WebGL2/SwiftShader）CPU 帧时间 P50 0.30 ms、**P99 0.60 ms**（≤4 ms）、60.0 fps；WebGPU(SwiftShader+shim) P99 0.50 ms；仅软件渲染，参考机未测。检查：`pnpm lint`/`typecheck`/`test`(18 项)/`build` 全绿。 | PI-005（compute 需自写薄封装；无头 WebGPU 无法 present，e2e 用 `client/e2e/webgpu-swiftshader-shim.js` 离屏）、PI-006（步长取 1000/30）；WebGPU 真机帧时间/像素/compute 需参考机复测（H-021）；WebGPU 路径无像素校验 | |
| M0-021 | client/net：传输与时钟同步 | todo | | | |
| M0-022 | client/game：输入、预测、对账与插值 | todo | | | |
| M0-023 | client/render：CDLOD 地形 | todo | | | |
| M0-024 | client/render：植被实例化与 impostor | todo | | | |
| M0-025 | client/render：灰盒角色、第一人称枪与射击反馈 | todo | | | |
| M0-026 | bots：M0 基础 bot | todo | | | |
| M0-027 | headless-client v0 与 10 客户端联测 | todo | | | |
| M0-028 | 测试：浏览器 WT/WS 互通矩阵 | todo | | | |
| M0-029 | 测试：确定性与 WASM 漂移 | todo | | | |
| M0-030 | 测试：M0 性能基线与报告 | todo | | | |
| M0-031 | 史实参数复核：Kar98k 与 M1 Garand（R8） | done | 1) 两文件 historical 共 15 字段，13 已核实（有 locator 的 _source 记录）+ 2 未核实（Kar98k 初速、射速，登记 _meta.unverified）= 15；`node tools/data-validate/index.mjs` 退出 0（hist-source-required 开启）；2) 汇总表 reports/m0/hist-review-m0.md；3) 一手来源：TM 9-1275、FM 23-5（战后版）、TM-E 30-451；4) 数值改动：M1 实用射速 40→24，新增 effective_range_m=460；gameplay 未改；5) `pnpm -C tools/data-validate test` 47/47；`cargo test -p frontline-sim` 全绿、clippy -D warnings、fmt 干净（data_weapons_m0.rs 改为 status=Reviewed 并加 historical 断言）；DATA_HASH 无字面量断言；6) 无原文入库。遗留：Kar98k 一手来源（HDv/Hogg/博物馆）未找到；两 status 现为 reviewed；见 PI-012 | | |
| M0-032 | 对标测量：M0 批回填 | todo | | | |
| M0-033 | M0 里程碑验收与 STATUS 汇总 | todo | | | |

## M1 垂直切片（M1-001 – M1-072）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M1-001 | 素材管线：ingest 与许可证闸门 | todo | | | |
| M1-002 | 素材管线：网格 LOD、纹理与碰撞简化 | todo | | | |
| M1-003 | 素材管线：.fpak 分块容器（D-PAK） | todo | | | |
| M1-004 | 素材管线：音频处理与打包 | todo | | | |
| M1-005 | 素材管线：manifest 与体积预算门禁 | todo | | | |
| M1-006 | client：fpak 加载、解码 Worker 与缓存 | todo | | | |
| M1-007 | 数据：M1 五兵种、loadout 与基础规则 | todo | | | |
| M1-008 | 数据：M1 武器与投掷物 | todo | | | |
| M1-009 | 数据：M4A1 Sherman（M1 的 1 辆坦克） | todo | | | |
| M1-010 | protocol：M1 全消息集 | todo | | | |
| M1-011 | sim：完整移动（姿态、体力、翻越、探身） | todo | | | |
| M1-012 | sim：弹道细化与穿透 | todo | | | |
| M1-013 | sim：冲锋枪、卡宾、手枪与近战 | todo | | | |
| M1-014 | sim：命中区与伤害模型 | todo | | | |
| M1-015 | sim：手雷与烟雾 | todo | | | |
| M1-016 | sim：倒地、救起与击杀归属 | todo | | | |
| M1-017 | sim：压制 | todo | | | |
| M1-018 | sim：据点占领与对局状态机 | todo | | | |
| M1-019 | sim：HQ 出生与 Garrison | todo | | | |
| M1-020 | sim：Supplies 与补给箱 | todo | | | |
| M1-021 | sim：小队、兵种名额与部署校验 | todo | | | |
| M1-022 | sim：载具物理（悬挂、履带、座位） | todo | | | |
| M1-023 | sim：载具武器与装甲判定（含 Panzerfaust 60 发射与 HEAT 判定） | todo | | | |
| M1-024 | server：AOI v1（敌友规则、团队通道） | todo | | | |
| M1-025 | server：延迟补偿 | todo | | | |
| M1-026 | server：部署/小队处理与可靠事件 | todo | | | |
| M1-027 | server：基础校验（速度/射速/视角） | todo | | | |
| M1-028 | gateway：服务器列表、游客鉴权与 ticket | todo | | | |
| M1-029 | server：注册、心跳与 ticket 验证 | todo | | | |
| M1-030 | 地图：Carentan 数据源许可证与锚点对账 | todo | | | |
| M1-031 | 地图：Carentan 1/3 区域高度图 | todo | | | |
| M1-032 | 地图：OSM 矢量提取与清洗 | todo | | | |
| M1-033 | 地图：1944 修正（M1 区域） | todo | | | |
| M1-034 | 地图：Carentan 1/3 关卡标注与建筑足迹 | todo | | | |
| M1-035 | 地图：区块烘焙、splat 与植被分布 | todo | | | |
| M1-036 | 地图：Recast 导航网格烘焙 | todo | | | |
| M1-037 | bots：navmesh 寻路与避障 | todo | | | |
| M1-038 | bots：行为树与效用 AI | todo | | | |
| M1-039 | bots：小队长 bot、兵种选择与填充 | todo | | | |
| M1-040 | client/game：游戏状态与事件总线类型 | todo | | | |
| M1-041 | client/net：载具驾驶预测与同步 | todo | | | |
| M1-042 | render：PBR 材质系统与变体管理 | todo | | | |
| M1-043 | render：光照、CSM 阴影与天空 | todo | | | |
| M1-044 | render：TAA、曝光与色调映射 | todo | | | |
| M1-045 | render：骨骼角色与动画状态机 | todo | | | |
| M1-046 | render：第一人称武器 pass | todo | | | |
| M1-047 | render：粒子与贴花基础 | todo | | | |
| M1-048 | render：坦克渲染 | todo | | | |
| M1-049 | world：区块流式加载 | todo | | | |
| M1-050 | world：地形 splat 材质与宏观色 | todo | | | |
| M1-051 | world：树篱、植被与道具 | todo | | | |
| M1-052 | world：建筑与可进入室内 | todo | | | |
| M1-053 | UI：框架、设计 token 与 i18n | todo | | | |
| M1-054 | UI：启动加载与主菜单 | todo | | | |
| M1-055 | UI：服务器浏览器与阵营选择 | todo | | | |
| M1-056 | UI：部署界面 | todo | | | |
| M1-057 | UI：核心 HUD | todo | | | |
| M1-058 | UI：战术地图 | todo | | | |
| M1-059 | UI：倒地与阵亡界面 | todo | | | |
| M1-060 | UI：Officer/Support 放置与载具进出 | todo | | | |
| M1-061 | UI：暂停菜单与基础设置 | todo | | | |
| M1-062 | 音频：引擎核心 | todo | | | |
| M1-063 | 音频：枪声分层、脚步与命中 | todo | | | |
| M1-064 | 素材接入：M1 视觉素材 | todo | | | |
| M1-065 | 素材接入：M1 音频与字体 | todo | | | |
| M1-066 | 运维：本地开发栈 | todo | | | |
| M1-067 | 运维：预发布部署 | todo | | | |
| M1-068 | 测试：20 客户端联测 | todo | | | |
| M1-069 | 测试：M1 性能基准 | todo | | | |
| M1-070 | 史实参数复核：M1 武器与 Sherman（R8） | todo | | | |
| M1-071 | 对标测量：M1 批回填与我方复测 | todo | | | |
| M1-072 | M1 里程碑验收与 STATUS 汇总 | todo | | | |

## M2 完整 Frontline 模式（M2-001 – M2-055）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M2-001 | 地图：Carentan 全图高度图与矢量 | todo | | | |
| M2-002 | 地图：Carentan 1944 修正（全图） | todo | | | |
| M2-003 | 地图：建筑样式与地标（全图） | todo | | | |
| M2-004 | 地图：Carentan Frontline 关卡标注 | todo | | | |
| M2-005 | 地图：全图区块、碰撞与导航烘焙 | todo | | | |
| M2-006 | 数据：M2 全步兵兵种、武器与基础载具 | todo | | | |
| M2-007 | protocol：M2 消息（建造、资源、投票、Ping） | todo | | | |
| M2-008 | sim：Frontline 模式规则 | todo | | | |
| M2-009 | sim：战略资源与资源节点 | todo | | | |
| M2-010 | sim：Engineer 建造系统 | todo | | | |
| M2-011 | sim：Outpost、维修站与反坦克炮 | todo | | | |
| M2-012 | sim：M2 兵种特有能力 | todo | | | |
| M2-013 | sim：M2 武器类（自动步枪、机枪、霰弹） | todo | | | |
| M2-014 | sim：火箭、地雷与炸药 | todo | | | |
| M2-015 | sim：M2 载具（轻型车、卡车、Panzer IV）与刷新 | todo | | | |
| M2-016 | sim：载具模块损伤与维修 | todo | | | |
| M2-017 | sim：计分与经验 | todo | | | |
| M2-018 | sim：自动平衡、TK、投票与挂机 | todo | | | |
| M2-019 | sim：标记、Ping 与聊天频道规则 | todo | | | |
| M2-020 | server：带宽调节器 | todo | | | |
| M2-021 | server：100 玩家 CPU 优化 | todo | | | |
| M2-022 | gateway：战绩、设置与可选 OAuth | todo | | | |
| M2-023 | gateway：管理 API 与举报 | todo | | | |
| M2-024 | 语音：LiveKit 自托管与 token | todo | | | |
| M2-025 | 语音：服务器订阅集管理 | todo | | | |
| M2-026 | 语音：客户端接入与空间化 | todo | | | |
| M2-027 | bots：Frontline 目标与战术行为 | todo | | | |
| M2-028 | bots：规则式指挥官 | todo | | | |
| M2-029 | bots：100 bot 规模优化 | todo | | | |
| M2-030 | render：GTAO/SSAO | todo | | | |
| M2-031 | render：体积雾 | todo | | | |
| M2-032 | render：TAAU 时间性超分 | todo | | | |
| M2-033 | render：烟雾、火焰、弹坑与血迹 | todo | | | |
| M2-034 | world：遮挡剔除 | todo | | | |
| M2-035 | world：水面与沼泽 | todo | | | |
| M2-036 | world：全图流式与内存预算 | todo | | | |
| M2-037 | UI：径向菜单与快捷指令 | todo | | | |
| M2-038 | UI：记分板 | todo | | | |
| M2-039 | UI：建造界面 | todo | | | |
| M2-040 | UI：文字聊天与语音状态 | todo | | | |
| M2-041 | UI：完整设置与可访问性基础 | todo | | | |
| M2-042 | UI：结算界面 | todo | | | |
| M2-043 | UI：投票、举报、换队与系统提示 | todo | | | |
| M2-044 | UI：战术地图的 Frontline 叠层 | todo | | | |
| M2-045 | 音频：遮挡低通与压制效果 | todo | | | |
| M2-046 | 音频：混响总线、环境与载具声 | todo | | | |
| M2-047 | 素材接入：M2 批 | todo | | | |
| M2-048 | 运维：监控与日志 | todo | | | |
| M2-049 | 测试：100 客户端压测（WT 与 WS） | todo | | | |
| M2-050 | 测试：视觉回归（Playwright） | todo | | | |
| M2-051 | 测试：balance-sim v0 | todo | | | |
| M2-052 | 测试：M2 性能基准 | todo | | | |
| M2-053 | 史实参数复核：M2 武器与载具（R8） | todo | | | |
| M2-054 | 对标测量：M2 批回填 | todo | | | |
| M2-055 | M2 里程碑验收与 STATUS 汇总 | todo | | | |

## M3 全兵种、全载具、Assault、指挥官能力、火炮（M3-001 – M3-054，含续作 M3-052b/c/d）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M3-001 | 数据：M3 全兵种补全与剩余武器 | todo | | | |
| M3-002 | 数据：M3 其余载具（10 辆） | todo | | | |
| M3-003 | 数据：指挥官能力与火炮 | todo | | | |
| M3-004 | 地图：Carentan 炮兵阵地、载具路线与 Assault 测试配置 | todo | | | |
| M3-005 | protocol：M3 消息（能力、炮兵、标记、载具座位） | todo | | | |
| M3-006 | sim：指挥官席位、资源池与指令校验 | todo | | | |
| M3-007 | sim：Supply Drop 与 Ammo Drop | todo | | | |
| M3-008 | sim：Bombing/Strafing/Precision | todo | | | |
| M3-009 | sim：Recon Flight、Rally 与 Airhead | todo | | | |
| M3-010 | sim：Spotter 标记、OP 与暴露规则 | todo | | | |
| M3-011 | sim：狙击步枪与屏息 | todo | | | |
| M3-012 | sim：Commander/Tank Commander/Spotter/Sniper 特有规则 | todo | | | |
| M3-013 | sim：全弹种穿深与装甲判定 | todo | | | |
| M3-014 | sim：轻型/侦察载具 | todo | | | |
| M3-015 | sim：中/重型坦克 | todo | | | |
| M3-016 | sim：运输载具座位与乘员射击 | todo | | | |
| M3-017 | sim：载具呼叫与认领 | todo | | | |
| M3-018 | sim：榴弹炮阵地 | todo | | | |
| M3-019 | sim：炮组操作与反坦克炮 | todo | | | |
| M3-020 | sim：Assault 模式规则 | todo | | | |
| M3-021 | sim：建筑破坏阶段与动态导航障碍 | todo | | | |
| M3-022 | server：载具延迟补偿与 AOI 优先级 | todo | | | |
| M3-023 | bots：载具驾驶与乘员 bot | todo | | | |
| M3-024 | bots：Spotter 与 Sniper bot | todo | | | |
| M3-025 | bots：炮组与车长 bot | todo | | | |
| M3-026 | bots：规则指挥官使用全部能力 | todo | | | |
| M3-027 | bots：LLM 指挥官（可选 feature） | todo | | | |
| M3-028 | bots：Assault 模式 bot 行为 | todo | | | |
| M3-029 | render：全部载具外观与交互 | todo | | | |
| M3-030 | render：空袭飞机与落弹表现 | todo | | | |
| M3-031 | render：炮击落点与战场环境表现 | todo | | | |
| M3-032 | render：载具击毁与残骸 | todo | | | |
| M3-033 | UI：指挥官界面 | todo | | | |
| M3-034 | UI：Spotter 与 OP 工具 | todo | | | |
| M3-035 | UI：载具与火炮操作界面 | todo | | | |
| M3-036 | UI：载具 HUD 细节与 Assault HUD | todo | | | |
| M3-037 | 音频：全部载具与重武器声 | todo | | | |
| M3-038 | 音频：爆炸、炮击与空袭 | todo | | | |
| M3-039 | 音频：战场人声与无线电 | todo | | | |
| M3-040 | 语音：指挥与车组频道 | todo | | | |
| M3-041 | 素材接入：M3 载具 | todo | | | |
| M3-042 | 素材接入：M3 武器与角色 | todo | | | |
| M3-043 | 素材接入：M3 音频 | todo | | | |
| M3-044 | 素材接入：飞机与火炮模型 | todo | | | |
| M3-045 | 测试：M3 全功能 100 客户端压测 | todo | | | |
| M3-046 | 测试：M3 平衡模拟 | todo | | | |
| M3-047 | 数据：M3 首轮数值调参 | todo | | | |
| M3-048 | 测试：M3 视觉回归扩充 | todo | | | |
| M3-049 | 测试：M3 性能基准 | todo | | | |
| M3-050 | 运维：夜间作业 | todo | | | |
| M3-051 | 史实参数复核：全部载具与剩余武器（R8） | todo | | | |
| M3-052 | 对标测量：M3 批回填（1/4：载具机动与炮塔 + Assault 时间窗） | todo | | | |
| M3-052b | 对标测量：M3 批回填（2/4：载具火力、乘员与损伤） | todo | | | |
| M3-052c | 对标测量：M3 批回填（3/4：指挥官能力与音频时序） | todo | | | |
| M3-052d | 对标测量：M3 批回填（4/4：火炮、狙击/特殊武器、压制复核） | todo | | | |
| M3-053 | UI：简体中文文案与 CJK 字体完整性 | todo | | | |
| M3-054 | M3 里程碑验收与 STATUS 汇总 | todo | | | |

## M4 Omaha Beach、Foy、Skirmish、画质与优化、公开测试（M4-001 – M4-046）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M4-001 | 地图：Omaha 数据源、许可证与锚点对账 | todo | | | |
| M4-002 | 地图：Omaha 高度图、海岸与矢量 | todo | | | |
| M4-003 | 地图：滩头障碍与防御工事摆放 | todo | | | |
| M4-004 | 地图：Omaha Assault 关卡标注 | todo | | | |
| M4-005 | 地图：Omaha 区块、碰撞与导航烘焙 | todo | | | |
| M4-006 | 地图：Foy 数据源许可证与锚点对账 | todo | | | |
| M4-007 | 地图：Foy 高度图与矢量 | todo | | | |
| M4-008 | 地图：Foy 森林与散兵坑布置 | todo | | | |
| M4-009 | 地图：Foy 关卡标注（Frontline 与 Assault） | todo | | | |
| M4-010 | 地图：Foy 区块、碰撞与导航烘焙 | todo | | | |
| M4-011 | sim：登陆艇出生与滩头规则 | todo | | | |
| M4-012 | sim：Skirmish 模式规则 | todo | | | |
| M4-013 | 地图：Skirmish 子区配置 | todo | | | |
| M4-014 | world：海洋、海岸与滩头 | todo | | | |
| M4-015 | world：冬季雪地与天气 | todo | | | |
| M4-016 | bots：新地图与新模式 bot 适配 | todo | | | |
| M4-017 | render：四档画质预设 | todo | | | |
| M4-018 | render：动态分辨率与自动推荐 | todo | | | |
| M4-019 | render：Ultra 屏幕空间反射 | todo | | | |
| M4-020 | render：Low 档与 WebGL2 路径优化 | todo | | | |
| M4-021 | render：着色器变体与预热门禁 | todo | | | |
| M4-022 | render：显存与纹理 mip 预算 | todo | | | |
| M4-023 | UI：模式/地图选择与加载界面更新 | todo | | | |
| M4-024 | UI：画质设置与基准向导 | todo | | | |
| M4-025 | UI：可访问性审计与修复 | todo | | | |
| M4-026 | UI：缩放、安全区与超宽屏测试 | todo | | | |
| M4-027 | UI：新手引导 | todo | | | |
| M4-028 | 音频：分地图混响与环境调校 | todo | | | |
| M4-029 | 素材接入：M4 Omaha/Foy | todo | | | |
| M4-030 | server：统计异常检测与回放 | todo | | | |
| M4-031 | gateway：对局编排与匹配 | todo | | | |
| M4-032 | 安全：加固与审计 | todo | | | |
| M4-033 | 运维：生产部署 | todo | | | |
| M4-034 | 运维：告警、运行手册与发布回滚 | todo | | | |
| M4-035 | 运维：隐私与合规（数据导出/删除） | todo | | | |
| M4-036 | 运维：匿名遥测（首次同意） | todo | | | |
| M4-037 | 运维：公开测试准备 | todo | | | |
| M4-038 | 测试：三图 60 分钟浸泡压测 | todo | | | |
| M4-039 | 测试：三图三模式平衡模拟 | todo | | | |
| M4-040 | 测试：三图视觉回归与四档画质 | todo | | | |
| M4-041 | 测试：四档画质性能矩阵 | todo | | | |
| M4-042 | 测试：浏览器兼容矩阵 | todo | | | |
| M4-043 | 史实参数最终清扫（R8） | todo | | | |
| M4-044 | 对标测量：复测与版本漂移处理 | todo | | | |
| M4-045 | 测试：封闭测试执行与反馈汇总 | todo | | | |
| M4-046 | M4 里程碑验收与 STATUS 汇总 | todo | | | |
