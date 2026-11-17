# M1-066 运维：本地开发栈

## 元数据
- 里程碑：M1
- 工作流：ops
- 规模：S
- 前置依赖：M1-029
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §3.2; ARCHITECTURE §18.1; ARCHITECTURE §18.2; ARCHITECTURE §5.5

## 背景
M1 起需要一键启动"gateway + 对局 server + 静态资源服务"，让开发者与 headless-client 在本机复现完整链路，也是 M1-067 预发布部署的镜像基础。ARCHITECTURE §3.2：`gateway`（443/TCP HTTPS，无状态+小库）、`server`（443/UDP QUIC + 443/TCP WS 兜底，每局一个端口）、静态资源（生产为 R2+CDN，开发为本地静态服务）。§18.2：WebTransport 要求受信任证书；`serverCertificateHashes`（自签名 ≤ 14 天）**仅用于本地开发**，各浏览器支持未核实；证书续期由主机上的 `certbot/lego` 完成，`server` 监听 SIGHUP 重载（生产，M1-067）。M0-017 已提供开发自签证书脚本；本任务将其接入 compose。M1 的 compose 不含 LiveKit/监控（M2-024、M2-048）。

## 具体内容
### 要创建/修改的文件
- `deploy/dev/docker-compose.yml` —— 服务：`gateway`、`server`（`--map carentan --mode frontline --match-id dev`）、`static`（nginx 或 `vite preview` 提供客户端构建与 `.fpak`，支持 HTTP Range）。
- `deploy/dev/Dockerfile.server`、`Dockerfile.gateway` —— 多阶段构建（Rust release，静态链接优先），非 root 用户，只读根文件系统（可行时）。
- `deploy/dev/certs/gen-dev-cert.sh`（复用/扩展 M0-017）—— 生成 ≤ 14 天自签证书并输出 SHA-256 指纹；`deploy/dev/.env.example`（端口、数据路径、gateway 密钥生成说明；**不含任何真实密钥**）。
- `deploy/dev/README.md` —— 一键启动步骤：`./dev.sh up`、浏览器如何信任/使用证书哈希、如何跑 headless-client 连入。
- `dev.sh` —— `up/down/logs/reset`；启动前检查 Docker 与端口占用。
- `client` 开发配置：`client/.env.development` 的 gateway 地址与 `serverCertificateHashes` 注入机制（仅开发构建）。
### 要实现的行为
1. `docker compose up`（或 `./dev.sh up`）后：gateway 健康检查通过；server 向 gateway 注册并保持心跳；静态服务可下载客户端与 `.fpak`（Range 请求返回 206）。
2. 浏览器（Chromium）经 WebTransport 进入对局：客户端取 join ticket → 连 server → 收 `Welcome` → 进入部署界面；WT 失败时自动 WS 兜底。
3. 开发证书过期前（≤ 14 天）脚本自动重新生成并提示重启；指纹经 gateway 的开发端点下发给客户端（仅开发配置）。
4. `docker compose down -v` 清理 SQLite 卷；`reset` 一键回到初始状态。
5. 镜像不含 `assets/incoming`；`/data` 与地图碰撞/导航数据以只读卷挂载。
### 接口与数据
```yaml
services:
  gateway: { build: ..., ports: ["8443:8443"], environment: [GATEWAY_SIGNING_KEY_FILE, GATEWAY_DB=/var/lib/gateway/gw.sqlite] }
  server:  { build: ..., ports: ["4433:4433/udp","4434:4434/tcp"], depends_on: [gateway], volumes: ["../../data:/data:ro"] }
  static:  { image: nginx, ports: ["8080:80"], volumes: ["../../client/dist:/usr/share/nginx/html:ro"] }
```
（端口以 M0-017/M1-029 实现为准；示意而非定稿。）

## 不做什么
- 不做预发布/生产部署、真实证书与域名（M1-067、M4-033）。
- 不做 LiveKit、Prometheus/Grafana/Loki（M2-024、M2-048），也不做多局调度/gateway 编排模块（首发由 gateway 简化启动，见 ARCHITECTURE §3.3，后续任务）。
- 不做 CI 中的 compose 作业（可选，若做仅 smoke）。
- 不在镜像或 compose 中放置密钥/ API key。

## 验收标准
1. `docker compose up` 后 ≤ 60 s（镜像已构建、冷启动）三服务全部健康：`curl -fsS https://localhost:8443/healthz` 返回 200；`curl -r 0-1023 http://localhost:8080/<某 .fpak>` 返回 206 与 1024 字节。
2. 浏览器可经 WT 进入对局：Playwright Chromium 脚本从主菜单→加入→阵营→进入部署界面，`Welcome` 到达且传输类型为 `wt`（遥测读取）；INDEX 指标。
3. 强制 WT 失败（阻断 UDP 端口）时自动回退 WS 并进入同一对局（Playwright，传输类型为 `ws`）。
4. server 向 gateway 注册成功：gateway 服务器列表在 ≤ 5 s 内出现该局；`docker stop server` 后 ≤ 10 s 该局从列表消失或标记失效（M1-029 的 5 s 心跳规则）。
5. 重复 `reset` 三次后仍可正常启动（无残留端口/卷冲突）。
6. 镜像体积：server ≤ 150 MB、gateway ≤ 100 MB（`docker image ls`；超出写入 STATUS，不阻断）。
7. 仓库内 `git grep -iE 'secret|api[_-]?key|BEGIN PRIVATE'` 在 `deploy/dev` 下无真实密钥（CI 步骤，仅允许示例占位）。

## 测试要求
- 单元测试：`dev.sh` 参数解析（shellcheck 通过）。
- 集成/无头测试：`./dev.sh up` + Playwright 进入对局 + headless-client 单客户端 60 s。
- 性能基准：无。
- 回归：M0-017 的 WT 连接测试在容器内网络下保持通过。

## 性能预算
- 无新增运行期预算；容器内 server 的 tick 指标沿用 ARCHITECTURE §6.10（稳态 ≤ 18 ms）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`shellcheck`、Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：端口与环境变量写入 `deploy/dev/README.md`；偏差记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Docker Desktop 对 UDP/QUIC 端口映射在部分平台（macOS/Windows）不稳定；如不可用，说明在 README 并提供原生 `cargo run` 替代路径。
- `serverCertificateHashes` 在各浏览器的支持未核实（ARCHITECTURE §18.2）；Chromium 为首要验证对象，Firefox/WebKit 走 WS。
