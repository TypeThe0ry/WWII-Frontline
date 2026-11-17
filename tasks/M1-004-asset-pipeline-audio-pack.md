# M1-004 素材管线：音频处理与打包

## 元数据
- 里程碑：M1
- 工作流：assets
- 规模：M
- 依赖：M1-003
- 素材需求：无（用样例音频）；素材状态：n.a.
- 引用文档：ARCHITECTURE §12.7; ARCHITECTURE §11.1; ARCHITECTURE §11.8; ASSET_SHOPPING_LIST §3.4; ASSET_GAPS §3.3; LICENSES.md §2

## 背景
音频走同一条管线（ARCHITECTURE §11.1 末尾、§12.7）：`ffmpeg`/`opusenc` 处理 → 打入 `.fpak`（M1-003 的 `kind:'audio'` 条目）。决定 D-AUD：**Opus（WebM 容器内的数据放入 .fpak）为主，AAC 兜底**；客户端启动时用 `decodeAudioData` 对内置探测样本**实测探测**，能解 Opus 用 Opus 包，否则取 AAC 包。
参数（ARCHITECTURE §12.7）：48 kHz；枪声近距层 96 kbps、其它 48–64 kbps、环境床 32 kbps；首张地图音频总量 ≈ 18 MB（Medium），High ≈ 40 MB。短音（< 3 s）预解码常驻，长音流式。
合规做法（ASSET_SHOPPING_LIST §3.4）：源 WAV 永不进入公开 CDN；**剔除 BWAV/iXML/Soundminer 等元数据**；多个音效合并为按武器/类别的包；不提供试听入口。处理动作须在台账 `allowed_processing` 内（`TRANSCODE`、`LOUDNORM`、`EDIT`、`LAYERMIX`，LICENSES §2）。
缺失音频在运行期的占位规则属客户端（ASSET_GAPS §3.3：返回 0.1 s 静音 `AudioBuffer`，每需求 ID 每会话一条日志）；本任务的"缺失"仅体现在报告里。

## 具体内容
### 要创建/修改的文件
- `tools/asset-pipeline/src/audio/loudness.ts` —— EBU R128 响度测量/归一（`ffmpeg loudnorm` 两遍），目标按类别：SFX 近层 -18 LUFS、环境 -24 LUFS、UI -20 LUFS【待调参】；输出每素材 LUFS/True Peak 报告
- `tools/asset-pipeline/src/audio/encode.ts` —— `encodeOpusWebm(wav,{bitrate,channels})`、`encodeAac(wav,{bitrate})`（ffmpeg 子进程封装）；48 kHz 重采样；去全部元数据（`-map_metadata -1`，并校验输出无 iXML/BWF chunk）
- `tools/asset-pipeline/src/audio/pack-audio.ts` —— 按"包"（每武器/类别）把多条目写入 FpakWriter；每个逻辑 ID 产出 `…#opus` 与 `…#aac` 两个条目
- `tools/asset-pipeline/src/audio/probe-sample.ts` —— 生成内置探测样本（极短静音 Opus/AAC，各 < 2 KB）并写入 `core` 包用的 `probe.json`
- `docs` 不改；探测协议写在 `tools/asset-pipeline/src/audio/PROBE.md`
- `tools/asset-pipeline/test/audio.*.test.ts` 与 `fixtures/sine_1s.wav`（自生成正弦，不引入外部音频）
### 要实现的行为
1. 输入限定为台账 `OK` 且 `type ∈ {sfx, music, voice, ir}` 的素材（复用 M1-001 闸门）。
2. 响度归一后输出报告：每条 `integrated_lufs`、`true_peak_dbtp`、`duration_s`；报告汇总进 `work/audio-report.json`。
3. 编码：每个源产出 Opus-in-WebM（默认 64 kbps；`class: gun_near` 96；`class: ambience` 32）与 AAC-LC（同比特率）；体积记入报告。
4. 打包：时长 < 3 s 标 `preload:true`（客户端预解码常驻），否则 `preload:false`（流式）；写入条目元数据（`duration_ms`、`channels`、`loop_start/loop_end` 可选）。
5. 探测协议：客户端启动时 `decodeAudioData(probe.opus)`，成功→请求 `#opus` 条目，失败（reject 或超时 500 ms）→`#aac`；协议与条目命名在 PROBE.md 固定，M1-006/M1-062 照此实现。
6. 元数据剔除自检：对输出 WebM/AAC 做 chunk 扫描，发现 `iXML`、`bext`、`SMED`、`LIST/INFO` 即失败。
### 接口与数据
```ts
interface AudioItem { logical_id:string; src:string; cls:'gun_near'|'gun_mid'|'gun_far'|'foley'|'ambience'|'ui'|'music'|'voice'; loop?:boolean }
function packAudio(items:AudioItem[], outPak:string): {entries:IndexEntry[]; report:AudioReport}
```
逻辑 ID 沿用 ARCHITECTURE §11.2 的 `snd_` 前缀（如 `snd_wpn_us_m1_garand_fire_near#opus`）；枪声三层（near/mid/far，ARCHITECTURE §12.2）为三个独立逻辑 ID。

