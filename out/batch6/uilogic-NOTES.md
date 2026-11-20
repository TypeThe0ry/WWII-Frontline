# B6-uilogic · M1-053b（2026-11-20）

里程碑线。开发入口 `/ui-gallery` 不启动渲染器或网关；实现无样式 §9.1 组件、双语画廊、键盘与焦点管理、字体许可与子集化检查。**待 Fable 设计**：颜色、排版、图标风格与视觉回归基线。未提交字体、截图或第三方图标二进制。

| 工作站检查（PI-109，非参考机） | 结果 |
|---|---|
| `CARGO_BUILD_JOBS=8 scripts/local-ci.sh ts`；客户端 lint、typecheck、build | 本批开发基线 760 个 Vitest 通过；生产入口不含画廊；core Brotli 1,194,098 B / 8,000,000 B（含 sim-wasm）。 |
| `pnpm -C client run i18n:check` | 149 键、0 问题；在副本移除 `gallery.title` 后退出 1。 |
| `FRONTLINE_UI_FONT_BUNDLE=<完整子集目录> pnpm -C client exec playwright test -c playwright.ui.config.ts ui-gallery` | 9 项：真实键盘 Tab 顺序/方向键、双向焦点循环与恢复、提示 Escape、千行虚拟列表、双语双缩放审阅截图、字体失败回退/按需加载及预算。另跑大厅/对局夹具 6 项通过；需外部真实网关的既有 1 项未测。 |
| `ui-gallery-perf.spec.ts` | 画廊 62/65，8 人小队 HUD 74，空宿主 1；各 60 个静止帧均 0 Paint，阳性对照 4/6/4/4。100 signals/帧，预热 30 + 采样 120，均值 0.456 ms、P95 0.600 ms。原始数据与计数口径见 `reports/m1-ui-framework.json`。 |
| `subset.py` 完整构建；`FONT_CHECK_DIR=<完整子集目录> python3 -B -m unittest discover -s tools/font-subset -v` | 5 份 TTF → 8 份 WOFF2，逐文件比对 SHA、OFL、改名与 cmap；7 项通过。英文/中文首屏 55,123 / 78,809 B Brotli。见 `tools/asset-pipeline/docs/font-subsetting.md`。 |
| `ledger-check`、`ingest --metadata-only`、`report --manifest … --strict`；`pnpm -C tools/asset-pipeline test` | 243 行（235 OK、1 PENDING、1 FLAGGED、6 INTERNAL），227 条既有原档索引通过，Medium 缺口报告仅字体处置说明变化；175 项资产管线单测通过。没有重打不含这些字体的 Medium 包。 |
| `CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism` | 3 种子 × 18,000 tick，native/WASM 每 tick 哈希与位置一致。 |

M1-053 保持 `partial`：完整对局 UI 开销、参考机和真实读屏器未测；画廊截图不作为视觉金标。未重构各正式屏幕、未做通用 CanvasLayer 或将字体应用到正式界面。人工进展见 H-021/036/043，决定见 PI-B6-uilogic-1。没有调用 GitHub Actions。
