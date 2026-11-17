# M4-022 render：显存与纹理 mip 预算

## 元数据
- 里程碑：M4
- 工作流：client-render
- 规模：M
- 前置依赖：M4-017, M2-036
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §13.3; ARCHITECTURE §13.4; ARCHITECTURE §9.3; ARCHITECTURE §10.4; ARCHITECTURE §11.5

## 背景
浏览器无法精确查询显存，需要**自估计预算 + 区块 LRU 软预算**（ARCHITECTURE §13.4 末段）。预算（§13.3）：
| 档位 | JS 堆 + WASM | GPU 显存 |
|---|---|---|
| Low | ≤ 0.9 GB | ≤ 1.5 GB |
| Medium | ≤ 1.2 GB | ≤ 2.5 GB |
| High | ≤ 1.5 GB | ≤ 4 GB |
| Ultra | ≤ 1.5 GB | ≤ 6 GB |
High 显存分解（§13.4，合计 ≈ 2 940 MB，余量 ≈ 1 060 MB）：渲染目标 170、阴影 52、地形 120、建筑/道具活跃集 1 100、角色/武器 280、载具 380、植被 160、特效/UI 120、IBL/LUT 60、驱动杂项 500 MB。
最大 mip：Low 512²、Medium 1024²、High 2048²、Ultra 2048²（建筑 4096²）（§13.1）。超预算时按 §9.3 驱逐并**降低纹理 mip（先动态调整最大 mip）**；区块缺失时禁止出现"洞"（低分辨率占位）。纹理 mip 尾部分层加载（先载低 mip 后补高 mip，§10.4）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/memory/VramEstimator.ts` —— 自估计器：登记每个 GPU 资源（纹理按 `w×h×bpp×mip 系数`、缓冲、RT）→ 总量与分类账。
- `client/src/render/memory/MipBudget.ts` —— 当前最大 mip 与动态降级阶梯（4096→2048→1024→512）。
- `client/src/world/streaming/EvictionPolicy.ts` —— LRU 驱逐（沿用 M2-036 状态机，增加显存压力输入）。
- `data/render/vram_budget.json` —— 每档 `soft_mb`、`hard_mb`、各分类上限（`initial`，对应 §13.4 表）。
- `client/src/debug/MemoryOverlay.ts` —— 调试叠层：当前估计/预算、分类、最大 mip、驱逐计数。
- `client/tests/render/vram.spec.ts`、`tests/e2e/memory_tour.spec.ts`。
### 要实现的行为
1. 所有纹理/RT/缓冲创建统一经 `GpuResourceRegistry.register(kind, bytes, tag)`；销毁时 `unregister`；估计误差系统偏置通过 `bias` 系数配置。
2. `soft_mb` 超限 → 触发区块驱逐 + 降最大 mip 一阶；`hard_mb` 超限 → 立即降两阶并提示（事件 `memory:pressure`，UI 不在本任务）。
3. 预算回落（< 85% soft）后分步恢复 mip（带迟滞，≥ 10 s 间隔，防抖）。
4. 防"洞"：被驱逐区块立即用上一级 LOD/占位高度采样（§9.3），断言巡游期间可见"洞"= 0（深度/像素检查脚本）。
5. 内存断言：JS 堆 + WASM（`performance.memory`/`measureUserAgentSpecificMemory`，Chromium）按档位上限检查；Safari/Firefox 无此 API 时跳过并标注。
6. 解码缓冲池化（预分配 128 MB 环形缓冲，§13.3）：核对未回归。
### 接口与数据
- `class VramEstimator { register(kind: 'tex'|'rt'|'buf', bytes: number, tag: string): Handle; total(): number; byTag(): Record<string,number> }`；`MipBudget.current(): number`（最大边像素）。

## 不做什么
- 不做真实显存查询（浏览器不支持）；不做驱动层 OOM 检测（沿用 UI_SPEC §4.1 的 device lost 提示）。
- 不改资产体积预算（manifest 门禁在 M1-005）。
- 不做遥测上报（M4-036）。

## 验收标准
1. **15 min 巡游**（三图各 5 min，固定路线，含高密度场景）显存估计峰值 ≤ 档位预算：Low 1.5 / Medium 2.5 / High 4 / Ultra 6 GB（`memory_tour.spec.ts` 输出 `reports/perf/m4-vram-tour.json`，每档 4×3 组；缺硬件仅测软件可得的估计值，实机显存验证标 `未测`）。
2. 无"洞"：巡游期间可见地形"洞"帧数 = 0（深度缓冲为 clear 值且在地形范围内的像素比例 > 0.01% 视为洞）。
3. 压力测试：人为把 `soft_mb` 调到 60%，系统在 ≤ 3 s 内驱逐并降 mip，估计值回落到 soft 内；恢复后 mip 回升（单测 + e2e）。
4. JS 堆 + WASM 峰值：Low ≤ 0.9 / Medium ≤ 1.2 / High、Ultra ≤ 1.5 GB（Chromium 巡游期间采样 P100）。
5. 注册表账目闭合：关卡切换 3 次后，估计总量回到初始值 ±2%（泄漏检测）。

## 测试要求
- 单元测试：`vitest`：估计公式（含 mip 系数 ≈ 4/3）、驱逐顺序（LRU）、迟滞逻辑、注册表泄漏。
- 集成/无头测试：Playwright 巡游脚本；`chrome://gpu`/`performance.measureUserAgentSpecificMemory` 采样。
- 性能基准：估计器开销 ≤ 0.05 ms/帧。
- 回归：预算表哈希写入 `DATA_HASH`；巡游报告基线。

## 性能预算
- 显存与内存预算见上表（§13.3）；High 的分解表（§13.4）为各分类上限的来源；预算超限即缺陷（README §5.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest` / Playwright）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 估计值 ≠ 真实显存（驱动开销、压缩格式差异）；实机验证需要 H-021 的设备，缺失则该项 `未测`。
- 素材为占位时（灰盒），活跃集远小于 1 100 MB，巡游结果不代表真实；真实素材接入（M4-029）后必须复测，建议在 M4-041 重跑。
- BC7/ASTC/ETC2 的 bpp 不同，估计器按实际转码格式计算。
