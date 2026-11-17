# LICENSES.md — 素材与第三方许可证台账

> 本文件是 Frontline 的**素材台账模板**与第三方软件许可证说明。人类负责人采购/下载素材后，在 §3 的台账表里登记；AI 执行者的素材管线读取该表决定哪些素材可以进入构建。
> 采购流程见 `docs/ASSET_SHOPPING_LIST.md`；找不到现成素材的项目见 `docs/ASSET_GAPS.md`。
> 本文不是法律意见。章节编号稳定，其它文档按 "LICENSES.md §3" 引用，请勿重排。

> **注记（2026-11-17，负责人新指示，当前生效）**：素材（模型、音效）优先使用免费资源，**许可证不再深究**——每件素材只需登记 asset_id、需求 ID、名称、作者、来源 URL、页面标注的许可、下载日期、是否需要署名，**详见本文 §6"简化登记（当前生效）"**。下文 §1–§4 的详细规则（5 项核对、书面确认、`PENDING` 不得入库、证据存档等）与 §2 的详细字段**保留为可选**——仅在购买付费素材时作为建议。免费素材的清单与来源见 `docs/ASSET_SHOPPING_LIST.md` §8，找不到免费素材的项目见其 §9。

---

## 1. 登记规则

1. **任何素材进入仓库或对象存储（含 `assets/processed` 上传到 R2/CDN）之前，必须先在 §3 登记。** 没有登记的素材，管线必须拒绝处理。
2. **状态枚举只有三个**：
   - `OK`：已核对 5 项且全部通过，许可证全文与（如需）卖家书面确认已存档，可以进入构建。
   - `PENDING`：条款不明、互相矛盾、等待卖家书面确认，或 5 项中任一项为 `PENDING`。**状态为 `PENDING` 的素材不得进入主分支**（可以留在本地 `assets/incoming/`，不得提交其处理产物，不得上传对象存储）。
   - `REJECTED`：5 项中任一项为 `N`，或被判定为疑似游戏提取/NC/ND/SA/Editorial/UE-Only 等禁用类别。保留台账行作为"已排除"记录，防止重复购买；**其文件必须从 `assets/incoming/` 删除**，不得进入构建。
3. **状态流转**：`PENDING → OK`（拿到书面确认且存档）；`PENDING → REJECTED`；`OK → REJECTED`（发现违规时，须同时下架已发布的包）。不允许直接编辑历史行的结论而不留痕——改状态时在 `notes` 追加"YYYY-MM-DD 原因"。
4. **5 项核对**（详见 §2）：非原生引擎、网页分发、商用（含捐赠）、署名、席位。`OK` 要求前四项里的"允许类"全部为 `Y`，署名项已明确（`NONE` 或 `REQUIRED` 且已填 `credit_text`），席位为 `Y`。
5. **付费素材的入库条件（D-LIC）**：许可条款允许"非原生引擎 + 网页游戏嵌入分发 + 商用"。对"慎用"渠道（TurboSquid、CGTrader、Sketchfab CC 之外、KitBash3D、ActorCore、A Sound Effect 多厂牌等）必须先拿到卖家/厂商**书面确认**，否则保持 `PENDING`。CC0（Poly Haven、ambientCG）与 Fab Standard License 为首选。
6. **证据存档**：
   - 许可证全文快照（HTML 另存 PDF/文本）→ `licenses/texts/{asset_id}.pdf`（或 `.txt`）。
   - 卖家/厂商书面确认邮件（含完整邮件头）→ `licenses/confirmations/{asset_id}.pdf` 或 `.eml`。
   - 发票/订单截图 → `licenses/receipts/{asset_id}.pdf`。
   - 以上目录**进 git**（它们是法律证据，体积小）；原始素材文件放 `assets/incoming/`（`.gitignore`，不进 git）。
