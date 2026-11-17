# M0-001 仓库脚手架与工具链

## 元数据
- 里程碑：M0
- 工作流：repo
- 规模：M
- 前置依赖：无
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §4.1; ARCHITECTURE §4.2; ARCHITECTURE §4.5; ARCHITECTURE §5.1; ARCHITECTURE §5.2; MILESTONES §1.2

## 背景
M0 之后的全部任务都建立在本任务产出的仓库骨架上。ARCHITECTURE §4.1 固定了布局：`/client`（TS + Vite + Babylon.js）、`/crates/*`（8 个 Rust crate）、`/tools/{asset-pipeline,map-pipeline}`、`/data`、`/assets/{incoming,processed}`、`/docs`、`/tasks`、`LICENSES.md`。§4.2/§4.5 给出 crate 间依赖方向（图中未画出的依赖一律禁止），本任务必须在 `Cargo.toml` 里把允许的边建好、禁止的边一条都不建。
本任务只建"空壳"：所有 crate 只有能编译的占位 `lib.rs`/`main.rs`；client 只有能构建的最小页面。真正的功能由 M0-005 起的任务填充。下游：M0-002（CI）依赖本任务的文件结构；M0-004/005/009/020 在此基础上加内容。

## 具体内容
### 要创建/修改的文件
- `Cargo.toml`（workspace，`resolver = "2"`，`members = ["crates/*"]`，统一 `[workspace.package]` edition/license/rust-version、`[workspace.lints]`：`rust.unsafe_code = "forbid"`（`sim-wasm` 单独放宽）、`clippy::all = "deny"`）
- `rust-toolchain.toml`（固定 stable 版本号；`targets = ["wasm32-unknown-unknown"]`，`components = ["rustfmt","clippy"]`；版本由执行者取当时 stable 并记录）、`rustfmt.toml`、`.cargo/config.toml`（`[alias] ci-lint = "clippy --workspace --all-targets -- -D warnings"`）
- `crates/{protocol,sim,sim-wasm,server,gateway,bots,headless-client,balance-sim}/Cargo.toml` + `src/lib.rs`（`server`/`gateway`/`headless-client`/`balance-sim` 为 `src/main.rs`）。依赖边严格按 ARCHITECTURE §4.5 表：`sim→protocol`；`sim-wasm→sim,protocol`；`bots→sim,protocol`；`server→sim,protocol,bots`；`gateway→protocol`；`headless-client→sim,protocol`；`balance-sim→sim,protocol,bots`；`protocol` 无内部依赖。外部依赖此时只加占位所需（不提前引入 tokio/rapier 等）。`protocol/lib.rs` 带 `#![no_std]` + `extern crate alloc`。
- `client/package.json`、`client/tsconfig.json`（`strict`、`noUncheckedIndexedAccess`）、`client/vite.config.ts`、`client/index.html`、`client/src/main.ts`、`client/src/{render,ui,audio,input,net,game}/index.ts`（每个目录只导出空占位）、`client/eslint.config.js`。依赖：`vite`、`typescript`、`preact`、`@preact/signals`、`@babylonjs/core`（**精确版本，不用 `^`**，取当时 8.x 最新稳定版并记录；ARCHITECTURE §5.1 要求锁版本）、`vitest`、`eslint`。
- `pnpm-workspace.yaml`（packages: `client`、`tools/*`）、根 `package.json`（脚本 `build`/`lint`/`test` 代理）、`pnpm-lock.yaml` 与 `Cargo.lock` 入库。
- `tools/asset-pipeline/package.json`（空壳）、`tools/map-pipeline/README.md` + `pyproject.toml`（空壳，M0-009 填充）、`data/README.md`（说明 JSON 为数值唯一真源，ARCHITECTURE §4.4）。
- `.gitignore`：`assets/incoming/`、`assets/processed/`、`docs/measurements/raw/`、`assets/incoming/ref-captures/`、`target/`、`node_modules/`、`dist/`、`*.pdb`。`assets/incoming/.gitkeep` 不入库（目录由人工创建）。
- `DEVELOPING.md`（根目录开发说明：前置工具版本、`cargo build --workspace`、`pnpm install && pnpm -C client dev/build`、目录说明、任务流程指向 `tasks/README.md`）。
### 要实现的行为
1. `cargo build --workspace` 在 `RUSTFLAGS="-D warnings"` 下零警告；`cargo test --workspace` 通过（可为空测试）。
2. `pnpm -C client build` 零警告产出 `client/dist/`；`pnpm -C client dev` 打开页面显示固定文本（不含 Babylon 渲染，渲染在 M0-020）。
3. 目录与 BRIEF §4 / ARCHITECTURE §4.1 一致（用脚本 `tools/ci/check-layout.mjs` 校验必有目录，供 M0-002 复用）。
### 接口与数据
- 无公开接口；`protocol::PROTOCOL_VERSION: u16 = 1` 可在此先声明（M0-005 沿用）。

## 不做什么
- 不写 CI 工作流与依赖图检查脚本（M0-002）。
- 不加载 Babylon 引擎、不创建 canvas（M0-020）；不写任何协议/模拟逻辑（M0-005 起）。
- 不引入 `tokio`、`rapier3d`、`bevy_ecs` 等运行期依赖（各自任务引入）。

## 验收标准
1. `RUSTFLAGS="-D warnings" cargo build --workspace` 退出码 0，警告数 0。
2. `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0。
3. `pnpm -C client build` 退出码 0，输出中无 warning；`pnpm -C client exec tsc --noEmit` 退出码 0。
4. `cargo metadata --format-version 1` 中 8 个 crate 的内部依赖边集合与 ARCHITECTURE §4.5 允许集合**完全相等**（手工比对并把比对结果写入 STATUS；M0-002 之后由脚本强制）。
5. `node tools/ci/check-layout.mjs` 退出码 0：BRIEF §4 所列目录全部存在。
6. `git check-ignore assets/incoming/x docs/measurements/raw/x assets/processed/x` 三者均被忽略。
7. `Cargo.lock`、`pnpm-lock.yaml` 已入库；`@babylonjs/core` 在 `package.json` 中为精确版本。

## 测试要求
- 单元测试：无（空壳）；每个 crate 保留一个 `#[test] fn smoke()` 以便 `cargo test` 非空。
- 集成/无头测试：无。
- 性能基准：无。
- 回归：无。

## 性能预算
无（无运行期行为）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 需要 GitHub 仓库与 Actions 可用（NEEDS_HUMAN H-014）；`.gitignore` 路径以 H-032 确认为准；依赖许可证政策见 H-037（本任务不引入 `cargo-deny`，M0-002 引入）。
- Rust 与 Node 具体版本号以执行当时的稳定版为准并写入 `rust-toolchain.toml`/`package.json#engines`；不要凭记忆写版本号。
- `docs/measurements/raw/` 在 REFERENCE_MEASUREMENTS §9.2 第 9 项中要求入 `.gitignore`，此处一并处理，M0-032 不再重复。
