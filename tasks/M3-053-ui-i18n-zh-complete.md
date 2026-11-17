# M3-053 UI：简体中文文案与 CJK 字体完整性

## 元数据
- 里程碑：M3
- 工作流：ui
- 规模：M
- 前置依赖：M3-033, M3-035, M3-034, M3-036
- 素材需求：V-FNT-*（字体，M0-003 已登记）；素材状态：real（字体许可证须为 OK）
- 引用文档：UI_SPEC App.A; UI_SPEC §2.3; UI_SPEC §6; UI_SPEC §7.3; ARCHITECTURE §2 (A7); ARCHITECTURE §11.4; GAMEDESIGN §0.3

## 背景
A7：UI 首发英文 + 简体中文，所有文案走 i18n key（禁止硬编码字符串）。到 M3 为止所有屏幕（UI_SPEC §4.1–§4.19，对应附录 A 的前缀 `ui.boot/menu/browser/faction/deploy/hud/map/radial/score/down/cmdr/officer/build/veh/chat/voice/settings/result/pause/report/vote/toast/banner/tutorial/dialog/common`）已出现；本任务补全 zh-CN，并验证字体与排版。
字体规则（UI_SPEC §2.3）：
- 自托管 WOFF2、`font-display: swap`；中文字体 Noto Sans SC（Regular 400/Medium 500/Bold 700），许可证（SIL OFL 1.1）**须由 M0-003 核实并登记 LICENSES.md，状态 OK 才可入库**。
- 子集化到"GB2312 一级二级 + 游戏词表 + 常用标点"；词表由 i18n 文件构建期自动抽取字符集；按 unicode-range 分 ≥ 4 片；启动页只加载启动页词表那一片；进入主菜单前总字体预算 ≤ 400 KB（br 后）【待调参】。
- 行高：拉丁 1.35、中文 1.5；最小字号 12 px（中文 13 px）；中文环境不使用 `text-transform: uppercase`、不使用字间距大写技巧；HUD 指南针显示 北/东北…。
体积：主菜单（JS+WASM+UI）≤ 8 MB，其中字体子集约 1.5 MB 的估算（ARCH §11.4）。
M3 完成时的退出标准 E3-12：en 与 zh-CN 键 100% 对齐（CI）；常用 3 500 字覆盖；最长文本不溢出。

## 具体内容
### 要创建/修改的文件
- `client/src/i18n/en/*.json`、`client/src/i18n/zh-CN/*.json` —— 补全全部 key（按前缀分文件）；译文由执行者撰写，需保留术语表一致性。
- `client/src/i18n/glossary.json` —— 术语表（Garrison→驻地、Outpost→前哨、Observation Post→观察哨、Airhead→空降场、Supply Drop→补给空投 等，**命名由执行者提议，待人类审阅，且不得使用参考作品的官方中文译名**）。
- `tools/i18n/check.ts` —— CI 检查：①en 与 zh-CN 键集合相等；②占位符（`{n}`、`{name}`）两侧一致；③无空字符串；④复数形式一致；⑤key 命名符合前缀表（附录 A）；⑥源码中无硬编码 UI 字符串（扫描 JSX 文本节点，白名单除外）。
- `tools/i18n/charset.ts` —— 从 zh-CN 文案抽取字符集，合并"通用常用 3 500 字"基础表，生成子集字符表。
- `tools/i18n/common_3500.txt` —— 常用字表。来源：《通用规范汉字表》一级字表（3 500 字）；**若无法取得权威文件，不得凭记忆编造**：改用 GB2312 一级字（按 Python `bytes([0xB0..0xD7, 0xA1..0xFE]).decode('gb2312')` 的区位范围生成约 3 755 字）作为覆盖基线，并写 PLAN_ISSUES 说明。
- `tools/asset-pipeline/src/font_subset.ts`（改，M1-065 已有）—— pyftsubset/`fonttools` 子集化流程、WOFF2 分片（≥ 4 片）、`unicode-range` CSS 生成。
- `client/tests/i18n/overflow.spec.ts` —— 长文本溢出检查（Playwright）。
- `.github/workflows/ci.yml`（改）—— `test-ts` 作业加入 `i18n:check`。
### 要实现的行为
1. 补全 zh-CN：把所有 `ui.*` key 翻译；游戏术语使用统一译法（术语表）；数字/单位格式（m、km/h、s）与英文一致。
2. CJK 覆盖检查：`font_subset` 产物中，常用 3 500 字与全部 zh-CN 文案字符 100% 覆盖；缺字时构建失败。
3. 溢出检查：对每个 M3 新增屏幕（指挥官、Spotter/OP、载具/火炮、HUD、Assault）在 zh-CN 与"伪长文本"模式（en 文本 ×1.4 扩展 + 全角字符）下渲染，断言带 `data-i18n-fit` 属性的容器 `scrollWidth ≤ clientWidth` 且无截断。
4. 字体加载策略：启动页仅加载词表片；其余按 `unicode-range` 按需；加载失败时回退并加 `data-font-fallback`（UI_SPEC §2.3）。
5. i18n 加载：语言切换（设置项）无需刷新生效；缺 key 时显示 key 本身并上报开发日志（生产构建 CI 不允许缺 key）。
### 接口与数据
- key 示例：`ui.cmdr.slot.bombing.name`、`ui.veh.arty.err.out_of_range`；占位符使用 ICU 风格 `{n}`。

