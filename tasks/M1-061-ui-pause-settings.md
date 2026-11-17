# M1-061 UI：暂停菜单与基础设置

## 元数据
- 里程碑：M1
- 工作流：ui
- 规模：M
- 依赖：M1-057
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：UI_SPEC §4.16; UI_SPEC §4.18; UI_SPEC §5; UI_SPEC §5.5; ARCHITECTURE §13.1

## 背景
浏览器中 `Esc` 会强制退出 Pointer Lock 且不可 `preventDefault`，所以 ESC 菜单（UI_SPEC §4.18）设计为"战术间歇面板"，**不暂停多人对局**，明示"游戏仍在进行"。M1 菜单项：继续（重新锁定指针）、重新部署/换兵种（→ M1-056，需确认）、换队（受冷却/平衡限制 → M1-055 阵营页，显示冷却）、设置、离开对局（二次确认）；举报/投票/小队面板入口留灰（M2）。设置（§4.16）M1 基础集：音量（主音量/音效/UI）、鼠标灵敏度（水平/垂直、ADS 倍率）、反转 Y、FOV、按键重绑定基础、画质预设占位（Low/Medium/High，仅写入设置并发 `settings.changed`，真实应用在 M4）、语言切换。热键（§5.2）：默认键位表（`W/A/S/D`、`Left Shift`、`Space`、`C`、`Z`、`Q/E`、`F`、`1–5`、`R`、`X`、`G`、`H`、`M`、`Tab`、`U`、`V/B/N`、`Enter`、`Esc`…）；规则（§5.3）：用 `KeyboardEvent.code`；同上下文冲突实时提示；保留键不可绑定（`Esc/F11/F12/Win/Meta/PrintScreen/Ctrl+W/T/N/Q/Tab`）；不允许 `Ctrl+x/Alt+x`；每动作主键+副键；3 个键位方案槽；导出/导入 JSON。浏览器限制（§5.5）：与浏览器保留键冲突须有提示；失焦（`blur`/`visibilitychange`）时清空按键状态并打开暂停菜单；对局内 `F5`/右键菜单/`Tab` 拦截；Pointer Lock 重锁可能被拒，需用户手势。

## 具体内容
### 要创建/修改的文件
- `client/src/ui/screens/Pause.tsx` —— ESC 菜单，状态机 `Closed/Open/ConfirmLeave/…`（§4.18.3，举报/投票分支 M1 禁用）。
- `client/src/ui/screens/Settings.tsx` —— 左侧分类栏+右侧表单（`SettingsForm` schema 驱动）；M1 分类：音频、控制（含键位）、画质（预设占位）、语言、网络/诊断（显示 ping/丢包/帧率开关、复制诊断包）。
- `client/src/ui/components/KeyBindingRow.tsx`、`SettingsForm.tsx` —— UI_SPEC §9.2 组件。
- `client/src/input/bindings.ts` —— 动作表（ActionId 枚举取 §5.2）、默认绑定、持久化（`localStorage`，try/catch）、冲突检测（同上下文）、保留键拒绝、导出/导入 JSON、3 个方案槽。
- `client/src/input/context.ts` —— 上下文集合 `OnFoot/InVehicle/Gunner/Downed/Map/Menu/Chat`（§5.3）。
- `client/src/input/focus-guard.ts` —— 失焦清键、`beforeunload` 提示、`contextmenu`/`auxclick`/`Tab`/`F5` 拦截、Pointer Lock 丢失→打开暂停菜单。
- `client/src/game/settings.ts` —— 设置存储与 `settings.changed` 事件（ARCHITECTURE §8.4）；默认值与范围取 `/data` 或常量（灵敏度范围等属 UI 偏好，不属游戏性数值）。
- 测试：`client/test/ui/pause.test.ts`、`bindings.test.ts`、Playwright `settings.spec.ts`。
### 要实现的行为
1. `Esc`→`pointerlockchange` 触发打开暂停菜单；"继续"在用户手势内 `requestPointerLock()`；被拒时显示"点击画面继续"覆盖层（z-index 80）。
2. 重绑定：点击行进入"按下新键"态，`Esc` 取消；冲突时红框（`danger`+⚠ 形）+ 三选项（互换/解除对方/取消）；保留键直接拒绝并说明原因（i18n key `ui.settings.controls.reserved`）。
3. 持久化：设置与键位刷新页面后保持；`localStorage` 不可用时（隐私窗口）内存回退并提示一次。
4. 画质预设占位：切换 Low/Medium/High 发 `settings.changed {key:'quality.tier'}`，渲染层仅记录（M4 实现）；M1 同时提供 FOV（60–100°，默认 80°）与灵敏度滑块。
5. 离开对局：二次确认 → 断开连接 → 回服务器浏览器。
6. 菜单打开时世界继续：显示"对局进行中—你的角色仍在战场"。
### 接口与数据
```ts
export type ActionId = 'move_fwd'|'move_back'|'move_left'|'move_right'|'sprint'|'jump'|'crouch'|'prone'|'lean_l'|'lean_r'|'interact'|'fire'|'ads'|'reload'|'fire_mode'|'throw'|'melee'|'map'|'scoreboard'|'squad_panel'|'pause'; // 取 §5.2，后续任务扩展
export interface Binding { primary?: string; secondary?: string }  // KeyboardEvent.code 或 'Mouse0'..'Mouse4'
export function rebind(action: ActionId, slot: 'primary'|'secondary', code: string): { ok: boolean; conflict?: ActionId; reasonKey?: string };
```

