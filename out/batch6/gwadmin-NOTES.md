# B6-gwadmin · M2-023

- 网关实现独立管理员令牌与可信来源 CIDR、事务审计、登录与签票封禁、公告、服务器发现控制、人工举报复核、按稳定身份去重/限速，以及内网管理命令轮询和回执。新增迁移 `0023_admin.sql`。
- 比赛服联通 C0 举报、输入与事件片段、文字聊天、踢出、禁言和广播。管理 UI 待 Fable 设计。
- 本机（PI-109，非参考机）`cargo test --release --locked -p frontline-gateway --test admin -- --include-ignored --nocapture --test-threads=1` 通过 10/10。拒绝请求各 1,000 次，P95 67.629/68.600 µs。SQLite 1,000 条举报、100 次分页请求，P95 751.114 µs。`local-ci.sh determinism` 严格对照 3×18,000 tick 一致。`local-ci.sh data` 75 测试通过，扫描 78 个文件，0 错误。
- 在线强制换队、在线换图、举报后聊天证据、参考机与生产环境待验收。H-065/H-075 跟踪线上配置和人工处置。没有新增依赖和素材。
- `FRONTLINE_TEST_GATEWAY_BIN=$PWD/target/release/frontline-gateway cargo test --locked -p frontline-server --test gateway_link -- --ignored --nocapture real_gateway_process` 通过 2/2。实服测试踢出 189.5 ms、回放 270 B。200 并发签票五轮 1,000/1,000 唯一票据，最慢轮 P95 21.923 ms。
- 保留并行 `gwstats` 已落地的设置/OAuth 占位、结算接收时间和迁移 `0002_stats_and_settings.sql`。
- 本地门禁 `scripts/local-ci.sh lint`、`rust`、`data`、`determinism` 分组均通过。Rust 包含工作区及额外的 sim `test-util` 回归。
