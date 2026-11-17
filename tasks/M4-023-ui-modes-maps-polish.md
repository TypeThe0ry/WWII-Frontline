# M4-023 UI：模式/地图选择与加载界面更新

## 元数据
- 里程碑：M4
- 工作流：ui
- 规模：M
- 前置依赖：M1-055, M4-012, M4-011
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.3; UI_SPEC §4.4; UI_SPEC §4.1; UI_SPEC §4.5; GAMEDESIGN §2.3; GAMEDESIGN §3.7; GAMEDESIGN §10.1

## 背景
首发三图（Carentan/Omaha Beach/Foy）× 三模式（Frontline/Assault/Skirmish）。UI_SPEC §4.3.1 已定义服务器浏览器字段：服务器名、地图名 + 缩略图、模式、人数（真人 + bot 分开：`72+18/100`）、对局阶段、ping、区域、版本、密码、bot 填充标记等；筛选项含地图（多选）、模式（多选）；筛选条件存 `localStorage`。UI_SPEC §4.1 为启动/加载页（地图资源进度：地形、纹理、模型、音频、着色器编译；缓存状态；提示轮播；`Esc` 取消）。UI_SPEC §4.4 阵营选择显示"当前地图/战况摘要"。
本任务做：
1. 服务器浏览器的**模式/地图过滤**与列表字段更新（三图缩略图、三种模式标签、`Assault` 攻方标记、`Skirmish` 子区名）。
2. **三图加载页与简报**：加载页按地图显示简报（地图名、模式规则一句话、史实背景一两句、操作提示）；进度准确（分项字节数/百分比）。
3. **登陆艇出生界面**：部署界面（UI_SPEC §4.5）的重生点列表与地图中新增 `Boat` 类型符号及状态（可用/满/封锁）、预计到岸时间（来自 M4-011 `SimEvent::BoatState`）。
约束：所有文案走 i18n key（`ui.browser.*`、`ui.boot.*`、`ui.deploy.spawn.*`、`ui.faction.*`）；历史陈述须遵守"未核实不展示"（Foy 德军番号、日期细节、Omaha 潮位）；不得出现参考作品名称；图标自绘 SVG（H-003）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/browser/ServerBrowser.tsx` —— 增加 `mode`/`map` 过滤与排序；`FilterState` 持久化。
- `client/src/ui/browser/MapThumb.tsx` —— 三图缩略图（占位线稿，自绘 SVG/canvas；后续替换）。
- `client/src/ui/boot/MapBriefing.tsx`、`LoadProgress.tsx` —— 简报与分项进度。
- `client/src/ui/deploy/SpawnPointBoat.tsx` + `client/src/ui/map/symbols/boat.svg.ts` —— 登陆艇符号与到岸倒计时。
- `client/src/i18n/{en,zh}/maps.json` 与 `modes.json` —— 地图/模式简介 key。
- `data/ui/map_cards.json` —— 每图元数据（`map_id`、支持模式、缩略图 key、简报 key），非数值。
- `client/tests/ui/{browser_filter,briefing,boat_spawn}.spec.ts`。
### 要实现的行为
1. 浏览器：模式与地图多选过滤 + 搜索；三种模式×三张图组合下列表字段显示正确；不支持的组合（如 Omaha+Frontline）在筛选里允许选择但结果可为空（空态文案 UI_SPEC §4.3.5）。
2. 加载页：按 `map_id` 选择简报；分项进度（地形/纹理/模型/音频/着色器编译，UI_SPEC §4.1.1）与 `M1-006` 加载器事件对齐；进度在 0–100% 单调递增，总字节数与 manifest 一致（误差 ≤ 1%）。
3. 简报文案 en/zh 完整，长度 ≤ 60 字（中文）/ 40 词（英文）；不含未核实史实。
4. 部署界面：`Boat` 符号（形状 + 文字双编码）、状态可用/满/封锁、"预计到岸 mm:ss"；不可选时显示原因；选择后确认文案"登陆艇将于 N s 后出发"。
5. 阵营选择页顶部显示 `地图 · 模式` 且 Assault 显示攻方阵营。
6. 无障碍：新增控件 Tab 可达，`aria-label` 来自 i18n。
### 接口与数据
- `interface MapCard { map_id: 'carentan'|'omaha_dog_green'|'foy'; modes: ('frontline'|'assault'|'skirmish')[]; thumb: string; brief_key: string }`；`ServerListItem` 已有字段沿用（`GW` 返回）。

## 不做什么
- 不做模式规则/登陆艇生命周期（M4-011/M4-012）；不做 gateway 编排（M4-031）。
- 不做画质设置页（M4-024）；不做新手引导（M4-027）。
- 不做地图真实缩略图渲染（用线稿占位，素材接入后续）。

## 验收标准
1. 三种模式 × 三张图的进入流程全部可达：Playwright E2E 对 9 个组合（不支持的 Omaha+Frontline 除外，需断言空态文案）各完成"浏览器筛选→加入→阵营选择→加载页"，通过率 100%。
2. 加载页进度准确：与 manifest 字节数比对误差 ≤ 1%；进度单调不回退（断言 100 次采样）。
3. 登陆艇出生界面：模拟 `BoatState` 事件，倒计时误差 ≤ 1 s；选择后不可更换逻辑与 sim 一致（e2e）。
4. i18n 完整性：所有新增 key 在 en/zh 均存在（`pnpm -C client test:i18n` 失败数 = 0）；中文简报无换行溢出（1280×720 与 1920×1080 截图）。
5. 无参考作品名称：对新增 i18n 文案做禁用词扫描（若仓库尚无 `tools/ip-scan`，由本任务以最小脚本新增，词表放 `data/ip_blocklist.txt`，不含在公开文档中）0 命中。
6. HUD/菜单 DOM 节点：服务器浏览器 ≤ 1500、部署界面 ≤ 1500（UI_SPEC §8.1），脚本统计。

## 测试要求
- 单元测试：`vitest`：过滤/排序逻辑、简报选择、倒计时格式化、`FilterState` 序列化（localStorage 不可用）。
- 集成/无头测试：Playwright 9 组合 E2E；`headless-client` 作后端模拟 `BoatState`。
- 性能基准：UI 总 CPU 时间/帧 ≤ 1.5 ms 均值（UI_SPEC §8.1；Playwright trace，记录）。
- 回归：UI 视觉基线（两种语言，`1280×720` 与 `1920×1080`）。

## 性能预算
- UI 预算：脚本+样式+布局+绘制 ≤ 1.5 ms 均值、≤ 3 ms P95（UI_SPEC §8.1）；菜单 DOM ≤ 1500 节点。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 简报文案须经 H-094/H-035 的对外沟通卫生审查；只写已核实史实，不写番号。
- Omaha 为 Assault 主图，Frontline 变体是否上线由 M4-004/M4-031 决定，列表要容忍"该地图只有部分模式"。
- 登陆艇符号为原创设计（H-043 审阅范围），避免与参考作品相似。
