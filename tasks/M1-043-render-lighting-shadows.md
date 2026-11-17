# M1-043 render：光照、CSM 阴影与天空

## 元数据
- 里程碑：M1
- 工作流：client-render
- 规模：L
- 依赖：M1-042
- 素材需求：V-SKY-001（CC0 HDRI）；素材状态：placeholder（程序化天空 + 渐变，真 HDRI 到位后由 M1-064 或本任务内的配置接入）
- 引用文档：ARCHITECTURE §10.1; ARCHITECTURE §10.2; ARCHITECTURE §9.8; ARCHITECTURE §13.5.2

## 背景
M1 需要可读的户外光照：一盏方向光（太阳）+ IBL + 烘焙探针 + 阴影 + 天空。ARCHITECTURE §10.1：阴影级联 4 级，分割 `[12, 40, 120, 350] m`，分辨率 `[2048, 2048, 2048, 1024]`（High），阴影距离 350 m；静态物体级联 0/1 在相机移动 < 阈值时每 2 帧更新一次（交错，节约 ≈ 0.5 ms）。§10.2：Babylon `CascadedShadowGenerator`（或自定义 CSM），PCF 3×3（Medium）/ 5×5（High+）；植被阴影用 alpha-tested 简化网格；角色阴影只在级联 0–2；光照 = 1 盏方向光 + IBL + 烘焙探针网格（每地图约 800–1500 探针，球谐 L2，存储 ≤ 1 MB）；建筑内部使用探针 + 少量点光（≤ 8 动态）。§9.8：天空用预计算大气散射 LUT（transmittance/multi-scatter 小 LUT，初始化一次）+ 云层纹理滚动；IBL 立方体由昼夜预设烘焙（每地图 2–3 预设，KTX2 立方体 256² + 混合）。WebGL2 路径默认 3 级阴影、分辨率 1024–2048（§10.3）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/lighting/Sun.ts` —— 方向光（方向/颜色/强度来自 `/data/maps/*/map.json` 的光照预设，不硬编码）。
- `client/src/render/lighting/Csm.ts` —— 级联阴影：WebGPU 4 级 / WebGL2 3 级；PCF 核按档位；静态级联交错更新；角色/植被阴影规则。
- `client/src/render/lighting/Ibl.ts` —— IBL 立方体加载与 2 预设混合（KTX2 cube 256²）；缺 HDRI 时回退程序化环境。
- `client/src/render/lighting/Probes.ts` —— 探针网格加载（L2 SH，≤ 1 MB 存储，来自地图包 `probes.bin`）、按位置查询/混合、建筑内部使用。
- `client/src/render/sky/Atmosphere.ts` —— 大气 LUT 预计算（初始化一次，WebGPU compute；WebGL2 片元 pass）与云层纹理滚动。
- `client/src/render/lighting/index.ts` —— 总装与 `setLightingPreset(id)`。
### 要实现的行为
1. 方向光 + CSM：分割/分辨率取 §10.1；WebGL2 降级 3 级（分割 `[15, 60, 250] m`【待调参】）。
2. 阴影接收：地形、建筑、角色、载具、植被；植被投射用简化网格；角色阴影仅级联 0–2。
3. 阴影缓存：静态物体级联 0/1 相机移动 < 0.5 m/帧时每 2 帧更新（阈值【待调参】）。
4. IBL：从 `/data` 光照预设选择，两预设按 `t∈[0,1]` 混合；探针：物体取最近 4 个探针三线性混合（CPU 每帧仅更新动态物体）。
5. 天空：LUT 一次性计算；太阳角度来自预设；云层纹理滚动速度取预设。
6. 缺失素材：无 HDRI 时 `PH_V-SKY-001` 程序化渐变 + 引擎内置大气。
### 接口与数据
```ts
export interface LightingPreset { id: string; sunDir: [number,number,number]; sunColor: [number,number,number]; sunIntensity: number; iblA: string; iblB?: string; blend: number; cloudSpeed: number }
export function setLightingPreset(p: LightingPreset): void;
export function setShadowQuality(tier: 'low'|'medium'|'high'|'ultra'): void;
```

## 不做什么
- 不做 TAA/曝光/Bloom（M1-044）、GTAO（M2-030）、体积雾（M2-031）、水面（M2-035）。
- 不做昼夜循环动画（M1 固定预设）、点光阴影。
- 不做探针烘焙工具（地图管线 M1-035 产出探针数据；本任务只加载）。
- 不做 SSR（M4，仅 Ultra）。

## 验收标准
1. 阴影 pass GPU 耗时 ≤ 2.2 ms（High 参考机 RTX 3060，1080p，固定机位"老城"与"开阔"；GPU timestamp 叠层导出）。
2. WebGL2 路径 3 级阴影可用，截图中三级边界无肉眼可见断层（固定机位截图对比，ΔE 阈值写入基线）。
3. 阴影静态缓存生效：相机静止 120 帧内级联 0/1 重绘次数 ≤ 60（计数器断言）。
4. 探针存储 ≤ 1 MB（加载器断言）；建筑内部与外部光照差异可见（截图对比，室内亮度均值 ≤ 室外 40%）。
5. 大气 LUT 预计算耗时 ≤ 50 ms（一次性，WebGPU）/ ≤ 150 ms（WebGL2）。
6. 光照预设与 IBL 来自 `/data`（grep 渲染代码无硬编码太阳方向常量）。

## 测试要求
- 单元测试：CSM 分割计算；探针混合权重；预设混合插值。
- 集成/无头测试：Playwright 固定 3 机位（开阔/树篱/老城）双路径截图；基线入 `client/test/visual/baselines/`。
- 性能基准：GPU timestamp 采集阴影/天空 pass，结果写 STATUS。
- 回归：M1-042 材质视觉回归仍通过。

## 性能预算
- ARCHITECTURE §13.5.2：CSM ≤ 2.2 ms，天空/雾合成 ≤ 0.3 ms（High，1080p）；探针与 IBL 采样计入不透明主通道 4.0 ms 预算。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、视觉回归）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材已按 ASSET_GAPS §3 命名；缺失素材需求 ID（V-SKY-001）已列出
- [ ] 文档同步：级联分割若调整，回写 ARCHITECTURE §10.1 并记 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Babylon 的 `CascadedShadowGenerator` 在 WebGPU 下的 PCF/稳定性需实测；若不达标改自定义 CSM（写 PLAN_ISSUES）。
- 探针数据依赖 M1-035；未到位时用单一环境球谐占位，验收 4 记 `partial`。
