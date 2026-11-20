# B6-gwstats · M2-022

2026-11-20，里程碑线，PI-109 工作站（非参考机）。

- **完成**：SQLite 0002、游客会话查询对局结果、按游客隔离的设置读写；复用 server 结算回投与 spool 重放，重复提交保留首笔结果及接收时间。Discord/Google OAuth 只预留路由和默认关闭的开关。
- **网关检查**：`CARGO_BUILD_JOBS=8 cargo test -p frontline-gateway --locked` 通过 36 项（12 单测、24 集成）；`cargo fmt --all --check` 通过。
- **本地门禁**：`CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh rust determinism` 退出 0，`pass=[rust determinism] fail=[] skipped=[]`。3 个种子各 18,000 tick 跨 native/WASM 哈希和位置完全一致。rebase 到 #167 后补测网关 36 项、`cargo test -p frontline-protocol --test m1_msgs --locked` 19 项、`cargo test -p frontline-server --lib c4:: --locked` 2 项通过。未使用 GitHub Actions。
- **本机联调**：真实 gateway 和 server，M0 空世界夹具。直接回投 61 tick；网关离线落盘、重启后重放 91 tick，重复提交未覆盖。详见 `reports/m2-gateway-stats.md`。
- **未做**：真实 OAuth、PKCE、游客升级和凭据审计未测（H-063），M2-022 保持 `partial`。正式环境、真人自然结算、参考机均未测。server 现有回投不填个人计分行。本批无 UI 或素材改动；战绩/设置页视觉待 Fable 设计。
