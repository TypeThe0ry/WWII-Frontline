# M2-024 语音：LiveKit 自托管与 token

## 元数据
- 里程碑：M2
- 工作流：voice
- 规模：M
- 前置依赖：M1-066, M1-028
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §14.1; ARCHITECTURE §14.3; ARCHITECTURE §18.1; ARCHITECTURE §19.4; ARCHITECTURE §7.4; ARCHITECTURE §5.8

## 背景
语音方案（ARCH §5.8/§14）：自托管开源 SFU **LiveKit**；gateway 签发 token；对局服务器通过 LiveKit Server API 控制订阅；客户端 WebAudio HRTF 空间化。本任务是语音链路的基础设施：LiveKit 的容器化部署配置 + gateway 的 token 签发 + 一个最小互通验证页。订阅集管理（M2-025）与客户端接入（M2-026）依赖它。
房间/权限模型（ARCH §14.1/§14.3）：一个对局 = 一个 Room（`room = match_id`），每名真人玩家 = 一个 Participant（`identity = player_id`）；所有玩家发布 1 条麦克风轨（Opus，24 kbps，DTX 开启，`maxBitrate` 32 kbps）；**默认不自动订阅**（客户端以 `autoSubscribe=false` 连接，订阅集合由服务器决定）。token（§14.3）：客户端 `POST /voice/token {match_id}`（带 join ticket 或会话 cookie）；gateway 校验该玩家在该局，签发 LiveKit JWT：`room=match_id`、`identity=player_id`、`canPublish=true`、`canSubscribe=true`、`canPublishData=false`、**TTL 15 min（可刷新）**；对局服务器用独立服务账户 key 创建房间（`emptyTimeout`、`maxParticipants=128`）；玩家被封/踢出 → server 调 `RemoveParticipant`。**隐私（§19.4）**：语音不录制、不存档；LiveKit 关闭 egress/录制；服务器日志不含音频。部署（§18.1）：LiveKit 4 vCPU/8 GB，与对局同机房，容量按 M2 压测确认（未核实）。

## 具体内容
### 要创建/修改的文件
- `deploy/livekit/livekit.yaml` —— 单节点配置：API key/secret 来自环境变量、RTC 端口范围、TURN 配置、`room.empty_timeout`、`room.max_participants:128`；**不部署 egress/ingress 服务**，配置中不启用录制/出口
- `deploy/docker-compose.yml`（M1-066 的文件，增补）—— `livekit` 服务（镜像版本**固定**到一个具体已发布版本，写入 compose 与 `docs`/`LICENSES.md` 备注；版本号由执行者在任务中查证并记录，**不得用 `latest`**）
- `crates/gateway/src/voice.rs` —— `POST /voice/token`：校验 ticket/会话、查玩家在局（向 server 内部 API 或 ticket 声明）、签发 JWT（HS256，`iss=API key`，`sub=identity`，`video:{room,roomJoin,canPublish,canSubscribe,canPublishData:false}`，`exp=now+900`），限速沿用 gateway 每 IP/账号规则
- `crates/gateway/src/voice_refresh.rs` —— `POST /voice/token/refresh`（TTL 内续签，限频）
- `crates/server/src/voice/room.rs` —— 开局 `CreateRoom`（或 LiveKit 自动创建 + 首次订阅控制前确保存在）、局终 `DeleteRoom`、`RemoveParticipant`（kick/ban 钩子，对接 M2-023）
- `tools/voice-smoke/index.html` + `tools/voice-smoke/main.ts` —— 最小互通页（两浏览器标签用测试 token 互相听到）
### 要实现的行为
1. 本地一键：`docker compose up livekit gateway` 后 `curl` 取 token 并能连上。
2. token 校验：非本局玩家 403；缺 ticket 401；`TTL=900 s`；`canPublishData=false`；过期 token 被 LiveKit 拒绝连接。
3. API key/secret 仅 gateway 与对局服务器服务账户持有，**不进入客户端、不入库、不进日志**。
4. 房间生命周期：局开始建房（`maxParticipants=128`），局终/空房超时回收。
5. 日志与指标：`livekit_token_issued_total`、`livekit_token_denied_total`（gateway）。
### 接口与数据
- 响应：`{ url:"wss://…", token:"<jwt>", expires_in:900 }`；`Welcome.livekit_url/livekit_token`（ARCH §6.8.1）可选内嵌，默认走此 HTTP 接口。

## 不做什么
- 订阅集合计算与增量 API → M2-025；客户端 `livekit-client` 接入、PTT、空间化 → M2-026；语音 UI → M2-040。
- LiveKit Cloud 方案（NEEDS_HUMAN H-018 默认自托管）；转写（R10，首发不做）；录制/egress（明确关闭）。
- 容量结论（100 玩家下的 CPU/带宽）→ M2-025 的实测与 M2-049。

## 验收标准
1. 两个浏览器经 LiveKit 互通音频：`tools/voice-smoke` 两标签页，A 发测试音，B 的 `AnalyserNode` 在 ≤ 2 s 内检测到信号，RMS > −50 dBFS（人工/脚本记录；无浏览器环境时标 `未测` → `partial`）。
2. token 过期被拒：用 `TTL=5 s` 的测试配置，6 s 后连接尝试失败（LiveKit 返回鉴权错误），并在 TTL 内续签成功。
3. 签发规则单测：`cargo test -p gateway voice_token`：非局内玩家 403；`video.canPublishData=false`；`exp - iat = 900`；`room` 等于 `match_id`；`identity` 等于 `player_id`。
4. 配置审计：`rg -i 'egress|record' deploy/livekit` 仅出现在"已禁用"注释中；compose 中无 egress 服务（脚本断言）。
5. 密钥不泄漏：`docker compose config`/日志/`/metrics` 输出不含 API secret（脚本 grep）。
6. 镜像版本为固定 tag（非 `latest`），已记录到 STATUS 与 `LICENSES.md` 备注（LiveKit 许可证条款**未核实**，由人类登记，参见 H-018）。

## 测试要求
- 单元测试：JWT 构造/过期/声明；限速。
- 集成：compose 起栈后跑 smoke 脚本；server 创建/删除房间的集成测试（对真实 LiveKit 容器）。
- 性能基准：无（容量在 M2-025/M2-049）。
- 回归：M1-066 的 `ops-dev-stack` 一键启动仍可用。

## 性能预算
无新增预算；语音带宽估算（ARCH §18.1）：每局约 5.8 Mbps（100 人，每听者 ≤ 8 路 × 24 kbps × DTX 活跃率 0.3），M2-025 实测核对。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 人工项：LiveKit 主机/域名/TLS 与 TURN UDP 端口范围由人提供（NEEDS_HUMAN H-018、H-017）；本任务只做本地 compose 与可部署配置。缺主机则"生产部署"项不做，不影响本地验收。
- LiveKit 的 token 声明字段与 Server API 路径以所钉版本官方文档为准（本规划未联网核实）；执行时核对并把差异写 PLAN_ISSUES。
- 自托管 TURN/UDP 端口是已知运维风险（ARCH §5.8）。