## 不做什么
- 客户端 WebAudio 图、解码与播放：M1-062/M1-063；`decodeAudioData` 探测的客户端实现在 M1-065 前的加载器（M1-006 负责取包，M1-062 负责探测调用）。
- 距离分层混音、卷积混响素材制作（LAYERMIX 的具体音频编辑）：属素材接入 M1-065，不属于管线。
- 语音（LiveKit）：M2。
- 采购与台账登记：人类（H-049）。

## 验收标准
1. 样例往返：正弦 WAV → Opus 包 → 读回 → 解码（Node `ffmpeg` 解码）后时长误差 ≤ 20 ms，与源波形相关系数 ≥ 0.95（单测）。
2. 响度报告：归一后样例的 integrated LUFS 与目标差 ≤ ±1 LU，True Peak ≤ -1 dBTP；报告 JSON 含每条记录（单测）。
3. AAC 兜底：删除 opus 条目的包上，探测协议选择 `#aac`，单测覆盖选择函数 `chooseCodec(probeResult)` 全部 4 种组合（opus 成功/失败 × aac 成功/失败，最后一种返回 `none` 并报错）。
4. 元数据自检：向输入 WAV 注入 `bext`/iXML chunk，输出产物检测为 0 个此类 chunk（单测）。
5. 包体积：1 s 64 kbps 单声道样例 Opus 条目 ≤ 9 KB（64000 bps ≈ 8 KB/s + 容器开销）。
6. 产物目录通过 M1-003 的 `assert-no-standalone`（无 .ogg/.opus/.wav/.m4a 独立文件）。
7. 缺失音频需求 ID 在 M1-001 `report` 中显示为 `missing`，不报错、不阻塞打包（单测）。

## 测试要求
- 单元测试：响度解析、编码参数选择表、元数据剔除、`chooseCodec`、preload 阈值（2.99 s/3.0 s 边界）。
- 集成：`ingest → audio pack → fpak read` 全链路（fixtures）。
- 性能基准：30 条各 5 s 样例打包 ≤ 60 s（记录机器）。
- 回归：PROBE.md 协议变更须登记 PLAN_ISSUES。

## 性能预算
- 构建期工具；产物预算服务 ARCHITECTURE §12.7（首图 Medium 音频 ≈ 18 MB）与 §11.4（地图包 ≤ 150 MB）；本任务只输出各包体积，门禁在 M1-005。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `ffmpeg`/`opusenc` 的许可证（LGPL/GPL 构建差异）需登记 LICENSES.md §5；不要把 ffmpeg 二进制打入发布物。
- Safari 对 Opus-in-WebM 的支持因版本而异（未核实）；这正是探测 + AAC 兜底存在的原因，不要在代码里按 UA 判断。
- 音频素材未到位不影响本任务（用自生成样例）。
