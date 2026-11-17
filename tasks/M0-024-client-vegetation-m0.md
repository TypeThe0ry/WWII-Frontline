# M0-024 client/render：植被实例化与 impostor

## 元数据
- 里程碑：M0
- 工作流：client-render
- 规模：M
- 依赖：M0-023
- 素材需求：无（灰盒即可）；素材状态：n.a.（程序化占位几何）
- 引用文档：ARCHITECTURE §9.6; ARCHITECTURE §13.5.3; ARCHITECTURE §20.1; ARCHITECTURE §13.1

## 背景
E0-5："单块地形 + 植被：RTX 3060 / 1080p ≥ 60 fps；Iris Xe / WebGL2 / 720p ≥ 30 fps（P50，叠层导出）"（ARCHITECTURE §20.1-5、§13.5）。植被是帧率压力的主要来源，必须在 M0 验证 GPU instancing + 八面体 impostor 的路线。
规格（ARCHITECTURE §9.6）：每 64 m 区块一份实例表 `(x:u16, z:u16, species:u8, scale:u8, rot:u8)` = 6 B；每块上限：草 4 096（仅 R_near）、灌木 256、树 192。渲染：①草——GPU instancing，3 档距离 LOD：0–25 m 完整叠片、25–60 m 简化、> 60 m 淡出，风动在顶点着色器；②灌木/小树——网格 LOD0（≈ 300–800 tri）→ LOD1（≈ 120 tri）→ 八面体 impostor（> 80 m）；③树——LOD0（≈ 2–4 k tri，近 30 m）→ LOD1（≈ 800 tri，< 100 m）→ 八面体 impostor（12×12 视角，每视角 128²，atlas 1536²）；全部使用实例化 + `Thin Instances`，**按区块一个 draw call/物种/LOD**。M0 无真素材：几何、贴图全部**程序化占位**（圆锥/圆柱/交叉面片 + 纯色/程序化噪声），命名遵循 ASSET_GAPS §3（树 `PH_V-NAT-002`，草/灌木 `PH_V-NAT-004`）。画质档参数（§13.1）：High 草 100%/45 m；M0 默认取 High 的比例，另给 Low 档（草 35%/20 m，树 impostor 近）用于 Iris Xe 测试。

## 具体内容
### 要创建/修改的文件
- `client/src/render/vegetation/{scatter.ts,species.ts,placeholder-meshes.ts,thin-instances.ts,impostor-bake.ts,impostor-material.{wgsl,glsl}.ts,grass-material.{wgsl,glsl}.ts,wind.ts,vegetation.ts,index.ts}` 与测试。
- `client/src/render/quality-m0.ts`（M0 渲染常量表：`high`/`low` 两档的密度与距离，渲染常量不属于游戏性数值，可留在客户端）。
### 要实现的行为
1. **撒点**（`scatter.ts`，纯函数）：输入 = 区块坐标、`Terrain.heightAt`/坡度、`seed`；输出 = 6 B/条实例表（`Uint8Array`）。规则：避开坡度 > 30° 与出生点周围 6 m；草密度按噪声聚簇；树/灌木泊松盘（最小间距 4 m/1.5 m）；每块不超过 4096/256/192；`seed` + 区块坐标确定性（同种子两次结果字节相同）。运行时生成（M1-035 才由 map-pipeline 烘焙）。
2. **占位几何**：树 LOD0（≈ 2–4 k tri：圆柱干 + 多层锥冠）、LOD1（≈ 800 tri）；灌木 LOD0（300–800 tri）/LOD1（≈ 120 tri）；草为交叉面片束。三角形数写成常量并在测试里断言落在 §9.6 范围。
3. **Thin Instances**：每（区块×物种×LOD）一个 mesh + thin instance 缓冲；LOD 切换用每帧按相机距离重分配实例到不同 LOD 缓冲（CPU 分桶，成本 ≤ 0.5 ms，按区块增量更新而非每帧全量），避免逐实例 draw。
4. **八面体 impostor**：启动时用 `RenderTargetTexture` 从 12×12 视角烘焙一张 1536² atlas（每视角 128²，含颜色 + 法线可选），片元着色按相机方向选最近视角（可选 3 视角混合【待调参】）；WebGL2 与 WebGPU 共用烘焙逻辑；烘焙耗时记录。
5. **风动**：顶点着色器 `sway = amp × sin(t·ω + phase(instance)) × heightFactor`，参数 `uWind{dir,strength,time}`；草与树冠摆动，树干固定。
6. **统计**：叠层显示各类实例数、各 LOD 实例数、draw call、三角形数；`window.__perf.dump()` 增加 `vegetation{drawCalls, instances, tris}`。
### 接口与数据
- `Vegetation.update(camera, dt)`；`Vegetation.setQuality('high'|'low')`；`Vegetation.dispose()`；M1 将替换撒点来源为区块烘焙数据（接口以"实例表字节"为界，不变）。

