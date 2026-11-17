# M1-003 素材管线：.fpak 分块容器（D-PAK）

## 元数据
- 里程碑：M1
- 工作流：assets
- 规模：L
- 前置依赖：M1-001
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §11.1; ARCHITECTURE §11.2; ARCHITECTURE §11.3; ARCHITECTURE §11.8; ASSET_SHOPPING_LIST §3.4; LICENSES.md §1

## 背景
D-PAK/R11：处理后的素材**不以标准 .glb/.ktx2/.ogg 独立文件暴露**，只以自有分块容器 `.fpak` 下发。这是合规层面的"非独立文件分发"，**不是 DRM**，文档与代码注释不得宣称其"防盗/加密保护"（ARCHITECTURE §11.8；ASSET_SHOPPING_LIST §3.4）。本任务定义格式、写打包器与 Node/TS 读取器；浏览器端读取（Range、Worker 解码）在 M1-006。
格式要点（ARCHITECTURE §11.8）：
- 布局：`magic "FPAK" | version:u16 | entry_count:u32 | index_offset:u64 | [entry data…] | index`；尾部**定长 footer** 含 `index_offset`，使索引可单独 Range 读取。
- 索引每条目：`logical_id`、`kind`、`offset:u64`、`bytes:u32`、`codec`、`sha256[0:8]`。
- 条目为**自有二进制布局**：网格条目 = Meshopt 压缩的 vertex/index buffer + 自定义小头（属性描述、包围盒）；贴图条目 = KTX2 载荷去掉标准文件头后重排 + 自定义头；音频条目见 M1-004。**不含可被通用查看器直接打开的完整 .glb/.ktx2 文件**。
- 分包：按 64 m 区块（与 ARCHITECTURE §9.1 对齐）与类别（`char`、`wpn`、`veh`、`snd`、`ui`…）；每包通常 1–16 MB。
- 发布路径内容寻址：`/a/{sha256[0:2]}/{sha256[2:16]}.fpak`（§11.2）；不使用原始文件名。

## 具体内容
### 要创建/修改的文件
- `docs/formats/FPAK.md` 不新建（避免文档增殖）；格式规范写在 `tools/asset-pipeline/src/fpak/SPEC.md`（代码旁说明，含字节级布局表）
- `tools/asset-pipeline/src/fpak/writer.ts` —— `FpakWriter.add(logicalId, kind, codec, payload)`、`finish()` 返回 Buffer 与索引
- `tools/asset-pipeline/src/fpak/reader.ts` —— `FpakReader`：`readFooter`、`readIndex`、`readEntry(id)`（Node `fs`/`fetch+Range` 两种后端，共享解析代码）
- `tools/asset-pipeline/src/fpak/entry-mesh.ts`、`entry-texture.ts` —— 条目自有布局编解码（解码仅用于往返测试；运行期解码在 M1-006 的 Worker）
- `tools/asset-pipeline/src/pack.ts` —— CLI 子命令 `pack`：读 `work/` 产物 → 分组 → 写 `assets/processed/a/xx/yyyy.fpak`
- `tools/asset-pipeline/scripts/assert-no-standalone.ts` —— 扫描产物目录，断言无 `.glb .gltf .ktx2 .ogg .opus .wav .png .jpg` 独立文件
- `tools/asset-pipeline/test/fpak.*.test.ts`
### 要实现的行为
1. 写入：条目按"大小对齐 16 B"追加；写完生成索引与 footer（footer 定长 32 B：`index_offset:u64 | index_bytes:u32 | entry_count:u32 | fnv/crc32:u32 | magic_end "KAPF"`）。
2. 完整性：`sha256[0:8]` 写入索引；读取时校验（可由选项关闭）；整包 sha256 决定文件名。
3. 读取：先读 footer（最后 32 B）→ 读 index → 对任意条目按 `offset/bytes` 范围读。
4. Range 后端：对 HTTP 服务器用 `Range: bytes=a-b`，失败（200 非 206）降级为整包下载（ARCHITECTURE §11.8 流式读取条款）。
5. 版本：`version:u16 = 1`；读取未知版本明确报错。
6. 禁止泄露原始文件名：`logical_id` 只含逻辑 ID（如 `wpn_us_m1_garand@lod0`），不含源路径/厂商名（ASSET_SHOPPING_LIST §3.4 合规做法 1）。
7. 武器 `_fp`/`_tp` 条目独立（D-FP），可分别 Range 读取。
### 接口与数据
```ts
type Kind = 'mesh'|'texture'|'audio'|'meta'
interface IndexEntry { logical_id:string; kind:Kind; offset:bigint; bytes:number; codec:number; sha8:Uint8Array }
class FpakWriter { add(id:string, kind:Kind, codec:number, payload:Uint8Array):void; finish():{bytes:Uint8Array; index:IndexEntry[]} }
interface RangeSource { read(start:number, endInclusive:number):Promise<Uint8Array>; size():Promise<number> }
class FpakReader { static open(src:RangeSource):Promise<FpakReader>; entries():IndexEntry[]; read(id:string):Promise<Uint8Array> }
```
manifest 中 `pak/entry/offset/bytes`（ARCHITECTURE §11.3）直接取自 `IndexEntry`，由 M1-005 写出。

