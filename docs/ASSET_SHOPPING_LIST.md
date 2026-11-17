# Frontline 素材采购清单（ASSET_SHOPPING_LIST）

> **先看这里（2026-11-17 负责人最新指示，优先于本文下方的旧流程）**
> 1. **当前基线 = "零采购 / 免费优先"**（总纲 R12）：模型、贴图、动画、音效、音乐最好全部用免费资源；许可证只记录来源页面标注的许可名称，不再逐条核对条款、不再要求卖家书面确认、不再因许可不确定而把免费素材卡在 PENDING。
> 2. **请先读 §8（能免费找到哪些）和 §9（找不到免费的 → 哪里可以买）**；§10 是覆盖率总览（含各里程碑只用免费素材时的状态、调研局限、建议人工补搜的需求 ID）。
> 3. §3–§6 的付费候选、渠道条款与许可流程**保留作参考，付费项为备选**（需负责人逐项批准）；§5.4 给出"零采购 = $0"与"最小补缺"两档金额。
> 4. **唯一保留的排除项**：从商业游戏/电影提取或翻录的素材、盗版合集、倒卖的盗版包（**包括闲鱼等二手平台上转卖的模型包**）、来源不明的网盘资源——一律不列为候选。理由：这类素材的"作者"无权授权，一旦混入，整个游戏的授权链都会失效；同时我们不自制、不 AI 生成。（检查方法见 §2.3 与 §8.22。）
> 5. 免费素材登记只需：asset_id、需求 ID、名称、作者、来源 URL、页面标注的许可、下载日期、是否需要署名——见 `LICENSES.md` §6"简化登记（当前生效）"。
> 6. **（2026-11-17 新增）每项最便宜的可用商品、合格规格表、两档总价与"找不到"清单见 §11**；§11 的新调研推翻了 §9 / §4 中的部分旧推荐（RenderHub Loss 德军武器系列不再推荐；两个 $19.99 Fab 枪包仅第三人称精度；EVOLVE Studio Fab Pz.IV / Pz.V 仅 Unreal 格式），冲突处已在原位加注，以 §11 为准。

> 读者：人类负责人（审核、付款、把文件放进 `assets/incoming/`、在本表勾选"已购"）。
> 核实日期：**2026-11-17**（本文所有"已核实"均指该日期当天的页面所见；促销价会变，**下单前以结算页为准**）。
> 本文由两份调研片段（视觉、音频）合并整理而成：**全部链接、价格、证据强度标签均沿用调研原文，没有新增或编造**。片段中标"未核实/需人工确认"的内容原样保留该标记。
> 本文不是法律意见；金额较大（> US$500）或条款含糊的采购，请负责人读一遍原文或取得卖家书面确认。
> 章节编号稳定，其它文档/任务文件按 "ASSET_SHOPPING_LIST.md §4.2" 的方式引用，请勿重排。
> 本文 §8–§10 为 2026-11-17 追加（零采购基线），不影响 §1–§7 的编号。

---

## 1. 使用说明

### 1.1 本文是什么

- §3 告诉你"哪些渠道的条款能用、哪些慎用、哪些禁用"。
- §4 是**按里程碑分批**的采购清单：每个需求 ID 一行，列出免费/付费候选、推荐选择、许可证状态，以及你要填的 `选定候选` 与 `[ ] 已购`。
- §5 是三档预算；§6 是给卖家的询问邮件模板；§7 是调研局限与未核实清单。
- 找不到合格现成素材的项目不在本文展开，见 `docs/ASSET_GAPS.md`（本文只在对应行写"→ GAP-xx"）。

### 1.2 购买流程（每个需求项都按此走）

1. **审核**：读该行的候选与"推荐选择"，在 `选定候选` 列写下你选定的候选（名称 + URL）。若要换成清单外的候选，先按 §2.3 的"疑似游戏提取识别检查表"自查。
2. **取得书面确认（仅"许可证状态"为"需书面确认/PENDING"的行）**：用 §6 的邮件模板向卖家/厂商发邮件，把回复邮件（含完整邮件头）存成 PDF 或 `.eml`。没有拿到肯定答复，**不付款**；或付款但状态保持 PENDING，素材不得进入主分支（见 `LICENSES.md` §1）。
3. **购买**：用以项目/个人名义注册的账号付款，保留发票/订单号。Fab/Mixamo/Adobe 等需登录的渠道，截图保存"许可证"栏（Fab 要看到 `Standard License` 字样）。
4. **放入原始文件**：解压后放到 `assets/incoming/{需求ID}/{渠道}-{订单号}/`（例如 `assets/incoming/V-WPN-GE-001/fab-XXXX/`）。该目录在 `.gitignore` 中，**不进 git**。许可证文本快照、发票、书面确认邮件放 `licenses/texts/` 与 `licenses/confirmations/`（路径在 `LICENSES.md` 登记）。
5. **登记**：在 `LICENSES.md` 的台账表新增一行（状态 `OK` 或 `PENDING`），填写 5 项核对结果与订单号。
6. **回本表勾选**：把该行 `[ ] 已购` 改成 `[x] 已购`，并在 `选定候选` 写明最终购买的商品。完成后通知执行者（执行者的素材管线任务会从 `assets/incoming/` 读取）。

> 采购顺序建议（沿用视觉片段 §5.3）：
> ① 先领取所有免费：Poly Haven、ambientCG、Megascans（Fab Standard，先确认价格为 0，截图留证）、Sonniss GameAudioGDC。
> ② M0 先买 2 个 Fab 枪械包（$39.98）与 1 个 Mixamo/MoCap Online 入门动画；**这时就向卖家发"浏览器分发确认"邮件**，等回复再批量采购。
> ③ 回复为否定或模糊的卖家，转向 Fab Standard/CC0 来源或外包（见 `docs/ASSET_GAPS.md`）。
> ④ 每笔采购在 `LICENSES.md` 与 `assets/incoming/` 下留证。

> **注记（负责人 2026-11-17 指示）**：按负责人 2026-11-17 指示，免费素材只需登记来源页面与标注的许可；本条流程仅在购买付费素材时作为建议。

### 1.3 表格列说明（§4 通用）

| 列 | 含义 |
|---|---|
| 需求 ID | 沿用调研片段原 ID。视觉前缀 `V-`（如 `V-WPN-GE-001`）；音频沿用片段的"类别字母-类别-序号"（如 `A-WPN-001` 枪械、`B-EXP-001` 爆炸、`D-TNK-001` 坦克），**音频 ID 没有 `V-` 前缀，且与视觉 ID 不会重名** |
| 规格 | 来自片段的规格要求（面数/采样率等多数是**设计建议【待调参】**，不是硬标准） |
| 候选 | 免费候选 + 付费候选。格式：名称｜渠道/作者｜URL｜价格｜许可要点｜型号吻合度｜证据标签 |
| 推荐 | 调研片段的推荐；"未核实/需人工确认"保留 |
| 许可证状态 | `OK`（条款已读且允许，可直接登记 OK）／`需书面确认`（付费素材必须先拿卖家书面确认，D-LIC）／`PENDING`（条款不明或互相矛盾，或购前须核对商品页许可证栏） |
| 选定候选 | **你填** |
| 已购 | `[ ] 已购`，**你勾选** |

> **注记（负责人 2026-11-17 指示）**：按负责人 2026-11-17 指示，免费素材只需登记来源页面与标注的许可；本条流程仅在购买付费素材时作为建议。
> 其中"许可证状态"列的"需书面确认 / PENDING"在购买付费素材时仍可参考；免费素材见 `LICENSES.md` §6（`OK` / `FLAGGED`）。

### 1.4 证据强度标签图例（调研原文两套标记，本文保留原样）

| 标签 | 含义 |
|---|---|
| **【核实】** / **[F]** | 本次直接打开官方/商品页读到原文（仍是摘要，非法律意见） |
| **【搜索摘要】** / **[S]** | 仅在搜索结果摘要或第三方转述中看到，**未打开官方页面**，下单前必须人工复核 |
| **【估算】** | 非来自页面，仅作量级规划 |
| **[M]** | 凭记忆，未核实 |
| **PENDING** | 条款不明或互相矛盾，需向厂商邮件确认 |
| **GAP** | 本次搜索未找到合格候选（详见 `docs/ASSET_GAPS.md`） |

### 1.5 价格说明

- CGTrader / RenderHub / TurboSquid / Unity / itch.io 的商品页多数无法被抓取，所以这些价格几乎全是【搜索摘要】或"价格需人工确认"，**购买前必须在商品页确认**。
- A Sound Effect 页面可见价常显示 "0,00 $" 占位，下文价格取其结构化数据并注明。
- 所有"原价/促销价"以抓取当天为准。

---

## 2. 素材原则与红线

### 2.1 基本原则（来自总纲 A1、A2）

1. 商业模式：免费游玩、接受捐赠、无付费内容 → 所有素材必须允许**商用 + 网页分发**；捐赠按"收入"保守对待。
2. 团队无专职美术/音频 → **不自制、不 AI 生成**模型/动画/音效，只用现成正规素材（购买/CC0/明确允许商用的 CC-BY）。AI 生成素材站（ElevenLabs、Morphic 等）**一律不采用**。
3. 视觉片段中有两件商品在搜索摘要里被标注为"AI 辅助制作"（CGTrader 的某 Tiger I 带部分内饰；CGTrader 的某低模 C-47），**已排除**。
4. 首发只做 US / GER 两个阵营（总纲 A3）；苏军、英军素材不在本清单范围。
5. 所有 NC（非商用）、ND、SA、Editorial、UE-Only 素材禁用（见 §2.4）。
6. 内容品质现实：现成"game ready"二战枪械面数多在 2k–12k 顶点，属第三人称/中距离品质，**不是 HLL 级第一人称展示品质**（那通常是 30k–80k 三角形带内部结构 + 4K 贴图）。是否接受"M1 先用第三人称品质"需负责人决定（§7.3）。

### 2.2 "不自制"的边界（D-MOD，待负责人最终确认，默认按此）

| 允许（对已购/已登记素材） | 不允许 |
|---|---|
| 格式转换、LOD/减面、贴图压缩 | 从零建模 |
| 重定向动画（retarget）、换骨架绑定权重传递 | 雕刻新部件 |
| 材质参数调整 | 重画贴图 |
| 把多个已购部件组合装配（如：基础身体 + 服装/装具模块） | AI 生成任何素材 |

- **D-MOD 例外（待人确认，默认允许）**：为合规，对已购素材贴图/模型上的纳粹符号与现实厂商商标做**遮盖/移除**属于允许的处理，不视为“重画贴图”（见 NEEDS_HUMAN H-002/H-024）；不得借此新绘制其它内容。
- 音频方面允许：格式转换、响度标准化、距离分层混音、剪辑、EQ/压缩、卷积混响处理。
- 不允许：重新录音、AI 生成、把素材（含修改版）作为音效库/素材包再分发。
- **待确认（NEEDS_HUMAN）**：①对现成素材做减面、重绑定、拼装服装/装具、重材质是否算"自制"，这决定 GAP-V01/V02/V04/V06 能否用"买现成 + 加工"解决；②UI 图标由执行者按 `UI_SPEC` 以 SVG 原创绘制是否可接受（见 V-FNT 备注）；③真人配音/外包录制属"重新录音"，需人工决定，**不得由执行者自行推进**。

### 2.3 严禁游戏提取素材 + 识别检查表（购买/下载前必做）

**红线**：
- 严禁从任何游戏（含参考作品）提取模型、贴图、动画、音效；不得以"参考作品的音效"为名采样。
- 参考作品只允许黑盒观测（总纲 §1）。
- Sketchfab 等平台上大量二战模型是"从某游戏提取"的，**即使标了 CC 也非作者原创、作者无权授权，必须排除**。

**疑似游戏提取识别检查表**（符合任一条即排除；视觉素材逐条核对，音频素材核对第 1、2、5、6、7 条的对应项）：

1. 标题/描述/标签出现游戏名或工作室名（Call of Duty、CoD WW2、Hell Let Loose、Battlefield、Sledgehammer、Heroes & Generals、Enlisted、Day of Defeat 等）或 "ripped / extracted / port / from game / rip"。
2. 卖家同一天批量上传大量二战模型，且无制作过程图、无线框图、无 Substance/Marmoset 渲染说明。
3. 贴图带游戏式命名（如 `_cm`、`_gls`、`_nml` 后缀的整套）、有水印、有 UI 元素残留；模型带游戏内部网格拆分/多余的隐藏骨骼。
4. 面数/拓扑与"同一游戏世代的 LOD"一致（多级 LOD 且 LOD0 偏低）而卖家未说明来源。
5. 卖家资料页无作品集、无社交账号。
6. 买前请卖家书面声明"原创建模，无第三方游戏资产"；拒绝则不买（邮件模板见 §6.1）。
7. 对每件购入素材，在 `LICENSES.md` 记录：渠道、URL、作者、购买日期、订单号、许可证文本快照、卖家声明邮件。

> 重要：本清单所列候选均为作者名下上架的商品，但调研者**无法从搜索摘要证明任何一件的来源**；也没有打开过任何素材文件，无法验证质量、历史准确性、UV/贴图/绑定质量。购买前须按上表核对。

> **注记（负责人 2026-11-17 指示）**：按负责人 2026-11-17 指示，免费素材只需登记来源页面与标注的许可；本条流程仅在购买付费素材时作为建议。
> 但**游戏提取/盗版/倒卖/来源不明的排除项与上面的识别检查表仍然有效**（对免费素材同样适用，见 §8.22）。

### 2.4 禁用清单

| 禁用项 | 原因 |
|---|---|
| Fab UE-Only / Reference Only（UEFN）/ Legacy UE Marketplace License 的商品 | 仅限 Unreal Engine 使用 |
| 任何 "Editorial" 许可（TurboSquid、CGTrader 等） | 仅限新闻类用途，禁用于游戏。已发现一件 $14 的 TurboSquid Sherman 为 Editorial，排除 |
| CC-BY-NC / ND / Sampling+ | 非商用/禁止演绎 |
| CC-BY-SA | 衍生作品需同许可，对嵌入游戏的转换后素材是否触发 SA 有争议，**禁用** |
| BBC Sound Effects（RemArc） | 仅个人/教育/研究 [S] |
| TAU 空间 IR 库 | 明确禁止商用 [S] |
| AI 生成素材站 | 违背素材原则 |
| 一切游戏提取资源 | 见 §2.3 |
| BuiltByBit 4 车包（含 GMC CCKW 与 Opel Blitz） | 用途为 Minecraft 类平台，许可不一定适用，不建议（片段原文"需核实，不建议"） |

> **注记（负责人 2026-11-17 指示）**：按负责人 2026-11-17 指示，免费素材只需登记来源页面与标注的许可；本条流程仅在购买付费素材时作为建议。
> 对**免费素材**，上表中的 NC / ND / SA / 许可未写明等条目降为"B 档"（§8.0.1），由负责人逐项决定是否使用；仍然硬排除的只有"游戏提取 / 盗版 / 倒卖 / 来源不明"。

---

## 3. 渠道条款总表

### 3.1 说明

- "适用"列：V = 视觉（3D/贴图/动画/特效/字体），A = 音频。
- 五问：①非原生引擎（能否在 Babylon.js 等非 Unreal/Unity 引擎使用）②网页分发（含"可被玩家从浏览器提取"问题，见 §3.4）③商用与捐赠 ④署名 ⑤席位（单人/多人）。
- 分级：**推荐 / 慎用 / 禁用**；另有"PENDING"（待书面确认）。
- 来源列：URL｜核实日期（均为 2026-11-17）｜证据强度。取不到官方页面的已写明。

> **注记（负责人 2026-11-17 指示）**：按负责人 2026-11-17 指示，免费素材只需登记来源页面与标注的许可；本条流程仅在购买付费素材时作为建议。

### 3.2 渠道 × 5 问总表（视觉 + 音频合并）

| 渠道 | 适用 | ①非原生引擎 | ②网页分发 / 可提取性 | ③商用 + 捐赠 | ④署名 | ⑤席位 | 分级 | 来源 · 证据 |
|---|---|---|---|---|---|---|---|---|
| Poly Haven（CC0） | V | 可【核实】 | 可，明确允许再分发【核实】；网站 ToS 禁爬站，请手动/官方 API 下载 | 可【核实】 | 否【核实】 | 无 | **推荐** | https://polyhaven.com/license 【核实】 |
| ambientCG（CC0 1.0） | V | 可【核实】 | 可，"raw files in … video game"【核实】 | 可【核实】 | 否【核实】 | 无 | **推荐** | https://docs.ambientcg.com/license/ 【核实】 |
| Fab Standard License（含 Megascans；视觉 3D/贴图/动画） | V | 可，"usage is not limited to Unreal Engine"【核实摘要】 | 摘要允许"含入 Project 分发/修改以并入"；禁止独立转售/免费再分发；**可提取性条款未核实（EULA 全文未取得）** | 可；档位按过去 12 个月总商业收入是否超 US$100,000；**捐赠是否计入"收入"未核实，保守按计入**【核实】 | 否【核实】 | 允许与合作者共享；未见席位数限制【核实摘要】 | **推荐（只买 Standard；PENDING 可提取性）** | https://www.fab.com/eula （仅摘要）；https://dev.epicgames.com/documentation/fab/licenses-and-pricing-in-fab ；https://forums.unrealengine.com/t/quixel-to-fab-transition-faqs/2033839.md 【核实/摘要】 |
| Fab（音频类 listing） | A | △ "任意引擎" [S] | △ 不得独立分发 [S] | △ | ？ | ？ | **慎用 / PENDING**（与视觉片段冲突，取保守，见 §3.3） | https://www.fab.com/eula （音频片段未直接读取）[S] |
| Fab UE-Only / Reference-Only / Legacy | V/A | **不可**（UE-Only 定义，来自搜索摘要，全文未逐条读） | — | — | — | — | **禁用** | https://www.unrealengine.com/en-US/eula/content ；https://forums.unrealengine.com/t/fab-ue-only-content-licensing/2082870 【搜索摘要】 |
| Megascans / Megaplants（Fab 上） | V | 可，Standard【核实 FAQ】 | 同 Fab Standard；**旧 Quixel/Bridge 下载的资产受旧许可约束，须从 Fab 重新领取并留存凭证** | 可；2026-11-05 公告恢复永久免费，**本次未逐个打开商品页确认价格为 0** | 否 | 同 Fab | **推荐（免费扫描首选；逐件确认价格=0、许可=Standard、是否 Nanite-only）** | https://forums.unrealengine.com/t/quixel-to-fab-transition-faqs/2033839.md 【核实】；https://www.cgchannel.com/?p=179945 【核实新闻，未逐页核实】 |
| Sketchfab（CC 模型） | V | 视单模型 CC | CC0/CC-BY 可 | CC0/BY 可；NC/ND/SA 禁 | CC-BY 需要 | 无 | **慎用**（仅限可核实原创作者、有制作过程说明的 CC0/CC-BY） | https://sketchfab.com/blogs/community/sketchfab-update-what-you-need-to-know-now-that-fabs-live/ ；https://sketchfab.com/blogs/community/kitbash-acquires-sketchfab-and-artstation ；https://www.gamedeveloper.com/business/epics-sells-artstation-and-sketchfab-to-kitbash 【核实】 |
| Sketchfab Store | V | 已于 2024-10-22 关闭，并入 Fab；已列商品可看不可买 | — | — | — | — | **不存在**（C-47 的 $29.99 价格来源存疑，见 V-VEH-AIR-004） | 同上 【核实】 |
| TurboSquid（Royalty-Free） | V | 不按引擎划分 | **要求专有格式；仅豁免 Unity/Unreal/Lumberyard/Stingray 的 WebGL 导出，"Other open formats are prohibited"（Sec. II.7b/c）；glTF/KTX2 直发属灰区甚至违反**；其他浏览器平台"可个案批准"（官方页：Others may be approved on a case by case basis）| 可；**Editorial 标签仅限新闻类，禁用于游戏**（Sec. II.4） | 未核实 | 搜索摘要称给客户交付须每客户单独许可；细则未核实 | **慎用（须事先书面申请个案批准）** | https://www.turbosquid.com/licensing （首次抓取 403）；https://www.turbosquid.com/help/en/articles/9937422-royalty-free-license 【核实】 |
| CGTrader（Royalty Free License） | V | 不限（摘要） | "incorporated into the product … 3rd party cannot retrieve it on its own"（摘要）；无"专有格式"字面要求；浏览器可提取风险同 §3.4；T&C Sec.21 全文**未能访问** | 可，示例含 video game（摘要） | 未核实 | 单账号购买；团队需 Business 账户；细则未核实 | **慎用（逐件核对商品页许可证栏 + 书面确认）**；"No AI" 标记对我们无影响 | https://help.cgtrader.com/hc/en-us/articles/360015124437 【搜索摘要】 |
| RenderHub | V | 未核实 | 未核实；商品页标 "Extended Use (IP Restricted)"（含义**未核实**） | 未核实 | 未核实（RC Art Muzzle Effects 为 CC-BY 4.0 需署名） | 未核实 | **PENDING**（条款页未读取） | 商品页（WebFetch 空/403）【搜索摘要】；Loss bundle 页 https://www.renderhub.com/loss/world-war-2-german-weapons 【核实价格】 |
| ArtStation Marketplace | V | 各卖家，未核实 | 各卖家 | 各卖家 | — | — | **不作主渠道**（现状未核实；2026-08 KitBash 收购后未提 Marketplace） | https://forums.unrealengine.com/t/further-clarity-regarding-artstation-marketplace-moving-to-fab/2044603 ；https://www.gamedeveloper.com/business/epics-sells-artstation-and-sketchfab-to-kitbash |
| Unity Asset Store | V | 可："not restricted to Unity projects"；排除 Restricted Asset/自带非标 EULA 的商品【核实】 | **禁止独立再分发或让项目用户提取原始文件**；项目主要目的不得是提供资产访问【核实 FAQ】 | 可 | 未核实 | 通例每人购买；**未核实** | **可用（补充渠道）**；交付 .unitypackage，需 Unity/Blender 导出 FBX | https://support.unity.com/hc/en-us/articles/34387186019988-Can-I-use-assets-from-the-Asset-Store-with-other-engines 【核实】 |
| KitBash3D | V | 官方支持 UE/Unity/DCC，其它引擎**未核实** | 禁"独立再分发"；Cargo "Larger Works" 游戏打包**未核实** | Cargo **Professional** 才可商用；买断 Kit 永久许可 | 未核实 | Enterprise 按员工规模 | **慎用/低相关**（无二战诺曼底套件） | https://help.kitbash3d.com/en/articles/7939191-cargo-licensing-faqs ；https://kitbash3d.com/pages/cargo-eula （EULA 全文未读，Last Modified 2024-04-17）【搜索摘要】 |
| Mixamo（Adobe） | V | 无引擎限制（摘要） | 嵌入游戏可；以"资产包"形式分发不可（社区**旧帖**） | 可（含 video games）（摘要） | 不需要（未核实） | 个人 Adobe ID | **可用（M0 占位 / 通用移动）**；2025-06 后部分用户登录/下载故障；仅人形 | https://helpx.adobe.com/creative-cloud/faq/mixamo-faq.html （WebFetch 403，**未能访问**）；https://community.adobe.com/t5/mixamo-discussions/mixamo-faq-licensing-royalties-ownership-eula-and-tos/m-p/13234775 【搜索摘要】 |
| MoCap Online | V | 无引擎限制 | 须"prevents their extraction for further distribution…"（示例：游戏内运行、无法不经逆向取出）；不得转售 | Standard：< 100 万最终用户，且项目预计/达到 US$100 万总收入须改购 Commercial；AI 用途需书面同意；超范围条款含 US$10,000 罚金【搜索摘要】 | 未核实 | 个人；第三方商店购买须同时遵守该店条款 | **推荐（动作首选；须确认浏览器可提取性；超线需谈 Commercial）** | https://mocaponline.com/pages/license 【核实】 |
| Rokoko Motion Library | V | 摘要称可 | 未核实 | 摘要称可（旧信息：150 个免费动作可商用）；EULA 未读取 | 未核实 | — | **备选**（军事类覆盖不足） | https://rokoko.com/pricing/ 【搜索摘要】 |
| Reallusion ActorCore / iClone | V | 须申请 **Mass Distribution License**（免费，需审批；论坛引述） | 未核实；新旧说法冲突，须读 "Licensing FAQ for Game Developers" 附录（未取得） | 动作包 Standard 可商用，角色按"一个角色"计 | 未核实 | — | **慎用** | https://www.reallusion.com/Content/EULA/EULA.htm 【搜索摘要】 |
| Sonniss GameAudioGDC（免费包） | A | △ 授权为"媒体制作"，无引擎限制【核实】 | 可随成品游戏出售；**不得作为音效再分发**（原件和编辑后的都不行）【核实】 | 可，免版税，终身无限项目【核实】；禁 AI/ML 训练 | 无需（"受欢迎"） | 页面未说明 → **PENDING（低风险）**，每位实际操作者各自下载 | **推荐** | https://sonniss.com/gameaudiogdc 【核实】；GDC 2026 包：https://rekkerd.org/sonniss-releases-gdc-2026-game-audio-bundle/ 【核实，第三方转述】；2023 版 EULA 非官方镜像 [S] |
| Freesound — CC0 | A | 可 | 可 | 可 | 无需 | 不限 | **推荐（CC0；逐文件核对）** | https://freesound.org/help/faq/ 【核实】 |
| Freesound — CC-BY | A | 可 | 可 | 可 | **必须**（游戏内 Credits：声音名、作者、链接、许可） | 不限 | **慎用** | 同上 【核实】 |
| Freesound — CC-BY-NC / Sampling+ | A | — | — | **禁止商用** | — | — | **禁用** | 同上 【核实】 |
| BOOM Library | A | △ 含 Games【核实 产品页】 | △ 仅内嵌可，独立再分发✘ [S]（EULA 页 WebFetch 404，仅摘要） | 可；产品仅向商业客户提供；一次性买断、无按项目费用【核实】 | 无需【核实】 | △ 单用户；多人需 multi-user [S] → **PENDING（团队 ≥ 2 人）** | **推荐 / 需书面确认** | https://www.boomlibrary.com/sound-effects/world-war-ii-firearms/ 【核实】；https://www.boomlibrary.com/support/faq/what-am-i-allowed-to-do-with-your-sounds/ 【核实】；https://www.boomlibrary.com/?p=480427 （EULA，404，[S]）；https://www.boomlibrary.com/support/enterprise-licensing/ （存在，内容未读） |
| Pro Sound Effects | A | ？ 页面未说明游戏 | △ 不得给未授权第三方；禁 soundboard、NFT、AI 训练、冥想类 app【核实】 | 终身许可免版税无限商用；订阅期内项目退订后仍受保护 | ？ | 单用户；多用户需定制【核实】 | **PENDING，先发邮件**（曾为 Star Citizen 做定制多用户许可 [S]） | https://www.prosoundeffects.com/licensing/ 【核实】 |
| Soundsnap | A | △ 含游戏，但两档文本互相矛盾 | ✘ 不得原样分发【核实】 | △ 帮助中心两篇 FAQ 矛盾 | 可选 | ？ | **慎用 / PENDING**（许可"可撤销"是否影响已发布游戏：PENDING） | https://www.soundsnap.com/licence 【核实】 |
| A Sound Effect（平台，多厂牌） | A | ✔ 明列 "video games and similar"（第 2.b 条）【核实】 | △ 与其它媒体结合后可；第 3 条禁独立分发/soundboard；"禁止改编、修改或重新包装"措辞极严 → **格式转换/响度标准化/分层混音是否构成 modifying：PENDING** | ✔ | 无需；不得声称所有权 | ✔ 按所购授权数/工作站（发票上） | **推荐 / 需书面确认**（各厂牌细则以结算页与 EULA 为准） | https://asoundeffect.com/license-agreement 【核实】；免费音效 EULA：https://www.asoundeffect.com/wp-content/uploads/2022/12/free_eula22.pdf [S] |
| Sound Ideas（代理 BOOM/Pole Position 等） | A | △ | △ "禁止将内容纳入任何库" [S]；BOOM TOS PDF 抓取 404 | royalty-free，无持续费用【核实 WW2 Tanks 页】 | 无需 [S] | 结算页选 | **推荐（同 A Sound Effect）/ PENDING 风险低** | https://sound-ideas.com |
| Soundly（订阅） | A | ✔ "cleared for commercial use, including video games"【核实 FAQ】 | △ EULA PDF 压缩流无法读出正文 | ✔；订阅期内创建的游戏退订后仍受保护 | ？ | 按席位 [M] | **可用，须读 EULA**；价格页无结果→需人工确认 | https://getsoundly.com/faq/can-i-use-the-sounds-in-a-video-game/ 【核实】；https://getsoundly.com |
| Epidemic Sound | A | △ 营销页称可 | △ 禁"独立使用" [S] | △ | ？ | ？ | **PENDING**（M0–M1 不依赖） | https://www.epidemicsound.com/game-development/music-for-video-games/ 【核实，营销页】 |
| Artlist | A | ✘/？ 仅 Enterprise 明列游戏 [S] | ✘ 禁独立包 [S] | △ | ？ | ？ | **慎用**（不建议经 Artlist 获取游戏音效；Artlist 平台上有 Pole Position/BOOM 的 WW2 单条音效，如 https://artlist.io/sfx/track/artillery---75mm-sherman-tank-single-shot-distant-hit/80654 ，但授权按 Artlist 条款而非原厂牌） | https://help.artlist.io/hc/en-us/articles/33980035053213 [S] |
| Zapsplat | A | △ [S] | △ [S] | △ | **免费账号须署名** [S] | ？ | **慎用（仅 Foley/杂项）** | https://www.zapsplat.com/license-agreement （官方页 403，[S]） |
| BBC Sound Effects（RemArc） | A | — | — | **✘ 仅个人/教育/研究** [S] | 须署名 [S] | — | **禁用** | https://sound-effects.bbcrewind.co.uk/ （工具拒绝抓取，[S]） |
| OpenAIR（IR 库） | A | 许可**逐条录音由贡献者设定**，多为 CC [S] | — | 含 NC 的条目禁用；仅用 CC0/CC-BY（署名）条目 | CC-BY 须署名 | — | **慎用（逐条核对）** | https://www.openair.hosted.york.ac.uk/ （本次 503，**未能核实许可**） |
| itch.io / Gumroad / Shockwave-Sound / Pixabay / SoundBible 等个人/小站素材 | V/A | 各自文本 | 各自文本 | 各自文本（"商用"对"免费+捐赠"的定义需确认） | 如 SoundBible CC-BY 3.0、Fatal Exit CC-BY 4.0 需署名 | — | **PENDING（逐个读许可/邮件询问作者）** | 各商品页，见 §4 |
| Flipped Normals / GameDev Market / Super Hive Market / Hum3D 等其它商店 | V | 未核实 | 未核实 | 未核实 | 未核实 | 未核实 | **PENDING**（本次未研究其条款，只记录了候选商品） | 商品页，见 §4 |
| 开源字体（SIL OFL） | V | 可（OFL） | 可（OFL） | 可 | 不要求，需随附 OFL 文本；有保留字体名限制 | 无 | **可用** | 见 V-FNT-001~004；多数凭记忆/聚合站，**需到字体仓库核实 OFL 文本** |

### 3.3 片段间冲突与取舍（以更保守者为准）

| # | 冲突 | 取舍 |
|---|---|---|
| C1 | Fab Standard：视觉片段"推荐（EULA 摘要读到）"，音频片段"慎用 / PENDING（EULA 未读）" | 3D/贴图/动画类按"推荐，但许可证栏必须显示 Standard、可提取性 PENDING"；**音频类 Fab listing 按"慎用/PENDING"**，并要求每个 listing 单独核实"可用于非 Unreal 引擎" |
| C2 | F-AMB-002（海滩海浪）：音频片段表内里程碑列写 M2，备注写"M4 才需要" | Omaha 属 M4，本清单按 **M4** 采购（§4.5），保守不提前花钱 |
| C3 | 视觉片段把"自有二进制容器 + 简单混淆/加密"作为缓解；音频片段同样建议自有容器 + 剥离元数据 | 统一为总纲 **D-PAK**：`.fpak` 分块容器（不是 DRM，诚实说明见 §3.4） |
| C4 | 音频片段 A.5.3 提到可选 WASM Opus 解码器兜底 | 以总纲 **D-AUD** 为准：Opus（WebM）为主，AAC 兜底，启动时用 `decodeAudioData` 实测探测；WASM 解码器不作为默认方案（提示留给架构文档） |
| C5 | 视觉片段把 Unity 商店/CGTrader/Mixamo 评为"黄/可用"，TurboSquid 为"橙" | 按 D-LIC：CGTrader、TurboSquid、Unity、MoCap Online 的付费素材**一律先拿书面确认**；TurboSquid 另需个案批准 |

### 3.4 "浏览器分发可提取性"问题与合规做法

**问题**：Frontline 是浏览器游戏，下发到玩家浏览器的资源（glTF/KTX2/Opus 等）玩家可从 DevTools/缓存里下载。TurboSquid、CGTrader（Royalty Free）、MoCap Online、Unity Asset Store 的条款都要求"素材必须嵌入产品且第三方无法单独提取"；音频渠道（A Sound Effect、Sonniss、BOOM、Artlist、Soundsnap）则共同禁止"以独立可提取形式再分发/放入库、素材包、音效板"。**本次检查的所有渠道文本中，没有任何一家直接讨论"网页游戏资源会落在玩家浏览器缓存里"这一场景**。

**诚实结论**：浏览器游戏天然做不到"无法提取"，只能做到"不以通用可导入格式、不以单文件资产形式提供，并证明无意分发"。这是**合规层面的"非独立文件分发"，不是 DRM，技术上无法阻止有决心的提取**。是否接受剩余风险由负责人决定（§7.3）。行业惯例（[M]，未核实）：任何游戏的本地安装包同样可被提取，厂商一般把"作品中的合理保护措施"视为足够——只要没有提供单独下载入口、没有把素材当作独立资源宣传。

**合规做法（D-PAK / D-AUD）**：

1. **`.fpak` 容器**：`/tools/asset-pipeline` 把 Meshopt 网格 + KTX2 贴图 + 音频打包进自有分块容器 `.fpak`（按 64 m 区块/类别分包，带索引，可 HTTP Range 流式读取）。**不以标准 `.glb/.ktx2/.ogg` 独立文件形式暴露**；使用哈希命名，不使用原始文件名。
2. 音频：下发 Opus/AAC 压缩数据（不是原始 WAV），多个音效合并（sprite/每武器一个包）；**剔除 BWAV/iXML/Soundminer 等元数据**；源 WAV 永不进入公开 CDN，只上传编码后的包。
3. **不提供任何"素材浏览/下载/试听"入口**；不做音效板；不开放 mod API 直接引用原素材（若日后开放 mod，参考 Pro Sound Effects 为 Star Citizen 做定制许可的先例 [S]，需单独授权）。
4. **向卖家/厂商发邮件取得书面确认**（模板 §6），保存邮件，登记进 `LICENSES.md`。把"拿到卖家确认"列为每个付费素材的采购完成条件（D-LIC）。
5. 优先 CC0 与 Fab Standard 来源（D-LIC 首选）。
6. Credits 页列出所有需署名素材（`LICENSES.md` §4）。
7. 在 `LICENSES.md` 和发布流程里记录"证明无意分发"的这些措施。

**书面确认邮件要点（完整英文模板见 §6）**：①项目性质：浏览器内运行、免费游玩、接受捐赠的多人射击游戏；②资源会以转换后格式、打包进自有容器随游戏下发，玩家在 DevTools/缓存中能看到包文件，请确认符合许可；③我们会做的加工：格式转换、LOD/减面、贴图压缩、重定向、响度标准化、剪辑、距离分层混音、卷积混响；④我们不会：提供独立下载入口、做音效板、开放 mod 引用原素材；⑤授权数量：1 名人类负责人 + 自动化执行者（AI 代理在同一机器上处理文件）；⑥项目将来出现捐赠奖励/变付费时是否需调整；⑦（订阅类）取消订阅后已发布游戏是否仍合规；⑧请求书面（邮件）确认。

### 3.5 风险分级速览

| 级别 | 渠道 |
|---|---|
| 绿 | Poly Haven、ambientCG（CC0）；Freesound CC0；Sonniss 免费包 |
| 黄绿 | Fab Standard（含 Megascans）、MoCap Online（用户/收入阈值）、BOOM、A Sound Effect、Sound Ideas |
| 黄 | Unity Asset Store、CGTrader RF、Mixamo、Soundly、Freesound CC-BY |
| 橙 | TurboSquid（专有格式明文）、ActorCore（需审批）、KitBash3D、Soundsnap、Pro Sound Effects、Epidemic、Artlist、Zapsplat、Fab 音频 |
| 红/禁 | Fab UE-Only/Reference-Only/Legacy、任何 Editorial、CC-BY-NC/ND/SA、BBC RemArc、AI 生成站、一切游戏提取资源 |

---
## 4. 按里程碑分批的采购清单

### 4.0 读表约定

- 里程碑含义（总纲 §5）：M0 技术验证（占位即可）；M1 垂直切片；M2 完整 Frontline；M3 全兵种全载具；M4 Omaha/Foy/Skirmish/优化。
- **分批规则**：每个需求 ID 归入"必须换成真素材"的最早里程碑；片段标注"M0 占位 / M1 正式"者归入 M1，在备注里写明 M0 可占位（占位约定见 `docs/ASSET_GAPS.md` §3）。
- 面数档位（设计建议【待调参】）：第一人称武器 25k–60k（D-FP，带内部机构与 4K）；第三人称武器 5k–12k；角色 25k–45k（LOD0）；中型载具 60k–150k 外壳（内饰另计）；建筑模块 5k–30k/件；树 5k–15k + impostor。
- **所有候选都未下载检查实际网格**；"面数"字样来自商品页或摘要，质量需购前看预览或向卖家要样图。
- 每批内分类：地图环境 / 兵种角色 / 武器 / 动画 / 载具 / 音效 / 其它。某类在该批无需求时省略。
- 音频规格默认：源文件 ≥ 24-bit / 48 kHz（枪械/爆炸/炮声首选 96 kHz 源，最终下发 48 kHz）；枪械距离分层 近（≤ 5 m）/ 中（≈ 30–100 m）/ 远（≥ 200 m），每层 ≥ 3–4 变体。
- 表列：`需求 ID｜规格｜候选｜推荐｜许可证状态｜选定候选｜已购`。"选定候选"与"已购"由负责人填写。

---

### 4.1 M0 必买（技术验证：单块地形、10 名 bot、步枪射击、网络同步）

**M0 总原则**：能免费的先免费；必买的只有 Kar98k 枪模（Fab 德军包）与 M1 Garand/Kar98 枪声（Sonniss）；角色/动画用 Mixamo 或入门动画占位。M0 小计见 §5.1。

#### 4.1.1 地图环境

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-NAT-007 | 地表材质：泥地、车辙、积雪、耕地/犁沟、石板路/鹅卵石、草地、碎石路、混凝土、焦土；PBR（含置换），1K–8K | **免费**：Poly Haven 与 ambientCG 全部 CC0（https://polyhaven.com/license ；https://docs.ambientcg.com/license/ ）；Megascans 材质库（免费，Fab Standard，须逐件确认价格=0）。完全满足 | 全部用免费来源；Megascans 须先截图留证 | OK（CC0）；Megascans 项：PENDING（价格=0 与 Standard 待逐件确认） | | [ ] 已购 |
| V-SKY-001 | HDRI 天空：晴、多云、阴、黄昏、雾晨 | **免费**：Poly Haven HDRIs（CC0，1K–16K）https://polyhaven.com/hdris （站点存在 HDRI 库，具体文件名未列）；ambientCG 也有 HDRI | 免费 | OK（CC0） | | [ ] 已购 |

#### 4.1.2 兵种角色

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| （无专属 ID；视觉片段 §3 汇总：M0 需 1 个绑定人形，任一） | 标准人形骨骼；可为通用占位 | **免费**：Mixamo 自带角色（条款见 §3.2；官方 FAQ 403 未能读取）；M1 正式角色见 §4.2.2 | Mixamo 占位；**不作为最终军事角色** | PENDING（Mixamo 官方 FAQ 未能访问） | | [ ] 已购 |

#### 4.1.3 武器

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-WPN-GE-001 | Kar98k（M0/M1）：栓动：栓柄、枪机运动；5 发 stripper clip | **免费**：无。**付费**：① **FPS World War Guns Pack – German**｜Raouf Benoudina｜https://www.fab.com/listings/13b4dea1-2849-480a-a3c7-b16fa5bd329a ｜**$19.99【核实】**｜FBX/OBJ/glTF/GLB/USDZ/Blender；2K 贴图；Kar98k **2.6k 顶点**、StG44 3.8k、MP40 3.6k、Luger P08 2k；每枪 1 材质｜**品质偏低，第三人称/中距离**｜许可证页未显示名称（Fab 默认 Standard 待确认）② **Mauser 98k FPS Rifle**｜MyNameIsVoo｜https://forums.unrealengine.com/t/mynameisvoo-mauser-98k-fps-rifle/2478140 ｜价格/许可/格式**未核实**，是否 UE-Only 须查 ③ Unity "Animated WW2 Characters & Weapons Collection" https://marketplace.unity.com/packages/3d/characters/animated-ww2-characters-weapons-collection-259992 （含 BA98 栓动步枪动画；价格需人工确认） | **①**（M0 步枪；同包还含 M1 要用的 MP40/StG44/Luger） | PENDING（Fab 许可证栏需确认=Standard；可提取性未核实 → 建议同发邮件，§6.1） | | [ ] 已购 |
| （采购顺序提示，非新需求） | 视觉片段 §5.3 建议 M0 同时买 **FPS World War Guns Pack – United States**（见 V-WPN-US-003 行，$19.99【核实】），合计 $39.98 | 见 §4.2.3 V-WPN-US-003 | 与 V-WPN-GE-001 同时下单 | 同上 | | [ ] 已购 |

#### 4.1.4 动画

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-ANM-001（M0 入门部分；M1 完整集见 §4.2.4） | 占位与基础走/跑/蹲/跳、待机、倒地、翻越、基础投掷 | **免费**：Mixamo（通用运动，占位）；**MoCap Online "Free Pistol Animation Starter Pack"** https://mocaponline.itch.io/free-pistol-animation-starter-pack （免费，非步枪）。**付费**：**MoCap Online Rifle Starter $3.99【搜索摘要】** https://mocaponline.itch.io/rifle-starter-anims （含走/慢跑/蹲姿 + 瞄准） | Mixamo + Rifle Starter $3.99 | Mixamo：PENDING；MoCap Online：需书面确认（防提取要求） | | [ ] 已购 |

> Mixamo 够用到什么程度：够 M0 占位、第三人称基础动作；不够的有——步枪持握（数量有限，未逐个核对）、8 向 + 瞄准偏移、匍匐持枪、换弹/栓动/救治/建造/修理、载具乘员、第一人称武器动画（完全没有）。2025-06 后有登录/下载故障的社区报告。**不要把 Mixamo 作为唯一来源。**

#### 4.1.5 音效

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| （无需求 ID：通用免费来源） | Sonniss GameAudioGDC 2015–2024 + 2026 全部下载（约 250 GB+，由执行者下载并按关键词建索引） | https://sonniss.com/gameaudiogdc 【核实】；GDC 2026 包（约 7.47 GB+，2026-03-16 发布，17 家厂牌）https://rekkerd.org/sonniss-releases-gdc-2026-game-audio-bundle/ 。页面称**没有 GDC 2025 包**。免费，无需署名，终身无限项目；**不得作为音效再分发**；禁 AI/ML 训练 | 全部领取；每位实际操作者各自下载 | OK（席位 PENDING，低风险） | | [ ] 已购 |
| A-WPN-001 | M1 Garand：近/中/远开火 + **弹夹弹出 "ping"** + 拉栓/上膛/换夹；≥4 变体/层；24/48+ | **免费**：Freesound/Pixabay 有零散 M1 Garand 与 ping 剪辑（Pixabay 条款与 SoundBible CC-BY 3.0 需逐条核实）[S]；**GAP：未找到专业级 CC0**。**付费**：① **Weapons of World War II – Designed**｜Sonniss / A Sound Effect（发布者 superthump2 / "Super Thumps"）｜https://sonniss.com/sound-effects/weapons-world-war-ii-designed/ ｜**$38.50**（原价 $55）[F]｜96 kHz/24-bit 立体声；133 变体；18 文件（约 350 MB）；含空仓 "Ping!"；多距离但页面未给具体距离 ② **BOOM World War II Firearms**（Designed / Construction Kit / Bundle）https://www.boomlibrary.com/sound-effects/world-war-ii-firearms/ ｜**$139–$269**（各版价格未分列；Adorama 列 Designed $129 [S]）[F] ③ **GCRadix Firearm Foley & Mechanics: WW2** https://www.asoundeffect.com/sound-library/firearm-foley-mechanics-ww2/ ｜**$119.40**（页面结构化数据，原价 $199）[F]｜机械动作。Diegetix 的 Garand 仅 10 条，不够 | **M0 首选 ①**（便宜，Garand + Kar98 + Thompson 等 14 枪一次到位） | 需书面确认（A Sound Effect/Sonniss 付费库；"modifying"措辞与可提取性） | | [ ] 已购 |
| A-WPN-009 | M0 Kar98k：开火近/中/远 + 拉栓/装弹夹 | **付费**：Sonniss WWII Designed（同上，$38.50）；BOOM WWII Firearms；Diegetix 德包（46 条）https://www.asoundeffect.com/sound-library/ww2-weapons-german-us/ ｜$25–$40（页面区间，德包/美包/合包）[F]｜**48 kHz**/24-bit；384 声音/257 MB；GCRadix | **与 A-WPN-001 同一个 Designed 库**（M0 与 Garand 并行需要） | 需书面确认 | | [ ] 已购 |
| C-BUL-001 | M0 子弹掠过 whizz / 超音速 crack（近、中，左右立体） | **免费**：Sonniss GDC [M]。**付费**：① A Sound Effect "Bullet Bys & Impacts"｜$29.99｜157 文件，96 kHz/24 立体声，UCS（含 whizz-by 与撞击，**未说明是否实弹录制**）https://www.asoundeffect.com/?p=414565 [S] ② LEGIT Audio "Hunting Slingshot"（itch.io，弹弓，非枪弹，**不合适**）[S] | **PENDING：先看文件清单确认是实枪录制；若不是 → GAP-A04 邻近缺口（暂缓购买）** | PENDING | | [ ] 已购 |
| E-FTS-001 | M0 军靴脚步：泥、草、石板/碎石、木地板、雪、沙、浅水；走/跑/蹲/落地；**脚步是 100 人同屏最高频声音，每表面 ≥ 8 变体** | **免费**：Freesound CC0（逐条核实）；Sonniss GDC [M]。**付费**：A Sound Effect 脚步类目 https://asoundeffect.com/sound-category/foley/footsteps （含军靴/负重步兵的游戏用库）[S]；Sonniss "Ultimate Footstep Collection" https://sonniss.com/product/ultimate-footstep-collection [S，价格人工确认]；Sound Ideas Foley Footsteps https://portal.sound-ideas.com/Product/442/Footsteps-Sound-Effects-Library [S] | 先用免费 CC0 + GDC；付费视试听 | Freesound CC0：OK；付费：需书面确认 | | [ ] 已购 |
| H-UI-001 | M0 UI 点击/确认/拒绝/倒计时 | **免费**：Sonniss GDC；Freesound CC0；Kenney 等 CC0 包 [M]。**付费**：任一 Foley/UI 库 | 免费 | OK（逐条核对 CC0） | | [ ] 已购 |

#### 4.1.6 其它（字体；片段未标里程碑，按"首个 UI 任务"归入 M0，可调整）

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-FNT-001 | 标题字体（次要标题） | **Black Ops One**（Sorkin Type）https://www.fontsquirrel.com/fonts/black-ops-one ｜SIL OFL（Font Squirrel 页面列出，**保留字体名 "Black Ops"**）。军事模板字体，**风格接近商业射击游戏常见字体，避免与参考作品相同的视觉识别**（总纲 §1：字体需原创表达）；如用，仅用于次要标题 | 慎选 | OK（下载时存档 OFL 文本） | | [ ] 已购 |
| V-FNT-002 | 模板风格标题 | **Saira Stencil One**（Omnibus-Type）https://fonts.google.com/specimen/Saira+Stencil+One/about ｜SIL OFL 1.1（聚合站标注，**需到仓库核实**） | 备选 | PENDING | | [ ] 已购 |
| V-FNT-003 | 模板风格标题 | **Allerta Stencil**（Matt McInerney）https://fontsource.org/fonts/allerta-stencil/about ｜SIL OFL（RFN "Allerta"） | 备选 | OK（存档 OFL 文本；注意 RFN） | | [ ] 已购 |
| V-FNT-004 | 正文/数字等宽 + 简体中文方案 | 推荐 Inter / Roboto Condensed / IBM Plex Sans Condensed（均为 OFL；**本次未逐个核实许可页，凭记忆，未核实**）；简体中文 **Noto Sans SC / Source Han Sans（OFL，凭记忆，未核实）**，注意中文字体包体积（建议子集化） | 到字体仓库核实后采用 | PENDING | | [ ] 已购 |
| （图标备注，非需求 ID） | UI 图标 | 总纲规定"图标将原创绘制"：由执行者按 `UI_SPEC` 以 SVG/Canvas 代码绘制，属 UI 代码，**不视为"模型/音效自制"——待负责人确认（NEEDS_HUMAN）**。若不接受，可选开源图标集（Material Symbols/Lucide 等，**Apache-2.0/ISC，凭记忆，未核实**），但军事/战术类图标需要原创，通用图标集难以满足 → GAP-V16 | 待你确认 | PENDING | | |

---

### 4.2 M1 垂直切片（Carentan 的 1/3、20 名真人、5 个兵种、1 辆坦克、Garrison + 补给）

**M1 视觉必须到位**（片段 §3 汇总）：美军 Thompson/BAR/Garand/M1911 + 德军 Kar98k/MP40/StG44/MG42；美/德夏季步兵各 1；1 辆坦克（Sherman 或 Panzer IV）；Jeep；沙袋/木箱/铁丝；树篱与树；至少 1 套民居（或占位盒体）；枪口焰/烟/爆炸。允许占位：坦克内饰、第一人称高精度。

#### 4.2.1 地图环境

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-BLD-001 | 诺曼底石造民居模块套件（墙/门窗/屋顶/山墙/烟囱，完好 + 受损版）20–40 个模块，2K–4K 模块化 PBR | **免费**：Megascans 有砖/石/灰泥材质（免费，见 V-NAT-007）可来自拼；无整套建筑。**付费**：**未发现诺曼底专用套件**。搜索只返回中世纪/奇幻村庄套件：CGTrader "medieval village asset modular game ready pbr low poly" https://www.cgtrader.com/3d-models/architectural/architectural-street/medieval-village-asset-modular-game-ready-pbr-low-poly ；CGTrader "Modular Village Kit"（无链接）；Fab 77 件中世纪套件 https://www.fab.com/listings/f26e2dfc-0dba-49c5-ba12-9015b28497c8 （UE5 为主，许可需核）。风格不符（木骨架/童话），不推荐 | **→ GAP-V02**（M1 先用占位盒体 + 免费材质） | — | | [ ] 已购 |
| V-BLD-002 | 农场建筑（谷仓、牲畜棚、苹果园棚） | 无 | **→ GAP-V02** | — | | [ ] 已购 |
| V-BLD-003 | 乡村教堂（Carentan 圣母院式钟楼，可破坏）M1/M2 | 无 | **→ GAP-V02** | — | | [ ] 已购 |
| V-NAT-001 | 树篱 bocage（土堤 + 灌木，可通行/半通行，可作掩体，多种宽度） | **免费**：未发现扫描版；Megascans 灌木/草/土堤/石墙散件可拼（需人工检索，**未核实具体资产名**）。**付费**：只找到 UK hedgerow 植物包（$4–$40【搜索摘要】）：https://www.cgtrader.com/3d-models/plant/leaf/uk-hedgerow-3d-collection ；https://www.turbosquid.com/3d-models/3d-british-hedgerow-plants-3d-asset-collection-model-2442125 ；**无真正的 bocage 土堤** | **→ GAP-V03**（Carentan 战术核心；植物包 + Megascans 土堤材质组合为替代） | CGTrader/TurboSquid 付费项：需书面确认 | | [ ] 已购 |
| V-NAT-002 | 诺曼底落叶乔木（橡、榉、苹果树） | **免费**：Megascans Trees（欧洲山毛榉 17 件、欧洲橡树等）https://www.fab.com/listings/d11cc01d-9422-41b7-950f-416c9ce79caf ；**许可是否为 Standard 待核（早期 Marketplace 版标 UE-Only）；是否免费待核；Nanite 版对 Babylon 不可用，需非 Nanite 网格**；Poly Haven 有若干树模型（Fir/Pine/Island Tree 01 等，CC0，**未核实是否扫描**）https://polyhaven.com/ 。**付费**：CGTrader/RenderHub 通用树包（本次未深查） | 视 Megascans 核实结果；否则 Poly Haven | PENDING（Megascans 逐件核实）；Poly Haven：OK | | [ ] 已购 |
| V-NAT-004 | 草、灌木、蕨、农作物（麦、玉米）、苹果园（M1–M2） | **免费**：Megascans/Megaplants（须核）；Poly Haven 有若干植物扫描；ambientCG 无植被模型（只有材质）。**付费**：通用植被包（本次未深查）；**农作物为 GAP-V14** | 免费来源 | PENDING | | [ ] 已购 |
| V-NAT-005 | 岩石、悬崖、海崖（M1/M4） | **免费**：Poly Haven 岩石模型；Megascans 岩石（Standard，须核） | 免费 | Poly Haven：OK；Megascans：PENDING | | [ ] 已购 |
| V-PRP-001 | 沙袋（墙、角、射击孔） | CGTrader "sandbag pack 12 modular game ready assets with 4k pbr support" https://www.cgtrader.com/3d-models/military/other/sandbag-pack-12-modular-game-ready-assets-with-4k-pbr-support （价格需确认） | 采用 | 需书面确认 | | [ ] 已购 |
| V-PRP-002 | 铁丝网（蛇腹、桩、带刺） | Sketchfab "Barbed Wire Fence WWII"（约 1.1k 三角，3 LOD，PBR）https://sketchfab.com/3d-models/barbed-wire-fence-wwii-242e75d10a1f416fa8519f2d1a5d7c3c （**许可未核，须检查作者来源，见 §2.3**） | 核实许可后采用 | PENDING | | [ ] 已购 |
| V-PRP-003 | 反坦克障碍（龙牙、刺猬）（M1/M4） | 见 V-BLD-007（§4.5）；"Military Trenches and Modular World War Compound"｜Sierra Division｜Fab｜**$179.99【核实】**｜仅 Unreal Engine 格式，含战壕模块、沙袋、铁丝网、梯、掩体、炮火/烟雾特效｜**格式仅为 UE，需人工确认能否导出为通用格式 → 倾向不买** https://www.fab.com/listings/ba4a316f-a363-4c2f-a776-26082681b91b | 不买该套件 | PENDING | | [ ] 已购 |
| V-PRP-004 | 弹药箱/补给箱（含美式/德式） | CGTrader 综合包：沙袋、铁丝、弹药箱、捷克刺猬、木补给箱（4K PBR，Royalty Free）；WW2 Battlefield Props Pack（Unreal Marketplace 56 件，**已不在 Fab 售，仅 Vault**）https://www.unrealengine.com/marketplace/en-US/product/ww2-battlefield-props-pack ；其许可可能为 UE-Only，**不建议** | CGTrader 综合包（价格/链接需人工确认） | 需书面确认 | | [ ] 已购 |
| V-PRP-006 | 木箱、木桶、油桶、农用工具 | **免费**：**Poly Haven**（Wooden Crate 01/02、Wooden Barrels 01，CC0）https://polyhaven.com/a/wooden_crate_02 | 免费 | OK（CC0） | | [ ] 已购 |

#### 4.2.2 兵种角色

> 通用说明：搜到的角色多为 A-Pose 静态模型或"rigged"（骨骼质量不明）；没有现成冬季着装与兵种差异化套件。推荐策略：购买 1 个高质量基础身体 + 多套服装/装具模块，在 Blender 里组合（"组装现成素材"，需负责人确认 D-MOD）。

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-CHR-US-001 | 美军步兵（夏季，M41/M1943 野战服、M1 头盔）25k–45k 面 LOD0，4K PBR，标准人形骨骼（含手指）、面部不必 | **付费**：① **SOLDIER WWII USA Paratrooper Rigged**｜albin｜RenderHub https://www.renderhub.com/albin/soldier-wwii-usa-paratrooper ｜**$60【搜索摘要】**｜CGTrader 版约 47.9k 多边形；描述"mix of several references"（历史准确性待查）② **WW2 US Soldier**｜TurboSquid https://www.turbosquid.com/3d-models/3d-army-man-model-1593497 ｜**$59.99【搜索摘要】**｜Game-Ready PBR 低模｜**TurboSquid 专有格式条款风险** ③ **Alpenwolf WW2 US Soldier**｜RenderHub https://www.renderhub.com/alpenwolf/ww2-us-soldier ｜约 46.3k 多边形，PBR + rigged【搜索摘要】④ **Fully Equipped American Soldier A-Pose**｜deep3dstudio｜https://www.renderhub.com/deep3dstudio/fully-equipped-american-soldier-in-a-pose-317 （未绑定，需自行绑定） | ① 或 ③（带绑定） | RenderHub：PENDING；TurboSquid：需书面确认（个案批准） | | [ ] 已购 |
| V-CHR-US-002 | 美军空降兵（M42 跳伞服、头盔网） | CGTrader "ww2 us paratrooper and thompson m1a1"（衣服类）https://www.cgtrader.com/3d-models/character/clothing/ww2-us-paratrooper-and-thompson-m1a1 ；albin 的 Paratrooper（见上）；RenderHub "101st Airborne Division Soldier"（中面数，PBR，**未标 rigged**，列表 https://www.renderhub.com/pbr-models/characters/soldiers ，价格需确认） | albin Paratrooper | 需书面确认 | | [ ] 已购 |
| V-CHR-GE-001 | 德军国防军（夏季，M43 野战服、M35/M42 钢盔） | **付费**：① **German Soldier Rig – WW2 Scanned Asset Pack**｜CGTrader https://www.cgtrader.com/3d-models/character/clothing/german-soldier-rig-ww2-scanned-asset-pack （亦在 Sketchfab https://skfb.ly/6UwUt ）｜重拓扑、4K PBR、标准人形骨骼含扭转骨；glTF/OBJ/USDZ【搜索摘要】；价格需确认；**"scanned"，来源应为真实扫描，需核对说明** ② **Wehrmacht Soldier - RIGGED ANIMATED**｜CGTrader https://cgtrader.com/3d-models/military/military-character/wehrmacht-soldier-rigged-animated ｜价格需确认 ③ **WWII German Soldier**（2015，4 套贴图，rigged）https://cgtrader.com/3d-models/character/man/wwii-german-soldier （老，需检查拓扑）④ **deep3dstudio German Wehrmacht Soldier**（A-Pose，PBR，未绑定）https://www.renderhub.com/deep3dstudio/german-wehrmacht-soldier-ready-for-animation-423 | ①（价格按 $40–80 估【估算】） | 需书面确认 | | [ ] 已购 |
| V-CHR-FPA-001 | 第一人称手臂（美军袖/德军袖）+ 手套；8k–15k 面，与武器共绑定骨骼 | 未发现 WW2 专用。Fab 上有现代 FPS 手臂 + 动画包（见 V-ANM-009）→ **近似 GAP（GAP-V19）** | 待 GAP 决定 | — | | [ ] 已购 |

#### 4.2.3 武器

> 通用说明：**Fab 两个 FPS World War Guns Pack（$19.99×2，【核实】）** 覆盖大部分主枪，**RenderHub Loss 德军步兵武器 bundle（$70，【核实】）** https://www.renderhub.com/loss/world-war-2-german-weapons 覆盖 G43/MG34/MG42/MP40/FG42（带绑定；单件价见各行）。但 Fab 包顶点数 2k–5.7k，只够第三人称；第一人称需另买高精度单件（D-FP：25k–60k）。建议 M1 先用第三人称品质 + 粗糙第一人称视角，M2 起补充高精度（品质妥协，需负责人决定）。

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-WPN-US-001 | M1 Garand（M0 占位 / M1 正式）：第一人称 25k–60k，含 en-bloc 弹夹与枪机；PBR 4K；可动部件：枪机、扳机、保险、弹夹（"ping"空夹需弹夹单独对象） | **免费**：无。**付费**：① **M1 Garand - Game and VR Ready**｜CGTrader｜https://cgtrader.com/3d-models/military/gun/m1-garand-game-and-vr-ready ｜**$25【搜索摘要】**｜PNG PBR；描述"first-person functional"｜型号吻合度需看图 ② **M1 Garand With Complete Internals - Game Ready**｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/m1-garand-with-complete-internals-game-ready ｜价格需人工确认｜带内部结构，适合第一人称 ③ **Lowpoly M1 Garand Rifle**｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/lowpoly-m1-garand-rifle ｜**$59【搜索摘要】**｜约 3.7k 面，2K PBR，第三人称品质。另见 https://superhivemarket.com/products/m1-garand-gameready （25k 三角形，7 张 PBR 贴图，价格需人工确认） | ① 或 ②（第一人称向） | 需书面确认（CGTrader RF；Super Hive 条款未核实 → PENDING） | | [ ] 已购 |
| V-WPN-US-003 | M1A1 Thompson：含 50 发鼓/20/30 发弹匣；全自动 | ① **FPS World War Guns Pack – United States**｜Raouf Benoudina｜https://www.fab.com/listings/00a8235f-055a-4532-9916-fe060949b9eb ｜**$19.99【核实】**｜Blender/FBX/OBJ；贴图 1K/2K/4K AO/Albedo/Height/Normal/Roughness/Metallic；Thompson **5.7k 顶点**；M1911 2.5k、M1903 3k、BAR 4k｜许可证页未显示名称（Fab 默认 Standard 待确认）｜型号标注为 "Thompson M1A1" ② CGTrader "thompson m1a1 low poly" https://www.cgtrader.com/3d-models/military/gun/thompson-m1a1-low-poly （价格需确认） | ① | PENDING（Fab 许可证栏需确认） | | [ ] 已购 |
| V-WPN-US-005 | M1918A2 BAR：二脚架、20 发弹匣、枪托 | 同上 Fab 美军包（BAR 4k 顶点，**标注 "BAR M1918"，非 A2 版，需人工核对二脚架/肩垫**，$19.99【核实】） | 同包 | PENDING | | [ ] 已购 |
| V-WPN-US-008 | M1911A1：弹匣、套筒 | Fab 美军包 M1911 2.5k 顶点【核实】 | 同包 | PENDING | | [ ] 已购 |
| V-WPN-US-010 | Mk2 手雷（小物件 1–2k） | 未发现 Mk2 专用商品 → **GAP-V09**（小件，可顺带向任一卖家询价） | — | — | | [ ] 已购 |
| V-WPN-GE-004 | MP40：折叠枪托 | Fab 德军包 MP40 3.6k 顶点【核实】（见 V-WPN-GE-001）；**Loss MP40 Rigged $15【核实】**，4,470 多边形，GLB 可用 https://www.renderhub.com/loss/world-war-2-german-weapons | 第三人称用 Fab 包；第一人称考虑 Loss | Fab：PENDING；Loss：PENDING（"Extended Use (IP Restricted)" 含义未核实） | | [ ] 已购 |
| V-WPN-GE-005 | StG 44（M1/M2）：弧形弹匣 | Fab 德军包 StG44 3.8k 顶点【核实】；CGTrader "WWII Weapons Collection" https://cgtrader.com/3d-model-collections/wwii-weapons-collection （含 StG-44 与 Kar98k，价格需确认）。**Loss FG42 Rigged（$15）**为伞兵步枪，可作 FJR 特色武器 | Fab 德军包 | PENDING | | [ ] 已购 |
| V-WPN-GE-007 | MG42（M1/M2）：弹链、两脚架、可折叠 | **Loss "MG42 Rigged"｜RenderHub｜$20【核实】｜7,288 多边形** https://www.renderhub.com/loss/world-war-2-german-weapons | Loss MG42 | PENDING（"Extended Use (IP Restricted)" 含义未核实）→ 需书面确认 | | [ ] 已购 |
| V-WPN-GE-008 | Luger P08 / Walther P38：手枪 | Fab 德军包 Luger P08 2k 顶点【核实】；P38 未发现 → **P38 归 GAP-V09**（可用 P08 代替，二者都在役） | Fab 德军包 | PENDING | | [ ] 已购 |
| V-WPN-GE-011 | M24 手雷（Stielhandgranate）：小件 | CGTrader "WW2 German Anti-Tank Grenade PBR"（**$5.99【搜索摘要，CGTrader 版】**）或 TurboSquid "WW2 Stielhandgranate" https://www.turbosquid.com/3d-models/pbr-stielhandgranate-grenade-3d-model-1404112 （**TurboSquid 条款见 §3.2**） | CGTrader 版 | 需书面确认 | | [ ] 已购 |

#### 4.2.4 动画

> 推荐付费动捕包：**MoCap Online（Rifle Pro + HipFire 等；明确列了步枪走/跑/蹲/8 向）为首选**；加 kevdev Human Soldier Animations 补匍匐。动作需重定向到统一骨架（建议在资源管线里固定 Mixamo/Humanoid 命名骨架，做自动重定向工具）。

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-ANM-001（M1 完整集） | 第三人称步枪运动集：站/蹲 × 走/跑/冲刺 × 8 向，瞄准偏移，射击/换弹层；原地+根运动两套；30 FPS | **免费**：Mixamo；MoCap Online Free Pistol https://mocaponline.itch.io/free-pistol-animation-starter-pack 。**付费**：① **MoCap Online Rifle Pro**（Fab/官网）；入门版 Rifle Starter $3.99【搜索摘要】https://mocaponline.itch.io/rifle-starter-anims ；Fab 列表 https://www.fab.com/listings/b6cf3b34-2f4f-427d-a488-44460acbbe25 、https://www.fab.com/listings/95dbdb60-d5df-4694-bfc6-3c22d6b79f72 （价格需确认，许可见 §3.2 MoCap Online）② Fab "Crouch Rifle Animation \| Vicon Mocap"（Gamma Studio，113 个蹲姿步枪动画含瞄准偏移，https://forums.unrealengine.com/t/gamma-studio-crouch-rifle-animation-vicon-mocap/2840709 ；价格未核实）③ Fab "Male Rifle Tense Movement"（93 条，8 向走/慢跑；无链接） | ① | 需书面确认（MoCap Online 防提取；Fab 须非 UE-Only） | | [ ] 已购 |
| V-ANM-002 | 匍匐/趴姿（持枪、转向、射击、换弹） | ① **Human Soldier Animations**｜kevdev｜itch.io｜**$30【搜索摘要】**｜https://kevdev.itch.io/human-soldier-animations ：含蹲走 8 向与匍匐前后 1 m/s、原地与根运动 ② Reallusion iClone "Rifle Combat Prone" 动作包（iClone 格式，需导出；**许可见 ActorCore 条款**）https://reallusion.com/contentstore/iclone/pack/iclone7-motion-rifle-combat-prone/default.html | ① | ①：PENDING（许可未读）；②：需书面确认 | | [ ] 已购 |
| V-ANM-004 | 投掷（手雷）、扔烟；单手/低手 | **免费**：Mixamo。付费：**GAP（军事版）**→ GAP-V17 | Mixamo 通用 | PENDING | | [ ] 已购 |
| V-ANM-005 | 受击/倒地/死亡（多方向）/布娃娃衔接 | **免费**：Mixamo。付费：MoCap Online 通用包；GAP 部分 → GAP-V17 | Mixamo + MoCap Online | PENDING / 需书面确认 | | [ ] 已购 |
| V-ANM-008 | 载具进出/驾驶/炮手/乘员（M1 1 辆坦克 – M3），与载具骨骼点吻合 | **GAP**（需自制或外包）→ **GAP-V08** | 通用动作 + 手动对位 | — | | [ ] 已购 |
| V-ANM-009 | 第一人称（M1 起）：步枪（站/冲刺/开镜）、栓动全流程（开栓、抛壳、上弹、关栓）、换弹（空/未空）、近战、投掷、救治手持；与 FP 手臂+武器骨骼共绑定 | **近似 GAP**：Fab "First Person Rifle Animations V2"（现代战术，UE5 骨架；无链接）与带栓动、检查枪膛、两种空弹换弹的现代狙击包 https://www.fab.com/listings/3f24e5a1-8c3e-4bc3-8b59-b8191befd4eb 、https://www.fab.com/listings/57e4a0d3-d8fe-4d4c-ab55-6cae93f2ff53 （**均为现代枪，价格/许可未核实**）；Unity WW2 动画包含 Kar98k 栓动（见 V-WPN-GE-001 ③）→ **GAP-V07** | 暂以现代包 + 重定向（观感偏现代） | PENDING | | [ ] 已购 |

#### 4.2.5 载具

> D 类总评：车辆"完整 + 可动 + 内饰"在现成素材里基本找不到；面数波动大（5k 到 60 万）；多数卖家的"rigged"只是炮塔/履带/轮分层，不是"物理驱动用"层级，需在管线中重新整理（需负责人确认可接受）。**M1 坦克内饰 → GAP-V01（GAP #1）**。

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-VEH-US-003 | M4 Sherman（75mm）— **M1 的 1 辆坦克** | ① **Sherman M4 Gameready Rigged**｜CGTrader｜https://www.cgtrader.com/3d-models/military/military-vehicle/sherman-m4-gameready-rigged ｜**$31【搜索摘要】**｜履带/轮随车体动，PBR 4K，Royalty Free；**内饰未提** ② **WW2 American Tank M4 Sherman**｜CGTrader｜https://www.cgtrader.com/3d-models/military/military-vehicle/ww2-american-tank-m4-sherman ｜$50【搜索摘要】，PBR + rigged，**62.3 万多边形，需大幅减面** ③ TurboSquid Sherman M4 PBR game ready https://www.turbosquid.com/pt_br/3d-models/3d-sherman-m4-wwii-tank-3d-model-pbr-textures-model-2509965 ｜$14【搜索摘要】，**Editorial 许可，禁用**。另见 RenderHub Panaristi 系列（M4A2/M4A3 约 40 万面，非 PBR 标注） | ① | 需书面确认 | | [ ] 已购 |
| V-VEH-US-007 | Willys Jeep（M1/M3） | CGTrader "ww2 jeep usa"（低模 PBR，4K）https://www.cgtrader.com/3d-models/military/military-vehicle/ww2-jeep-usa ；"ww2 american willys jeep" https://www.cgtrader.com/3d-models/vehicle/military-vehicle/ww2-american-willys-jeep ；价格需确认（按 $20–50 估【估算】） | 任选其一，看预览 | 需书面确认 | | [ ] 已购 |
| V-VEH-GE-003 | Panzer IV（Ausf. H/J 为 1944 主力；M1/M3，可选，同一阶段对手） | ① **Panzer IV Production models**｜EnnuiShao｜Fab https://www.fab.com/listings/690cc828-da71-4778-9433-894a7c0d99f5 ｜**$25.99【核实】**｜Blender/FBX/OBJ/GLB/glTF/USDZ；含 Ausf. B/C/D、F1、G、H/J；"Gameready"；**面数/绑定/内饰均未提供**；许可证页未显示名称 ② CGTrader "Pz Kpfw IV Tank Low-Poly"（约 11.6k 面，车身/炮塔/履带三网格，rigged PBR）https://cgtrader.com/3d-models/military/vehicle/pz-kpfw-iv-tank-low-poly ；价格需确认 | ①（先确认 Standard） | PENDING | | [ ] 已购 |

#### 4.2.6 音效

> 枪械 Bundle 建议：M0 买 Sonniss WWII Designed（$38.50）即可开工；M1 追加 Pole Position MG42 + GCRadix；M2 视预算买 BOOM Bundle（拿 30 路源做近/中/远分层，**只有 Construction Kit 含多路源**）。
> 核心库规格见 §4.2.6 末尾"核心库速查"。

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| A-WPN-002 | M1 Carbine | Sonniss WWII Designed；BOOM WWII Firearms（两库都含） | 已含于 A-WPN-001 的 Designed | 需书面确认 | | [ ] 已购 |
| A-WPN-003 | Thompson（M1928A1/M1A1）：单发/点射/全自动 | **免费**：Sonniss GDC 里**可能**有，需解包后搜索 [M]。**付费**：Sonniss WWII Designed；BOOM；Diegetix US 包（166 条含 700/800 RPM）；GCRadix | Designed + Diegetix（便宜） | 需书面确认 | | [ ] 已购 |
| A-WPN-004 | M3 Grease Gun | Sonniss WWII Designed；BOOM | 已含于 Designed | 需书面确认 | | [ ] 已购 |
| A-WPN-005 | BAR（M1918） | Sonniss WWII Designed（仅单发）。**BOOM 清单无 BAR**；全自动 BAR 需从 M1919/Bren 变调替代 → **GAP-A02** | Designed（单发）+ GAP | 需书面确认 | | [ ] 已购 |
| A-WPN-006 | M1919 Browning（点射/全自动） | Sonniss WWII Designed；SoundFX Studio Part III https://soundfx.studio/shop/sfx-for-games/weapons/wwii-weapons-sound-fx-library-part-iii/ [S，价格人工确认] | 已含于 Designed | 需书面确认 | | [ ] 已购 |
| A-WPN-007 | M1903 Springfield | Sonniss WWII Designed（仅单发）；拉栓声见 GCRadix 的 Kar98K/Mosin 拉栓可作替代参考（**不是**同一枪） | 已含于 Designed | 需书面确认 | | [ ] 已购 |
| A-WPN-008 | M1911 | Sonniss WWII Designed；Diegetix US 包（27 条，含混响版本 V01–V03）；GCRadix | 已含于 Designed | 需书面确认 | | [ ] 已购 |
| A-WPN-010 | G43/G41 | Sonniss WWII Designed（G43）；GCRadix（K43、G43）；BOOM 仅 Gewehr 41（**注意 BOOM 清单是 G41 非 G43**） | 已含于 Designed | 需书面确认 | | [ ] 已购 |
| A-WPN-011 | MP40 | Sonniss；BOOM；Diegetix；GCRadix | 已含于 Designed | 需书面确认 | | [ ] 已购 |
| A-WPN-014 | **MG42**：单发（点射）/全自动；近/中/远；长/短尾音；操作声 | ① **Pole Position — MG 42 Machine Gun**｜Sonniss｜https://sonniss.com/product/mg-42-machine-gun/ ｜**$111.75**（原价 $149）[F]｜96 kHz/24-bit；183 文件/4.81 GB；24 视角/44 声道；**两个地点录制，含长/短尾音**；含操作声；含 Pro Tools/Reaper 工程；**实枪 MG42（7.92×57）** ② BOOM WWII Firearms ③ Sonniss WWII Designed（仅点射/全自动，无单发） | **强烈推荐 ①** | 需书面确认 | | [ ] 已购 |
| A-WPN-015 | P38 / Luger P08 | Sonniss；BOOM（P38）；Diegetix（P38）；GCRadix（P38、Luger） | 已含于 Designed | 需书面确认 | | [ ] 已购 |
| A-WPN-016 | 尾音/混响分层：室外开阔、城镇、森林各一套尾音/反射 | **付费**：Pole Position MG42（两地点长短尾音）；BOOM（30 路含远麦）；The Warfare 2 Library（22 视角含碉堡内）。**室外/森林/城镇尾音多数库没有独立提供**，需用 J 类 IR 对近层干声做卷积（"允许的后期"）；远层用 BOOM Construction Kit 的 100 m Ambisonics 麦 | 先以 A-WPN-014 + J-IR-001 实现 | 需书面确认 | | [ ] 已购 |
| A-WPN-017 | 机械动作（拉栓/换弹/上膛/干击/保险/收枪） | **免费**：Freesound 零散（逐条 CC0）。**付费**：**GCRadix Firearm Foley $119.40（最全）**；BOOM Construction Kit；Sonniss（仅部分） | GCRadix | 需书面确认 | | [ ] 已购 |
| B-EXP-001 | 手雷爆炸（近/中/远，室外/室内） | **免费**：Sonniss GDC 包可能含（解包搜索）[M]。**付费**：① **The Warfare 2 Library**｜Pole Position｜**$199**｜627 声音，真实军事演习；含手雷、阔剑地雷、人员地雷、车辆地雷、反坦克地雷、C-4、炮弹，木/石爆裂+碎屑；24/96；22 视角：近/中/远+碉堡内 37 外 4 内声道｜https://www.asoundeffect.com/sound-library/the-warfare-2-library [F] ② A Sound Effect "The Warfare 3 / Heavy Bombardment"｜$199，140 声音/17.29 GB，发射点+落点同步录制｜[S，URL `https://www.asoundeffect.com/?p=760476` 为搜索所得，需核实] | **M1 必买 ①** | 需书面确认 | | [ ] 已购 |
| C-BUL-002 | 各材质命中（泥、混凝土、金属、木、沙袋、水） | **免费**：Sonniss GDC [M]；Freesound CC0 [M]。**付费**：Bullet Bys & Impacts（见 C-BUL-001，PENDING）；Warfare 2（金属板命中） | 免费 + Warfare 2 | 需书面确认 / Freesound CC0 OK | | [ ] 已购 |
| C-BUL-003 | 跳弹（ricochet） | 同 C-BUL-002 | 同上 | 同上 | | [ ] 已购 |
| C-BUL-004 | 击中人体（友/敌，身体/头部） | **GAP**（专业实录几乎不存在；行业用西瓜/肉类 Foley，属二次录制）→ **GAP-A04** | 暂用沙袋/泥土命中声叠加（见 GAPS） | — | | [ ] 已购 |
| C-BUL-005 | 压制感（近距离多弹掠过 + 低频冲击 + 耳鸣） | 用 C-BUL-001 + AudioWorklet 低通/高通（**程序处理，不属素材**）；耳鸣高频声需 Sonniss GDC 或购买 | 免费 | OK（Sonniss GDC） | | [ ] 已购 |
| D-TNK-001 | Sherman 发动机怠速/加速多转速循环（外部近/远、车内） | ① **Pole Position — Sherman M4A3**｜Sonniss｜https://sonniss.com/product/sherman-m4a3-medium-tank ｜**$224.25** [F]｜96 kHz/24-bit；183 文件/25.47 GB；16 车外 + 12 车内声道；**Ford GAA V8**；含同步元素、Altiverb IR（扫频于坦克内部）② **Pole Position — Sherman M4A1**｜https://sonniss.com/product/sherman-m4a1-medium-tank ｜**$224.25**（原价 $299）[F]｜249 文件/27.99 GB；**Continental R975 星型汽油发动机**；75 mm 炮另售 ③ **BOOM / Pole Position — World War 2 Tanks**｜https://sound-ideas.com/Product/787/World-War-2-Tanks-Sound-Effects-by-Boom （Sound Ideas $248，原价 $309）、https://www.asoundeffect.com/sound-library/world-war-2-tanks （$247.20）[F]｜400+ 声音/约 27 GB；8 种：B1bis、Hetzer、M24 Chaffee、M4A1 Sherman、Panther、Somua S35、T-34、Valentine | **M1 只买一辆：推荐 ① M4A3**（现役最多，Ford GAA）；M4A1 可在 M3 补；**不要混用不同发动机的循环** | 需书面确认 | | [ ] 已购 |
| D-TNK-002 | 履带声（碾压/转向） | 同 D-TNK-001（"tracks" 视角；多数库里是车载麦的一路） | 已含 | 需书面确认 | | [ ] 已购 |
| D-TNK-003 | 炮塔转动（电/液压） | **GAP/PENDING**：M4A3 页未写；BOOM WW2 Tanks "handling sounds" [F] 可能含；Panther 库含"turret"标签 [F]。购买前看文件列表 → **GAP-A14** | 看清单后定 | PENDING | | [ ] 已购 |
| D-TNK-004 | 主炮发射（内/外视角）+ 装填 | **Pole Position — Sherman Cannon 75mm M4A1**｜https://int.test-env.asoundeffect.com/sound-library/sherman-cannon-75mm-m4a1 [S，**测试环境域名，正式页未核实**] / A Sound Effect 页｜**$149** [S]｜含近/中/远 + wild foley，空包录制｜Sonniss 另有 "75mm Sherman M4A1 Cannon"（249 声音/27.99 GB，疑为同一/相邻产品，**PENDING**）；BOOM WW2 Tanks（含炮击）；Panther（炮管麦） | **M1 买 Sherman Cannon**（德军 75/88mm 到 M3 再说） | 需书面确认；产品页 PENDING | | [ ] 已购 |
| E-GER-001 | 装备晃动（背包、弹药袋、水壶、头盔、枪背带）、卧倒/匍匐、翻越 | **免费**：Sonniss GDC [M]。**付费**：Sound Ideas Ultimate Foley Collection（2,300 Foley，含军人/背包/穿越灌木）https://portal.sound-ideas.com/Product/446/Foley-Collection [S]；Fab 战术装备 Foley（现代装备）https://www.fab.com/listings/4616f81b-935f-4220-bb53-1f9abf532ebb [S，WW2 适配度需判断]；GCRadix Firearm Foley（含背带、衣物、背包摩擦 [F]）。**匍匐/卧倒专用库：GAP-A08** | GCRadix + GDC | 需书面确认 | | [ ] 已购 |
| F-AMB-001 | 诺曼底乡村日/夜（鸟、虫、风、远处牛羊）；立体声/5.1 床 | **免费**：Sonniss GDC 环境类 [M]；Freesound CC0。**付费**：A Sound Effect 环境类目（自然/乡村，多通道 Ambisonics/5.1）https://www.asoundeffect.com/sound-category/environments/nature/ [S]；**Normandy 专用库：GAP-A11**（法国西部乡村"温带欧洲"可通用替代） | 免费 | OK（逐条核对 CC0） | | [ ] 已购 |
| F-AMB-003 | 风、雨 | Freesound CC0 / Sonniss GDC | 免费 | OK | | [ ] 已购 |
| F-AMB-005 | 远处战场交火/炮击床 | 用 B/A 类远层混成：Warfare 2/3 的远层；Pole Position 榴弹炮远层（至 150 m） | 复用已购 | 需书面确认 | | [ ] 已购 |
| F-AMB-006 | 建筑内部房间音（用于卷积/混响湿声底噪） | **免费**：Freesound CC0。**付费**：A Sound Effect 房间调（roomtones）类目 [S] | 免费 | OK | | [ ] 已购 |
| G-VOX-001 | 中枪/倒地/痛呼/死亡（英语男声） | A Sound Effect "British Soldier Voices"（800+，命令/喊叫/呻吟，**英式**）https://www.asoundeffect.com/sound-library/british-soldier-voices [S]；Raketenfaust "Army Voice Pack"（itch.io，$4.49 起，155 条 WW2 士兵语音，专业配音）https://raketenfaust.itch.io/army-voice-pack [S，许可需读]。美式英语库：**未找到明确标注"美式 WW2"的专业库 → GAP-A12** | 英式包 + 口音差异需负责人接受 | PENDING | | [ ] 已购 |
| G-VOX-002 | 德语战场喊话/痛呼 | Shockwave-Sound "SonicArchive Military Personnel German"｜$30｜46 条，16-bit/44.1 kHz｜https://shockwave-sound.com/sound-effects/12396/sonicarchive-military-personnel-german [S]。**规格偏低；条数少（46），重复使用易穿帮；是否含 WW2 术语以试听为准** | 试听后定 | PENDING | | [ ] 已购 |
| H-RAD-001 | PTT 开/闭声、静电、对讲噪声（**年代感**） | **免费**：Sonniss GDC [M]。**付费**：A Sound Effect "Squelch"（6 台电台/对讲机/CB 实录）[S]；Sonniss "Military Radio Voices"（425 条现代军事无线电语音）https://sonniss.com/product/military-radio-voices [S]；MuseHub 老式电台静电（1940–60s 风味，许可需读）https://www.musehub.com/loops-and-audio/radios-and-tvs [S]。**WW2 时期无线电实录：GAP-A10** | Squelch + 带通/压缩（允许的后期） | PENDING | | [ ] 已购 |
| H-UI-002 | 复活提示/占点 sting/补给到达等 HUD 提示（UI 提示音） | 同 H-UI-001 | 免费 | OK | | [ ] 已购 |
| I-MUS-001 | 主菜单管弦主题（60–120 s 可循环，≥ 320 kbps 母带或无损） | **免费**：Fatal Exit "After The War"（CC-BY 4.0，需署名）https://fatalexit.itch.io/after-the-war [S，奇幻向，是否适合 WW2 需试听]。**付费**：Daniel Carl "Top Secret WW2 Music Pack"（itch.io，14 首+循环，"WW2 游戏用"，许可需读）https://danielcarlmusic.itch.io/top-secret-ww2-music-pack [S]；Bobby Cole "Warfare Soundtrack"（25 首管弦，无限免版税）https://bobbycolemusic.itch.io/warfare-spundtrack-25-tracks-vol-1 [S，URL 如搜索所返]；Redearth "Battle Litany"（13 首军事管弦，免版税商用）https://redearthmusic.itch.io/battle-litany [S]；Shockwave-Sound 军事/战争类 https://www.Shockwave-Sound.com/browse/production-type/military-war/stock-music/ [S] | 各读许可；需确认"免费 + 捐赠"是否算商用，必要时邮件询问作者；可委托创作见 GAP-A13 | PENDING | | [ ] 已购 |
| J-IR-001 | 卷积混响 IR：室外开阔/城镇/森林/室内（单声道或 FOA，48 kHz，0.5–4 s） | **免费**：**OpenAIR**（约克大学）https://www.openair.hosted.york.ac.uk/ （本次 503，**未能核实许可**）；搜索摘要称许可逐条由贡献者设定，多为 Creative Commons，**需逐条检查 NC/ND 标记** [S]。**付费**：A Sound Effect/Sonniss 提供的 IR 库（本次未专门检索）；**Pole Position Sherman M4A3 附带 Altiverb 坦克内部 IR** [F]。**含 NC 的条目禁用**；仅用 CC0/CC-BY（署名）；EchoThief 等零散库许可需另查（[S]，未读）。TAU 空间 IR 库禁止商用，禁用 | OpenAIR 逐条 + M4A3 附带 IR | PENDING | | [ ] 已购 |

**核心库速查（音频，价格 2026-11-17 页面所见）**

| 库 | URL | 价格 | 规格 | 覆盖 |
|---|---|---|---|---|
| BOOM World War II Firearms（Designed / Construction Kit / Bundle） | https://www.boomlibrary.com/sound-effects/world-war-ii-firearms/ [F] | $139–$269（各版未分列；Adorama Designed $129 [S]） | 24-bit/96 kHz 立体声；Designed 197 文件/1,030 声音/2 GB；Kit 1,013 文件/8,148 声音/23.9 GB；30 路麦克风多轨，**仅 Kit 含 30 路源**；每枪 10 次发射；含单发/点射/全自动、换弹、上膛、干击、两脚架；声称一处 100 m 的 Ambisonics 远麦 | 25 种：Thompson M1928A1、M3、Sten、MP40、MP28、PPS43、PPD40、MG42、FG42、Bren、DP27、P38、PPK、TT33、StG44、M1 Garand、M1 Carbine、Gewehr 41、SVT40、AVS-36、Kar98K、G33、Mosin M1938、M30 Drilling |
| Weapons of World War II – Designed | https://sonniss.com/sound-effects/weapons-world-war-ii-designed/ [F] | $38.50（原价 $55） | 96 kHz/24-bit 立体声；133 变体；18 文件（约 350 MB） | 14 种：M1919、M1911、BAR M1918、G43、M3、Kar98、Luger、M1 Carbine、M1 Garand（含空仓 "Ping!"）、MG42、MP40、P38、M1 Thompson、M1903 Springfield 狙击 |
| Diegetix — WW2 Weapons: German & US | https://www.asoundeffect.com/sound-library/ww2-weapons-german-us/ [F] | $25–$40（页面区间） | 48 kHz/24-bit；384 声音/257 MB；无距离分层 | StG44、P38、Kar98K、MP40、Colt 1911、M1 Garand（仅 10 条）、Thompson A1（166 条含 700/800 RPM） |
| GCRadix — Firearm Foley & Mechanics: WW2 | https://www.asoundeffect.com/sound-library/firearm-foley-mechanics-ww2/ [F] | $119.40（结构化数据，原价 $199） | 96 kHz/24-bit；4,011 声音/468 文件/5.28 GB；UCS 命名 | 14 种（MP40、Thompson M1A1、K43、M1 Garand、Mosin、G43、Kar98K、P38、Luger P08、M1911、StG44 等），拉栓/换弹/上膛/干击/背带/收枪/衣物 Foley |
| Pole Position — MG 42 Machine Gun | https://sonniss.com/product/mg-42-machine-gun/ [F] | $111.75（原价 $149） | 96 kHz/24-bit；183 文件/4.81 GB；24 视角/44 声道 | 实枪 MG42 |
| SoundFX Studio — WWII Weapons Part III | https://soundfx.studio/shop/sfx-for-games/weapons/wwii-weapons-sound-fx-library-part-iii/ [S] | 人工确认 | 有 Construction Kit 与 Designed | M1 Garand、M1919、M1911 等 |
| Sound Ideas — WW2 Firearms Bundle（BOOM 分销） | https://sound-ideas.com/Product/1311/World-War-2-Firearms-Sound-Effects-Bundle [S] | 人工确认 | — | 同 BOOM 25 种 |
| Pole Position — Panzer IV Ausf. G | https://sonniss.com/sound-effects/panzer-iv-ausf/ [F] | $224.25 | 96 kHz/24-bit；252 文件/11.18 GB；Maybach HL 120 TRM | 德军 M3 配（页面**未写**是否含主炮） |
| Pole Position — Panther Ausf. A (1943) | https://sonniss.com/product/panther-ausf-1943 [F] | $224.25 | 96 kHz/24-bit；252 文件/19.93 GB；25 路同步通道；Maybach HL230 P30 | 德军 M3 配；含炮/炮塔标签 |
| Pole Position — 105mm Howitzer Cannon | https://www.asoundeffect.com/sound-library/howitzer-cannon-105mm [F] | $149 | 27 声音，24/96，0–150 m 近/中/远，空包弹录制 | M3；页面未写型号，仅称"WWII 105mm 榴弹炮"，**不要照抄 M101 写法** |

> 注意：**Tiger 型没有找到**（只有 Tiger II：A Sound Effect Tiger II 合集，16 路车载，[S]）。迈巴赫 HL 系列（HL120 TRM、HL230 P30）都有真实录音。

#### 4.2.7 其它（特效贴图）

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-FX-001 | 枪口焰贴图（步枪/冲锋枪/机枪/火炮，不同形状） | **免费**：无合格免费发现。**付费**：RenderHub "RC Art - Muzzle Effects"（**CC-BY 4.0，需署名**）https://www.renderhub.com/reactorcore/rc-art-muzzle-effects ；Unity Asset Store 一件含 15 种武器特效 + 56 张弹孔 https://assetstore.unity.com/packages/slug/169018 （价格/许可需确认） | RC Art（需署名） | OK（CC-BY 4.0，记入 Credits；条款页以商品页为准，PENDING 核对）；Unity：PENDING | | [ ] 已购 |
| V-FX-002 | 烟雾（手雷烟、坦克炮口烟、烟囱、火烧）flipbook | Fab "37 sprite sheet effects" 包（**royalty-free 商用、无需署名、禁转售**，【搜索摘要】）https://fab.com/s/cc7e3fda26c2 ；价格需确认 | 采用 | PENDING | | [ ] 已购 |
| V-FX-003 | 爆炸/火球/尘土/碎片 flipbook（含大、中、小） | ducvu Gumroad "2D FX Explosion Pack"（含 Studio License 商业权）https://ducvu.gumroad.com/l/2d-fx-explosion-pack ；风格偏**风格化**，写实度需看图；sincpc itch.io 同类（无链接） | 看图后定 | PENDING | | [ ] 已购 |
| V-FX-004 | 弹孔贴花（砖/木/金属/泥/雪/混凝土） | **免费**：ambientCG/Poly Haven **无弹孔贴花**；可由其 PBR 材质 + 开孔 alpha 自行做 → **属自制**。**付费**：Unity 一件含 56 弹孔（见 V-FX-001）；Fab "Realistic Bullet Impact" 包（弹孔贴花+弹壳+枪口焰，Niagara 构建，**是 UE 资源，贴图可抽取，许可需核**）https://www.fab.com/listings/11f357a1-ca41-4c10-a7a9-f7a28d2a7e21 | 需决定 | PENDING | | [ ] 已购 |
| V-FX-006 | 曳光弹/弹道线/地面尘土/弹壳 | 弹壳属小模型，可从武器包拆（Fab 美/德包均为 FBX 单件，可能无弹壳） | 视武器包内容 | PENDING | | [ ] 已购 |

---
### 4.3 M2 完整 Frontline 模式（Carentan 完整地图、100 人压测）

**M2 视觉必须到位**（片段 §3）：Carentan 全部建筑种类、更多兵种装备、手雷与烟雾、火焰/弹孔/血迹。M2 起补充高精度第一人称武器（D-FP）。

#### 4.3.1 地图环境

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-BLD-004 | 城镇街道模块（连排房、路缘、路灯、橱窗） | **GAP**（无候选）→ **GAP-V02** | 外包或替代 | — | | [ ] 已购 |
| V-BLD-008 | 桥梁（石拱桥、简易木桥、桁架桥） | **GAP**（本次未搜，建议后续补搜）→ **GAP-V13** | 补搜 | — | | [ ] 已购 |
| V-BLD-010 | 废墟/受损建筑件（碎砖、断墙、屋顶塌陷），可作损毁状态 | **GAP**；CGTrader "Realistic WWII Battlefield and Wasteland Wreckage Collection" https://www.cgtrader.com/3d-model-packs/realistic-wwii-battlefield-and-wasteland-wreckage-collection （价格需确认，**含残骸/废墟，风格需看图**） | 看图后定 | 需书面确认 | | [ ] 已购 |
| V-PRP-005 | 野战无线电（SCR-300/536/Torn.Fu.） | **未发现 → GAP-V12** | — | — | | [ ] 已购 |
| V-PRP-007 | 军用帐篷、伪装网 | 通用军营包：CGTrader "3dt military base modular pack" https://www.cgtrader.com/3d-models/military/other/3dt-military-base-modular-pack （风格需看图，价格需确认） | 看图后定 | 需书面确认 | | [ ] 已购 |
| V-PRP-008 | 散兵坑/战壕（可挖掘模块） | RenderHub "Lowpoly Modular War Trenches Kit"（CGTrader 同名）https://www.renderhub.com/hend-z/lowpoly-modular-war-trenches-kit （低模，不匹配 AAA）；Unity "World War Trenches" https://marketplace.unity.com/packages/3d/environments/historic/world-war-trenches-152381 （一战风格）。写实战壕 → 部分 GAP；**建议由地形系统实现散兵坑** | 地形系统 | PENDING | | [ ] 已购 |
| V-PRP-009 | 电线杆、电线、围栏（木栅、石墙、篱笆） | Megascans/Poly Haven 通用 + **GAP（欧式乡村电线杆）** | 免费通用 + GAP | Poly Haven：OK；Megascans：PENDING | | [ ] 已购 |
| V-PRP-010 | 残骸：被击毁坦克（Sherman/Panzer IV）、卡车、滑翔机（M2–M3） | 在载具模型基础上做烧毁材质/贴花；CGTrader "Realistic WWII Battlefield and Wasteland Wreckage Collection"（见 V-BLD-010） | 复用载具 | 需书面确认 | | [ ] 已购 |

#### 4.3.2 兵种角色

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-CHR-US-003 | 美军兵种变体：军官/医护/坦克兵/狙击手（M2–M3）：头盔涂装、红十字袖标、皮夹克、毡帽 | **GAP**，需用基础身体 + 配件拼装 → **GAP-V18** | 拼装（需确认 D-MOD） | — | | [ ] 已购 |

#### 4.3.3 武器

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-WPN-US-002 | M1 Carbine：第三人称 6k + 第一人称 25k+；15/30 发弹匣 | CGTrader "weapon ww2 low poly"（含 M1 Carbine + Thompson M1）https://www.cgtrader.com/3d-models/military/gun/weapon-ww2-low-poly ｜价格需确认；Fab "WW2 Weapons" 系列中的美军包**未发现 M1 Carbine** → 偏 GAP | CGTrader 该包 | 需书面确认 | | [ ] 已购 |
| V-WPN-US-007 | M1903A4 Springfield（带 M73 镜） | Fab 美军包含 "M1903 Springfield"（3k 顶点，**是否 A4/带镜待核**，$19.99【核实】） | 同包（M1 已买） | PENDING | | [ ] 已购 |
| V-WPN-US-011 | M18 烟雾手雷 | 未发现 → **GAP-V09** | — | — | | [ ] 已购 |
| V-WPN-US-012 | M1 刺刀/M3 刺刀（近战） | 未发现 → **GAP-V09** | — | — | | [ ] 已购 |
| V-WPN-GE-010 | Panzerfaust（30/60/100 型） | **免费**：CGTrader 上有 **WW2 Panzerfaust 低模作者 Lord_akaal 标注免费** https://www.cgtrader.com/3d-models/military/gun/panzerfaust-ww2 （【搜索摘要】，"2K PBR game ready"；**免费且许可证需核对**）。**付费**：① Lord_akaal "Panzerfaust With Box"（RenderHub https://www.renderhub.com/lord-akaal/panzerfaust-with-box-low-poly-3d-model ；CGTrader 版 **$20【搜索摘要】** 4K PBR）② Fab 36 件 PBR 军械包（含 Stielhandgranate 41 与多款 Panzerfaust）https://www.fab.com/listings/1773c3e2-c7e2-4928-8aea-57f5e9d5f2f9 （**本次打开返回 404，需人工确认仍上架**） | ① | 需书面确认；免费版 PENDING（核对许可证） | | [ ] 已购 |

#### 4.3.4 动画

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-ANM-003 | 翻越矮墙/攀爬/跳跃/落地 | **免费**：Mixamo（通用）。付费：**GAP（军事版需再搜）**→ GAP-V17 | Mixamo | PENDING | | [ ] 已购 |
| V-ANM-006 | 救治/拖拽伤员/复活（医护兵动作） | **GAP** → GAP-V17 | — | — | | [ ] 已购 |
| V-ANM-007 | 建造/挖掘/修理（工程兵：锤、铲） | **免费**：Mixamo（部分通用）。付费 GAP → GAP-V17 | Mixamo | PENDING | | [ ] 已购 |

#### 4.3.5 音效

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| A-WPN-012 | StG 44 | **付费**：BOOM WWII Firearms（$139–$269）；Diegetix 德包（54 条）；GCRadix。**Sonniss 14 枪清单不含 StG44** | 视预算：**M2 买 BOOM Bundle（$269）**拿 30 路源做近/中/远分层 | 需书面确认 | | [ ] 已购 |
| A-WPN-013 | **MG34** | **GAP**（BOOM/Sonniss/Diegetix/GCRadix 清单均无 MG34）→ **GAP-A01**；最接近替代：MG42 同弹种，射速低，音色更低沉，仅混音/变调（**不是重新录音**，需人工接受史实偏差） | 替代 MG42（待决定） | — | | [ ] 已购 |
| G-VOX-003 | 战场人群 walla（远处喊叫/冲锋） | A Sound Effect "Historical Battle Voices Vol. 1"（~$33）[S]；Artlist battle-shouts pack https://artlist.io/sfx/pack/battle-shouts/4948 [S，Artlist 许可风险见 §3.2]。语言构成不明；**真人配音录制属"重新录音"，需人工决定** | 暂缓 | PENDING | | [ ] 已购 |
| G-VOX-004 | 指挥官/军官口令（英、德，短句：Fire/Take cover/Medic） | **GAP**（可授权的、系统化的美军/德军战术喊话库未找到）→ **GAP-A09** | — | — | | [ ] 已购 |
| I-MUS-002 | 结算（胜/负）短曲 | 同 I-MUS-001 的候选（见 §4.2.6） | 与 I-MUS-001 同一作者包 | PENDING | | [ ] 已购 |
| J-IR-002 | 远距离炮声用"户外大空间"IR（长尾音 3–6 s） | 同 J-IR-001；可借 Pole Position 多库的远距离麦直接取代部分 IR 需求 | 复用 | PENDING | | [ ] 已购 |

#### 4.3.6 其它

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-FX-005 | 血迹/飞溅/拖痕贴花 | 同 V-FX-004，未专门搜索 → **GAP-V15**（建议后补搜）；估算 US$30–100【估算】 | 补搜 | — | | [ ] 已购 |

---

### 4.4 M3 全兵种、全载具、Assault 模式、指挥官能力、火炮

#### 4.4.1 地图环境

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-BLD-009 | 火车站（小站站房 + 站台） | **GAP**（本次未搜）→ **GAP-V13** | 补搜 | — | | [ ] 已购 |
| V-SKY-003 | 云层/体积云贴图 | 需另行评估；Babylon 内置或程序生成（程序化不属于本清单） | 评估 | — | | [ ] 已购 |

#### 4.4.2 兵种角色

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-CHR-GE-002 | 德军伞兵 FJR（伞兵钢盔、迷彩罩衫） | **GAP**（未发现 Fallschirmjäger 角色；Loss FG42 可作武器配套）→ **GAP-V05** | 国防军角色 + 伞兵钢盔/罩衫（贴图变体，需确认 D-MOD） | — | | [ ] 已购 |
| V-CHR-GE-004 | 德军坦克兵（黑色短夹克）/ 军官 | **GAP**；"WW2 Axis Uniform German"（服装，rigged PBR）https://www.cgtrader.com/3d-models/character/clothing/ww2-axis-uniform-german 可做底 → **GAP-V18** | 拼装 | 需书面确认 | | [ ] 已购 |

#### 4.4.3 武器

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-WPN-US-004 | M3 Grease Gun | 未在搜索中发现专用商品 → **GAP-V09** | — | — | | [ ] 已购 |
| V-WPN-US-006 | M1919A6：带枪托/二脚架型；弹链动画 | 未发现 → **GAP-V09** | — | — | | [ ] 已购 |
| V-WPN-US-009 | M1A1 Bazooka（5.5k+ 面，火箭弹） | CGTrader "Anti Tank Rocket Launcher Collection Low-poly PBR" https://www.cgtrader.com/3d-model-collections/anti-tank-rocket-launcher-collection-low-poly-pbr ｜含 M1A1 Bazooka（约 5.5k 面）与 Panzerschreck（约 6.2k 面），4096 PBR，Royalty Free｜价格需人工确认 | 该合集（同时覆盖 V-WPN-GE-009） | 需书面确认 | | [ ] 已购 |
| V-WPN-GE-002 | Kar98k + ZF41 镜（狙击型） | 未发现专用 → **GAP-V09**（可由第一人称高精度 Kar98k 卖家加购） | — | — | | [ ] 已购 |
| V-WPN-GE-003 | Gewehr 43：半自动，10 发弹匣 | **Loss "WW2 Gewehr 43 Walther G43 Rigged"**｜RenderHub｜https://www.renderhub.com/loss/ww2-gewehr-43-walther-g43 ｜**$15【核实，bundle 页】**｜4,368 多边形/4,814 顶点｜OBJ/FBX/Blend/GLB/USD｜**许可 "Extended Use (IP Restricted)"（含义未核实）**｜已绑定 | Loss（或买 $70 bundle：https://www.renderhub.com/loss/world-war-2-german-weapons ） | PENDING → 需书面确认 | | [ ] 已购 |
| V-WPN-GE-006 | MG34：两脚架/弹鼓 | **Loss "MG34 Maschinengewehr"｜RenderHub｜$20【核实】｜12,149 多边形｜GLB/FBX 等** https://www.renderhub.com/loss/world-war-2-german-weapons | Loss | PENDING → 需书面确认 | | [ ] 已购 |
| V-WPN-GE-009 | Panzerschreck：带防护板 | CGTrader 合集（见 V-WPN-US-009）；https://www.cgtrader.com/3d-models/military/gun/panzerschreck-german-anti-tank-rocket-launcher （Royalty Free No AI，价格需确认） | 同上 | 需书面确认 | | [ ] 已购 |
| V-WPN-GE-012 | 德军烟雾手雷（Nebelhandgranate 39） | 未发现 → **GAP-V09** | — | — | | [ ] 已购 |
| V-WPN-GE-013 | Tellermine 35/42：反坦克地雷 | 未发现专用商品 → **GAP-V09** | — | — | | [ ] 已购 |
| V-WPN-GE-014 | S-Mine（Schrapnellmine 35）：跳雷 | 未发现 → **GAP-V09** | — | — | | [ ] 已购 |

#### 4.4.4 动画

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-ANM-010 | 指挥官手势/敬礼/标记动作 | **免费**：Mixamo（部分）。付费 GAP → GAP-V17 | Mixamo | PENDING | | [ ] 已购 |

#### 4.4.5 载具

**D1. 美军**

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-VEH-US-001 | M8 Greyhound | ① CGTrader "m8 greyhound game ready"（85k tris，含高模烘焙）https://www.cgtrader.com/3d-models/vehicle/military-vehicle/m8-greyhound-game-ready ② "Low Poly M8 Greyhound armored car"（约 6.2k 多边形 PBR）https://cgtrader.com/3d-models/vehicle/military/low-poly-m8-greyhound-armored-car ；价格均需人工确认 | ① | 需书面确认 | | [ ] 已购 |
| V-VEH-US-002 | M5A1 Stuart | CGTrader "low poly m5a1 stuart light tank" https://cgtrader.com/3d-models/military/vehicle/low-poly-m5a1-stuart-light-tank ；另一件标 PBR 的约 30.5 万面，不适合；价格需确认 | 该低模 | 需书面确认 | | [ ] 已购 |
| V-VEH-US-004 | M4A3(76)W / Jumbo | CGTrader "M4A3E8 Sherman Easy Eight" https://cgtrader.com/3d-models/vehicle/military/m4a3e8-sherman-easy-eight-rice-red-devils ；"m4a3 76 w" https://www.cgtrader.com/3d-models/military/armor/m4a3-76-w （价格/面数需确认；**Jumbo (M4A3E2) 未发现 → GAP-V10**） | 看图 | 需书面确认 | | [ ] 已购 |
| V-VEH-US-005 | GMC CCKW 2½ ton 卡车 | BuiltByBit 4 车包含 GMC CCKW 与 Opel Blitz https://builtbybit.com/resources/ww2-truck-4-different-vehicles.117632/ （**用途为 Minecraft 类平台，许可不一定适用，需核实，不建议**）；Unity "WW2 United States Armed Forces"（含 M3A1 半履带等）https://discussions.unity.com/t/ww2-united-states-armed-forces/804561 ；CCKW 专用 → **GAP-V10** | GAP | — | | [ ] 已购 |
| V-VEH-US-006 | M3 半履带 | CGTrader "Low Poly M3A1 Half-track Personnel Carrier"（约 11.9k 多边形，PBR，Royalty Free）https://cgtrader.com/3d-models/military/vehicle/low-poly-m3-half-track-personal-carrier ；价格需确认 | 采用 | 需书面确认 | | [ ] 已购 |

**D2. 德军**

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-VEH-GE-001 | Sd.Kfz. 234 Puma | 未发现 → **GAP-V10** | — | — | | [ ] 已购 |
| V-VEH-GE-002 | Panzer II Luchs（Pz.Kpfw. II Ausf. L） | 仅发现 PZII-G（TurboSquid "PZII-G Carro armato pronto per il gioco" https://www.turbosquid.com/it/3d-models/pziig-tank-gameready-3d-2142429 ），**不是 Luchs** → **GAP-V10**（可用 Ausf.G 作最接近替代） | GAP | 需书面确认（TurboSquid 个案批准） | | [ ] 已购 |
| V-VEH-GE-004 | Panther（Ausf. G） | ① CGTrader "Panzerkampfwagen V Panther G with texture pack GameReady"（$15，rigged，贴图约 803MB，Royalty Free **No AI**）https://cgtrader.com/3d-models/vehicle/military-vehicle/panzerkampfwagen-v-panther-g-with-texture-pack-gameready ② "Panther G Tank"（$5，rigged PBR 低模）https://cgtrader.com/3d-models/vehicle/military-vehicle/panther-g-tank-e909cd72-0ca7-456c-b390-b722e16d741f ③ "World War II - Panther Panzer V - Game Ready - PBR"（$25，约 16k 三角，4K；**买家评价称并非真正 game ready**）https://cgtrader.com/3d-models/military/vehicle/world-war-ii-panther-panzer-v-game-ready-pbr 。以上价格均【搜索摘要】 | ① 或 ② | 需书面确认 | | [ ] 已购 |
| V-VEH-GE-005 | Tiger I | ① **Tiger I German Heavy Tank**｜max3ddd｜RenderHub https://www.renderhub.com/max3ddd/tiger-i-german-heavy-tank ｜game ready，PBR，约 65.5k 多边形，下载 6.9MB；**Extended Use (IP Restricted)**；价格需确认 ② CGTrader "tiger 1 pbr gameready" https://www.cgtrader.com/3d-models/military/armor/tiger-1-pbr-gameready ③ CGTrader "Tiger 1 GameReady tank" https://cgtrader.com/3d-models/military/military-vehicle/tiger-1-gameready （炮塔旋转、炮管俯仰、负重轮悬挂层级；价格需确认）。**另一件"含部分内饰"的 Tiger I 被标注为 AI 辅助制作，已排除** | ③ | ①：PENDING；②③：需书面确认 | | [ ] 已购 |
| V-VEH-GE-006 | Opel Blitz | BuiltByBit 4 车包（见 V-VEH-US-005，不建议）；MyMiniFactory 为 3D 打印 STL（不适用）→ **GAP-V10** | GAP | — | | [ ] 已购 |
| V-VEH-GE-007 | Sd.Kfz. 251 | Unity "WW2 Asset Pack"（含 Sd.Kfz. 251，2K PBR，约 6.4k 三角）https://assetstore.unity.com/packages/slug/182490 、Flipped Normals 同款 https://flippednormals.com/product/ww2-asset-pack-1613 ；价格需确认；CGTrader Sd.Kfz. 250 系列半履带 https://www.cgtrader.com/3d-models/vehicle/military/sd-kfz-250-half-truck-collection （**250 非 251**） | Unity 包 | 需书面确认；Flipped Normals：PENDING | | [ ] 已购 |

**D3. 火炮与反坦克炮**

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-VEH-ART-001 | M2A1 105mm 榴弹炮 | 搜索未发现 "M2A1" 命名商品；M101A1 即 M2A1 后来的名称，CGTrader "military artillery weapon howitzer 105mm" https://cgtrader.com/3d-models/military/military-vehicle/military-artillery-weapon-howitzer-105mm （型号吻合度需看图，价格需确认）→ 近似 **GAP-V11** | 该商品（看图） | 需书面确认 | | [ ] 已购 |
| V-VEH-ART-002 | leFH 18 | ① CGTrader "Low Poly 10.5 cm leichte Feldhaubitze"（约 6.7k 多边形，PBR，rigged）https://cgtrader.com/3d-models/military/gun/low-poly-10-5-cm-leichte-feldhaubitze-anti-tank-gun ② RenderHub Howitzer FH18 https://www.renderhub.com/gemreer/howitzer-fh18-artillery-unit （game ready，rigged，PBR）；价格需确认 ③ Hum3D 高模 10.5 cm leFH 18，**$150【搜索摘要】**，约 70.8 万面，需减面 https://hum3d.com/3d-models/10-5-cm-lefh-18/ | ① | 需书面确认；Hum3D：PENDING | | [ ] 已购 |
| V-VEH-ART-003 | 57mm M1（6-pounder 的美版） | 未发现 → **GAP-V10**（RenderHub 有 Pak 36 37mm 可作外形近似，非同型号） | GAP | — | | [ ] 已购 |
| V-VEH-ART-004 | Pak 40 | ① **WW2 Pak 40 - Rigged**｜Loss｜RenderHub https://www.renderhub.com/loss/ww2-pak-40-rigged-panzerabwehrkanone ｜game ready，约 16k 面，Extended Use；价格需确认 ② CGTrader "Low Poly 7.5 cm Pak 40"（**$20【搜索摘要】**）https://cgtrader.com/3d-models/military/gun/low-poly-7-5-cm-panzerabwehrkanone-40-anti-tank-gun | ② 或 ① | PENDING / 需书面确认 | | [ ] 已购 |

**D4. 飞机（空袭过场）**

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-VEH-AIR-001 | P-47 | CGTrader "United States fighter aircraft Republic P-47 Thunderbolt"（低模，PBR，约 1.2 万面，**$139【搜索摘要】**，Royalty Free）https://cgtrader.com/3d-models/aircraft/military/united-states-fighter-aircraft-republic-p-47-thunderbolt | 采用 | 需书面确认 | | [ ] 已购 |
| V-VEH-AIR-002 | Ju 87 | CGTrader "ju87 b2 stuka 4k pbr game asset for unity unreal" https://www.cgtrader.com/3d-models/aircraft/military-aircraft/ju87-b2-stuka-4k-pbr-game-asset-for-unity-unreal ；GameDev Market https://gamedevmarket.net/asset/ww2-german-dive-bomber-ju87-stuka （$26【搜索摘要】；**买家留言称贴图缺失**，需谨慎） | CGTrader | 需书面确认；GameDev Market：PENDING | | [ ] 已购 |
| V-VEH-AIR-003 | Bf 109 | RenderHub "Bf-109 F"（game ready，PBR 未标）https://www.renderhub.com/game-ready-models/aircraft ；价格/面数需确认 | 看图 | PENDING | | [ ] 已购 |
| V-VEH-AIR-004 | C-47（M3，Airhead 过场） | Sketchfab "WW2 US Military Transport Aircraft C-47 Skytrain"，**$29.99【搜索摘要】** https://sketchfab.com/3d-models/ww2-us-military-transport-aircraft-c-47-skytrain-1aa07eceb48a48a091ce72a5580e8a28 （**Sketchfab Store 已关闭，该价格来源存疑，需人工确认是否仍可购买**）；CGTrader 合集 https://www.cgtrader.com/3d-model-packs/historic-air-superiority-and-logistics-collection-wwi-and-wwii 。（另一件低模 C-47 被标 AI 辅助制作，已排除） | CGTrader 合集（确认非 AI） | 需书面确认 | | [ ] 已购 |

#### 4.4.6 音效

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| B-EXP-002 | 地雷（人员/车辆/反坦克）爆炸 | Warfare 2（含三类地雷）[F]（见 B-EXP-001，M1 已买） | 复用 | 需书面确认 | | [ ] 已购 |
| B-EXP-003 | 炸药包（Satchel/C-4） | Warfare 2（含 C-4）[F]。注意是 C-4，非 WW2 时代 TNT；需人工接受（声学差异小） | 复用 | 需书面确认 | | [ ] 已购 |
| B-HVY-001 | Bazooka M1/M9 发射与命中；Panzerschreck 发射与命中；Panzerfaust | Warfare 2（RPG、榴弹发射器、火炮，现代军演录制）[F]；Sound Ideas "Rockets & Missiles"（380 效果，**$295**，含 bazooka/RPG，**是否实录未标**）[S]。**GAP（WW2 型号专用实录未找到）→ GAP-A03**；最接近替代：Warfare 2 的 RPG 发射 + 后期 EQ，需负责人决定史实容忍度 | 替代 | PENDING | | [ ] 已购 |
| B-ART-001 | 105mm 榴弹炮发射（近/中/远，0–150 m） | **Pole Position 105mm Howitzer Cannon**｜**$149**｜27 声音，24/96｜https://www.asoundeffect.com/sound-library/howitzer-cannon-105mm [F]（页面未写型号，**不要写 M101**） | 采用 | 需书面确认 | | [ ] 已购 |
| B-ART-002 | 迫击炮发射/落弹；榴弹炮落弹（近/远） | Warfare 2；Warfare 3 Heavy Bombardment [S]；BOOM "Cannons"（26 通道迫击炮/火炮含落弹与飞过）`https://www.boomlibrary.com/?p=373324` [S]。落弹 + 飞过（incoming whistle）由 Warfare 3/BOOM Cannons 覆盖 | Warfare 3 | 需书面确认 | | [ ] 已购 |
| B-ART-003 | 远处炮击床（背景轰鸣） | **免费**：Sonniss GDC [M]。付费：Warfare 3；Warfare 2 的远距离麦。用远层做 loop 床 | 复用 | OK / 需书面确认 | | [ ] 已购 |
| B-AIR-001 | 航空炸弹落下+爆炸 | Warfare 3 [S]（未确认含航弹）。若无 → 需 GAP 处理 → **GAP-A15** | 视清单 | PENDING | | [ ] 已购 |
| B-AIR-002 | 飞机俯冲、扫射（机炮） | 见 D-AIR-001；扫射用 M1919/MG42 近层 + 多普勒后期 | 复用 | 需书面确认 | | [ ] 已购 |
| D-TNK-005 | 被击中/跳弹（金属击打、内部回响） | Warfare 2 金属板命中 [F]；Sonniss GDC [M]。**GAP**（坦克被击中专用实录没找到）→ **GAP-A05** | 替代 | PENDING | | [ ] 已购 |
| D-TNK-006 | Panzer IV / Panther 全套（发动机、履带、炮）：德军方 M3 配 | **Panzer IV Ausf. G**｜$224.25｜https://sonniss.com/sound-effects/panzer-iv-ausf/ ；**Panther Ausf. A (1943)**｜$224.25｜https://sonniss.com/product/panther-ausf-1943 （规格见 §4.2.6 核心库速查） | 两者均买 | 需书面确认 | | [ ] 已购 |
| D-VEH-001 | 卡车（GMC 2.5 吨 Deuce、Opel Blitz）、半履带（M3、SdKfz 251）、吉普（Willys） | A Sound Effect 军车分类 https://www.asoundeffect.com/sound-category/vehicles/military-vehicle-sounds/ [S，仅分类页]。**GAP 待深入搜索；本次未锁定具体库 → GAP-A07** | 补搜 | PENDING | | [ ] 已购 |
| D-AIR-001 | P-47 / Ju 87 Stuka / C-47 | A Sound Effect "WW2 Aircraft"（~$150，英国机场录制，多种稀有 WW2 机，含飞过/滑行/发动机启动）[S]；Sound Ideas "Sounds of War"（$129 标价/约 $65 促销，75 条，含 Hurricane、Stukas、Flying Fortress）https://sound-ideas.com/products/sounds-of-war-sound-effects-library [S]；A Sound Effect B-25 Mitchell（~$125，115 声音）[S]；Sound Ideas P-51 Mustang by BOOM（$65）[S]。**P-47、C-47 专用库：未找到（GAP-A06）**；最接近替代：P-51（**P-47 是星型发动机，音色不同**，需人工接受）；Stuka 俯冲警笛"Jericho-Trompete"：Sounds of War 声称含 Stukas，需核实是否有俯冲警笛 | 替代 | 需书面确认 | | [ ] 已购 |
| E-ACT-001 | 包扎、建造敲打、铲土 | **免费**：Freesound CC0。付费：**GAP（未专门搜索到军事版）**；通用 Foley（锤击、铁铲挖土）见 A Sound Effect Foley 类目 → GAP-A08 | 免费通用 | OK（CC0） | | [ ] 已购 |

---
### 4.5 M4 Omaha Beach、Foy、Skirmish、画质与优化、公开测试

#### 4.5.1 地图环境

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-BLD-005 | 阿登村舍（半木、石基、冬季屋顶雪） | **GAP** → **GAP-V02** | — | — | | [ ] 已购 |
| V-BLD-006 | 大西洋壁垒：碉堡（Regelbau）、Tobruk 掩体；LOD，内外双材质 | ① **D-Day Atlantik Wall Collection**｜CGTrader｜https://www.cgtrader.com/3d-model-packs/d-day-atlantik-wall-collection ｜**$89【搜索摘要】**｜21 件低模 PBR，含 6 种滩头障碍（3 木斜桩、1 比利时门、1 铁丝网、1 捷克刺猬）；Royalty Free ② CGTrader "bunker m162"（Regelbau 带 3 LOD，内外双材质）https://www.cgtrader.com/3d-models/military/armor/bunker-m162 ③ CGTrader 碉堡套件"74 个对象 + 95 个模块" https://www.cgtrader.com/3d-model-collections/d-day-atlantik-wall-collection （同上页，价格需确认）。**Tobruk 专用：未发现** | ① + ② | 需书面确认 | | [ ] 已购 |
| V-BLD-007 | 滩头障碍：捷克刺猬、比利时门、木桩 | ① CGTrader "WW2 Anti-tank Obstacle PBR Game ready"，**$5.99【搜索摘要】** https://www.cgtrader.com/3d-models/military/other/ww2-anti-tank-obstacle-pbr-game-ready ② Flipped Normals "Czech Hedgehog Obstacle WWII"：1,202 面，三级 LOD（638/364/86），干净与海草两套贴图 https://flippednormals.com/product/czech-hedgehog-obstacle-wwii-2440 （价格需确认）③ RenderHub KangaroOz https://www.renderhub.com/kangarooz-3d/czech-hedgehog-obstacle-wwii | 先看 V-BLD-006 ① 是否已含 | 需书面确认；Flipped Normals/RenderHub：PENDING | | [ ] 已购 |
| V-NAT-003 | 阿登冬季云杉 + 积雪 | **免费**：Poly Haven "Fir Tree 01 / Pine Tree 01"（CC0）作基础。**付费**：① Fab "Spruce & Larch Pack"（扫描纹理，PivotPainter 风，Nanite 转换）https://www.fab.com/migrated-listing/22bfb416c8e64f3e841a8fed007ee68f （价格/许可/雪变体未核）② RenderHub "Winter Snow Forest Trees Pack 55"（高/低模）https://www.renderhub.com/3dgardenplants/winter-snow-forest-trees-pack-55-highpoly-low-poly ③ NordicCanopy Norway Spruce Pack（UE 论坛贴，game ready PBR）https://forums.unrealengine.com/t/nordiccanopy-realistic-norway-spruce-pack-game-ready-pbr/2747557 ；价格均需确认 | 免费基础 + 视预算 | Poly Haven：OK；付费：PENDING | | [ ] 已购 |
| V-NAT-006 | 海滩沙/卵石/海草/潮线材质 | **免费**：Poly Haven、ambientCG 沙/卵石/湿沙 PBR 材质（CC0） | 免费 | OK（CC0） | | [ ] 已购 |
| V-NAT-008 | 水（河、池塘、海浪）材质与泡沫 | **免费**：无（需着色器与泡沫贴图；ambientCG 无）。视需求补搜 | 补搜 | — | | [ ] 已购 |
| V-SKY-002 | 冬季/雪天 HDRI | **免费**：Poly Haven 有雪天与阴天（具体文件**未核实**） | 免费 | OK（CC0） | | [ ] 已购 |

#### 4.5.2 兵种角色

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| V-CHR-US-004 | 美军冬季（Foy 1945-01：大衣、风帽、白色雪披、套鞋）4K PBR | **GAP**：未发现冬季美军 → **GAP-V04** | 夏季角色 + 冬季服装模块（需确认 D-MOD） | — | | [ ] 已购 |
| V-CHR-GE-003 | 德军冬季（雪披、毛皮连帽服、白色罩衣） | **GAP**（未发现）→ **GAP-V04** | 同上 | — | | [ ] 已购 |

#### 4.5.3 音效

| 需求 ID | 规格 | 候选 | 推荐 | 许可证状态 | 选定候选 | 已购 |
|---|---|---|---|---|---|---|
| F-AMB-002 | 海滩海浪（Omaha）。**冲突**：片段表内里程碑列写 M2，备注写"M4 才需要"，按保守归 M4（§3.3 C2） | **免费**：Freesound CC0。**付费**：A Sound Effect 海洋/海滩库（29 条，页面未读细节）[S] | 免费 | OK（CC0） | | [ ] 已购 |
| F-AMB-004 | 冬季森林（Foy） | **免费**：同上。**付费**：A Sound Effect 有"休眠雪林"库 [S，未读] | 免费 | OK（CC0） | | [ ] 已购 |
| I-PD-001 | M4+ 公共领域二战时期**录音**（做氛围/收音机广播） | 无候选。**PD 注意（凭记忆 [M]，未核实；搜索摘要 [S] 有冲突）**：美国联邦政府雇员创作的作品属 PD，但"政府委托的承包商/商业唱片"不一定；作曲与录音分别有版权；1923–1946 年发行的录音在美国通常 95/100 年才进 PD，**WW2 时期（1939–1945）录音多数要到 2030s–2040s 才进入 PD**。因此"WW2 老唱片"**默认不能直接用**；可用的是：①作曲已 PD 的古典乐/进行曲 + 自己授权的新录音（需购买/外包）；②NARA/Library of Congress 明确标注 PD 的政府文件（逐条核对）。PD 判断仅针对美国，欧洲玩家的法律情况可能不同。**不得凭搜索结果直接使用，建议书面向 NARA/LOC 或律师确认** | 暂缓 | PENDING | | [ ] 已购 |

---

## 5. 三档预算

### 5.1 价格可信度说明

规则：【核实】/[F] = 已打开商品页读到；【搜索摘要】/[S] = 摘要所见，需人工确认；【估算】= 无来源，仅作量级。**以下均不含税、不含外包（外包仅在极致档单列）。** 免费来源（Poly Haven、ambientCG、Megascans 若确认免费、Sonniss GDC）为 $0。订阅类（Soundly/Epidemic/Artlist）价格"需人工确认"，**未计入**。

**已核实价格（【核实】/[F]）**

| 项 | 价格 | 备注 |
|---|---|---|
| Fab FPS World War Guns Pack – German | $19.99 | 许可证栏待确认 |
| Fab FPS World War Guns Pack – United States | $19.99 | 同上 |
| Fab Panzer IV Production models | $25.99 | 同上 |
| Fab Military Trenches and Modular World War Compound | $179.99 | 仅 UE 格式，倾向不买 |
| RenderHub Loss 德军步兵武器 bundle | $70 | 单件：G43 $15、MP40 $15、MG34 $20、MG42 $20；FG42 $15（片段未单独标注证据）；"Extended Use (IP Restricted)" 条款未核实 |
| Sonniss Weapons of WWII – Designed | $38.50（原价 $55） | [F] |
| BOOM WWII Firearms | $139–$269 | 各版未分列 [F] |
| GCRadix Firearm Foley & Mechanics: WW2 | $119.40（原价 $199） | [F] |
| Diegetix WW2 Weapons: German & US | $25–$40 | 页面区间 [F] |
| Pole Position MG 42 | $111.75（原价 $149） | [F] |
| Pole Position Sherman M4A3 / M4A1 / Panzer IV Ausf. G / Panther Ausf. A | 各 $224.25 | [F]（M4A1 原价 $299） |
| BOOM / Pole Position World War 2 Tanks | $247.20（A Sound Effect）；$248（Sound Ideas，原价 $309） | [F] |
| The Warfare 2 Library | $199 | [F] |
| Pole Position 105mm Howitzer Cannon | $149 | [F] |

**搜索摘要价格（【搜索摘要】/[S]，须人工确认）**：M1 Garand（CGTrader）$25、Lowpoly M1 Garand $59、albin Paratrooper $60、TurboSquid WW2 US Soldier $59.99、Sherman M4 Gameready Rigged $31、WW2 American Tank M4 Sherman $50、Panther $15/$5/$25、P-47 $139、Pak 40 $20、Panzerfaust With Box $20、Stielhandgranate $5.99、Hum3D leFH 18 $150、Atlantic Wall Collection $89、WW2 Anti-tank Obstacle $5.99、MoCap Online Rifle Starter $3.99、kevdev Human Soldier Animations $30、GameDev Market Stuka $26、Sketchfab C-47 $29.99（来源存疑）；音频：Bullet Bys & Impacts $29.99、Sherman Cannon 75mm $149、Warfare 3 $199、Rockets & Missiles $295、Sounds of War $129/约 $65、WW2 Aircraft 约 $150、B-25 约 $125、P-51 $65、Historical Battle Voices 约 $33、Shockwave German $30、Army Voice Pack $4.49 起。

### 5.2 M0 / M1 最小集合（先买这些）

**视觉 M0/M1 最小集合**

| 项 | 来源 | 价格 | 备注 |
|---|---|---|---|
| 德军枪械包（Kar98k/MP40/StG44/Luger） | Fab German | **$19.99【核实】** | 确认 Standard |
| 美军枪械包（Thompson/BAR/M1903/M1911） | Fab US | **$19.99【核实】** | 同上 |
| 德军机枪/G43/MG42 等 | RenderHub Loss bundle | **$70【核实】** | 条款未核实 |
| 美军 M1 Garand（第一人称向） | CGTrader | $25【搜索摘要】 | |
| 美军步兵角色 1 个 | CGTrader/RenderHub albin | ~$60【搜索摘要】 | |
| 德军步兵角色 1 个 | CGTrader German Soldier Rig | 价格需确认 | 按 $40–80 估【估算】 |
| Sherman（1 辆，可动） | CGTrader | $31【搜索摘要】 | 无内饰 |
| Panzer IV（可选） | Fab | **$25.99【核实】** | 无绑定信息 |
| Jeep | CGTrader | 按 $20–50 估【估算】 | |
| 沙袋/障碍/道具 | CGTrader | $6–$30/件【搜索摘要+估算】 | |
| 动画 | MoCap Online Rifle Starter | $3.99【搜索摘要】；Rifle Pro 价格需确认；Human Soldier Animations $30【搜索摘要】 | |
| 其余全部（地表、HDRI、木箱桶、岩石、部分树） | Poly Haven / ambientCG / Megascans | **$0** | |
| **视觉 M0/M1 合计** | | **约 $350–600【估算，含未核实价格】** | 不含建筑 GAP、冬季角色与坦克内饰 |

**音频 M0 最小集合（≈ $70–$270）**

| 项 | 价格 | 备注 |
|---|---|---|
| Sonniss GameAudioGDC 2015–2024 + 2026 | $0 | 约 250 GB+ |
| Weapons of World War II – Designed | **$38.50** | |
| Bullet Bys & Impacts | $29.99 [S] | 先看文件清单确认是实枪；否则暂缓 |
| （可选）Warfare 2 Library | $199 | M0 不必 |

**音频 M1 追加**：Pole Position MG 42 $111.75；Sherman M4A3 $224.25；Sherman Cannon 75mm M4A1 $149 [S]；Warfare 2 $199；GCRadix $119.40；Diegetix 合包 ≤ $40；脚步 + 环境（估算）$150–$450；人声（德语 46 条 + 英语包）≈ $40–$100。**M0+M1 音频合计（片段估算）约 $1,150–$1,650**。文档作者按上表价格复算为约 $1,100–$1,460（M0 $68.49 + M1 已核实项 $843.40 + 估算项 $190–$550），**取片段较高上限更保守**。

### 5.3 三档预算（美元，均【估算】）

> 总纲 A6：以"标准"档为计划基线。视觉与音频区间由两份片段分别给出，**合计 = 两者区间的算术相加**（文档作者计算，非来源）。实际可能偏差 ±50%（视觉片段自述）。

| 档位 | 视觉 | 音频 | 合计（视觉 + 音频） |
|---|---|---|---|
| 精简 | 约 $800–1,500 | 约 $800–1,500 | **约 $1,600–3,000** |
| 标准（基线） | 约 $3,000–6,000 | 约 $3,000–4,500 | **约 $6,000–10,500** |
| 极致（含外包） | 约 $12,000–25,000 | 约 $6,000–12,000+ | **约 $18,000–37,000+** |

#### 5.3.1 精简档

- **视觉（$800–1,500）**：M0/M1 最小集合（$350–600）+ M2 小件补齐（手雷/烟雾/刺刀/MG34/FG42，$100–300）+ 剩余载具各 $30–60（约 10 件，$300–600）+ 火炮/反坦克炮（$50–150）+ FX 包（$50–150）。**不含**：诺曼底建筑、冬季角色、内饰。视觉品质低于"AAA 写实"，仅适合内部验证。
- **音频（$800–1,500）**：仅 M0/M1 必买 + Sonniss 免费包；坦克仅 Sherman M4A3；火炮仅 105mm 榴弹炮 + Warfare 2；人声/音乐用低价 itch.io/Shockwave；只做美德双方最小武器集。

#### 5.3.2 标准档（计划基线）

- **视觉（$3,000–6,000）**：精简档全部 + 第一人称高精度武器 6–8 把 $600–1,800 + 角色扩充（兵种变体、冬季、FJR，按"基础身体 + 服装模块"）$800–1,500 + 动捕包（MoCap Online Rifle Pro + HipFire + 匍匐 + 战斗）$150–400 + 建筑：通用欧式石屋模块 + Atlantic Wall 集（$89【搜索摘要】）$300–800 + 载具补缺（Puma/Luchs/Blitz/CCKW）$300–800 + 其余 FX/贴花/字体 $100–300。**仍存在 GAP**（诺曼底专用建筑、树篱、内饰），需外包或接受近似替代。
- **音频（$3,000–4,500）**：上述 + BOOM WWII Firearms Bundle（$269）+ Panzer IV + Panther + BOOM WW2 Tanks + Sherman M4A1 + Warfare 3 + WW2 Aircraft + 脚步/Foley/环境 + 若干音乐包。片段复算：Firearms $269 + Designed $38.50 + Diegetix ~$40 + GCRadix $119.40 + Warfare 2 $199 + Warfare 3 $199 + 105mm $149 + M4A3 $224.25 + M4A1 $224.25 + 75mm 炮 $149 + Panzer IV $224.25 + Panther $224.25 + BOOM WW2 Tanks $247.20 + MG42 $111.75 + WW2 Aircraft ~$150 + Bullet Bys $30 ≈ **$2,600**（文档作者复算 $2,598.85）；再加脚步/环境/Foley/人声/音乐（估算 $500–$1,500）。Soundly 一年订阅等价格需人工确认，**未计入**。

#### 5.3.3 极致档（含外包）

- **视觉（$12,000–25,000）**：标准档全部 + 外包：诺曼底建筑套件 $3,000–10,000、带内饰坦克 ×2 $3,000–12,000、树篱套件 $800–2,500、冬季角色 ×2 $1,600–5,000、第一人称动画集 $1,200–3,600（均【估算】）。若需要全部兵种、全载具的"第一人称品质"，成本继续线性上升。
- **音频（$6,000–12,000+）**：标准档 + BOOM Construction Kit 全多轨 + 更多飞机/车辆/Foley 库 + **外包补录 GAP**（MG34、Panzerfaust/Bazooka、德英战场喊话、WW2 无线电等，**需人工决定是否算"重新录音"**）+ 音乐委托（估算）。
- 外包的现实提醒：外包（含内饰坦克、街道模块）总价易达 US$10,000+；建议 M1 只外包最必要的 1–2 项。外包合同须写明：原创建模/录制、无第三方游戏资产、授权范围含网页分发、交付源文件。

> 提示：上述音频购买均为一次性购买（BOOM/Sonniss/A Sound Effect 按"永久授权"）；订阅（Soundly/Epidemic/Artlist）的"永久性"取决于条款，**不建议依赖订阅内容长期运营**。促销结束会涨价。

### 5.4 "零采购"档与"最小补缺"档（负责人 2026-11-17 指示；不改动 §5.1–§5.3 的数字）

| 档位 | 内容 | 金额 |
|---|---|---|
| **零采购（新基线）** | 全部使用 §8 的免费素材；§9 的缺口按 `docs/ASSET_GAPS.md` §3 用灰盒/静音占位 | **$0** |
| **最小补缺** | §9.4 清单：视觉约 8 件 + 音频 3–5 个库 | 视觉 ≈ $325 + 音频 ≈ $354–$369，合计 **约 $680–$695【估算】**；其中"起步子集"（两个 Fab 枪包 + MoCap Online Rifle Starter + Sonniss Designed）约 **$82.47** |
| 精简 / 标准 / 极致 | 见 §5.3 | **降为备选**（数字不变：约 $1,600–3,000 / $6,000–10,500 / $18,000–37,000+） |

- 价格全部沿用本文 §4/§5.1 的原值，**未重新核实**；视觉合计中的德军步兵角色（价格需确认）按 $60 估算。
- 音频调研原文写"总计控制在约 $320 以内"，与其所列五项价格相加（$354.25–$369.25）不符，本节按相加结果。
- "零采购"档下的后果（哪些里程碑停留在灰盒/静音占位）见 §10.2；外包（诺曼底建筑、带内饰坦克等）不在本档，价格见 `docs/ASSET_GAPS.md`。

---

## 6. 给厂商/卖家的询问邮件模板（英文）

> 使用提示：先把 `[ ]` 占位符替换为实际内容；**不要**在邮件里写姓名、地址等个人信息之外不必要的内容，发件人以你想对外公开的项目/个人名义署名。收到回复后把完整邮件存档，路径登记到 `LICENSES.md` 的"书面确认存档路径"字段。

### 6.1 视觉素材（3D 模型 / 贴图 / 动画 / 特效；发给 Fab/CGTrader/RenderHub/Flipped Normals 等卖家、MoCap Online、TurboSquid 个案批准）

```
Subject: Licensing question — browser-based free-to-play WWII shooter (donation-supported) — [PRODUCT NAME / LISTING URL]

Hello [SELLER NAME],

I am interested in purchasing "[PRODUCT NAME]" ([LISTING URL]) for a multiplayer
first/third-person shooter that runs entirely in the web browser (WebGPU/WebGL2,
Babylon.js engine; not Unreal or Unity). Before buying, could you please confirm
the following in writing?

1. Use in a non-Unreal/Unity engine: the asset may be used in a browser game
   built on Babylon.js.
2. Web distribution: the game is free to play and supported by voluntary
   donations (no paid content). Assets are converted (e.g. glTF + KTX2 textures
   + mesh compression), packed into a proprietary chunked container file and
   streamed to the player's browser. A determined player could extract data from
   the browser cache or DevTools. We provide no download page, no asset browser
   and no mod access to the raw assets. Is this "embedded in the product" use
   compatible with your license?
3. Modifications we will make: format conversion, LOD/polygon reduction,
   texture compression, re-targeting animations, re-binding skin weights to a
   common skeleton, adjusting material parameters, and combining purchased
   parts into assembled characters/props. We will not resell or redistribute the
   assets on a standalone basis. Is this permitted?
4. Seats: one human project lead plus an automated agent processing the files on
   the same machine. Is a single-user license sufficient?
5. If the project later receives donation rewards or becomes paid, does the
   license need to change?
6. Originality: please confirm that the model/animation is your original work
   and does not contain assets taken or extracted from any commercial video
   game or other third-party work.
7. (If listed as "Editorial" or with a custom license) please confirm whether a
   game-use license is available.

Please reply by email so that we can archive your confirmation with our license
records. Thank you!

Best regards,
[YOUR NAME / PROJECT NAME]
[CONTACT EMAIL]
```

### 6.2 音频素材（发给 BOOM Library、A Sound Effect、Sonniss/Pole Position、Pro Sound Effects、Soundly、Epidemic Sound 等）

```
Subject: Licensing question — browser-based free-to-play WWII game (donation-supported) — [LIBRARY NAME]

Hello [VENDOR NAME],

We would like to license "[LIBRARY NAME]" ([URL]) for a free-to-play,
donation-supported multiplayer shooter that runs in the web browser (WebAudio).
Please confirm the following in writing:

1. Audio is compressed to Opus/AAC, packed into sprite files or a proprietary
   container and delivered to the player's browser together with the game. A
   player can see these package files in DevTools or the browser cache. Does
   this "embedded and distributed with the game" use comply with your license?
2. We will perform: format conversion, loudness normalization, editing/cutting,
   distance-layer mixing (near/mid/far), EQ/compression and convolution reverb
   processing. Please confirm these count as permitted synchronization/editing
   and do not violate any "no modification / no repackaging" clause.
3. We will NOT: provide any standalone sound download or preview page, build a
   soundboard, or expose the original audio to mods. Original WAV files will
   never be uploaded to our public servers; only the encoded game packages are.
   We will also strip embedded metadata (BWAV/iXML/Soundminer) from delivered files.
4. Seats: one human project lead plus an automated agent (an AI assistant)
   processing files on the same machine. Is a single-user license sufficient?
5. If the project later receives donation rewards or becomes paid, does the
   license need to change?
6. (Subscription products) If we cancel the subscription, may the already
   released and continuously operated game keep using the content?
7. (Where relevant) Is credit/attribution required or appreciated?

Please reply by email so that we can archive your written confirmation.

Best regards,
[YOUR NAME / PROJECT NAME]
[CONTACT EMAIL]
```

---

## 7. 调研局限与未核实清单

### 7.1 调研局限（诚实声明）

- 大量商品页（CGTrader、RenderHub、TurboSquid、Unity、itch.io）**无法被 WebFetch 打开或只返回空内容**，所以这些页面的价格、面数、许可均来自搜索摘要，**需人工确认**。
- **没有打开任何素材文件**，无法验证模型质量、历史准确性、UV/贴图质量、绑定质量、是否为游戏提取。音频同理：没有试听任何库。
- 搜索区域受限（US 搜索引擎），欧洲/中国卖家与本地站点覆盖可能不足。
- 以下页面本次**未能访问**（403/404/503/被工具屏蔽）：Adobe Mixamo 官方 FAQ、TurboSquid 授权页首次抓取（改由帮助中心页取得）、BOOM 官方 license 页（`/license/` 404）、Pro Sound Effects `/license-agreement/`（404；`/licensing/` 可读）、Soundsnap `/licensing`（404；`/licence` 可读）、Zapsplat 许可页（403）、BBC Sound Effects 站点（工具拒绝抓取）、OpenAIR 站（503）、Epidemic `/music-licensing/`（404）、Sound Ideas 的 BOOM TOS PDF（404）、Soundly EULA PDF（压缩流无法读出正文）、Fab 列表页 https://www.fab.com/listings/1773c3e2-c7e2-4928-8aea-57f5e9d5f2f9（404）。
- 没读到的条款全文：Fab EULA 全文、Epic Content License Agreement 全文、CGTrader T&C Sec.21、Reallusion "Licensing FAQ for Game Developers" 附录、Rokoko EULA、KitBash3D Cargo EULA 全文、BOOM 单用户 EULA 全文。
- 所有结论不是法律意见。

### 7.2 未核实 / 需人工确认清单（购买前逐项处理）

**许可与条款**

1. Fab EULA 全文中关于"原始格式可提取性"的条款；Fab 上每个商品的"许可证"栏是否为 Standard（两个枪械包与 Panzer IV 的许可证页都未显示名称）。
2. 捐赠收入是否计入 Fab 的 $100,000 档位门槛（保守按计入）。
3. Megascans：目标资产在 Fab 上当前价格是否 = 0、许可是否 = Standard、是否带 Nanite-only 网格；Megascans Trees/Megaplants 是否在 Fab 上改为 Standard。旧 Quixel/Bridge 下载的资产须从 Fab 重新领取。
4. RenderHub "Extended Use (IP Restricted)" 的含义；RenderHub、Flipped Normals、GameDev Market、Super Hive Market、Hum3D 的条款。
5. TurboSquid 个案批准流程是否走；CGTrader 团队/席位细则、署名要求；Unity 席位与署名；KitBash3D 游戏打包。
6. Mixamo 官方 FAQ 现状（可否打包进游戏）；2025-06 后登录/下载故障是否仍存在。
7. MoCap Online：1M 用户 / US$1M 收入阈值的商业许可条件；Rifle Pro 价格。
8. ActorCore Mass Distribution License 流程；Rokoko EULA。
9. Sonniss GDC 的单用户/多用户；BOOM 团队 ≥ 2 人是否需 multi-user；A Sound Effect 对"modifying"的解释；Pro Sound Effects 对游戏的条款；Soundsnap 许可"可撤销"的影响；Epidemic/Artlist/Fab 音频/Soundly 的条款与价格。
10. Freesound/Pixabay/SoundBible/itch.io 作曲者/Shockwave-Sound 单条许可；OpenAIR 逐条许可；EchoThief 许可。
11. 字体：Inter/Roboto Condensed/IBM Plex Sans Condensed/Noto Sans SC/Source Han Sans/Saira Stencil One 的 OFL 文本（凭记忆/聚合站）；Lucide/Material Symbols 的许可（凭记忆）。

**价格与商品**

12. CGTrader/RenderHub/TurboSquid/Unity/itch.io 全部价格；Sketchfab C-47 价格来源存疑（Store 已关闭）；Fab 36 件军械包链接 404，是否仍上架。
13. Sherman Cannon 75mm 产品页（测试环境域名，正式页未核实）；Sonniss "75mm Sherman M4A1 Cannon" 是否与之同一产品；Warfare 3 的 URL（`?p=760476` 为搜索所得）；A Sound Effect "Bullet Bys & Impacts" 是否实枪录制；Pole Position 105mm 页面未写型号。
14. 候选的型号吻合度（BAR M1918 vs A2、M1903 vs A4）；所有模型的网格质量、绑定质量、内饰有无。

**其它**

15. 音频码率、LUFS 目标、AudioBuffer 内存计算为 [M] 凭记忆的工程建议（见 `docs/ARCHITECTURE.md`/音频任务，非本表采购依据）；Safari Opus 支持状况须在目标版本实测。
16. 公共领域（PD）判断（I-PD-001）为 [M]，须律师/NARA/LOC 确认。

### 7.3 需负责人决定的事项（汇总）

1. **浏览器分发"可提取性"风险**是否可接受？是否采用 `.fpak` 容器 + 书面确认的缓解策略（§3.4）？
2. 对现成素材做**减面、重绑定、拼装服装/装具、重材质**是否算"自制"（D-MOD，决定 GAP-V01/V02/V04/V05/V06 能否用"买现成 + 加工"解决）？
3. **UI 图标**由执行者以 SVG 代码绘制是否可接受（NEEDS_HUMAN）？
4. **TurboSquid** 个案批准流程是否值得走？还是完全放弃 TurboSquid？
5. 是否接受**第一人称武器先以第三人称品质（2k–6k 顶点）上线 M1**？
6. **外包预算与范围**；真人配音/外包录制是否算"重新录音"（音频片段要求需人工决定，不得由执行者自行推进）。
7. 史实容忍度：MG34 → MG42 变调、Bazooka/Panzerschreck → 现代 RPG、P-47 → P-51、美式配音 → 英式配音、C-4 ↔ TNT 等替代是否可接受。
8. 免费游玩 + 捐赠是否满足各 itch.io 作曲者/卖家对"商用"的定义；项目收入/用户量接近 MoCap Online 阈值时的商业许可预算。

### 7.4 本次未搜索项（需补搜）

桥梁、火车站、农作物、血迹贴花、云层贴图、阿登冬季建筑、各兵种具体装具（背包、挂具）、军事版翻越/投掷/救治/建造动画、WW2 专用第一人称动画；音频方面：Airborne Sound、SoundMorph、Watson Wu、Red Libraries 未返回任何 WW2 枪械/坦克库（**未找到不等于不存在**）、军用卡车/半履带/吉普实录、匍匐/翻越/包扎/铲土军事 Foley、美式口音 WW2 士兵语音。

### 7.5 来源汇总（均为本次实际访问或搜索所得，核实日期 2026-11-17）

**视觉条款与渠道**：https://polyhaven.com/license ；https://docs.ambientcg.com/license/ ；https://www.fab.com/eula ；https://dev.epicgames.com/documentation/fab/licenses-and-pricing-in-fab ；https://forums.unrealengine.com/t/quixel-to-fab-transition-faqs/2033839.md ；https://www.cgchannel.com/?p=179945 ；https://www.unrealengine.com/en-US/eula/content ；https://forums.unrealengine.com/t/fab-ue-only-content-licensing/2082870 ；https://sketchfab.com/blogs/community/sketchfab-update-what-you-need-to-know-now-that-fabs-live/ ；https://sketchfab.com/blogs/community/kitbash-acquires-sketchfab-and-artstation ；https://www.gamedeveloper.com/business/epics-sells-artstation-and-sketchfab-to-kitbash ；https://www.turbosquid.com/help/en/articles/9937422-royalty-free-license ；https://help.cgtrader.com/hc/en-us/articles/360015124437 ；https://support.unity.com/hc/en-us/articles/34387186019988-Can-I-use-assets-from-the-Asset-Store-with-other-engines ；https://help.kitbash3d.com/en/articles/7939191-cargo-licensing-faqs ；https://community.adobe.com/t5/mixamo-discussions/mixamo-faq-licensing-royalties-ownership-eula-and-tos/m-p/13234775 ；https://mocaponline.com/pages/license ；https://rokoko.com/pricing/ ；https://www.reallusion.com/Content/EULA/EULA.htm

**视觉关键候选页（已打开）**：https://www.fab.com/listings/00a8235f-055a-4532-9916-fe060949b9eb （美军枪械包 $19.99）；https://www.fab.com/listings/13b4dea1-2849-480a-a3c7-b16fa5bd329a （德军枪械包 $19.99）；https://www.fab.com/listings/690cc828-da71-4778-9433-894a7c0d99f5 （Panzer IV $25.99）；https://www.fab.com/listings/ba4a316f-a363-4c2f-a776-26082681b91b （战壕套件 $179.99，仅 UE）；https://www.renderhub.com/loss/world-war-2-german-weapons （德军步兵武器 bundle $70）

**音频条款与渠道**：https://sonniss.com/gameaudiogdc ；https://rekkerd.org/sonniss-releases-gdc-2026-game-audio-bundle/ ；https://freesound.org/help/faq/ ；https://www.boomlibrary.com/sound-effects/world-war-ii-firearms/ ；https://www.boomlibrary.com/support/faq/what-am-i-allowed-to-do-with-your-sounds/ ；https://www.prosoundeffects.com/licensing/ ；https://www.soundsnap.com/licence ；https://asoundeffect.com/license-agreement ；https://getsoundly.com/faq/can-i-use-the-sounds-in-a-video-game/ ；https://www.epidemicsound.com/game-development/music-for-video-games/ ；https://help.artlist.io/hc/en-us/articles/33980035053213 ；https://www.zapsplat.com/license-agreement ；https://sound-effects.bbcrewind.co.uk/ ；https://www.openair.hosted.york.ac.uk/

---

## 8. 免费素材总表（零采购基线）

> 负责人 2026-11-17 指示：当前基线 = **零采购 / 免费优先**。本节列出两份调研（视觉、音频，核实日期 2026-11-17）里**所有"有免费候选"的需求 ID**；找不到免费素材的需求 ID 与购买渠道见 §9，覆盖率与里程碑影响见 §10。
> **链接、作者、许可、证据标签全部原样取自调研片段，没有新增或编造**；调研未给出完整 URL 的条目，原样写"调研未给 URL"。调研者**没有下载任何素材本体、没有聆听任何音频、没有注册账号**，所有"吻合度/不足"判断只来自页面文字、文件名或索引，**导入前必须打开页面确认许可、比例、贴图/绑定、音质**。
> 本节不是法律意见。

### 8.0 读表说明

#### 8.0.1 档位与列含义

| 项 | 含义 |
|---|---|
| **A 档** | 页面标注 CC0 / 公共领域 / CC-BY（游戏内署名）/ 平台免费商用许可（Fab Standard、CGTrader Royalty Free、Sonniss GameAudioGDC 许可等）。按负责人指示，**只记录页面标注的许可名称**，不逐条核对条款，不要求卖家书面确认。 |
| **B 档** | 页面标注 CC-BY-NC / CC-BY-SA / CC-BY-NC-SA / CC-BY-NC-ND / 仅个人使用 / **许可未写明** / **来源存疑**（权属或出处无法判断）。是否使用由负责人决定（NEEDS_HUMAN H-101）；使用时在 `LICENSES.md` 登记为 `FLAGGED`。 |
| 吻合度 | **准确** = 型号/用途对得上（不等于品质达标）；**近似** = 不是该型号，可作最接近替代；**仅可占位** = 风格或精度不足，只适合开发期占位；**部分** = 音频只覆盖该 ID 的一部分（如缺近场/缺全自动）。 |
| 证据 | **[F]** = 调研时直接读到该页面/官方 API/文件索引；**[S]** = 仅搜索结果摘要，未打开页面；**[M]** = 凭记忆，未核实。 |
| 状态 | `[ ] 已下载`：负责人（或执行者）下载后勾选，并把文件放入 `assets/incoming/{需求ID}/`，同时在 `LICENSES.md` §6 简化登记。 |

#### 8.0.2 免费渠道简表（页面标注的许可；摘自调研渠道表）

| 渠道 | 页面标注的许可 | 说明 | 证据 |
|---|---|---|---|
| Poly Haven | CC0（"any purpose, including commercial work"） | 模型 522 / 贴图 870 / HDRI 997（API 计数 2026-11-17）；站点文字/Logo/示例渲染不在 CC0 内 https://polyhaven.com/license | [F] |
| ambientCG | CC0 1.0（可 "include the raw files in a project, such as a video game"） | 材质约 2042 / HDRI 439 / 贴花 127；基本无 3D 模型 https://docs.ambientcg.com/license/ | [F] |
| Fab 免费内容 + Megascans | Fab Standard License | 2026-11-05 公告 Megascans 全库免费且 "for good"；调研**未逐个打开商品页确认价格=0 与许可栏**，下载时须逐件截图确认；Nanite 专用资产对 Babylon 无用 https://www.cgchannel.com/?p=179945 | [F]（新闻层面） |
| Sketchfab（CC 模型） | 每模型各自（见 CC BY 4.0、CC BY-SA、CC BY-NC-SA、CC BY-NC-ND；有些页面不显示许可也无下载按钮） | 调研**未能使用站内"可下载"筛选**，召回率偏低；逐件看页面许可与作者来源 | [F]/[S] |
| Sonniss GameAudioGDC（2015–2024、2026；无 2025） | Sonniss "Unlimited User License" v2.0：免版税、无需署名、无限项目；禁止把原件或改动件作为"音效"再分发；禁止用于 AI/ML 训练 | 下载页 https://sonniss.com/gameaudiogdc ；许可 https://sonniss.com/gdc-bundle-license/ ；本节引用文件名来自第三方索引 https://github.com/yansixing/gdc-bundle-finder（仅文件名/大小，未下载、未聆听） | [F] |
| Freesound | 每条各自：CC0 / CC-BY / CC-BY-NC / Sampling+（旧） | 本节候选**全部为 CC0**（过滤串 `license:"Creative Commons 0"`）；游戏翻录泛滥，逐条核对（见 §8.22） https://freesound.org/help/faq/ | [F] |
| Kenney | CC0（"public domain licensed (CC0)"） | 风格化低模/UI/音频，仅占位与 UI https://kenney.nl/support | [F] |
| Quaternius | 各包页：CC0 | 风格化植被/低模，仅占位 | [S] |
| OpenGameArt | 逐条：CC0 / CC-BY / CC-BY-SA / GPL | FAQ 页 403 未能访问；只取 CC0，逐条看评论 | [S] |
| itch.io 免费/PWYW 包 | 各自；本次读到的两个 WWII 包页面均**未写许可**，一个自述 CC0 | 多为 B 档 | [F] |
| Mixamo | 社区转述 FAQ：个人/商业/非营利可用，含电子游戏，署名不要求；不得转售或批量下载 | 官方 FAQ 页 403；需 Adobe ID；2026 年第三方指南称 "unmaintained" | [S] |
| CMU Mocap Database | 摘要转述官方 FAQ："free for use in research and commercial projects"，请求署名 | FAQ 页 503 未能访问；4TU 的 FBX 转换版标 CC BY-NC 4.0（B 档） | [S] |
| Incompetech（Kevin MacLeod） | CC BY 4.0 | 署名随游戏进 Credits 页 https://incompetech.com/music/royalty-free/faq.html | [F] |

### 8.1 武器（V-WPN-US-001..012、V-WPN-GE-001..014）

**覆盖率 14/26**（准确 9、近似/占位 3、仅 B 档 2；12 项无合格候选见 §9.1）。**所有 A 档 3D 武器面数几乎全在 4.5k–20k（第三人称区间），没有第一人称级（25k–60k + 内部结构 + 可动部件）免费模型**；拆分枪机/弹匣为独立网格需在 DCC 里切模型，属 D-MOD 边界（待确认，GAP-V06）。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-WPN-US-001 | M1 Garand（第一人称 25k–60k，可动部件） | M1 | ① 'M1 Garand, America's WW2 Work Horse'｜CannonMonkey｜https://sketchfab.com/3d-models/m1-garand-americas-ww2-work-horse-28d1e527d7f546a4b904d869ceba4aa2（4.5k 三角，Blender+Substance，可下载）<br>② 'M1 Garand'｜CG_Soup｜https://sketchfab.com/3d-models/m1-garand-4dac51f2da3642d1b66838eac3126cdd（20.6k 三角，可下载 1598 次，tag 含 pbr） | 'Mini pack of weapons of the second world war'（含 M1 Garand、Colt M1911 等，每枪带弹壳/弹匣）｜victorcstr｜https://victorcstr.itch.io/mini-pack-of-weapons-of-the-second-world-war｜**许可未写明**（PWYW 可付 0）；OBJ+.blend，合计约 14k 三角，无贴图 | ①② CC BY 4.0（需署名）；B 档：未写明 | 准确（型号）；仅第三人称精度：①面数不足；②面数最接近第一人称但作者自述首作、无可动部件拆分说明；贴图情况多未写 | [F] | [ ] 已下载 |
| V-WPN-US-003 | M1A1 Thompson（含弹匣/鼓，全自动） | M1 | ① 'Thompson M1A1'｜Bora Özakaltun｜https://sketchfab.com/3d-models/thompson-m1a1-67977e1f6fac4feda2802c770b3d9271（约 5k 三角，Blender；需确认 Sketchfab 页是否仍可下载）<br>② 'The Survivalists Thompson'｜作者调研未给｜https://sketchfab.com/3d-models/the-survivalists-thompson-101f7a2c6f5146968f51aa7681d7592c（是 M1 而非 M1A1，游戏设计硕士项目） | — | CC Attribution（搜索摘要） | ①准确 / ②近似；仅第三人称精度；两条都只读到搜索摘要，WebFetch 返回空 | [S] | [ ] 已下载 |
| V-WPN-US-004 | M3 Grease Gun | M3 | 'Free CC0 guns & explosives pack'（19 个模型：Grease gun M3、Luger、Frag Grenade、M24 Grenade、Smoke grenade、Anti-tank mine、Claymore 等；rar 154 MB）｜3dmodelscc0（itch.io）｜https://3dmodelscc0.itch.io/free-cc0-guns-explosives-pack | — | CC0（页面自述） | 准确（型号）；面数未写；**来源中风险**：页面写明这些模型来自已关闭的旧站、批量上传、无制作过程图（作者回复称全部自建），须看实物拓扑后再定；本包同时是 US-011、GE-008、GE-011、GE-013 的候选 | [F] | [ ] 已下载 |
| V-WPN-US-005 | M1918A2 BAR | M1 | 无 A 档候选 | BAR 1918｜MikeeUSA｜https://opengameart.org/content/browning-automatic-rifle-1918-bar｜**CC-BY-SA / GPL v2 双许可**（B 档）；非 A2 版需核对 | B 档：CC-BY-SA / GPL v2 | 仅 B 档（SA/GPL）；另有两个 Sketchfab 低模（4k / 2.5k 三角）许可未见，调研未列为候选 | [S] | [ ] 已下载 |
| V-WPN-US-008 | M1911A1 | M1 | 无 A 档候选 | ① 'M1911 Pistol - Game Ready'｜fadlparlak｜https://sketchfab.com/3d-models/m1911-pistol-game-ready-bdc0fe27306d48b0b2b23967b159971c｜**许可未写明**，页面链到 Fab 列表，是否可直接下载未写明；8.5k 三角，PBR，已绑定<br>② 'M1911 ｜ Post Apocalyptic Series'｜creationwasteland｜仅有聚合站链接 https://stlfinder.com/model/camilo/6292638（原 Sketchfab 页未打开，聚合站按调研口径不作可靠来源）｜CC Attribution（摘要）<br>③ victorcstr 包内 Colt M1911（见 V-WPN-US-001 的 B 档，许可未写明） | B 档：未写明 / 来源存疑 | 仅 B 档；①型号为 M1911 但许可与下载状态不明；②需回原页核对 | [F]（①）/[S]（②） | [ ] 已下载 |
| V-WPN-US-011 | M18 烟雾手雷（小件） | M2 | 3dmodelscc0 包内 'Smoke grenade'（通用，非 M18）｜3dmodelscc0｜https://3dmodelscc0.itch.io/free-cc0-guns-explosives-pack | — | CC0（页面自述） | 仅可占位（通用烟雾手雷）；来源评估同 V-WPN-US-004 | [F] | [ ] 已下载 |
| V-WPN-GE-001 | Kar98k（栓动，M0/M1） | M0 | 近似占位：'Bolt Action Rifle 7.62'｜Mateusz Sadek｜https://polyhaven.com/a/bolt_action_rifle_7_62（约 19,985 三角，8K 贴图，带分离弹匣与瞄准镜、磨损木托） | — | CC0（Poly Haven） | 近似/仅可占位：**不是 Kar98k**（长枪管/小瞄准镜/弹匣可拆，更像 Mosin 型狙击枪），可作栓动步枪占位，不可冒充 Kar98k | [F] | [ ] 已下载 |
| V-WPN-GE-004 | MP40（折叠枪托） | M1 | 'MP40 - Final'｜Matt Rafferty｜https://sketchfab.com/3d-models/mp40-final-3685dd295e5d4773bdf59929e2dae71c（约 81k 三角，Maya 2017 + Substance Painter，测试导出） | — | CC Attribution（搜索摘要） | 准确（型号）；面数过高需减面（D-MOD 允许）；WebFetch 返回空，仅搜索摘要 | [S] | [ ] 已下载 |
| V-WPN-GE-005 | StG 44（弧形弹匣） | M1 | 'Sturmgewehr STG 44 Mid-poly'｜leimoses3｜https://sketchfab.com/3d-models/sturmgewehr-stg-44-mid-poly-888beb836d784500b9d428bad4a0c404（13.8k 三角/7.1k 顶点，可下载 350 次，页面含史实介绍） | — | CC BY 4.0（需署名） | 准确（型号）；第三人称精度；无制作过程图（提取嫌疑 低-中） | [F] | [ ] 已下载 |
| V-WPN-GE-007 | MG42（弹链、两脚架） | M1 | 'Low-Poly MG 42'｜TastyTony｜https://sketchfab.com/3d-models/low-poly-mg-42-141e542d223d483a88f04c5d1ae79872（11.7k 三角，可下载 3,667 次，未写贴图/绑定） | Sketchfab 'MG42'（2.5k 三角，2020；作者调研未给）｜https://sketchfab.com/3d-models/mg42-3bbe7e01e10949b2a923dcf3b5b83057｜**CC BY-SA** | A 档：CC BY 4.0；B 档：CC BY-SA | 准确（型号）；第三人称精度；未写贴图/绑定 | [F]（A）/[S]（B） | [ ] 已下载 |
| V-WPN-GE-008 | Luger P08 / Walther P38 | M1 | 3dmodelscc0 包内 'Luger'（P08）｜3dmodelscc0｜https://3dmodelscc0.itch.io/free-cc0-guns-explosives-pack；P38：无 | 'German P.08 Luger Pistol (VCU_3D_1497)'｜VCU 3D｜https://sketchfab.com/3d-models/german-p08-luger-pistol-vcu-3d-1497-74f9047ea99249aa8645212492839916｜**CC BY-NC-ND**（摘要）；ND 禁止改动（含减面），实际不可用，仅备查 | A 档：CC0（页面自述）；B 档：CC BY-NC-ND | 准确（仅 P08；P38 无）；来源评估同 V-WPN-US-004 | [F]（A）/[S]（B） | [ ] 已下载 |
| V-WPN-GE-010 | Panzerfaust（30/60/100 型） | M2 | A-档：'Panzerfaust WW2'（低模，2K PBR）｜Lord_akaal｜https://www.cgtrader.com/3d-models/military/gun/panzerfaust-ww2｜页面被摘要标"免费"，CGTrader Royalty Free（No AI）；免费状态与许可栏需再确认 | ① free3d 'low poly panzerfaust 2061'｜https://free3d.com/3d-model/low-poly-panzerfaust-2061.html｜摘要同时出现 "Royalty Free" 与 "Editorial"，**许可状态不清**<br>② BlendSwap 'Panzerfaust Modified'｜https://www.blendswap.com/blend/28561｜CC-BY，**下载需登录（未注册，未打开）** | A-档：CGTrader Royalty Free（No AI）；B 档：状态不清 / CC-BY（需登录） | 准确（型号）；免费状态未确认；A- 档不是 CC0/CC-BY，属平台免费商用许可 | [S] | [ ] 已下载 |
| V-WPN-GE-011 | M24 Stielhandgranate（小件） | M1 | ① 3dmodelscc0 包内 'M24 Grenade'｜3dmodelscc0｜https://3dmodelscc0.itch.io/free-cc0-guns-explosives-pack<br>② A-档：CGTrader 免费 'world war ii german grenade'（低模）｜https://www.cgtrader.com/free-3d-models/military/other/world-war-ii-german-grenade | victorcstr 包内 Stielhandgranate｜https://victorcstr.itch.io/mini-pack-of-weapons-of-the-second-world-war｜许可未写明，无贴图 | A 档：①CC0（页面自述）；②CGTrader Royalty Free（No AI）；B 档：未写明 | 准确（型号）；来源评估同 V-WPN-US-004；②为搜索摘要 | [F]（①）/[S]（②） | [ ] 已下载 |
| V-WPN-GE-013 | Tellermine 35/42（反坦克地雷） | M3 | 3dmodelscc0 包内 'Anti-tank mine'（通用，形状待核）｜3dmodelscc0｜https://3dmodelscc0.itch.io/free-cc0-guns-explosives-pack | — | CC0（页面自述） | 仅可占位（通用反坦克地雷）；来源评估同 V-WPN-US-004 | [F] | [ ] 已下载 |

### 8.2 角色（V-CHR-*）

**覆盖率 0/9**：A、B 档均未找到任何免费、带绑定的 WWII 士兵（搜索只见 CGTrader/RenderHub 付费或 3D 打印用模型）。这是零采购档**最大且唯一无法用加工弥补的缺口**：D-MOD 不允许从零建模，没有基础身体就无法拼装服装模块。全部 9 项见 §9.1。

下面只是**占位**（不计入覆盖率；不是 WWII 角色，只可作 bot/调试占位）：

| 占位 | 来源 | 页面标注的许可 | 证据 |
|---|---|---|---|
| Mixamo 自带通用人形角色与自动绑定（任意 FBX 上传后自动绑骨） | Adobe｜https://www.mixamo.com | 社区转述 FAQ（见 §8.0.2） | [S] |
| Kenney 低模人偶 | Kenney｜https://kenney.nl/support | CC0 | [F] |
| Quaternius 低模人偶 | Quaternius（调研未给人偶包链接） | CC0 | [S] |

### 8.3 动画（V-ANM-001..010）

**覆盖率 7/10**（全部为通用/近似，**准确 0**）；V-ANM-006、-008、-009 无合格候选（§9.1）。免费库**没有步枪专用、栓动、第一人称、载具乘员动画**；通用动作需重定向，手感与 AAA 差距大。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-ANM-001 | 第三人称步枪运动集（站/走/跑/蹲/跳、基础投掷） | M0/M1 | ① Mixamo｜Adobe｜https://www.mixamo.com（未打开；需 Adobe ID，未注册）<br>② CMU Mocap Database（BVH/C3D 原版）｜CMU｜http://mocap.cs.cmu.edu/faqs.php（FAQ 页 503，未能访问） | ① 'Free Pistol Animation Starter Pack'｜MoCap Online｜https://mocaponline.itch.io/free-pistol-animation-starter-pack｜标准许可（非开源）；发布者留言：可用于游戏，成功后或承包项目需商业许可；FBX/BIP 38 MB；**只有手枪**，不含换弹/冲刺/8 向<br>② Bandai Namco Research Motion Dataset｜https://github.com/BandaiNamcoResearchInc/Bandai-Namco-Research-Motiondataset｜CC BY-NC 4.0（不可商用）<br>③ Rokoko 免费动作｜https://rokoko.com/pricing/｜2020 年称可商用，当前条款未读 | Mixamo：社区转述 FAQ——可用于商业游戏，不要求署名，不得转售/批量下载（官方 FAQ 页 403）；CMU：摘要转述"free for use in research and commercial projects"，请求署名；B 档：见左 | 近似（通用动作）：**免费步枪专用动画无**；CMU 需重定向且无 WWII 持枪姿，工作量大；Mixamo 2026 年第三方指南称 "unmaintained"、2025-06 后有登录/下载故障报告 | [S]/[M]（Mixamo 具体步枪动作凭记忆） | [ ] 已下载 |
| V-ANM-002 | 匍匐/趴姿（持枪、转向、射击、换弹） | M1 | Mixamo / CMU 有爬行类通用动作（凭记忆）；持枪匍匐无｜Adobe（Mixamo）｜https://www.mixamo.com；CMU http://mocap.cs.cmu.edu/faqs.php（503，未能访问） | — | 同 V-ANM-001（Mixamo 社区转述；CMU 摘要转述） | 近似（通用动作，需重定向；非军事版/无持枪版）；具体有哪些动作凭记忆，未核实 | [M] | [ ] 已下载 |
| V-ANM-003 | 翻越矮墙/攀爬/跳跃/落地 | M2 | Mixamo 通用动作；CMU（摘要）｜Adobe（Mixamo）｜https://www.mixamo.com；CMU http://mocap.cs.cmu.edu/faqs.php（503，未能访问） | — | 同 V-ANM-001（Mixamo 社区转述；CMU 摘要转述） | 近似（通用动作，需重定向；非军事版/无持枪版）；具体有哪些动作凭记忆，未核实 | [M]/[S] | [ ] 已下载 |
| V-ANM-004 | 投掷（手雷）、扔烟 | M1 | Mixamo 通用投掷｜Adobe（Mixamo）｜https://www.mixamo.com | — | 同 V-ANM-001（Mixamo 社区转述；CMU 摘要转述） | 近似（通用动作，需重定向；非军事版/无持枪版）；具体有哪些动作凭记忆，未核实 | [M] | [ ] 已下载 |
| V-ANM-005 | 受击/倒地/死亡（多方向） | M1 | Mixamo 通用受击/倒地/死亡；CMU（摘要）｜Adobe（Mixamo）｜https://www.mixamo.com；CMU http://mocap.cs.cmu.edu/faqs.php（503，未能访问） | — | 同 V-ANM-001（Mixamo 社区转述；CMU 摘要转述） | 近似（通用动作，需重定向；非军事版/无持枪版）；具体有哪些动作凭记忆，未核实 | [M]/[S] | [ ] 已下载 |
| V-ANM-007 | 建造/挖掘/修理 | M2 | Mixamo 部分通用动作｜Adobe（Mixamo）｜https://www.mixamo.com | — | 同 V-ANM-001（Mixamo 社区转述；CMU 摘要转述） | 近似（通用动作，需重定向；非军事版/无持枪版）；具体有哪些动作凭记忆，未核实 | [M] | [ ] 已下载 |
| V-ANM-010 | 指挥官手势/敬礼/标记动作 | M3 | Mixamo 有敬礼/挥手类通用动作｜Adobe（Mixamo）｜https://www.mixamo.com | — | 同 V-ANM-001（Mixamo 社区转述；CMU 摘要转述） | 近似（通用动作，需重定向；非军事版/无持枪版）；具体有哪些动作凭记忆，未核实 | [M] | [ ] 已下载 |

### 8.4 载具（V-VEH-*）

**覆盖率 5/22**（准确 3、近似 1、仅 B 档 1；17 项无合格候选见 §9.1）。仅 Sherman（2 个，无贴图无绑定）、Panzer IV（1）、Panther（高面数）；**坦克无内饰、无可动部件**；无半履带/卡车/吉普/侦察车/火炮/飞机。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-VEH-US-003 | M4 Sherman（75mm，M1 的 1 辆坦克） | M1 | ① 'Low Poly M4 Early'｜Dezryelle｜https://sketchfab.com/3d-models/low-poly-m4-early-19f4f2295b6044c88a716ca371f3e502（30k 三角/17.1k 顶点，可下载 189 次，"历史上准确的中期 M4(75)"）<br>② 'Low Poly M4A3(75)W Sherman tank'｜Dezryelle｜https://sketchfab.com/3d-models/low-poly-m4a375w-sherman-tank-0d5533e114bf4849bc8c9213e4ee9cf9（25k 三角，UV 已展开，可下载 400 次，"1944 末期 M4A3(75)W"） | 待核：Toshuey 'M4A3 (75)W Sherman Medium Tank'（229.4k 三角）｜https://www.sketchfab.com/3d-models/m4a375w-sherman-medium-tank-toshueyi-4fc8d15fb5ae4f9490ae053b8c09e10b｜CC BY（摘要，页面未打开） | CC BY 4.0（需署名） | 准确（型号/年代）；**无贴图、未绑定、无内饰**；只能做材质参数调整；分拆炮塔/炮/履带为独立节点需切分网格（D-MOD 待确认，见 GAP-V01） | [F] | [ ] 已下载 |
| V-VEH-US-004 | M4A3(76)W / Jumbo | M3 | 'Low poly M4A3(76) HVSS'｜Dezryelle｜https://sketchfab.com/3d-models/low-poly-m4a376-hvss-710e93ef16c84160a5e8b980ce07163a | — | CC BY（摘要，同作者同系列） | 近似：HVSS 悬挂的 M4A3(76)W（页面写 untextured/unrigged）；Jumbo（M4A3E2）无 | [S] | [ ] 已下载 |
| V-VEH-GE-003 | Panzer IV（Ausf. H/J 为 1944 主力） | M1 | 'Tank PZ_IV'｜Don Adam｜https://sketchfab.com/3d-models/tank-pz-iv-3d11217421fc4eac80698d2c69c17fed（22.2k 三角/13.8k 顶点，可下载 222 次，3ds Max + Substance Painter，PBR 贴图，"为游戏优化"） | — | CC BY 4.0（需署名） | 准确（车型）；Ausf. 页面未写，需看图核对 H/J 型；无内饰；提取嫌疑 低-中（有制作流程自述） | [F] | [ ] 已下载 |
| V-VEH-GE-004 | Panther（Ausf. G） | M3 | 'Panzer V Panther medium tank'｜作者调研未给｜https://www.sketchfab.com/3d-models/panzer-v-panther-medium-tank-b4693513fec841bbad571fbd0120ce95（约 211.5k 三角，2015 年上传）；另有 'tankerpanzer'（约 127.5k 三角，描述稀疏，调研不推荐）https://sketchfab.com/3d-models/tankerpanzer-402a8fa6dc5d40e38e456f0b222b1144 | — | CC Attribution（搜索摘要） | 准确（车型）；Ausf. 未核；需大幅减面；无内饰 | [S] | [ ] 已下载 |
| V-VEH-GE-005 | Tiger I | M3 | 无 A 档候选 | OpenGameArt 'tank (panzer tiger)'｜作者调研未给｜https://opengameart.org/content/tank-panzer-tiger｜CC-BY 3.0，**但评论者指出贴图许可有疑问** | B 档：CC-BY 3.0（页面评论质疑贴图许可） | 仅 B 档；贴图许可存疑 | [S] | [ ] 已下载 |

### 8.5 建筑与掩体（V-BLD-001..010）

**覆盖率 5/10**（准确 1、近似 3、仅 B 档 1；5 项无合格候选见 §9.1）。**没有诺曼底/阿登民居、教堂、街道**；只有 Regelbau 低模和几个英国/泽西/Île de Ré 扫描。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-BLD-001 | 诺曼底石造民居模块套件（20–40 个模块） | M1 | 近似（材质来源，非建筑）：Poly Haven 材质 `rough_plaster_brick`、`plastered_stone_wall`、`stacked_stone_wall`、`old_stone_wall`、`church_bricks_02/03`、`clay_roof_tiles`、`grey_roof_tiles`、`slate`（asset 页格式 https://polyhaven.com/a/<asset_id>）；ambientCG Bricks/Plaster 系列（https://docs.ambientcg.com/license/）<br>近似扫描（非诺曼底）：'Abandonned farmhouse'（瑞士，293 张无人机照片）｜作者调研未给｜https://sketchfab.com/3d-models/abandonned-farmhouse-b35de1cb85ed4eb8924234943dd4056d | — | Poly Haven / ambientCG：CC0；农舍扫描：CC Attribution（摘要） | 近似：**无整套模块**；D-MOD 不允许从零建模，用材质拼装模块民居不成立；整体扫描体可玩性（进出、窗口、多层）受限；M1 仍以灰盒盒体 + 墙面材质占位（GAP-V02） | [F]（材质）/[S]（扫描） | [ ] 已下载 |
| V-BLD-002 | 农场建筑（谷仓、牲畜棚、苹果园棚） | M1 | 近似同 V-BLD-001（瑞士废弃农舍扫描 https://sketchfab.com/3d-models/abandonned-farmhouse-b35de1cb85ed4eb8924234943dd4056d；材质同上） | — | 同 V-BLD-001 | 近似；无诺曼底型；'Old barn georeferenced scan' 许可未见，调研未列为候选 | [S] | [ ] 已下载 |
| V-BLD-006 | 大西洋壁垒碉堡（Regelbau）、Tobruk 掩体 | M4 | ① 'German WWII Bunker Regelbau M178'｜ustoopia｜https://sketchfab.com/3d-models/german-wwii-bunker-regelbau-m178-078dfe6b2e4f47dca2eedf0b7752726a（2.4k 三角/1.3k 顶点，可下载 304 次，仿 IJmuiden 真实碉堡，手建）<br>② 近似扫描（英国非德军）'Greenway Pillbox'｜artfletch｜https://sketchfab.com/3d-models/greenway-pillbox-44c5481094de4f43a40909b244dd6942（1.8M 三角，Reality Capture，可下载 239 次） | ① 'German machine gun emplacement R.N. La Mare Mill'｜Underground｜https://sketchfab.com/3d-models/german-machine-gun-emplacement-rn-la-mare-mill-ecf4e30ce9644a9599dffb6d6be5d065｜CC BY-NC-SA 4.0；221.4k 三角（泽西岛，大西洋壁垒）<br>② 'German WWII bunker fading into the sand'（法国 Île de Ré）｜https://www.sketchfab.com/3d-models/german-wwii-bunker-fading-into-the-sand-1-62c836a9acfd476998b906bad5795307｜CC BY-NC-SA（摘要）<br>③ 'nuclearwinter94 3D trench warfare low poly'（含 1 个碉堡）｜nuclearwinter94｜https://nuclearwinter94.itch.io/tre｜许可未写明 | A 档：CC BY 4.0；B 档：CC BY-NC-SA / 未写明 | ①准确（型号 M178，面数偏低）；②近似（需大幅减面 + 烘焙）；Tobruk 无 | [F]（①②及 B①③）/[S]（B②） | [ ] 已下载 |
| V-BLD-007 | 滩头障碍（捷克刺猬、比利时门、木桩） | M4 | 无 A 档候选 | ① itch '3D trench warfare low poly'（含捷克刺猬）｜nuclearwinter94｜https://nuclearwinter94.itch.io/tre｜许可未写明；glb/fbx/obj/blend，PWYW，低模<br>② 线索（许可未见，调研未列为候选）：Sketchfab 'Czech Hedgehog Seaweed' | B 档：未写明 | 仅 B 档；比利时门/木桩未见 | [F] | [ ] 已下载 |
| V-BLD-010 | 废墟/受损建筑件 | M2 | 近似（材质）：Poly Haven `broken_brick_wall`、`concrete_debris`、`rough_plaster_broken`、`damaged_plaster`（asset 页格式 https://polyhaven.com/a/<asset_id>） | itch '3D trench warfare low poly' 内含 1 个受损建筑｜nuclearwinter94｜https://nuclearwinter94.itch.io/tre｜许可未写明 | A 档：CC0（Poly Haven）；B 档：未写明 | 近似：A 档只有材质，无整体模型 | [F] | [ ] 已下载 |

### 8.6 自然（V-NAT-001..008）

**覆盖率 7/8**（准确 5、近似 2；V-NAT-008 水见 §9.1）。地表/HDRI/岩石/冷杉强；**落叶乔木、bocage、农作物是真实缺口**。Poly Haven 链接格式为 `https://polyhaven.com/a/<asset_id>`（调研原文；asset_id 来自官方 API，2026-11-17 抓取）。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-NAT-001 | 树篱 bocage（土堤 + 灌木，可作掩体） | M1 | 近似拼装（需 D-MOD"多个部件组合装配"确认）：Poly Haven `shrub_01..04`、`fern_02`、`grass_medium_01/02`、`moss_01`，材质 `stacked_stone_wall`/`mossy_stone_wall`/`forest_floor`（asset 页格式 https://polyhaven.com/a/<asset_id>）；Megascans 土堤/灌木（Fab Standard，未逐件确认）https://www.cgchannel.com/?p=179945 | — | CC0（Poly Haven）；Megascans：Fab Standard License（需逐件确认价格=0） | 近似：**无 bocage 扫描**；真实缺口（GAP-V03） | [F]（Poly Haven）/[S]（Megascans） | [ ] 已下载 |
| V-NAT-002 | 诺曼底落叶乔木（橡、榉、苹果树） | M1 | ① Poly Haven `tree_small_02`｜http://polyhaven.com/a/tree_small_02<br>② 'Textured LowPoly Trees'（45 棵，风格化）｜Quaternius｜https://quaternius.itch.io/textured-lowpoly-trees<br>③ Sketchfab 'apple tree'（可作果树）｜作者调研未给｜https://sketchfab.com/3d-models/apple-tree-512db13e35f9449cbd4215389edd905e<br>④ Megascans 欧洲山毛榉/橡树（Fab Standard，未逐件核；Nanite 版对 Babylon 不可用） | — | ① CC0；② CC0；③ CC Attribution（摘要）；④ Fab Standard | 近似：缺欧洲橡/榉的写实扫描；Quaternius 为风格化 | [F]（①）/[S]（②③④） | [ ] 已下载 |
| V-NAT-003 | 阿登冬季云杉 + 积雪 | M4 | ① 'Fir Tree 01'｜Rob Tuytel（摄影）/Rico Cilliers（建模）｜https://polyhaven.com/a/fir_tree_01（约 7,853,731 三角，8K，带 LOD）<br>② 'Pine Tree 01'｜同作者｜https://polyhaven.com/a/pine_tree_01（约 17.4M 三角，带 LOD）<br>③ `fir_sapling`、`fir_sapling_medium`<br>雪面：Poly Haven `snow_01..05`、`snow_floor`、`asphalt_snow`、`snow_field_aerial`；ambientCG Snow001–015 | — | CC0 | 准确；**面数极高，需 impostor/减面** | [F] | [ ] 已下载 |
| V-NAT-004 | 草、灌木、蕨、农作物、苹果园 | M1 | Poly Haven `grass_medium_01/02`、`grass_bermuda_01`、`shrub_01..04`、`fern_02`、`weed_plant_02`、`nettle_plant`、`periwinkle_plant`；苹果树：Sketchfab 'apple tree'（https://sketchfab.com/3d-models/apple-tree-512db13e35f9449cbd4215389edd905e）、itch 'Apple Tree GLB FBX'｜venturon｜https://venturon.itch.io/apple-tree（卖家自称 CC0，未打开）；CGTrader 免费低模苹果树（Royalty Free No AI） | — | CC0（Poly Haven）；苹果树：CC Attribution（摘要）/ 卖家自称 CC0 / CGTrader Royalty Free | 准确（草/灌/蕨）；**农作物（麦/玉米）无**（GAP-V14） | [F]（Poly Haven）/[S]（其余） | [ ] 已下载 |
| V-NAT-005 | 岩石、悬崖、海崖 | M1 | Poly Haven `boulder_01`（约 124k 三角）、`coastal_cliff_01/02/04`（`coastal_cliff_01` 约 866k 三角）、`coast_rocks_01..05`、`coast_land_rocks_*`、`stone_01`；（`namaqualand_*` 为南非沙漠地貌，不贴合） | — | CC0 | 准确；高面数需减面 | [F] | [ ] 已下载 |
| V-NAT-006 | 海滩沙/卵石/海草/潮线材质 | M4 | Poly Haven `coast_sand_01..05`、`damp_beach_sand`、`damp_beach_sand_02`、`damp_sand`、`dense_sand`、`sand_01..03`、`gravel_*`、`pebbles`；ambientCG 沙/卵石系列 | — | CC0 | 准确（材质）；海草/潮线专用无 | [F] | [ ] 已下载 |
| V-NAT-007 | 地表材质（泥、车辙、积雪、犁沟、石板路、草地、碎石、混凝土、焦土） | M0 | Poly Haven `brown_mud*`、`muddy_tracks`、`farm_furrows`、`farm_soil`、`forest_floor`、`forest_ground_04..06`、`burned_ground_01`、`snow_*`、`cobblestone_*`、`gravel_*`、`concrete_*`、`grass_*`（870 张材质）；ambientCG Ground/Gravel/Snow（约 2042 张材质）；ShareTextures https://www.sharetextures.com/p/terms（手动下载）；cgbookcase https://cgtricks.com/free-textures-cgbookcase-dorian-zgraggen；3dtextures.me https://3dtextures.me/about | — | CC0（Poly Haven、ambientCG、cgbookcase、3dtextures.me）；ShareTextures 自称 "Custom CC0"（仅限直接下载） | 准确；**整个清单里免费覆盖最强的一项** | [F]（Poly Haven/ambientCG）/[S]（其余） | [ ] 已下载 |

### 8.7 道具（V-PRP-001..010）

**覆盖率 9/10**（准确 3、近似 2、仅 B 档 4；V-PRP-007 见 §9.1）。箱/桶/油桶/电台尚可；沙袋/铁丝网/障碍/战壕只有一个许可未写明的 itch 低模包。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-PRP-001 | 沙袋（墙、角、射击孔） | M1 | 无 A 档候选 | itch '3D trench warfare low poly'（含沙袋）｜nuclearwinter94｜https://nuclearwinter94.itch.io/tre｜许可未写明；PWYW；glb/fbx/obj/blend；低模，面数未写 | B 档：未写明 | 仅 B 档（Poly Haven 只有 `cement_bag`、`compost_bags` 等非军用） | [F] | [ ] 已下载 |
| V-PRP-002 | 铁丝网（蛇腹、桩） | M1 | 无 A 档候选 | ① 同上 trench 包（含铁丝网元素）｜https://nuclearwinter94.itch.io/tre｜许可未写明<br>② Sketchfab 'Barbed Wire Fence WWII'｜KangaroOz｜https://sketchfab.com/3d-models/barbed-wire-fence-wwii-242e75d10a1f416fa8519f2d1a5d7c3c｜**许可未见**，未打开页面（清单所列约 1.1k 三角） | B 档：未写明 / 许可未见 | 仅 B 档 | [F]（①）/[S]（②） | [ ] 已下载 |
| V-PRP-003 | 反坦克障碍（龙牙、刺猬） | M1 | 无 A 档候选 | 同上 trench 包内捷克刺猬｜https://nuclearwinter94.itch.io/tre｜许可未写明；龙牙无 | B 档：未写明 | 仅 B 档 | [F] | [ ] 已下载 |
| V-PRP-004 | 弹药箱/补给箱（美式/德式） | M1 | ① 'Ammo Box'｜DanKit｜https://polyhaven.com/a/ammo_box（约 4,382 三角，4K，橄榄绿金属弹药箱）<br>② 'Old Military Crate'｜Jack Mava｜https://polyhaven.com/a/old_military_crate（约 10,476 三角，8K，开/合两态）<br>③ 'Wooden Military Crate'｜Prabhjinder Singh｜https://polyhaven.com/a/wooden_military_crate（约 22,986 三角，4K） | — | CC0 | 准确（通用军用箱）；美式 M2A1 风格需看实物；箱体手绘/模板文字需检查是否带现代标记或商标（D-MOD 例外允许遮盖） | [F] | [ ] 已下载 |
| V-PRP-005 | 野战无线电（SCR-300/536/Torn.Fu.） | M2 | 近似：'Vintage Radio Transceiver'｜Mateusz Sadek｜https://polyhaven.com/a/vintage_radio_transceiver（约 42,350 三角，4K，军绿壳体、频率盘、天线、桌面话筒与电键） | — | CC0 | 近似：桌面电台，不是 SCR-300 背负式（GAP-V12） | [F] | [ ] 已下载 |
| V-PRP-006 | 木箱、木桶、油桶、农用工具 | M1 | Poly Haven `wooden_crate_01/02`（https://polyhaven.com/a/wooden_crate_02）、`wooden_barrels_01`、`Barrel_01/02`、`barrel_03`、`metal_jerrycan`、`metal_jerrycan_green`、`wooden_bucket_01/02`、`wooden_ladder`、`trowel_01` | — | CC0 | 准确；jerrycan 为 20 L 型，德军 Wehrmacht-Einheitskanister 外形需看图 | [F] | [ ] 已下载 |
| V-PRP-008 | 散兵坑/战壕（可挖掘模块） | M2 | 'Foxhole 101st Easy Company - Battle of the Bulge'｜davloots｜https://sketchfab.com/3d-models/foxhole-101st-easy-company-battle-of-the-bulge-1633d24339f843c08976706173837e1b（7.4M 三角/3.7M 顶点，RealityCapture 98 张照片，Bois Jacques 即 Foy 附近 Easy Company 阵地复原点，可下载） | itch '3D trench warfare low poly'（模块化战壕，"fully movable"）｜nuclearwinter94｜https://nuclearwinter94.itch.io/tre｜许可未写明 | A 档：CC BY 4.0（需署名）；B 档：未写明 | 准确（Foy 直接相关）；**需减面到约 5–20k + 烘焙**；一个散兵坑可作静态装饰，**不能当"可挖掘模块"**（需拆分） | [F] | [ ] 已下载 |
| V-PRP-009 | 电线杆、电线、围栏 | M2 | 近似：Poly Haven `modular_electricity_poles`（约 200,610 三角，8K，现代城市风）、`modular_chainlink_fence`、`large_iron_gate`（asset 页格式 https://polyhaven.com/a/<asset_id>） | — | CC0 | 近似：欧式乡村木杆/木栅栏/石墙围栏无 | [F] | [ ] 已下载 |
| V-PRP-010 | 残骸：被击毁坦克/卡车/滑翔机 | M2 | 无 A 档候选 | Sketchfab 'destroyed M4A1E9 sherman'｜作者调研未给｜https://sketchfab.com/3d-models/destroyed-m4a1e9-sherman-931f910486234ea683ce9fde5f9d008e｜许可未见，未打开；亦可用 V-VEH-US-003 + 烧毁材质（Poly Haven rust/burned，属 D-MOD 材质参数调整） | B 档：许可未见 | 仅 B 档 / 近似 | [S] | [ ] 已下载 |

### 8.8 特效（V-FX-001..006）

**覆盖率 5/6**（准确 0，全部为近似/仅可占位；V-FX-006 见 §9.1）。免费 2D 序列帧/贴花很多，但**风格化/像素风居多，写实度不足**。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-FX-001 | 枪口焰贴图 | M1 | ① 'muzzle flash particle sheet'｜作者调研未给｜https://opengameart.org/content/muzzle-flash-particle-sheet（小型单张）<br>② Reactorcore 像素风枪口焰｜https://opengameart.org/content/gun-muzzle-flash-effects-fire-and-ion-and-melee（页面写 CC0，建议回原页确认） | — | CC0（页面标注，②建议复核） | 仅可占位：像素风/小型单张，风格不匹配写实 | [S] | [ ] 已下载 |
| V-FX-002 | 烟雾 flipbook | M1 | ① 'Free 2D VFX Pack'｜bigdamnhero6｜https://bigdamnhero6.itch.io/free-2d-vfx-pack（含 muzzle flash/explosion/poison smoke）<br>② TornadoGames CC0 魔法特效包（含烟迹；调研未给链接） | — | ①评论里作者答"CC0，在文件里"，需下载后核对 license 文件；② CC0 | 仅可占位：风格化，不适合写实 | [S] | [ ] 已下载 |
| V-FX-003 | 爆炸/火球/尘土 flipbook | M1 | ① 'Explosion Spriteanim Sheet Minipack'（10 张，各 16 帧 4×4）｜Reactorcore｜https://reactorcore.itch.io/explosion-spriteanim-sheet-minipack<br>② '2D Explosion Animations 2'｜https://opengameart.org/content/2d-explosion-animations-2-frame-by-frame<br>③ 'more explosions'｜https://opengameart.org/content/more-explosions（作者要求署名可选） | — | CC0（①评论称 CC0；②③页面标注） | 近似：偏风格化，写实度不足 | [S] | [ ] 已下载 |
| V-FX-004 | 弹孔贴花 | M1 | 'bullet decal'（512×512 PNG，下载约 9,000 次）｜https://opengameart.org/content/bullet-decal-bullethole0png（页面 403，内容未能读取）；ambientCG 贴花类前 100 项无弹孔 | Fab 免费 "5 PBR decals（含弹孔与污渍）"｜https://www.fab.com/listings/823bc036-df2a-4f6f-a644-bab8eac213b4｜许可/价格未读 | A 档：CC0（搜索摘要）；B 档：未读 | 近似：2D 低分辨率 | [S] | [ ] 已下载 |
| V-FX-005 | 血迹/飞溅贴花 | M2 | ① 'Liquid Splatter Pixel Texture'（标 blood）｜https://opengameart.org/content/liquid-splatter-pixel-texture<br>② '100 grunge brushstrokes and splatters set'（标 blood）｜https://opengameart.org/content/100-grunge-brushstrokes-and-splatters-set<br>③ OGA 'Splatter Pack'（CC-BY 3.0，要求署名；调研未给链接） | — | ①② CC0；③ CC-BY 3.0（需署名） | 近似/仅可占位：像素风或笔刷类，写实度不足（GAP-V15） | [S] | [ ] 已下载 |

### 8.9 天空（V-SKY-001..003）

**覆盖率 3/3**（准确 2、近似 1）。HDRI 覆盖最好；体积云仍需程序化。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-SKY-001 | HDRI 天空（晴、多云、阴、黄昏、雾晨） | M0 | Poly Haven 997 张 HDRI：`kloofendal_overcast`、`kloofendal_misty_morning`、`kloofendal_48d_partly_cloudy`（及 `_puresky`）、`farmland_overcast`、`misty_farm_road`、`misty_dawn`、`belfast_farmhouse`、`belfast_open_field`、`farm_field`、`autumn_field`、`overcast_soil`、`rogland_overcast`、`qwantani_*`、`evening_meadow`、`sunset_meadow_path` 等；ambientCG 另有 439 张 HDRI | — | CC0 | 准确；皆南非/欧洲摄影，非诺曼底 | [F] | [ ] 已下载 |
| V-SKY-002 | 冬季/雪天 HDRI | M4 | Poly Haven `snow_field`、`snow_field_2`、`snow_field_puresky`、`snowy_field`、`snowy_forest`、`snowy_forest_path_01/02`、`snowy_hillside`、`snowy_hillside_02`、`snowy_park_01`、`snowy_cemetery`、`passendorf_snow`、`horn-koppe_snow` | — | CC0 | 准确 | [F] | [ ] 已下载 |
| V-SKY-003 | 云层/体积云贴图 | M3 | 近似：Poly Haven `cloud_layers`、`furry_clouds`、`wasteland_clouds`（HDRI，非体积云） | — | CC0 | 近似：体积云仍需程序化（不属本清单） | [F] | [ ] 已下载 |

### 8.10 字体（V-FNT-001..004）

**覆盖率 4/4**（准确 4，**全部凭记忆 [M]，未核实 OFL 文本**，须到字体仓库核实后登记）。无差距。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| V-FNT-001 | 标题字体（次要标题） | M0 | Black Ops One｜Sorkin Type｜https://www.fontsquirrel.com/fonts/black-ops-one | — | SIL OFL（凭记忆；保留字体名） | 准确；风格接近商业射击游戏常见字体，选用时避免与参考作品相同的视觉识别 | [M] | [ ] 已下载 |
| V-FNT-002 | 模板风格标题 | M0 | Saira Stencil One｜Omnibus-Type｜https://fonts.google.com/specimen/Saira+Stencil+One | — | SIL OFL（凭记忆；license 子页 404 未能访问） | 准确 | [M] | [ ] 已下载 |
| V-FNT-003 | 模板风格标题 | M0 | Allerta Stencil｜Matt McInerney｜https://fontsource.org/fonts/allerta-stencil/about（沿用 ASSET_SHOPPING_LIST §4.1.6） | — | SIL OFL（RFN "Allerta"；沿用清单，调研片段 [M]） | 准确 | [M] | [ ] 已下载 |
| V-FNT-004 | 正文/等宽 + 简体中文方案 | M0 | Inter / Roboto Condensed / IBM Plex Sans Condensed；简体中文 Noto Sans SC / Source Han Sans（需子集化）（调研未给逐个链接） | — | SIL OFL（凭记忆，未核实） | 准确 | [M] | [ ] 已下载 |

### 8.11 音频：枪械（A-WPN-001..017）

**覆盖率 14/17**（准确 6、部分 8；A-WPN-010、-012、-013 无合格候选见 §9.2）。**每个 GDC 库在免费包里只有约 2–4 个文件**（"代表性样本"，不是整库）：同一型号能用的开火文件通常 1–3 个；只有 BAR、M1919A4、Thompson、M3 同时有"近 + 300 m 真实远场"。

> 引用格式 `GDC{年}-p{分卷}` = Sonniss GameAudioGDC 第几年第几分卷（来源页 https://sonniss.com/gameaudiogdc ；文件名取自第三方索引，**未下载、未聆听**）；Freesound 条目为 `Freesound {编号}`，URL 为调研所给。**所有音频均未聆听，导入前必须试听。**

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| A-WPN-001 | M1 Garand：开火近/中/远 + 弹夹 ping + 拉栓/换夹 | M0 | ① Freesound 725398 'An M1 Garand clip being ejected w/ ping'｜serøutōnin--deprivəd｜https://freesound.org/people/ser%C3%B8ut%C5%8Dnin--depriv%C9%99d/sounds/725398/（1.185 s，44.1 kHz/16-bit/mono，SM-57 自录）<br>② Freesound 725399 同作者（空仓换夹全序列，24 s）｜https://freesound.org/people/ser%C3%B8ut%C5%8Dnin--depriv%C9%99d/sounds/725399/<br>③ Freesound 385785 'Rifle Shot (M1 Garand)'｜Rijam｜https://freesound.org/people/Rijam/sounds/385785/（2.76 s，44.1 kHz/单声道/128 kbps MP3，无来源说明）<br>④ Freesound 386842 'M1 Garand Rifle'｜nioczkus｜https://freesound.org/people/nioczkus/sounds/386842/（70 s，无来源说明）<br>⑤ Freesound 505206 保险开关｜shelbyshark｜https://freesound.org/people/shelbyshark/sounds/505206/ | — | CC0（Freesound 页面标注） | 近似/部分：ping 与换夹有可信真实 CC0 实录（①②）；**开火声只有来源不明的③④**（MP3 128 kbps 不足以作近层），无近/中/远分层；GDC 无 Garand。M0 需负责人接受来源不明条目或以栓动枪声暂代 | [F] | [ ] 已下载 |
| A-WPN-002 | M1 Carbine | M1 | ① Freesound 46655 'carbine single.wav'｜cognito perceptu｜https://freesound.org/people/cognito%20perceptu/sounds/46655/（单发 .30，林地射击场）<br>② Freesound 46656 'carbine.wav'｜同作者｜https://freesound.org/people/cognito%20perceptu/sounds/46656/（多发连射）<br>③ Freesound 46654 'carbine debris.wav'｜同作者｜https://freesound.org/people/cognito%20perceptu/sounds/46654/<br>④ Freesound 508246 'M1 30 Carbine Being Loaded'｜shelbyshark｜https://freesound.org/people/shelbyshark/sounds/508246/ | — | CC0（Freesound 页面标注） | 部分：真实 M1 卡宾枪实录，天然带森林尾音；仅 3 条开火素材，采样率/位深未取，无分层 | [F] | [ ] 已下载 |
| A-WPN-003 | Thompson（M1928A1/M1A1） | M1 | ① GDC2016-p4 Pole Position 'M1928A1 Thompson Submachine Gun .45cal'：`0.3m_to_side_ORTF_Burst_of_four`、`300m_in_front_Burst_of_four`<br>② GDC2020-p8 Pole Position 'Thompson 1928A1 automatic submachine gun'：`Firing, t2, Burst, 1m, Right, Above, MKH8060`、`Handling, Dry Fire, MKH416`（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确：真实 1928A1；含近（0.3 m）+ 真实远（300 m）两层；仅 4 个文件，无单发/全自动长序列 | [F 索引] | [ ] 已下载 |
| A-WPN-004 | M3 Grease Gun | M1 | ① GDC2016-p4 Pole Position 'M3 Submachine Grease Gun .45cal'：`0.3m_to_side_Triple_shots`、`300m_in_front_Burst_of_thirteen`<br>② GDC2020-p13 Super Thump 'M3 Grease Gun Recorded Source'：`…Auto_Shot_X7_Attached`、`…Single_Shot_X10_Comp_Full`（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确：免费包里最完整的单枪覆盖（近 + 远 300 m + 单发 ×10 + 连发 ×7） | [F 索引] | [ ] 已下载 |
| A-WPN-005 | BAR（M1918） | M1 | GDC2016-p4 Pole Position 'M1918 Browning Automatic Rifle .30cal'：`0.1m_to_right_Double_shots_x_1`、`300m_in_front_Double_shots_x_1`（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：近 + 远 300 m 两层，**只有二连发，无长点射/全自动**（GAP-A02 仍在） | [F 索引] | [ ] 已下载 |
| A-WPN-006 | M1919 Browning（点射/全自动） | M1 | ① GDC2016-p4 Pole Position 'M1919A4 Browning Machine Gun .30cal'：`200m_left_behind_blanks_Triple_shots`、`5m_behind_ORTF_blanks_Triple_shots`<br>② 同库 on turret：`on_turret_1m_left_blanks_Triple_shots_x_2`、`on_turret_300m_in_front_blanks_Triple_shots_x_2`<br>③ GDC2020-p13 Super Thump WoWW2 Designed：`STDSGN_WoWW2_Wep_MG_M1919_Machinegun_Auto-Burst_Shot_X6`（Designed 版，来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确：真实型号，1/5/200/300 m 多视角（空包弹录制）；仅三连发；全自动长序列仅③（经混音处理，非原始录音） | [F 索引] | [ ] 已下载 |
| A-WPN-007 | M1903 Springfield | M1 | GDC2020-p8 Pole Position 'Springfield 1903A3 bolt-action rifle'：`Firing, t2, 1m, Right, Above, MKH8060`、`Handling, Cycling Bolt, MKH416`（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：1903A3（非 A4 狙击型，声学相同）；仅 1 个近场开火 + 拉栓 | [F 索引] | [ ] 已下载 |
| A-WPN-008 | M1911 | M1 | ① GDC2020-p7 Pole Position 'Colt M1911A1 semi-automatic pistol'：`Firing, t2, 200m, Front, Target, M10`、`Handling, Dry Fire`<br>② GDC2016-p1 Audiobeast 'London Warehouse Firearms Library'：`Colt_M1911_.45_ACP_Pistol_10m_MKH8040_Pair_001_Single_Shot_x6`（仓库混响）<br>③ Freesound 112073 'kimber 1911 shooting.mp3'｜cheesepuff（Kimber 现代型，非军版；调研未给 URL）<br>④ Freesound 271453/271458 Springfield Armory GI 1911 装填/拉栓｜bdunis4（调研未给 URL）（GDC 来源页 https://sonniss.com/gameaudiogdc） | — | ①② Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；③④ CC0 | 准确：①为真实 GI 型 M1911A1，远场 200 m；③④补机械动作 | [F 索引]/[F 列表] | [ ] 已下载 |
| A-WPN-009 | Kar98k：开火近/中/远 + 拉栓/装弹夹 | M0 | ① GDC2020-p7 Pole Position 'Mauser Karabiner 98 kurz K98k'：`Firing, t2, MKH416`、`Handling, Various, t2, 1m, Right, MKH8060`<br>② Freesound 802675 'Mauser 98k Shooting'｜Walking.With.Microphones｜https://freesound.org/people/Walking.With.Microphones/sounds/802675/（2.085 s，48 kHz/24-bit/立体声 ORTF，历史再现活动空包弹）<br>③ Freesound 802673 'Mauser 98k - bolt-action mechanism'｜同作者｜https://freesound.org/people/Walking.With.Microphones/sounds/802673/<br>④ Freesound 508747 'Bolt Action Rifle Reload'｜AugustSandberg｜https://freesound.org/people/AugustSandberg/sounds/508747/<br>⑤ Freesound 505198 'CzechVZ24 German K98 Load Stripper Clip'｜shelbyshark｜（捷克 VZ24，K98 同系；调研未给 URL）（GDC 来源页 https://sonniss.com/gameaudiogdc） | — | ① Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；②–⑤ CC0 | 准确：M0 可零成本启动；无近/中/远层，需后期距离处理 | [F] | [ ] 已下载 |
| A-WPN-011 | MP40 | M1 | ① GDC2020-p7 Pole Position 'MP 40 submachine gun'：`Firing, t1, 250m, Holophone`（真实 250 m 远场）、`Handling, Cocking, MKH416`<br>② Freesound 187534 'mp40 Near and far.wav'｜rakurka｜https://freesound.org/people/rakurka/sounds/187534/（8 s，来源无说明，中低可信）（GDC 来源页 https://sonniss.com/gameaudiogdc） | — | ① Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；② CC0 | 部分：**只有 250 m 远场 + 上膛，缺近场开火**；需配合其它近场 SMG 声并 EQ（"替代"灰区，需负责人确认） | [F] | [ ] 已下载 |
| A-WPN-014 | MG42：单发/全自动、近/中/远、长/短尾音 | M1 | GDC2020-p7 Pole Position 'MG 42 machine gun'：`MG 42, Firing, t1, Burst, Long, Mountain Top, D100`、`Handling, Top Cover Up, 1m, Right, MKH8060`（来源页 https://sonniss.com/gameaudiogdc） | Freesound 338242 'MG 42 (Solo) WW2.wav'｜Lubini｜（调研未给完整 URL）｜CC-BY；自述"made from real MG 42 + few additional layers"，来源未说明，非纯实录 | A 档：Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；B 档：CC-BY | 部分：有长点射但只有山顶远场一条，近距尾音与单发需另配（与付费清单推荐的 Pole Position MG42 同厂牌同条目的免费样本） | [F] | [ ] 已下载 |
| A-WPN-015 | P38 / Luger P08 | M1 | ① GDC2020-p7 Pole Position 'Walther P38 semi automatic pistol'：`Firing, t1, 2m, Behind, LS5`、`Handling, Removing Magazine`<br>② GDC2020-p13 Super Thump WoWW2 Designed：`Pistol_Luger_Shot_X5`（设计版）（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确：两种型号都有 | [F 索引] | [ ] 已下载 |
| A-WPN-016 | 尾音/混响分层（室外开阔、城镇、森林） | M1 | ① GDC2018-p5 Pole Position 'The Outdoor Gun Acoustics Library'：`AK47_big_open_area_2m_above_behind_gun`、`AK5_valley_field_forest_50m_behind_gun_off_axis_behind_buildings`、`MP5_forest_road_10m_behind_gun_pointing_at_forest_off_axis`<br>② GDC2017-p6 Pole Position 'The Warfare Library'：`mg_firing_close_projectile_tail_large_field_Telinga`、`cannon_firing_forest_distant_MKH8060`<br>③ GDC2018-p5 / GDC2024-p6 Indoor Gun Acoustics 1/2（室内，现代枪）（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：环境声学样本，**枪型不是二战枪**；用作室外/森林/山谷反射参考或叠到二战枪近层（叠加他枪尾音属混音灰区，需负责人确认） | [F 索引] | [ ] 已下载 |
| A-WPN-017 | 机械动作（拉栓/换弹/上膛/干击/保险） | M1 | ① GDC2020-p7/p8 Pole Position 各枪 `Handling` 文件（MG42 掀盖、MP40 上膛、K98k 多种操作、M1903A3 拉栓、Thompson 干击、M1911A1 干击、P38 卸弹匣）<br>② GDC2016-p4 Pole Position 'Various Gun Foley & Handling'（4 文件，均为现代枪）<br>③ Freesound：shelbyshark 505206/508243/508246；AnthonyChan0 Springfield Armory 1911 handling 487500（调研未给 URL）（GDC 来源页 https://sonniss.com/gameaudiogdc） | — | ①② Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；③ CC0 | 部分：二战型号机械声来自①；②为现代枪，仅作通用替代 | [F] | [ ] 已下载 |

### 8.12 音频：爆炸与重武器（B-EXP / B-HVY / B-ART / B-AIR）

**覆盖率 8/9**（准确 1、部分 7；B-AIR-001 航空炸弹见 §9.2）。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| B-EXP-001 | 手雷爆炸（近/中/远，室外/室内） | M1 | ① GDC2017-p4 Gamemaster Audio 'Explosion Sound Pack'：`explosion_far_distant_02`、`explosion_large_08`、`explosion_large_no_tail_03`、`explosion_med_long_tail_01`<br>② GDC2020-p12 Stefano Cremona 'Explosions'：`BigExplosion02`、`DeepExplosion02`、`FarExplosion03`<br>③ GDC2021-2023-p2 Bluezone 'Detonation – Explosion'：`explosion_mortar_002_01`、`explosion_urban_004_02`<br>④ Freesound 693421 'CTS 7290.wav'｜areniporgen｜https://freesound.org/people/areniporgen/sounds/693421/（真实震爆弹，半室外）<br>⑤ Freesound 516914 'Explosion, Medium Blast'｜Kinoton｜https://freesound.org/people/Kinoton/sounds/516914/（中距离长混响）（GDC 来源页 https://sonniss.com/gameaudiogdc） | — | ①②③ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；④⑤ CC0 | 近似：**都不是二战手榴弹实录**；①②③为设计库（多距离），④为现代震爆弹（高频更尖）；通用爆炸近/中/远可行，史实匹配度低 | [F] | [ ] 已下载 |
| B-EXP-002 | 地雷爆炸（人员/车辆/反坦克） | M3 | ① GDC2024-p6 Pole Position 'The Warfare 2 Library'：`Tank Mine - EXPLOSION - Mounted on Metal Beam - DISTANT - AMBEO BLD`（真实军演反坦克雷）<br>② GDC2017-p1 Bluezone 'War Zone (Designed Explosions)'：`beep-mine-explosion-002/007`（设计）（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：只有 1 个真实反坦克雷远录；近场/人员雷（跳雷）无 | [F 索引] | [ ] 已下载 |
| B-EXP-003 | 炸药包（Satchel/C-4） | M3 | ① GDC2020-p2 Bluezone 'Tank – Explosion Sound Effects'：`explosion_outdoors_large_005`、`debris_falling_rock_dust_002`（设计）<br>② 同 B-EXP-001 的候选（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 近似：无 satchel/C-4 专用实录；通用大爆炸 + 碎屑 | [F 索引] | [ ] 已下载 |
| B-HVY-001 | Bazooka / Panzerschreck / Panzerfaust 发射与命中 | M3 | ① GDC2024-p6 Warfare 2：`RPG - FIRING AND IMPACT - Through Car Roof Hit Metal - XY - MKH8040`（真实军演 RPG，现代）<br>② Freesound 249298 'rpg launcher three shots.wav'｜klangfabrik｜https://freesound.org/people/klangfabrik/sounds/249298/（74 s，三次中距离户外发射，来源无说明）（GDC 来源页 https://sonniss.com/gameaudiogdc） | Freesound craigsmith 675581 'S19-34 Bazooka shot'｜craigsmith｜https://freesound.org/people/craigsmith/sounds/675581/｜CC0（上传者标注；48 kHz/24-bit/mono）；craigsmith 438531 'G33-25-Bazooka'；上传者称素材为数字化的好莱坞 1930s–60s 效果母带，**母带权属无从判断** | A 档：① Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；② CC0；B 档：craigsmith CC0（权属存疑） | 近似：**没有可确认的二战 Bazooka/Panzerfaust/Panzerschreck 实录**；①②为现代 RPG（发射声更尖，无 Bazooka 管内气流声） | [F] | [ ] 已下载 |
| B-ART-001 | 105 mm 榴弹炮发射（近/中/远，0–150 m） | M3 | GDC2019-p1 Airborne Sound 'Battlefield Howitzers'：`Howitzer,M101,C1,105 mm,Distant,Right Side,Shot,Pound,Thick`、`…C3,105 mm,Distant,…Shot,Explode,Crack,Sweetener`、`…C3,105 mm,Medium Distant,…Shot,Firm,Heavy,EQ Version,Soft Attack`（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确：文件名明写 M101 105 mm（美军二战主力榴弹炮），有远/中远层；"Sweetener/EQ Version"表明含设计处理，需听确认；**近场（0–50 m）缺** | [F 索引] | [ ] 已下载 |
| B-ART-002 | 迫击炮发射/落弹 | M3 | ① Freesound 184382 'Mortar Shots.flac'｜qubodup｜https://freesound.org/people/qubodup/sounds/184382/（28 s，抽取自美国政府视频）<br>② Freesound 240567 'mortar line.wav'｜greenbastardd｜https://freesound.org/people/greenbastardd/sounds/240567/（五门迫击炮演习）<br>③ Freesound 529239 'Mortar'｜Mozfoo｜https://freesound.org/people/Mozfoo/sounds/529239/（真实训练 + 额外混响）<br>④ Freesound 854481 'Mortar Shell Loading Slide 3'｜qubodup｜https://freesound.org/people/qubodup/sounds/854481/（M120 装弹）<br>⑤ GDC2021-2023-p2 Bluezone Detonation：`explosion_mortar_002_01`（设计）（来源页 https://sonniss.com/gameaudiogdc） | — | ①–④ CC0；⑤ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：发射声有真实实录（现代迫击炮）；落弹前"来袭哨声"无可信来源（Zagge28 'Incoming mortar' 条目来源不明，未列入） | [F] | [ ] 已下载 |
| B-ART-003 | 远处炮击床（背景轰鸣） | M3 | ① Freesound 239137 'Artillery Gunfire'｜qubodup｜https://freesound.org/people/qubodup/sounds/239137/（54 s，美国政府视频）<br>② Freesound 182822 'Artillery Shots.flac'｜qubodup｜https://freesound.org/people/qubodup/sounds/182822/（16 s）<br>③ GDC2017-p6 Warfare Library：`cannon_firing_forest_distant_MKH8060`（真实远炮）<br>④ Freesound 702245 'GUNArtl_69thFieldArtilleryGun01_InMotionAudio_FREESampleSunday'｜InMotionAudio（18 s，商业厂牌的免费样本；调研未给 URL）（GDC 来源页 https://sonniss.com/gameaudiogdc） | Freesound 486039 'R30-11-Large Guns Heard from Inside Building'｜craigsmith｜CC0（权属存疑，好莱坞母带；调研未给 URL） | A 档：①②④ CC0；③ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；B 档：CC0（权属存疑） | 部分：够做"远处炮声"层，循环床需自己拼接；①②音质受源视频压缩音轨限制 | [F] | [ ] 已下载 |
| B-AIR-002 | 飞机俯冲、扫射（机炮） | M3 | ① Freesound 854186 'Autocannon Three Shot Burst'｜qubodup｜https://freesound.org/people/qubodup/sounds/854186/（4 s，美国海军陆战队实弹视频）<br>② GDC2018-p5 P-51D（`ext_startup_ramps_shutdown_distant_behind`）、GDC2019-p5 Spitfire IX（`ext_take_off_by_right_position`）、GDC2018-p5 Spitfire XIV（`ext_landing_passby`）、GDC2017-p6 Bearcat（`flybys_cockpit`）<br>③ GDC2019-p3 B-17G（`Ext Take Off By`）、GDC2017-p6 B-25（`ext_startup_idle`）（来源页 https://sonniss.com/gameaudiogdc） | — | ① CC0；②③ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：扫射用 M1919/MG42 近层 + 多普勒（后期）；飞机掠过有真实二战机型；无 Stuka 俯冲警笛真实录音 | [F] | [ ] 已下载 |

### 8.13 音频：子弹（C-BUL-001..005）

**覆盖率 5/5**（准确 3、部分 2）。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| C-BUL-001 | 子弹掠过 whizz / 超音速 crack | M0 | ① GDC2017-p6 Pole Position 'The Warfare Library'：`warfare_t3_mg_whizzes_ricochets_bullet_cracks_M10`（99.7 MB，真实军演）<br>② GDC2020-p5 PMSFX 'Bullet Bys & Impacts'：`PM_BBI_Bullet_Passby_Whizzby_Airy_5`（66.9 MB）<br>③ GDC2017-p4 Gamemaster 'Bullet Impact Sounds'：`bullet_flyby_fast_05`；GDC2018-p3 Gamemaster 'Guns, Bullets and Explosions'：`bullet_flyby_fast_06`（设计库）（来源页 https://sonniss.com/gameaudiogdc） | ① Freesound 423301 'VisceralBulletImpacts.wav'｜u1769092｜CC0｜无来源说明，疑似视频/影视采样（调研未给 URL）<br>② Freesound 855248 'Real Bullet Flyby Sound'｜qubodup｜CC0｜抽取自 Natochannel b-roll（非美国政府视频），仅 0.079 s，作者页许可自相矛盾（调研未给 URL） | A 档：Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；B 档：CC0（来源存疑） | 准确：①一个 100 MB 长录音可剪出大量 whizz/crack/ricochet 变体；左右立体声需听确认 | [F 索引] | [ ] 已下载 |
| C-BUL-002 | 各材质命中（泥、混凝土、金属、木、沙袋、水） | M1 | ① GDC2020-p5 PMSFX Bullet Bys & Impacts：`Bullet_Impact_Dirt_3`、`Bullet_Impact_Glass_1`、`Bullet_Impact_Hit_Body_Flesh_25`<br>② GDC2017-p4 Gamemaster Bullet Impact Sounds：`concrete_brick_01`、`metal_heavy_08`、`body_thump_02`<br>③ GDC2018-p4 Olivier Girardot 'Guns & Explosions'：`Bullet Impact 22`（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确（泥/混凝土/金属/玻璃）：PMSFX/Gamemaster 为设计/Foley 型库，是否实弹录制未知；木、沙袋、水需另找 | [F 索引] | [ ] 已下载 |
| C-BUL-003 | 跳弹（ricochet） | M1 | ① Warfare Library 的 `…ricochets…`（见 C-BUL-001，来源页 https://sonniss.com/gameaudiogdc）<br>② Freesound 148827 '.22 ricochet.mp3'｜cedarstudios｜https://freesound.org/people/cedarstudios/sounds/148827/（53 s，户外 30 m 外）<br>③ Freesound 148840 'ricochet.mp3'｜cedarstudios｜https://freesound.org/people/cedarstudios/sounds/148840/（106 s）<br>④ Freesound 523403 '22 caliber with ricochet.wav'｜C-V｜https://freesound.org/people/C-V/sounds/523403/（27 s，沙漠，Rode NTG-3 + Zoom H6） | — | ① Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；②–④ CC0 | 准确：①为军用机枪弹；②③④为 .22 小口径（音调偏高，需 EQ/变调） | [F] | [ ] 已下载 |
| C-BUL-004 | 击中人体（友/敌，身体/头部） | M1 | ① GDC2020-p5 PMSFX `Bullet_Impact_Hit_Body_Flesh_25`<br>② GDC2017-p4 Gamemaster `bullet_impact_body_thump_02`（来源页 https://sonniss.com/gameaudiogdc） | Freesound 423301 'VisceralBulletImpacts.wav'｜u1769092｜CC0｜来源无说明，疑似视频/影视采样（调研未给 URL） | A 档：Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；B 档：CC0（来源存疑） | 近似：专业实录几乎不存在（GAP-A04）；PMSFX/Gamemaster 属设计库 Foley | [F 索引] | [ ] 已下载 |
| C-BUL-005 | 压制感（近距多弹掠过 + 低频冲击 + 耳鸣） | M1 | 掠过用 C-BUL-001 的候选（来源页 https://sonniss.com/gameaudiogdc）；低频冲击与耳鸣：无合格免费候选（本次未专门搜索耳鸣高频声）；AudioWorklet 低通/高通属代码，不需素材 | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：仅掠过部分有素材 | [F 索引] | [ ] 已下载 |

### 8.14 音频：载具与飞机（D-TNK / D-VEH / D-AIR）

**覆盖率 6/8**（准确 3、部分 3；D-TNK-004、-005 见 §9.2）。载具声音出乎意料地好（真实 Sherman、Panzer IV、Panther、Opel Blitz 样本），但**炮声缺**。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| D-TNK-001 | Sherman 发动机怠速/加速多转速循环 | M1 | ① GDC2018-p5 Pole Position 'Sherman M4A3 Medium Tank'（Ford GAA V8）：`sherman_m4a3_t3_onbrd_start_attempt_medium_drive_decelerate_slow_stop_off_engine_DPA4061`（81.5 MB）、`…t4_onbrd_medium_drive_steady_slow_short_tracks_right_RE50`（67.2 MB）、`…t9_onbrd_fast_drive_stop_off_interior_DPA4021`（70.6 MB）<br>② GDC2018-p5 'Sherman M4A1 Medium Tank'（Continental 星型）：`t3_onboard_medium_driving_turning_stopping_gearing_interior_left/right_DPA4021`、`t11_foley_siren_long_CMC6`（来源页 https://sonniss.com/gameaudiogdc） | Freesound craigsmith 438121 'G15-25-M-4 Tank Slow Drive By'｜craigsmith｜CC0（权属存疑，好莱坞母带；页面未确认 M4 是否 Sherman；调研未给 URL） | A 档：Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；B 档：CC0（权属存疑） | 准确（真实 Sherman，车载/车内）；**缺车外近/远麦的怠速/加速循环**；不要混用 M4A1 与 M4A3 发动机 | [F 索引] | [ ] 已下载 |
| D-TNK-002 | 履带声（碾压/转向） | M1 | ① GDC2018-p5 M4A3 `t4_…tracks_right_RE50`（履带专用麦）<br>② GDC2020-p7 M10 `t12_onbrd_idle_various_tracks_1_left_E935`、`t5_…tracks_1_right_E935`<br>③ GDC2018-p5 Stuart `t10_onbrd_fast_drive_short_stop_tracks_RE50`<br>④ Freesound 329800 't26_moving_01.wav'｜greatmganga｜（苏制 T-26 森林行进；调研未给完整 URL）<br>⑤ Freesound 849062 'tracks moving'｜christerljung（调研未给 URL）（GDC 来源页 https://sonniss.com/gameaudiogdc） | — | ①–③ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；④⑤ CC0 | 准确：M4A3/M10 履带麦为真实 WWII 载具（M10 为 Sherman 底盘车体） | [F 索引]/[F 列表] | [ ] 已下载 |
| D-TNK-003 | 炮塔转动（电/液压） | M1 | ① GDC2018-p4 Chaffee M24：`t7_var_sfx_ext_hydraulic_traverse_XY_RSM191`（车外液压转塔）<br>② GDC2020-p8 Panther Ausf. A：`t10, Var SFX, Turret, Rotate, Off, Interior, Driver`<br>③ GDC2018-p5 M18 Hellcat `t9_foley_cannon_elevating_manually_slow_from_inside_CMC6`；GDC2016-p4 T-34-85 `foley_cannon_height_adjustment`<br>④ Freesound 418881 'Tank Turret Rotate'｜KieranKeegan｜https://freesound.org/people/KieranKeegan/sounds/418881/（37 s，Zoom H2n，车型不明）（GDC 来源页 https://sonniss.com/gameaudiogdc） | — | ①–③ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；④ CC0 | 部分：Sherman 自己的转塔声没有；Chaffee/Hellcat/Panther 的真实转塔/俯仰声可作美/德替代 | [F] | [ ] 已下载 |
| D-TNK-006 | Panzer IV / Panther 全套（发动机、履带） | M3 | ① GDC2019-p4 Pole Position 'Panzer IV Ausf. G'：`t10_ext_start_idle_steady_blips_main`（车外）、`t1_ext_start_slow_away_up_stop_off_main`、`t6_onbrd_start_medium_drive_various_stop_off_track…`<br>② GDC2020-p8 Panther Ausf. A：`t1, Onbrd, Start, Slow, Drive, Stop, Off, Engine`、`t10 Turret Rotate`、`t5, Ext, Start, Fast, Stop, Off, CSS5`<br>③ GDC2019-p4 Panzer III Ausf. M；④ GDC2019-p5 Tiger II；⑤ GDC2020-p7 Jagdpanzer 38(t) Hetzer（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确：德军坦克有真实实录样本且带车外视角（Panzer IV G、Panther A 正是付费清单推荐的两款）；炮声缺；Tiger I 无（只有 Tiger II） | [F 索引] | [ ] 已下载 |
| D-VEH-001 | 卡车/半履带/吉普 | M3 | ① GDC2019-p4 Pole Position 'Opel Blitz 3600 1939'：`t14_ext_fast_bys_horn_position_b_MKH416`、`t15_onbrd_start_idle_steady_blips_off_interior_DPA4021`<br>② GDC2018-p5 'M16 Halftrack'：`t4_onbrd_slow_drive_accelerate_to_fast_stop_reverse_brief_tracks_left_RE50`<br>③ GDC2020-p7 Mercedes L3750 1945<br>④ GDC2017-p6 M1 Corbitt Prime Mover Cargo Truck（`t11_ext_passby_DPA4011` 等）<br>⑤ GDC2019-p5 Volkswagen Kübelwagen 1943（`t12_Ext_Bys_Gearshifts_H6`）（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：Opel Blitz（德军标志卡车）、M16 半履带（M3 系列近似）、Kübelwagen 真实；**GMC CCKW / Willys 吉普没有**（GDC 中的 Jeep 都是现代车） | [F 索引] | [ ] 已下载 |
| D-AIR-001 | P-47 / Ju 87 Stuka / C-47 | M3 | ① GDC2018-p5 P-51D Mustang、Spitfire Mk XIV；GDC2019-p5 Spitfire Mk IX 1943；GDC2017-p6 F8F Bearcat；GDC2020-p9 Corsair F4U-1A；GDC2021-23-p6 Zero A6M5<br>② GDC2017-p6 B-25 Mitchell、GDC2019-p3 B-17G 1945（来源页 https://sonniss.com/gameaudiogdc）<br>③ Freesound 818696 'Fly over' / 818695 'Prop areoplanes fly over (Three)'｜studioste（4 s / 7 s，航展 WW2 风格飞机；调研未给 URL） | ① Freesound 437771 'G04-07 WWII Stuka engine drone'｜craigsmith｜CC0（权属存疑，好莱坞母带；调研未给 URL）<br>② Freesound 758467 'War Airplane Stuka Siren Sound Effect'｜Mastersoundboy2005｜CC0｜来源无说明，该上传者同时上传游戏创作向 'Stuka Creature Screaming'（调研未给 URL） | A 档：①② Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；③ CC0；B 档：CC0（来源/权属存疑） | 部分：**P-47、C-47、Ju 87 无真实实录**；P-51D 作替代（P-47 星型发动机音色不同）；Stuka 俯冲警笛无可信免费实录 | [F] | [ ] 已下载 |

### 8.15 音频：Foley（E-FTS / E-GER / E-ACT）

**覆盖率 3/3**（准确 1、部分 2）。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| E-FTS-001 | 军靴脚步（泥、草、碎石、木地板、雪、沙、浅水） | M0 | ① Freesound BenDrain 'Footsteps_Mud_01…05 / Single_01/03'（488068、488069、487875、487874、488076、488077、488082 等，泥地）｜BenDrain｜https://freesound.org/people/BenDrain/sounds/488068/<br>② Freesound 548384 'Footsteps_Mountain_Boots_Mud_Mono'｜Nox_Sound｜https://freesound.org/people/Nox_Sound/sounds/548384/<br>③ Freesound 611278 'Footsteps on slush'｜xkeril｜https://freesound.org/people/xkeril/sounds/611278/（48 kHz/24-bit）<br>④ GDC2016-p2 Levan Nadashvili 'Soldier Footsteps'：`FS Concrete Soldier Crouch N02/N05`、`FS Metal Soldier Walk N01/N05`<br>⑤ GDC2019-p6 Studio 23 'Ultimate Footstep Collection'：`Gravel_Loafers_Loops_Walk_Normal`、`Metal_Boots_Loop_Jogging`、`Snow_Singles_Sequence`；GDC2020-p14 Wav Junction Footsteps；GDC2019-p3 PMSFX 'STEPS Dirt & Gravel'；GDC2020-p5 PMSFX 'STEPS Dry Grass & Shrubs'（来源页 https://sonniss.com/gameaudiogdc） | — | ①②③ CC0；④⑤ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确（泥地充裕，≥8 变体达标）；草、碎石、雪、石板、木地板、浅水：GDC 各 1–3 个样本，未达每表面 ≥8 变体，需分别补搜或从长序列剪切 | [F] | [ ] 已下载 |
| E-GER-001 | 装备晃动/卧倒/翻越 | M1 | ① GDC2020-p14 Wav Junction 'Zippers, Buckles & Clothing'<br>② GDC2019-p1 Borg Sound 'Bag Foley'；GDC2017-p5 Joshua Reinhardt 'Ultimate Cloth and Prop Collection'<br>③ GDC2024-p1 Eiravaein Works 'Flask'（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：无军事装具专用库；通用布料/扣具/背包 Foley 勉强替代；匍匐/卧倒/翻越无（GAP-A08） | [F 索引] | [ ] 已下载 |
| E-ACT-001 | 包扎、建造敲打、铲土 | M3 | ① Freesound 353907 'Shovel_dirt.wav'｜dr19｜https://freesound.org/people/dr19/sounds/353907/<br>② Freesound 574859 'Digging dirt shovel 05.flac'｜TRP（调研未给 URL）<br>③ Freesound 513866 'Digging v1.wav'｜wormletter（金属铲，泥+砾石；调研未给 URL） | — | CC0（Freesound 页面标注） | 部分：铲土可行；**包扎无**；建造敲打本次未专门搜 | [F] | [ ] 已下载 |

### 8.16 音频：环境（F-AMB-001..006）

**覆盖率 5/6**（准确 4、部分 1；F-AMB-005 成品床见 §9.2，可用远场文件自混）。**没有"诺曼底专用"环境**，只有欧洲乡村通用。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| F-AMB-001 | 诺曼底乡村日/夜 | M1 | ① GDC2018-p3 Fox Audio Post-Production 'Countryside – Nature & Field'：`Amb_Countryside_Day_Flying_Insects_Birds_LP`、`…Day_Rooster_Birds`、`…Evening_Birds_Cow_Rooster_Tractor_Far_LP01`、`…Night_Birds_Cow_Cricket`（LP = 可循环）<br>② GDC2020-p3 Coll Anderson 'AMBISONICS A Countryside Nature'；GDC2024-p8 Sonik Sound Library 'Spatial Countryside'<br>③ GDC2018-p1 Alexander Kopeikin 'Quad ambience of Crimea in summer'<br>④ Freesound 458113 'Countryside'｜brunoboselli｜https://freesound.org/people/brunoboselli/sounds/458113/（48 kHz/24-bit/立体声，黎明）<br>⑤ Freesound 514550 'Countryside Ambience Spring'｜Kinoton｜https://freesound.org/people/Kinoton/sounds/514550/（来源页 https://sonniss.com/gameaudiogdc） | — | ①②③ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；④⑤ CC0 | 准确：日/夜均有，含农场动物与远处拖拉机；欧洲乡村感好，但不保证是"诺曼底"（GAP-A11） | [F] | [ ] 已下载 |
| F-AMB-002 | 海滩海浪（Omaha） | M4 | ① Freesound 827530 'Ocean_coast_04_092025_0659AM'｜YevgVerh｜https://freesound.org/people/YevgVerh/sounds/827530/（132 s，轻浪沙滩）<br>② Freesound 623051 / 623052 'Ocean Waves, Rocky Beach'｜goochiano｜https://freesound.org/people/goochiano/sounds/623051/<br>③ Freesound 457149 'Binaural Beach Waves 4'｜Benbojangles｜（双耳，57 s；调研未给完整 URL） | — | CC0（Freesound 页面标注） | 准确：Omaha 滩头强浪/涌浪需试听，已有候选偏安静 | [F] | [ ] 已下载 |
| F-AMB-003 | 风、雨 | M1 | ① GDC2015-p4/p5 Soundopolis 'Natures Fury_Rain_01 / Wind_01'<br>② GDC2017-p4 Hzandbits 'Wind In Trees'、'Urban Winds II'；GDC2017-p3 Double Trouble 'Rain and Thunder'；GDC2018-p2 Discover Oregon 'Wind and Storms'；GDC2018-p2 Borg Sound 'Rain Indoor Ambiences'<br>③ Freesound 637566 'forest light wind through bare trees winter'｜kyles｜https://freesound.org/people/kyles/sounds/637566/（来源页 https://sonniss.com/gameaudiogdc） | — | ①② Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；③ CC0 | 准确：充足 | [F] | [ ] 已下载 |
| F-AMB-004 | 冬季森林（Foy） | M4 | ① Freesound 637566｜kyles｜https://freesound.org/people/kyles/sounds/637566/<br>② Freesound 673266 'Forest snow falling and birds'｜xkeril｜https://freesound.org/people/xkeril/sounds/673266/（瑞士森林）<br>③ Freesound 676801 '2023 leaves snow wind forest_MIXPRE-759'｜peterkotrha（调研未给 URL）<br>④ Freesound 846045｜Dakenber（冰冻傍晚枝条吱嘎；调研未给 URL）<br>⑤ GDC2018-p2 Discover Oregon 'Wind and Storms'（来源页 https://sonniss.com/gameaudiogdc） | — | ①–④ CC0；⑤ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确：够用 | [F] | [ ] 已下载 |
| F-AMB-006 | 建筑内部房间音 | M1 | GDC2018-p4 Gladestock Studios 'Urban Decay'：`GS amb abandoned industrial site QUAD`、`GS amb empty domed hall QUAD`、`GS amb underground carpark QUAD`、`GS amb warehouse storage QUAD`（来源页 https://sonniss.com/gameaudiogdc） | — | Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 部分：工业/厂房类；民居房间底噪本次未专门搜（Freesound 未查） | [F 索引] | [ ] 已下载 |

### 8.17 音频：人声（G-VOX-001..004）

**覆盖率 2/4**（准确 0、部分 2；G-VOX-002 德语、G-VOX-004 口令见 §9.2）。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| G-VOX-001 | 中枪/倒地/痛呼/死亡（英语男声） | M1 | ① GDC2021-23-p7 Pole Position 'The Grunts Library'：`Male 2 - Grunt - Battle Cry - Long - Ehrlund M1`、`Male 1 - Breathing - Gasping`、`Female 1 - Grunt - Getting Hit - Medium Intensity`<br>② GDC2019-p1 Articulated Sounds 'Fight Vocalizations'：`EMOTE Joshua, Man, Pain Hurt Grunt Big 03`<br>③ Freesound 166944 'Grunts of pain by military soldiers.flac'｜qubodup｜https://freesound.org/people/qubodup/sounds/166944/（19 s，抽取自美国政府视频）<br>④ Freesound 800870 'Soldier-DeathHit'｜TyrantTim｜（11 s；调研未给完整 URL）（来源页 https://sonniss.com/gameaudiogdc） | — | ①② Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；③④ CC0 | 部分：非语言痛呼/喘息/咆哮无口音问题，可零成本满足；带语句的台词仍缺（GAP-A12） | [F] | [ ] 已下载 |
| G-VOX-003 | 战场人群 walla（远处喊叫/冲锋） | M2 | ① GDC2015-p1 Coll Anderson 'Battle Crowd'（12 文件）：`EFX EXT GROUP Battle Approach 01 A`、`Battle Cry 03 A`、`Battle Steady Fighting 03 A`、`Battle End Agony Moans 02 A`、`Marching Puddle Mud 01`、`Battle Movement Forrest 01 A` 等<br>② GDC2019-p2 Coll Anderson 'The Battle Crowd Collection Add on'（4 文件）<br>③ Freesound 563011 'People screaming in agony when charging into battle'｜florianreichelt｜CC0｜来源/真实性不明（调研未给 URL）（来源页 https://sonniss.com/gameaudiogdc） | — | ①② Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；③ CC0 | 部分：①为专业战斗群声库，语言构成需试听（吼叫为主，无清晰台词，对 FPS 反而合适） | [F 索引] | [ ] 已下载 |

### 8.18 音频：无线电与 UI（H-RAD-001、H-UI-001/002）

**覆盖率 3/3**（准确 2、部分 1）。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| H-RAD-001 | PTT 开/闭声、静电、对讲噪声（年代感） | M1 | ① GDC2016-p4 Sergey Eybog 'Handheld Tranceivers'：`Walkie Talkie, Cobra MicroTalk, … Noise Squelch, Beep, Series`、`Kenwood TK-3107 … Squelch Feedback`（现代对讲机）<br>② GDC2020-p2 Bluezone 'Interception – Radio Transmission'：`radio_transmission_interference_noise_transmitter`、`radio_beep_walkie_talkie`<br>③ GDC2015-p2 Detunized 'Newsreel Noises'：`Crackle_steady`、`Cadenced_buzz-hum`（老胶片噪声）<br>④ Freesound 524205 'Radio Sign Off / Squelch'｜JovianSounds；759967 / 760245｜LukaCafuka（ELBEX GT 410 + Zoom H1n）（来源页 https://sonniss.com/gameaudiogdc；④ 调研未给 URL） | — | ①②③ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；④ CC0 | 部分：**都是现代无线电**；年代感靠 EQ（窄带、失真、噪声）+ ③ 的老噪声（允许的后期，GAP-A10 仍在） | [F] | [ ] 已下载 |
| H-UI-001/002 | UI 点击/确认/拒绝/倒计时；占点 sting/复活提示 | M0 | ① Kenney Impact Sounds｜Kenney｜https://kenney.nl/assets/impact-sounds（130 文件）；Kenney Interface Sounds https://kenney.nl/assets/interface-sounds（其余音频包许可未逐个读，Kenney 惯例为 CC0）<br>② Freesound 包：benzix2 'Game Sounds'（packs/23566）、el_boss 'Hexa Puzzle Saga SFX'（packs/37861）、newlocknew 'UI_Misc'（packs/34385）、el_boss 'General SFX'（packs/32182）<br>③ GDC 多个 UI 库（Digital Rain Lab 'Bleeping Buttons' 等；来源页 https://sonniss.com/gameaudiogdc） | — | ① CC0（Impact Sounds 页面明示）；② CC0（过滤命中）；③ Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练） | 准确：充足；占点 sting/复活提示需从中挑，风格契合度需试听 | [F] | [ ] 已下载 |

### 8.19 音频：音乐（I-MUS-001/002）

**覆盖率 2/3**（准确 0、部分 2；I-PD-001 二战公共领域录音见 §9.2，调研不建议）。CC-BY 条目需在游戏内 Credits 署名。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| I-MUS-001 | 主菜单管弦主题（60–120 s 可循环） | M1 | ① 'War Theme'｜Spring Spring｜https://opengameart.org/content/war-theme（timpani、铜管、合唱）<br>② 'Call to War'｜Umplix｜https://opengameart.org/content/call-to-war（2023，号角与军鼓，页面标不要求署名）<br>③ 'Laments of the War'｜Cethiel（OGA，管弦+合唱）；'Battle Theme A'｜cynicmusic；'Qazijamjam Orchestral Battle Theme' https://opengameart.org/content/qazijamjam-orchestral-battle-theme<br>④ Incompetech（Kevin MacLeod）https://incompetech.com/music/royalty-free/faq.html（需试听 "Film / Epic / Military" 类，具体曲目未选）<br>⑤ 'After The War'｜Fatal Exit｜https://fatalexit.itch.io/after-the-war（奇幻向） | — | ①③ CC0（页面标注）；② 页面标不要求署名；④ CC BY 4.0（需署名）；⑤ CC-BY 4.0（需署名，沿用现有清单） | 部分：都需试听挑选；**零采购下偏奇幻/电影化管弦，缺"二战进行曲"感**；≥320 kbps/无损需看单曲（OGA 多为 MP3/OGG） | [S]（OGA、⑤）/[F]（④ 许可页） | [ ] 已下载 |
| I-MUS-002 | 结算（胜/负）短曲 | M2 | 同 I-MUS-001；另有 'The Tread of War'（军事进行曲）｜Jonathan Shaw / Pixelsphere / Jobromedia｜https://opengameart.org/content/the-tread-of-war-jonathan-shaw-pixelsphere-jobromedia | — | 同 I-MUS-001；'The Tread of War'：CC-BY 3.0（需署名） | 部分：同上 | [S] | [ ] 已下载 |

### 8.20 音频：IR（J-IR-001/002）

**覆盖率 1/2**（准确 0、部分 1；J-IR-002 见 §9.2）。

| 需求 ID | 需要什么 | 里程碑 | 免费候选（A 档：CC0 / 公共领域 / CC-BY / 平台免费商用许可）名称｜作者｜URL | B 档候选（标页面许可状态） | 页面标注的许可 | 吻合度与主要不足 | 证据 | 状态 |
|---|---|---|---|---|---|---|---|---|
| J-IR-001 | 卷积 IR（室内/车内/室外） | M1 | ① GDC2024-p6 Pole Position 'The Vehicle Impulse Response Library'：`MAN Lions Coach … Altiverb 2 … INTERIOR`、`Patria 6x6 … Passenger Compartment … Altiverb 3`、`Pontiac Firebird … Altiverb`（Altiverb 格式需确认可转 WAV）<br>② GDC2018-p4 Navi Retlav Studio 'Impulse Reverbism / Exploration 4 / Fractalism / Cabinetism 2'（创意 IR）<br>③ Freesound 648945 'IR_Cathedral_5m_Stereo'｜Nox_Sound｜https://freesound.org/people/Nox_Sound/sounds/648945/（96 kHz/24-bit 立体声）<br>④ Freesound 252847 'IR New Office 01'｜kijjaz｜https://freesound.org/people/kijjaz/sounds/252847/；382908 'Little Basement'｜Uzbazur；868385 'IR_Two_Car_Garage_HandClap_02'｜loganzsound（来源页 https://sonniss.com/gameaudiogdc） | — | ①② Sonniss GameAudioGDC 许可（Unlimited User License v2.0：免版税、无需署名；禁止作为音效再分发；禁止 AI/ML 训练）；③④ CC0 | 部分：**没有室外开阔/森林/城镇 IR**；坦克内部 IR 可用①（不同车辆但金属腔体特征类似）；OpenAIR 因 503 未能核实 | [F] | [ ] 已下载 |

### 8.21 距离分层与混音的可行处理（音频，摘自调研）

免费单层录音可用"允许的后期"做距离分层（D-MOD 已列：响度标准化、剪辑、EQ/压缩、距离分层混音、卷积混响）：近→中（高频衰减、低通、减弱瞬态、加早期反射 + 短混响）；中→远（更强低通、延迟 = 距离/声速、混响湿比增高）；真实远麦（BAR、M1919A4、Thompson、M3、MP40 250 m、M1911A1 200 m）可直接用。**灰区（需负责人在 NEEDS_HUMAN H-002 的 D-MOD 确认中一并明确）**：①把 M1919 全自动叠加/变调后冒充 BAR 或 MG34；②把 MP40 的 250 m 远场叠到近场 SMG 上；③用 .22 跳弹变调代替 .30；④把多个来源的爆炸叠成"手雷爆炸"。**不属允许**：用 DAW 合成器/噪声生成新声音素材、自己录音、AI 生成。另：统一响度（LUFS）与 EQ 配方会增加调音工作量（调研粗估 3–5 人日【待调参】）。

### 8.22 搜到但已排除（不列为候选；排除项检查表）

排除原则（总控保留的唯一排除项）：**从商业游戏/电影提取或翻录的素材、盗版合集、倒卖的盗版包（含闲鱼等二手平台转卖的模型包）、来源不明的网盘资源**。理由：这类素材即使页面标了免费或 CC，作者也无权授权，会让整个游戏的授权链失效，且违背不自制/不侵权的底线。下载前对每件素材核对：标题/描述/标签是否含游戏名或 "ripped / extracted / from game"；是否批量上传且无制作过程；贴图是否带游戏式命名；Freesound 条目是否含游戏名/商业库名/"inspired by 某影视"。

| 已排除条目 | 原因 | 来源 |
|---|---|---|
| Sketchfab 'Mauser karabıner 98K'（Ekbercg，CC BY） | tag 含 "pubg"，疑似游戏提取 | 视觉 §2.2 |
| Sketchfab 'MP40 (one for all textures) for free fire' | 标题含游戏名 | 视觉 §2.2 |
| Sketchfab 'MP40 – Wolfenstein: Enemy Territory' | 页面自述为游戏原模型改尺寸 | 视觉 §2.2 |
| Sketchfab 'Strike Fighters tanks pack'（CC BY-NC） | 名称含游戏，疑似游戏资产 | 视觉 §2.5 |
| Sketchfab 'TF2 HD Lugermorph pack' | 《Team Fortress 2》游戏资产改造 | 视觉 §2.12 |
| 聚合站 stlfinder / 3dcadbrowser / modelnet.cs.princeton.edu 转载的模型 | 来源不明（V-WPN-US-008 仅有聚合站链接者已降为 B 档并标"来源存疑"） | 视觉 §2.12 |
| 网盘/论坛"WW2 资源合集"类链接 | 本次搜索未命中，也不查找 | 视觉 §2.12 |
| Freesound 387508 morganpurkis 'MG42 SFX.wav' | 自述混合了从游戏取得的 MG42 声音 | 音频 §2.0 |
| Freesound 704968 Artninja 'GUNMech_handling_and_movement_sounds' | 描述承认来源含商业库 Hollywoodedge | 音频 §2.0 |
| Freesound 258198 wadaltmon 'Thompson SMG Shot' | 自述合成拼贴，不是实录 | 音频 §2.0 |
| Freesound nioczkus '1911 Comedian Pistol / Perdition 1911 Pistol' | 明示以电影角色枪声为模板 | 音频 §2.0 |
| Freesound 262436 The_Sound_Side 'Real explosions Real gunshots.wav' | 来源是别人拍摄的视频，来源不明 | 音频 §2.0 |
| Freesound 840492 qubodup 'War Zone (Lo-Fi)' | 自述实为城市烟花录音 | 音频 §2.0 |
| Freesound Zagge28 'Incoming mortar 1/3/4'（241840/241838/241837） | 描述仅为标签式，来源不明，未列入候选 | 音频 §2.2 |
| BBC Sound Effects（RemArc） | 仅个人/教育/研究，商业游戏不可用 | 音频 §1 |
| Scan the World / MyMiniFactory 3D 打印 STL | 非游戏可用 PBR 资产，且条款为个人使用/NC 居多 | 视觉 §1、§2.12 |
| 78rpm / 二战时期商业唱片 | 录音与作曲各自版权，未进入 PD | 音频 §1 |


## 9. 找不到免费素材的项目与购买渠道

> 本节 = 两份调研里"**无合格免费候选**"的全部需求 ID（视觉 49 个 + 音频 11 个），再加上 §9.3"免费候选仅可占位、达不到可发布品质"的关键项。
> "哪里可以买"**只取自 §4 现有付费候选**（商品名、商店、URL、价格、证据标签原样保留；价格**未重新核实**，下单前以结算页为准）；§4 也没有付费候选的写"暂无现成商品，见 ASSET_GAPS 的 GAP-xxx"。
> 付费项一律为**备选**，需负责人逐项批准（勾选"[ ] 决定购买"）；购买前仍建议按 §2.3 排查游戏提取，必要时按 §1.2 向卖家确认浏览器分发（该流程仅在购买时作为建议，不再是硬性门槛）。
> "不买的后果"遵循 ASSET_GAPS §3 的灰盒占位约定：视觉占位 = 程序生成基本几何体，音频占位 = 静音 `AudioBuffer`，里程碑验收时仍须在 ASSET_GAPS 里对该 GAP 有"决定"。
> **2026-11-17 补充（以 §11 为准）**：①RenderHub 上 Loss 的德军武器系列（G43 / MG34 / MG42 / MP40 / FG42 与 $70 bundle）面数仅 4k–12k 且许可疑为 Editorial / IP Restricted，**不再推荐，见 §11.2**；②两个 $19.99 的 Fab 枪械包（FPS World War Guns Pack 美 / 德）**仅第三人称精度，不满足第一人称硬性要求**，见 §11.2；③EVOLVE Studio 的 Fab Pz.IV / Pz.V（各 $59.99，69.8K / 76.5K 三角，面数达标）**仅 Unreal 格式**，需在 UE 导出，见 §11.5。

### 9.1 视觉：无合格免费候选（49）

#### 武器（12）

| 需求 ID | 需要什么 | 里程碑 | 为什么免费的不行 | 哪里可以买（商店｜商品｜URL｜价格｜证据） | 不买的后果 | 决定 |
|---|---|---|---|---|---|---|
| V-WPN-US-002 | M1 Carbine | M2 | 搜索未见 M1 Carbine 的 CC 模型 | CGTrader 'weapon ww2 low poly'（含 M1 Carbine + Thompson M1）｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/weapon-ww2-low-poly｜价格需确认（未核实）；Fab 美军包未发现 M1 Carbine（偏 GAP） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-WPN-US-006 | M1919A6 | M3 | 免费搜索未见 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V09 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-WPN-US-007 | M1903A4 Springfield（带 M73 镜） | M2 | 免费搜索未搜到 | Fab 'FPS World War Guns Pack – United States'（**仅第三人称精度，不满足第一人称硬性要求**，见 §11.2）｜Raouf Benoudina｜https://www.fab.com/listings/00a8235f-055a-4532-9916-fe060949b9eb｜$19.99【核实】；含 M1903 Springfield（3k 顶点，是否 A4/带镜待核；M0/M1 美军枪包，同时覆盖 US-003/005/008） | 灰盒占位（ASSET_GAPS §3.2）；或用同包 M1903（无镜）近似 | [ ] 决定购买 |
| V-WPN-US-009 | M1A1 Bazooka | M3 | Sketchfab 搜索只见现代 Panzerfaust 3 等，未见 M1A1 | CGTrader 'Anti Tank Rocket Launcher Collection Low-poly PBR'（含 M1A1 Bazooka 约 5.5k 面 + Panzerschreck 约 6.2k 面，4096 PBR，Royalty Free）｜CGTrader｜https://www.cgtrader.com/3d-model-collections/anti-tank-rocket-launcher-collection-low-poly-pbr｜价格需人工确认（未核实） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-WPN-US-010 | Mk2 手雷 | M1 | 本次搜索免费包未见 Mk2 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V09（小件，可顺带向任一卖家询价） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-WPN-US-012 | M1/M3 刺刀（近战） | M2 | 严格意义无；仅 victorcstr 包内 Ka-Bar（非刺刀，许可未写明） | 暂无现成商品，见 ASSET_GAPS 的 GAP-V09 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-WPN-GE-002 | Kar98k + ZF41 镜（狙击型） | M3 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V09（可由第一人称高精度 Kar98k 卖家加购） | 灰盒占位（ASSET_GAPS §3.2）；或用 V-WPN-GE-001 的 Kar98k 去镜替代 | [ ] 决定购买 |
| V-WPN-GE-003 | Gewehr 43（半自动，10 发弹匣） | M3 | 本次未搜到免费候选 | **【不再推荐，见 §11.2】** RenderHub Loss 'WW2 Gewehr 43 Walther G43 Rigged'｜Loss｜https://www.renderhub.com/loss/ww2-gewehr-43-walther-g43｜$15【核实，bundle 页】；4,368 多边形，已绑定；许可 "Extended Use (IP Restricted)"（含义未核实）；德军武器 bundle $70：https://www.renderhub.com/loss/world-war-2-german-weapons | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-WPN-GE-006 | MG34（两脚架/弹鼓） | M3 | 免费无合格候选 | **【不再推荐，见 §11.2】** RenderHub Loss 'MG34 Maschinengewehr'｜Loss｜https://www.renderhub.com/loss/world-war-2-german-weapons｜$20【核实】；12,149 多边形，GLB/FBX 等；许可 "Extended Use (IP Restricted)" 含义未核实 | 灰盒占位（ASSET_GAPS §3.2）；或用 MG42 外形近似 | [ ] 决定购买 |
| V-WPN-GE-009 | Panzerschreck（带防护板） | M3 | 免费无合格候选 | CGTrader 合集（同 V-WPN-US-009）；CGTrader 单品 Panzerschreck｜https://www.cgtrader.com/3d-models/military/gun/panzerschreck-german-anti-tank-rocket-launcher｜Royalty Free No AI，价格需确认（未核实） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-WPN-GE-012 | Nebelhandgranate 39（德军烟雾手雷） | M3 | 仅 3dmodelscc0 包内通用 'Smoke grenade'（非 Nebelhandgranate，可作占位） | 暂无现成商品，见 ASSET_GAPS 的 GAP-V09 | 灰盒占位（ASSET_GAPS §3.2）；或用通用烟雾手雷占位 | [ ] 决定购买 |
| V-WPN-GE-014 | S-Mine（跳雷） | M3 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V09 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |

#### 角色（9）——零采购档下最大且唯一无法用加工弥补的缺口

| 需求 ID | 需要什么 | 里程碑 | 为什么免费的不行 | 哪里可以买（商店｜商品｜URL｜价格｜证据） | 不买的后果 | 决定 |
|---|---|---|---|---|---|---|
| V-CHR-US-001 | 美军步兵（夏季，25k–45k 面，带绑定） | M1 | 免费、带绑定的 WWII 士兵 A/B 档均无；D-MOD 不允许从零建模 | ① RenderHub 'SOLDIER WWII USA Paratrooper Rigged'｜albin｜https://www.renderhub.com/albin/soldier-wwii-usa-paratrooper｜$60【搜索摘要】<br>② RenderHub 'Alpenwolf WW2 US Soldier'｜https://www.renderhub.com/alpenwolf/ww2-us-soldier｜约 46.3k 多边形，PBR + rigged【搜索摘要】<br>③ TurboSquid 'WW2 US Soldier'｜https://www.turbosquid.com/3d-models/3d-army-man-model-1593497｜$59.99【搜索摘要】（TurboSquid 专有格式条款风险） | 灰盒占位（胶囊体）；或用 Mixamo 通用人形作 bot/占位（不是 WWII 兵） | [ ] 决定购买 |
| V-CHR-US-002 | 美军空降兵 | M1 | 同上 | albin Paratrooper（同 V-CHR-US-001 ①）；CGTrader 'ww2 us paratrooper and thompson m1a1'（衣服类）https://www.cgtrader.com/3d-models/character/clothing/ww2-us-paratrooper-and-thompson-m1a1；RenderHub '101st Airborne Division Soldier'（未标 rigged，价格需确认） | 同上 | [ ] 决定购买 |
| V-CHR-US-003 | 美军兵种变体（军官/医护/坦克兵/狙击手） | M2 | 缺基础身体，无法拼装 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V18（需基础身体 + 配件拼装，依赖 D-MOD 确认） | 同上 | [ ] 决定购买 |
| V-CHR-US-004 | 美军冬季（Foy） | M4 | 未发现冬季美军 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V04（夏季角色 + 冬季服装模块，需确认 D-MOD） | 同上 | [ ] 决定购买 |
| V-CHR-GE-001 | 德军国防军（夏季） | M1 | 免费无合格候选 | CGTrader 'German Soldier Rig – WW2 Scanned Asset Pack'｜https://www.cgtrader.com/3d-models/character/clothing/german-soldier-rig-ww2-scanned-asset-pack（亦见 https://skfb.ly/6UwUt）｜价格需确认（按 $40–80【估算】）；备选 CGTrader 'Wehrmacht Soldier - RIGGED ANIMATED' https://cgtrader.com/3d-models/military/military-character/wehrmacht-soldier-rigged-animated（价格需确认） | 同上 | [ ] 决定购买 |
| V-CHR-GE-002 | 德军伞兵 FJR | M3 | 未发现 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V05（国防军角色 + 贴图变体替代，需确认 D-MOD；Loss FG42 可作武器配套） | 同上 | [ ] 决定购买 |
| V-CHR-GE-003 | 德军冬季 | M4 | 未发现 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V04 | 同上 | [ ] 决定购买 |
| V-CHR-GE-004 | 德军坦克兵/军官 | M3 | 缺基础身体 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V18（'WW2 Axis Uniform German' 服装 https://www.cgtrader.com/3d-models/character/clothing/ww2-axis-uniform-german 可做底） | 同上 | [ ] 决定购买 |
| V-CHR-FPA-001 | 第一人称手臂（美/德袖 + 手套） | M1 | 免费无 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V19（Fab 现代 FPS 手臂 + 动画包为近似） | 灰盒占位；或现代手臂 + 重材质（需确认） | [ ] 决定购买 |

#### 动画（3）

| 需求 ID | 需要什么 | 里程碑 | 为什么免费的不行 | 哪里可以买（商店｜商品｜URL｜价格｜证据） | 不买的后果 | 决定 |
|---|---|---|---|---|---|---|
| V-ANM-006 | 救治/拖拽伤员/复活 | M2 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V17 | 通用动作 + 手动对位；或灰盒占位 | [ ] 决定购买 |
| V-ANM-008 | 载具进出/驾驶/炮手/乘员 | M1 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V08（需自制或外包） | 通用动作 + 手动对位 | [ ] 决定购买 |
| V-ANM-009 | 第一人称动画（含栓动全流程、换弹） | M1 | 免费动捕库没有第一人称武器动画 | 近似：Fab 'First Person Rifle Animations V2'（现代战术，UE5 骨架；无链接）；带栓动、检查枪膛的现代狙击包 https://www.fab.com/listings/3f24e5a1-8c3e-4bc3-8b59-b8191befd4eb 、https://www.fab.com/listings/57e4a0d3-d8fe-4d4c-ab55-6cae93f2ff53（均现代枪，价格/许可未核实）；见 ASSET_GAPS 的 GAP-V07 | 现代包 + 重定向（观感偏现代）；或灰盒占位 | [ ] 决定购买 |

#### 载具（17）

| 需求 ID | 需要什么 | 里程碑 | 为什么免费的不行 | 哪里可以买（商店｜商品｜URL｜价格｜证据） | 不买的后果 | 决定 |
|---|---|---|---|---|---|---|
| V-VEH-US-001 | M8 Greyhound | M3 | 免费无合格候选 | CGTrader 'm8 greyhound game ready'（85k tris）｜https://www.cgtrader.com/3d-models/vehicle/military-vehicle/m8-greyhound-game-ready｜价格需人工确认（未核实） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-US-002 | M5A1 Stuart | M3 | 免费无合格候选 | CGTrader 'low poly m5a1 stuart light tank'｜https://cgtrader.com/3d-models/military/vehicle/low-poly-m5a1-stuart-light-tank｜价格需确认（未核实） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-US-005 | GMC CCKW 2½ ton 卡车 | M3 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V10（BuiltByBit 4 车包用途为 Minecraft 类平台，不建议） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-US-006 | M3 半履带 | M3 | 免费无合格候选（仅 CGTrader 付费） | CGTrader 'Low Poly M3A1 Half-track Personnel Carrier'（约 11.9k 多边形，PBR，Royalty Free）｜https://cgtrader.com/3d-models/military/vehicle/low-poly-m3-half-track-personal-carrier｜价格需确认（未核实） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-US-007 | Willys Jeep | M1 | 免费候选页无许可/无下载按钮 | CGTrader 'ww2 jeep usa'｜https://www.cgtrader.com/3d-models/military/military-vehicle/ww2-jeep-usa ；'ww2 american willys jeep' https://www.cgtrader.com/3d-models/vehicle/military-vehicle/ww2-american-willys-jeep｜价格需确认（按 $20–50【估算】） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-GE-001 | Sd.Kfz. 234 Puma | M3 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V10 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-GE-002 | Panzer II Luchs | M3 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V10（TurboSquid 仅见 PZII-G，非 Luchs） | 灰盒占位（ASSET_GAPS §3.2）；或用 Ausf. G 作最接近替代 | [ ] 决定购买 |
| V-VEH-GE-006 | Opel Blitz | M3 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V10（BuiltByBit 4 车包不建议） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-GE-007 | Sd.Kfz. 251 | M3 | 免费仅见 Free3D 条目，许可未显示 | Unity Asset Store 'WW2 Asset Pack'（含 Sd.Kfz. 251，2K PBR，约 6.4k 三角）｜https://assetstore.unity.com/packages/slug/182490 ；Flipped Normals 同款 https://flippednormals.com/product/ww2-asset-pack-1613｜价格需确认（未核实） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-ART-001 | M2A1 105mm 榴弹炮 | M3 | 免费无合格候选 | CGTrader 'military artillery weapon howitzer 105mm'｜https://cgtrader.com/3d-models/military/military-vehicle/military-artillery-weapon-howitzer-105mm｜型号需看图，价格需确认（未核实）；近似 GAP-V11 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-ART-002 | leFH 18 | M3 | 免费无合格候选 | ① CGTrader 'Low Poly 10.5 cm leichte Feldhaubitze'（约 6.7k 多边形，PBR，rigged）｜https://cgtrader.com/3d-models/military/gun/low-poly-10-5-cm-leichte-feldhaubitze-anti-tank-gun<br>② RenderHub Howitzer FH18｜https://www.renderhub.com/gemreer/howitzer-fh18-artillery-unit<br>③ Hum3D 10.5 cm leFH 18｜https://hum3d.com/3d-models/10-5-cm-lefh-18/｜$150【搜索摘要】（约 70.8 万面，需减面）；①② 价格需确认 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-ART-003 | 57mm M1 反坦克炮 | M3 | 免费无合格候选 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V10（RenderHub Pak 36 37mm 可作外形近似，非同型号） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-ART-004 | Pak 40 | M3 | 免费条目（CGTrader / Free3D）的免费状态与许可均未核 | ① CGTrader 'Low Poly 7.5 cm Pak 40'｜https://cgtrader.com/3d-models/military/gun/low-poly-7-5-cm-panzerabwehrkanone-40-anti-tank-gun｜$20【搜索摘要】<br>② RenderHub Loss 'WW2 Pak 40 - Rigged'｜https://www.renderhub.com/loss/ww2-pak-40-rigged-panzerabwehrkanone｜约 16k 面，Extended Use，价格需确认 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-VEH-AIR-001 | P-47 | M3 | 免费无合格候选 | CGTrader 'United States fighter aircraft Republic P-47 Thunderbolt'（低模 PBR 约 1.2 万面，Royalty Free）｜https://cgtrader.com/3d-models/aircraft/military/united-states-fighter-aircraft-republic-p-47-thunderbolt｜$139【搜索摘要】 | 灰盒占位（十字形盒，仅过场/能力演出） | [ ] 决定购买 |
| V-VEH-AIR-002 | Ju 87 | M3 | 免费无合格候选 | CGTrader 'ju87 b2 stuka 4k pbr game asset'｜https://www.cgtrader.com/3d-models/aircraft/military-aircraft/ju87-b2-stuka-4k-pbr-game-asset-for-unity-unreal｜价格需确认；GameDev Market 'WW2 German Dive Bomber Ju87 Stuka' https://gamedevmarket.net/asset/ww2-german-dive-bomber-ju87-stuka｜$26【搜索摘要】（买家留言称贴图缺失） | 同上 | [ ] 决定购买 |
| V-VEH-AIR-003 | Bf 109 | M3 | 免费无合格候选 | RenderHub 'Bf-109 F'｜https://www.renderhub.com/game-ready-models/aircraft｜价格/面数需确认（未核实） | 同上 | [ ] 决定购买 |
| V-VEH-AIR-004 | C-47（Airhead 过场） | M3 | 免费无合格候选 | CGTrader 合集 https://www.cgtrader.com/3d-model-packs/historic-air-superiority-and-logistics-collection-wwi-and-wwii（确认非 AI 制作）；Sketchfab 'WW2 US Military Transport Aircraft C-47 Skytrain' https://sketchfab.com/3d-models/ww2-us-military-transport-aircraft-c-47-skytrain-1aa07eceb48a48a091ce72a5580e8a28｜$29.99【搜索摘要】（Sketchfab Store 已关闭，价格来源存疑） | 同上 | [ ] 决定购买 |

#### 建筑 / 自然 / 道具 / 特效（8）

| 需求 ID | 需要什么 | 里程碑 | 为什么免费的不行 | 哪里可以买（商店｜商品｜URL｜价格｜证据） | 不买的后果 | 决定 |
|---|---|---|---|---|---|---|
| V-BLD-003 | 乡村教堂（可破坏钟楼） | M1 | Sketchfab 只见 CyArk 的 Normandy American Cemetery Chapel（页面标 premium）；Devon 废教堂扫描为 CC BY-NC-SA 且不在法国 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V02 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-BLD-004 | 城镇街道模块 | M2 | Poly Haven `modular_urban_apartments_facade` 等风格非诺曼底，且 D-MOD 不允许改建筑本体 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V02 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-BLD-005 | 阿登村舍（半木、石基、屋顶雪） | M4 | 免费无 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V02 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-BLD-008 | 桥梁（石拱/木桥/桁架） | M2 | Poly Haven 仅 `modular_wooden_pier` 等，不适用 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V13（本次未搜，建议补搜） | 灰盒占位（ASSET_GAPS §3.2）；或地形 + 通用结构件 | [ ] 决定购买 |
| V-BLD-009 | 火车站（小站站房 + 站台） | M3 | 免费无 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V13（本次未搜，建议补搜） | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-NAT-008 | 水（河/池塘/海浪）材质与泡沫 | M4 | 需要着色器与泡沫贴图；ambientCG 无 | 暂无现成商品（ASSET_SHOPPING_LIST §4.5.1：视需求补搜；ASSET_GAPS 无专属 GAP 编号） | 纯色/引擎内置占位水面 | [ ] 决定购买 |
| V-PRP-007 | 军用帐篷、伪装网 | M2 | 免费无 | CGTrader '3dt military base modular pack'｜https://www.cgtrader.com/3d-models/military/other/3dt-military-base-modular-pack｜风格需看图，价格需确认（未核实）；见 ASSET_GAPS 的 GAP-V12 | 灰盒占位（ASSET_GAPS §3.2） | [ ] 决定购买 |
| V-FX-006 | 曳光/弹道线/弹壳 | M1 | 仅 victorcstr 包带弹壳（许可未写明，属 B 档） | 暂无现成商品；弹壳可能随武器包附带（Fab 美/德包为 FBX 单件，可能无弹壳，未核实） | 引擎内置粒子占位 | [ ] 决定购买 |

### 9.2 音频：无合格免费候选（11）

| 需求 ID | 需要什么 | 里程碑 | 为什么免费的不行 | 哪里可以买（商店｜商品｜URL｜价格｜证据） | 不买的后果 | 决定 |
|---|---|---|---|---|---|---|
| A-WPN-010 | G43/G41 | M1 | GDC 索引 `g43/gewehr` 命中 0；Freesound 未搜到 | Sonniss / A Sound Effect 'Weapons of World War II – Designed'｜https://sonniss.com/sound-effects/weapons-world-war-ii-designed/｜$38.50（原价 $55）[F]（含 G43；同时补 Garand 开火等 14 枪）；GCRadix 'Firearm Foley & Mechanics: WW2'（含 K43、G43）$119.40 [F]；BOOM 仅 Gewehr 41 | 静音占位；或用 Kar98k/M1 Carbine 声替代 | [ ] 决定购买 |
| A-WPN-012 | StG 44 | M2 | GDC 索引 `stg/sturmgewehr` 命中 0；Freesound 无 | ① Diegetix 'WW2 Weapons: German & US'｜A Sound Effect｜https://www.asoundeffect.com/sound-library/ww2-weapons-german-us/｜$25–$40（页面区间）[F]（德包 54 条含 StG44；48 kHz/24-bit，无距离分层）<br>② BOOM 'World War II Firearms'｜https://www.boomlibrary.com/sound-effects/world-war-ii-firearms/｜$139–$269 [F]（Bundle 含 30 路麦克风做分层）；Sonniss 14 枪清单不含 StG44 | 静音占位；或用 MP40 / Kar98k 声替代（史实偏差） | [ ] 决定购买 |
| A-WPN-013 | MG34 | M2 | GDC 与 Freesound 均无 | 暂无现成商品，见 ASSET_GAPS 的 GAP-A01（BOOM/Sonniss/Diegetix/GCRadix 清单均无 MG34） | MG42 声混音/变调替代（需负责人接受史实偏差） | [ ] 决定购买 |
| B-AIR-001 | 航空炸弹落下 + 爆炸 | M3 | Freesound CC0 'aerial bomb falling whistle' 无结果；GDC 未检索到相关实录 | A Sound Effect 'Warfare 3 / Heavy Bombardment'｜https://www.asoundeffect.com/?p=760476（搜索所得，需核实）｜$199 [S]（未确认含航弹）；见 ASSET_GAPS 的 GAP-A15 | 静音占位；或用大爆炸 + 远炮合成（混音，需确认） | [ ] 决定购买 |
| D-TNK-004 | 主炮发射（内/外视角）+ 装填 | M1 | 免费只有通用远炮（GDC Warfare Library `cannon_firing_forest_distant`）与现代坦克，无 75 mm Sherman 炮或 88 mm 实录 | Pole Position 'Sherman Cannon 75mm M4A1'｜A Sound Effect（测试环境域名链接，正式页未核实）https://int.test-env.asoundeffect.com/sound-library/sherman-cannon-75mm-m4a1｜$149 [S]（近/中/远 + wild foley，空包录制；Sonniss 另有 '75mm Sherman M4A1 Cannon' 疑为同一产品，PENDING）；BOOM WW2 Tanks / Panther 亦含炮声 | 通用远炮声替代（无近场） | [ ] 决定购买 |
| D-TNK-005 | 坦克被击中/跳弹（内部回响） | M3 | 免费仅有枪打装甲板（GDC Warfare 2）等通用替代 | 暂无现成商品，见 ASSET_GAPS 的 GAP-A05（Warfare 2 金属板命中 $199 [F] 为替代） | 枪打装甲板 + 金属 Foley 替代 | [ ] 决定购买 |
| F-AMB-005 | 远处战场交火/炮击床（成品） | M1 | Freesound CC0 'battlefield ambience' 仅合成/循环预设 | 暂无现成商品（用 B-ART-003、C-BUL-001、A-WPN-014 远场文件自己混成，属混音；Warfare 2/3 远层可复用） | 自混床 | [ ] 决定购买 |
| G-VOX-002 | 德语战场喊话/痛呼 | M1 | Freesound 仅一条英语 'Germans!'；GDC 无德语军人喊话 | Shockwave-Sound 'SonicArchive Military Personnel German'｜https://shockwave-sound.com/sound-effects/12396/sonicarchive-military-personnel-german｜$30 [S]（46 条，16-bit/44.1 kHz，规格偏低，是否含 WW2 术语以试听为准） | 静音占位；仅用非语言痛呼 | [ ] 决定购买 |
| G-VOX-004 | 指挥官口令（英、德短句） | M2 | GDC 'Military Radio Voices' 为现代美式无线电语音，年代不符 | 暂无现成商品，见 ASSET_GAPS 的 GAP-A09（可授权的系统化喊话库未找到；真人配音属"重新录音"，须负责人本人决定） | 静音占位；或现代无线电语音（人工试听后定） | [ ] 决定购买 |
| I-PD-001 | 二战公共领域录音 | M4 | WW2 时期（1939–1945）录音在美国多数要到 2030s–2040s 才进 PD；NARA/Signal Corps 音轨多为事后配音 | 无（不建议；怀旧广播感用 H-RAD-001 后期处理）；需 NARA/LOC 或律师书面确认后才可考虑 | 静音占位 | [ ] 决定购买 |
| J-IR-002 | 远距离炮声大空间 IR（3–6 s，户外） | M2 | 免费 IR 均为室内/大厅 | 暂无现成商品；可借 Pole Position 各库远距离麦直接取代部分需求（付费库见 ASSET_SHOPPING_LIST §4.2.6）；免费可用 Freesound 648945 大教堂 IR（非户外）作近似 | 用 GDC 真实远麦录音本身（已含环境反射） | [ ] 决定购买 |

### 9.3 免费候选"仅可占位 / 达不到可发布品质"的关键项

这些需求 ID **在 §8 里有免费候选**（不计入 §9.1/§9.2 的 60 项），但调研判断其精度、风格、型号或来源**不足以作为最终发布品质**；如果负责人想在零采购之外补强，这里是性价比线索。

| 需求 ID | 需要什么 | 里程碑 | 为什么免费的不行 | 哪里可以买（商店｜商品｜URL｜价格｜证据） | 不买的后果 | 决定 |
|---|---|---|---|---|---|---|
| V-WPN-US-001 等全部第一人称武器 | 第一人称 25k–60k + 内部结构 + 可动部件 | M1 | A 档 3D 武器面数几乎全在 4.5k–20k（第三人称区间）；**没有第一人称级免费模型**；拆分枪机/弹匣需切分网格（D-MOD 待确认，GAP-V06） | CGTrader 'M1 Garand - Game and VR Ready'｜https://cgtrader.com/3d-models/military/gun/m1-garand-game-and-vr-ready｜$25【搜索摘要】；CGTrader 'M1 Garand With Complete Internals - Game Ready'（带内部结构）https://www.cgtrader.com/3d-models/military/gun/m1-garand-with-complete-internals-game-ready｜价格需人工确认 | M1 先用第三人称品质免费模型（GAP-V06 默认建议） | [ ] 决定购买 |
| V-WPN-GE-001 | Kar98k（M0/M1） | M0 | 免费只有 Poly Haven 栓动步枪占位（19,985 三角，非 Kar98k） | Fab 'FPS World War Guns Pack – German'（**仅第三人称精度，不满足第一人称硬性要求**，见 §11.2）｜Raouf Benoudina｜https://www.fab.com/listings/13b4dea1-2849-480a-a3c7-b16fa5bd329a｜$19.99【核实】；含 Kar98k 2.6k 顶点、StG44、MP40、Luger P08（品质偏低，第三人称；许可证页未显示名称，待确认 Standard） | 用 Poly Haven 栓动步枪占位（不可冒充 Kar98k） | [ ] 决定购买 |
| V-WPN-US-003/005/008 | Thompson、BAR、M1911A1（免费仅 Thompson 有 A 档；BAR/M1911 只有 B 档） | M1 | BAR 仅 CC-BY-SA/GPL；M1911 许可未写明 | Fab 'FPS World War Guns Pack – United States'（**仅第三人称精度，不满足第一人称硬性要求**，见 §11.2）｜https://www.fab.com/listings/00a8235f-055a-4532-9916-fe060949b9eb｜$19.99【核实】（Thompson 5.7k 顶点、M1911 2.5k、M1903 3k、BAR 4k；BAR 标注非 A2 版，需核对） | BAR/M1911 灰盒占位；或使用 B 档（负责人决定） | [ ] 决定购买 |
| V-VEH-US-003 | M4 Sherman（M1 的 1 辆坦克） | M1 | 免费 Sherman 无贴图、未绑定、无内饰 | CGTrader 'Sherman M4 Gameready Rigged'｜https://www.cgtrader.com/3d-models/military/military-vehicle/sherman-m4-gameready-rigged｜$31【搜索摘要】（履带/轮随车体动，PBR 4K，Royalty Free；内饰未提） | 免费 Sherman + 单色材质 + 简化内饰盒（GAP-V01） | [ ] 决定购买 |
| V-VEH-GE-004 | Panther | M3 | 免费版约 211.5k 三角，需大幅减面 | CGTrader 'Panzerkampfwagen V Panther G with texture pack GameReady'｜https://cgtrader.com/3d-models/vehicle/military-vehicle/panzerkampfwagen-v-panther-g-with-texture-pack-gameready｜$15【搜索摘要】（rigged，Royalty Free No AI）；'Panther G Tank' $5 https://cgtrader.com/3d-models/vehicle/military-vehicle/panther-g-tank-e909cd72-0ca7-456c-b390-b722e16d741f | 减面后使用免费版 | [ ] 决定购买 |
| V-VEH-GE-005 | Tiger I | M3 | 免费只有 OGA Tiger（贴图许可存疑，B 档） | CGTrader 'Tiger 1 GameReady tank'（炮塔旋转/炮管俯仰/负重轮悬挂层级）｜https://cgtrader.com/3d-models/military/military-vehicle/tiger-1-gameready｜价格需确认；RenderHub max3ddd Tiger I｜https://www.renderhub.com/max3ddd/tiger-i-german-heavy-tank（65.5k 多边形，Extended Use；价格需确认）。含部分内饰的另一件 Tiger 被标注 AI 辅助制作，已排除 | 灰盒占位；或 B 档 OGA Tiger（负责人决定） | [ ] 决定购买 |
| V-BLD-001/002 | 诺曼底石屋/农场（全套建筑） | M1 | 免费只有材质与非诺曼底扫描；D-MOD 不允许从零建模 | 暂无现成商品（CGTrader/Fab 只见中世纪/奇幻村庄套件，风格不符），见 ASSET_GAPS 的 GAP-V02 | 灰盒盒体 + Poly Haven 墙面材质（M1 画面不会出现像样的建筑） | [ ] 决定购买 |
| V-BLD-006/007 | 大西洋壁垒碉堡与滩头障碍（A 档只有低模 M178；障碍只有 B 档包） | M4 | M178 仅 2.4k 三角；障碍/沙袋只有许可未写明的 itch 包 | CGTrader 'D-Day Atlantik Wall Collection'｜https://www.cgtrader.com/3d-model-packs/d-day-atlantik-wall-collection｜$89【搜索摘要】（21 件低模 PBR，含 6 种滩头障碍；Royalty Free）；CGTrader 'bunker m162'（Regelbau 带 3 LOD）https://www.cgtrader.com/3d-models/military/armor/bunker-m162；CGTrader 'WW2 Anti-tank Obstacle PBR Game ready' https://www.cgtrader.com/3d-models/military/other/ww2-anti-tank-obstacle-pbr-game-ready｜$5.99【搜索摘要】 | 免费 M178 低模 + B 档障碍包（负责人决定） | [ ] 决定购买 |
| V-PRP-001/002/003 | 沙袋、铁丝网、反坦克障碍 | M1 | A 档无；只有一个许可未写明的 itch 包 | CGTrader 'sandbag pack 12 modular game ready assets with 4k pbr support'｜https://www.cgtrader.com/3d-models/military/other/sandbag-pack-12-modular-game-ready-assets-with-4k-pbr-support｜价格需确认；Flipped Normals 'Czech Hedgehog Obstacle WWII' https://flippednormals.com/product/czech-hedgehog-obstacle-wwii-2440（1,202 面，价格需确认） | B 档 itch 包（负责人决定）或灰盒盒体 | [ ] 决定购买 |
| V-NAT-001 | 树篱 bocage（Carentan 战术核心） | M1 | 无扫描；Poly Haven 灌木 + 石墙材质只能拼装近似 | CGTrader/TurboSquid UK hedgerow 植物包｜https://www.cgtrader.com/3d-models/plant/leaf/uk-hedgerow-3d-collection｜$4–$40【搜索摘要】；**无真正的 bocage 土堤**，见 ASSET_GAPS 的 GAP-V03 | 灌木 + 土堤材质组合替代（碰撞/掩体判定按玩法数据） | [ ] 决定购买 |
| V-NAT-004（农作物） | 麦/玉米农作物 | M2 | 免费无 | 暂无现成商品，见 ASSET_GAPS 的 GAP-V14 | 草 + 贴图平面 | [ ] 决定购买 |
| V-ANM-001/002 | 第三人称步枪运动集、持枪匍匐 | M0/M1 | 免费只有 Mixamo/CMU 通用动作，无步枪专用 | MoCap Online 'Rifle Starter'｜https://mocaponline.itch.io/rifle-starter-anims｜$3.99【搜索摘要】（走/慢跑/蹲姿 + 瞄准）；MoCap Online Rifle Pro（Fab/官网，价格需确认）；kevdev 'Human Soldier Animations'｜https://kevdev.itch.io/human-soldier-animations｜$30【搜索摘要】（蹲走 8 向与匍匐） | Mixamo 通用动作 + 免费 MoCap Online 手枪包凑数 | [ ] 决定购买 |
| V-FX-001..005 | 枪口焰、烟、爆炸、弹孔、血迹（写实） | M1 | 免费 2D 序列帧/贴花多为风格化或像素风 | RenderHub 'RC Art - Muzzle Effects'｜https://www.renderhub.com/reactorcore/rc-art-muzzle-effects｜CC-BY 4.0（需署名）；Fab '37 sprite sheet effects'｜https://fab.com/s/cc7e3fda26c2｜价格需确认；ducvu Gumroad '2D FX Explosion Pack' https://ducvu.gumroad.com/l/2d-fx-explosion-pack（风格偏风格化） | 免费 2D 序列帧（风格化）或引擎内置粒子 | [ ] 决定购买 |
| A-WPN-001 | M1 Garand 开火声（M0） | M0 | 免费只有来源不明的 Freesound 条目（MP3 128 kbps） | Sonniss / A Sound Effect 'Weapons of World War II – Designed'｜https://sonniss.com/sound-effects/weapons-world-war-ii-designed/｜$38.50（原价 $55）[F]（96 kHz/24-bit，133 变体，含空仓 ping） | 接受来源不明的 Freesound 条目，或先用 M1903A3/Kar98k 近场声暂代 | [ ] 决定购买 |
| A-WPN-005 | BAR 全自动 | M1 | 免费只有二连发 | 暂无现成商品，见 ASSET_GAPS 的 GAP-A02（BOOM 清单无 BAR；Designed 仅单发） | M1919 全自动 + EQ 替代 | [ ] 决定购买 |
| A-WPN-011 / A-WPN-014 | MP40 近场、MG42 近/中/远与近距尾音 | M1 | MP40 只有 250 m 远场；MG42 只有山顶长点射 | Pole Position 'MG 42 Machine Gun'｜Sonniss｜https://sonniss.com/product/mg-42-machine-gun/｜$111.75（原价 $149）[F]（96 kHz/24-bit，183 文件，两地点长/短尾音，含操作声）；MP40 见 Sonniss Designed $38.50 [F] | MP40 用近场 SMG 声 + EQ 替代；MG42 用山顶样本 + 单层距离处理 | [ ] 决定购买 |
| B-HVY-001 | Bazooka / Panzerschreck / Panzerfaust | M3 | 免费只有现代 RPG 与权属存疑的好莱坞母带 | The Warfare 2 Library｜Pole Position｜https://www.asoundeffect.com/sound-library/the-warfare-2-library｜$199 [F]（RPG、榴弹发射器、火炮，现代军演录制）；Sound Ideas 'Rockets & Missiles'｜$295 [S]；见 ASSET_GAPS 的 GAP-A03 | 现代 RPG + EQ 替代（史实偏差） | [ ] 决定购买 |
| D-AIR-001 / B-AIR-002 | P-47 / Ju 87 Stuka / C-47 与扫射 | M3 | 免费无 P-47/C-47/Ju 87 真实实录 | A Sound Effect 'WW2 Aircraft'（约 $150，英国机场录制）[S]；Sound Ideas 'Sounds of War'｜https://sound-ideas.com/products/sounds-of-war-sound-effects-library｜$129 标价/约 $65 促销 [S]（含 Hurricane、Stukas、Flying Fortress）；A Sound Effect B-25 Mitchell 约 $125 [S]；Sound Ideas P-51 Mustang by BOOM $65 [S]；见 ASSET_GAPS 的 GAP-A06 | P-51D 替代（需接受音色偏差） | [ ] 决定购买 |
| H-RAD-001 | WW2 年代无线电（PTT/静电） | M1 | 免费都是现代无线电 | A Sound Effect 'Squelch'（6 台电台/对讲机/CB 实录）[S，价格/URL 需核实]；MuseHub 老式电台静电 https://www.musehub.com/loops-and-audio/radios-and-tvs [S]（许可需读）；Sonniss 'Military Radio Voices' https://sonniss.com/product/military-radio-voices [S]；见 ASSET_GAPS 的 GAP-A10 | 现代对讲机声 + 带通/压缩（允许的后期） | [ ] 决定购买 |
| I-MUS-001 / I-MUS-002 | "二战进行曲"风格配乐 | M1 | 免费 OGA/Incompetech 偏奇幻/电影化管弦 | Daniel Carl 'Top Secret WW2 Music Pack'｜https://danielcarlmusic.itch.io/top-secret-ww2-music-pack [S，许可需读]；Bobby Cole 'Warfare Soundtrack'｜https://bobbycolemusic.itch.io/warfare-spundtrack-25-tracks-vol-1 [S]；Redearth 'Battle Litany'｜https://redearthmusic.itch.io/battle-litany [S]；见 ASSET_GAPS 的 GAP-A13（价格调研未给出） | 接受奇幻/电影化管弦；或静音 | [ ] 决定购买 |
| G-VOX-001 / G-VOX-003（台词类） | 英语士兵带语句台词、战场人群 | M1 | 免费只有非语言痛呼/咆哮与战斗群声 | Raketenfaust 'Army Voice Pack'｜https://raketenfaust.itch.io/army-voice-pack｜$4.49 起 [S]（155 条 WW2 士兵语音；许可需读）；A Sound Effect 'British Soldier Voices'（800+，英式）https://www.asoundeffect.com/sound-library/british-soldier-voices [S]；'Historical Battle Voices Vol. 1'（约 $33）[S]；见 ASSET_GAPS 的 GAP-A12 | 非语言痛呼/咆哮；台词暂缺 | [ ] 决定购买 |
| C-BUL-004 / C-BUL-005 | 击中人体、压制感（低频冲击/耳鸣） | M1 | 免费只有设计/Foley 库；耳鸣未专门搜 | 暂无现成商品，见 ASSET_GAPS 的 GAP-A04；A Sound Effect 'Bullet Bys & Impacts'｜https://www.asoundeffect.com/?p=414565｜$29.99 [S]（先看文件清单确认是实枪录制） | 沙袋/泥土命中声叠加；AudioWorklet 滤波做压制 | [ ] 决定购买 |
| D-TNK-001（车外循环） | Sherman 车外近/远发动机循环 | M1 | GDC 样本均为车载/车内 | Pole Position 'Sherman M4A3 Medium Tank'｜https://sonniss.com/product/sherman-m4a3-medium-tank｜$224.25 [F]（96 kHz/24-bit，183 文件/25.47 GB，16 车外 + 12 车内声道，Ford GAA V8） | 用车载样本 + 后期距离处理 | [ ] 决定购买 |
| B-ART-001（近场） | 105 mm 榴弹炮近场（0–50 m） | M3 | GDC 样本只有远/中远 | Pole Position '105mm Howitzer Cannon'｜https://www.asoundeffect.com/sound-library/howitzer-cannon-105mm｜$149 [F]（27 声音，24/96，0–150 m 近/中/远；页面未写型号） | 用远/中远样本 + 后期 | [ ] 决定购买 |

### 9.4 花最少的钱补最关键的缺口（"最小补缺"推荐清单）

> 合并两份调研的建议：**视觉约 8 件、音频 3–5 个库**。**价格全部沿用现有清单（ASSET_SHOPPING_LIST §4 / §5.1）的原值，本次未重新核实**；【核实】= 清单读到商品页，【搜索摘要】= 摘要所见，【估算】= 无来源。这些仍受原 D-LIC 流程建议约束（CGTrader / RenderHub 的卖家确认），但按负责人 2026-11-17 指示可自行决定跳过。

#### 9.4.1 视觉（8 件）

| 序 | 需求 ID | 推荐商品 | 价格（未重新核实） | 理由 |
|---|---|---|---|---|
| 1 | V-CHR-US-001（含 -002 变体基础身体） | RenderHub albin 'SOLDIER WWII USA Paratrooper Rigged' 或 Alpenwolf 带绑定版 | 约 $60【搜索摘要】 | 免费完全没有；一个带绑定的美军士兵同时解决 M1 全部美军角色 |
| 2 | V-CHR-GE-001 | CGTrader 'German Soldier Rig – WW2 Scanned Asset Pack' | 价格需确认；清单估 $40–80（按 $60 计）【估算】 | 德军侧同上 |
| 3 | V-WPN-GE-001（含 -004、-005、-008） | Fab 'FPS World War Guns Pack – German' | $19.99【核实】 | 补 Kar98k 缺口；品质偏低（Kar98k 2.6k 顶点）；**仅第三人称精度，不满足第一人称硬性要求（见 §11.2）** |
| 4 | V-WPN-US-003（含 -005、-008、-007） | Fab 'FPS World War Guns Pack – United States' | $19.99【核实】 | 补 BAR（A 档无）、M1903 与 Thompson 的统一风格；**仅第三人称精度，不满足第一人称硬性要求（见 §11.2）** |
| 5 | V-ANM-001 | MoCap Online 'Rifle Starter' | $3.99【搜索摘要】（Pro 价格需确认） | 免费库没有步枪专用动作 |
| 6 | V-VEH-US-003 | CGTrader 'Sherman M4 Gameready Rigged' | $31【搜索摘要】 | 免费 Sherman 无贴图无绑定；M1 唯一一辆坦克 |
| 7 | V-VEH-GE-004 / V-VEH-GE-003 | CGTrader Panther G（$15）/ Fab Panzer IV Production models（$25.99） | $15【搜索摘要】/ $25.99【核实】；合计 $40.99 | 对手坦克；免费 Panzer IV 已有，Panther 免费版高面数。**注（§11.5）**：EVOLVE Studio 的 Fab Pz.IV / Pz.V（各 $59.99，面数达标）**仅 Unreal 格式**，需在 UE 导出 |
| 8 | V-BLD-006（含 -007） | CGTrader 'D-Day Atlantik Wall Collection'（21 件低模 PBR，含 6 种滩头障碍） | $89【搜索摘要】 | 一次补 BLD-006 与 BLD-007，让 Omaha（M4）有可用内容 |
| | | **视觉小计** | **约 $325【估算】**（60 + 60 + 19.99 + 19.99 + 3.99 + 31 + 40.99 + 89 = 324.96；其中序 2 德军角色价格需确认，按 $60 计） | 调研原文同为"约 $325（估算）" |

#### 9.4.2 音频（5 个库中先选 3–5 个）

| 序 | 需求 ID | 推荐商品 | 价格（未重新核实） | 理由 |
|---|---|---|---|---|
| ① | A-WPN-001 / 002 / 010 / 008 / 015 … | Sonniss / A Sound Effect 'Weapons of World War II – Designed' | $38.50【核实】 | 一次补齐 Garand、Carbine、G43、M1911、Luger、BAR 单发、M1903、MG42、MP40、Thompson、M3 等 14 枪；填掉 G43 与 Garand 开火声两个免费缺口 |
| ② | A-WPN-012（StG 44） | Diegetix 'WW2 Weapons: German & US' | $25–$40【核实，页面区间】 | 补免费完全空白的 StG 44（德包 54 条） |
| ③ | D-TNK-004（主炮发射+装填） | Pole Position 'Sherman Cannon 75mm M4A1' | $149【搜索摘要】（产品页状态 PENDING） | 补 Sherman 炮近/中/远；预算紧可延后到 M3 |
| ④ | G-VOX-002（德语喊话） | Shockwave-Sound 'SonicArchive Military Personnel German' | $30【搜索摘要】（规格偏低，需试听） | 补德语空白 |
| ⑤ | A-WPN-014（MG42 近/中/远 + 长/短尾音） | Pole Position 'MG 42 Machine Gun' | $111.75【核实】 | 免费包里仅 1 个山顶长点射；MG42 是关键兵种声音，值得单买 |
| | | **音频小计** | ①② = **$63.50–$78.50**（M0/M1 最关键）；①–④ = $242.50–$257.50；①–⑤ = **$354.25–$369.25** | 音频调研原文写"总计控制在约 $320 以内"，与五项相加不符（按其所列价格相加为 $354.25–$369.25），**本表按相加结果**；若只买到 ④ 则约 $243–$258；音频调研的整体上限提示为 ≤ US$300–$320 |

#### 9.4.3 合计

| 档 | 内容 | 合计（未重新核实） |
|---|---|---|
| 起步子集（M0 即可开工） | 序 3、4（两个 Fab 枪包 $39.98）+ 序 5（MoCap Online Rifle Starter $3.99）+ 音频 ①（$38.50） | **约 $82.47**（文档作者相加） |
| 最小补缺（推荐） | 视觉 8 件 + 音频 ①–⑤ | **约 $680–$695【估算】**（≈ $325 + $354–$369） |

> 注：未纳入的重要但无现成商品的缺口（诺曼底建筑、bocage、冬季角色、带内饰坦克、WW2 无线电/口令等）即使花这笔钱也**补不上**，见 §9.1 的"暂无现成商品"行与 ASSET_GAPS（外包价格见 ASSET_GAPS 各 GAP 的【估算】）。


## 10. 覆盖率总览

> 数字来自两份调研的计数（视觉 108 个需求 ID、音频 60 个需求 ID，共 168 个）；"准确可用"= 型号/用途对得上的 A 档候选数（**不等于品质达标**，也未下载验证）。视觉的"准确"归类是本节作者按调研原文的"近似/B 档"标注整理，已在 §8 逐行写明。

### 10.1 类别 × 覆盖率

| 类别 | 有免费候选数 | 总数 | 准确可用数 | 其余有候选者（近似/占位 · 仅 B 档） | 无候选（见 §9） |
|---|---|---|---|---|---|
| 武器（US 12 + GE 14） | 14 | 26 | 9 | 近似/占位 3 · 仅 B 档 2 | 12 |
| 角色 | 0 | 9 | 0 | — | 9 |
| 动画 | 7 | 10 | 0 | 近似（通用）7 | 3 |
| 载具（US 7 + GE 7 + ART 4 + AIR 4） | 5 | 22 | 3 | 近似 1 · 仅 B 档 1 | 17 |
| 建筑与掩体 | 5 | 10 | 1 | 近似 3 · 仅 B 档 1 | 5 |
| 自然 | 7 | 8 | 5 | 近似 2 | 1 |
| 道具 | 9 | 10 | 3 | 近似 2 · 仅 B 档 4 | 1 |
| 特效 | 5 | 6 | 0 | 近似/仅可占位 5 | 1 |
| 天空 | 3 | 3 | 2 | 近似 1 | 0 |
| 字体 | 4 | 4 | 4（凭记忆 [M]） | — | 0 |
| **视觉小计** | **59** | **108** | **27** | | **49** |
| 音频：枪械 | 14 | 17 | 6 | 部分 8 | 3 |
| 音频：爆炸与重武器 | 8 | 9 | 1 | 部分 7 | 1 |
| 音频：子弹 | 5 | 5 | 3 | 部分 2 | 0 |
| 音频：载具与飞机 | 6 | 8 | 3 | 部分 3 | 2 |
| 音频：Foley | 3 | 3 | 1 | 部分 2 | 0 |
| 音频：环境 | 5 | 6 | 4 | 部分 1 | 1 |
| 音频：人声 | 2 | 4 | 0 | 部分 2 | 2 |
| 音频：无线电与 UI | 3 | 3 | 2 | 部分 1 | 0 |
| 音频：音乐 | 2 | 3 | 0 | 部分 2 | 1 |
| 音频：IR | 1 | 2 | 0 | 部分 1 | 1 |
| **音频小计** | **49** | **60** | **20** | | **11** |
| **合计** | **108** | **168** | **47** | | **60** |

读法：**环境层**（地表材质、天空、岩石、树、草、箱桶道具、字体）免费即可接近 AAA 水准（Poly Haven 与 ambientCG 是最强来源，V-NAT-007 地表是整个清单覆盖最强的一项）；**决定"像不像二战 FPS"的另一半——角色、载具、建筑、第一人称武器与动画——要么没有、要么只有第三人称/低模/无贴图品质**，D-MOD 约束下无法靠加工弥补。音频方面：Sonniss GDC 里有大量真实二战型号样本（Thompson、M3、BAR、M1919A4、MG42、MP40、K98k、M1903A3、M1911A1、P38、Sherman、Panzer IV/Panther、Opel Blitz、M101 榴弹炮等），但每个库仅 2–4 个文件。

### 10.2 里程碑：只用免费素材时的状态

| 里程碑 | 可达到可玩的部分（免费） | 停留在灰盒/静音占位的部分 | 结论 |
|---|---|---|---|
| **M0** 技术验证 | 地形材质、HDRI 天空、冷杉植被、岩石、步枪（Garand A 档，第三人称品质）、Mixamo 基础动画、枪口焰/爆炸 2D、字体；音频：Kar98k 开火 + 拉栓、Garand ping/换夹、子弹 whizz、泥地脚步、UI | bot 角色（Mixamo/Kenney/Quaternius 通用人形，不是 WWII 兵）；第一人称武器与手臂；步枪专用动画（用手枪/通用动作凑）；Garand 开火声（需接受来源不明的 Freesound 条目，或以栓动枪声暂代） | **M0 目标可达成，不依赖付费** |
| **M1** 垂直切片 | Garand/Thompson/MP40/StG44/MG42（第三人称品质，需减面/贴图修整）；Sherman（25–30k，无贴图无绑定）；Panzer IV（有 PBR）；箱/桶/油桶/电台；冷杉与草灌；地表与天空；音频：Thompson、M3、M1919、M1911A1、M1903A3、K98k、MP40（仅远场）、P38/Luger、Sherman M4A3 车载引擎/履带、跳弹/远炮、乡村环境床、士兵痛呼、Battle Crowd | **全部 5 个兵种的美/德角色**；Kar98k（只有近似占位）；BAR（仅 SA/B 档；音频仅二连发）；M1 Carbine 模型；**诺曼底石屋/农场/教堂/街道**（灰盒盒体 + Poly Haven 墙面材质，GAP-V02）；树篱 bocage；第一人称手臂与动画；坦克内饰与炮塔可动件；载具乘员动画；水；音频：BAR 全自动、G43、Sherman 主炮、人体命中、无线电（现代对讲机 + 后期）、美式台词、室外 IR | **能出"能打、能玩的灰盒战场"，但画面里不会出现像样的士兵与建筑，不满足垂直切片的"展示画质"目的** |
| **M2** 完整 Frontline | 同 M1；Battle Crowd walla；MP40/Thompson 额外样本 | 另缺：桥梁、火车站、帐篷、农作物、欧式电线杆/围栏、写实血迹/弹孔；音频：StG 44（无）、MG34（无，用 MG42 替代）、德语喊话、口令、户外长尾 IR | **100 人压测可做（占位模型足够）**，"完整 Carentan 的视觉"仍是灰盒 + 通用材质 |
| **M3** 全兵种、全载具、Assault、指挥官能力、火炮 | Sherman/Panzer IV/Panther；105 mm（M101 远/中远）、迫击炮发射、各坦克与卡车引擎声、P-51D/Spitfire 掠过、转塔声 | M8、M5A1、M3 半履带、GMC CCKW、Willys、Puma、Luchs、Opel Blitz、Sd.Kfz.251、火炮 4 项、飞机 4 项全灰盒；Bazooka/Panzerschreck/M1919A6/手雷/地雷多为近似或缺；角色变体（军官/医护/坦克兵/狙击手）缺；动画缺救治、载具乘员、第一人称全套；音频：航空炸弹、Stuka 警笛、P-47/C-47、GMC/Willys、坦克被击中、来袭哨声 | 载具与重武器大面积灰盒/替代 |
| **M4** Omaha、Foy、Skirmish、优化、公开测试 | 沙/卵石/海崖/岩石材质与模型；**Regelbau M178 低模** + 1–2 个扫描碉堡（B 档 NC-SA）；**Foy 的 Easy Company 散兵坑扫描（CC BY，直接相关）**、雪面材质、雪天 HDRI、冷杉；音频：海浪、冬林环境 | 滩头障碍（仅 B 档低模）；水/海浪（无）；登陆艇（仅 B 档包里一个）；冬季德/美军角色全缺；村舍/教堂全缺；二战 PD 录音（无） | 环境可用，人物与建筑仍灰盒 |

### 10.3 调研局限（诚实声明）

1. **Sketchfab 站内筛选未能使用**（调研没有站内搜索，无法用 `Downloadable` + 许可过滤器），只能经通用网页搜索逛 Sketchfab，**召回率偏低**；很多条目的许可只是搜索摘要（[S]）；WebFetch 对部分 Sketchfab 页面返回空。
2. **"没搜到 ≠ 不存在"**：§9 里"无合格免费候选"只代表"本次搜索未见"，不代表平台上不存在。
3. 视觉调研**没有打开任何素材文件**，无法评估实际质量、比例、UV、绑定、贴图分辨率、历史准确性；音频调研**没有下载或聆听任何文件**，GDC 条目仅凭第三方索引的文件名/大小，Freesound 条目仅凭页面描述（WebFetch 经小模型摘要，个别 URL、时长、描述可能有转写误差）。
4. Megascans/Fab 免费内容只读到新闻层面，**没有逐个打开 Fab 商品页**，因此 Fab 里的具体免费资产未列入候选，只作为渠道。
5. 版权细节：真实商标、纳粹符号需要按 D-MOD 例外处理；扫描类文物若含涂装/徽章同样适用。
6. 凭记忆未核实：Mixamo 具体步枪动画、字体 OFL 文本、Mixamo/CMU 条款的官方原文。
7. 对照总纲：调研的 B 档（NC/SA/未写明）与总纲 §7 R12 的原红线口径不同；按负责人 2026-11-17 指示，B 档由负责人逐项决定，不再自动排除。
8. 音频 B 档中 **CC-BY-NC / CC-BY-SA / 个人使用的完整候选未系统检索**（把 Freesound 搜索过滤串换成 `license:"Attribution NonCommercial"` / `license:"Attribution ShareAlike"` 即可补搜，约 1–2 小时）。

### 10.4 建议人工补搜的需求 ID

**Sketchfab 用 `Downloadable` + `CC Attribution / CC0` 过滤器补搜**（优先级从高到低）：

- 角色（最大缺口）：V-CHR-US-001、-002、-003、-004；V-CHR-GE-001、-002、-003、-004；V-CHR-FPA-001（带 "rigged" 的 CC-BY 军人模型可能存在）。
- 武器：V-WPN-GE-001（Kar98k）、V-WPN-GE-002、V-WPN-GE-003（G43）、V-WPN-GE-006（MG34）、V-WPN-US-002（M1 Carbine）、V-WPN-US-006、V-WPN-US-007、V-WPN-US-009（Bazooka）、V-WPN-GE-009（Panzerschreck）、V-WPN-GE-012、V-WPN-GE-014；对已有线索的许可确认：V-WPN-US-005 的两个 Sketchfab 低模、V-WPN-US-003 的 'Thompson M1A1 Machine Gun'（24.8k）与 'm1-thompson'（23.8k）、V-WPN-US-001 的 'M1 Garand Rifle'（ikonane，约 10.9k）与 'low poly m1 garand'（3.9k）。
- 载具：V-VEH-US-007（Jeep）、V-VEH-US-006（M3 半履带）、V-VEH-US-001/002/005、V-VEH-GE-001/002/006/007、V-VEH-ART-001..004（含 Pak 40 的 CGTrader/Free3D 免费状态确认）、V-VEH-AIR-001..004。
- 建筑与场景：V-BLD-001..005（诺曼底民居）、V-BLD-008/009（桥梁、火车站，调研本次未搜）、V-BLD-007 与 V-PRP-001/002/003 的 Sketchfab 线索（Czech Hedgehog Seaweed、Panzerschokolade 的 Czech Hedgehog、Barbed Wire Fence WWII）、V-PRP-010（destroyed M4A1E9 sherman）、V-NAT-001（bocage）、V-NAT-008（水）。

**音频补搜**（Freesound CC0 过滤，逐表面/逐用途）：E-FTS-001 的草、碎石、雪、石板、木地板、浅水（组合查询结果为 0，需分别搜）；E-ACT-001 的包扎与建造敲打；F-AMB-006 民居房间底噪；C-BUL-002 的木、沙袋、水命中；C-BUL-005 耳鸣高频声；D-TNK-004/005 的专用坦克炮/被击中；B-AIR-001 航空炸弹；J-IR-001/002 的室外/森林 IR（OpenAIR 站点当时 503，需重试）；B 档中 CC-BY-NC / CC-BY-SA 条目。

**未逐站搜索**：Pixabay、Mixkit、Zapsplat、SoundBible、99Sounds、freeSFX 的具体军事音效；Wikimedia Commons 3D、Europeana、The Tank Museum、BlenderKit 登录后内容、textures.com、Fab 免费列表的 WWII 过滤结果（本次摘要只见一件 Cornwall 混凝土碉堡扫描，价格与许可未读）；A Sound Effect / BOOM 免费赠品清单。

### 10.5 调研时未能访问的站点

| 站点 | 状态 | 影响 |
|---|---|---|
| Smithsonian Open Access 3D（3d.si.edu、airandspace.si.edu） | 403 | 目录未能访问；搜索未见任何 WWII 实物扫描（仅一战法军头盔/水壶） |
| OpenGameArt（FAQ 及部分条目页，如 bullet decal、War Theme） | 403 | 相关许可来自搜索摘要，需到原页复核 |
| OpenAIR（约克大学 IR 库） | 503 | 户外/森林 IR 未能核实 |
| Musopen | 403 | 仅搜索摘要 |
| Zapsplat 官方许可页 | 403 | 条款来自搜索摘要 |
| BBC Sound Effects 条款 | 拒绝抓取 | 沿用"仅个人/教育/研究" |
| freeSFX 条款 | 仅见协议标题 | 未核实 |
| Adobe Mixamo 官方 FAQ | 403 | 许可为社区转述 |
| CMU Mocap 官方 FAQ | 503 | 许可为摘要转述 |
| Sketchfab 部分模型页 | WebFetch 返回空 | 仅搜索摘要 |
| Google Fonts 'Saira Stencil One' license 子页 | 404 | OFL 凭记忆 |
| BlenderKit | 301，未再抓取 | 凭记忆，未核实 |
| Fab 若干商品页（如 36 件 PBR 军械包） | 404 / 未打开 | 免费状态未核实 |


---

## 11. 每项最便宜的可用方案（2026-11-17 调研）

> 本节是负责人 2026-11-17 追加调研（"每项最便宜的可用商品"）的结果，**优先于 §9 中与之冲突的旧推荐**（冲突处已在 §9 原位加注）。§1–§10 的章节编号与需求 ID 均未改动。
> **全部商品名、作者/商店、URL、价格、面数、证据标签均原样取自三份调研片段（角色/武器/动画、载具/环境、音频），未新增或编造。**
> 证据标签：**[F]** = 商品页直接读到；**[S]** = 仅搜索摘要（CGTrader 页面多数抓不到正文，只能 [S]，下单前须人工核对）。价格为美元、不含税（€ 者另注），促销价会变，**以结算页为准**。
> **所有候选均未下载检验**（未登录、未购买、未下载），面数、贴图、部件独立性、许可均需购买前按 §11.10 逐件核对。
> "达标"以 §11.1 规格为准。**现实结论：市面上"单价便宜"的枪几乎都在 0.4–2 万三角形；单价便宜的车辆多是 2k–12k 面的低模，对 5 万–15 万面的规格是"不达标"。下表凡不足均如实标"部分达标"或"不达标"，不隐瞒。**
> 仅 Unreal / 仅 Unity 的商品，格式栏明确标出"需在该引擎导出"：本项目是 Babylon.js，须先在对应引擎里导出 FBX/glTF，并确认商店 EULA 允许导出再分发（未读到条款，需人工确认）。
> 每行末尾的 `[ ] 决定购买` 由负责人勾选；与 §9 一致，付费项一律是备选。

### 11.1 合格规格表（负责人给定）

| 类别 | 面数（三角形） | 贴图 / 绑定 / 部件要求 |
|---|---|---|
| 第一人称武器 | 2.5 万–6 万 | 4K 贴图；枪栓、弹匣、扳机、表尺独立 |
| 第三人称武器 | 5 千–1.2 万 | 由第一人称武器减面得到，**不单买** |
| 士兵角色 | 3 万–6 万 | 人形骨骼，含手指；4K |
| 第一人称手臂 | 8 千–1.5 万 | 带手指骨骼与军装袖 |
| 坦克和车辆 | 5 万–15 万 | 炮塔、炮管、履带、负重轮、舱盖独立 |
| 建筑 | 每栋 5 千–5 万 | 模块化 |
| 道具和植被 | 几百–1 万 | PBR |
| 文件格式 | fbx / glb / gltf / blend | — |

### 11.2 武器（第一人称精度）

> **Loss 的 RenderHub 德军武器系列（G43 / MG34 / MG42 / MP40 / FG42 与 $70 bundle）不再推荐。** 面数仅 4.4k–12k（G43 4,368；MP40 4,470；MG42 7,288；MG34 12,149；FG42 7.8k），均不达第一人称标准；许可为 "Extended Use License (IP Restricted)"，同站另一枪页（blackraven37 Kar98，[F]）的 IP Restricted 条款明写 "subject to an Editorial Use Only Restriction"，Loss 的同名许可很可能同样带该限制，**按"Editorial 排除"处理，不列为候选**。§4 与 §9 中引用 Loss 的项（G43 $15、MG34 $20、MG42 $20、MP40 $15、$70 bundle）已作废，需另核实后才可恢复。
> 同样已排除：CGTrader 'Military WW2 Soldier Game Ready'（$20，含 Garand）页标 editorial；TurboSquid 'WW2 US Soldier' $59.99 为 Editorial Uses Only。
> Fab 各页"许可"字段几乎都读不到（显示 Select a license），下表统一写"未读到"。

| 需要什么（需求 ID） | 最便宜推荐（商品 / 作者·商店 / URL） | 价格与证据 | 格式 | 面数 / 关键规格 | 达标情况 | 备选（简写） | 决定 |
|---|---|---|---|---|---|---|---|
| M1 Garand（V-WPN-US-001） | 'M1 Garand With Complete Internals - Game Ready'｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/m1-garand-with-complete-internals-game-ready | $8 [S] | FBX [S] | 约 30.8k 多边形；扳机、弹夹为独立件（含内部结构）；标 rigged/animated/PBR；贴图分辨率与枪栓是否独立未读到；许可未读到 | 部分达标：4K、枪栓待核（面数达标，全场最便宜） | ② 'PBR M1 Garand and Attachments'｜CGTrader｜https://cgtrader.com/3d-models/military/gun/m1-garand-and-attachments｜$49 [S]（14 个独立对象，Maya/OBJ/FBX）；③ 'M1 Garand With FPS Animations'｜Fab / Agora Games Brazil｜https://www.fab.com/listings/84f12c05-9209-45be-a4db-89e7c3bb8614｜$45.99 [F]（4K；FP 武器+手臂动画；**仅 Unreal（UE5），需在 UE 导出**；面数未读到） | [ ] 决定购买 |
| M1 Carbine（V-WPN-US-002） | 'M1 Carbine Rifle'｜Fab / Max3d｜https://fab.com/listings/a8dc03f9-eab1-4e36-bf80-2e9b23ef3c02 | $23.99 [F] | FBX、glTF、GLB、USDZ、OBJ、MAX | 17,894 三角；4096² PBR；"separate objects for animation"；未绑定；许可未读到 | 部分达标：面数 1.8 万 < 2.5 万 | 无 | [ ] 决定购买 |
| M1A1 Thompson（V-WPN-US-003） | '[3D] Thompson SMG'｜itch.io / sundio｜https://sundio.itch.io/3d-thompson-smg | $5.50 起 [F] | glb、blend（glTF 可直接用） | 10,043 三角；4K UDIM PBR；"Simple rig covers almost every moving part"；自带简单开火动画；可商用、禁转售、署名可选 [F] | 部分达标：面数 1 万 | ② Neapolitan Studios 'M1A1 Thompson Sub-Machine Gun'｜Gumroad｜https://neapolitanstudios.gumroad.com/l/OTsat｜约 $25 [S]（页面正文读不到，需人工确认）；③ 'Fps weapon m1A1 ww2 Fps - Tps'｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/fps-weapon-m1a1-ww2-fps-tps｜$50 [S]（7,175 多边形，带骨骼 + FPS/TPS 手臂动画） | [ ] 决定购买 |
| M3 Grease Gun（V-WPN-US-004） | Neapolitan 'M3 Grease Gun Sub-Machine Gun'｜Gumroad / Neapolitan Studios｜https://neapolitanstudios.gumroad.com/l/nuInc | 约 $25 [S]（Gumroad 页读不到价格） | FBX（+TGA）；同作者另有 Unity 版 | 摘要：约 16.4k 三角、4K PBR、可动枪机/拉机柄/枪托/曲柄，30 发弹匣+子弹；枢轴已设；许可未读到 | 部分达标：面数 1.6 万 | Unity 'M3 Grease Gun'｜Zeus Game Assets｜https://marketplace.unity.com/packages/3d/props/guns/m3-grease-gun-201933｜$30 [F]（**仅 Unity 包，需在 Unity 导出**；仅 Built-in 管线；页面无面数/贴图；数据不足） | [ ] 决定购买 |
| M1918A2 BAR（V-WPN-US-005） | 'LowPoly Realistic GameReady M1918 BAR'｜Fab / Raouf Benoudina｜https://www.fab.com/listings/dd07cad5-eae8-4287-99f6-eb6594251e7c | $4.99 [F] | FBX / OBJ / Blend | 约 4k 顶点；4K 贴图；分部件；许可未读到 | **不达标（低模）**，仅可作占位 | CGTrader 'M1918 BAR gun'：摘要称 4096² + 已绑定 blend，价格/面数未读到 | [ ] 决定购买 |
| M1919A6（V-WPN-US-006） | RenderHub eight-vertex 'M1919 Browning Gun All PBR Unity UE'｜https://www.renderhub.com/eight-vertex/m1919-browning-gun-all-pbr-unity-ue-textures-included | 价格需人工确认 | 未读到 | 未读到 | 线索，未核实 | 无 | [ ] 决定购买 |
| M1903A4 带镜（V-WPN-US-007）、M1911A1（V-WPN-US-008） | 'FPS World War Guns Pack – US'｜Fab｜https://www.fab.com/listings/00a8235f-055a-4532-9916-fe060949b9eb（URL 同 §9.1） | $19.99 [F] | FBX / OBJ / Blend（§9.4 原记） | M1903 3k 顶点、M1911 2.5k（低模）；4K 贴图；许可未读到 | **不达标**（仅第三人称精度，不满足第一人称硬性要求）；是否带镜待核 | 未找到达到第一人称标准的商品 | [ ] 决定购买 |
| M1A1 Bazooka（V-WPN-US-009） | 'M1A1 Bazooka Anti-tank Rocket Launcher'｜Fab / Max3d｜https://fab.com/listings/47052cd4-80e6-4f4f-9712-726bf415b403 | $23.99 [F] | FBX、glTF、GLB、USDZ、OBJ、blend | 10,542 三角；炮 4096²、火箭 2048²；无绑定；许可未读到 | 部分达标：面数 1 万；火箭 2K | 无 | [ ] 决定购买 |
| Mk2 手雷（V-WPN-US-010） | 'mk2 grenade gameready'｜CGTrader｜https://www.cgtrader.com/3d-models/military/other/mk2-grenade-gameready | $2 [S] | 未读到 | 低模；Substance 贴图 | 小件，三角形下限不适用；贴图分辨率待核 | 无 | [ ] 决定购买 |
| M1/M3 刺刀（V-WPN-US-012）；Tellermine（V-WPN-GE-013）；S-Mine（V-WPN-GE-014） | 本次未搜到任何商品 | — | — | — | 找不到（见 §11.8） | — | — |
| Kar98k（V-WPN-GE-001）；带镜 Kar98k（V-WPN-GE-002） | 'Kar98k 5 lods'｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/kar98k-5-lods | $7 [S] | 未读到 | "Fully Rigged and Textured (5 variants)"，4K；**自带 5 级 LOD**（可顺带解决第三人称）；面数未读到；许可未读到 | 部分达标：面数/枪栓独立待核；带镜版可能在 5 个变体里，待核 | ② 'Kar98 Sniper Rifle – Realistic'｜Superhive（Blender Market）｜https://superhivemarket.com/products/kar98-sniper-rifle--realistic-weapon-3d-model｜$35.99 [F]（标 Rigged、"Detailed bolt-action mechanism"、Low/High；Blender/Maya/Unity/UE；Royalty Free；面数/贴图分辨率页面未给）；③ 'Mauser Karabiner Kar98k'｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/mauser-karabiner-kar98k｜价格需人工确认（高模无 UV + 4K PBR，需自行减面；Royalty Free (no AI) [S]） | [ ] 决定购买 |
| Gewehr 43（V-WPN-GE-003） | 未找到达第一人称标准的商品 | — | — | Loss G43（RenderHub $15，4,368 多边形，IP Restricted）**不再推荐** | 不达标 + 许可风险 | — | — |
| MP40（V-WPN-GE-004） | 'Animated FPS MP40 Pack'｜CGTrader / Ironbelly Studios｜https://cgtrader.com/3d-models/military/gun/animated-fps-mp40-pack | $24.99 [S] | 未读到 | 4K、3 级 LOD、各部件独立网格、**10 个第一人称动画 + 附赠带绑定的 FP 手臂**；面数未读到 | 部分达标：**许可待确认**（摘要写 "editorial license (no AI)"，**若为 Editorial 则排除**）；若许可合格，是唯一带 FP 手臂的 WW2 包 | 无（Loss MP40 不再推荐） | [ ] 决定购买 |
| StG 44（V-WPN-GE-005） | 'sturmgewehr 44 stg44'｜CGTrader｜https://cgtrader.com/3d-models/military/gun/sturmgewehr-44-stg44 | $55 [S] | 未读到 | 约 46k 多边形；4K PBR；已绑定；买家称导入后贴图"偏平" | **达标（面数/4K）**，待核部件独立与贴图问题 | Unity 'MP44 StG44 PBR'（12k 三角）已下架，排除 | [ ] 决定购买 |
| MG34（V-WPN-GE-006） | 未找到达第一人称标准的商品 | — | — | Loss MG34（12,149 面，许可存疑）**不再推荐** | 找不到（见 §11.8） | — | — |
| MG42（V-WPN-GE-007） | 'WW2 MG42'｜Fab / ToLoss｜https://www.fab.com/listings/1935b32b-9a62-476c-a142-ea50218661be | $9.99 [F] | Blender、FBX、GLB | 约 1.6 万三角（含弹鼓+弹药）；**仅 2K**；完全绑定，弹链样条网格；许可未读到 | 部分达标：面数/贴图不足；便宜且含弹链 | 达标版：'MG42 Maschinengewehr 42'｜CGTrader｜https://cgtrader.com/3d-models/military/gun/mg42-maschinengewehr-42｜$60 [S]（约 27,550 多边形；4K PBR；已绑定（UE 测试）；FBX；待核枪机等独立部件） | [ ] 决定购买 |
| Luger P08 / P38（V-WPN-GE-008） | 'Luger P08 Realistic'｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/luger-p08-realistic-3d-model | $17.95 [S] | 未读到 | 约 16.6k 多边形；4K PBR；标 rigged；为 FPS 设计 | 部分达标：面数 1.7 万 | Fab 'Luger P08 - World War'｜Rush Games｜https://www.fab.com/listings/bd3d5a66-7190-4ece-ba28-8bb788f84aa9｜$14.99 [F]（含骨骼网格；**仅 Unreal，需在 UE 导出**；面数未读到，数据不足）；P38：'Walther P38'｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/walther-p38-542626e9-0eee-487c-ae6d-d098be999a25｜$25 [S]（约 12k 三角；4K；rigged；部分达标） | [ ] 决定购买 |
| Panzerschreck（V-WPN-GE-009） | Panzerschreck 单品｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/panzerschreck-german-anti-tank-rocket-launcher（片段另有含它的 CGTrader 合集） | 价格需人工确认 | 未读到 | 6.2k 面，低模 | **不达标（低模）** | 无 | [ ] 决定购买 |
| Panzerfaust（V-WPN-GE-010） | 'Panzerfaust With Box'｜CGTrader｜https://www.cgtrader.com/3d-models/military/rocketry/panzerfaust-with-box | $20 [S] | 未读到 | 20,287 多边形，4K PBR，未绑定；弹头独立 | 部分达标：面数 2 万；绑定另一件 | Panzerfaust 60（rigged 弹头，4K）｜CGTrader｜https://cgtrader.com/3d-models/military/rocketry/panzerfaust-60｜价格需人工确认 | [ ] 决定购买 |
| M24 手雷（V-WPN-GE-011） | 'German Stick Grenade M24 4K PBR'｜Fab / Rodrigo Maranzana｜https://fab.com/listings/e198ce25-fcb5-4b6c-a699-1f9b5083116a | $2.99 [F] | FBX、glTF、GLB、USDZ | 低模；4K PBR 16-bit；免费商用（不得转售）[F] | 小件，下限不适用；**达标（小件）** | 无 | [ ] 决定购买 |

**整包方案（单价低但面数低，仅供参考，不入推荐）**

| 整包 | 价格与证据 | 内容 / 规格 | 判定 |
|---|---|---|---|
| 'WW2 Polygon Pack'｜CGTrader｜https://www.cgtrader.com/3d-model-packs/ww2-polygon-pack | $50 [F] | MP40(3.5k)、Luger、Thompson/M1919、Kar98(1.1k)、M1 Carbine、Colt、M1A1 Bazooka(0.5k)、刀；FBX 全部 + glTF 部分 + blend；Royalty Free；同时含 8 个角色（见 §11.3） | 低多边形风格，面数远低于第一人称标准，只能当第三人称/占位 |
| 'FPS World War Guns Pack' 美 / 德各一个｜Fab | 各 $19.99 [F] | 1–5.7k 顶点，4K 贴图，FBX/OBJ/Blend | **仅第三人称精度，不满足第一人称硬性要求** |
| 'Ultimate Realistic World War Game Ready FPS Guns Pack'｜Fab | $54.99 [F] | 16 枪（苏/美/德/近战），Blender/OBJ/FBX；页面未列具体型号与面数 | 需人工确认 |
| 'WW2 US Weapons Pack'｜Fab / OnurErdem | $44.99 [F] | 5 把枪，全带动画，3 档贴图，**仅 Unreal（需在 UE 导出）**；型号页面未列出 | 需人工确认 |
| 'FPS World War Pack'｜Fab / Deadghost | $19.99 [F] | 4K，含动画与手臂，**仅 Unreal（需在 UE 导出）**，UE Marketplace License；型号未列 | 需人工确认 |

### 11.3 角色与第一人称手臂

| 需要什么（需求 ID） | 最便宜推荐 | 价格与证据 | 格式 | 面数 / 关键规格 | 达标情况 | 备选（简写） | 决定 |
|---|---|---|---|---|---|---|---|
| 美军步兵（V-CHR-US-001） | 'US WW2 GI Soldier'｜itch.io / owsam｜https://owsam.itch.io/usww2gisoldier | €9.99 起 [F] | FBX | 3,294 多边形，已绑定，含 base color/法线/AO 贴图（无手指信息）；可用，不得转售；PDF 协议 | **不达标（面数 3k）**；便宜，可作 bot/远景 | ② 'WWII American Soldier'｜GameDev Market｜https://gamedevmarket.net/asset/wwii-american-soldier-8759｜$19 [S]（约 6k 面，3ds Max biped，2017 年，MAX/OBJ/3DS/FBX；不达标）；③ 'WW2 US Soldier'｜RenderHub / Alpenwolf｜https://www.renderhub.com/alpenwolf/ww2-us-soldier｜页面 404，§9 原记约 46.3k 多边形、PBR、rigged，价格需人工确认；Extended Use（是否 IP Restricted 需核），**若许可无 Editorial 限制则达标** | [ ] 决定购买 |
| 美军空降兵（V-CHR-US-002） | 'SOLDIER WWII USA Paratrooper Rigged'｜CGTrader/RenderHub / albin｜https://www.cgtrader.com/3d-models/military/military-character/soldier-wwii-usa-paratrooper | $60 [S，原清单] | 未读到 | 已绑定；面数未核 | 待核 | 无 | [ ] 决定购买 |
| 美军兵种变体：军官/医护/坦克兵/狙击手（V-CHR-US-003）；德军坦克兵/军官（V-CHR-GE-004） | 'WW2 Polygon Pack'｜CGTrader｜https://www.cgtrader.com/3d-model-packs/ww2-polygon-pack | $50 [F] | FBX 14/14，glTF 10/14，blend | 含 US Rifleman/Medic/Pilot/Commander 与 DE Tank Crew/Leader/Sniper/Rifleman；风格化低模，面数未给；Mixamo 兼容的步枪/刀/机枪/驾驶动作；Royalty Free | **不达标（不达 3 万面标准）**；兵种变体最便宜 | 无 | [ ] 决定购买 |
| 德军国防军步兵（V-CHR-GE-001） | 'German Soldier Rig – WW2 Scanned Asset Pack'｜CGTrader｜https://www.cgtrader.com/3d-models/character/clothing/german-soldier-rig-ww2-scanned-asset-pack | $40 [S] | FBX | 扫描重拓扑，人形绑定含扭转骨，4K PBR，头盔独立网格可隐藏；Royalty Free [S]；面数待核 | **最接近达标**（面数待核） | 'WW2 German Soldier Rigged Animated'｜CGTrader｜https://www.cgtrader.com/3d-models/character/clothing/ww2-german-soldier-rigged-animated｜$35 [S]（低模版已绑定 + 14 个动画，高模版，头盔、MP40，4K；部分达标，待核面数） | [ ] 决定购买 |
| 德军伞兵（V-CHR-GE-002）、德军冬季（V-CHR-GE-003）、美军冬季（V-CHR-US-004） | 本次未找到现成商品 | — | — | Fallschirmjäger 仅找到 RenderHub FG42 枪（面数 7.8k）与 Daz 用 Genesis 8 人偶；冬季服无 | 找不到（见 §11.8） | — | — |
| 第一人称手臂（V-CHR-FPA-001） | 'Animated FPS MP40 Pack' 附赠带绑定 FP 手臂｜CGTrader / Ironbelly Studios｜https://cgtrader.com/3d-models/military/gun/animated-fps-mp40-pack | $24.99 [S]（同 §11.2 MP40） | 未读到 | 附赠 FP 手臂；面数未读到 | 部分达标：**许可待确认（Editorial 则排除）**；未找到现成"WW2 袖子"商品 | ② 'FPS Arms Pack 01B / FPS Arms 02'｜RenderHub / Davlet｜https://www.renderhub.com/davlet/fps-arms-02｜价格需人工确认（通用手臂，含前臂扭转骨，非 WW2 袖口，需重贴图）；③ 'FPS First Person Arms - 01 B'｜CGTrader｜https://cgtrader.com/3d-models/character/man/fps-first-person-arms-01-b｜价格需人工确认（含军装类贴图变体 [S]）；④ 'FPS World Wars Pack'｜Fab / Deadghost｜$19.99 [F]（含手臂+4K+动画，**仅 Unreal，需在 UE 导出**） | [ ] 决定购买 |

### 11.4 动画

| 需要什么（需求 ID） | 最便宜推荐 | 价格与证据 | 格式 | 关键规格 | 达标情况 | 备选（简写） | 决定 |
|---|---|---|---|---|---|---|---|
| 第三人称步枪运动/射击（V-ANM-001） | 'Rifle Starter'｜itch.io / MoCap Online｜https://mocaponline.itch.io/rifle-starter-anims | $3.99 [F] | FBX/BIP；UE/Unity/Blender | 11+ 个：瞄准、转身、站/蹲射击、走/慢跑/蹲走、跳；个人项目不限，**商业仅 1 个项目，再商用需再买** | 部分达标：缺匍匐、Bazooka | 无 | [ ] 决定购买 |
| 蹲/匍匐/火箭筒（V-ANM-002 等） | 'Human Soldier Animations'｜itch.io / kevdev｜https://kevdev.itch.io/human-soldier-animations | $15（半价，原 $30）[F] | Blender / Unity / Godot-Unreal | 538 个动画；步枪/突击/手枪/Bazooka；蹲走 8 向、匍匐；未提栓动 | 部分达标：缺栓动 | 无 | [ ] 决定购买 |
| 第一人称武器动画：栓动/换弹（V-ANM-009） | 'Animated WW2 Weapons #2'｜Unity Asset Store｜https://marketplace.unity.com/packages/3d/props/weapons/animated-ww2-weapons-2-102602 | $12 [F] | 仅 Unity 包，**需在 Unity 导出**；标准 Unity EULA | 页面未列武器；摘要称含拉栓/换弹 FBX | 数据不足，需核 | Fab 'M1 Garand With FPS Animations' $45.99 [F]（仅 UE，需在 UE 导出）；Fab 'FPS Animation Pack Lite'｜KINEMATION｜https://www.fab.com/listings/79752ba2-c659-4375-8fa9-5e6fc8025b62｜$24.99 [F]（含 Kar98k 装填循环；仅 UE，需在 UE 导出） | [ ] 决定购买 |
| 医护救治/拖拽（V-ANM-006）、载具进出/驾驶/炮手（V-ANM-008） | 本次未找到现成商品 | — | — | — | 找不到（见 §11.8） | — | — |

### 11.5 载具 / 火炮 / 飞机

> 负责人规格（5 万–15 万面、部件独立）与"最便宜"天然冲突：页面能看到的便宜车辆多是 2k–12k 面低模。能同时在页面上读到"5 万面以上 + 独立部件"的，只有 Fab 上 EVOLVE Studio 的 Pz.IV / Pz.V，且**只有 Unreal 格式（需在 UE 导出）**；M8 Greyhound（85k）面数达标但价格未读到。
> JonahH 的两个 Unity 大包（各 $49.99）面数 2.6k–12.1k、部件独立，同样低于 5 万下限；Unity EULA（Extension Asset，Single Entity），导出模型放进浏览器游戏分发是否允许，**未读到条款，需人工确认**。
> 'World War Toon Tank Pack'（卡通风）、已下架的 'Low Poly WWII Vehicles'、标 Editorial 的 'Panzer II Ausf L Lynx'（CGTrader）与 TurboSquid 'Panzer II Luchs'（$25）、标 AI 辅助的 'WWII field howitzer'、3D 打印用 STL 均已排除。

| 需要什么（需求 ID） | 最便宜推荐 | 价格与证据 | 格式 | 面数 / 关键规格 | 达标情况 | 备选（简写） | 决定 |
|---|---|---|---|---|---|---|---|
| M4 Sherman 75（V-VEH-US-003） | 'M4 Sherman'｜CGTrader｜https://cgtrader.com/3d-models/military/armor/m4-sherman-ddd981af-e9d7-41d3-a94e-90327e0ffff7 | $9 [S] | FBX（2 文件，28.2 MB）[S]；2048 PBR | 低模，三角数未读到；许可未读到 | 部分达标：有 PBR 与 FBX，面数/部件未知 | Fab 'M4 Sherman-US WW2 Medium Tank'｜Erich Von Manstein｜https://www.fab.com/listings/1cbcdcea-479c-40c2-b93b-2e266ec55406｜$20.99 [F]（Blender/FBX/glTF/GLB/USDZ；有炮管、悬挂、负重轮、履带；带绑定标签；面数未列；无内饰）；JonahH 'WW2 United States Armed Forces'｜Unity｜https://assetstore.unity.com/packages/3d/vehicles/land/ww2-united-states-armed-forces-176742｜$49.99 [F] | [ ] 决定购买 |
| M4A3(76)W / Jumbo（V-VEH-US-004） | Sherman M4A3(76)W HVSS（Dezryelle）｜Sketchfab｜https://sketchfab.com/3d-models/low-poly-m4a376-hvss-710e93ef16c84160a5e8b980ce07163a | 免费下载 [S] | 页面未读 | 34.6k 面 [S]；**无贴图、未绑定**；CC BY 4.0（M4A3(75)W 页 [F]；76 页未逐条读） | 不达标（无贴图、面数不足）；免费且合规，需自己上贴图 | 含在 JonahH 美军包（$49.99 [F]）内（M4A3(76)） | [ ] 决定购买 |
| M5A1 Stuart（V-VEH-US-002） | 'WW2 US Low Poly Tanks #3'｜Unity / Kado3D｜https://assetstore.unity.com/packages/3d/vehicles/land/wwii-low-poly-us-tanks-pack-3-107371 | $9.90 [F] | 仅 Unity 包，**需在 Unity 导出**；Unity EULA，Extension Asset | M3 Lee 3,257 / **M3A1 Stuart 2,966** / M10 / M12 / WC51（Sketchfab 页 [F]）；注意是 M3A1 不是 M5A1 | 不达标（面数） | JonahH 美军包 $49.99 [F]（含 M3A1/M3A3/M5A1 Stuart） | [ ] 决定购买 |
| M8 Greyhound（V-VEH-US-001） | 'Low Poly M8 Greyhound armored car'｜CGTrader｜https://cgtrader.com/3d-models/vehicle/military-vehicle/low-poly-m8-greyhound-armored-car | $12–$15 [S]（两个版本价格不同） | FBX（Kado3D 系，假定） | 低模；Royalty Free [S] | 不达标（面数） | 'M8 Greyhound game ready'｜CGTrader｜https://www.cgtrader.com/3d-models/vehicle/military-vehicle/m8-greyhound-game-ready｜价格需人工确认（85,000 三角，另附高模用于烘焙；Royalty Free [S]；**面数达标**，部件独立性未知） | [ ] 决定购买 |
| M3 半履带（V-VEH-US-006） | 'Low Poly M3A1 Half-track Personnel Carrier'｜CGTrader / Kado3D 疑似｜https://cgtrader.com/3d-models/military/vehicle/low-poly-m3-half-track-personal-carrier | $12 [S] | FBX、Maya、Collada、OBJ；PBR；有绑定 | 约 11.9k 面；Royalty Free | 不达标（面数） | 含在 JonahH 美军包（M3A1 半履带）内 | [ ] 决定购买 |
| Willys Jeep（V-VEH-US-007） | Willys MB 低模（CGTrader 商品页 slug：ww2-willys-jeep-vehicle-game-ready-low-poly）｜CGTrader｜https://cgtrader.com/3d-models/military/vehicle/ww2-willys-jeep-vehicle-game-ready-low-poly | $5 [S] | FBX（1.95 MB），PBR，三种贴图变体 | 25,473 面 [S]；Royalty Free [S] | 不达标（面数），但已是同类最便宜 | 免费 'Jeep Willys'｜CGTrader｜https://cgtrader.com/3d-models/military/military-vehicle/jeep-willys-4e91d899-32b6-4e0d-a5cd-760371d4a995｜免费 [S]（OBJ、FBX、GLB；车轮独立于车身；面数未读到；部分达标）；Unity Kado3D US Tanks #2 内含 Willys MB 2,202 三角 | [ ] 决定购买 |
| GMC CCKW（V-VEH-US-005） | 'WW2 US Low Poly Tanks #2'（"WWII Low Poly US Tanks Pack 2"）｜Unity / Kado3D｜https://assetstore.unity.com/packages/3d/vehicles/land/wwii-low-poly-us-tanks-pack-2-95689 | $7.90 [F] | 仅 Unity 包，**需在 Unity 导出**（PSD 贴图）；Unity EULA，Extension Asset | **CCKW 353 卡车 2,501 三角**（来自 Sketchfab 同名包页 [F]，Unity 页未列内容，假定同内容，需确认）；另含 M4A4 Firefly 2,317 / M24 / M26 / Willys MB 2,202 | 不达标（面数远低于 5 万）；史实型号吻合，价格最低 | 无（其他商店读不到 CCKW 低价合规件） | [ ] 决定购买 |
| Panzer IV（V-VEH-GE-003） | 'Low Poly Panzer IV G / Tiger Ausf E'（Kado3D）｜CGTrader｜https://cgtrader.com/3d-models/military/vehicle/low-poly-panzer-iv-g-medium-tank | $12 [S] | 摘要写有 FBX | 5,003 面 | 不达标（面数） | **面数达标版：'Tank Drivable Pz.IV'｜Fab / EVOLVE Studio｜https://www.fab.com/listings/1228a974-3491-4ec8-a426-dcf9be02bb14｜$59.99 [F]｜69.8K 三角；2 个骨骼网格 + 3 个静态网格；5 个材质（4 张 4K）；无内饰；注明 AI 使用不允许；格式：仅 Unreal Engine，需在 UE 导出；部分达标（部件独立性页面未逐项命名）**；JonahH 德军包 $49.99 [F] | [ ] 决定购买 |
| Panther（V-VEH-GE-004） | 'WWII Low Poly German Tanks #2'（含 Panther D）｜Unity / Kado3D｜https://assetstore.unity.com/packages/3d/vehicles/land/wwii-low-poly-german-tanks-2-94574 | $7.90 [F] | 仅 Unity 包，**需在 Unity 导出**；Unity EULA | Panther D、Puma 2,143 三角、Sd.Kfz.251 Hanomag 等 5 辆（搜索摘要 [S]；Unity 页未列内容）；约 2k 面级 | 不达标（面数） | **面数达标版：'Tank Drivable Pz.V'｜Fab / EVOLVE Studio｜https://www.fab.com/listings/99add306-d186-4fe7-a797-082ed5403eda｜$59.99 [F]｜76.5K 三角；结构同 Pz.IV；无内饰；格式：仅 Unreal Engine，需在 UE 导出；部分达标**；CGTrader 'Panther Ausf A' $12 [S]（Low-poly，需人工确认）；Panther G（texture pack GameReady）$15 [S]（旧清单值，需确认面数） | [ ] 决定购买 |
| Tiger I（V-VEH-GE-005） | 'Low Poly Panzer IV G / Tiger Ausf E'（Kado3D）｜CGTrader｜https://cgtrader.com/3d-models/military/vehicle/low-poly-panzer-iv-g-medium-tank（片段合并列出 Pz.IV G / Tiger E，各 $12） | $12 [S] | 摘要写有 FBX | 5k–11k 面（片段对该合并条目的范围） | 不达标（面数） | Fab 'Tiger I'｜TANK｜https://www.fab.com/listings/8a43cbb5-fb9a-485d-aa01-90f411867c0e｜$14.99 [F]（OBJ、glTF、GLB、USDZ，无 FBX、无 blend；三角数、部件、内饰页面均未写；无法判定）；JonahH 德军包 $49.99 [F]（含 Tiger E / (P) / II） | [ ] 决定购买 |
| Panzer II Luchs（V-VEH-GE-002） | 无单买达标商品 | — | — | 其他商店读不到合规的 Luchs；Fab 'Tank Drivable PzII.G'｜EVOLVE Studio｜https://www.fab.com/listings/a5f7ded1-28a3-42c1-9ab4-fa87730ab933｜$49.99 [F] 是 Pz.II Ausf.G **不是 Luchs**（35.5K 三角；Unreal、Blender、FBX、OBJ、GLB、glTF、USDZ）；CGTrader / TurboSquid 的 Luchs 为 Editorial，排除 | 找不到合规单品；近似替代见 §11.8 | JonahH 德军包 $49.99 [F]（含 Luchs；Unity 包，需导出） | [ ] 决定购买 |
| Sd.Kfz. 234 Puma（V-VEH-GE-001） | 'WWII Low Poly German Tanks #2'（含 Puma）｜Unity / Kado3D｜https://assetstore.unity.com/packages/3d/vehicles/land/wwii-low-poly-german-tanks-2-94574 | $7.90 [F] | 仅 Unity 包，**需在 Unity 导出** | Puma 2,143 三角 [S] | 不达标（面数） | 'Low Poly Sd.Kfz.234/2 Puma'｜CGTrader｜https://cgtrader.com/3d-models/military/vehicle/low-poly-sdkfz-234-puma-armored-car｜$12 [S]（6,754 面，FBX/OBJ/Maya，PBR）；'Sd Kfz 234 Puma Low-poly PBR'｜Fab / Max3d｜https://fab.com/listings/1e3114e9-acba-40fa-8a13-a673f20b2a6b｜$28.99 [F]（FBX/glTF/GLB/USDZ；4K PBR；三角数未列；部分达标） | [ ] 决定购买 |
| Sd.Kfz. 251（V-VEH-GE-007） | 'WWII Low Poly German Tanks #2'（含 Sd.Kfz.251 Hanomag）｜Unity / Kado3D | $7.90 [F] | 仅 Unity 包，**需在 Unity 导出** | 约 2k 面级（搜索摘要 [S]） | 不达标（面数） | RenderHub max3ddd 'Sd.Kfz. 251 German Armored Fighting Vehicle'｜https://www.renderhub.com/max3ddd/sd-kfz-251-german-armored-fighting-vehicle｜价格需人工确认（约 41.5k 三角，game ready；接近达标）；CGTrader 版 $29 [S]（FBX + PBR；Royalty Free No AI） | [ ] 决定购买 |
| Opel Blitz（V-VEH-GE-006） | 'Opel Blitz 1940'｜CGTrader｜https://cgtrader.com/3d-models/military/vehicle/opel-blitz-b06e3dbf-f574-4678-8c60-6e044d9028f6 | $10 [S] | 格式未读到 | 低模；摘要写 VR/AR 通过、有绑定；Royalty Free | 不达标（面数） | 'Opel Blitz WWII - Rigged with interior'｜CGTrader｜https://www.cgtrader.com/3d-models/vehicle/truck/opel-blitz-wwii-rigged-with-interior｜$29.99 [S]（FBX；3 种涂装；有内饰；摘要写约 800 多边形，可疑需核；部分达标） | [ ] 决定购买 |
| M2A1 105mm 榴弹炮（V-VEH-ART-001） | 含在 JonahH 'WW2 United States Armed Forces'（105mm M2A1、M3 105mm、75mm M1A1、155mm M1）｜Unity｜https://assetstore.unity.com/packages/3d/vehicles/land/ww2-united-states-armed-forces-176742 | $49.99 [F]（整包） | 仅 Unity 包，**需在 Unity 导出** | 火炮 0.9k–2.4k 三角 [论坛 v1.0 数据] | 不达标（面数） | CGTrader 'M2A1 howitzer'｜https://www.cgtrader.com/3d-models/military/vehicle/m2a1-howitzer｜价格需人工确认（摘要写 Maya + FBX，4K 贴图；M101A1 页 LOD1 约 44k 面、2048 PBR；另一个 43,900 面低模 PBR + FBX；面数接近下限） | [ ] 决定购买 |
| leFH 18（V-VEH-ART-002） | 含在 JonahH 'WW2 German Armed Forces'（10.5cm leFH 18）｜Unity｜https://assetstore.unity.com/packages/3d/vehicles/land/ww2-german-armed-forces-165215 | $49.99 [F]（整包） | 仅 Unity 包，**需在 Unity 导出** | 火炮 0.9k–2.4k 三角（美军包 v1.0 数据；德军包面数未单独读到） | 不达标（面数） | Hum3D '10.5 cm leFH 18'｜https://hum3d.com/3d-models/10-5-cm-lefh-18/｜$150 [S，旧清单]；约 70.8 万面需减面；贵，不推荐 | [ ] 决定购买 |
| 57mm M1 反坦克炮（V-VEH-ART-003） | 未找到 M1 57mm | — | — | 替代：'low poly ordnance qf 6 pounder'｜CGTrader｜https://www.cgtrader.com/3d-models/military/gun/low-poly-ordnance-qf-6-pounder｜价格需人工确认；Royalty Free（6-pdr 是允许的替代） | 找不到（见 §11.8） | — | — |
| Pak 40（V-VEH-ART-004） | 'Low Poly 7.5 cm Pak 40'｜CGTrader｜https://cgtrader.com/3d-models/military/gun/low-poly-7-5-cm-panzerabwehrkanone-40-anti-tank-gun | $12 [S]（另一处摘要写 $20，**价格冲突需人工确认**） | 摘要写含 FBX | 面数未读到 | 需人工确认 | 'Pak 40 - Rigged'｜RenderHub / Loss｜https://www.renderhub.com/loss/ww2-pak-40-rigged-panzerabwehrkanone｜价格需人工确认（FBX、Blend；4K PBR；下载 885.7 MB；§9 原记约 16k 面；Extended Use（IP Restricted）；面数不达标，有绑定）；含在 JonahH 德军包内 | [ ] 决定购买 |
| P-47（V-VEH-AIR-001）；仅过场，低精度可 | 'P-47 Thunderbolt'｜TurboSquid｜https://www.turbosquid.com/de/3d-models/p-47-1038354 | $9 [S] | 含 FBX（是否 game-ready、许可是否允许游戏用均未确认；TurboSquid 标准许可） | — | 低精度可接受；许可需核 | CGTrader 低模 PBR（约 1.2 万面）｜https://cgtrader.com/3d-models/aircraft/military-aircraft/united-states-fighter-aircraft-republic-p-47-thunderbolt｜$139 [S]（Royalty Free，FBX；旧清单值） | [ ] 决定购买 |
| Ju 87（V-VEH-AIR-002） | Ju 87 低模｜CGTrader｜https://www.cgtrader.com/3d-models/aircraft/historic-aircraft/ww2-german-dive-bomber-ju87-stuka | $15–$18 [S] | FBX | 低模单贴图含驾驶舱；另有 $4.99 手绘风版本，风格不符不选 | 低精度可接受；许可需核 | 无 | [ ] 决定购买 |
| Bf 109（V-VEH-AIR-003） | 'BF - 109 Messerschmitt'｜CGTrader｜https://cgtrader.com/3d-models/aircraft/military/bf-109-messerschmitt-3d-model | $15 [S] | FBX（约 160 MB） | Royalty Free；$2 的 Bf 109 F 只有 3ds Max 格式，排除 | 低精度可接受；许可需核 | 无 | [ ] 决定购买 |
| C-47（V-VEH-AIR-004） | 'Douglas C-47 Skytrain'｜CGTrader｜https://cgtrader.com/3d-models/aircraft/military/c-47-skytrain | $30 [S] | FBX | 低模 PBR | 低精度可接受；许可需核 | 'Historic Air Superiority and Logistics Collection WWI and WWII'｜CGTrader｜https://www.cgtrader.com/3d-model-packs/historic-air-superiority-and-logistics-collection-wwi-and-wwii｜价格未读到（螺旋桨/副翼分离，FBX/GLB/OBJ）；Synty 'POLYGON War Pack'｜https://syntystore.com/products/polygon-war-pack｜$50 促销（原价 $99.99）[F]，风格化低模，与写实不符，页面同时出现 "Sold out" | [ ] 决定购买 |

**载具小结（片段原结论）**：美军 JonahH 包（M4 各型、Stuart、M8、半履带、105mm 榴弹炮；无 Jeep、CCKW、57mm、无内饰）与德军 JonahH 包（Pz.IV 各型、Panther、Tiger E/(P)/II、Luchs、Puma、251、Opel Blitz、Kübelwagen、leFH 18、Pak 40 等，约 105 个初始模型，4 种涂装）单车摊价最低，部件独立达标但面数不达标。免费样本 'WW2 Armed Forces Demo'｜itch.io / sloffice｜https://sloffice.itch.io/ww2-armed-forces-demo｜标 Free（未写价格）[F]：M5A1 Stuart、M4A3(75)，4.3k–8.2k 三角，Unity 包 233 MB，**页面未写许可，不能直接用于发布**，仅用于先试看质量。

### 11.6 建筑 / 环境 / 道具 / 植被 / 特效

| 需要什么（需求 ID） | 最便宜推荐 | 价格与证据 | 格式 | 面数 / 关键规格 | 达标情况 | 备选（简写） | 决定 |
|---|---|---|---|---|---|---|---|
| 大西洋壁垒碉堡、滩头障碍（捷克刺猬/比利时门/木桩）、铁丝网、桶、无线电（V-BLD-006/007、V-PRP-002/003/005/006） | 'D-Day Atlantik Wall Collection'｜CGTrader｜https://www.cgtrader.com/3d-model-packs/d-day-atlantik-wall-collection | $89 [F] | BLEND/FBX/OBJ/DAE，glTF 18/21 | 21 件：7 个组装好的碉堡 + 74 物体 + 95 模块；PBR（基础色/金属度/粗糙度/法线/AO）；3 级 LOD；Royalty Free（No AI）；不含 LCVP；评论称个别件的 STL/DAE/OBJ 不准，FBX 未被指出有问题 | **达标**（道具与碉堡、有 LOD、PBR） | 单件捷克刺猬：'WW2 Anti-tank Obstacle PBR Game ready'｜CGTrader｜https://cgtrader.com/3d-models/military/other/ww2-anti-tank-obstacle-pbr-game-ready｜$5.99 [S]（部分达标，LOD 未读到）；KangarooZ 捷克刺猬｜RenderHub｜https://www.renderhub.com/kangarooz-3d/czech-hedgehog-obstacle-wwii｜价格需人工确认（638 面，PBR，Extended Use） | [ ] 决定购买 |
| 沙袋、战壕、铁丝网、弹药箱（V-PRP-001/008 等） | 'World War Trenches'｜Unity Asset Store / Aquarius Max｜https://assetstore.unity.com/packages/3d/environments/historic/world-war-trenches-152381 | $44 [F] | 仅 Unity 包，**需在 Unity 导出** | 道具 2.5k–5k 三角，PBR（搜索摘要 [S]）；含沙袋、铁丝网、金属/纸弹药箱 | 道具规格达标 | 无 | [ ] 决定购买 |
| 废墟街区 / 民居模块（V-BLD-004、V-BLD-010、V-BLD-001） | 'WW2 Pack - Ruined Town'｜Unity Asset Store / BSP｜https://assetstore.unity.com/packages/3d/environments/historic/ww2-pack-ruined-town-284837 | $39.99 [F] | 仅 Unity 包（1.9 GB，URP/HDRP；Built-in 在 Unity 6 不兼容），**需在 Unity 导出** | 300+ 物体 [S]；PBR；面数、格式未读到 | 无法判定：是否法国石屋、是否模块化未读到 | 'Modular House & Ruins Mega Pack'｜Unity Asset Store｜https://assetstore.unity.com/packages/3d/props/modular-house-ruins-mega-pack-perfect-for-horror-war-historic-sc-317174｜$75 [S]（23 栋完整房屋 + 21 栋废墟版；Unity 与 Unreal，**需在对应引擎导出**；有破损版；是否诺曼底风格未读到）；'European architecture collection'｜CGTrader｜https://cgtrader.com/3d-model-collections/european-buildings-3d5e4f6c-a67d-4cfb-9112-9128719ec245｜$159 [S]（19 栋建筑 + 2 路 + 2 桥；PBR 低模；贵） | [ ] 决定购买 |
| 桥梁（V-BLD-008） | 'Stone Bridge'（4 段，可拼接）｜CGTrader｜https://cgtrader.com/3d-models/architectural/street/stone-bridge-8dfa3584-2e71-4f79-b8f1-c31c84e6c372 | $5 [S] | FBX、OBJ、Unity 包 | 约 31.6k 面 | 部分达标：中世纪风格，非诺曼底 | 无 | [ ] 决定购买 |
| 登陆艇 / 帐篷 / 树篱 / 农舍 / 石墙等风格化合集（V-PRP-007 等） | 'POLYGON War Pack'｜Synty｜https://syntystore.com/products/polygon-war-pack | $50 促销（原价 $99.99）[F]；页面同时出现 "Sold out" | FBX 源文件 + Unity 2022.3 + UE 4.25 | 588 件；沙袋 8、木箱 6、迷彩网帐篷 4、木栅栏 7、树篱 4、联排屋 6、农舍/谷仓、石墙 8、登陆艇、战壕 | **风格不符**（风格化低模与写实方向不符）；但是唯一读到含登陆艇、帐篷、树篱的合集 | 'WW2 Invasion Beach'｜Unreal Marketplace｜https://www.unrealengine.com/marketplace/en-US/product/ww2-invasion-beach｜价格需人工确认（4 种混凝土碉堡、Higgins 登陆艇（带绑定/动画）、4 种坦克障碍；UE 4.26–5.1，**仅 Unreal，需在 UE 导出**，未迁到 Fab） | [ ] 决定购买 |
| 树篱 bocage / 落叶树（V-NAT-001、V-NAT-002） | 'UK Hedgerow 3D Collection'｜CGTrader｜https://www.cgtrader.com/3d-models/plant/leaf/uk-hedgerow-3d-collection | $4–$40 [S，旧清单] | Hornbeam 集合有 PBR，FBX/OBJ/USD | 含山毛榉树篱、灌木、常春藤；LOD 未读到 | 部分达标：植物可用，但没有土堤 | 'broadleaf oak trees for games'｜CGTrader｜https://cgtrader.com/3d-models/plant/leaf/broadleaf-oak-trees-for-games｜价格需人工确认（4 级 LOD，末级公告牌；偏 Unreal） | [ ] 决定购买 |
| 阿登云杉含雪（V-NAT-003） | 'Canadian Spruce Tree Snow V1'｜CGTrader｜https://cgtrader.com/3d-models/plant/conifer/canadian-spruce-tree-snow-v1 | 价格需人工确认 | 未读到 | 4 级 LOD，PBR，LOD0 619 面；单棵树 | 达标（植被规格）；欧洲云杉是否贴近未核 | 'Conifers Spruces'｜CGTrader｜https://www.cgtrader.com/3d-models/plant/conifer/conifers-spruces｜价格需人工确认（25 个云杉模型含雪；LOD2/3 为公告牌） | [ ] 决定购买 |
| 水面材质 / 泡沫（V-NAT-008） | 'Realistic Water Texture Pack'｜Fab｜https://www.fab.com/listings/b7800dde-fb2c-4134-aa02-7e8ce8e647ff | 价格需人工确认 | 未读到该页 | 含水面贴图与泡沫遮罩（贴图可用于任何引擎） | 需人工确认 | 'Easy Water'｜itch.io / symphonie-studio｜https://symphonie-studio.itch.io/easy-water｜$2.99+ [S]（Unity 着色器，**不能直接用于 Babylon.js**；未提泡沫；不推荐） | [ ] 决定购买 |
| 爆炸序列帧（V-FX-003） | 'Realistic Explosion VFX Spritesheet/flipbook'｜Gumroad｜https://thenerdforum.gumroad.com/l/vznsx | $10 [S] | 序列帧图 | 5 张写实爆炸序列帧；枪口焰、烟未包含 | 部分达标 | 'Animated Game VFX Vol.1 - 37 Sprite Sheet Effects'｜Fab｜https://fab.com/s/cc7e3fda26c2｜价格需人工确认（37 套序列帧；是否写实未核） | [ ] 决定购买 |
| 枪口焰 / 烟 / 弹孔 / 血迹（V-FX-001/002/004/005） | 本次只找到爆炸类；RenderHub 'RC Art - Muzzle Effects'（CC-BY 4.0，需署名）见 §8/§4 旧清单，未重新核实 | — | — | — | 找不到（见 §11.8） | — | — |

### 11.7 音效与音乐

> 许可栏只写页面原话要点，完整条款均需另读 License 页。Sonniss / A Sound Effect 常在促销，价格以结算页为准。
> 已排除 / 不入候选：fusehive 'Military Weapons of War'（https://fusehive.itch.io/military-weapons-of-war-sound-effects-library-army-combat-battlefield-weapon-sou ，$99.99 [F]，现代通用设计音效且未写许可）；delusiondrive 'Sounds of War'（https://delusiondrive.itch.io/sounds-of-war-full-audio-effects-pack-in-mp3-format ，$3.99 起 [F]，MP3/OGG，现代通用，来源未说明，仅可作兜底占位）；Sound Ideas 'World War 2 Firearms Bundle' $204（原 $255）[F]（比 BOOM 起步价高）；Soundstripe（订阅制）、ElevenLabs（AI 生成站）、AudioJungle 'German Soldiers chant'（古代战争）。

| 需要什么（需求 ID） | 最便宜推荐 | 价格与证据 | 格式 | 内容 / 关键规格 | 达标情况 | 备选（简写） | 决定 |
|---|---|---|---|---|---|---|---|
| 步枪/冲锋枪/手枪开火：Garand、Carbine、BAR 单发、G43、Kar98k、MG42 点射、MP40、Thompson、M3、M1911、Luger、P38、M1903、M1919（A-WPN-001–011、015） | 'Weapons of World War II - Designed'｜Sonniss｜https://sonniss.com/sound-effects/weapons-world-war-ii-designed/ | $38.50（原 $55）[F] | 18 文件 / 133 变体，立体声 96k/24bit，350 MB | 设计音效（Designed）；14 枪：M1919、M1911、BAR M1918、Gewehr 43、M3、Kar98k、Luger、M1 Carbine、M1 Garand、MG42、MP40、P38、Thompson、M1903；"Royalty free forever"，无订阅；完整条款未读到 | 部分达标：**无 StG 44、无 MG34**；BAR 仅单发 | BOOM 'World War II Firearms'｜https://boomlibrary.com/sound-effects/world-war-ii-firearms｜$139–$269 [F]（25 枪含 StG44、MG42、Garand、Carbine、Kar98k、G41、FG42；**无 BAR、无 G43、无 MG34**；各版本价未读到） | [ ] 决定购买 |
| StG 44（A-WPN-012） | 'WW2 Weapons: German & US'（德包）｜A Sound Effect / Diegetix｜https://www.asoundeffect.com/sound-library/ww2-weapons-german-us/ | $25–$40 区间 [F]（德包/美包/合集各一档，页面未标明哪档对应哪个；页面显示 $0,00 为占位）**需人工确认** | 48k/24bit，116 MB（德）/ 141 MB（美），未标文件类型 | 德包：StG 44、P38、Kar98k、MP40（181 条）；美包：M1911、Garand、Thompson（203 条）；是否实录页面未明说；Royalty-free，结算时选用户数 | 部分达标：德包单价待确认 | 无 | [ ] 决定购买 |
| MG34（A-WPN-013） | 无商品 | — | — | 搜索中所有 WW2 枪库清单均无 MG34；用 MG42 替代需负责人接受 | 找不到（见 §11.8） | — | — |
| MG42 近/中/远（A-WPN-014） | 'MG 42 Machine Gun'（Pole Position）｜Sonniss｜https://sonniss.com/product/mg-42-machine-gun/ | $111.75（原 $149）[F] | 183 文件 4.81 GB，96k/24，mono/stereo/M-S/环绕，含 Pro Tools/Reaper 会话 | 实录，两个场地，多距离（24 视角/44 声道），含操作声；Royalty-free，无订阅 | 达标（可选升级；非必买） | 无 | [ ] 决定购买 |
| BAR 全自动（A-WPN-005） | 未找到全自动实录 | — | — | Designed 仅单发 | 找不到（见 §11.8） | — | — |
| 75mm 坦克主炮（D-TNK-004） | 'Sherman Cannon - 75mm M4A1'（Pole Position）｜A Sound Effect｜https://int.test-env.asoundeffect.com/sound-library/sherman-cannon-75mm-m4a1（测试环境域名，搜索摘要；Sonniss 页面 URL 未读到，正式页需核实） | $149 [S] | 24+ 文件，96k/24，约 780 MB | 实录 WWII 75mm M4A1 Sherman 空包发射，近/中/远，侧后方，wild foley；Royalty-free [S]；唯一明确 WW2 Sherman 炮实录；德军 88/75 炮无单独商品 | 部分达标：仅美军炮 | 预算紧：'Military Tanks of Warfare'（mini）｜A Sound Effect / Sonniss｜https://www.asoundeffect.com/sound-library/military-tanks-of-warfare-sound-effects-library-war-tank-army-land-vehicles-artillery｜$34.99（原 $39.99）[F]（现代/通用坦克，**非 WW2 型号**，180 文件/60 声音，96k/16bit，约 250 MB；仅作占位）；BOOM 'WW2 Tanks'｜https://asoundeffect.com/sound-library/world-war-2-tanks｜$247.20（原 $309，价格有效期至 2026-12-03）[F]（实录，含 Sherman M4A1、Panther、Hetzer 等 8 种；400+ 文件 27 GB） | [ ] 决定购买 |
| 德军坦克车外/内 + 履带 + 空包发射（D-TNK-006、D-TNK-002） | 'Panzer Tank'（Rob Nokes，发行 sounddogs）｜Sonniss｜https://sonniss.com/product/panzer-tank/ | $37.50（原 $75，-50%）[F] | 39 文件，48k/24，721 MB | WWII Panzer 坦克车内外实录：怠速、加速、换挡、启动、停止、机动、驶过、空包发射（Blank Fire）、倒车提示、履带；"Royalty free forever"，无订阅；页面未写具体型号，需试听 | 部分达标：德军侧；美军 Sherman 需另买 | Sherman 车外：'Sherman M4A1 Medium Tank'（Pole Position）｜Sonniss｜https://sonniss.com/product/sherman-m4a1-medium-tank｜$224.25（原 $299）[F]（Continental R975，249 文件 27.99 GB，96k/24，**不含炮声**）；Sonniss 'Panzer IV Ausf. G' $224.25（原 $299）[F]（https://sonniss.com/sound-effects/panzer-iv-ausf/ ，太贵）；预算紧可用 Panzer 声变调近似美军坦克（需负责人接受） | [ ] 决定购买 |
| Stuka / 战斗机扫射 / 航弹 / 远炮 / 战场环境（B-AIR-001、B-AIR-002、D-AIR-001、F-AMB-005） | 'Sounds of War Sound Effects Library'｜Sound Ideas｜https://sound-ideas.com/products/sounds-of-war-sound-effects-library | $65（原 $129）[F] | 75 条，24/48 WAV，74 MB（页面三种规格均标 "Sold out" 但有加购按钮，**需人工确认可买**） | 页面称历史 WWII 音频：武器、飞机、部队、装甲突击、战场环境、整场战斗序列、装备与坦克；提到 Stukas、Flying Fortresses；型号清单未列；实录/翻制来源页面未明说；Royalty-free，"无后续费用" | 部分达标：只确认含 Stukas 与飞机，未确认含航弹、扫射，需试听 | 单独 WW2 战斗机：A Sound Effect 'WW2 Aircraft' 约 $150 [S]；Sound Ideas 'P-51 Mustang'（BOOM）$65 [S]（https://sound-ideas.com/Product/183/P-51-Mustang-Sound-Effects ，本次抓取 404，价格未能复核）；Sonniss Spitfire Mk IX $449 [F]（太贵，排除）；fusehive 'Warfare Ambiences'（https://fusehive.itch.io/warfare-ambiences-sound-effects-library-army-combat-battlefield-weapon-war-backg ，[S]，价格与内容未读到，需人工确认） | [ ] 决定购买 |
| Bazooka / Panzerschreck / Panzerfaust（B-HVY-001） | 未找到任何 WW2 发射器实录商品 | — | — | fusehive 现代库含 rocket launcher 但是现代设计音效（$99.99）；A Sound Effect 'Warfare 2' $199 [F]（清单提供，未复核）为现代 RPG；建议不买 | 找不到（见 §11.8） | — | — |
| 坦克被击中/跳弹（D-TNK-005）、卡车/吉普/半履带（D-VEH-001）、Stuka 俯冲警报专门库 | 未找到 | — | — | — | 找不到（见 §11.8） | — | — |
| 英语战场喊话（G-VOX-001、G-VOX-003、G-VOX-004） | 'Army Voice Pack'｜itch.io / Raketenfaust｜https://raketenfaust.itch.io/army-voice-pack | $4.49 起 [F] | WAV | 155 条 WW2 士兵语音，专业配音（英语，美式与否页面未写）；允许商用与游戏、可编辑；不需署名；禁止转售/再分发 [F]；可能含口令（页面未列，需试听）；受伤/倒地痛呼未明确 | 部分达标：需试听 | 无 | [ ] 决定购买 |
| 德语喊话 / 口令（G-VOX-002、G-VOX-004） | 'SonicArchive Military Personnel German'｜Shockwave-Sound｜https://shockwave-sound.com/sound-effects/12396/sonicarchive-military-personnel-german | $30 全套（单条 $3.45）[F] | WAV 16bit/44.1k 立体声，每条 1–3 秒 | 46 条德语军人喊话：Achtung、Kompanie、Marsch、Links、Rechts、Jawohl 等；出品方为 DYNAMEDION 声音设计师；页面仅链接 License Terms，未读到 | 部分达标：规格偏低；受伤/倒地痛呼未明确 | 无 | [ ] 决定购买 |
| 无线电（H-RAD-001） | 'Military Radio Voices'（GfxSoundsStudios）｜Sonniss｜https://sonniss.com/product/military-radio-voices | $25（原 $35）[F] | 48k/32bit，155 文件，112 MB | 现代风格军事无线电语音，125+ 条；**非 WW2 年代**，无静电/PTT 专门说明 | 不达标（年代不符） | 'Just Noise'｜Sound Ideas｜https://portal.sound-ideas.com/Product/430/Just-Noise-Sound-Effects-Library｜$195 [S]（140 条，含 1940s–50s 老电台杂音、静电、嗡声）；MuseHub 'Radios and TVs'｜https://www.musehub.com/loops-and-audio/radios-and-tvs｜价格、许可需人工确认 [S]；片段另提 WebAudio 自生成噪声 $0（是否算允许的后期，须按 §2.2 D-MOD 由负责人确认） | [ ] 决定购买 |
| 菜单 / 结算音乐（I-MUS-001、I-MUS-002） | 'Warfare Soundtrack 25 Tracks Vol 1'｜itch.io / Bobby Cole｜https://bobbycolemusic.itch.io/warfare-spundtrack-25-tracks-vol-1 | $24.99 起 [F] | WAV 44.1k/16bit | 25 曲（22 个唯一 WAV），约 1.5 小时，不可循环（循环需自行处理）；itch 标签含 WWII（搜索摘要）；"Unlimited" royalty-free | 部分达标 | 免费：'Free Military WW2 Game Music Pack'｜Unity Asset Store / alkakrab｜https://assetstore.unity.com/packages/audio/music/free-military-ww2-game-music-pack-362056｜免费 [F]（457.7 MB，曲目清单页面未显示；标准 Unity EULA（Extension Asset），用于浏览器游戏是否允许需人工确认）；'TOP SECRET WW2 Music Pack'｜itch.io / Daniel Carl｜https://danielcarlmusic.itch.io/top-secret-ww2-music-pack｜$34.99 起 [F]（14 主题 32:36 + 13 无缝循环，WAV 44.1k/16bit，591 MB） | [ ] 决定购买 |
| 户外混响 IR（J-IR-002 等） | 'Forest Reverb'（outdoor IR）｜A Sound Effect｜https://www.asoundeffect.com/?p=30802（该页抓取只返回其他商品，未能确认；价格来自搜索摘要） | $25 [S] | 未读到 | 134 条森林 IR；需人工确认商品页 | 需人工确认 | 免费 'Canyon Soundings'｜KVR｜https://www.kvraudio.com/forum/viewtopic.php?p=9276689｜免费 [S]（51 条 IR，48k/24 立体声；峡谷场景不匹配西线；许可需人工确认）；BOOM 'Outdoor Impulse Responses'｜https://www.boomlibrary.com/sound-effects/outdoor-impulse-responses/｜页面显示 "$69.00 / year"（与 "One purchase. Forever" 矛盾，需人工确认）[F]（68 WAV，192k/24，约 0.6 GB；royalty-free，禁止转售；仅向商业客户销售） | [ ] 决定购买 |

### 11.8 找不到任何达标或可用商品的清单

> 处理建议三选一：**灰盒占位**（程序生成基本几何体 / 静音 `AudioBuffer`，按 ASSET_GAPS §3.2 约定，里程碑验收时仍须在 ASSET_GAPS 里有"决定"）、**近似替代**（须负责人接受史实偏差）、**定制**（外包）。以下建议为文档作者依据片段结论所拟，需负责人确认。

| 类别 | 找不到的项 | 片段原结论 | 处理建议 |
|---|---|---|---|
| 武器（视觉） | BAR（第一人称级）、M1919（线索未核）、M1903A4 带镜、M1911、M1/M3 刺刀、Kar98k 带镜（待核）、Gewehr 43、MG34、Panzerschreck（仅 6.2k 低模）、Tellermine、S-Mine，以及 Mk2/M24 之外的第一人称级手雷 | Loss 的 G43/MG34/MG42/MP40/FG42 因面数低且许可存疑不再推荐 | BAR / M1903 / M1911 / Panzerschreck：用低模（见 §11.2）作第三人称与占位，第一人称灰盒占位或定制；MG34：近似替代（外形用 MG42）或灰盒；G43、刺刀、地雷：灰盒占位 |
| 角色 | 德军伞兵、德军冬季、美军冬季 | 现成商品未找到；Fallschirmjäger 仅有 RenderHub FG42 枪与 Daz Genesis 8 人偶 | 近似替代：国防军 / 美军夏季角色 + 贴图变体（须按 §2.2 D-MOD 由负责人确认）；或灰盒；或定制 |
| 第一人称手臂 | 带 WW2 军装袖的现成手臂 | 未找到现成"WW2 袖子"商品；通用手臂需重贴图（是否允许见 D-MOD） | 近似替代（买通用/MP40 包手臂 + 贴图变体，需确认 D-MOD），或定制 |
| 动画 | 医护救治/拖拽（V-ANM-006）、载具进出/驾驶/炮手（V-ANM-008） | 本次未找到现成商品 | 灰盒占位（无动画的姿势切换）；或定制 |
| 载具与火炮 | 面数 5 万–15 万且价格明确的美军车辆（只有 M8 85k 价格未读到）；Jeep / CCKW 的达标版；57mm M1 反坦克炮；Luchs 单品；带内饰的坦克（仅 Rng3r 美军车辆包宣称有"详细内饰"，价格/面数/格式/许可均未读到，见 https://forums.unrealengine.com/t/rng3r-us-military-vehicle-pack-1/2840763 ，需人工确认） | 单价便宜的车均为低模；达标者仅 Fab Pz.IV / Pz.V（仅 Unreal）与 M8 | 低模近似替代或灰盒；57mm 用 6-pounder 近似（价格需确认）；内饰：灰盒占位或定制 |
| 建筑 | 诺曼底乡村教堂（含钟楼）、阿登冬季村舍、欧洲小火车站、bocage 土堤（只有植物）、诺曼底风格石屋/农场的写实模块化套件、单独的写实 LCVP（只有 UE 包与 Synty 风格化登陆艇） | 只见中世纪/奇幻或风格不明的合集；Unity 'WW II French Village Asset Pack'（https://assetstore-fallback.unity.com/packages/3d/environments/world-war-2-french-village-25233）已下架 | 灰盒占位（盒体 + 通用材质）；或定制；LCVP 可考虑 UE 包 'WW2 Invasion Beach'（仅 Unreal，价格需确认，需在 UE 导出） |
| 环境 / 特效 | 农作物（麦、玉米）、水面（无合适商品，Unity 着色器不能直接用于 Babylon.js）、写实枪口焰 / 烟序列帧 | 本次只找到爆炸类序列帧 | 水面：自写 Babylon 着色器（贴图可另买）；农作物与枪口焰：灰盒或定制 |
| 音频：枪械与重武器 | MG34、BAR 全自动、Bazooka / Panzerschreck / Panzerfaust、坦克被击中/跳弹、航空炸弹专门库、Stuka 俯冲警报专门库 | 搜索中所有 WW2 枪库清单均无 MG34；未找到任何 WW2 发射器实录商品 | MG34：近似替代（MG42 声，须负责人接受史实偏差）；其余：静音占位或定制 |
| 音频：载具 | 卡车、吉普、半履带 | 未找到 WW2 车辆实录商品 | 近似替代（现有坦克库通用引擎声，需接受）或静音占位 |
| 音频：人声与无线电 | 受伤/倒地人声（未证实）、WW2 年代无线电 PTT | 现有人声库两者均未明确；无线电仅现代语音 | 试听后再定；否则静音占位或定制 |

### 11.9 总价（两档）

> **口径**：
> - **最低价方案（含不达标的低模）**：每个需求取 §11.2–§11.7 里"最便宜推荐"列的商品，包括"不达标"的低模；音乐取免费 Unity 包（若其许可经确认不可用，加 $24.99）。
> - **尽量达标方案**：每个需求取页面读到的"达标"或最接近规格的商品；车辆取 JonahH 两个包 + EVOLVE Pz.IV / Pz.V（仅 Unreal）；音频含 MG42 近/中/远升级与付费音乐；**不含**不达标的低模（BAR、美军枪包、美军步兵 €9.99 低模、变体包）；Jeep / CCKW 无更达标替代，仍保留最便宜的低模；JonahH 两包仅"部件独立"达标、面数仍不达标。
> - 两档均**不含**："价格需人工确认"的商品（Alpenwolf 美军步兵、M8 Greyhound 85k、M2A1 单品、Panzerschreck、各云杉/橡树/水面贴图/37 套序列帧等）、"找不到"的项（§11.8）、可选项（World War Trenches $44、单件捷克刺猬 $5.99、片段中的现代通用库）。
> - 小计为本文档作者对上表价格的逐项相加，未计税；€ 单列不折算。

| 类别 | 最低价方案（含不达标的低模） | 尽量达标方案 |
|---|---|---|
| 武器（含 FP 手臂，见 MP40 包） | $251.38：Garand 8 + Carbine 23.99 + Thompson 5.50 + M3 25 + Bazooka 23.99 + Mk2 2 + Kar98k 7 + MP40 24.99 + StG44 55 + **MG42 Fab 9.99** + Luger 17.95 + Panzerfaust 20 + M24 2.99 + **BAR 4.99 + 美军枪包 19.99**（低模） | $276.41：同上，但 MG42 取 CGTrader $60，且不含 BAR、美军枪包。若还要 P38 另加 $25 |
| 角色 | $145 + €9.99：德军 $35 + 空降兵 $60 + 变体包 $50 + 美军步兵 €9.99 | $100：德军 $40 + 空降兵 $60；美军步兵（Alpenwolf，价格需人工确认）未计入 |
| 动画 | $30.99：Rifle Starter 3.99 + kevdev 15 + Unity Animated WW2 Weapons #2 12 | $30.99（同左） |
| 载具与火炮 | $109.70：Sherman 9 + Willys 5 + Unity US#2 7.90 + US#3 9.90 + M8 12 + 半履带 12 + Unity 德 #2 7.90 + Pz.IV G 12 + Tiger E 12 + Blitz 10 + Pak 40 12 | $232.86：JonahH 美军包 49.99 + JonahH 德军包 49.99 + EVOLVE Pz.IV 59.99 + Pz.V 59.99 + Willys 5 + Unity US#2 7.90（JonahH 德军包与 EVOLVE 两车有重叠，如接受低面数可去掉其一） |
| 飞机（仅过场） | $69：P-47 9 + Ju 87 15 + Bf 109 15 + C-47 30 | $69（同左） |
| 建筑 / 环境 / 特效 | $143.99：Atlantik 89 + 废墟街区 39.99 + 石桥 5 + 爆炸序列帧 10 | $143.99（同左，该类无更便宜或更达标的替代） |
| 音效与音乐 | $374.49：Designed 38.50 + 德包 25 + Sherman 炮 149 + Panzer Tank 37.50 + Sounds of War 65 + 英语喊话 4.49 + 德语喊话 30 + 森林 IR 25；音乐取免费 Unity 包（许可待确认，若不可用 +$24.99 = $399.48） | $511.23：同上，音乐取 Bobby Cole $24.99，另加 MG42 近/中/远 111.75 |
| **合计** | **$1,124.55 + €9.99** | **$1,364.48** |

**价格证据构成**（相同口径，按上表商品）

| 构成 | 最低价方案 | 尽量达标方案 |
|---|---|---|
| 页面直接读到 [F] | $502.61 | $756.54 |
| 仅来自搜索摘要 [S] | $596.94 | $582.94 |
| [F] 区间下限、需人工确认（Diegetix 德包 $25–$40，按下限计） | $25.00 | $25.00 |
| 合计 | $1,124.55 | $1,364.48 |

- **需人工确认的价格 / 条件**（影响小计）：Diegetix 德包单价（$25–$40，最高可再加 $15）；Sherman 炮声 $149 为 [S]，且链接为测试环境域名；Sounds of War 页面标 "Sold out" 但有加购按钮；Pak 40 的 $12 与 $20 冲突；MP40 包许可若为 Editorial 则排除（−$24.99，且失去 FP 手臂）；CGTrader 的 [S] 价格占武器、角色、飞机、多数低模车辆的大头，须在结算页逐项核对。
- **仍缺价格、未计入**：Alpenwolf 美军步兵、M8 Greyhound 85k、M2A1 单品、Panzerschreck、Panzerfaust 60、Davlet 手臂、云杉 / 橡树、水面贴图、37 套序列帧、Mauser Kar98k、max3ddd 251、MuseHub 等。
- 促销有截止期（如 BOOM WW2 Tanks 至 2026-12-03），Unity Asset Store / Sonniss 价格可能随促销变动。
- **所有候选均未下载检验**：面数、贴图分辨率、部件独立性、格式与许可均为页面或搜索摘要所见。

### 11.10 购买前必查清单（每件）

对每一件打算购买的商品，下单前逐项核对，并在 `LICENSES.md` §6 简化登记里留痕：

- [ ] **格式**：文件列表里有没有 FBX / glTF（glb）？只有 .uasset / .unitypackage 的，须确认能在对应引擎导出 FBX/glTF，且商店 EULA 允许导出再分发（本项目是 Babylon.js，浏览器分发）。仅 Unreal / 仅 Unity 的已在表中标"需在该引擎导出"。
- [ ] **Fab 许可**：Fab 页面"许可"字段多数读不到。下单前确认是 **Standard** 还是 **UE-Only**；UE-Only 的不可用于本项目。
- [ ] **是否标 Editorial**：商品页或许可文字含 "Editorial"、"Editorial Use Only"、"IP Restricted" 的一律排除（如 Loss 系列、MP40 包摘要中的 "editorial license (no AI)"、TurboSquid Luchs、CGTrader 'Panzer II Ausf L Lynx'）。
- [ ] **面数与贴图**：对照 §11.1，实际三角形数（不是顶点数、不是多边形数）、贴图分辨率（4K / 2K）、有无 LOD；Fab 页面多数不列面数。
- [ ] **可动部件是否独立**：第一人称武器的枪栓、弹匣、扳机、表尺；车辆的炮塔、炮管、履带、负重轮、舱盖是否为独立网格 / 骨骼；是否已绑定。
- [ ] **AI 使用声明**：页面是否声明 "No AI"；EVOLVE Studio 页面注明"AI 使用不允许"，须与 §2.4 禁用清单一并核对。
- [ ] **来源**：按 §2.3 排查是否从商业游戏提取（本次未发现，但 CGTrader / RenderHub 卖家未逐一核查来源）。
