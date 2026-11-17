# M2-036 world：全图流式与内存预算

## 元数据
- 里程碑：M2
- 工作流：world
- 规模：M
- 前置依赖：M2-005, M2-034
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §9.3; ARCHITECTURE §9.4; ARCHITECTURE §13.3; ARCHITECTURE §13.4; ARCHITECTURE §11.6; ARCHITECTURE §11.8

## 背景
M1-049 实现了区块流式（1/3 区域）。M2 的地图是完整 2048×2048 m、32×32 = 1 024 个 64 m 区块（ARCH §9.1）。ARCH §9.3 流式状态机：三个同心环——`R_phys = 1`（3×3，物理/碰撞）、`R_near = 4`（9×9，完整显示）、`R_mid = 12`（25×25，地形 + 建筑外壳 + 大树）；下载并发 ≤ 6、解码 Worker 池 = `min(4, hardwareConcurrency-2)`、上传每帧 ≤ 1.5 ms 或 ≤ 2 MB；区块缺失显示低分辨率占位（禁止"洞"）；碰撞缺失时玩家不得进入该区块；Resident → Evicting 需离开 `R_near+2` 且超软预算。槽位容量 K = 9×9 = 81（Medium）至 13×13 = 169（Ultra）。
ARCH §11.6 缓存：OPFS 存区块，LRU，默认配额 ≤ 1.5 GB（可设置），`navigator.storage.estimate()` 检查，不足降为仅内存；必须能无缓存运行。ARCH §13.3 内存预算：Medium JS 堆 + WASM ≤ 1.2 GB，High ≤ 1.5 GB；解码缓冲池化（预分配 128 MB 环形缓冲）。§11.8：条目来自 `.fpak` 的 Range 读取，条目级 LRU。

## 具体内容
### 要创建/修改的文件
- `client/src/render/streaming/chunk_streamer.ts`（修改）—— 扩展到全图：环半径随档位（Medium K=81，High 121，Ultra 169，数值取自 `/data/render/streaming.json`）、优先级评分公式（ARCH §9.3 `score`）、部署倒计时对出生点周围 `R_near` 强制最高优先级预取（ARCH §9.4）。
- `client/src/render/streaming/lru_budget.ts` —— 软预算 LRU：CPU 侧与 GPU 侧（自估计）分别记账，超预算时驱逐 `R_near+2` 外最久未用区块并触发 mip 降级。
- `client/src/render/streaming/mip_governor.ts` —— 纹理最大 mip 动态降级（先降建筑/道具纹理，再降地形材质）与恢复（预算回落 10% 后逐级恢复）。
- `client/src/render/streaming/opfs_cache.ts` + `client/src/workers/opfs_worker.ts` —— OPFS 缓存（`FileSystemSyncAccessHandle`，Worker 内）、配额检查、LRU、版本化 key；不可用时降级为内存/重新下载。
- `client/src/render/streaming/hole_detector.ts` —— 调试"洞"检测：相机视锥内地形节点缺高度数据即计数（用于验收与 M2-052）。
- `data/render/streaming.json`、`data/schemas/render_streaming.schema.json` —— 环半径、槽位数、预算（Medium 1.2 GB/Low 0.9/High 1.5）、并发数、上传预算。
### 要实现的行为
1. 全图 1 024 块按距离/视锥/朝向评分排队；"低分辨率占位"用上一级 LOD 高度采样（ARCH §9.3 失败处理）。
2. 内存记账：每个常驻区块记录 CPU/GPU 字节；总量逼近软预算 90% 开始驱逐；超 100% 触发 mip 降级；HUD 调试叠层显示 JS 堆（`performance.memory`，Chromium）与自估计显存。
3. OPFS：区块与 `.fpak` 条目缓存 ≤ 1.5 GB（可配置），LRU 淘汰；`storage.estimate()` 不足、隐私模式、权限异常时全部 try/catch 并降级。
4. 碰撞联动：`R_phys` 环内碰撞数据未就绪时，向预测层暴露 `blockedChunks`（`sim-wasm` 拒绝步入）。
5. 重生/传送后：以出生点为中心重建优先级并预取 `R_near` ≥ 81 块（ARCH §9.4 估算 ≈ 10 MB，100 Mbps 约 1 s）。
### 接口与数据
```ts
export interface StreamingConfig { rNear: number; rMid: number; slots: number; maxConcurrentFetch: number; uploadMsPerFrame: number; uploadBytesPerFrame: number; cpuSoftBudgetMB: number; gpuSoftBudgetMB: number }
export class ChunkStreamer { setConfig(c: StreamingConfig): void; prefetchAround(pos: Vec3, priority: 'spawn' | 'normal'): void; readonly stats: StreamStats }
```

