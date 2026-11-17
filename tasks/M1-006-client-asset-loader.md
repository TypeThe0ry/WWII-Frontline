# M1-006 client：fpak 加载、解码 Worker 与缓存

## 元数据
- 里程碑：M1
- 工作流：client-render
- 规模：L
- 依赖：M1-003, M0-020
- 素材需求：无（灰盒即可）；素材状态：n.a.（本任务含"占位回退"）
- 引用文档：ARCHITECTURE §11.5; ARCHITECTURE §11.6; ARCHITECTURE §11.8; ARCHITECTURE §9.3; ARCHITECTURE §8.4; ASSET_GAPS §3.1; ASSET_GAPS §3.3

## 背景
客户端资源加载器：把 M1-003 的 `.fpak` 变成 Babylon 可用的对象，且**不卡主线程**。ARCHITECTURE §11.5：先取 `.fpak` 索引（footer→index），按 manifest 的 `offset/bytes` 用 HTTP Range 流式读；Worker 池解码 KTX2（BasisU transcoder WASM）与 Meshopt；主线程只做 GPU 上传（分帧）；`stream.progress` 事件（§8.4：`{ loaded, total, phase }`）驱动加载屏。
关键数字：
- 解码 Worker 池 = `min(4, hardwareConcurrency-2)`（§9.3）；下载并发 ≤ 6；
- 上传预算：每帧 ≤ 1.5 ms 或 ≤ 2 MB（先到先得，§9.3）；
- 缓存（§11.6）：Cache API 存 `core`/manifest/小资产（key `v{game_version}`）；OPFS 存大文件（`FileSystemSyncAccessHandle` 在 Worker 内），LRU，默认配额 ≤ 1.5 GB，`navigator.storage.estimate()` 不足则降为仅内存；**必须能无缓存运行**；
- 失败：超时/404 重试 ≤ 3，指数退避；缺失资源用占位（`PH_{需求ID}` 灰盒，ASSET_GAPS §3.1）；音频缺失返回 0.1 s 静音 `AudioBuffer`，每需求 ID 每会话一条日志（§3.3）。
本任务不负责区块流式状态机（M1-049），但提供其所需的"取条目→解码→上传"原语。

## 具体内容
### 要创建/修改的文件
- `client/src/render/assets/fpak-reader.ts` —— 浏览器版 `FpakReader`（`fetch` + Range；与 M1-003 共享布局常量，通过 `tools/asset-pipeline` 导出的解析函数或复制的最小解析器，不得 import Node 专用 API）
- `client/src/render/assets/manifest.ts` —— 加载并缓存 manifest；`resolve(logicalId, tier)` → `{pak, offset, bytes}`
- `client/src/render/assets/decode-worker.ts` —— Worker：Meshopt 解码、KTX2 转码（BasisU transcoder WASM，按设备选 BC7/ASTC/ETC2/RGBA 回退）、返回 Transferable ArrayBuffer
- `client/src/render/assets/worker-pool.ts` —— 池与任务队列（优先级、取消）
- `client/src/render/assets/upload-scheduler.ts` —— 每帧上传预算（1.5 ms/2 MB）与队列
- `client/src/render/assets/cache.ts` —— `CacheLayer`：Cache API、OPFS（Worker 内同步句柄）、内存 LRU；特性探测与降级
- `client/src/render/assets/placeholder.ts` —— 灰盒回退工厂：`PH_{reqId}` 材质、`PH_{reqId}__{instance}` 节点；开发构建叠加需求 ID 标签
- `client/src/render/assets/index.ts` —— 对外 API；事件 `stream.progress`
- `client/test/assets/*.test.ts`（vitest；Worker 用 `vitest` + 内存假实现）
### 要实现的行为
1. `loadMesh(id, lod)` / `loadTexture(id)` / `loadAudioBytes(id)` 返回 Promise；同一条目并发请求去重。
2. Range 读取：`Range` 请求返回 200（服务器忽略）→ 降级整包下载并缓存整包；206 时按条目读。
3. 解码全部在 Worker；主线程仅接收 Transferable 并排队上传；上传按预算分帧，超预算的推迟到下一帧。
4. 缓存：命中 Cache/OPFS 跳过网络；OPFS 不可用（隐私模式/无权限/配额不足）→ 仅内存运行，不报错，设置 `cacheMode:'memory'` 并 emit 信息事件。
5. 失败处理：重试 3 次（200/400/800 ms 退避）；仍失败 → 返回占位并记日志；区块缺失由 M1-049 处理，本任务只返回错误对象。
6. `stream.progress`：阶段 `download|decode|upload`，`loaded/total` 以字节计；节流 ≤ 10 Hz。
7. 取消：区块离开预取半径时可取消未完成下载与解码。
8. 音频：`loadAudioBytes` 只取字节，`decodeAudioData` 探测与解码由 M1-062；缺失返回静音（日志每需求 ID 每会话 1 条）。
### 接口与数据
```ts
interface AssetLoader {
  init(opts:{manifestUrl:string; tier:'low'|'medium'|'high'|'ultra'}): Promise<void>
  loadMesh(id:string, lod:number, prio?:number): Promise<MeshData|Placeholder>
  loadTexture(id:string, prio?:number): Promise<GpuTextureHandle|Placeholder>
  loadBytes(id:string): Promise<Uint8Array>
  cancel(token:LoadToken): void
  stats(): {cacheMode:'opfs'|'cache'|'memory'; inflight:number; hitRate:number}
}
```
事件：`stream.progress { loaded:number; total:number; phase:'download'|'decode'|'upload' }`（ARCHITECTURE §8.4）。

