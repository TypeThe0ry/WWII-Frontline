# M0-009 地图：M0 单块地形（256 m）数据管线

## 元数据
- 里程碑：M0
- 工作流：maps
- 规模：M
- 前置依赖：M0-001, M0-004
- 素材需求：无（灰盒即可）；素材状态：n.a.（可选 Carentan DEM，见 MAPS §2.1，缺失用程序化）
- 引用文档：MAPS §7.3; MAPS §1.1; MAPS §1.2; MAPS §6; ARCHITECTURE §9.1; ARCHITECTURE §9.5

## 背景
M0 要验证"单块地形"：一块 256 m × 256 m 的可玩区（MAPS §7.3：M0 对应 Carentan 局部 256 m，验证 T-DEM 管线），外加 8 km 远景环。`/tools/map-pipeline`（Python + GDAL，ARCHITECTURE §4.4）此时只出第一版：**默认程序化分形高度图**（无任何外部数据即可跑通）；若 `assets/incoming/maps/carentan/` 下存在 DEM，则走 GDAL 路径裁出 256 m 窗口。产物放 `/data/maps/m0/`，被 `sim`（M0-010 碰撞）、服务器（出生点）、客户端（M0-023 地形）共同使用。
格式取 MAPS §6.1/§6.2 与 ARCHITECTURE §9.1/§9.5：高度图小端 `uint16`，`h_u16 = round((h-hmin)/(hmax-hmin)*65535)`，`hmin/hmax` 写 `meta.json`（MAPS §1.2）；1 m 分辨率 → 256 m 块是 **257×257 样本**；64 m 区块切片每块 **65×65 样本**（相邻块共享边界样本）→ 4×4 = 16 块；远景环 8 m × 1025² 覆盖 8192 m（§9.1）。

## 具体内容
### 要创建/修改的文件
- `tools/map-pipeline/pyproject.toml`、`tools/map-pipeline/map_pipeline/{__init__,cli,procedural,dem,export,chunks,validate}.py`、`tools/map-pipeline/tests/test_*.py`（pytest）。依赖：`numpy`；GDAL（`osgeo`）仅在 DEM 路径按需导入，缺失时该路径给出清晰错误而非崩溃。
- `data/maps/m0/meta.json`（schema `frontline.map.meta/1`：`map_id:"m0"`、`size_m:256`、`height{res_m:1,samples:257,hmin_m,hmax_m,datum:"procedural"}`、`far_ring{res_m:8,samples:1025,extent_m:8192}`、`generator{kind,seed,params,tool_version}`、`sources[]`）
- `data/maps/m0/heightmap_1m.r16`（257² 小端 u16）、`data/maps/m0/heightmap_far_8m.r16`（1025² 小端 u16；中心 256 m 与近景高度连续）
- `data/maps/m0/chunks/b_{bx}_{bz}.bin`（视觉区块：65² u16 高度）与 `data/maps/m0/chunks/c_{bx}_{bz}.bin`（碰撞区块，格式见下），`bx,bz ∈ 0..3`
- `data/maps/m0/spawns.json`（schema `frontline.map.spawns/1`：2 个队出生点，各给 `pos:[x,z]` 与由高度图采样出的 `y`，并含供 bot 使用的 8 个巡逻路点 `patrol_points`）
### 要实现的行为
1. CLI：`python -m map_pipeline m0 --seed 1 --out data/maps/m0 [--dem assets/incoming/maps/carentan/<file>.tif --window E,N]`。无 `--dem` 且无 incoming 数据 → 程序化。
2. 程序化高度：确定性值噪声 fBm（种子化 `numpy.random.Generator(PCG64)`），总起伏 ≈ 0–12 m，**包含一个专门的 > 35° 陡坡区（一道土坎）与一块平坦区**，供 M0-010 的坡度限制测试；起伏参数写入 `meta.json.generator.params`。
3. DEM 路径：`gdalwarp -r average` 重采样到 1 m（MAPS §1.2），裁 256 m 窗口，量化 u16，`hmin/hmax` 写 meta；缺 GDAL 时清晰报错。DEM 到位前此路径只需通过单元测试（合成小 GeoTIFF 用例，GDAL 缺失时 skip）。
4. 区块切片：`b_*.bin` = `[u16 × 65×65]`；`c_*.bin` 碰撞格式（小端）：`magic "FCOL"(4B) | version u8=1 | pad u8 | samples u16=65 | hmin_m f32 | hmax_m f32 | heights u16×65×65 | material_ids u8×64×64（M0 全 0）`；M0 不压缩（Meshopt 压缩属 M1-035）。
5. `validate`：样本数、字节长度、所有区块边界样本与相邻块相等、`hmin ≤ 每个样本反量化值 ≤ hmax`、出生点不在坡度 > 25° 处、坐标全在 0..256；失败退出码 1。
### 接口与数据
- 反量化公式 `h = hmin + u16/65535 × (hmax − hmin)` 写入 `meta.json` 注释字段 `dequant`；`sim`（M0-010）与客户端（M0-023）均按此解码。