## 不做什么
- 不做四档预设联动、显存精细预算与 Ultra 4096 纹理（M4-017、M4-022）。
- 不做区块内容的烘焙（M2-005）与碰撞数据格式（已定，ARCH §9.5）。
- 不做 Service Worker 更新策略与 CDN 配置（M1-067/M4-033）。

## 验收标准
1. 15 min 脚本化巡游（固定路径覆盖老城、树篱、沼泽、堤道，`tools/camera-tour.ts`，Medium 配置）：JS 堆 + WASM 峰值 ≤ 1.2 GB（`performance.memory` + WASM memory 页数采样，每 5 s 一次）。
2. 同一巡游中"洞"计数（`hole_detector`）累计 = 0（相机视锥内）；区块缺失时占位显示而非空洞（单测模拟延迟下载）。
3. 上传预算：每帧上传耗时 P99 ≤ 1.5 ms 或字节 ≤ 2 MB（调试采样），巡游期间帧时间 P99 ≤ 25 ms（无 > 50 ms 卡顿帧，计数 = 0）。
4. OPFS：二次巡游 OPFS 命中率 ≥ 90%（统计），总占用 ≤ 配额 1.5 GB；禁用 OPFS（模拟异常）时巡游仍可完成，无报错。
5. 驱逐/恢复：人为压低预算触发 mip 降级后，预算恢复时 mip 在 ≤ 5 s 内回升（单测 + 集成）。
6. 出生预取：部署选定出生点后，`R_near` 内 81 块在 100 Mbps 限速下 ≤ 3 s 就绪（Playwright 网络节流，ARCH §9.4 估算 ≈ 1 s，留余量）。

## 测试要求
- 单元测试（vitest）：评分函数；LRU 驱逐顺序；预算记账；mip 降级/恢复状态机；配额不足降级。
- 集成/无头测试：Playwright `camera-tour` 全程 15 min（CI 夜间，PR 跑 3 min 缩短版）；限速场景。
- 性能基准：巡游期间采集帧时间、内存时间序列，输出 `reports/world/streaming_tour.json`。
- 回归：M1-049 的流式测试（环切换、失败重试）继续通过。

## 性能预算
- ARCHITECTURE §13.3：JS 堆 + WASM Low ≤ 0.9 / Medium ≤ 1.2 / High ≤ 1.5 GB；§9.3 上传 ≤ 1.5 ms 或 2 MB/帧；§13.5.1 流式 CPU 1.5 ms；§13.4 显存各项（建筑/道具活跃集 ≈ 1 100 MB 为 High 预算）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright 巡游）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出（本任务无）
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 浏览器无法精确查询显存，GPU 预算用自估计（ARCH §13.4）；`performance.memory` 仅 Chromium，其它浏览器标注未测。
- 灰盒素材体积远小于真素材，真实内存需在 M2-047 接入后复测（M2-052 复核）。
- 区块数据量取决于 M2-005 的实际输出；若 Medium 下 `R_mid` 数据量超过 150 MB 包体预算（ARCH §11.4），写 PLAN_ISSUES。
