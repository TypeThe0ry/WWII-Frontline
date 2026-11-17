# M0-003 字体许可证核实与登记（R10）

## 元数据
- 里程碑：M0
- 工作流：assets
- 规模：S
- 依赖：M0-001
- 素材需求：V-FNT-*（仅登记，不购买）；素材状态：n.a.（本任务不引入字体文件进构建）
- 引用文档：UI_SPEC §2.3; ASSET_SHOPPING_LIST §4.1.6; LICENSES §1; LICENSES §3; BRIEF R10

## 背景
UI_SPEC §2.3 选定了 4 个字体：标题 **Barlow Semi Condensed**（500/600）、正文 **IBM Plex Sans**（400/500/600）、等宽数字 **IBM Plex Mono**（500）、简体中文 **Noto Sans SC**（400/500/700）。它们的许可证在文档里全部标"SIL OFL 1.1（未核实）"。总控裁定 R10 要求在 M0 核实并登记 LICENSES.md，**未登记不得入库**（UI_SPEC 附录 B-1）；M1-053（UI 框架）依赖本任务结论才能加载字体。
UI_SPEC §2.3 另规定：全部自托管、WOFF2、`font-display: swap`、中文按 unicode-range 分 ≥ 4 片并子集化，进入主菜单前字体总预算 ≤ 400 KB（br 后）【待调参】。SHOP §4.1.6 的 V-FNT-001..004 是更早的候选（Black Ops One 等），其中 V-FNT-004 把 Plex/Noto 列为"待核实"；UI_SPEC 选型优先，V-FNT-001..003 **不登记**（未被选用），仅在备注里说明。

## 具体内容
### 要创建/修改的文件
- `LICENSES.md` §3 台账：为每个**实际选用的字体族**新增一行（`type=font`）。`asset_id` 用 `AST-V-FNT-004-NN`（IBM Plex Sans、IBM Plex Mono、Noto Sans SC 归 V-FNT-004，序号 01..03）；Barlow Semi Condensed 无专属需求 ID，`req_id` 填 `—` 并在 `notes` 写"UI_SPEC §2.3 标题体"，`asset_id` 用 `AST-V-FNT-UI-01`。字段按 LICENSES §2 填：`license_name_version`、`chk_engine/chk_web/chk_commercial`（Y/N/PENDING）、`chk_attribution`（OFL 通常 NONE，但随字体分发须附带许可证文本，需在 notes 写明）、`allowed_processing` 只填 `CONVERT`（TTF→WOFF2 格式转换；LICENSES §2 的处理代码里没有"子集化"专用码，子集化是否属于 OFL "Modified Version" 须按许可证原文/FAQ 判断后写入 notes，并在 PLAN_ISSUES 提议是否需要新增处理码）。
- `licenses/texts/{asset_id}.txt`：各字体上游仓库中的许可证原文快照（用 WebFetch/下载取回；存档访问日期与 URL 写入文件头注释行之外的 notes，不改原文）。
- `tools/asset-pipeline/docs/font-subsetting.md`：子集化流程说明（见下"行为"）。
- 如无法联网：`docs/PLAN_ISSUES.md` 追加条目，台账行状态保持 `PENDING`。
### 要实现的行为
1. 对 4 个字体逐一到**上游官方来源**（字体项目官方仓库或 Google Fonts 页面）核实：①许可证名称与版本及原文；②是否允许网页嵌入（@font-face 自托管）、子集化、WOFF2 转换、随游戏分发；③是否有**保留字体名（Reserved Font Name, RFN）**声明——若有，子集化/格式转换后的衍生文件能否继续使用原名，按许可证原文与官方 FAQ 判断，**拿不准则记 PENDING 而不是 OK**；④当前版本号与文件来源 URL。
2. 记录风险：Noto Sans SC 全量文件体积大（UI_SPEC §2.3：量级数 MB，未核实具体字节数）——本任务记录实际下载到的文件大小，作为 M1-053 子集化预算的输入。
3. 写子集化流程说明：输入 = i18n 文案抽取的字符集（GB2312 一二级 + 游戏词表 + 常用标点，UI_SPEC §2.3）；工具建议 `fonttools`（`pyftsubset`）+ WOFF2 压缩；按 unicode-range 切 ≥ 4 片，首屏仅启动页词表一片；产物命名与 `font-display: swap`；预算检查命令（`brotli -q 11` 后求和 ≤ 400 KB【待调参】）。说明中不写具体字符数或字节数估计，除非本任务实测。
4. 不下载字体进仓库构建目录；若需验证子集化可行性，放 `assets/incoming/fonts/`（已被忽略）。
### 接口与数据
- LICENSES §3 表头列名不得改动（管线按列名解析）；单元格内不得出现 `|`。

## 不做什么
- 不实现字体加载、`@font-face`、i18n 抽取脚本（M1-053）；不做子集化代码（M1-053/M1-054）。
- 不核实图标集许可证（图标由执行者原创 SVG，见 NEEDS_HUMAN H-003）。
- 不采购任何付费字体；V-FNT-001..003 候选不登记。

## 验收标准
1. LICENSES.md 台账中 UI_SPEC §2.3 的 4 个字体族**各有一行**（共 ≥ 4 行），每行状态为 `OK` 或 `PENDING`，无空白必填字段。
2. 每行对应的 `licenses/texts/*.txt` 文件存在且非空（`wc -c` > 1000 字节），内容为上游许可证原文。
3. 每个字体的 RFN 情况在该行 `notes` 里有明确结论（"无 RFN"/"有 RFN：…，子集化处理：…"）；结论依据的 URL 与访问日期（`YYYY-MM-DD`）写在 notes。
4. `tools/asset-pipeline/docs/font-subsetting.md` 存在，含：输入字符集来源、子集化命令、≥ 4 片切分规则、400 KB 预算检查命令。
5. 无法联网核实的项在 PLAN_ISSUES 有条目且台账状态为 `PENDING`；`grep -c "未核实" LICENSES.md` 不得在 `OK` 行内出现。

## 测试要求
- 单元测试：无。
- 集成/无头测试：人工核对表头列名未变（`head -n 70 LICENSES.md` 比对原表头）。
- 性能基准：无。
- 回归：无。

## 性能预算
无（字体 400 KB 预算由 M1-053 验收，见 UI_SPEC §2.3）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- tasks/README §1 说 LICENSES.md "人类负责人登记；执行者只读"，但 INDEX 明确本任务要在 LICENSES.md 登记，视为对该规则的显式例外；`purchaser` 填"免费-执行者核实"，`order_id` 填 `FREE-{日期}`。
- 对应人类事项：NEEDS_HUMAN H-036（字体方案确认与许可证核实）。若人类改选字体，本任务只需重复流程。
- 不得编造许可证条款或链接：只记录实际取到的原文；取不到就 PENDING。