## 不做什么
- 新增 UI 功能或改版式；其它语言（Backlog）。
- 语音转写；游戏内聊天翻译。
- 字体许可证核实（M0-003）；若状态非 OK 则本任务 `blocked`。
- 为每种长文本重新设计布局（只做溢出检查与 CSS 修正）。

## 验收标准
1. `pnpm -C client i18n:check` 通过：en 与 zh-CN 键 100% 对齐（差集 = 0）、占位符一致（违规 = 0）、空值 = 0。
2. 字体子集覆盖：常用 3 500 字（或 GB2312 一级基线，见上）+ zh-CN 全部文案字符覆盖率 100%（`charset.ts` 输出缺字数 = 0）。
3. 字体预算：进入主菜单前字体总量 ≤ 400 KB（br 后，`ls -l` + `brotli -q 11` 统计）；分片数 ≥ 4；启动页只请求 1 片（Playwright 网络日志）。
4. 溢出：5 类 M3 屏幕 × 2 种语言（zh-CN、伪长文本）= 10 个截图用例，`data-i18n-fit` 容器溢出数 = 0。
5. 硬编码扫描：`client/src/ui` 中疑似硬编码字符串违规数 = 0（`i18n:check` 子项）。
6. 视觉回归：中文版 HUD 与指挥官面板各 1 机位入基线（M3-048 机制），差异阈值 0.1%。
7. LICENSES.md 中 V-FNT 对应字体状态均为 `OK`（读取并断言）；否则任务 `blocked`。

## 测试要求
- 单元（vitest）：`check.ts` 的各规则（缺 key/多 key/占位符不一致样例必报错）；`charset.ts`。
- 集成：Playwright 加载 zh-CN，遍历 M3 屏幕；字体网络请求统计。
- 性能基准：字体首包体积、首次内容绘制（`performance.timing`，主菜单可交互 ≤ 2 s 二次访问，UI_SPEC §8.1）。
- 回归：英文界面截图基线不变。

## 性能预算
- ARCH §11.4 主菜单 ≤ 8 MB；UI_SPEC §2.3 字体 ≤ 400 KB（br）；UI_SPEC §8.1 UI 脚本预算不回归。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 译文质量需母语者审阅（人类，H-036 字体与方案确认之外的文案审阅属 H-095 前的自查项）；机器翻译痕迹是已知风险。
- Noto Sans SC 子集后 ≤ 400 KB 是否可达为"未核实"（UI_SPEC 附录 B-5）；超出则写 PLAN_ISSUES，调整分片/字重（不得放宽预算数字）。
- 依赖 M3-033/034/035/036 完成后才能覆盖其 key；之前的 M0–M2 key 若已缺失，一并补齐。
