# LICENSES.md — 素材与第三方许可证台账

> 本文件是 Frontline 的**素材台账模板**与第三方软件许可证说明。人类负责人采购/下载素材后，在 §3 的台账表里登记；AI 执行者的素材管线读取该表决定哪些素材可以进入构建。
> 采购流程见 `docs/ASSET_SHOPPING_LIST.md`；找不到现成素材的项目见 `docs/ASSET_GAPS.md`。
> 本文不是法律意见。章节编号稳定，其它文档按 "LICENSES.md §3" 引用，请勿重排。

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
| AST-EXAMPLE-0001 | V-NAT-007 | 【示例，非真实登记】某 CC0 地表材质 | texture | Poly Haven | 示例作者 | https://polyhaven.com/（示例） | CC0 1.0 | licenses/texts/AST-EXAMPLE-0001.pdf | Y | Y | Y | NONE | Y | FREE-2026-01-01 | 2026-01-01 | 示例 | — | assets/incoming/V-NAT-007/polyhaven-FREE-2026-01-01/ | — | CONVERT, TEXCOMP | OK | 示例行：CC0 无需书面确认；已按 ASSET_SHOPPING_LIST §2.3 核对（示例） | — |
| AST-EXAMPLE-0002 | A-WPN-001 | 【示例，非真实登记】某付费枪声库 | sfx | A Sound Effect | 示例厂牌 | https://asoundeffect.com/（示例） | 厂商 EULA（示例日期） | licenses/texts/AST-EXAMPLE-0002.pdf | Y | PENDING | Y | NONE | Y | EXAMPLE-ORDER-0000 | 2026-01-01 | 示例 | — | assets/incoming/A-WPN-001/asoundeffect-EXAMPLE-ORDER-0000/ | — | TRANSCODE, LOUDNORM, EDIT, LAYERMIX | PENDING | 示例行：等待厂商就"浏览器分发/可提取性"书面确认，确认前不得进入主分支 | — |

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
