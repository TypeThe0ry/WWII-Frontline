# B6-acceptdemo · M1-072（2026-11-20）

- 基线 `0e5b41f`（里程碑线 #163）。核对已落地 B6 #153–#163，补入两轮 Carentan WT 1,800 s 无 netem 的带宽与同 tick AOI、素材台账和 LOD、反作弊注入实测。E1 共 3 PASS / 11 partial / 3 未测 / 0 FAIL。详情见 `reports/m1-acceptance.md`。
- `scripts/demo-m1.sh` 最终复跑退出 0。无头 Chromium 153.0.8010.12 软件 WebGL2，Playwright 1/1（2.5 min），core Brotli 1,193,738 B，驻点重生距离 5.61 m，Officer 权威备弹 120 → 90 → 120。Medic/压制 sim 9+14 通过，Sherman 独立 M0 WS 1/1，权威位移 11.17 m，据点规则 6/6。HTTP `/metrics` 4,416 个 tick 样本，主动停服后网关写入 4,418 tick。逐步范围见 `reports/m1-demo-run.md`。首轮浏览器成功后，执行期间脚本文件被修改导致后半段未跑，完整复跑取代该轮。
- `scripts/local-ci.sh determinism` 通过，3 种子 × 18,000 tick 逐 tick native/WASM 一致。`scripts/local-ci.sh ts` 通过（97 文件 / 741 tests）。生产客户端构建、相关 ESLint/TypeScript、`cargo fmt --all --check`、`bash -n`、`shellcheck`、`node tools/ci/check-acceptance.mjs` 和对应 3 个测试均通过。`ledger-check` 为 242 行（232 OK / 3 PENDING / 1 FLAGGED / 0 REJECTED / 6 INTERNAL）。全部工作站执行（PI-109，非参考机）。
- M1-072 `partial`，M1 保持开放。预发布及正式 TLS、真人实图和录像、自然结算界面、参考设备、M1 对标追测仍未测，人工进展已登记。UI 视觉待 Fable 设计。无新增素材。
- 收尾清理工作树 `target`、`client/node_modules/.vite` 与演示专用 scratch，完整日志保存在共享 `out/batch6/logs/acceptdemo-*`。落地使用 `scripts/dev/land-ms.sh acceptdemo`。

落地 rebase 到 `90770bf` 包含 #164/#165 的文档更新，验收表、STATUS、计划与人工进展保留双方条目。两次合入的变更均为文档，测试代码与运行基线一致。
