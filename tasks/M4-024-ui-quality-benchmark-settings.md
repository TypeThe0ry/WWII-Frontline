# M4-024 UI：画质设置与基准向导

## 元数据
- 里程碑：M4
- 工作流：ui
- 规模：M
- 依赖：M2-041（完整设置）, M4-018
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.16; ARCHITECTURE §13.6; ARCHITECTURE §13.2; ARCHITECTURE §13.1; UI_SPEC §4.1

## 背景
M2-041 已有完整设置页框架；M4-017/M4-018 提供四档预设、动态分辨率与 5 s 基准/推荐。UI_SPEC §4.16 画质分类：**预设 Low/Medium/High/Ultra**；渲染分辨率（内部渲染比例 50–100%）；超分（TAAU/空间放大/关）；帧率上限（30/60/120/无）；垂直同步；FOV（60–100°，默认 80°【待调参】）；画质细项（阴影质量、植被密度/距离、体积雾、反走样、SSAO/GTAO、SSR（仅 Ultra）、纹理分辨率、后处理、粒子数）。修改后预设变"自定义"；不可用项**灰置并显示原因，不隐藏**（§4.16.4）；画质类变更需"应用/还原"，**10 s 内无操作自动还原**（状态机 Viewing→Editing→Previewing→Applied/Reverted）；预设显示**预计显存占用**与参考帧率。
首次运行基准向导：进入固定场景 5 s → 推荐档位，用户可改；**推荐不自动降档**（ARCHITECTURE §13.2）。
隐私：基准结果仅本地，不上传（§13.6、§19）。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/settings/GraphicsPage.tsx` —— 预设选择、细项、预计显存/帧率、应用/还原倒计时。
- `client/src/ui/settings/BenchmarkWizard.tsx` —— 首次运行向导（3 步：说明→5 s 基准→结果与选择）。
- `client/src/ui/settings/DowngradeHint.tsx` —— 接收 `quality:suggest_downgrade`，显示非阻断提示（toast，UI_SPEC §4.19）。
- `client/src/state/graphicsSettings.ts` —— 信号状态，经 `QualityManager`（M4-017）应用；持久化 `localStorage`（try/catch）。
- `client/src/i18n/{en,zh}/settings.gfx.json`。
- `client/tests/ui/{graphics_page,benchmark_wizard}.spec.ts`。
### 要实现的行为
1. 切换预设 → 调用 `QualityManager.setTier`；展示预计显存（M4-022 估计器）；不可用项（WebGL2 下的 SSR/froxel/TAAU 等）灰置并显示原因 key。
2. 单项修改 → 预设显示"自定义"；"恢复默认"回到当前预设值。
3. 应用/还原：修改画质类项后进入 Previewing，10 s 倒计时无确认则还原（`prefers-reduced-motion` 时倒计时环改为文字）。
4. 动态分辨率开关与目标帧率（30/60）；显示当前 `render_scale` 与建议提示。
5. 基准向导：首次进入游戏（设置中无 `gfx.tier` 时）或设置里"重新基准测试"触发；调用 `Benchmark.run()`；结果展示 `avg/P95 ms`、推荐档位与依据（GPU 名称可能被隐藏则显示"未知"）；"采用推荐"/"保持当前"二选一；不自动应用。
6. 档位切换 ≤ 2 s 生效（反馈用 M4-017 事件），期间显示进度而不阻塞。
### 接口与数据
- 订阅 `quality:recommendation{tier,reason}` 与 `quality:suggest_downgrade{tier}`；设置键 `gfx.tier`、`gfx.dynres`、`gfx.fps_cap`、`gfx.fov`、`gfx.overrides{}`（Schema 在 `/data/schemas/ui/settings.schema.json`）。

## 不做什么
- 不做渲染实现、动态分辨率算法（M4-017/M4-018）。
- 不做遥测上报与同意（M4-036）。
- 不做控制/音频/语音设置页（M2-041 已有）。
- 不做自动降档。

## 验收标准
1. 档位切换 ≤ 2 s 生效：Playwright 在 Low↔High 往返 10 次，从点击到 `quality:changed` 事件的 P95 ≤ 2 s。
2. 推荐结果与档位表一致：对合成基准结果表（5 组：iGPU/GTX 1650/RTX 3060/RTX 4070/无 WebGPU）调用 `recommend()`，输出档位等于预期表（单测 5/5）。
3. "应用/还原"：10 s（±0.5 s）无确认自动还原；点击"应用"后立即持久化（e2e）。
4. 灰置项：WebGL2 强制模式下 SSR、froxel、TAAU 控件全部不可编辑且显示原因文案（断言 `aria-disabled` 与原因 key）。
5. 推荐不自动降档：模拟 10 s 触底，档位不变，仅出现一个 toast（单测）。
6. i18n：en/zh key 完整；中文标签不溢出（1280×720）；设置页 DOM ≤ 1500 节点。
7. 可访问性：Tab/方向键/Enter/Space/Esc 可完成全部操作（Playwright 键盘脚本）。

## 测试要求
- 单元测试：`vitest`：设置状态机、自定义检测、还原定时器、`localStorage` 不可用降级。
- 集成/无头测试：Playwright 键盘与鼠标双路径；与 M4-018 的 `Benchmark` 用 mock 驱动。
- 性能基准：打开设置页 UI CPU ≤ 3 ms/帧 P95（UI_SPEC §8.1）。
- 回归：设置页视觉基线（en/zh × 1280×720/1920×1080）。

## 性能预算
- UI 预算 ≤ 1.5 ms 均值（UI_SPEC §8.1）；基准向导运行期间 UI 不得占用 > 0.5 ms/帧以免污染基准。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 浏览器可能隐藏 GPU 信息（UI_SPEC §4.1.1，未核实各浏览器行为），向导必须容忍缺失。
- 预计帧率仅"参考"，不得承诺；文案中使用"参考"字样。
- 若 H-039 改变最低配置，Low 档说明文案与推荐阈值只改数据与 i18n，不改逻辑。