## 不做什么
- 区块流式状态机、环半径、碰撞数据装载：M1-049。
- PBR 材质系统与着色器：M1-042。
- 音频播放与 `decodeAudioData` 探测实现：M1-062。
- 资源打包/manifest 生成：M1-003/M1-005。
- 真素材接入：M1-064/M1-065。

## 验收标准
1. 解码期间主线程长任务（> 50 ms，`PerformanceObserver('longtask')`）数量为 0（Playwright 加载 ≥ 100 MB 样例包过程中采集，报告写 STATUS）。
2. OPFS 不可用（以 `--disable-features` 或 mock 关闭）时仍能完整加载并运行，`cacheMode==='memory'`（vitest + Playwright 各 1 例）。
3. 上传预算：逐帧采样的上传耗时 P95 ≤ 1.5 ms 且单帧上传字节 ≤ 2 MB（叠层/`performance.now` 打点；样例场景加载 200 个小条目）。
4. 缓存二次加载：同一 `.fpak` 第二次加载网络请求数 = 0（Playwright `route` 计数），冷启动 → 热启动加载时间下降 ≥ 50%（记录两次耗时）。
5. 故障：对 404 条目返回占位，重试次数恰为 3（vitest 断言），节点/材质命名符合 `PH_{需求ID}` 规范（正则断言）。
6. Range 降级：服务器返回 200 时仍正确读取条目（vitest，对照 M1-003 的 fixtures）。
7. `stream.progress` 发布频率 ≤ 10 Hz，且 `loaded` 单调不减（vitest）。
8. 缺失音频 ID 在同一会话内重复请求 100 次只产生 1 条日志（vitest）。

## 测试要求
- 单元测试：manifest.resolve、worker-pool 调度/取消、upload-scheduler 预算算法、cache LRU 与降级、重试退避、去重。
- 集成/无头：Playwright（Chromium）加载本地 fixtures；`longtask` 采集；OPFS 开/关两种。
- 性能基准：加载 100 MB 样例的总耗时与各阶段占比，写入 `reports/m1-asset-loader.json`。
- 回归：无视觉基线；manifest schema 变更需同步。

## 性能预算
- 引用 ARCHITECTURE §9.3：上传 ≤ 1.5 ms/帧 或 ≤ 2 MB/帧；§13.3 内存：JS 堆 + WASM ≤ 1.5 GB（缓存内存 LRU 软预算按 §13.4 档位）；OPFS 默认 ≤ 1.5 GB。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 规模 L 且含 KTX2/Meshopt/OPFS/Worker 四块，容易超一次会话；建议先做 fpak-reader + manifest + 缓存降级 + 占位，KTX2 转码细节若超出可按 README §5.6 追加 `M1-006b`。
- BasisU transcoder WASM 体积 ≈ 0.4 MB br（ARCHITECTURE §11.4 core 预算已含），其来源/许可证须登记 LICENSES.md §5。
- `FileSystemSyncAccessHandle` 仅 Worker 内可用；Safari/Firefox 支持度未核实，降级路径必须测。
