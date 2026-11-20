# Carentan 1/3 Medium 首图包

2026-11-20，EPYC 7402 工作站本地构建，按 PI-109 记录为非参考机。构建全程使用 CPU。

## 入包内容

- M1-035 的 380 个 `FCHK` 地形和 380 个 `FCOL` 碰撞条目，沿用空间分包。重烘焙的地形清单与已提交的 `terrain_carentan.json` 一致。
- 5 个 Poly Haven CC0 模型。31 个 glTF、BIN 和纹理文件共 141,569,522 B，通过登记的路径、SHA-256、大小、`OK` 状态和许可门禁。模型走 normalize、LOD、KTX2、meshopt 和 FPAK 的已有路径。
- 15 个程序化灰盒模型。22 个合成提示音各带 WebM/Opus 和 MP4/AAC，使用 `snd_ph_*` ID。包内公开灰盒覆盖四个有内部专用来源的需求。

| 需求 | 入包模型 | `LICENSES.md` 台账 ID |
| --- | --- | --- |
| `V-NAT-002` | `veg_tree_small_02` | `AST-V-NAT-002-01-11` |
| `V-NAT-004` | `veg_fern_02` | `AST-V-NAT-004-01-02` |
| `V-NAT-005` | `prop_rock_05` | `AST-V-NAT-005-04-02` |
| `V-PRP-004` | `prop_ammo_box` | `AST-V-PRP-004-01-02` |
| `V-PRP-006` | `prop_wooden_crate_02` | `AST-V-PRP-006-03-05` |

Tree Small 02 的枝条贴图使用同一组 UV 变换。构建将该变换烘入 UV0，以适配 mesh v1。树叶透明度、画质与帧率未测，视觉部分待 Fable 设计与验收。

## 验证

复现命令与缓存目录见 `DEVELOPING.md` 的 Carentan Medium 小节。原档、转换中间件、`.fpak` 和生产客户端产物均在本地构建，仓库交付清单及报告。门禁对照实际 `.fpak` 文件，检查索引、条目偏移、SHA-256、整包物理字节、内容寻址、独立素材文件扫描以及读回重写后的逐字节相等。core 使用含 sim-wasm 的生产客户端，以 Brotli 字节计量。

| 本地门禁 | 实测 | 上限 |
| --- | ---: | ---: |
| Medium `.fpak` 物理总量（35 包、870 条目） | 107,438,594 B | 150,000,000 B |
| `manifest-budget` Medium 条目和索引 | 107,430,081 B | 157,286,400 B |
| 生产 core Brotli（195 文件，原始 4,395,538 B） | 1,189,937 B | 8,000,000 B |

`carentan-medium` 全流程 703.02 s。独立复跑的三个真实模型包与组合构建同路径、逐字节一致。占位模型和音频的重复构建由测试验证。`scripts/local-ci.sh assets` 在 #158 基线上复跑用时 100.80 s，175 单测、11 e2e 通过。台账 242 行，232 OK、3 PENDING、1 FLAGGED、0 REJECTED、6 INTERNAL。Medium 只引用 5 个 OK 来源。

构建级缺失报告覆盖 168 项。按 M1 的 72 项计，5 real、14 placeholder、49 missing、4 internal-only。High / Ultra 未测。

`CARGO_BUILD_JOBS=8 scripts/local-ci.sh determinism` 通过。3 个种子各 18,000 tick，原生与 WASM 哈希和位置逐 tick 一致。

## 余项

逐素材运行时落位、浏览器帧性能、真实录音、历史语音、播放混音与参考机未测。`work/playable` 的独立音频台账和编码来自另一条集成线，本次里程碑包使用当前台账与程序合成音效。逐需求缺口见 `assets/manifests/missing-report.json` 和 `docs/ASSET_GAPS.md` §3.7，人工裁决见 H-047、H-058。
