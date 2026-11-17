# M3-043 素材接入：M3 音频

## 元数据
- 里程碑：M3
- 工作流：assets
- 规模：M
- 前置依赖：M2-047, M3-037, M3-038, M3-039
- 素材需求：A-WPN-*; B-*; D-*; E-ACT-001; F-AMB-*；素材状态：real（仅限 OK 项），缺失保持静音
- 引用文档：ASSET_SHOPPING_LIST §4.4.6; ASSET_GAPS §3.3; ASSET_GAPS §3.5; ARCHITECTURE §11.8; ARCHITECTURE §12.2; ARCHITECTURE §12.7; ARCHITECTURE §11.4

## 背景
M3-037/038/039 已用静音占位完成音频映射。本任务接入**已 `OK`** 的 M3 音频：重武器（A-WPN-*）、爆炸/地雷/炸药包（B-EXP-002/003）、炮击/航弹（B-ART-*、B-AIR-*、B-HVY-001）、载具/飞机（D-TNK-*、D-VEH-001、D-AIR-001）、Foley（E-ACT-001）、环境床（F-AMB-*）、配乐与混响脉冲响应（IR）。（INDEX 写"配乐/IR"，其需求 ID 前缀在 SHOP §4.4.6 现有表中未单列，执行时以 ASSET_SHOPPING_LIST 实际登记的 ID 为准，Grep 补全。）
规则：
- 编码（D-AUD，ARCH §12.7）：Opus（WebM 容器内的数据放入 `.fpak`）为主、AAC 兜底；客户端启动时用 `decodeAudioData` 对内置探测样本实测，能解 Opus 则取 Opus 包，否则取 AAC 包。码率：枪声近距层 96 kbps，其它 48–64 kbps，环境床 32 kbps。
- 预算：首张地图音频总量 ≈ 18 MB（Medium）；High 加入变体至 ≈ 40 MB；短音（< 3 s）常驻为预解码 `AudioBuffer`，长音流式解码。
- 缺失素材保持静音占位，每 ID 每会话一条日志（ASSET_GAPS §3.3），不用其它层顶替；近/中/远三层缺哪层按该层静音。
- 只用 `LICENSES.md` 状态 `OK` 的素材；多厂牌/商店条款需书面确认（D-LIC）；`.fpak` 分块（R11），不提交原件。

## 具体内容
### 要创建/修改的文件
- `tools/asset-pipeline/recipes/audio/{需求ID}.json` —— 切分、响度归一（目标 −23 LUFS ±2，峰值 ≤ −1 dBTP）【待调参】、近/中/远层命名、循环点标记、Opus/AAC 双编码参数。
- `tools/asset-pipeline/src/audio_pack.ts` —— 转码（ffmpeg）、打包 `.fpak` 的 `audio` 类别分块，输出 manifest（含 `placeholder`、`codec`、`layer`、`duration_ms`、`loop` 字段）。
- `tools/asset-pipeline/src/audio_checks.ts` —— 校验：采样率 48 kHz、层完整性（近/中/远）、循环点、响度、时长、体积。
- `assets/manifest/audio_m3.json`（沿用 M2-047 路径）。
- `client/src/audio/AssetBank.ts`（改）—— 优先加载真素材，缺失回退静音占位；启动探测 Opus/AAC。
- `client/data/audio_events/*.json`（改）—— 映射表中补入真实素材 ID（不改事件名）。
### 要实现的行为
1. 读取 LICENSES.md，列出目标 ID 的状态；仅处理 `OK`。
2. 转码与打包；生成缺失报告；未 OK 保持静音占位。
3. 对每个 `OK` 素材做响度归一与层标注（near/mid/far），写 manifest。
4. 客户端：启动探测 Opus；失败走 AAC 包（用 Safari/不支持环境的模拟用例验证）。
5. 配乐（若有）：循环淡入淡出（≥ 2 s），音乐总线独立音量；缺失静音。
6. 卷积 IR：field/forest/town 三种（若 OK），长度 ≤ 3 s，单声道或立体声按 ARCH §12.1；缺失时混响总线直通（日志一次）。
### 接口与数据
- manifest 字段按 ARCH §11.3；`.fpak` 规则见 ARCH §11.8（按 64 m 区块/类别分包，可 HTTP Range）。

## 不做什么
- 事件映射与参数自动化（M3-037/038/039，已完成）；语音 LiveKit 链路。
- 自制/AI 生成/剪切拼接创造新音效（A2/D-MOD 禁止）；未经确认的付费声库入库。
- M4 的环境声（F-AMB-002/004，见 M4-029）。

## 验收标准
1. 报告列出所有目标需求 ID 的 `real/placeholder/missing` 数量，写入 STATUS（`pnpm -C tools/asset-pipeline report --milestone M3 --type audio`）。
2. 首张地图 Medium 音频总量 ≤ 18 MB 的增量预算：M3 新增后总量 ≤ 18 MB（Medium）/ ≤ 40 MB（High）；`manifest-budget` 作业通过，包总体积 ≤ 150 MB（Medium）。
3. 响度：每个已接入素材 `integrated LUFS` 在 −23 ±2（脚本表；对氛围床/配乐另列目标，写入 recipe）。
4. 近/中/远层完整性：已接入的炮/重武器声每个至少含 near+mid+far 三层，或在报告中显式标注缺哪层。
5. 编码探测：Chromium 下取 Opus 包；强制禁用 Opus（`?force_aac=1` 调试开关）下取 AAC 包，两条路径均可 `decodeAudioData` 成功（Playwright）。
6. 缺失保持静音：删除 1 个已接入素材后，运行 30 s 场景，0 未捕获错误，日志该 ID 1 条。
7. 管线对非 `OK` 素材拒绝处理；`git ls-files assets/incoming assets/processed` 为空。

## 测试要求
- 单元：recipe 解析、响度计算、层命名规则、探测逻辑。
- 集成：转码一个样例 → `.fpak` → 客户端加载 → OfflineAudioContext 渲染检查非静音。
- 性能：解码耗时（短音批量预解码 ≤ 300 ms/首包，Playwright 记录）；内存增量写 STATUS。
- 回归：M3-037/038/039 的静音占位测试需在"全缺失"开关下仍通过。

## 性能预算
- ARCH §12.7（18 MB/40 MB）；§11.4 体积；§12.4 复音；音频主线程 ≤ 0.5 ms（§13.5.1）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 真正依赖素材：全部非 OK 则 `blocked`（H-070、H-050、H-069：5 项音频 GAP）；部分则 `partial`。
- GAP-A05（坦克被击中）、GAP-A15（航弹）等默认保持静音，不用其它素材顶替。
- 响度目标 −23 LUFS 为本任务提议值【待调参】，须在听感检查后定稿（需人工）。
- 付费声库的"嵌入产品、不得独立提取"条款由 `.fpak` 满足合规层面，非 DRM（D-PAK）。