7. **疑似游戏提取检查**：登记前必须按 `docs/ASSET_SHOPPING_LIST.md` §2.3 的检查表核对；通过后在 `notes` 写"已按 §2.3 核对"。
8. **"不自制"边界（D-MOD）**：台账 `allowed_processing` 只能填 D-MOD 允许的处理，且不得超过卖家书面确认的范围；允许的加工代码见 §2 末尾。**D-MOD 例外（默认允许，待人确认）**：为合规，对素材贴图/模型上的纳粹符号（卐等）与现实厂商商标做**遮盖/移除**（`MATPARAM` / 贴图遮盖处理）属于允许的处理，不视为“重画贴图”；仍不得新绘制替代图案以外的内容。
8a. **执行者职责**：AI 执行者**可以且必须**在本文件登记新素材/新依赖（来源、作者、许可证、链接、订单号），但**不得把状态从 `PENDING` 改为 `OK`**，除非 `licenses/` 下已有条款全文或卖家书面确认作为证据；执行者不得购买任何素材（购买由人类负责人完成）。
9. **原始文件与处理后包**：原始文件路径填 `assets/incoming/{需求ID}/{渠道}-{订单号}/…`；处理后素材只以 `.fpak` 容器分块形式交付（D-PAK），**不以标准 `.glb/.ktx2/.ogg` 独立文件暴露**；源 WAV/源模型永不进入公开 CDN。
10. **日常节奏**：每次购买当天登记；每个里程碑验收前，执行者运行一次"台账校验"（见 §4.4）。

---

## 2. 字段定义

