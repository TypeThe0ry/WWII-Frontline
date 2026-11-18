# M1-046b：第一人称武器 pass 实测（工作站 RTX 3080，PI-109）

## 做了什么
- `render/fp/fp-pass.ts` **`FpWeaponPass`**：武器用单独的相机渲染（`layerMask` = `FP_LAYER`，世界相机排除该层），`scene.activeCameras = [世界相机, 武器相机]`，第二个相机渲染前清深度；武器相机的 FOV 固定为数据里的 `view.hip_fov_deg`，位置/朝向每帧从世界相机复制。所以世界 FOV（设置 40–110°、ADS 变焦）不会改变武器在屏幕上的大小（`fp_fov_independent`），武器也不会穿进几何体。`ActorManager` 默认创建（`?fp_pass=0` 关闭）。
- **贴墙回缩（`wall_pushback` 接线）**：`FirstPersonRig.wallProbe` 沿视线射线探测，枪口socket 相对相机的前向距离 = 武器偏移 + `muzzle.position.z`，按 `wallPushback`（WALL_MARGIN_M = 5 cm）后撤；世界场景里用建筑网格拾取（`?scene=world`，`metadata.blocks`；树篱 thin-instance 不参与）。
- **`ads.state` 总线事件**：hip → entering → ads → leaving → hip（`FirstPersonRig.onAds` → `GameEvents['ads.state']`）。
- 占位枪名 `PH_*__fp` 与 socket 原本就有（M1-046a 之前已做），这里把它们挂到武器层。

## 实测
- **贴墙（真实建筑墙，`?local=` 把玩家放在 0.45 m 外朝向墙面，WebGPU 与 WebGL2 一致）**：射线测得墙距 0.4495 m（构造值 0.45），回缩后枪口在视线方向上距相机 0.3996 m = 0.4495 − 0.05（5 cm 余量，误差 < 0.1 mm）；空旷处枪口距相机 0.909 m（无回缩）。
- 单测 `fp-pass.test.ts`（4 项）：武器网格都在武器层、世界相机排除、两个 active camera；世界 FOV 在 40/60/90/110° 时武器相机 FOV 不变，武器相机跟随世界相机位姿；墙距 0.1/0.25/0.5/0.8/1.2/2 m 时枪口前伸 ≤ 墙距 − 5 cm（≥0.5 m 时仍 > 0.3 m）；`ads.state` 顺序。
- **开销（`?fp_pass=0` 对照，old_town 1080p，2 次交替，整帧 GPU / CPU P50 ms）**：WebGPU 有 pass 0.175 / 0.205 GPU、4.7 / 4.5 CPU，无 pass 0.233 / 0.180 GPU、3.9 / 3.6 CPU——**GPU 在噪声内，CPU 约 +0.6–0.9 ms**；WebGL2 有/无 0.12–0.15 / 0.12 GPU，CPU 3.3–3.8 / 3.5–3.6（无可测差）。任务卡没有毫秒阈值，也没有把 FP pass 单独计时，所以只给整帧数字。

## 没做
`FpAnimator`（装填/拔枪/奔跑动画）与开火/装填事件的帧同步断言（装填占位动画没有）、树篱不参与贴墙探测（thin-instance 拾取未开）、武器数据 `length_m` 字段（/data 里没有，用占位枪的 socket 距离）、Playwright 截图基线（只做了运行时截图人工检查）。
