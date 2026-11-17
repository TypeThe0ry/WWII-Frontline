# M0-002 GitHub Actions 基线与依赖图检查

## 元数据
- 里程碑：M0
- 工作流：repo
- 规模：M
- 依赖：M0-001
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.5; ARCHITECTURE §4.5; ARCHITECTURE §4.3.2; ARCHITECTURE §19.5

## 背景
ARCHITECTURE §17.5 规定了 PR 阶段的 CI 作业与目标时长：`lint`(<6 min)、`test-rust`(<12 min)、`test-ts`(<5 min)、`wasm-build`(<8 min)，另有 `data-validate`(<2 min，M0-004 补全)。§4.5 的"禁止依赖速查表"要靠 `cargo deny` + 自写脚本检查 `cargo metadata` 依赖图，违规即失败（E0-10）。§4.3.2 对 `sim` 还有源码级禁令（`HashMap/HashSet`、`Instant/SystemTime`、`thread_rng`、`rayon`），单靠依赖图查不出，需要一个轻量源码扫描。
本任务建立这些守门员；之后每个任务的提交都要过它们。M0-014 补全 `wasm-build` 的体积检查，M0-029 追加 `determinism` 作业。

## 具体内容
### 要创建/修改的文件
- `.github/workflows/lint.yml` —— `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`pnpm -C client exec eslint .`、`tsc --noEmit`、`cargo deny check`、`node tools/ci/check-deps.mjs`、`node tools/ci/check-sim-purity.mjs`、`node tools/ci/check-layout.mjs`。
- `.github/workflows/test-rust.yml` —— `cargo test --workspace`，使用 `Swatinem/rust-cache`（或 `sccache`，ARCHITECTURE §17.5 写 sccache，二选一并记录）。
- `.github/workflows/test-ts.yml` —— `pnpm -C client test`（vitest）。
- `.github/workflows/wasm-build.yml` —— `wasm-pack build crates/sim-wasm`；`sim-wasm` 尚空时用 `if: hashFiles('crates/sim-wasm/src/**')` 之类条件跳过实质步骤并保持绿（M0-014 去掉跳过）。
- `deny.toml` —— 许可证白名单（MIT/Apache-2.0/BSD/ISC/Unicode/Zlib 等，具体清单待 NEEDS_HUMAN H-037 确认；未确认前取保守集合并在备注记录）、`advisories` 开启、`bans` 对 `openssl`（用 rustls）等给出 deny。
- `tools/ci/check-deps.mjs` —— 读 `cargo metadata --format-version 1`，把 ARCHITECTURE §4.5 的允许矩阵编码为数据表，对 8 个 workspace crate 的**内部依赖（含 dev/build 依赖）**做校验；另检查 `sim`/`protocol` 不得传递依赖 `tokio`、`wasm-bindgen`、`wtransport`、`rayon`（`protocol` 的 `std` 特性除外需说明）。违规打印"from → to（来自 Cargo.toml 位置）"并 `exit 1`。
- `tools/ci/check-sim-purity.mjs` —— 对 `crates/sim/src/**/*.rs` 做正则扫描，命中 `\bHashMap\b|\bHashSet\b|Instant::|SystemTime|thread_rng|\brayon\b|wasm_bindgen` 即失败（允许文件内 `// purity-allow: <原因>` 行注释豁免并列入报告）。
- `tools/ci/fixtures/` —— 两份故意违规的 `cargo metadata` JSON 与一份违规 `.rs` 样例，供脚本自测；`tools/ci/test/*.test.mjs`（`node --test`）。
### 要实现的行为
1. 依赖方向矩阵（§4.5）为数据，不散落在脚本逻辑里；矩阵与 §4.5 逐格一致。
2. 所有 PR 作业并行；总墙钟 < 15 min，且各作业不超过 §17.5 的目标时长。
3. `lint` 作业失败信息指明违规边或违规文件行号。
### 接口与数据
- `check-deps.mjs --metadata <file>`（默认实时调用 cargo）；退出码 0/1。

## 不做什么
- 不加 `determinism`、`manifest-budget`、`e2e-visual`、`loadtest`、`balance` 作业（分别由 M0-029、M1-005、M2-050、M1-068/M2-049、M2-051 等任务加入）。
- 不实现 `data-validate` 的校验逻辑（M0-004 提供工具并接入本任务建的占位作业）。
- 不做 sccache 远端存储或自托管 runner（H-014 决定）。

## 验收标准
1. 在测试分支上给 `crates/sim/Cargo.toml` 加入 `tokio` 依赖并发 PR：`lint` 作业失败，日志含 `sim -> tokio`；撤销后转绿（把两次运行链接/日志摘要写入 STATUS）。
2. 同理，给 `crates/protocol` 加 `sim` 依赖、给 `crates/gateway` 加 `sim` 依赖各被拒绝一次（可用 `node --test` 对 fixtures 自动验证，3 个违规样例全部 `exit 1`）。
3. `node --test tools/ci/test/` 全部通过；`check-deps.mjs` 对当前仓库 `exit 0`。
4. 在 `crates/sim/src` 加入 `use std::collections::HashMap;` 的故意违规样例使 `check-sim-purity.mjs` 失败；带 `purity-allow` 注释时通过并在输出中列出。
5. 全部 PR 作业在空缓存与热缓存下的墙钟分别记录；热缓存下总墙钟 < 15 min，`lint` < 6 min，`test-rust` < 12 min，`test-ts` < 5 min，`wasm-build` < 8 min（取 Actions 运行页数字）。
6. `cargo deny check` 在当前依赖树上退出码 0。

## 测试要求
- 单元测试：`tools/ci/test/` 覆盖允许边通过、禁止边失败、传递依赖失败、豁免注释三种路径。
- 集成/无头测试：见验收 1（真实 PR 触发一次）。
- 性能基准：记录各作业墙钟（见验收 5）。
- 回归：无。

## 性能预算
CI 目标时长即 ARCHITECTURE §17.5 表（lint <6、test-rust <12、test-ts <5、wasm-build <8 min）；本任务自身不得使后续任务被迫超出，若缓存配置后仍超限在 PLAN_ISSUES 登记。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 验收 1 需要真实 GitHub 仓库与 Actions（NEEDS_HUMAN H-014）；若 Actions 暂不可用，可用 `act` 或本地脚本演示，该项记 `partial` 并说明。
- 许可证白名单依赖 H-037 的政策确认；取保守集合时在 `deny.toml` 顶部注释写明"待 H-037"。
- `check-sim-purity` 是正则扫描，会有漏报（宏展开、别名 import）；它是辅助，不替代 code review。`bevy_ecs` 内部使用哈希表不受此扫描约束，查询顺序的确定性由 M0-007 的显式排序保证。
