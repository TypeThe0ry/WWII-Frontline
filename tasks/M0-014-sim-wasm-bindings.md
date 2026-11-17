# M0-014 sim-wasm：WASM 封装

## 元数据
- 里程碑：M0
- 工作流：sim
- 规模：M
- 前置依赖：M0-011, M0-006, M0-012
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §4.3.3; ARCHITECTURE §8.3; ARCHITECTURE §17.5; ARCHITECTURE §13.5.1

## 背景
`sim-wasm` 是 `sim + protocol` 的 `wasm-bindgen` 封装，暴露给 TS 客户端：预测会话（`Predictor`）、快照解码（`SnapshotDecoder`）、消息编解码、数据查询（ARCHITECTURE §4.3.3/§8.3）。**不得新增游戏逻辑**；与 JS 之间只传 `Uint8Array/Float32Array/POD`；`DecodedSnapshot` 用 SoA 视图（零拷贝 `subarray`，下一帧失效，调用者本帧内消费），避免每帧分配对象造成 GC 抖动。体积预算：`wasm-opt -O3` 后 ≤ 1.2 MB，brotli 后 ≤ 400 KB（E0-9）。构建：`wasm-pack build --target web`。依赖：`sim`、`protocol`、`wasm-bindgen`、`js-sys`、`console_error_panic_hook`。
M0 的预测范围（ARCHITECTURE §6.4）：本地玩家的移动 + 本地武器开火表现（后坐/弹药/冷却，不决定命中）。因此 `Predictor` 除 M0-011 的移动外还要含 M0-012 的武器状态机（M0-012 编号早于本任务，执行顺序上已完成；INDEX 依赖列表未列出，见备注）。

## 具体内容
### 要创建/修改的文件
- `crates/sim-wasm/Cargo.toml`（`crate-type = ["cdylib","rlib"]`；release profile：`opt-level="z"`或`3`二选一以体积为准、`lto=true`、`codegen-units=1`、`panic="abort"`）、`crates/sim-wasm/src/{lib.rs,predictor.rs,snapshot.rs,messages.rs,data.rs}`。
- `crates/sim-wasm/tests/node-smoke.mjs`（Node 里用 `initSync` 加载 wasm 的冒烟测试）；`tools/ci/check-wasm-size.mjs`（对 `pkg/*.wasm` 做 wasm-opt 后体积与 `brotli -q 11` 体积检查）。
- `client/package.json` 增加对 `crates/sim-wasm/pkg` 的 workspace 依赖（`pnpm-workspace.yaml` 纳入）；`.github/workflows/wasm-build.yml` 去掉 M0-002 的跳过条件并加体积门禁。
### 要实现的行为（TS 可见 API，按 ARCHITECTURE §8.3）
1. `init(): Promise<void>`、`protocolVersion(): number`、`dataHash(): bigint`、`weaponDef(id): Uint8Array`（JSON bytes，来自 `GameData`）、`roleDef(id): Uint8Array`（含 `capsule` 尺寸供 M0-025 使用）。
2. `class Predictor { constructor(matchSeed: bigint, mapId: number, myPlayerId: number); loadCollision(bx, bz, data: Uint8Array): void; unloadCollision(bx, bz): void; reset(authState: Uint8Array, tick: number): void; step(cmd: Uint8Array): Float32Array; state(): Float32Array; weaponState(): Uint8Array; stateHash(): bigint; free(): void }`。`step` 输出布局 `[x,y,z,vx,vy,vz,yaw,pitch,stance,grounded,ammo_in_mag,ammo_reserve,fire_cooldown_ticks,recoil_pitch,recoil_yaw,…]`，布局常量同时导出为 TS 常量表（`LAYOUT_*`）。内部使用 `PredictSim`（`feature="client-predict"`，M0-011）。
3. `class SnapshotDecoder { constructor(); decode(bytes): DecodedSnapshot; reset(): void }`；`DecodedSnapshot` = `{tick, ackInputSeq, inputSlack, self: Float32Array, ids: Uint8Array, pos: Float32Array(3n), yaw: Float32Array, pitch: Float32Array, anim/stance/weapon/flags/health: Uint8Array, removed: Uint8Array, events: Uint8Array}`，数组为 wasm 内存视图。
4. 消息封装（M0-005 的具体类型；ARCHITECTURE §8.3 原文的泛型 `encodeC0(kind,payload)` 无法描述 bit-packed 字段，故增加类型化函数）：`encodeHello({protocolVersion,dataHash,ticket,clientBuild,caps})`、`encodePing(clientTime)`、`encodeDisconnect(reason)`、`decodeServerC0(bytes)->{kind,fields…}`（Welcome/Reject/Pong）、`encodeInputBundle(cmds: Uint8Array, ackTick: number, predHash: number): Uint8Array`、`encodeInputCmd`、`packInputCmd(...)`；同时保留 §8.3 的 `encodeC0/decodeC0` 薄封装。
5. 数据哈希：`dataHash()` 即 M0-004 的 `DATA_HASH`；`Hello.data_hash` 由 TS 取此值。
6. `console_error_panic_hook` 在 `init` 里安装；panic 转为 JS 异常信息。
### 接口与数据
- 所有跨边界函数对非法输入返回 JS 异常（`Result<_, JsError>`），不 abort；`Predictor` 要求 `free()` 且在 TS 侧由 M0-022 管理生命周期。