| 字段 | 类型/取值 | 说明 |
|---|---|---|
| AST-V-FNT-UI-01 | — | Barlow Semi Condensed（500/600） | font | GitHub / Google Fonts (官方仓库) | The Barlow Project Authors (Jeremy Tribby) | https://github.com/jpt/barlow | SIL OFL 1.1 | licenses/texts/AST-V-FNT-UI-01.txt | Y | Y | Y | NONE | Y | FREE-2026-11-17 | 2026-11-17 | 免费-执行者核实 | — | —（未入库；如需验证放 assets/incoming/fonts/） | — | CONVERT | OK | UI_SPEC §2.3 标题体。许可证原文取自 https://raw.githubusercontent.com/google/fonts/main/ofl/barlowsemicondensed/OFL.txt（与 https://raw.githubusercontent.com/jpt/barlow/master/OFL.txt 内容一致，4377 字节），访问 2026-11-17；Google Fonts METADATA.pb 标 license OFL。无 RFN：版权行为 "Copyright 2017 The Barlow Project Authors" 无 Reserved Font Name 声明。OFL 允许自托管 @font-face、随游戏分发、商用；子集化与 WOFF2 转换属 Modified Version（OFL FAQ 2.2/2.6，https://openfontlicense.org/ofl-faq/ 2026-11-17），因无 RFN 可保留原名。随字体分发须附带许可证文本（CREDITS/许可页放 OFL 全文）。版本号：未取到上游 tag，构建时以实际下载文件的 name 表版本为准。 | — |
| AST-V-FNT-004-01 | V-FNT-004 | IBM Plex Sans（400/500/600） | font | GitHub / Google Fonts (官方仓库) | IBM Corp. | https://github.com/IBM/plex | SIL OFL 1.1 | licenses/texts/AST-V-FNT-004-01.txt | Y | Y | Y | NONE | Y | FREE-2026-11-17 | 2026-11-17 | 免费-执行者核实 | — | —（未入库；如需验证放 assets/incoming/fonts/） | — | CONVERT | PENDING | 许可证原文取自 https://raw.githubusercontent.com/IBM/plex/master/LICENSE.txt（4456 字节），访问 2026-11-17；npm @ibm/plex-sans 最新 1.1.0（OFL-1.1），@fontsource/ibm-plex-sans 5.3.0。有 RFN：版权行 "Copyright © 2017 IBM Corp. with Reserved Font Name Plex"。按 OFL FAQ 2.2/2.6/2.7 子集化与（非严格无损）WOFF2 转换属 Modified Version，不能继续使用 RFN "Plex"，除非保持功能等价或获作者许可。拿不准故 PENDING：待负责人决定，方案 A 子集产物改内部与 CSS 族名（不含 Plex，如 FL Sans，用 fonttools 改 name 表）；方案 B 使用 IBM 官方已发布的 WOFF2 且不子集化（体积超预算）；方案 C 向 IBM 取书面许可。许可本身（嵌入/商用/分发）无问题。OFL 无署名要求，但须随分发附许可证文本；自愿署名 voluntary。 | — |
| AST-V-FNT-004-02 | V-FNT-004 | IBM Plex Mono（500） | font | GitHub / Google Fonts (官方仓库) | IBM Corp. | https://github.com/IBM/plex | SIL OFL 1.1 | licenses/texts/AST-V-FNT-004-02.txt | Y | Y | Y | NONE | Y | FREE-2026-11-17 | 2026-11-17 | 免费-执行者核实 | — | —（未入库；如需验证放 assets/incoming/fonts/） | — | CONVERT | PENDING | 同 AST-V-FNT-004-01：同一上游仓库与同一 LICENSE.txt（4456 字节），访问 2026-11-17；npm @ibm/plex-mono 最新 2.5.0，@fontsource/ibm-plex-mono 5.3.0。有 RFN "Plex"，子集化后不得沿用原名，处理方案见 004-01 notes，待负责人决定前保持 PENDING。仅用于数字与坐标，子集极小（拉丁数字与符号）。 | — |
| AST-V-FNT-004-03 | V-FNT-004 | Noto Sans SC（400/500/700） | font | GitHub / Google Fonts (官方仓库) | Google / Adobe（Source Han Sans 衍生） | https://github.com/notofonts/noto-cjk | SIL OFL 1.1 | licenses/texts/AST-V-FNT-004-03.txt | Y | Y | Y | NONE | Y | FREE-2026-11-17 | 2026-11-17 | 免费-执行者核实 | — | —（未入库；如需验证放 assets/incoming/fonts/） | — | CONVERT | OK | 许可证原文取自 https://raw.githubusercontent.com/google/fonts/main/ofl/notosanssc/OFL.txt（4388 字节），访问 2026-11-17；上游 https://github.com/notofonts/noto-cjk 的 Sans/LICENSE 同为 OFL 1.1。有 RFN "Source"（版权行 Copyright 2014-2021 Adobe with Reserved Font Name Source）：本字体族名 Noto Sans SC 不含 Source，子集化后须保证 name 表不出现 Source 一词（Noto 自身即为改名衍生）。OFL 允许嵌入、子集、WOFF2、商用、分发。体积实测：google/fonts 的 ofl/notosanssc/NotoSansSC[wght].ttf（可变，wght 100-900）为 17772300 字节（约 16.9 MiB），访问 2026-11-17，作为 M1-053 子集预算输入。版本号：未取到上游 tag，以实际下载文件 name 表为准。随分发附许可证文本。 | — |
| `asset_id` | 字符串，格式 `AST-{需求ID}-{NN}` | 台账唯一键，如 `AST-V-WPN-GE-001-01`（同一需求 ID 的第 1 件素材）。`NN` 为两位序号 |
| `req_id` | 字符串 | 对应需求 ID（沿用 `ASSET_SHOPPING_LIST.md`：视觉 `V-…`；音频如 `A-WPN-001`、`D-TNK-001`）。无专属 ID 的通用素材（如 Sonniss GDC 免费包、字体）填 `—` 并在 `notes` 说明用途 |
| `name` | 字符串 | 素材/商品名称（商品页标题） |
| `type` | 枚举 | `model` / `texture` / `hdri` / `animation` / `vfx` / `font` / `sfx` / `music` / `voice` / `ir` / `other` |
| `source_channel` | 字符串 | 来源渠道（如 Poly Haven、Fab、CGTrader、Sonniss、A Sound Effect） |
| `author_vendor` | 字符串 | 作者/厂商/厂牌名 |
| `url` | URL | 商品页/下载页链接（购买时的页面） |
| `license_name_version` | 字符串 | 许可证名称与版本，如 `CC0 1.0`、`Fab Standard License`、`CC-BY 4.0`、`SIL OFL 1.1`、`CGTrader Royalty Free License`、厂商自定义 EULA（写日期/版本） |
| `license_text_path` | 路径 | 许可证全文存档，位于 `licenses/texts/` |
| `chk_engine` | `Y` / `N` / `PENDING` | ① 允许在非原生引擎（Babylon.js 浏览器游戏）使用 |
| `chk_web` | `Y` / `N` / `PENDING` | ② 允许以转换后格式嵌入并随浏览器游戏分发（含"玩家可从缓存提取"风险已按 D-PAK 缓解并被许可/书面确认接受） |
| `chk_commercial` | `Y` / `N` / `PENDING` | ③ 允许商用，且覆盖"免费游玩 + 接受捐赠"（捐赠保守按收入对待） |
| `chk_attribution` | `NONE` / `REQUIRED` / `PENDING` | ④ 是否要求署名；`REQUIRED` 时必须填 `credit_text`，条目自动进入游戏内 Credits（§4） |
| `chk_seats` | `Y` / `N` / `PENDING` | ⑤ 席位：1 名人类负责人 + AI 执行者（同机处理文件）是否在授权数内 |
| `order_id` | 字符串 | 订单号/发票号；免费素材填下载日期时间戳或 `FREE-{日期}` |
| `purchase_date` | `YYYY-MM-DD` | 购买/下载日期 |
| `purchaser` | 字符串 | 购买人（账号/姓名缩写） |
| `confirmation_path` | 路径或 `—` | 卖家/厂商书面确认存档，位于 `licenses/confirmations/`；D-LIC 要求的素材此项必填 |
| `raw_path` | 路径 | 原始文件路径，位于 `assets/incoming/{需求ID}/{渠道}-{订单号}/…` |
| `processed_package` | 字符串 | 处理后所在 `.fpak` 包名（尚未处理填 `—`） |
| `allowed_processing` | 代码列表 | 允许的处理（D-MOD）。视觉/动画：`CONVERT`（格式转换）、`LOD`（减面/LOD）、`TEXCOMP`（贴图压缩）、`RETARGET`（动画重定向）、`REBIND`（换骨架/绑定权重传递）、`MATPARAM`（材质参数调整）、`ASSEMBLE`（多已购部件组合装配）。音频：`TRANSCODE`（编码转换）、`LOUDNORM`（响度标准化）、`EDIT`（剪辑）、`LAYERMIX`（距离分层混音）、`EQCOMP`（EQ/压缩）、`CONVOLVE`（卷积混响）。**不得出现 D-MOD 禁止项**（从零建模、雕刻新部件、重画贴图、AI 生成、重新录音）；若卖家书面确认只允许其中一部分，只填允许的 |
| `status` | `OK` / `PENDING` / `REJECTED` | 见 §1 |
| `notes` | 字符串 | 备注：游戏提取检查结论、疑点、状态变更记录（带日期） |
| `credit_text` | 字符串 | 扩展字段：游戏内署名文本；`chk_attribution = REQUIRED` 时必填（格式见 §4.2）；`NONE` 但作者"欢迎署名"的可选填，并在 `notes` 标注"voluntary"（自愿署名，Credits 页放在"致谢"分组） |

