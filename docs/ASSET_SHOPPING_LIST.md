# Frontline 素材采购清单（ASSET_SHOPPING_LIST）

> 读者：人类负责人（审核、付款、把文件放进 `assets/incoming/`、在本表勾选"已购"）。
> 核实日期：**2026-11-17**（本文所有"已核实"均指该日期当天的页面所见；促销价会变，**下单前以结算页为准**）。
> 本文由两份调研片段（视觉、音频）合并整理而成：**全部链接、价格、证据强度标签均沿用调研原文，没有新增或编造**。片段中标"未核实/需人工确认"的内容原样保留该标记。
> 本文不是法律意见；金额较大（> US$500）或条款含糊的采购，请负责人读一遍原文或取得卖家书面确认。
> 章节编号稳定，其它文档/任务文件按 "ASSET_SHOPPING_LIST.md §4.2" 的方式引用，请勿重排。

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

---

## 3. 渠道条款总表

### 3.1 说明

- "适用"列：V = 视觉（3D/贴图/动画/特效/字体），A = 音频。
- 五问：①非原生引擎（能否在 Babylon.js 等非 Unreal/Unity 引擎使用）②网页分发（含"可被玩家从浏览器提取"问题，见 §3.4）③商用与捐赠 ④署名 ⑤席位（单人/多人）。
- 分级：**推荐 / 慎用 / 禁用**；另有"PENDING"（待书面确认）。
- 来源列：URL｜核实日期（均为 2026-11-17）｜证据强度。取不到官方页面的已写明。

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