## 不做什么
- 不做 TS 侧的传输、时钟、对账（M0-021/022）；不新增 `sim` 逻辑。
- 不做服务器端 WASM（服务器原生）；不做 SharedArrayBuffer/多线程（ARCHITECTURE §19.5/§20 #3：本设计不依赖）。
- 不做 wasm 与 native 的漂移比较（M0-029）。

## 验收标准
1. `wasm-pack build crates/sim-wasm --target web --release` 成功；`wasm-opt -O3` 后 `.wasm` ≤ 1.2 MB，`brotli -q 11` 后 ≤ 400 KB（`node tools/ci/check-wasm-size.mjs` 输出字节数写入 STATUS）。超标时先尝试：`opt-level="z"`、移除未用 Rapier 特性、`strip`；仍超标 → PLAN_ISSUES 并记 `partial`。
2. Node 冒烟 `node crates/sim-wasm/tests/node-smoke.mjs` 通过：`init → dataHash/protocolVersion 取值与原生一致 → new Predictor → loadCollision(16 块) → reset → step(120 次) 位置前进 → SnapshotDecoder 解码一个由原生 `protocol` 生成的黄金快照（`tests/golden/snapshot_0.bin`）字段相等`。
3. CI `wasm-build` 作业通过，含体积门禁，墙钟 < 8 min（ARCHITECTURE §17.5）。
4. `Predictor.step` 单次调用（Node 内测 10 000 次）中位数 ≤ 50 µs（含 WASM 边界开销；参考：客户端预测预算 `≈0.3 ms/帧`，§13.5.1）。
5. 无浮点以外的对象分配：`SnapshotDecoder.decode` 连续 1000 次调用，wasm 线性内存增长 ≤ 64 KB（`memory.buffer.byteLength` 前后对比）。
6. 任一非法输入（截断快照、错误长度 cmd）均抛出 JS 异常而非 abort（3 个用例）。

## 测试要求
- 单元测试：`cargo test -p sim-wasm`（非 wasm 目标可测的纯 Rust 逻辑）；`wasm-pack test --node` 或自写 Node 脚本。
- 集成/无头测试：Node 冒烟（验收 2）。
- 性能基准：验收 4，结果交 M0-030。
- 回归：golden 快照字节文件与布局常量快照测试（改布局须同步 M0-022）。

## 性能预算
`sim-wasm` ≤ 1.2 MB（br ≤ 400 KB）（ARCHITECTURE §4.3.3）；客户端 `game` 更新含 WASM 预测 0–2 步约 0.3 ms（§13.5.1）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖已含 M0-012：`Predictor` 的武器预测需要 M0-012 的 `WeaponState`，不得跳过武器部分。
- 体积是最大风险：Rapier 在 wasm 下体积可观；超标的备选方案（只含 KCC 子集、关闭 `parallel/simd`）写 PLAN_ISSUES，不要放宽 1.2 MB。
- INDEX 提到的 `apply_snapshot` 对应 `Predictor.reset(authState, tick)`（ARCHITECTURE §4.3.2 的 `apply_snapshot_for_prediction`）。