> `asset_id` 前缀 `AST-EXAMPLE-` 保留给示例行，校验工具忽略这类行。

---

## 3. 台账

> 下表**为空模板**，含 2 行**示例，非真实登记**（`AST-EXAMPLE-…`）。第一次真实登记时请**删除示例行**。表格为 Markdown，一行一件素材；列顺序与 §2 字段一致；单元格内不得出现 `|` 字符。管线按**表头列名**解析（不依赖列序），请勿改列名。

| asset_id | req_id | name | type | source_channel | author_vendor | url | license_name_version | license_text_path | chk_engine | chk_web | chk_commercial | chk_attribution | chk_seats | order_id | purchase_date | purchaser | confirmation_path | raw_path | processed_package | allowed_processing | status | notes | credit_text |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

（以下为真实登记区，从这里往下追加行；也可把示例行替换为首条真实记录。）

---

## 4. 游戏内署名清单（CREDITS）生成规则

### 4.1 哪些条目必须署名

- `chk_attribution = REQUIRED` 且 `status = OK` 的素材**必须**进入游戏内 Credits。典型来源（取自调研，具体以台账为准）：CC-BY 类素材（Freesound CC-BY、RenderHub "RC Art - Muzzle Effects" 为 CC-BY 4.0、Fatal Exit "After The War" 为 CC-BY 4.0、OpenAIR 中的 CC-BY 条目、SoundBible CC-BY 3.0 等）、Zapsplat 免费账号下载的素材，以及任何卖家书面确认要求署名的素材。
- `chk_attribution = NONE` 但有填 `credit_text` 且 `notes` 含 "voluntary" 的条目，进入 Credits 页的"致谢（自愿署名）"分组（如 Poly Haven、Sonniss 的署名为"受欢迎但非必需"）。
- 软件依赖的许可证声明（§5）单独成"开源许可证"页，由工具自动生成，不手填。

