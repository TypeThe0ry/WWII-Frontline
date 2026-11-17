# M0-028 测试：浏览器 WT/WS 互通矩阵

## 元数据
- 里程碑：M0
- 工作流：test
- 规模：S
- 前置依赖：M0-021
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §5.0; ARCHITECTURE §20.1; ARCHITECTURE §5.5; ARCHITECTURE §6.6

## 背景
ARCHITECTURE §20.1-1 要求验证"`wtransport` datagram 与 Chrome/Firefox/Safari 互通"；§5.0 的浏览器支持表来自二手来源，标"未核实"。本任务用 Playwright 在 Chromium / Firefox / WebKit 上实测 WT 与 WS 连接，**如实登记**支持矩阵与失败原因（E0-7）：Chromium 的 WT 与 WS 必须通过；Firefox/WebKit 结果不预设。WT 被禁用/不可用时须在 ≤ 5 s 内回退 WS（M0-021 已实现，此处验证）。注意：开发期自签名证书依赖 `serverCertificateHashes`（ARCHITECTURE §5.5/§18.2：有效期 ≤ 14 天，各浏览器支持**未核实**）；若某浏览器不支持该选项，则需要受信任证书（NEEDS_HUMAN H-017）才能测 WT，此时该格记"无法测试（缺受信任证书）"，不记为失败。

## 具体内容
### 要创建/修改的文件
- `client/e2e/interop/{interop.spec.ts,fixtures.ts,server.ts}`（`server.ts` 负责启动/停止 `frontline-server --map m0 --mode sandbox --bots 0 --dev-cert` 子进程并读取证书指纹）。
- `client/src/net/interop-probe.ts` + `client/interop.html`（探针页：使用 M0-021 的 `connect()`；运行 10 s：握手 → 发 10 s 的 `InputBundle` → 统计收到的快照数与 RTT；结果写入 `window.__interop`）。
- `scripts/run-interop.sh`；`reports/m0/browser-matrix.json`（运行产物）；`.github/workflows/interop.yml`（`workflow_dispatch` + nightly，不进 PR 必跑集合）。
### 要实现的行为
1. 每个浏览器（`chromium`、`firefox`、`webkit`，Playwright 提供的构建）× 每种传输（`wt`、`ws`）跑一次探针：记录 `{browser, version, transport, supported(WebTransport存在), connected, serverCertificateHashes可用, snapshotsReceived, rttMsMedian, error}`。
2. 判定"通过"：握手成功 + 10 s 内收到快照 ≥ 150 个（20 Hz 理想 200，容忍 25%）+ 0 次断线。
3. **回退测试**（每个浏览器）：`addInitScript` 删除 `window.WebTransport` 后 `?transport=auto` 连接，断言 `transport === 'ws'` 且从开始到握手完成 ≤ 5 000 ms。
4. **WT 被拒测试**（Chromium）：服务器不监听 WT 端口、仅开 WS，`auto` 模式 ≤ 5 s 回退成功。
5. 汇总：生成 Markdown 表（行 = 浏览器，列 = WT / WS / 回退 / 备注），`reports/m0/browser-matrix.json` 保存原始数据；表同时复制进 `tasks/STATUS.md` 的 M0-028 实测结果。
6. 失败原因按原文记录（异常消息、`WebTransportError` 的 `source`/`streamErrorCode` 等），不做猜测性归因；Safari 不在 Playwright 范围，用 WebKit 构建**近似**并明确标注"WebKit ≠ Safari 正式版"。
### 接口与数据
- `window.__interop = {browser?, transport, connected, snapshots, rttMs[], error?}`；探针页复用 `net`，不引入新协议。

## 不做什么
- 不做真实 Safari/Edge 实机测试与四浏览器完整矩阵（M4-042）；不做受限网络（UDP 被阻断）测试台（NEEDS_HUMAN H-022，M1+）。
- 不做 WebGPU/WebGL2 渲染矩阵（M4-042）。
- 不修复浏览器兼容问题（只登记；严重项写 PLAN_ISSUES，进入后续任务）。

## 验收标准
1. **Chromium：WT 与 WS 均通过**（握手成功，10 s 内快照 ≥ 150，RTT 中位 < 20 ms loopback，0 断线）。
2. Firefox 与 WebKit 的 WT/WS 各有一条**如实记录**（通过/失败 + 原文原因，或"无法测试"+ 理由）；矩阵无空格。
3. 回退测试：三个浏览器上 `delete WebTransport` 后均回退 WS，**耗时 ≤ 5 000 ms**（记录各自毫秒数）。
4. Chromium 的"WT 端口关闭"用例回退成功且耗时 ≤ 5 000 ms。
5. `reports/m0/browser-matrix.json` 与 STATUS 中的表一致（脚本比对）；浏览器版本号取自 Playwright `browser.version()`。
6. 对 ARCHITECTURE §5.0 的"未核实"条目，在 PLAN_ISSUES 写一条汇总：哪些被本次实测证实/推翻（只写实测事实，不写猜测）。

## 测试要求
- 单元测试：`fixtures.ts` 的矩阵汇总函数（vitest）。
- 集成/无头测试：本任务本身即集成测试；`run-interop.sh` 在干净环境可一键运行（需 `npx playwright install` 浏览器）。
- 性能基准：无。
- 回归：把矩阵 JSON 作为 M4-042 的起点。

## 性能预算
无（功能性测试）；回退 ≤ 5 s 是 E0-7 的硬指标（MILESTONES §1.3）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Playwright 的 headless Chromium 对 WebTransport 的支持与参数（如是否需要额外启动参数）**未核实**；以安装版本的文档与实测为准，所需参数写入脚本并记录。
- 自签名证书的 14 天有效期会让夜间作业过期：脚本每次运行前重新生成证书（M0-017 的 `gen-dev-cert`）。
- WebKit 在 Linux 上的 WT 支持可能缺失；这是记录对象而不是缺陷。
