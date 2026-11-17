# M1-044 render：TAA、曝光与色调映射

## 元数据
- 里程碑：M1
- 工作流：client-render
- 规模：M
- 依赖：M1-043
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §10.1; ARCHITECTURE §10.2; ARCHITECTURE §10.3; ARCHITECTURE §13.5.2

## 背景
M1 的后处理链：TAA → Bloom + 曝光 + 色调映射 + 颗粒/暗角（ARCHITECTURE §10.1 帧图 9、10 步）。§10.2：Halton(2,3) 抖动 8 相位；历史重投影用运动向量 + 深度；邻域 clamp（YCoCg 方差 clip）；运动向量仅对动态物体与相机重投影，植被风动物体用 `motion_mask` 降低历史权重；自动曝光：直方图 compute（64 bins），WebGL2 用 1 次 mip 链平均。TAAU（超分）与 GTAO/体积雾在 M2，本任务只做**等分辨率 TAA**，但 `render_scale` 接口留给 M2-032。预算：TAA/TAAU 0.8 ms，Bloom/曝光/色调映射 0.7 ms（§13.5.2，High 1080p），M1 要求后处理合计 ≤ 1.5 ms。

## 具体内容
### 要创建/修改的文件
- `client/src/render/post/Taa.ts` + `taa.wgsl`/`taa.glsl` —— Halton 8 相位抖动、运动向量缓冲、历史混合（YCoCg clip）、`motion_mask`、相机剧烈运动/切换时历史重置。
- `client/src/render/post/MotionVectors.ts` —— 动态物体（角色/载具）与相机运动向量；静态几何由相机重投影得到。
- `client/src/render/post/Exposure.ts` —— WebGPU：64-bin 亮度直方图 compute，取 [10%, 90%] 分位平均并指数平滑（适应速度 1.5 EV/s【待调参】）；WebGL2：mip 链平均。
- `client/src/render/post/Bloom.ts` —— 降采样/上采样链（5 级），阈值与强度取设置。
- `client/src/render/post/Tonemap.ts` —— ACES 近似或 AgX（实现任选其一，写入注释），颗粒与暗角可开关。
- `client/src/render/post/index.ts` —— 后处理链总装，`setPostEnabled(flags)`。
### 要实现的行为
1. TAA 开关：关时用 FXAA 兜底（Low 档默认）；TAA 开时相机抖动写入投影矩阵偏移，UI 不受影响（DOM 覆盖层）。
2. 静止场景无闪烁：连续 120 帧静止帧间差异低于阈值。
3. 动态物体拖影抑制：运动向量与邻域 clamp 协同；快速旋转时历史权重下降。
4. 自动曝光：从阴影到日光的亮度跳变在 ≤ 2 s 内收敛，无明显振荡。
5. 双路径：WebGPU 使用 compute；WebGL2 使用片元全屏 pass；输出视觉一致（ΔE < 3）。
### 接口与数据
```ts
export interface PostSettings { taa: boolean; bloom: boolean; exposure: 'auto'|number; tonemap: 'aces'|'agx'; grain: boolean; vignette: boolean }
export function setPostSettings(s: Partial<PostSettings>): void;
export function getFrameJitter(frameIndex: number): [number, number]; // Halton(2,3), 8 相位
```
设置值通过 `settings.changed` 事件（ARCHITECTURE §8.4）驱动，不直接读 UI。

## 不做什么
- 不做 TAAU/超分与动态分辨率（M2-032、M4）。
- 不做 GTAO/SSAO（M2-030）、体积雾（M2-031）、SSR（M4）。
- 不做 UI 层后处理；不做运动模糊/景深。
- 不做色盲滤镜（M2-041）。

## 验收标准
1. 静止场景 120 帧内，相邻帧 TAA 输出的平均绝对差（亮度，0–255）≤ 0.5（`test:visual taa-stability`，固定机位，两路径）。
2. 后处理合计 GPU 耗时 ≤ 1.5 ms（High 参考机 RTX 3060，1080p；GPU timestamp 叠层导出，TAA+Bloom+曝光+色调映射之和）。
3. 曝光适应：阴影→日光跳变场景，亮度在 2 s 内进入稳态 ±5% 且无超调 > 15%（脚本场景 `exposure-step`）。
4. 快速旋转相机（180°/s）时拖影长度 ≤ 3 px（截图测量脚本）。
5. WebGL2 路径后处理 ≤ 2.5 ms（Iris Xe，720p，Low 设置下 FXAA 兜底 ≤ 0.6 ms）。
6. 切换 TAA 开/关不触发材质变体增长（变体计数不变，M1-042 门禁）。

## 测试要求
- 单元测试：Halton 序列前 8 项与解析值一致；曝光直方图分位计算；运动向量重投影数学。
- 集成/无头测试：Playwright 固定机位序列截图，两路径各一套基线。
- 性能基准：GPU timestamp 各 pass 耗时导出到 `reports/m1-post.json`。
- 回归：M1-043 阴影基线需更新（TAA 开启后抖动导致基线变化，更新并记录）。

## 性能预算
- ARCHITECTURE §13.5.2：TAA/TAAU 0.8 ms + Bloom/曝光/色调映射 0.7 ms = 1.5 ms（High，1080p）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、视觉回归）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：色调映射曲线选择记入 PLAN_ISSUES"细化"
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Babylon 自带 TAA 后处理可能不满足 `motion_mask` 与自研 TAAU 的衔接；若采用自研，需保证 M2-032 能复用历史缓冲与抖动接口。
- 抖动与阴影缓存/植被风动的交互易产生"闪烁"，验收 1 必须覆盖植被场景。