## 不做什么
- manifest.json 与体积预算门禁：M1-005。
- 音频编码/封装细节：M1-004（本任务只提供 `kind:'audio'` 的容器槽位与 codec 编号）。
- 浏览器侧 Range 加载、Worker 解码、缓存：M1-006。
- 加密/DRM/混淆：**不做**，也不得在文档宣称。

## 验收标准
1. 往返字节一致：对 ≥ 50 个随机/样例条目（含 0 B、1 B、1 MB、16 MB 边界）写入后读回，逐字节比较 100% 一致（`fpak.roundtrip.test`）。
2. 随机 Range 读取：对一个含 ≥ 200 条目的包，用随机条目顺序 + 随机起止位置读取 1000 次，全部与源数据一致（含本地 HTTP 服务器 206 路径）。
3. 损坏检测：翻转任一条目 1 个字节，`read` 抛 `IntegrityError`（单测）。
4. `assert-no-standalone` 对 `assets/processed/` 样例产物目录通过；向目录放入一个 .ktx2 时必须失败（单测）。
5. 条目载荷不被通用工具识别：对网格/贴图条目运行 `file`/glTF 解析器/`ktx info` 均失败（脚本 `assert-opaque`，列出结果）。
6. footer/索引读取开销：对 16 MB 包打开 + 读 1 个条目，网络字节 ≤ 索引字节 + 条目字节 + 64 B（Range 日志断言）。
7. 武器 `_fp`/`_tp` 条目可独立读取且互不依赖（单测）。

## 测试要求
- 单元测试：writer 对齐、footer 编解码、索引序列化边界（长 ID、u64 offset）、版本拒绝、sha 校验、未知 kind 拒绝。
- 集成：本地 `http.createServer` 提供 Range，驱动 `FpakReader`；含"服务器忽略 Range 返回 200"降级用例。
- 性能基准：16 MB 包写入 ≤ 1 s、读取 1000 次随机 Range ≤ 5 s（本机，记录型号）。
- 回归：格式 `version` 变更需同步 SPEC.md 并在 PLAN_ISSUES 登记。

## 性能预算
- 单包 1–16 MB（ARCHITECTURE §11.8）；索引读取为单次 Range；不新增客户端预算（客户端侧见 M1-006）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 诚实声明：玩家仍可从 DevTools 取得 `.fpak` 并自行解析；这只是合规层面的非独立文件分发（H-005 为人类接受该残余风险的事项）。
- 条目自有布局一旦发布即成为兼容负担，务必在 SPEC.md 里写明 `version` 升级策略。
- 本任务不依赖素材到位；用 CC0/自造样例即可。
