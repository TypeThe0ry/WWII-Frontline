# M1-065 素材接入：M1 音频与字体

## 元数据
- 里程碑：M1
- 工作流：assets
- 规模：M
- 前置依赖：M1-063, M1-004, M1-053
- 素材需求：A-WPN-*; C-BUL-*; D-TNK-001..004; E-GER-001; F-AMB-*; G-VOX-001; G-VOX-002; H-RAD-001; H-UI-002; J-IR-001; V-FNT-*；素材状态：real / placeholder（静音）混合
- 引用文档：ASSET_SHOPPING_LIST §4.2; ASSET_GAPS §3.5; ASSET_GAPS §3.3; ARCHITECTURE §12.7; ARCHITECTURE §11.4; ARCHITECTURE §11.8

## 背景
把已登记 `OK` 的音频与字体按需求 ID 打包进 `.fpak` 并接入客户端；未到位的保持静音/系统字体占位。D-AUD（ARCHITECTURE §12.7）：Opus 为主（WebM 容器数据放入 `.fpak`），AAC 兜底，客户端启动时用 `decodeAudioData` 实测探测；Opus 参数——枪声近距层 96 kbps、其它 48–64 kbps、环境床 32 kbps；首张地图音频总量 ≈ 18 MB（Medium），High 加入多样化变体至 ≈ 40 MB；短音（< 3 s）预解码常驻、长音流式。字体（UI_SPEC §2.3）：Barlow Semi Condensed / IBM Plex Sans / IBM Plex Mono / Noto Sans SC，自托管 WOFF2，子集化，中文按 unicode-range 分片；许可证由 M0-003 核实登记为 `OK` 才可入库（附录 B-1，H-036）。音频素材的"可闻距离/分层"等行为参数【待测】来自 REFERENCE_MEASUREMENTS §4.12，不是素材。已知 GAP 项（ASSET_GAPS §2.2）：A-WPN-005 BAR 全自动（GAP-A02）、C-BUL-004 人体命中（GAP-A04）、D-TNK 坦克炮塔声（GAP-A14）、A-WPN-013 MG34（GAP-A01，M2）等——缺则静音，不顶替（ASSET_GAPS §3.3.4）。

## 具体内容
### 要创建/修改的文件
- `assets/manifests/m1-audio.json`、`assets/manifests/m1-fonts.json`（进 git）—— 需求 ID → `.fpak` 条目，`placeholder` 字段，`codec: opus|aac`，`loudness_lufs`，`license_id`。
- `tools/asset-pipeline/recipes/m1-audio/*.json` —— 每个枪声的 near/mid/far 层与变体（每层 ≥ 3–4 变体）、响度归一目标（M1-004 约定值）、Opus 码率、AAC 兜底；机械声（拉栓/换弹）、脚步（E-FTS-001 各材质）、UI（H-UI-002）、环境床（F-AMB）、人声（G-VOX-001/002 德/美喊话）、无线电（H-RAD-001，可选）、IR（J-IR-001，M2 使用但可先打包）。
- `tools/asset-pipeline/recipes/m1-fonts/*.json` —— 字体子集化（Latin + CJK 常用字，词表自 i18n 抽取，M1-053 工具）、WOFF2、按 unicode-range 分片。
- `client/src/audio/bank.ts`（修改，仅映射表）—— 需求 ID → fpak 条目；`data/audio/weapons.json` 填入层级素材 ID。
- `client/src/ui/fonts/loader.ts`（修改，仅配置）—— 字体文件清单。
- `reports/m1-asset-status.md`（与 M1-064 共用报告，追加音频与字体节）。
### 要实现的行为
1. 逐需求 ID 核对 `LICENSES.md` 状态：非 `OK` 保持静音/系统字体，报告标 `blocked(license)`。
2. 音频打包：响度归一后编码 Opus（WebM）+ AAC 兜底，写入 `snd` 类 `.fpak`；探测样本与包类型选择（M1-062）端到端验证。
3. 字体：子集化后统计总量；zh-Hans 首屏分片 ≤ 400 KB（br）；字体按 UI_SPEC §2.3 自托管 WOFF2，随 core/静态资源下发（不进 `.fpak`；许可证允许再分发的前提须在 LICENSES.md 核实，登记署名）。
4. 署名：CC-BY/需署名素材的 `credit_text` 汇总进游戏内署名清单（LICENSES.md §4.3 的 `credits` 步骤）。
5. 校验：`LICENSES.md` §4.4 台账校验输出 PENDING/REJECTED 条数、缺失存档文件、manifest 与台账不一致项，贴入回报。
### 接口与数据
```json
{ "id": "A-WPN-001", "pak": "snd_wpn_us.fpak", "entries": { "near": ["a_wpn_001_n1","a_wpn_001_n2","a_wpn_001_n3"], "mid": [], "far": [] }, "codec": "opus", "placeholder": false, "license_id": "LIC-0045" }
```