## 不做什么
- 不做完整设置（画质细项、语音、界面显隐、辅助功能、隐私，M2-041）、举报/投票/换队完整流程（M2-043）。
- 不做 Keyboard Lock API 的全屏方案（M4 可选）。
- 不做手柄、触屏。
- 不做画质预设真实应用（M4）。

## 验收标准
1. 重绑定持久化：重绑定 `reload` 到 `T`，刷新页面后仍为 `T`（Playwright，`localStorage` 可用路径）；`localStorage` 抛异常时回退内存且不崩溃（vitest 注入异常）。
2. 与浏览器保留键冲突有提示：尝试绑定 `Ctrl+W`、`F11`、`Meta` 均被拒绝并显示 `ui.settings.controls.reserved`（vitest 3 用例；UI_SPEC §5.5）。
3. 同上下文冲突实时检测：将 `fire` 绑到 `Space`（`jump` 默认）→ 显示冲突与互换选项；跨上下文复用（如 `1–5` 在 OnFoot 与 InVehicle）不报冲突（vitest）。
4. 失焦清键：模拟 `blur` 后按键状态全部为 false，且暂停菜单打开（vitest + Playwright）。
5. ESC 流程：Pointer Lock 释放→菜单打开 ≤ 100 ms；"继续"在手势内重锁（Playwright，Chromium）；被拒时出现点击恢复层。
6. 设置变更发出 `settings.changed` 事件，payload 含 key/value（vitest）；FOV 滑块范围 60–100°。
7. 暂停菜单与设置页 DOM ≤ 1 500 节点；打开 ≤ 100 ms（`?uiperf=1`）。

## 测试要求
- 单元测试：绑定表冲突/保留键/方案槽/导入导出（含非法 JSON 拒绝）；设置存储回退；上下文规则。
- 集成/无头测试：Playwright Chromium 流程（ESC、重绑定、刷新）；Firefox/WebKit 仅烟测并记录差异（不阻断）。
- 性能基准：无。
- 回归：M0-022 的 Pointer Lock/输入采样与新 `bindings` 对接后保持通过。

## 性能预算
- UI_SPEC §8.1：菜单 DOM ≤ 1 500；UI CPU ≤ 1.5 ms 均值；本任务无新增预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：ActionId 扩展写 PLAN_ISSUES"细化"
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Pointer Lock 重锁冷却的具体时长未核实（UI_SPEC 附录 B-2）；以实测为准并写 PLAN_ISSUES。
- 各浏览器的 `unadjustedMovement`、键盘布局 API 支持未核实；不可用时降级并提示。
- `localStorage` 仅用于每玩家便利状态，不得存放影响对局的数据。