## 不做什么
- 不做 splat、植被分布、树篱、建筑、1944 修正、导航网格（M1-032…036）。
- 不做 Carentan 真实数据下载与许可证登记（M1-030/031，人工）。
- 不做 2049² 全图与 1024 块全量（M1/M2）。

## 验收标准
1. `python -m map_pipeline m0 --seed 1` 在无外部数据的干净检出上成功，生成：`heightmap_1m.r16` 大小恰为 257×257×2 = 132 098 B；`heightmap_far_8m.r16` 大小 1025×1025×2 = 2 101 250 B；16 个 `b_*.bin` 各 8 450 B；16 个 `c_*.bin` 各 `16+8450+4096` = 12 562 B（按上面格式核对实测，若不符以格式定义计算值为准并写入 STATUS）。
2. `python -m map_pipeline validate data/maps/m0` 退出码 0；`node tools/data-validate/index.mjs` 对 `meta.json/spawns.json` 通过。
3. 同参数运行两次，所有输出文件的 SHA-256 全部相同（`pytest tests/test_determinism.py`，同机）。
4. 相邻区块共享边界 65 个样本逐一相等（`pytest tests/test_chunk_seams.py`）；远景环中心 256 m 窗口与 `heightmap_1m.r16` 在 8 m 抽样点上误差 ≤ 1 量化步长。
5. 地形中存在 ≥ 1 处连续 ≥ 8 m 长、坡度 > 35° 的坡面，且存在 ≥ 1 块 ≥ 30 m × 30 m、坡度 < 5° 的平地（`pytest tests/test_features.py` 用有限差分检查）。

## 测试要求
- 单元测试：量化往返误差 ≤ (hmax−hmin)/65535/2；窗口裁剪坐标；区块索引；校验器对故意损坏文件（截断、边界不连续、越界出生点）返回失败。
- 集成/无头测试：`/data` 校验 + `cargo test -p sim`（M0-010 起读取这些文件）。
- 性能基准：流水线总耗时（记录）。
- 回归：`meta.json` 与高度图 SHA-256 记入 STATUS，供后续任务核对。

## 性能预算
无（离线工具）。运行时体积参考：近景高度 132 KB + 远景 2.1 MB，与 ARCHITECTURE §9.1 的"远景环 ≈ 2.1 MB"一致。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 只列"257² 高度图与 meta.json"，但 M0-023 要求 8 km 低精度远景环，M0-010 需要碰撞区块；本任务把 `heightmap_far_8m.r16` 与 `c_*.bin` 一并产出，属范围的小幅澄清（在 PLAN_ISSUES 记"细化"）。
- `data/maps/**` 参与 `DATA_HASH`（M0-004），地图重生成会改变握手哈希，这是预期行为；产物 ≈ 2.4 MB，入 git 前确认仓库体积策略（H-041 前保持私有仓库）。
- 地图数据真实来源（DEM、OSM 衍生层许可证）在 M1-030/031 处理；M0 的程序化地形无许可证问题。
