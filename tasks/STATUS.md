# 任务状态表（tasks/STATUS.md）

> 执行者每完成/推进/阻塞一个任务，必须更新本表对应行与顶部摘要，并随该任务的提交一起提交。
> 状态枚举：`todo` / `in-progress` / `done` / `partial` / `blocked`（定义见 `tasks/README.md` §4）。
> 任务清单与 `tasks/INDEX.md` 一致；新增/拆分任务须**先改 INDEX 再改本表**。

## 摘要

- **当前里程碑**：M0 技术验证（进度总览与任务日志见 `docs/PROGRESS.md`）
- **M0 统计（2026-11-17）**：33 个任务——done 16、partial 6、blocked 0、in-progress 2（M0-018、M0-024，行内状态待执行者更新）、todo 9（M0-019、022、025、026、027、028、030、032、033）；M1–M4 共 230 个全部 todo。
- **下一个任务**：`M0-018`（server：会话、输入缓冲与校验；执行中）→ 之后 `M0-019`（依赖 M0-018）、`M0-022`（依赖 M0-018/021）、`M0-024`（执行中）、`M0-025`、`M0-026`。
- **blocked 清单**：（无）
  - 格式：`任务 ID — 所缺条目 — 登记日期 — 谁来解除（人类/素材/其它任务）`
  - 人工侧待办（不阻塞任务，但卡退出标准）：GitHub 无 main 分支/无法开 PR（H-102）、参考机（H-021）、对标录像（H-011/H-025/H-026，M0-032 将受阻）、受信任 TLS 证书（H-017）。
- **partial 清单**：
  - M0-002 — workflow/脚本本地通过 — 未在 GitHub 实跑（真实 PR 红绿对、Actions 墙钟）、cargo deny advisories 未验证 — H-102
  - M0-004 — schema/校验器/DATA_HASH 全部通过 — CI 墙钟与"坏 JSON 的 PR 被拒"需 GitHub 实跑 — H-102；PI-002/003/004
  - M0-011 — KCC 全部功能与确定性验收通过 — KCC 单次 18.6–19.9 µs 与 100 人 step 需参考机复测（本机非参考机、余量≈1–7%） — H-021；PI-015
  - M0-015 — 验收 2–6 通过 — tick 间隔抖动 P99 <1 ms 在共享虚机上 2.83 ms（复测 0.79 ms），需参考机/独占机复测 — H-021
  - M0-020 — WebGL2 路径与 SwiftShader WebGPU 通过 — WebGPU 真机帧时间/像素/compute 真 GPU 未测 — H-021；PI-005、PI-006
  - M0-023 — 地形算法、接缝、一致性、纹理体积通过 — GPU 帧时间（RTX 3060/Iris Xe）未测、WebGPU 无像素校验 — H-021；PI-014
- **最近更新**：2026-11-17 / 总控整理（填完成提交哈希、更新摘要、新增 `docs/PROGRESS.md`）；M0-021（done，哈希待执行者填）

## 列说明

- **实测结果**：验收标准要求的**实测数字**（如"tick_ms P99 = 21.4 ms，100 客户端×30 min"），附测量命令或报告路径；不得只写"通过"。
- **遗留问题**：未解决的偏差/风险，写 `docs/PLAN_ISSUES.md` 条目编号；无则留空。
- **完成提交**：完成该任务的 commit 短哈希（多个提交用逗号分隔）。

