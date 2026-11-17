# M1-049 world：区块流式加载

## 元数据
- 里程碑：M1
- 工作流：world
- 规模：L
- 依赖：M1-006（fpak 加载/解码 Worker/缓存）, M1-035（区块烘焙与索引）
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §9.3; ARCHITECTURE §9.4; ARCHITECTURE §9.1; ARCHITECTURE §9.5; ARCHITECTURE §11.5; ARCHITECTURE §13.5.1

## 背景
可玩区 2048 m × 2048 m 按 64 m 切成 32×32 = 1 024 个区块（ARCHITECTURE §9.1）。M1 的 Carentan 1/3 区域约 1180 m × 1180 m ≈ 340 块全精度 + 8 m 远景环。客户端必须按玩家位置流式加载，保证"出生即用"，不出现"洞"，上传不卡帧。本任务实现 `world` 流式系统，M1-050/051/052 在其上挂载地形材质/树篱/建筑。
关键约束（§9.3）：三个同心环 `R_phys = 1`（3×3，物理/碰撞/脚步材质）、`R_near = 4`（9×9，完整显示）、`R_mid = 12`（25×25，地形 + 建筑外壳 + 大树）；状态机 `Unloaded → Requested → Fetching → Decoding → Uploading → Resident → Evicting`，失败重试 ≤ 3 次指数退避；优先级 `score = 1/(d+1) × (in_frustum?1.5:1) × (heading_ahead?1.3:1) × (is_spawn_target?3:1)`；下载并发 ≤ 6，解码 Worker = `min(4, hardwareConcurrency−2)`；上传预算每帧 ≤ 1.5 ms 或 ≤ 2 MB；区块缺失时显示上一级 LOD 的低分辨率占位，禁止出现"洞"；碰撞数据缺失时玩家被限制不得进入该区块（预测会话拒绝步入）。§9.4：部署选定出生点后立即预取其周围 R_near + 碰撞；Medium 每块 ≈ 40 KB（地形）+ 80 KB（道具/植被）→ 81 块 ≈ 10 MB，100 Mbps 约 1 s；本地速度向前预取（heading cone ±45°，外扩 1 圈）。

## 具体内容
### 要创建/修改的文件
- `client/src/render/world/ChunkStreamer.ts` —— 状态机、优先级队列、环管理、预取、驱逐（`R_near + 2` 且超软预算）。
- `client/src/render/world/ChunkIndex.ts` —— 读取 M1-035 的区块索引（`chunks/index`，fpak 内），按 `(bx, bz)` 查询偏移/长度。
- `client/src/render/world/UploadBudget.ts` —— 每帧上传预算调度（≤ 1.5 ms 或 ≤ 2 MB，先到先得）。
- `client/src/render/world/ChunkSlots.ts` —— 高度纹理数组槽位管理（容量 `K = 9×9 = 81`（Medium）…169（Ultra））。
- `client/src/game/collision-feed.ts` —— 把区块碰撞数据（`collision/c_{bx}_{bz}.bin`）送入 sim-wasm（R_phys + 预取圈 5×5）。
- `client/src/render/world/prefetch.ts` —— `prefetchAround(spawnPos)`，供部署界面（M1-056）倒计时调用，并回调进度（`stream.progress` 事件）。
- 测试：`client/test/world/*.test.ts`、Playwright 快速穿越脚本。
### 要实现的行为
1. 环与状态机如 §9.3；Cache API 存清单、OPFS 存区块（经 M1-006 缓存层）；命中缓存跳过网络。
2. 优先级队列：分数每 0.25 s 重算；`is_spawn_target` 用于部署预取（×3）。
3. 预取：部署选点后对出生点周围 R_near 强制最高优先级；游戏中按速度向前预取（±45° cone，外扩 1 圈）。
4. 缺失区块：显示上一级 LOD 的高度采样占位（不渲染洞）；碰撞缺失则通知 game 禁止预测进入。
5. 驱逐：离开 `R_near + 2` 且超软预算（内存/显存自估计）时释放 GPU 资源与槽位。
6. 指标：`stream_resident`、`stream_queue_len`、`stream_upload_ms`、`stream_holes`（调试叠层）。
### 接口与数据
```ts
export class ChunkStreamer { update(playerPos: Vec3, vel: Vec3, frustum: Frustum, dt: number): void; prefetchAround(pos: Vec3, opts?: {ring?: number}): Promise<PrefetchResult>; getState(bx: number, bz: number): ChunkState }
export type ChunkState = 'unloaded'|'requested'|'fetching'|'decoding'|'uploading'|'resident'|'evicting'|'failed';
```

## 不做什么
- 不做地形材质/splat（M1-050）、树篱/植被实例（M1-051）、建筑（M1-052）的具体渲染；只提供"区块资源到达"回调钩子。
- 不做 Hi-Z 剔除（M2-034）、水面（M2-035）。
- 不做服务器侧 AOI（M1-024）；不做 fpak 容器/解码 Worker 本身（M1-003/M1-006）。
- 不做动态分辨率/画质档自动切换（M4）。

## 验收标准
1. 快速穿越测试：以 8 m/s（冲刺）+ 载具 10 m/s 直线穿越 1 000 m，全程可见"洞"（未覆盖像素 / `stream_holes`）= 0（Playwright 脚本 `stream_fast_traverse`，两路径）。
2. 上传预算：全程每帧上传 ≤ 1.5 ms（`stream_upload_ms` 的 P99 ≤ 1.5 ms）或 ≤ 2 MB。
3. 部署预取：R_near（81 块）预取在 100 Mbps、冷缓存下 ≤ 1 s 完成（`prefetch_r_near` 计时，限速 100 Mbps 的本地 HTTP 服务）。
4. 缓存命中后再次进入相同出生点，R_near 预取 ≤ 0.2 s。
5. 失败重试：对 404/超时注入故障，重试 ≤ 3 次指数退避，期间无洞、无异常抛出（`stream_failure_inject`）。
6. 碰撞缺失时 `game` 拒绝预测步入该区块，且服务器权威不受影响（单测 + 无头场景）。
7. 流式状态机+队列的主线程 CPU ≤ 1.5 ms/帧（ARCHITECTURE §13.5.1 流式预算；叠层）。

## 测试要求
- 单元测试：状态机全部转移；优先级公式；驱逐顺序；槽位分配/回收；上传预算计量。
- 集成/无头测试：Playwright 快速穿越；缓存命中/未命中两轮；故障注入。
- 性能基准：`reports/m1-streaming.json`（预取耗时、上传 P99、队列长度）。
- 回归：M0-023 CDLOD 地形在新流式接口下仍无裂缝。

## 性能预算
- ARCHITECTURE §13.5.1：流式（上传分帧 + 状态机）≤ 1.5 ms/帧；§9.3 上传 ≤ 1.5 ms 或 ≤ 2 MB；§11.4 首图 Medium 包 ≤ 150 MB（由 M1-005 门禁，不在本任务）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、Playwright）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）
- [ ] 占位素材无；缺失素材需求 ID 无
- [ ] 文档同步：区块索引格式如有细化，写 PLAN_ISSUES
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L，状态机与槽位/预取易膨胀；若超出，把"预取与碰撞喂送"拆为 `M1-049b`。
- 依赖 M1-035 的区块包与索引；若地图数据仍是程序化高度图（M0-009 兜底），按同接口工作。
- 100 Mbps 限速测试需本地限速代理（脚本放 `tools/dev/throttle.*`）。
