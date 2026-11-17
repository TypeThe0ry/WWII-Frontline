# M3-050 运维：夜间作业

## 元数据
- 里程碑：M3
- 工作流：ops
- 规模：S
- 前置依赖：M2-049, M2-051, M2-050
- 素材需求：无；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.5; ARCHITECTURE §17.2; ARCHITECTURE §17.3; ARCHITECTURE §17.4; MILESTONES §4.3 (E3-13)

## 背景
M0-002 建立了 PR 级 CI（`lint`/`test-rust`/`test-ts`/`wasm-build`/`determinism`/`data-validate`/`manifest-budget`）。ARCH §17.5 规定三类夜间/手动作业：
| Job | 触发 | 内容 | 目标时长 |
|---|---|---|---|
| `e2e-visual` | PR（改 client/render）与 nightly | Playwright 视觉回归 | < 20 min |
| `loadtest` | nightly | 100 headless 客户端 15 分钟 | < 30 min |
| `balance` | nightly/手动 | 200 局缩比对局 balance-sim（M2-051 口径） | ≤ 4 h |
M2-049/050/051 已有各自的脚本，但尚未按夜间调度运行。退出标准 E3-13：loadtest/balance/e2e-visual 连续 3 晚成功。本任务把它们接入定时作业，并提供报告归档与失败通知；**本任务验收只要求"夜间作业配置完成且首次运行成功"，连续 3 晚稳定性在 M3-054 验收时复核**（总控裁定）。

## 具体内容
### 要创建/修改的文件
- `.github/workflows/nightly.yml` —— `schedule: cron`（UTC 02:17 之类避开整点）+ `workflow_dispatch`；三个 job 并行：`loadtest`、`balance`、`e2e-visual`；每个 job 设置 `timeout-minutes`（loadtest 40、balance 55、e2e-visual 30，均为目标时长的约 1.3–1.4 倍）。
- `.github/workflows/nightly-notify.yml`（或在 nightly.yml 末尾的 job）—— 当任一 job 失败时，用 GitHub 原生机制创建/更新带 `nightly-failure` 标签的 issue（`actions/github-script` 或 `gh issue`），附运行链接与失败摘要；同一失败不重复开 issue（以标题 + 最近 24 h 去重）。
- `tools/ci/archive_reports.sh` —— 汇总 `reports/`（loadtest JSON、balance CSV、visual diff）为 `nightly-{date}.zip`，上传 artifact，保留期 30 天【待调参】。
- `tools/ci/check_nightly_history.sh` —— 用 `gh run list --workflow nightly.yml --limit 10 --json conclusion,createdAt` 判定"最近连续 N 晚成功"。
- `docs/` 不改；运行说明写入 `tools/ci/README.md`（新建，限 CI 目录）。
### 要实现的行为
1. loadtest：M2-049 的 100 headless 客户端 15 分钟脚本（WT 与 WS 各一次太长，夜间默认 WT；WS 每周日），指标判定沿用 M2-049（tick P99 ≤ 28 ms、下行 P99 ≤ 400 kbps），失败退出非零。
2. balance：`balance-sim --games 200 --mode frontline`，夜间轮换 mode（奇数日 Frontline，偶数日 Assault），越界项写入 job summary，不因越界失败，只在崩溃/卡死/确定性失败时红（越界是"报告"，由人工审阅）。
3. e2e-visual：全量视觉回归（M2 + M3 机位），失败上传 diff。
4. 所有 job 失败 → 开/更新 `nightly-failure` issue；全部成功 → 自动关闭该 issue。
5. 报告可下载：artifact 名 `nightly-{run_id}`；README 说明如何下载。
6. 自托管 GPU runner 为可选项（label `gpu`）；无则 e2e-visual 走软件渲染基线（ARCH §17.4）。
### 接口与数据
- workflow 输出 job summary 表：`job, duration, result, key metrics`。

## 不做什么
- 优化测试本身的耗时/稳定性（若超时写 PLAN_ISSUES）。
- 外部告警服务（PagerDuty 等）与生产监控（M4 运维）。
- 修改 PR 级 CI 作业。

## 验收标准
1. `workflow_dispatch` 手动触发一次，三个 job 全部成功且各自在目标时长内：loadtest < 30 min、balance ≤ 4 h、e2e-visual < 20 min（Actions 日志记录）。
2. **首次定时（schedule）运行**三作业全部成功，且各自报告 artifact 可下载（`tools/ci/check_nightly_history.sh 1` 输出 OK）。连续 3 个定时夜晚的稳定性（`check_nightly_history.sh 3`）**不是本任务验收项**，在 M3-054 验收时复核；不得用 `workflow_dispatch` 次数顶替定时运行。
3. 故意让 loadtest 失败一次（例如临时降低阈值的分支触发），`nightly-failure` issue 在 10 min 内被创建，且包含运行链接；再次成功后 issue 自动关闭。
4. 报告 zip 内含 loadtest JSON、balance CSV、visual diff 目录（`unzip -l` 检查）。
5. 工作流使用最小权限（`permissions:` 明确列出，仅 `contents: read`、`issues: write`、`actions: read`），无硬编码密钥（`grep -ri 'secret\|token' .github/workflows` 仅有 `secrets.GITHUB_TOKEN` 引用）。
6. 作业不阻塞 PR：`nightly.yml` 不含 `pull_request` 触发。

## 测试要求
- 单元：`check_nightly_history.sh` 对样例 JSON 的输出（bats 或 shell 测试）。
- 集成：`act`（本地）或实际 Actions dry-run 验证 YAML；手动触发一次全流程。
- 性能基准：记录三个 job 的实际时长。
- 回归：PR 级作业时长不变（< 15 min）。

## 性能预算
- ARCH §17.5 各作业目标时长；GitHub 托管 runner 资源有限（约 4 vCPU），100 客户端压测可能超限——若是，使用自托管/租用 runner（H-067）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 验收 2 受日历时间限制：本任务最早在作业上线后第 3 个夜晚才能 `done`；其余条目完成时应立即提交并把状态置 `partial`，后续会话补记。
- 夜间 loadtest 15 min（ARCH §17.5）与 M3-045 的 30 min 不同，二者独立：前者是回归监控，后者是里程碑验收。
- 本仓库若为私有，Actions 分钟数配额需确认（H-020 月度预算）。
