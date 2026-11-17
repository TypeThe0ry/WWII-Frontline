# M1-048 render：坦克渲染

## 元数据
- 里程碑：M1
- 工作流：client-render
- 规模：M
- 依赖：M1-042, M1-041
- 素材需求：V-VEH-US-003（占位可，GAP-V01）；素材状态：placeholder（盒体车体+炮塔盒+炮管圆柱+左右履带盒）
- 引用文档：GAMEDESIGN §14.1; ARCHITECTURE §6.5.2; ASSET_GAPS §3.2; ARCHITECTURE §13.5.3; GAMEDESIGN §14.3

## 背景
M1 唯一的载具是 M4A1 Sherman（`us_m4a1_75`，史实 30.3 t，路速 38 km/h，游戏最高速 = 史实×0.75，GD §14.3）。渲染层需要车体/炮塔/炮管三段独立姿态、履带滚动、乘员位 socket。ASSET_GAPS §3.2：载具占位 = 长方体车体 + 炮塔盒 + 炮管圆柱 + 左右履带盒；炮塔/炮管须可独立旋转/俯仰；尺寸与碰撞取自 `/data/vehicles/*.json`（ARCHITECTURE §8.5.4）。快照：载具 quat smallest-three 压 32 bit，附 4 B 速度；炮塔/炮管角度字段单独（M1-010/M1-041）。可见载具上限 12（§13.5.3），M1 只有 1 辆但按预算设计。GD §14.1：座位 driver/gunner/commander，乘员为独立实体。

## 具体内容
### 要创建/修改的文件
- `client/src/render/vehicles/VehicleRenderer.ts` —— 载具实例管理（按 `vehicle_id`）、插值姿态应用、LOD、阵营着色。
- `client/src/render/vehicles/TankModel.ts` —— 层级：`hull` → `turret`（yaw）→ `gun`（pitch）；socket：`socket_muzzle`、`socket_seat_driver/gunner/commander`、`socket_hatch`、`socket_exhaust`、`socket_track_l/r`、`socket_mg_coax`。
- `client/src/render/vehicles/Tracks.ts` —— 履带滚动：UV 偏移速度 = 左右履带线速度（来自载具速度与差速转向）；轮子/驱动轮旋转。
- `client/src/render/vehicles/PlaceholderTank.ts` —— 程序化占位（`PH_V-VEH-US-003__sherman`），尺寸取自 `/data/vehicles/us_m4a1_75.json`（含 `dimensions_m` 字段；缺则补并注来源）。
- `client/src/render/vehicles/Damage.ts` —— 简易损伤视觉：履带断（模块损坏）、起火烟（调用 M1-047）、击毁黑化；数据来自 `Module` 状态位。
- 测试：`client/test/render/vehicle.test.ts`、Playwright 截图。
### 要实现的行为
1. 车体姿态：由插值器（M1-041）提供位置与 quat；乘员视角下用预测/平滑值。
2. 炮塔/炮管：yaw/pitch 由服务器权威角度驱动，插值；炮手本地视角使用预测角（M1-041）；屏幕上炮塔朝向与 sim 一致 ≤ 1°。
3. 履带滚动速度与车速匹配：履带纹理滚动速度 = 车体沿前向速度 ±（转向差速项）；停车时静止。
4. 乘员位：座位 socket 供摄像机与 M1-045 的乘员姿势使用；M1 仅在座位处显示乘员占位（头球）或隐藏。
5. 炮口焰/炮口烟在 `socket_muzzle` 触发 M1-047 预设（炮类）。
6. 损伤视觉：模块状态改变时发事件，触发烟/火占位。
### 接口与数据
```ts
export interface VehicleView { id: number; type: number; pos: Vec3; quat: Quat; vel: Vec3; turretYaw: number; gunPitch: number; moduleBits: number; team: 0|1 }
export class VehicleRenderer { sync(views: ReadonlyArray<VehicleView>, alpha: number): void; getSocket(vehicleId: number, name: string): Mat4 | null }
```

## 不做什么
- 不做载具预测/座位同步（M1-041）、载具 UI（M1-060）。
- 不做其它载具（M3-029）、内饰与车内视角细节（GAP-V01，M3）。
- 不做模块损伤的细致贴图/物理碎片；不做履带地形接触印痕。
- 不做真 Sherman 模型的减面与接入（M1-064）。

## 验收标准
1. 炮塔朝向与 sim 权威值差 ≤ 1°（headless 回放对比：1 000 帧采样，最大偏差 ≤ 1°，`vehicle_turret_sync`）。
2. 履带滚动速度与车速匹配：直线 20 km/h 时履带纹理滚动线速度 = 车速 ±5%（脚本读取 uniform 对比）。
3. 占位 Sherman 尺寸与 `/data/vehicles/us_m4a1_75.json` 误差 < 1%（单测）。
4. 单辆坦克渲染 GPU ≤ 0.3 ms（High，1080p，近景；叠层导出）。
5. 占位节点命名符合 `PH_{需求ID}__{实例名}`，且全部 socket 存在（遍历断言）。
6. 炮口焰在开炮事件后 ≤ 1 渲染帧出现。

## 测试要求
- 单元测试：层级变换、履带速度公式、socket 查询。
- 集成/无头测试：Playwright 固定机位（堤道直线、转弯）截图。
- 性能基准：叠层导出载具 pass 到 `reports/m1-vehicle.json`。
- 回归：M1-045 角色与 M1-047 VFX 不受影响。

## 性能预算
- ARCHITECTURE §13.5.3：可见载具 ≤ 12（High）；载具三角形计入主视图 ≤ 4.0 M；M1 单车预算 ≤ 150k 三角形（真素材规格，ASSET_SHOPPING_LIST §4.0 中型载具 60k–150k）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、视觉回归）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID（V-VEH-US-003）已列出
- [ ] 文档同步：`dimensions_m` 等数据字段回写 schema 并记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 真素材候选（CGTrader Sherman）面数可能 > 60 万，需 M1-002 减面；许可证需书面确认（见 ASSET_SHOPPING_LIST V-VEH-US-003，H-049）。
- 内饰与车内视角属 GAP-V01，M1 不要求。