## 不做什么
- 不做 M2 批音频（MG34/StG44 声、G-VOX-003/004、IR-002，M2-047）、音乐（I-MUS，可选）。
- 不做素材采购/书面确认（H-049、H-050）；不修改 LICENSES.md 状态。
- 不自制/录制/AI 生成音频；不剪辑成新的混合声（D-MOD 仅允许响度归一、格式转换、裁切静音等"处理"）。
- 不做 AudioWorklet 或混响（M2）。

## 验收标准
1. 每个状态为 `OK` 的 M1 音频/字体需求 ID 在 manifest 中 `placeholder: false` 且游戏内使用真资源（脚本 `asset-status` 的 real 计数 = LICENSES `OK` 计数；Playwright 触发事件时 `bank.get(id)` 返回非静音缓冲，峰值电平 > −60 dBFS）。
2. 音频包体积（Medium）≤ 18 MB（`manifest-budget`；INDEX 指标）。
3. 解码探测通过：Chromium 下 Opus 路径、强制 AAC 路径（mock 探测失败）各解码全部 M1 短音成功率 100%（Playwright `decodeAudioData` 测试）。
4. 往返一致：`.fpak` 打包后读出的音频条目与处理后源文件字节一致（sha256；沿用 M1-003 往返测试）。
5. 产物目录无独立音频文件：`find <upload_dir> \( -name '*.ogg' -o -name '*.opus' -o -name '*.wav' \)` 为空。
6. zh-Hans 首屏字体分片合计 ≤ 400 KB（br），Latin 字体合计 ≤ 200 KB（br）（脚本统计；不可达写 PLAN_ISSUES）；core 包总体仍 ≤ 8 MB（`core-size` 门禁）。
7. 缺失素材报告列出全部 M1 音频/字体需求 ID 的状态，且每个 `missing/placeholder` 项有 GAP 决定或"推迟到 Mx"（E1-17）。

## 测试要求
- 单元测试：配方解析；响度归一数值（目标 ±0.5 LU）；探测回退；署名汇总。
- 集成/无头测试：`pnpm asset:build m1-audio m1-fonts`；Playwright 触发枪声/脚步/UI 事件并检查缓冲。
- 性能基准：音频包加载与解码耗时（100 Mbps）入 `reports/m1-asset-status.md`。
- 回归：M1-062/063 的静音占位路径在缺素材时继续工作。

## 性能预算
- ARCHITECTURE §11.4：首张地图音频 ≈ 18 MB（Medium）、core ≤ 8 MB（含字体子集 ≈ 1.5 MB 估算）；§12.4 复音预算不变。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、CI `manifest-budget`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材（静音）已按 ASSET_GAPS §3.3 处理；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：LICENSES §4.4 校验输出贴入回报
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 素材未 OK 时本任务 `blocked`/`partial`，不影响其它任务；登记 blocked 清单并追加 NEEDS_HUMAN 条目（H-049、H-050、H-036 字体方案）。
- A Sound Effect 多厂牌、Sonniss WWII 库等的授权范围需书面确认（D-LIC）；未确认保持 PENDING。
- Opus 在 Safari 的支持未核实，AAC 兜底包使音频总量接近翻倍；M1-005 预算门禁以实际下发包（按探测选择）计量。