### 4.2 `credit_text` 格式

`"{素材名}" by {作者} — {许可证名称与版本} — {URL}`；若许可证要求注明修改，追加 `（已修改：{处理}）`。示例（仅格式，非真实）：`"Example Sound" by Example Author — CC BY 4.0 — https://example.org/...`。署名文本保持原语言，不翻译；Credits 页的标题/分组名走 i18n key。

### 4.3 自动汇总流程（`/tools/asset-pipeline` 的 `credits` 步骤）

1. 解析本文件 §3 表格（按表头列名），忽略 `AST-EXAMPLE-` 行与 `REJECTED`/`PENDING` 行。
2. 选出 `chk_attribution = REQUIRED`（必须署名）与 voluntary 条目，按 `type` 分组、组内按 `name` 排序；按 `(author_vendor, license_name_version)` 合并相同作者的多件素材以缩短列表。
3. 输出 `assets/processed/credits.json`（清单进 git）：每条含 `asset_id`、`group`、`credit_text`、`required`（bool）、`url`。该 JSON 体积很小，随主包发布，**不放进 `.fpak` 素材包**（避免与"不可独立提取"策略混淆）。
4. 客户端 Credits 界面读取 `credits.json`，按分组渲染；`required = true` 的条目不可被设置项隐藏。
5. 管线在构建时校验（任一失败即构建失败）：
   - `.fpak` manifest 中出现的每个 `asset_id` 在台账里必须存在且 `status = OK`；
   - manifest 中所有 `chk_attribution = REQUIRED` 的素材都出现在 `credits.json`；
   - `REQUIRED` 条目的 `credit_text` 非空；
   - 台账中不存在 `OK` 行缺少 `license_text_path` 对应文件，或 D-LIC 要求书面确认的素材缺少 `confirmation_path` 对应文件。
6. 素材被替换/下架时，对应条目随台账状态变化自动从 Credits 中移除。

### 4.4 台账校验（里程碑验收前运行）

执行者运行一次校验脚本并在任务回报里贴出结果：列出 `PENDING` 条目数、`REJECTED` 条目数、缺失存档文件、manifest 与台账不一致项。**存在 `PENDING` 条目对应的处理产物进入主分支即视为违规。**

---

## 5. 第三方软件依赖许可证

> **总体说明**：下表许可证均为**凭记忆，未核实**（核实日期：未执行）。**执行者在 M0 用工具自动生成并核实**：Rust 用 `cargo-deny`（`cargo deny check licenses` + `cargo about`/等价工具生成声明），前端用 `license-checker`（或等价工具）导出 `THIRD_PARTY_LICENSES`，并把结果提交到仓库、接入 CI。下表在 M0 自动生成后用实测结果覆盖。

