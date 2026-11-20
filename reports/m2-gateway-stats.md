# M2-022 网关战绩、游客设置与 OAuth 预留

2026-11-20，PI-109 工作站（非参考机）。

- SQLite 0002 保留原 `results` 行，新增可空的 `received_at_ms` 和以游客 ID 为键的 `player_settings` 表。旧结果接收时间为 `null`，从首次提交起记录接收时间。
- `GET /v1/matches/{match_id}/result` 使用游客会话读取已结算对局。`GET/PUT /v1/me/settings` 支持个人设置读取与整对象替换，请求上限 16 KiB。
- `GET /v1/oauth/{provider}/start` 与 `/callback` 默认返回 `404`。`OAUTH_ENABLED=1` 下 Discord/Google 返回 `501`，无需提供第三方凭据。

## 本机进程验证

用 `frontline-gateway` 临时 SQLite 文件和票据密钥启动在 loopback 随机端口。`frontline-server --empty-world --map m0 --mode sandbox --ws-port 0 --wt-port 0 --metrics-port 0` 通过现有 `GATEWAY_URL` 注册，并用 `--duration 2` 自行退出。退出前查询结算返回 `404`，退出后返回 `200`，服务器列表摘除该局。结算和服务器 JSON 报告均为 **61 tick**。

第二局用 `--duration 3`。注册后停止本测试启动的网关进程，服务端退出时写出 spool。使用相同数据库和票据密钥重启网关后，复查第一局结算及游客设置。再启动第三个 server 触发现有 spool 重放。第二局以原 payload 入库（**91 tick**），spool 文件移除。使用同一 ID 重投其他 payload 返回 `204`，查询仍返回首笔 payload 和接收时间。

## 自动检查

`cargo test -p frontline-gateway --locked` 通过 36 项，覆盖旧库迁移、重启后读取、结算幂等、设置隔离和 OAuth 开关。`CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh rust determinism` 两组均通过，3×18,000 tick 的 native/WASM 哈希与位置完全一致。rebase 到 #167 后，网关、协议 `m1_msgs` 和 server C4 定向复测通过。

## 余项

当前 server 的结算在进程停服流程中触发，计分行为空。真实 Discord/Google OAuth、PKCE、游客升级和第三方凭据审计未测（H-063）。正式环境、参考机和真人自然结算未测。M2-022 为 `partial`。
