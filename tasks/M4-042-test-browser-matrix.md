# M4-042 测试：浏览器兼容矩阵

## 元数据
- 里程碑：M4
- 工作流：test
- 规模：M
- 前置依赖：M0-028, M4-020, M4-017
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §5.0; ARCHITECTURE §20; ARCHITECTURE §12.7; ARCHITECTURE §14.5; ARCHITECTURE §6.6

## 背景
ARCHITECTURE §5.0（核实日期 2026-11-17，多数为二手来源）：WebGPU：Chrome 113+/Edge；Firefox 141 起 Windows 启用（macOS Apple Silicon 在 145–147 之间，来源矛盾，**未核实**；Linux/Android 在推进）；Safari 26 起。WebTransport：Chrome/Edge 2021 末–2022 初起；Firefox 114+；Safari 26.4（2026-03）起，MDN 标 "Baseline Newly available"。覆盖率百分比**未核实**。结论：WebGL2 兜底不可省；WS 兜底必须作为一等路径测试；语音只用基础 Opus，不依赖 Insertable Streams/SVC/AV1。
§20 第 6 项限制：**CI 仅保证 Chromium**；Firefox/Safari 体验为"尽力"，问题优先级低于 Chromium。
音频编码探测（D-AUD，§12.7）：启动时用 `decodeAudioData` 对内置探测样本实测，能解 Opus 则用 Opus 包，不通过则改取 AAC 包。
本任务产出**实测矩阵**：浏览器（Chrome、Edge、Firefox、Safari）× 平台（Windows、macOS、Linux）× 能力项：
- 渲染：WebGPU 可用/不可用 → WebGL2 回退；compute 路径（Hi-Z、froxel、TAAU）功能；WebGL2 路径功能；四档渲染是否可用。
- 网络：WebTransport（datagram+stream）/WS 兜底；证书/CSP 行为；重连。
- 音频：AudioContext、HRTF PannerNode、AudioWorklet、`decodeAudioData` 对 Opus/AAC。
- 语音：LiveKit 加入、麦克风权限、PTT、Opus。
- 输入：Pointer Lock、`unadjustedMovement`（UI_SPEC §4.16 标注未核实）、键位保留/冲突（UI_SPEC §5.5）。
- 存储：Cache API/OPFS/`localStorage` 可用性与配额。
- UI：基础布局、字体子集、i18n。
**需要多种实机/虚拟机**（H-021 含 macOS/Safari 与 Firefox）；缺则对应格标 `未测`。

## 具体内容
### 要创建/修改的文件
- `tests/compat/probe.html` + `client/src/diagnostics/CapabilityProbe.ts` —— 一键探测页：输出 JSON（WebGPU adapter/limits、WebGL2 扩展、WT/WS、音频解码、AudioWorklet、Pointer Lock、OPFS/Cache 等）。
- `tests/compat/run_matrix.ts` —— 在可用浏览器上（Playwright 的 Chromium/Firefox/WebKit 与手动实机）运行功能清单，产出 `reports/compat/{browser}-{os}.json`。
- `tests/compat/features.json` —— 功能清单（每项含 `id`、`area`、`priority`、`pass_criteria`）。
- `reports/compat/m4-matrix.md` —— 汇总表（Chromium/Edge 全功能通过；Firefox/Safari 问题分级）。
- `tests/compat/README.md` —— 人类手动测试步骤。
### 要实现的行为
1. 自动化（Playwright）：Chromium/Firefox/WebKit 各跑功能清单（软件渲染下 WebGPU 可能不可用，以 WebGL2 路径为主）；记录通过/失败/不适用。
2. 手动（人类/H-021）：Edge、Safari（macOS）、Firefox（Windows/macOS/Linux）实机跑探测页与对局冒烟（连接 WT/WS、进入对局 60 s、语音加入、画质切换）。
3. 问题分级：`blocker`（Chromium/Edge 内的失败；或任一浏览器无法进入对局）、`major`、`minor`、`wontfix-first-release`（Firefox/Safari 的非关键差异）；每个问题登记 PLAN_ISSUES 并附优先级与浏览器版本。
4. 回退正确性：禁用 WebGPU（命令行/策略）→ 自动走 WebGL2 并提示（UI_SPEC §4.1.5）；禁用 WT → 自动 WS；Opus 不支持 → AAC。
5. 矩阵入 STATUS：每格 `ok / degraded / fail / 未测`，附浏览器版本号与测试日期。
6. 不向外部上传指纹；探测页仅本地输出，复制/下载 JSON 由用户操作。
### 接口与数据
- `CapabilityProbe` 输出：`{"webgpu":{"available":bool,"features":[],"limits":{}},"webgl2":{...},"webtransport":bool,"audio":{"opus":bool,"aac":bool,"worklet":bool},"input":{"pointerlock":bool,"unadjusted":bool},"storage":{"opfs":bool,"cache":bool,"quota_mb":n}}`。

## 不做什么
- 不修复 Firefox/Safari 的非阻断问题（登记优先级，后续排期）。
- 不做移动浏览器（A4）。
- 不做性能测量（M4-041）；不做安全头/CSP 审计（M4-032，仅记录与浏览器交互的差异）。

## 验收标准
1. **Chromium/Edge 全功能通过**：features.json 中全部 `priority=P0/P1` 项在最新稳定版 Chrome 与 Edge 的桌面平台通过率 100%（失败项为 0；不适用项须注明）。
2. **矩阵入 STATUS**：Chrome/Edge/Firefox/Safari × Windows/macOS/Linux 的 12 格，每格有结果或 `未测`，且含版本号与日期。
3. Firefox/Safari 问题已分级登记：每个失败/降级项都有 PLAN_ISSUES 编号、严重级与优先级（脚本检查无"无编号失败项"）。
4. 回退路径：强制禁用 WebGPU / WebTransport / Opus 三个开关，对应回退成功率各 100%（Playwright + 手动）。
5. 音频探测：`decodeAudioData` Opus/AAC 探测结果与实际播放一致（至少 Chromium 与一个非 Chromium 实机）。
6. 诚实声明：`m4-matrix.md` 明确标注覆盖率数字**未核实**，且 Firefox/Safari 为"尽力支持"（与 §20 一致）。

## 测试要求
- 单元测试：`vitest`：`CapabilityProbe` 在 mock 环境下的各分支。
- 集成/无头测试：Playwright 三引擎功能清单；WebKit 非真 Safari，需标明"WebKit≠Safari"并以实机为准。
- 性能基准：无。
- 回归：Chromium 清单纳入 PR 作业（可选，时长受控）。

## 性能预算
无新增预算；兼容性清单不得使 §17.5 的 `test-ts`/`e2e` 作业超时。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 依赖 H-021（测试设备矩阵）；macOS/Safari 与 Firefox 实机缺失 → 对应格 `未测`，任务 `partial`。
- 浏览器版本变化快，矩阵需要标注测试日期；§5.0 的版本号来自二手来源，须以实测为准。
- WebTransport 在各浏览器的证书/`serverCertificateHashes` 行为**未核实**；生产证书为公共 CA，不依赖该特性。