| 依赖 | 用途 | 许可证（凭记忆，未核实） | M0 核实与处理 |
|---|---|---|---|
| Babylon.js 8.x | 渲染引擎 | Apache-2.0（未核实） | 自动生成；Apache-2.0 通常要求随分发保留许可证文本与 NOTICE，具体以实测文本为准 |
| Rapier 3D | 物理（服务器原生 + 客户端 WASM） | Apache-2.0（未核实） | 同上；WASM 产物同样属于分发 |
| LiveKit（自托管 SFU 与客户端 SDK） | 语音 | Apache-2.0（未核实） | 同上；区分服务器端（自托管，不随游戏分发）与客户端 SDK（随游戏分发） |
| bevy_ecs | 模拟核心 ECS（`sim` crate） | MIT / Apache-2.0 双许可（未核实） | 同上；WASM 产物随游戏分发 |
| wtransport | 服务器 WebTransport | 未核实（凭记忆倾向 MIT/Apache-2.0 类宽松许可，不确定） | **必须实测**，不得凭记忆放行 |
| Preact | UI | MIT（未核实） | 同上；MIT 要求保留版权声明与许可证文本 |
| 其余直接/间接依赖（tokio、gltf-transform、meshoptimizer、KTX/Basis Universal、libopus、Recast 导航库、Vite 等） | 各模块 | **未核实**（不在此凭记忆给出） | 自动生成，覆盖全部传递依赖 |
| 字体（与 `docs/UI_SPEC.md` §2.3 一致：Barlow Semi Condensed、IBM Plex Sans、IBM Plex Mono、Noto Sans SC） | UI | 计划为 SIL OFL 1.1（**未核实**） | 状态 `PENDING`，**M0 任务核实**许可证文本、可网页嵌入/子集化条款与当前版本后，作为素材登记在 §3（`type = font`）并存档 OFL 文本；注意保留字体名（RFN）限制与子集化是否触发改名条款；未登记不得入库（R10） |

**依赖许可证政策（建议，待负责人确认）**：

- 默认允许：MIT、Apache-2.0、BSD-2-Clause、BSD-3-Clause、ISC、Zlib、CC0-1.0、SIL OFL-1.1（字体）；SPDX 标识符以所用工具版本的实际名称为准。
- 默认禁止进入客户端 WASM 与前端打包：GPL/AGPL 家族（静态链接或随游戏分发视同分发，风险高）。
- 需个案评估：LGPL、MPL-2.0 及其它弱 copyleft（由 `cargo-deny` 报出后，负责人逐案决定）。
- 任何新增依赖都必须通过 CI 的许可证检查（`cargo deny` / `license-checker`），未知许可证一律阻断合并。
- 服务器端依赖（不随游戏分发给玩家）可适度放宽，但仍须登记；公开测试前再复核一次。

---

## 6. 简化登记（当前生效）

> 依据：负责人 2026-11-17 指示（总纲 R12 零采购基线）。**本节优先于 §1–§4 中与之冲突的表述**（"必须书面确认""`PENDING` 不得入库""5 项逐条核对""许可证全文/邮件存档"等对免费素材不再是硬性要求）。本文不是法律意见。

### 6.1 每件素材只需登记这些

在 §3 的台账表里追加一行（列名沿用 §2，管线按表头列名解析；§2 的其它列可省略或填 `—`）：

| 列 | 填什么 |
|---|---|
| `asset_id` | `AST-{需求ID}-{NN}`（同 §2） |
| `req_id` | 需求 ID（如 `V-WPN-GE-007`、`A-WPN-009`）；通用免费包（如 Sonniss GDC）填 `—` 并在 `notes` 写用途 |
| `name` | 素材名称（来源页标题） |
| `author_vendor` | 作者 / 厂牌 |
| `url` | 来源页面 URL（`ASSET_SHOPPING_LIST.md` §8 已列出候选的链接） |
| `license_name_version` | **页面标注的许可名称**，如 `CC0`、`CC BY 4.0`、`Fab Standard License`、`CGTrader Royalty Free License`、`Sonniss GameAudioGDC License`；页面没写就填 `未写明` |
| `purchase_date` | 下载日期 `YYYY-MM-DD`（免费素材的"购买日期"） |
| `chk_attribution` | `NONE` 或 `REQUIRED`：**CC-BY 一律 `REQUIRED`**；`REQUIRED` 时必须填 `credit_text`，条目自动进入游戏内 Credits（§4） |
| `status` | 见 §6.2 |
| `notes` | 一句话备注（如"来源页截图已存"、"B 档：页面标注 CC-BY-NC"、"已排查游戏提取"） |

