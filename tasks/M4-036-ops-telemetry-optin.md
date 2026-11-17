# M4-036 运维：匿名遥测（首次同意）

## 元数据
- 里程碑：M4
- 工作流：ops
- 规模：S
- 依赖：M4-035, M4-017
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §18.3; ARCHITECTURE §19.4; ARCHITECTURE §13.6; UI_SPEC §4.16; NEEDS_HUMAN H-082

## 背景
ARCHITECTURE §18.3 末行：客户端遥测（匿名、可选）上报**帧时间分布、画质档、传输类型（wt/ws）、着色器编译耗时**；§19.4：客户端遥测匿名、**默认关闭或首次明确同意**；**不含麦克风/IP 信息；不采集 GPU 以外的硬件指纹**；§13.6：能力探测结果仅用于推荐，不上传（本任务的遥测是**另一条受同意控制的通道**，GPU 名称可作为字符串类别上报，不得上报 `deviceMemory`/`hardwareConcurrency` 的精确值——以"粗分桶"形式，见下）。MILESTONES 风险：封闭测试用遥测补充缺硬件的性能数据（需同意）。
隐私设置页入口在 M4-035 已有；设置项"允许收集诊断信息"（UI_SPEC §4.16.1 隐私）对应本任务开关。
**遥测默认关闭**；首次同意弹窗只出现一次，选项"同意/不同意/稍后"，"不同意"等同关闭且不再追问（可在设置里改）。文案须经 H-082 法务审阅（模板由本任务出）。

## 具体内容
### 要创建/修改的文件
- `client/src/telemetry/{TelemetryClient.ts,Schema.ts,Consent.ts,Collector.ts}` —— 同意状态、采集器（帧时间直方图、画质档、传输类型、着色器编译耗时分桶）、批量发送（≤ 1 次/60 s，≤ 2 KB/包）。
- `client/src/ui/privacy/TelemetryConsent.tsx`、i18n `privacy.telemetry.*`（en/zh）。
- `crates/gateway/src/telemetry/{ingest.rs,schema.rs}` —— 接收端：严格 schema 校验、拒绝未知字段、**不记录 IP**（反代层亦关闭该路径的访问日志 IP 字段）、聚合写入（计数/直方图）。
- `data/telemetry/schema.json` —— 允许的字段与分桶定义（版本化）。
- `tests/e2e/telemetry_consent.spec.ts`、`crates/gateway/tests/telemetry.rs`。
### 要实现的行为
1. 默认状态 `unset`：不采集、不发送、不创建标识符。同意前不发起任何遥测网络请求。
2. 同意后才采集：帧时间分布（桶：≤8.3/16.7/25/33.3/50/>50 ms，计数）、画质档、传输类型（`wt|ws`）、着色器编译耗时分桶、渲染后端（`webgpu|webgl2`）、浏览器大类（`chromium|firefox|safari|other`）、GPU 类别（`integrated|discrete|unknown`，**不上传具体型号字符串**——除非 H-082 批准；默认不）、内存分桶（`<=8GB|>8GB`）。
3. 不含：IP、麦克风/设备列表、精确硬件指纹、用户昵称/账号 id、聊天、位置轨迹。会话标识为每次页面加载随机生成、**不持久化**，不可关联跨会话。
4. 撤回：设置内关闭立即停止并清除本地缓冲；"删除我的遥测"不适用（匿名聚合，说明文案诚实告知无法逐人删除）。
5. 服务端：schema 外字段 → 400 并丢弃；速率限制每 IP 6 次/min（仅限流用内存计数，不落盘）；聚合表仅保存计数。
6. 与 M4-018 推荐不同：基准结果不自动上传；仅在同意后以分桶形式包含推荐档位与实际档位。
### 接口与数据
- `POST /api/telemetry`：`{v:1, sid:"<rand per load>", backend, tier, transport, ftime_hist:[6 ints], shader_ms_hist:[5 ints], browser, gpu_class, mem_bucket, dur_s}`；响应 204；`data/telemetry/schema.json` 为唯一真源，客户端与服务端同源校验。

## 不做什么
- 不做崩溃堆栈/回放上传；不做第三方分析 SDK（零第三方脚本，§19.5）。
- 不做 A/B 实验。
- 不做遥测可视化看板（可用 Grafana 自行查询聚合表，不在本任务范围）。
- 不做法律文本定稿（H-082）。

## 验收标准
1. **未同意时零上报（抓包验证）**：Playwright 拦截全部网络请求，从加载到进入对局 5 min，在 `unset` 与 `declined` 状态下对 `/api/telemetry` 请求数 = 0；同时无任何新增 `localStorage` 追踪标识。
2. 上报不含 IP/麦克风/硬件指纹：对同意后捕获的 20 个包做字段审计，仅含 schema 允许字段；服务端日志样本无 IP（grep 断言）；包大小 ≤ 2 KB。
3. 同意流程：首次进入出现一次同意弹窗；"同意/不同意/稍后"三态持久化（localStorage try/catch）；"稍后"下次会话再次出现且最多 3 次（单测）。
4. 撤回即停：关闭开关后 ≤ 1 s 无新请求（e2e），本地缓冲清空。
5. 服务端校验：含未知字段/超长/负数的包被拒（4 个负向单测）；聚合表无个人标识列（schema 断言）。
6. 采集开销：同意后采集 CPU ≤ 0.05 ms/帧；发送不在渲染关键路径。

## 测试要求
- 单元测试：`vitest`：同意状态机、分桶函数、schema 校验；`cargo test -p gateway telemetry`。
- 集成/无头测试：Playwright 抓包（`page.on('request')`）三状态；与 M4-035 隐私页联动。
- 性能基准：采集器开销。
- 回归：schema 版本化，变更须升 `v` 并更新隐私文案。

## 性能预算
- 不得使 §13.5.1 的"UI/GC/杂项"预算回归；发送包 ≤ 2 KB/60 s。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright / `cargo test`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖 H-082（隐私政策/同意文案法务审阅）；未审阅前遥测保持默认关闭、不弹窗（功能开关 `telemetry.enabled=false` 随构建配置），STATUS 标注 `partial`。
- 即使"匿名"，在 GDPR 下是否构成个人数据取决于法务判断（**未核实**），故保守处理为需同意。
- GPU 型号字符串是否可上报 INDEX/ARCH 未明确（§19.4 "不采集 GPU 以外的硬件指纹"暗示 GPU 可采集）：本任务保守只上报类别，写 PLAN_ISSUES 请 Fable 裁决。
