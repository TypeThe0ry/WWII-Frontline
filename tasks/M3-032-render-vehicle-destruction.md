# M3-032 render：载具击毁与残骸

## 元数据
- 里程碑：M3
- 工作流：client-render
- 规模：M
- 前置依赖：M3-029, M2-033, M2-016
- 素材需求：V-PRP-010（残骸，灰盒可）；素材状态：placeholder
- 引用文档：GAMEDESIGN §14.5; GAMEDESIGN §14.9; MAPS §3.7; ARCHITECTURE §13.5.3; ASSET_GAPS §3.2

## 背景
sim 已定义载具击毁规则（GD §14.5）：
- `hull_hp` 归零或殉爆 → **Burning**：非殉爆击毁燃烧 8 s，乘员可逃生（受 20 HP 燃烧伤害），8 s 后爆炸（半径 8 m，Dmax 200），变为残骸（碰撞体保留 60 s，提供掩体）后移除。
- 发动机起火（Engine 被击穿毁且 40%）：火势 15 s，每秒对 hull 20 HP、乘员 5 HP/s；车组灭火器 2 次、每次扑灭 6 s 内的火。
- 殉爆（弹药架）：车辆立即毁、乘员全灭、炮塔弹出（视觉）、爆炸半径 12 m、Dmax 350；概率见 GD §14.5.3。
- 模块损伤（GD §14.5.1）：履带/发动机/炮塔座圈/主炮 ≤ 50% 与 = 0 的效果。
M3-029 已做基础损伤烟（履带/发动机）。本任务做"击毁"完整视觉与残骸实体，并控制开销（残骸数量上限、特效 ≤ 0.5 ms）。MAPS §3.7 提示 Carentan 的性能与制作风险（场景复杂度），残骸是额外实体，必须有上限与回收。

## 具体内容
### 要创建/修改的文件
- `client/src/render/vehicles/DestructionFx.ts` —— 击毁序列：起火（持续火焰粒子 + 黑烟柱）、8 s 倒计时后爆炸、殉爆版本（立即大爆炸 + 炮塔抛飞）。
- `client/src/render/vehicles/WreckManager.ts` —— 残骸实体：由灰盒车体 + 焦黑材质 + 炮塔偏转 + 持续冒烟；上限 `max_wrecks`（默认 12【待调参】），超限时回收最远/最老。
- `client/src/render/vehicles/TurretEject.ts` —— 炮塔抛飞（可选，Low 档关闭）：纯视觉，物理由客户端简单抛物线，不进 sim。
- `client/src/render/vehicles/ModuleDamageFx.ts` —— 模块损伤烟火分级（>50% 无、≤50% 轻烟、=0 重烟/火）。
- `client/src/game/vehicleEvents.ts`（改）—— 订阅 `vehicle.knocked_out`、`vehicle.ammo_cookoff`、`vehicle.module`。
- `client/tests/vehicles/destruction.spec.ts`
### 要实现的行为
1. 收到击毁事件：切到 Burning 状态机 `Alive → Burning(8 s) → Exploded → Wreck(60 s) → Removed`；各阶段时长取自快照/事件，不在客户端硬编码。
2. 乘员弹出表现（逃生乘员由 sim 处理，渲染只播放离车动作占位）。
3. 殉爆：不经过 Burning，直接爆炸；炮塔视觉抛出 15–25 m 弧线（随机但由事件 seed 决定）。
4. 残骸：碰撞体取 sim 事件（客户端不自建碰撞）；渲染材质为焦黑色（`PH_V-PRP-010__{inst}`），残骸 60 s 后淡出（0.5 s）。
5. 车体上的灭火：灭火事件使火焰粒子在 0.3 s 内收束。
6. 视觉降级：Low/WebGL2 档去掉炮塔抛飞与残骸持续烟（仅 5 s）。
### 接口与数据
- 事件：`vehicle.knocked_out{id, cause, cookoff:boolean, pos, turret_yaw}`、`vehicle.fire{id, kind:'engine'|'hull', on:boolean}`。
- 数值常量全部来自 `/data/vehicles/*.json` 的 `wreck.*` 字段（GD §14.5.2 的 8 s/60 s/半径）。

## 不做什么
- 击毁判定、殉爆概率、乘员伤害（sim：M3-013/M3-015）。
- 击毁声音（M3-037）；爆炸冲击波对地形形变（不做）。
- 真残骸模型接入（V-PRP-010，后续素材任务）。

## 验收标准
1. 击毁特效整体 GPU 耗时 ≤ 0.5 ms（单辆击毁 + 2 辆残骸在屏，High 参考机，GPU timestamp；无参考机标"未测"）。
2. 残骸上限：连续击毁 20 辆后，场景中残骸实体数 ≤ `max_wrecks`；被回收的实体 `dispose()` 后 Babylon mesh 数回落（Playwright + `scene.meshes.length`）。
3. 时序：Burning→爆炸间隔 8 s ±0.1 s（事件驱动，vitest 假时钟）；残骸保留 60 s ±0.5 s。
4. 殉爆场景无 Burning 阶段（状态机单测）；炮塔抛飞可由 seed 重现（同 seed 同轨迹，误差 < 1 cm）。
5. 20 辆击毁序列重复 3 次，JS 堆增长 < 5 MB。
6. 两客户端对同一事件序列的残骸位置一致（碰撞体来自 sim 快照，位置差 ≤ 0.1 m）。

## 测试要求
- 单元：状态机转换、seed 可重现、回收策略（最远优先/最老优先）。
- 集成：Playwright `?vis_scene=destruction`（与 M3-048 共用），固定时刻截图。
- 性能：写入 `reports/perf/m3-destruction.json`，供 M3-049。
- 回归：M3-029 载具视觉基线不变。

## 性能预算
- 击毁特效 ≤ 0.5 ms；粒子上限 ≤ 1 600（ARCH §13.5.3）；残骸计入"可见载具 ≤ 12"之外但计入 draw call/三角形预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- `max_wrecks = 12` 为提议初值；M3-049 实测后复核。
- 若 sim 的残骸碰撞体保留 60 s 而渲染提前移除，会出现"隐形掩体"：渲染移除时间必须 ≥ sim 保留时间，若冲突写 PLAN_ISSUES。
- 殉爆炮塔抛飞纯视觉，禁止对 sim 产生影响（P1 服务器权威）。