## M0 技术验证（M0-001 – M0-033）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M0-001 | 仓库脚手架与工具链 | done | 1) `RUSTFLAGS="-D warnings" cargo build --workspace` 退出 0、警告 0；2) `cargo clippy --workspace --all-targets -- -D warnings` 退出 0；`cargo fmt --check`、`cargo test --workspace` 通过（8 个 smoke 测试）；3) `pnpm -C client build` 退出 0 无 warning（dist 0.75 kB js），`tsc --noEmit` 0，`pnpm lint` 0；4) `cargo metadata` 内部依赖边 14 条，与 ARCHITECTURE §4.5 允许集合逐条比对完全相等（sim→protocol；sim-wasm→sim,protocol；bots→sim,protocol；server→sim,protocol,bots；gateway→protocol；headless-client→sim,protocol；balance-sim→sim,protocol,bots；protocol 无）；5) `node tools/ci/check-layout.mjs` 退出 0（24 目录+4 文件）；6) `git check-ignore` 三路径均被忽略；7) 两 lockfile 已生成待入库，`@babylonjs/core` 精确 8.56.2。版本：Rust 1.97.0、Node 22.23.4、pnpm 10.28.0、vite 8.3.8、TS 6.0.3、vitest 5.1.0、eslint 10.15.0、preact 11.2.0、@preact/signals 2.11.4。 | BRIEF 文档不存在，目录清单按 ARCHITECTURE §4.1；check-layout 不要求 gitignore 的 assets/incoming、processed（新增 assets/.gitkeep）；TS 取 6.0.3（typescript-eslint 8.73 要求 <6.1，故未用 7.x）；Babylon 取 8.x 最新（最新大版本已是 9.x） | ca86eea |
| M0-002 | GitHub Actions 基线与依赖图检查 | partial | | 已建 lint/test-rust/test-ts/wasm-build/data-validate(占位) 5 个 workflow、deny.toml、tools/ci/{dep-matrix,check-deps,check-sim-purity}.mjs + fixtures + 11 个 node:test（全过）。本地逐 step 验证：fmt/clippy/eslint/tsc/vitest/check-*/cargo deny(bans+licenses+sources) 见下；check-deps 对当前仓库 exit 0；真实 cargo metadata 沙盒副本中加 sim→tokio、gateway→sim 均 exit 1 且日志含 `sim -> tokio`（protocol→sim 被 cargo 自身环检测拒绝，改用 fixture 验证）。purity 扫描违规 exit 1，purity-allow 通过并列出。 **未在 GitHub 上实跑**：验收 1 的真实 PR 红绿对、验收 5 的 Actions 墙钟（空/热缓存）未取得；`cargo deny` 的 advisories 检查本机无法访问 github.com/RustSec，未验证（CI 上首次运行需确认）。 本机并行他人代码导致：`cargo fmt --check`（protocol 在途）、eslint/vitest（client/src/render 在途）暂失败，非本任务范围。 遗留：缓存选 Swatinem/rust-cache 而非 sccache（H-014 前无远端存储）；deny 许可证清单待 H-037；wasm-build 在 sim-wasm 无 wasm-bindgen 前跳过（M0-014 去掉）；cargo-deny/第三方 action 版本未在 GitHub 上验证可用。 | 7360d3b |
| M0-003 | 字体许可证核实与登记（R10） | done | 4 字体各 1 行：Barlow、Noto Sans SC 为 OK（RFN 分别为无、Source，族名不冲突）；IBM Plex Sans/Mono 为 PENDING（RFN "Plex"，子集化须改名，见 PI-009）。licenses/texts 4 文件 4377-4456 字节；Noto Sans SC 可变 TTF 实测 17,772,300 字节；font-subsetting.md 已写 | PI-009 待裁决；LICENSES §5 字体行仍写"未核实"（未改） | 3deb07d |
| M0-004 | /data schema 与校验工具 | partial | 1) invalid fixtures 23 个（覆盖 weapon/role/loadout/vehicle/map_meta/spawns；缺字段、错单位 kph、无单位、范围越界、引用不存在、falloff 非递增、tunable 路径不存在等），`pnpm -C tools/data-validate test` 42/42 通过，其中对每个 invalid 样例单独起 CLI 子进程：退出码 1 且输出含 `_expect_error`；8 个 valid 样例全过（`node tools/data-validate/index.mjs --fixtures`）；2) 真实 `/data` 退出码 0（`--root data`，M0 仅含 schemas+fixtures+README）；3) `cargo test -p frontline-sim data_`（注意包名是 frontline-sim）7/7 通过，含 `data_hash_stable`（同内容两次/打乱文件序/改缩进与键序相等；改一个值或改文件名不等）与 `data_hash_matches_tree`；4) 在 scratchpad 副本中放入缺 `historical._source` 的 weapon 文件，`cargo build -p frontline-sim` 退出 101 `data validation failed: weapons/us_fixture_rifle.json: missing field `_source``（未入库）；5) CI 墙钟：本机等价步骤（validator 测试 4.9 s + fixtures + validate <0.5 s）约 6 s，远小于 2 min；仅 `data/**`、`tools/data-validate/**` 触发（.github/workflows/data-validate.yml 已接入）。构建增量：build.rs 重跑 ≈0.26 s（<3 s）。`cargo fmt --check` / `clippy --workspace --all-targets -D warnings` / `cargo test --workspace` 通过；wasm32 目标可编译。**当前 DATA_HASH = 0x141cfc9842c4b0e3**（/data 无数据文件时的空集哈希；M0-008 起会变，属预期）。 | 未在真实 GitHub Actions 上跑：CI 作业墙钟与“故意提交坏 JSON 的 PR 被拒绝一次”需总控推送后在 GitHub 上记录（故标 partial）；PI-002（字段命名文档冲突）、PI-003（schema 细化）、PI-004（serde/serde_json/sha2 依赖、包名 frontline-sim）；rules/*.json 与 maps/*/map.json 暂无 schema，仅进哈希；.github/workflows/data-validate.yml 由 M0-002 创建占位，此处已改写；根 package.json 追加 data:validate / data:test；pnpm-lock 新增 tools/data-validate（ajv 8.20.0） | 2bdf74a |
| M0-005 | protocol：位流、量化与 M0 消息集 | done | 包名 `frontline-protocol`（`-p frontline-protocol`）。1) `--release -- --ignored roundtrip_1m` 10^6 样本（InputCmd/InputBundle/Hello/Welcome/Ping/Pong 混合）全部往返相等，1.5 s；`PROPTEST_CASES=10000` 快速版通过；2) 10^6 随机样本最大量化误差：x/z/y 均 0.015625 m（=1/64）；yaw 10 bit 0.176°、8 bit 0.703°；pitch 8 bit 0.3530°（略大于 0.35°，见遗留）、9 bit 0.176°、16 bit 0.0014°、i16 0.0014°；3) 典型 3 条走动 InputBundle = 29 B（≤30），最坏全字段突变 49 B；4) fuzz_smoke：2×10^5 随机串 + 全部前缀截断 + 全部单比特翻转，0 panic；ticket=257 的 Hello 被拒（TooLong）；5) `grep -R unsafe crates/protocol/src` 为空（由 workspace lint forbid 保证，已验证加 unsafe 即编译失败）；`cargo build --target wasm32-unknown-unknown` 通过；`cargo tree` 仅 arrayvec 0.7.8。性能（--release，仅记录）：InputBundle 编码 141 ns、解码 137 ns。`cargo fmt --check`、`cargo clippy -p frontline-protocol --all-targets -D warnings`、`cargo test -p frontline-protocol` 通过（13 单元 + 6 limits + 6 roundtrip；golden bytes 8 个）。未做 criterion 基准（可选项，用 ignored 测试 bench_codec 代替）。 | PI-001：Welcome 加字段、InputBundle 差分位宽、origin_hint 含义待 M1、pitch 8 bit 误差 0.353° vs 0.35° | 89aad49 |
| M0-006 | protocol：快照编码与基线增量 | done | 先落实 PI-001 裁定：新增 `PITCH_BITS_EVENT=9`（Shot.dir_pitch；M0 唯一的 8 bit pitch），limits 测试更新；InputBundle 无需改（pitch 为 i16），重测典型 3 条 = 29 B、最坏 49 B，golden 不变；PI-001 已标 resolved 并追加裁定。`crates/protocol/src/snapshot/*` + tests/{snapshot_prop,snapshot_size}.rs + fuzz/。 1) 属性测试（--release，各 10^5 次，随机 1..60 实体、关键帧/增量/移除/新增混合）全部通过：关键帧路径 `decode(encode(b,None))==b`；增量路径 `decode(encode(b,Some(a)),Some(a))==b`（近景精确，远景 yaw 8 bit 量化且 pitch 取基线，测试按此预期；约 258 万 delta + 50 万 keyframe 记录）；debug 下默认 10^4。 2) cargo-fuzz（nightly 1.102.0-nightly，本机新装 cargo-fuzz 0.13.2）：`snapshot_decode` 601 s、637 万次执行，`input_bundle_decode` 331 s、3770 万次，均 0 crash/panic/OOM（artifacts 为空）；日志 reports/m0/M0-006-fuzz-{snapshot_decode,input_bundle_decode}.log。 3) 字节数（60 s 内存回放，4 m/s 直线，20 Hz，ack 间隔≤2 tick）：近景 delta 纯走动 4.12 B/记录、边走边转 6.12 B（目标 ≤10）；远景 4.53 B（目标 9）；ack 晚 3 个快照（≈100 ms RTT）仍 6.12 B；关键帧 12.50 B（≤13）；self_state 7 B（含 pos_corr 13 B，≤14）；事件 Shot 98 bit ≈ 12.3 B/条。 4) 40 实体（近 4+中 10 用近景格式 + 远 26，delta，含 self_state 与 5 条 Shot）平均 282 B、最大 287 B（≤600）；40 实体全关键帧 512 B；预算 60/100/333/700/1150 B 下 `add_entity` 返回 `Ok(false)` 且已写内容可解码（含 127 记录上限）。对照 ARCHITECTURE §6.9：20 Hz × 282 B ≈ 45 kbps 载荷（未含 58 B/包传输开销），均值预算 256 kbps 内。 5) 基线环：ack 落后 32 tick 仍为 delta，33 tick 全部关键帧（断言记录类型）；30% 下行丢包 + 30% ack 丢包 60 s：849 解码/351 丢，0 次 MissingBaseline，最终与服务器一致。 性能（--release，`--ignored bench_snapshot`）：snapshot_encode_client（60 实体，30 近 + self）≈ 5.6 µs/客户端（含建输入 6.2 µs；预算 25 µs），snapshot_decode 6.3 µs。检查：`cargo fmt --check`、`cargo clippy -p frontline-protocol --all-targets -D warnings`、`cargo test -p frontline-protocol`（20 单元 + 6 limits + 6 roundtrip + 7 snapshot_prop + 4 snapshot_size，含 3 个新 golden/2 个快照 golden）通过；wasm32 构建通过；无新增依赖（fuzz 工程独立 workspace，依赖 libfuzzer-sys）。 | PI-010：快照位宽/基线语义细化（单一基线 tick、记录类型、位置大小类阈值改为 32/256/2048 步、远景不发 pitch）；M0-019 应让近/中景实体每包都出现，否则多发关键帧（12.5 B）；CI 只需跑 fuzz 60 s 冒烟（`cargo +nightly fuzz run <target> -- -max_total_time=60`）；criterion 未用，以 ignored 测试代替。 | 177a6ed |
| M0-007 | sim：World/step/确定性底座 | done | 1) `cargo test -p frontline-sim --features test-util` 的 `determinism_basic`：2 次 10 000 tick（各在独立线程、其间扰动堆分配）逐 tick 相等 10000/10000，最终 hash golden=0x700dc1b825c9ae38；2) 100 次扰动（50 次改 match_seed、50 次翻转某玩家某 tick 一个按钮位）在 ≤2 tick 内 hash 出现差异 100/100；3) check-sim-purity OK（0 waiver）、check-deps OK、`cargo tree -p frontline-sim --target all -e normal` 无 tokio/rayon/wasm-bindgen/getrandom；4) wasm32 构建（含 `--features client-predict`）通过；5) 乱序输入：debug 构建 panic（should_panic 单测）、release 构建结果与已排序一致（`--release` 单测）均通过；6) 槽位复用代数+1、旧 id 查询 None 单测通过。基准 `cargo bench -p frontline-sim`（release，本机软件环境，仅记录）：world_step_empty 10 玩家 ≈0.31 µs/step、100 玩家 ≈2.78 µs/step。fmt、clippy `-D warnings`（`--no-deps --all-targets --features test-util`）通过；lib 单测 17 + determinism 5 全过。 | bevy_ecs 只能锁 =0.16.1 且不开 `std`，否则 wasm32 传递依赖 wasm-bindgen/tokio 使 check-deps 失败（PI-011）；determinism 测试需 `--features test-util`（CI 须带该参数）；`WorldView::entity_state` 暂返回 sim 自有 `EntityStateView`（protocol 尚无导出的 EntityState）；los/height_at/team_resources 为占位（M0-010/013）；`cargo clippy -p frontline-sim` 不带 `--no-deps` 时会 lint 并行中的 protocol（M0-006）而失败，非本任务。 | b6e07c4 |
| M0-008 | 数据：M0 步枪（Kar98k、M1 Garand） | done | 1) `node tools/data-validate/index.mjs --root data` 退出 0（checked 2 files, 0 error）；`--fixtures` 8 valid/23 invalid 通过；`pnpm -C tools/data-validate test` 42/42（连跑 3 次全过；改 schema 后第一次曾见 1 项失败，未复现、原因未查明）；2) 两文件 `_meta.status=initial`，`grep -c _source` 各 1；3) 逐项对照 GAMEDESIGN §6.2–§6.6：差异 0 项（由 `crates/sim/tests/data_weapons_m0.rs::data_weapons_m0_load` 断言全部数值）；4) 躯干 STK（HP 100）：Garand 0/100/200/300/600/800 m 均 2 发（每发 70→50.4 HP）；Kar98k 0/100/200 m 为 1 发，201 m 起 2 发（201 m 99.97 HP；300 m 96.7；800 m 80.0），与 §6.6 一致，已作为快照测试 `data_weapons_m0_stk_snapshot`；5) DATA_HASH 本任务后 = **0x8e4ea0ee2af4bf8f**（此前空集 0x141cfc98…），清理后重建两次相同。测试 2/2 通过；fmt/clippy -D warnings 在隔离副本通过。 | 当前工作区 `cargo test -p frontline-sim` 无法编译（M0-007 进行中：rng.rs/schedule.rs 报错，非本任务代码），故 Rust 侧在 scratchpad 隔离副本（同一份 data/、data*.rs、build.rs、测试）验证；M0-007 稳定后总控应重跑一次 `cargo test -p frontline-sim data_weapons_m0`。schema 追加可选字段 `part_mult.pelvis`、顶层 `fx{muzzle_flash_ms,tracer}` 并同步 data_defs.rs（PI-007）；pelvis 缺省回退规则交 M0-012；步枪 `caliber`/`cartridge` 拆分写法与 `magazine_type` 取值（en_bloc_clip / stripper_clip_internal_box）为本任务自定。同时完成 PI-002/003/004 裁决回填与 ARCH §8.5 变更说明。 | c59dd79 |
| M0-009 | 地图：M0 单块地形（256 m）数据管线 | done | 1) `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1` 全流水线 0.3–0.7 s；heightmap_1m 132098 B、far_8m 2101250 B、16 个 b_*.bin 各 8450 B、16 个 c_*.bin 各 12562 B（与任务公式一致），共 2.6 MB；hmin/hmax=1/11 m（实际 1.71–10.20 m）。2) `validate data/maps/m0` 退出 0；`node tools/data-validate/index.mjs` 0 错误；`pnpm data:test` 42 通过；`cargo test -p frontline-sim data_` 通过。3) pytest 21 通过 1 skip（合成 GeoTIFF 用例，本机无 GDAL）；test_determinism 两次运行 37 个文件 SHA-256 全同。4) 区块边界 65 样本逐一相等；远景中心与近景在 8 m 抽样点差 0 量化步。5) 最大坡度 38.7°，>35° 的土坎沿 z 向连续 9 个样本、沿 x 向多段 ≥8；<5° 平地占 88.5%，含 64×64 m 的两块平台（出生点所在）。SHA-256：meta.json 32bc28fc917371778a24d1f59af37f251c64a38da3b9bb7bbe08b8303a542f90、spawns.json 75ea6d44925402d05a61ffbf93ebb91a668d4fa26bd438d748dd040f60cca4b7、heightmap_1m.r16 76fe2d58af3bc864c8cf7d2889d29ec8c5685cf968aa5b7e3554be6bddd44756、heightmap_far_8m.r16 1c958f6f179088893cd10b260cc4de67c73425884977a54e717b936bf85b1d30（完整值见 data/maps/m0/manifest.json 与本地 sha256sum）。产物：meta/spawns/manifest.json 入库；*.r16 与 chunks/ 已 .gitignore，重生成命令见 DEVELOPING.md。DEM 路径（GDAL）本机未装，仅清晰报错路径与窗口计算有单测，未实跑。 | PI-007 | ef41e84 |
| M0-010 | sim：高度场碰撞 | done | 机型：Xeon 2.3 GHz 云主机（4 核，release）。1) `cargo bench -p frontline-sim --bench collision`：collision_raycast 10 000 条种子射线（≤800 m，M0 地形）中位数 1.47 ms（<5 ms；最坏情形 6 m 高水平射线 3.14 ms；ground_probe 约 52 ns/次）；2) `slope_limit_35deg`（Rapier KCC，胶囊 r0.35/h1.8，4.4 m/s×5 s，max_slope_climb=35°）：36/40/45/55° 坡增高 0.02 m（≤0.5）；10/20/25/30° 坡增高 ≥100% 坡高；M0 真实路堤 38.7° 增高 0.02 m，其缓侧（≈19°）可爬 2.5 m；3) 反量化：1000 点 height_at 与 r16 双线性参考最大差 1.1e-6 m（≤1e-4）；4) 接缝：1000 个交点邻域样本 1 cm 差均 ≤ 全局最大样本步长×1 cm+1e-4，缺块 None；5) `collision_parse_fuzz` 10^5 例（31684 Ok/68316 Err）无 panic，合法解析 ≤21 KB（先验长度再分配，无法在 forbid(unsafe) 下装计数分配器，按设计保证）；6) 同数据构建两次/克隆/乱序插入：Rapier 碰撞体句柄顺序与 `CollisionData::state_hash` 全等。另：自写射线与 Rapier cast_ray 3000 条中 2999 条一致（1 条 Rapier 漏检，暴力步进裁定自写正确）；M0 全 16 块驻留 0.336 MB（≤5 MB）；wasm32 构建、check-deps、check-sim-purity、fmt、clippy -D warnings 通过；sim 测试（带 test-util）全过。CI：test-rust.yml 增加 `cargo test -p frontline-sim --features test-util --locked` 一步。 | PI-013（Rapier =0.36.1 + enhanced-determinism；查询自写；`from_bytes(bx,bz,..)`、`height_at->Option`）。地形二进制缺失时测试/基准打印重建命令后跳过（`FRONTLINE_REQUIRE_TERRAIN=1` 则失败）。native↔wasm 逐位一致性待 M0-014/029 比对；未用 criterion；cargo-deny 本机未安装未实跑。 | 7705e34 |
| M0-011 | sim：Rapier 角色控制器与基础移动 | partial | 机型：本机 Xeon 2.3 GHz 云主机 4 核、**非参考机**、测量时系统负载均值≈6.7（release，enhanced-determinism，Instant 自制 harness，非 criterion）。1) `cargo bench -p frontline-sim --bench kcc`：kcc_step 中位数 18.6 µs（均值 22.8，p99 96.3；预算 ≤20，余量仅≈7%）；Rapier 自带 snap_to_ground 使其 ≈41 µs，故改为解析 snap（PI-015）；2) kcc_100_players_tick（100 人随机走/跳/蹲/冲刺，M0 地形，整个 World::step）中位数 2.14 ms（≤3），p99 2.63 ms；基线 reports/baselines/sim-m0.json；3) `speed_matches_data` 平地 3 s：walk 2.000/run 4.400/sprint 6.400/蹲跑 2.400/蹲走 1.500 m/s，相对误差均 <0.01%（另测对角不超速、后退冲刺按跑速）；4) 跳跃峰值 0.5987 m（数据 0.6，误差 0.22%），空中 20 tick，落地 grounded=true；5) 坡度：>35°（36/38.7/45/55/70°）跑 5 s 高度增益 ≤0.040 m（M0 38.7° 堤坝 0.033 m）；≤30° 可上，水平速度占比 10°97%/20°88%/25°82%/30°75%（34° 69%）；M0 缓坡爬升 2.5 m、均速 4.23 m/s；6) 确定性：`movement_replay_10k` 10 000 步（含随机跳/蹲/冲刺）与另一线程重建碰撞世界的结果逐步位相同，golden 轨迹哈希 0x04275f5ff85febd8；`movement_world_vs_predict` World 与 PredictSim 3 000 tick 位相同（合成与 M0 地形）。加速/减速：即时（1 tick）；站↔蹲 250 ms=7.5 tick，第 8 tick 完成；空中控制 0.3/tick。`cargo test -p frontline-sim --features test-util`（FRONTLINE_REQUIRE_TERRAIN=1）全过；clippy -D warnings、fmt、wasm32（含 client-predict）、check-deps、check-sim-purity、data-validate（47/47）通过。`determinism_basic` golden 因 state_hash 折入移动状态而重钉为 0x63ac47f15c2331d8。 | 仅差参考机复测 KCC 两项耗时（本机非参考机且余量小，记 partial，见 PI-015）；空碰撞下玩家不动（server 用 `CollisionData::empty()` 时）；`aux` bit0=走路键、其余位待 M0-012；平滑加速待 REF M-24–M-26；PlayerAuthState 字节布局 39 B 供 M0-014/022 | a54126d |
| M0-012 | sim：步枪射击、弹道与装填 | done | 机型：本机 Xeon 2.3 GHz 云主机 4 核、**非参考机**、测量时系统负载均值≈4（release）。**先落实 PI-015 补充裁定**：role `movement.accel_time_s` 0.25 / `decel_time_s` 0.20（缺省=瞬时），实测静止→跑 4.4 m/s 第 8 tick 到达（7.5 tick）、冲刺同样 8 tick、跑→停 6 tick（=0.20 s）；`vel` 地面存指令速度（斜坡速度占比不变：25° 82.1%、30° 75.0%）；速度测试改稳态后测量并新增加减速测试；golden 重钉（movement_replay：加减速改变轨迹，脚本命令保持 30 tick；determinism_basic：state_hash 折入生命/武器/弹丸）。GAMEDESIGN §9.1 已加一行、PI-015 已 resolved、新增 PI-016。验收：1) `ballistics_vs_reference`（RK4 步长 0.1 ms，f64）500 m：Garand 下坠 2.243 m vs 2.199 m（误差 2.0%）、速度 584.8 vs 586.3 m/s（0.25%）；Kar98k 下坠 2.745 vs 2.699 m（1.7%）、速度 540.0 vs 541.0（0.19%）；100/200/300 m 下坠 Garand 0.0764/0.3110/0.7296 m vs RK4 0.0709/0.2988/0.7090 m（相对 7.7/4.1/2.9%，绝对 ≤1.3 cm；半隐式欧拉系统性多算 ≈½·g·h·t，验收仅 500 m）；对 GAMEDESIGN §6.4 示例：速度 584.8 vs 586.3 m/s（−0.25%）、TOF 0.714 vs 0.72 s（−0.9%）、下坠 2.243 vs 2.54 m（−11.8%，文档用 ½gt² 未计阻力耦合）；2) 射速：最快点按者 10 s Garand 20 发（间隔恒 15 tick=0.5 s）、Kar98k 7 发（间隔恒 45 tick）；按住不放 1 发；乱点（隔 tick 点按）Garand 间隔 16 tick；3) 状态机/散布/后坐/命中体单测共 17 项（`weapon_fsm`）+ 模块内单测，覆盖半自动松开重按、栓动循环、装填中/冲刺/姿态过渡不可射击、空膛点击、装填不可重入/换枪无效、ADS、累加器；4) 装填：Garand 空仓 84 tick=2.800 s、有余弹 102 tick=3.400 s，Kar98k 夹装 90 tick=3.000 s（均与数据差 0 tick）；弹药守恒测试（每 tick 断言 mag+reserve+已射=初值）；5) 散布：Garand 盲射 10 000 发 10 000 发在 3.5° 半角内（最大 3.500°）；ADS 静止 100 m 95% 半径 0.254 m（≤0.35，最大 0.261 m）；走/跑/蹲散布最大角 5.25/8.75/2.80°（=3.5×1.5/2.5/0.8）；6) 伤害：公式 50/300/600 m Garand 躯干 70.000/62.160/50.400 HP、Kar98k 100/96.667/86.667；World 端到端（弹丸实际飞行距离 49.3/299.3/599.3 m）70.000/62.187/50.427 与 Kar98k 100/96.690/86.690 HP，与公式差 <0.01，生命守恒；爆头 ×3.000（Garand 210、Kar98k 300 HP）、腿 ×0.65、盆骨 ×0.9；7) 确定性：4 玩家 5000 tick 随机脚本两次（含另一线程）state_hash 5000/5000 相同；服务器与 PredictSim 5000 tick 的 PlayerAuthState 与 WeaponState 逐 tick 位相同（35 次预测射击、13 次装填，覆盖 fsm 状态 0/2/3/4/5）；golden 对射事件序列哈希 0x2ced8d499901efde（`weapon_duel`）。性能（`cargo bench -p frontline-sim --bench ballistics`）：20 人 + 400 弹丸×4 子步（贴地 3 m 起飞）弹丸阶段中位数 0.30 ms（p99 0.41；预算 2.5 ms），20 人连发 World::step 0.12 ms；`--bench kcc`：KCC 单次中位数 19.3–19.9 µs（三次，预算 ≤20，余量更小；加减速只加几次 sqrt，差异在噪声内，系统负载≈4）、100 人 World::step 中位数 2.20–2.24 ms（预算 3，p99 2.8–3.3 ms）。检查：`FRONTLINE_REQUIRE_TERRAIN=1 cargo test -p frontline-sim --features test-util` 104 项全过（不带 feature 也全过）；clippy --all-targets -D warnings、`cargo fmt --check -p frontline-sim`、wasm32 构建（含 `--features client-predict`）、`cargo build -p frontline-server`、check-deps、check-sim-purity、data-validate（root 7 文件 0 错；fixtures 8 valid/24 invalid；`pnpm -C tools/data-validate test` 48/48）均通过。 | 命中体尺寸/眼高/移动散布阈值为初值【待测 → M-15】；M0 无输入缓冲（不可射击期间的扳机上升沿丢失）；颈部无独立体积、同队/自身不命中；`SimEvent`/`StepOutput` 不再实现 `Eq`；新增 `World::{weapon_state,health,projectiles,spawn_projectile}`、`PredictSim::{set_entity,reset_weapon,weapon,last_weapon_out}`、`WeaponState`（33 B 字节布局供 M0-014/015）；弹丸离开已载入碰撞区域即销毁（空碰撞世界不会有飞行弹丸）；`aux` 开火模式位未分配（M0 无模式切换）；详见 PI-016。 | 5585573 |
| M0-013 | sim：生命、死亡与简易重生 | done | 机型：本机 Xeon 2.3 GHz 云主机 4 核、非参考机（release）。1) `damage_death::stk_matches_data`（8 例）：数据推导与模拟射击的躯干致死发数一致——Garand 50/300/600/800 m = 2/2/2/2，Kar98k = 1/2/2/2（致命区域均为 Torso）；`headshot_kills_with_one_shot`：Garand 头 210 HP、Kar98k 300 HP 一发致死；2) 重生计时（`respawn_timing_matches_the_formula` + `respawn.rs` 单测）：死于 7 s→15.000 s、1 s→15.000 s、16 s→30.000 s、15 s（恰在波次 tick）→30 s，`wave_aligned=false`：7 s→12 s、1 s→6 s，出生 tick 与公式误差 0 tick（≤1）；3) 重生后 HP=100、Kar98k 弹匣 5/备弹 30（等于开局值，死前已打空部分弹匣）、位置在出生点 ≤3 m（偏移 ≤2.5 m）、朝向地图中心（误差 <0.002 rad）；死亡期间对尸体射击 7 发命中数 0；4) 死者输入（前进+跳+开火）10+ tick 位置/弹匣不变、无 Shot；5) 5000 tick 含 9 次击杀/9 次重生的场景两次运行逐 tick state_hash 相同，golden 终值 0xd6bf346cbfbc9b62；20 人随机对射 3000 tick 无 panic：24 击杀 = 24 死亡，19 重生 + 5 在途 = 24。另有：队伍出生点选择/无点回退到加入位置、断线清理（队列清空、不再 Spawn）、killer 已离开时 `killer=None`。golden：determinism_basic / movement_replay 不变（state_hash 仅在死亡状态下追加字节）；`weapon_duel` 事件 golden 重钉 0x2ced8d499901efde→0x09a7e29170609f65（对射现在以击杀+重生收尾，且 Kill/Spawn 进入哈希；该测试的伤害/弹药守恒改为按"出生后"累计）。性能：规则阶段仅在有命中/到期重生时做 O(死亡数) 工作，无死亡时每 tick 仅两次空检查；`--bench kcc`：kcc_step 中位数 19.75 µs（均值 24.0，p99 98；预算 ≤20，PASS，余量≈1%）、100 人 World::step 中位数 2.265 ms（p99 2.93；预算 3）；world_step_empty 100 人 7.2 µs。检查：`FRONTLINE_REQUIRE_TERRAIN=1 cargo test -p frontline-sim --features test-util` 119 项全过（不带 feature 也全过）；clippy --all-targets -D warnings（sim，带/不带 test-util）、`cargo fmt -p frontline-sim --check`、wasm32 构建（含 client-predict）、`cargo build -p frontline-server`、check-deps、check-sim-purity、`pnpm data:test` 49/49、data-validate（含 rules 新种类与 1 个 valid / 1 个 invalid fixture）通过。顺带更正 GAMEDESIGN §6.4 示例 2.54 m 注记（积分 2.24 m / RK4 2.20 m）。 | 新增 `data/rules/respawn.json` + `rules.schema.json` + data-validate 'rules' 种类 + `GameData.rules`（手写 `GameData{..}` 字面量需补 `rules`）；新增 API：`World::{player_life,set_team_spawn,set_map_center,respawn_pending}`、`SimEvent::{Kill,Spawn}`、`WorldView::{health,alive}`（默认实现）、`EntityStateView.{health_q,alive}`、`health::{Health{hp,max_hp},LifeState,quant_health}`、`respawn::{respawn_time,RespawnPolicy,RespawnQueue}`；`components::Health` 由元组结构改为 `{hp,max_hp}`（`World::health()` 仍返回 f32）；死者保留实体但不在 `players()` 中；服务器须在载入地图后调用 `set_team_spawn`（M0-018），否则回到加入位置；KCC 余量仅≈1%（本任务未触碰移动阶段，噪声内）；见 PI-017。 | a4f7dcf |
| M0-014 | sim-wasm：WASM 封装 | done | 机型：本机 Xeon 2.3 GHz 云主机 4 核、非参考机、Node 22.23。工具（版本写死）：wasm-bindgen =0.2.130（Cargo 锁定，CLI 同版）、wasm-pack 0.15.1、wasm-opt binaryen version_132；brotli 用 node:zlib q11。1) 体积（`wasm-pack build crates/sim-wasm --target web --profile wasm-release` + `node tools/ci/check-wasm-size.mjs`，报告 reports/m0/M0-014-wasm-size.json）：cargo 原始 2 243 931 B → wasm-pack 产物 742 711 B → `wasm-opt -O3` 746 965 B；**gzip-9 308 281 B，brotli-11 240 627 B**（门限 1 200 000 / 400 000，通过；对 ARCHITECTURE 首包预算的 WASM 部分占用 br 约 60%）。opt-level=3 无 LTO 的对照：-O3 后 1 065 002 B / br 324 953 B（也在限内）。2) `node crates/sim-wasm/tests/node-smoke.mjs` 8 组全过（protocolVersion/dataHash 与原生 `native.json` 相等；weaponDef/roleDef；16 块地形 + reset + 120 步 z 56→73.30、grounded；C0/C1 字节与 protocol golden 相等；黄金快照 key+delta 全部字段与原生解码相等；布局常量）。3) `Predictor.step` 10 000 次（Node，含 cmd 拷贝与返回视图，步行+转向+开火脚本）中位数 **12.3–13.7 µs**，p95 24–29 µs（≤50 µs）。4) 1000 次 decode 线性内存增长 0 B（≤64 KB）。5) 非法输入 3 类（截断快照、cmd 长度 17/19、错误 auth/块/玩家号等）全部抛 JS 异常且之后仍可用。6) **native vs wasm 逐位一致性：一致**——原生 `PredictSim` 与 wasm `Predictor` 对同一 3000 tick 脚本（走/跑/冲刺/跳/蹲/瞄准/连发/装填，真实 M0 地形，Garand）每 tick 的 PlayerAuthState(39 B)+WeaponState(33 B) 全部逐字节相同（`cargo run --release -p frontline-sim-wasm --example trace` + `node crates/sim-wasm/tests/native-vs-wasm.mjs`；无分歧 tick）。7) CI：wasm-build.yml 去掉探测，加 wasm-opt 安装、体积门禁、地形生成、node 冒烟/对比/基准；**未在 GitHub 上实跑**（墙钟 <8 min 未取得；本机冷编译 wasm-release 约 1 m 10 s）。检查：`cargo test -p frontline-sim-wasm`（8 项）、`cargo clippy -p frontline-sim-wasm --all-targets -- -D warnings`、`cargo fmt --check`、check-deps、`pnpm -C client lint/typecheck/test`、`node --test tools/ci/test/*.test.mjs` 通过。 | PI-018（不加 workspace 依赖以免干净检出 `pnpm install` 失败；decode 用访问器视图；build.rs 嵌入去 `_meta` 的数据 JSON；wasm-release profile）。`SelfState` 目前只给 AmmoInMag 等原值，M0-022 需把 pos_corr 对账接上；`EntityStateView`/Kill/Spawn 等 M0-013 变化不影响 sim-wasm（它不用 view/事件）；`step` 返回视图须在下次调用前消费。native 与 wasm 的更长/更多武器覆盖留 M0-029。 | 151b4b0 |
| M0-015 | server：进程骨架与 30 Hz 主循环 | partial | 验收 2-6 通过，验收 1 抖动项未达标。10 min 空载（release，4 核 Firecracker 虚机，Xeon 2.3GHz，内核 6.18.44，期间 load avg 4-8，其它执行者并行编译/测试）：17995 tick，server_tick_ms P50 0.006 / P99 0.017 / max 5.34 ms，tick_overrun_total=0，consecutive_max=0；tick_interval_error P50 0.0003 / P99 2.83 / max 64 ms（跳过 6 个时间槽）。100 玩家空世界 step P99<1 ms（debug 构建，集成测试 step_overhead）；SIGTERM 退出码 0 且 <100 ms；日志 JSON + 限速、假时钟（50 ms tick 后无追赶/无漂移，连续 3 次超限 max=3）单测通过；/metrics 桶数 13。 | 抖动 P99 < 1 ms 未达标：共享虚机负载所致，需在参考机/独占物理机复测（H-021），故 partial。--pin-sim-core 仅接受参数（M1-067）；--bots 仅告警、server_bots 恒 0（M0-026）；WS/WT 端口未绑定（M0-016/017）；apply_inputs 仅用本 tick 新到输入（M0-018）；无新增 PLAN_ISSUES。 | aee3f6c |
| M0-016 | server：WebSocket 传输与握手 | done | 验收 1-5 全部通过。1) ws_soak：10 客户端 1 Ping/s，60 s（CI，debug）610 次回显、0 意外断开；10 min（release，`--ignored ws_soak_10min`，日志 reports/m0/M0-016-ws-soak-10min.log）6010 次回显、0 意外断开、server_tick 递增、client_time 回显全部正确；进程 CPU（含 10 个进程内测试客户端）0.19% 单核≈≤0.02%/连接。2) 握手异常：版本/data_hash 不符→Reject 后关闭；无 Hello 5.001 s 关闭（计数 net_handshake_timeout_total）；257 B ticket 断开；同 IP 第 5 连接被拒（TCP 层，未升级）。3) ws_drop_oldest：100 个 C2 + 100 个 C0 交错压入暂停的写端，客户端收到 C2=[96,97,98,99]（单调，最后一包+3 个更早），C0 100/100 保序；不读数据的客户端：C4 积压>64 KB 即断开（net_rejects_total{reason=slow_client}）。4) 畸形帧：10059 个（10^4 随机字节帧〔含握手前/后〕+ 59 个合法帧前缀）全部关闭连接，net_frames_malformed_total=10059，0 panic，后续连接仍正常。5) Hello→Welcome 往返 P50 0.07 ms（max 0.31 ms，40 次，debug）；10 并发握手含建连共 5.7 ms。另：入站令牌桶限速（150 帧/s，突发 300）超限断开；帧>4096 断开；每连接缓冲 64 KB；C1 解码后入 InboundQueue。M0-015 回归：release 空载 10 min（绑定 WS 端口，无连接）18000 tick，tick_ms P99 0.013 ms，interval_error P99 0.79 ms（<1 ms；上次 2.83 ms，共享虚机负载相关），overrun 0。检查：fmt、`clippy --all-targets -D warnings`、`cargo test -p frontline-server -p frontline-protocol`、check-deps 通过。另按总控交办修了 skeleton_smoke 的负载敏感断言：原 sleep 1.5 s 后断言 tick 计数≥30、overrun/consecutive_max 精确为 0、SIGTERM 测试 ticks≥10（0.7 s 后）；改为轮询直到 tick≥5（上限 4 s），overrun 系列只断言存在，SIGTERM 测试等 1.0 s 且 ticks≥3；功能性断言（/metrics 200、桶数、日志 JSON 含 match_id/tick/level、退出码）不变。 | 新增依赖 tokio-tungstenite 0.26 / futures-util / bytes（均 MIT）；协议帧 `[ch u8][len u16 LE][payload]`（framing.rs，含 proptest/golden）；新增 `Command::ClientGone`（M0-018 接手）；Welcome.team 暂为 player_id&1 占位，M0-018 应改；`ClientTransport` 另含 `handle()`（ClientHandle 供 M0-019 发送，send 不阻塞 sim），M0-017 复用 `conn::run_connection`；WS 层握手超时覆盖 TCP 接入起的 5 s（含 HTTP 升级）；per-IP/总连接超限在升级前丢弃 TCP；性能：未测真实网络下 CPU，只有环回数据。 | ca977df |
| M0-017 | server：WebTransport 传输 | done | 验收 1-6 通过（验收 3 用无头 Chromium 153 代替桌面 Chrome 人工操作，见下）。1) `wt_loopback`：100 个 C1 datagram→C2 回显 **100/100 到达且保序**（loopback，max_datagram_size 1413）；C0 与 C4 各 1000 条（服务器推送）保序无丢，另 1000 次 C0 Ping/Pong 往返（8 s，限速内）保序。2) `wt_stress`（release，`--ignored`，日志 reports/m0/M0-017-wt-stress-120s.log）：服务器以真实二进制子进程运行（`--dev-cert --dev-echo`），100 个 wtransport 客户端（绑 25 个 127.0.x.y 源地址以保留每 IP≤4 限制）×20 C1/s 上行 + 20 C2/s 回显下行 × 120 s：上行 240100/240100、下行 240100/240100 = **100.0000 %**，0 次意外断开，0 超限/丢弃，回显 RTT P50 1.35 ms / P99 5.50 ms / max 17 ms，握手 P50 51 ms（100 个并发，含 QUIC Retry；max 1.02 s）；服务器 CPU 13.66 s/121 s = **11.3 % 单核**（≈0.11 %/连接，含 QUIC/AEAD）；窗口内 `server_tick_ms` P99 桶上界 0.5 ms，整进程 tick_ms P99 0.050 ms / max 1.34 ms、interval_error P99 0.20 ms、overrun 0（预算 ≤28 ms）。注意：M0-018 未到，sim 里无玩家，故 tick 数字只反映网络线程争用，不含 sim 自身开销。3) 浏览器：`tools/dev/wt-smoke/headless.cjs` 在本机无头 Chromium 153 里用开发证书指纹连入 → ready、maxDatagramSize(客户端侧)=1024、收到 Welcome、10 个 datagram 收 10 个回包（reports/m0/M0-017-chromium-smoke.txt/.png；服务器侧该会话 max_datagram_size=1389）。UDP 在本容器可用（loopback）。无头 Chromium 的坑：页面若由 Playwright `route` 伪造 http://localhost 响应，则 Chrome 153 的 Local Network Access 检查报 ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS；用真实 localhost HTTP 服务器托管页面则无需任何 flag。4) `transport_matrix`：9 个 M0-016 握手/限制用例各跑 [ws, wt] 共 18 个全过（Welcome 字段/PlayerId/ClientGone、Reject 后关闭、5 s 无 Hello 超时、257 B ticket、第 5 连接被拒、非法消息、C1 入队与 Disconnect、限速、握手延迟：Hello→Welcome P50 WS 0.20 ms / WT 1.06 ms，10 并发握手 WS 3.3 ms / WT 27.4 ms，debug）；M0-016 的 ws_* 测试全部保持通过。5) 超大 datagram：3000 B C2 与 70000 B C3 被丢弃且 `net_datagram_oversize_total` 各 +1，后续小包照常到达；边界 `[ch]+payload==max` 发出、+1 丢弃。6) `gen_dev_cert`：13 天（notBefore Nov 17 / notAfter Nov 30，`openssl x509 -noout -dates`）、ECDSA P-256、SAN localhost/127.0.0.1/::1，指纹与 `openssl x509 -outform der | sha256sum` 一致，输出 hex+base64。另：畸形输入 316 例（数据报：空/未知通道/非 C1 通道/C1 垃圾；流：超长帧头、垃圾 C0、第二个 Hello、额外 bidi/uni 流；握手前后的随机帧）每例恰好关闭会话并计数 1 次；半帧后 FIN 视为普通断开（不计畸形）；C1 洪泛触发 rate_limit。检查：fmt、`clippy --all-targets -D warnings`、`cargo test -p frontline-server -p frontline-protocol`、check-deps 通过。 | 新增依赖 wtransport =0.7.2（MIT/Apache，features=["quinn"]；经 quinn 0.11/rustls 0.23/ring 0.17(Apache-2.0 AND ISC)/rcgen 0.14），未装 cargo-deny 故未跑 `cargo deny`。协议细节（DEVELOPING.md 已写）：C0/C4 流帧 `[len u16 LE][payload]`（无通道字节，与 WS 的 3 字节信封不同），datagram `[ch u8][payload]`，C1 只许走 datagram、C2/C3 仅下行；会话 URL 路径 `/` 或 `/wt`；服务器 QUIC keep-alive 3 s、idle 15 s；未验证源地址先走 QUIC Retry 再做每 IP 准入（防伪造源占位）；优雅关闭=finish→等对端 ACK≤1 s→等对端先关≤0.5 s（避免 CONNECTION_CLOSE 吞掉 Reject）。`ClientHandle::max_datagram_size()` 暴露给 M0-019（`datagram::packet_budget`）；`ClientTransport` 新增 `kind()`；新指标 `net_transport_connections{transport}`、`net_datagram_oversize_total`、`net_datagrams_in/out_total`。服务器新参数 `--cert/--key/--dev-cert`、隐藏的 `--dev-echo`（M0-018 前的 C1→C2 回显脚手架，M0-018 应移除或保留为测试专用）。未做：Firefox/Safari 互通与受信任证书（M0-028 / H-017）、桌面 Chrome 人工操作（以无头 Chromium 代替）、C5 bulk 流只有实现无测试。 | 8819f08 |
| M0-018 | server：会话、输入缓冲与校验 | todo | | | |
| M0-019 | server：AOI v0 与快照发送 | todo | | | |
| M0-020 | client/render：引擎初始化与调试叠层 | partial | 2026-11-17 无 GPU 环境（4 核 Xeon 2.3 GHz，Chromium 软件渲染）实测：①验收1 `pnpm -C client test:e2e` 5/5 通过：`?renderer=webgl2` 5 点（中心+4 角）与清屏色差>30 且 60 帧后变化≤6，无 console error/pageerror；WebGPU 真机项**未测**。②验收2 `__perf.dump()` 固定 16 字段 schema 已测（vitest+e2e）；WebGPU(SwiftShader)路径 `gpu_ms.source=timestamp-query` 有值，**数值为软件适配器耗时，真 GPU 未测**；WebGL2/SwiftShader 无可用计时则回退 `interval-estimate`（SwiftShader 上 EXT_disjoint_timer_query 可用但读数~0.0006 ms，不可信）。③验收3 compute 冒烟在 SwiftShader WebGPU 上 `__smoke.ok=true, mismatches=0, 782 ms`，Hi-Z/froxel(921600/921600)/TAAU 三试验均通过；真 GPU **未测**；结论见 reports/m0/compute-feasibility.md。④验收4 FixedStepper vitest 通过（100 ms→3 步；1000 ms→5 步且 clamped+1；1 h 随机 dt 偏差≤1）。⑤验收5 `navigator.gpu=undefined` auto→WebGL2，页面导航起 219 ms 内就绪（≤2 s），叠层显示 `fallback: navigator.gpu is undefined`。⑥验收6 1920×1080 空场景 60 s（600 帧窗口，WebGL2/SwiftShader）CPU 帧时间 P50 0.30 ms、**P99 0.60 ms**（≤4 ms）、60.0 fps；WebGPU(SwiftShader+shim) P99 0.50 ms；仅软件渲染，参考机未测。检查：`pnpm lint`/`typecheck`/`test`(18 项)/`build` 全绿。 | PI-005（compute 需自写薄封装；无头 WebGPU 无法 present，e2e 用 `client/e2e/webgpu-swiftshader-shim.js` 离屏）、PI-006（步长取 1000/30）；WebGPU 真机帧时间/像素/compute 需参考机复测（H-021）；WebGPU 路径无像素校验 | 09e58e0 |
| M0-021 | client/net：传输与时钟同步 | done | 1) clock_sync（假定时器，RTT 80 ms、每条链路 ±10 ms 均匀抖动、60 s、5 个种子）：连接后 5 s 起 551/551 个采样 |估计−真值|≤1 tick（最大误差恰为 1，Pong 的整数 tick 量化所致，已加 +0.5 tick 补偿）；2) fallback 4 用例 + 附加（WT 构造抛错、WT 就绪但握手无应答、两路皆死、transport=wt 不回退）全绿，ready 永不 resolve 时 3 s 回退、假时钟总耗时≤5 s，真浏览器（UDP 黑洞）回退后 WS 握手完成 3014 ms；3) 真实联调（无头 Chromium + 真 frontline-server，页面由 localhost HTTP 托管，e2e/net.spec.ts 11 例全绿）：WT 路径 transport=wt、connect 13.5 ms（含 QUIC 建连）、Hello→Welcome 2.1 ms、回环 RTT EWMA 1.8 ms（抖动 0.87 ms）；WS 路径 transport=ws、connect 39 ms、Hello→Welcome 2.8 ms、RTT 1.83 ms（0.62 ms）；Welcome tick_hz=30、match_seed 非零；C1→C2 回显（--dev-echo）3.5 s 内 104 包两路各收到；自动回退：浏览器无 WebTransport / 证书未固定 / 哈希错误 / 端口关闭均 <40 ms 回退 WS；服务器 SIGTERM 后 net.disconnected，再启动同端口后重新 connect 成功（WT、WS 各一例）；4) 握手拒绝 4 例（WT/WS × 版本/数据哈希不符）显示 `Server rejected the connection: version_mismatch|data_hash_mismatch ...` 且 WT 路径的 Reject 不回退 WS，另有 Welcome.data_hash 不符中止与握手超时单测；5) EWMA [100,100,200,100]→[100,100,110,109] 逐项一致；6) EventBus 6 例（同步、异常隔离、退订、100 万次 emit 堆增量<1 MB）；帧格式与 protocol 黄金字节/黄金消息一致（codec.test.ts）；每帧 net 处理（收 400 B 快照 + 时钟查询 + slack + 发 1 包）实测 1.9 µs（预算 200 µs）。vitest 全部 net/game 53 例通过，eslint/tsc 无错。测试命令：`pnpm -C client test`；联调 `FRONTLINE_REQUIRE_SERVER=1 pnpm -C client exec playwright test -c playwright.net.config.ts`（需 target/debug 的 frontline-server、gen_dev_cert 与 sim-wasm pkg，可用 FRONTLINE_SERVER_BIN/FRONTLINE_GEN_CERT_BIN 指定），数据落 reports/m0/M0-021-net-e2e.json | 仅 Chromium 验证 serverCertificateHashes（Firefox/Safari 归 M0-028，H-017）；M0 不做自动重连（M1-040），断线后由调用方再次 connect()；Pong 的 server_tick 为整数，估计加 0.5 tick 补偿且取最小 RTT 样本；wasm 模块由调用方注入（M0-022 接线，net 未改 main.ts）；input_slack 目标值取 1 tick、校正每秒 ≤0.1 tick（解释 §6.2「±10%/s」） | |
| M0-022 | client/game：输入、预测、对账与插值 | todo | | | |
| M0-023 | client/render：CDLOD 地形 | partial | 2026-11-17 本机（4 核 Xeon，Chromium/SwiftShader 软件渲染）实测：①验收1 GPU 帧时间**未测，需在参考机上测（H-021）**（复测入口 `/?scene=terrain&cam=0|1|2&debug=1`，叠层 GPU timer；M0-030 的三个机位待其定义，现用 `cam=0/1/2` 占位）。②验收2 `selectNodes`（含视锥平面提取）1000 随机位姿中位数 **0.077–0.082 ms**、p99 0.13–0.17 ms（<0.3 ms 通过，vitest）；**节点数中位未达下限**：可见 **27**（15–46）、视锥前切分叶子 76，验收要求 [150,450]——按 §9.2 同一张表 + 共享 65×65 网格每 LOD 环仅约 11 个节点，该估计与表不自洽（PI-014②，未调参凑数；对应三角形 ≈0.22 M/帧，远低于预算）。③验收3 数值：`cdlod-quadtree.test.ts` 真实选择结果上 128 对随机相邻节点（同级/差 1 级，共 574 条混合级共享边可选）×10 000 采样点，最差高度差 **9.8e-15 m**（≤1e-4），另全部共享边×257 点与真实 m0 曲面各一遍；对照：去掉邻接掩码（纯距离 morph）最差 **1.67 m**（→PI-014①：§9.2 纯距离 morph 会裂缝，加每实例邻接掩码 + 2:1 平衡）；GPU 侧：flat 着色+洋红清屏、25 倍垂直夸张、13 个随机机位 819 个 LOD 接缝节点，洞像素 **0**（未装掩码对照在平滑 m0 面上同为 0，像素检测对照不灵敏，故数值测试为主证据）；截图 `docs/measurements/M0-023/orbit-lod-0..7.png`（`?terrain_debug=lod` 绕场景一圈，无可见裂缝/洞）、`ground-shaded.png`、`far-ring-edge.png`、`overview-normal.png`。④验收4：`Terrain.heightAt` 对 sim `CollisionData::height_at`（M0-010，一次性临时 Rust 工程调用，1000 随机块内点）最大误差 **6.7e-6 m**；对独立 numpy 参考（`client/scripts/gen-height-ref.py`，1000 块内点 + 1000 块外点 + 边/角点，已入库为 fixtures）最大 4.6e-7 / 1.4e-7 m（≤1e-3）；GPU 采样 vs CPU（height 调试模式俯视，近景 643 点/远景 360 点）最大 0.032 / 0.023 m（8 位灰度步长 0.039 m）。待 M0-014 后把 sim 比对改为 vitest 调 wasm。⑤验收5：`?renderer=webgl2` 下 6 种调试模式（none/lod/morph/normal/height/flat）全部编译渲染、无 console error，**1 个 draw call**；WebGPU(SwiftShader+shim) 路径 WGSL 编译并绘制、无校验错误、1 draw call，**无像素校验**；WebGL2 Iris Xe 帧时间**未测**。⑥验收6：近景+远景纹理上传 **2.13 MB**（RG8 原样上传 u16，<4 MB；R32F 为 4.4 MB，见 PI-014③）；地形数据 fetch+解码+建树+首次选择 **48–100 ms**（≤500 ms）；页面导航起到首帧 630–830 ms（vite dev 未打包，含 M0-020 引擎启动 430–640 ms，非地形部分）。⑦验收7：块边两张图原始差最大 **0.446 m**（≤0.5），融合后曲面在块边 2 mm 步长跳变 2e-8 m（近景块外 16 m smoothstep 过渡，PI-014④）；截图 `far-ring-edge.png`，4 km 外环边缘剪影 e2e 通过。检查：`pnpm -C client lint`/`typecheck`/`test`(44 项)/`build` 全绿；`pnpm -C client test:e2e` 12/12 通过（含 M0-020 的 5 项）；`cargo` 未改。新增：vite 无新依赖；`client/scripts/sync-map.mjs`（`pnpm -C client sync-map`，dev/build/e2e 自动调用，必要时先跑地图流水线）；`client/src/node-test-shims.d.ts`（不引 @types/node）；`.gitignore` 增 `client/public/maps/`、`client/test-results/`（总控需 `git rm --cached -r client/test-results`，之前的 wip 提交误入）。 | PI-014（纯距离 morph 裂缝→邻接掩码；节点数与验收2下限不符；RG8 高度纹理；近远景融合带；LOD3 与四叉树对齐）；真 GPU 帧时间（验收1/5 Iris Xe）与 WebGPU 像素需参考机复测（H-021）；M0-030 需定义三个机位并接管 `cam=` 预设 | c020a7e |
| M0-024 | client/render：植被实例化与 impostor | partial | 2026-11-17 本机（4 核 Xeon，Chromium/SwiftShader 软件渲染，WebGL2 像素校验 + WebGPU 编译/绘制/方向校验）实测，入口 `/?scene=vegetation&cam=0|1|2&debug=1`（地形 + 植被；`?scene=terrain` 不变）： ①验收1 **帧率未测，需在参考机上测（H-021）**（RTX 3060 WebGPU 1080p ≥60 fps、Iris Xe WebGL2 720p Low ≥30 fps 均无硬件；SwiftShader 软件渲染不代表 GPU，不报数；复测：`?scene=vegetation&quality=high|low&debug=1` 叠层导出 + `__perf.dump()`）。可客观量的负载：ground-north 机位植被 49 draw、8 130 实例、450 k tri（+地形 311 k tri、1 draw），overview 32 draw / 9.9 k tri，west-edge 12 draw / 318 k tri；Low 档 cam0 44 draw / 76 k tri；上界 16 块×8 mesh=128 draw。 ②验收2 draw call（含地形，SceneInstrumentation）最大 **50**（WebGL2）、**50**（WebGPU/SwiftShader），均 ≤400；Low ≤500、WebGL2 Medium ≤900 也满足。 ③验收3 `scatter.test.ts`：同 seed 逐字节相同；16 块上限 grass 实测最大 2 770 ≤4096、shrub 256（上限生效）≤256、tree 73 ≤192；条目 6 B（**PI-020：§9.6 字段宽度和为 7 B，x/z 各压成 12 bit**）；泊松间距实测 tree–tree 4.023 m、shrub–shrub 1.500 m、shrub–tree 1.502 m（跨块边界成立）；坡度 >30° 与出生点 6 m 零实例。 ④验收4 三角形：树 LOD0 **3 140**、LOD1 **852**、灌木 LOD0 **576**、LOD1 **120**，均在范围内（草 LOD0 35 / LOD1 3 tri）。 ⑤验收5 `impostor-coverage`（WebGL2，完整 1536² atlas）150 m×8 方位前景面积差：树最大 **13.2 %**、灌木 **18.4 %**（≤25 %），轮廓 IoU 最差 0.80/0.84（防镜像）；WebGPU（lowres 768² atlas，离屏捕获）树最大 9.3 %、IoU 最差 0.90；atlas 12×12×128=1536×1536（albedo + 法线各一张，树/灌木各一对）；烘焙耗时（SwiftShader，4 张 1536²）约 2.1–3.5 s，lowres 约 1.9 s；每帧不回读。 ⑥验收6 30 s 6.4 m/s 径向穿越（1800 次 update，浏览器 JS，0.1 ms 计时分辨率）：重分桶总耗时（分类+上传）P50 0.10 / **P99 0.30–0.50** / max 2.3–5.6 ms，仅分类 P99 0.30 ms（≤0.5 通过）；node 单测 48 桶 P99 0.15 ms；无实例丢失/重复，无 >33 ms 尖峰（update 侧；**整帧尖峰需参考机**）。⑦验收7：材质均 `PH_V-NAT-002/004*`、节点 `PH_V-NAT-00x__…`（单测 NullEngine + e2e 真实场景遍历）；源码 grep 无外部 URL/贴图文件。风动（顶点着色器）e2e：树冠像素变化 445–2112、树干带 0。 检查：`pnpm -C client lint`/`typecheck` 干净，`vitest` 19 文件 138 测试通过，`playwright test vegetation.spec.ts` 7/7（另 render-bootstrap fixed-schema 与 terrain 全部复跑通过；terrain 高度测试在整套并行高负载下曾因截图超时失败一次，单跑 39.7 s 通过）。 | ①**验收1 帧率未测（H-021 参考机）→ partial**。②GPU 显存/内存：植被 atlas 4 张 1536²（含 mip）约 48 MB、实例缓冲 CPU 5.4 MB，每块每 LOD 一份独立几何（见③）。③踩坑：Babylon 薄实例的矩阵 buffer 挂在 geometry 上，克隆 mesh 共享 geometry 会互相覆盖，故每个块 mesh 用 `makeGeometryUnique()`（几何按块复制，约 4 MB 级）。④相机瞬移后需 `Vegetation.settle(camera)`（update 默认 0.45 ms 预算，分帧补齐）。⑤impostor 视角切换无交叉淡入（硬切，M1 视需要加抖动过渡）；impostor 只烘焙无阴影；草 LOD 间切换同样硬切，远处草按末 25 % 距离缩成 0。⑥草/树/灌木均为灰盒，缺素材 V-NAT-002、V-NAT-004（已写 NEEDS_HUMAN §11.4）。⑦改了共享文件：`main.ts`（`?scene=vegetation`、`__vegetation`、perf 的 `vegetation` 字段）、`debug/perf-export.ts`（PerfDump 新增顶层字段 `vegetation`，同步 `render-bootstrap.spec.ts` 的固定字段列表）、`client/scripts/sync-map.mjs`（多拷 `spawns.json`）。M0-030 三机位请用 `?scene=vegetation`。PI-020 登记。 | |
| M0-025 | client/render：灰盒角色、第一人称枪与射击反馈 | todo | | | |
| M0-026 | bots：M0 基础 bot | todo | | | |
| M0-027 | headless-client v0 与 10 客户端联测 | todo | | | |
| M0-028 | 测试：浏览器 WT/WS 互通矩阵 | partial | 验收 1、4 通过，2、3 仅 Chromium 部分通过（Firefox/WebKit 本机无构建，如实记"未验证"，矩阵无空格）。矩阵（reports/m0/browser-matrix.{json,md}，scripts/check-interop-table.mjs 比对一致）：<br>chromium 153.0.8010.12：WT=通过（199 快照/10 s，RTT 中位 1.1 ms）；WS=通过（199 快照/10 s，RTT 中位 0.9 ms）；回退（删除 WebTransport）=通过（回退 WS，10 ms）；回退（WT 端口关闭）=通过（回退 WS，13 ms）；headless Chromium；C2 来源 dev-echo<br>firefox n/a：WT=未验证（browser binary not installed (npx playwright install firefox)）；WS=未验证（browser binary not installed (npx playwright install firefox)）；回退（删除 WebTransport）=未验证（browser binary not installed (npx playwright install firefox)）；回退（WT 端口关闭）=未设计（任务仅要求 Chromium）；Playwright 打补丁的 Firefox 构建<br>webkit n/a：WT=未验证（browser binary not installed (npx playwright install webkit)）；WS=未验证（browser binary not installed (npx playwright install webkit)）；回退（删除 WebTransport）=未验证（browser binary not installed (npx playwright install webkit)）；回退（WT 端口关闭）=未设计（任务仅要求 Chromium）；WebKit 构建 ≠ Safari 正式版。证据：验收 1/3(Chromium)/4 = 本任务新增 client/e2e/interop/interop.spec.ts 4 个用例（含 10 s 探针）；WT/WS 握手、证书哈希、UDP 黑洞 3.2 s 回退、端口关闭、Reject 不回退、断线重连 = 引用并重跑 M0-021 的 client/e2e/net.spec.ts（11/11 通过）；验收 5 = scripts/check-interop-table.mjs；验收 6 = PLAN_ISSUES PI-021。vitest fixtures 4 个用例通过。 | partial 原因：Firefox、WebKit 的 WT/WS/回退格与 Safari 无法在本机验证（H-103，人工步骤见 DEVELOPING.md "Browser interop matrix"；CI 矩阵 .github/workflows/interop.yml 待 H-102 后实跑）；受信任证书属 H-017。探针当前数的"快照"是 --dev-echo 的 C1→C2 回显（M0-019 无真实快照），M0-019 完成后须复跑；未新建 interop.html/interop-probe.ts（复用 M0-021 的 net-harness 页，不重复造）。 | |
| M0-029 | 测试：确定性与 WASM 漂移 | done | 机型：本机 Xeon 云主机 4 核（非参考机）。3 个种子 × 18 000 tick（10 min，30 Hz）随机输入（走/跑/冲刺/蹲/跳/瞄准/脉冲开火 144–161 发/换弹/大幅转视角），M0 真实地形，三个出生点（南坡脚、陡坡 39° 一带、北高地；到访 6/6/9 个碰撞块，y 1.7–10.8 m，含被坡/边界阻挡的 tick）。1) native×2（`PredictSim`，第二次前扰动分配）逐 tick state_hash **54 000/54 000 相等**；另 10 玩家 `World`×2 每种子 18 000 tick **全等**。2) wasm×2（同一 wasm，第二个实例在垃圾分配+另一会话之后）**54 000/54 000 相等**。3) native vs wasm：每 tick 哈希与位置**逐位相同（无分歧 tick）**，漂移 rms_m=0、max_m=0、cm_per_s=0、over_0_05m_ratio=0（三种子；每 60 s 分段最大误差全 0，无随时间发散），目标 <2 cm/s 满足。4) CI `determinism.yml` 已写（native 测试+示例、wasm、compare、vitest）；**未在 GitHub 实跑，墙钟 <6 min 未取得**；本机各步合计约 15 s 运行 + 冷编译。5) 故障注入（SystemTime 纳秒扰动第二次运行 yaw）使对比在第 700 tick 失败（`injected_nondeterminism_is_detected`）。6) summary.json 重跑逐字节相同（含版本：DATA_HASH 0xeb4ef8d3b81fc45c、rustc 1.97.0、rapier3d 0.36.1 enhanced-determinism、wasm sha256）。耗时（18 000 tick）：native 0.28–0.35 s、wasm（Node）0.61–0.80 s（含每 tick stateHash）、10 玩家 World 3.4–4.1 s。golden：seed1 终值 0xb170_3824_1d9e_4c55。严格模式开关：`--strict`/`FRONTLINE_DETERMINISM_STRICT=1`，默认只告警（R5）。顺带：client `terrain-vs-sim.test.ts`（vitest 调 wasm，新增 `Predictor.heightAt`）506 032 点 `TerrainData.heightAt` vs sim `height_at` 最大误差 6.9e-6 m。检查：`cargo test -p frontline-sim-wasm`、clippy -D warnings、`pnpm ci:test` 18 项、vitest terrain 全过；`pnpm typecheck` 仅有他人 vegetation 目录的既有错误。 | PI-019（斜率度量定义、输入不入库改为运行时生成、出生点、`heightAt`）。建议 CI 开严格模式（实测零误报）。 | 58b880d |
| M0-030 | 测试：M0 性能基线与报告 | todo | | | |
| M0-031 | 史实参数复核：Kar98k 与 M1 Garand（R8） | done | 1) 两文件 historical 共 15 字段，13 已核实（有 locator 的 _source 记录）+ 2 未核实（Kar98k 初速、射速，登记 _meta.unverified）= 15；`node tools/data-validate/index.mjs` 退出 0（hist-source-required 开启）；2) 汇总表 reports/m0/hist-review-m0.md；3) 一手来源：TM 9-1275、FM 23-5（战后版）、TM-E 30-451；4) 数值改动：M1 实用射速 40→24，新增 effective_range_m=460；gameplay 未改；5) `pnpm -C tools/data-validate test` 47/47；`cargo test -p frontline-sim` 全绿、clippy -D warnings、fmt 干净（data_weapons_m0.rs 改为 status=Reviewed 并加 historical 断言）；DATA_HASH 无字面量断言；6) 无原文入库。遗留：Kar98k 一手来源（HDv/Hogg/博物馆）未找到；两 status 现为 reviewed；见 PI-012 | | 847c704 |
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