## 不做什么
- 不做真树模型/贴图（M1-051，V-NAT-*、GAP-V03 树篱）；不做树篱、道具、建筑（M1-051/052）。
- 不做阴影中的植被（alpha-tested 简化网格，M1-043）；不做遮挡剔除（M2-034）；不做碰撞（树干碰撞属 sim，M1+）。
- 不做动态分辨率（M4-018）。

## 验收标准
1. **帧率（E0-5）**：单块地形 + 植被固定机位（M0-030 的 3 个机位）：RTX 3060 / 1080p / WebGPU **≥ 60 fps**（P50）；Iris Xe 级 / WebGL2 / 720p（Low 档参数）**≥ 30 fps**（P50）；数字取自叠层导出 JSON，机型与命令写入 STATUS；缺硬件的项记 `未测`，任务 `partial`。
2. **draw call ≤ 400**（含地形，叠层 `SceneInstrumentation` 数据；两条路径均测）。
3. 撒点确定性：同 `seed` 两次输出逐字节相同；每块实例数 ≤ 4096/256/192；实例表每条 6 B（`scatter.test.ts`）。
4. 三角形数断言：树 LOD0 ∈ [2 000, 4 000]、LOD1 ≈ 800（±20%）、灌木 LOD0 ∈ [300, 800]、LOD1 ≈ 120（±20%）。
5. impostor：从 8 个水平方位在 150 m 处比较"LOD1 网格"与"impostor"渲染的前景像素覆盖面积，差异 ≤ 25%（`impostor-coverage` Playwright 测试，软件渲染可跑）；12×12 atlas 尺寸为 1536×1536。
6. LOD 过渡：相机以 6.4 m/s 径向穿过植被区 30 s，LOD 重分桶耗时 P99 ≤ 0.5 ms，无单帧 > 33 ms 的尖峰（叠层记录）。
7. 命名：材质均为 `PH_V-NAT-*`、节点 `PH_V-NAT-…__{instance}`（单元测试遍历场景断言）；`grep` 代码无外部贴图 URL。

## 测试要求
- 单元测试：撒点规则（坡度/出生点排除/最小间距）、LOD 分桶、风动函数边界、实例表打包。
- 集成/无头测试：Playwright（WebGL2 软件渲染）加载场景无错误；验收 5。
- 性能基准：验收 1/2/6，交 M0-030。
- 回归：无。

## 性能预算
草/树的 GPU 成本包含在不透明主通道 4.0 ms 内（ARCHITECTURE §13.5.2）；draw call 本任务硬指标 ≤ 400（INDEX），整体仍须满足 §13.5.3（WebGPU High ≤ 1 200；WebGL2 Medium ≤ 900；Low ≤ 500）；植被密度/距离参数取 §13.1 的 High/Low。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 参考机缺失是此任务最大阻碍：Iris Xe 级机器与 RTX 3060 见 NEEDS_HUMAN H-021；没有时只能给出本机数字并标 `partial`，**不得**用别的机器数字顶替。
- 本任务的缺失素材需求 ID（回报用）：V-NAT-002（乔木）、V-NAT-004（草/灌木）；地表 V-NAT-007（CC0）属 M0-023。占位材质分别命名 `PH_V-NAT-002`、`PH_V-NAT-004`。
- impostor 烘焙若在 WebGL2 软件渲染环境中过慢，允许 CI 用 `?impostor=lowres`（64² 每视角）并只在参考机测满尺寸。
