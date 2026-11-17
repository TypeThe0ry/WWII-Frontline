# M1-050 world：地形 splat 材质与宏观色

## 元数据
- 里程碑：M1
- 工作流：world
- 规模：M
- 依赖：M1-049, M0-023（CDLOD 地形）, M1-042（PBR 材质系统）
- 素材需求：V-NAT-007（CC0 地表材质）；素材状态：placeholder（棋盘格/纯色）→ real（CC0 免费，应尽快换真，ASSET_GAPS §3.2）
- 引用文档：ARCHITECTURE §9.2; ARCHITECTURE §9.1; ARCHITECTURE §13.5.2; MAPS §6.1

## 背景
M0-023 的地形用程序化着色。M1 需要真实的地表观感与可读性：splat 权重 + 8 层材质数组 + 宏观色彩图 + 陡坡 triplanar（ARCHITECTURE §9.2：材质 = splat 权重 + 8 层材质数组（KTX2 UASTC，1024² 每层，ASTC/BC7 取决于设备）+ 宏观色彩图（4096² 全图一张，KTX2）+ 法线细节；近距离 triplanar 用于陡坡）。高度纹理为 texture array，层 = 区块槽位（M1-049 的 `ChunkSlots`）。区块文件含 splat 64×64×4 权重 8-bit（§9.1）。MAPS §6.1：`splat_0/1.png` 最多 8 层。地表材质是"唯一免费即达标"的品类（Poly Haven/ambientCG CC0，ASSET_GAPS §3.2），不应长期占位。

## 具体内容
### 要创建/修改的文件
- `client/src/render/terrain/TerrainMaterial.ts` + `terrain.wgsl`/`terrain.glsl` —— 地形着色器：双 RGBA splat（8 层）混合、宏观色调制、法线/ORM 采样、triplanar（坡度 > 40°【待调参】时启用）、距离淡出到宏观色；挂入 M1-042 的 PBR 光照模型（不新增宏变体，使用地形专用变体并计入 ≤ 60）。
- `client/src/render/terrain/MacroColor.ts` —— 宏观色图（4096² 全图一张，KTX2）加载与采样；远景环采样。
- `client/src/render/terrain/LayerArray.ts` —— 8 层材质数组（1024² KTX2 UASTC）加载；缺省时棋盘格 `PH_V-NAT-007__layerN`。
- `client/src/render/terrain/SlotSplat.ts` —— 区块 splat 权重纹理数组与槽位对齐（与 `ChunkSlots` 同步上传，走 M1-049 的上传预算）。
- `client/src/render/terrain/FootstepMaterial.ts` —— 查询脚下材质 ID（供 M1-063 脚步声）：由 splat 最大权重层映射到材质类别。
- 数据：`/data/maps/carentan/terrain_layers.json`（层 ID → 材质名/平铺尺度/脚步类别；放地图数据，不硬编码）。
- 测试：`client/test/render/terrain-material.test.ts`、Playwright 陡坡/平地/远景截图。
### 要实现的行为
1. splat 双纹理 8 通道归一化混合；平铺尺度每层独立；防止重复感：使用 2 种尺度叠加。
2. 近距 triplanar 仅在坡度 > 阈值时混入，避免拉伸；远处回退宏观色。
3. 区块边界无接缝（splat 采样边界对齐，使用槽位相邻采样或 1 px 重叠）。
4. 材质数组按设备选择 ASTC/BC7/ETC2（来自 KTX2 转码选择，M1-006）。
5. WebGL2 路径同效：使用 2D 纹理数组（WebGL2 支持）；分层采样次数在 Low 档减少（≤ 4 层混合）。
### 接口与数据
```ts
export interface TerrainLayerDef { id: string; albedo: string; normal: string; orm: string; tileScale: number; footstep: number }
export class TerrainMaterial { setLayers(defs: TerrainLayerDef[]): Promise<void>; setSlotSplat(slot: number, splat: Uint8Array): void; getSurfaceAt(x: number, z: number): number }
```

## 不做什么
- 不做区块流式本身（M1-049）、树篱/植被（M1-051）、建筑（M1-052）。
- 不做地形置换/曲面细分、水面（M2-035）、体积雾（M2-031）。
- 不做地表贴花（弹坑/车辙，M2-033+）。
- 不做地图 splat 规则生成（M1-035）。

## 验收标准
1. 地形材质 GPU ≤ 1.5 ms（High 参考机，1080p，"开阔"机位；GPU timestamp 叠层；INDEX 指标）。
2. 陡坡（≥ 45° 的人工检查点）无明显拉伸：人工截图审阅通过（附 3 张截图入 `docs/` 外的 `reports/m1-terrain/`；标"需人工"，未测则 partial）。
3. 区块边界无接缝：相邻区块交界处亮度突变 ΔL ≤ 2（图像差分脚本，3 个机位）。
4. 8 层材质总显存 ≤ 24 MB（UASTC 1024²×8 层含 mip，加载器断言）。
5. 材质变体计数不超过 M1-042 门禁的 60（CI）。
6. `getSurfaceAt` 返回层映射与 splat 最大权重一致（1 000 随机点单测）。

## 测试要求
- 单元测试：splat 归一化、triplanar 权重曲线、层平铺、脚步类别映射。
- 集成/无头测试：Playwright 三机位（开阔/陡坡/区块交界）截图，两路径基线。
- 性能基准：地形 pass GPU 耗时导出到 `reports/m1-terrain.json`。
- 回归：M0-023 CDLOD 无裂缝测试保持通过。

## 性能预算
- ARCHITECTURE §13.5.2：地形计入不透明主通道 4.0 ms；INDEX 要求地形材质 ≤ 1.5 ms；§13.4 显存预算（High 4 GB）内。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`vitest`、视觉回归）；`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_V-NAT-007__*`）；缺失素材需求 ID 已列出
- [ ] 文档同步：`terrain_layers.json` 格式写入 PLAN_ISSUES"细化"
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- CC0 材质（Poly Haven/ambientCG）免费且可登记 OK，但须先由人类放入 `assets/incoming/` 并登记 `LICENSES.md`（H-008 领取免费素材、H-023/H-049 采购批）；未到位用棋盘格，不阻塞。
- 若地形材质超 1.5 ms，优先降低 Low/Medium 的混合层数而非改架构。