可选（想存证时再填）：`license_text_path`、`confirmation_path`、`raw_path`、`allowed_processing`、5 项核对列。**不再要求**卖家书面确认、`licenses/confirmations/`、发票存档。

### 6.2 状态：枚举保留，免费素材默认 `OK` 或 `FLAGGED`

| 状态 | 含义 | 何时用 |
|---|---|---|
| `OK` | 可以进入构建 | **来源页有明确许可**（CC0、公共领域、CC-BY、平台免费商用许可等，即 `ASSET_SHOPPING_LIST.md` §8 的 A 档）；免费素材登记时的默认值 |
| `FLAGGED` | 登记在案，**等待负责人决定是否使用** | **B 档**：页面标注 CC-BY-NC / CC-BY-SA / CC-BY-NC-SA / CC-BY-NC-ND / 仅个人使用 / **许可未写明** / **来源存疑**（权属或出处无法判断）。负责人决定使用 → 改 `OK` 并在 `notes` 追加"owner-accepted YYYY-MM-DD 原因"；决定不用 → 改 `REJECTED`。 |
| `PENDING` | 保留，仅用于**付费素材**等待卖家书面确认的情形（§1 规则 5） | 免费素材**不再因许可不确定而使用 `PENDING`** |
| `REJECTED` | 已排除，保留台账行防止重复下载；文件必须从 `assets/incoming/` 删除 | 命中下面 §6.4 的排除项，或 B 档被负责人否决 |

> 给 `asset-pipeline` / `ingest` 相关任务的备注：`FLAGGED` 是在原三值枚举上新增的第四个值，**构建校验默认把 `FLAGGED` 视同 `PENDING`（不进主分支、不上传对象存储）直到负责人改成 `OK`**；`OK` 的免费素材无需 `confirmation_path` 即可通过。若校验脚本只识别 `OK/PENDING/REJECTED`，需要兼容该值（不改动 §1–§3 的既有字段与枚举）。

### 6.3 署名（CC-BY）

- `chk_attribution = REQUIRED`（CC-BY 与要求署名的免费档，如 Zapsplat 免费账号须署名 "ZapSplat"）：必须填 `credit_text`（标题 + 作者 + 链接 + 许可名），由 `credits` 步骤自动汇总进游戏内 Credits（§4.3）；漏填会被 §4.4 的台账校验报出。
- CC0 / Sonniss GameAudioGDC / Kenney 等不要求署名：填 `NONE`，作者"欢迎署名"的可在 `notes` 标 `voluntary`。
- 当前零采购方案预计需署名条目很少（主要是 CC-BY 的模型与音乐，调研估计音频侧 3–10 条）。

### 6.4 排除项（唯一保留的硬性排除）

下列素材**不得登记为候选，也不得进入 `assets/incoming/`**；已发现的登记为 `REJECTED`：

1. **从商业游戏或电影提取、拆包、翻录的素材**（含标题/标签/描述出现游戏名或 "ripped / extracted / from game" 的模型与音效，含 Freesound 上的游戏翻录）；
2. **盗版合集**；
3. **倒卖的盗版包**（**包括闲鱼等二手平台上转卖的模型包**）；
4. **来源不明的网盘资源**（论坛/网盘"WW2 资源合集"类）。

理由：这类素材的"作者"无权授权，页面标了免费或 CC 也一样；一旦混入，整个游戏的授权链失效，且违背"不自制、不 AI 生成、只用正规现成素材"的原则。识别方法见 `docs/ASSET_SHOPPING_LIST.md` §2.3 检查表与 §8.22 已排除清单；每件素材登记时在 `notes` 写"已排查提取"即可。

### 6.5 其它仍然有效的约定

- 仍不自制、不 AI 生成；`allowed_processing` 仍以 D-MOD 为准（可选填）。
- 处理后素材仍只以 `.fpak` 分块容器交付（D-PAK）；源文件不进公开 CDN；未登记的素材管线仍应拒绝处理（§1 规则 1，登记成本已降到 §6.1 的几列）。
- 免费平台素材可随时被替换：用同一需求 ID 的新素材替换时，把旧行状态改 `REJECTED` 或在 `notes` 追加"superseded by AST-…"。
