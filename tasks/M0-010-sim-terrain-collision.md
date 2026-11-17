# M0-010 sim：高度场碰撞

## 元数据
- 里程碑：M0
- 工作流：sim
- 规模：M
- 依赖：M0-009, M0-007
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §9.5; ARCHITECTURE §5.3; ARCHITECTURE §4.3.2; ARCHITECTURE §8.3; MAPS §6.1

## 背景
服务器载入全图碰撞：高度场（Rapier `HeightField`，全图 2049² f32 ≈ 16.8 MB）+ 静态建筑碰撞，按区块组织（ARCHITECTURE §9.5）；客户端 WASM 只载入 `R_phys + 预取圈`（5×5 块）的碰撞（每块 65² f32 ≈ 16.9 KB，总内存 ≤ 5 MB），通过 `Predictor.loadCollision(blockX, blockZ, data)` 喂入（§8.3）。M0 只有 4×4 个 64 m 区块。本任务让 `sim` 具备：区块碰撞数据结构、由 M0-009 产出的 `c_{bx}_{bz}.bin`（格式见 M0-009）构建 Rapier 高度场、地面查询与射线查询 API。KCC 移动在 M0-011，射击弹道在 M0-012，二者都依赖这里的查询。
Rapier 配置：`rapier3d` 开启 `enhanced-determinism`（ARCHITECTURE §5.3；该 feature 与 `simd-*`、`parallel` 互斥，**实现前核对当前 Rapier 文档**）。坡度限制取 35°（INDEX 验收；MAPS §6.2 导航网格亦用 35°）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/Cargo.toml` 增加 `rapier3d`（`default-features=false`，仅 `enhanced-determinism` 与必要 feature；精确版本）。
- `crates/sim/src/collision/{mod.rs,chunk.rs,heightfield.rs,query.rs}`
- `crates/sim/benches/collision.rs`（criterion）、`crates/sim/tests/collision_*.rs`
### 要实现的行为
1. `CollisionChunk{bx,bz,heights_m:Vec<f32>(65²),material_ids:Vec<u8>(64²)}`，`CollisionChunk::from_bytes(&[u8])`：解析 M0-009 的 `c_*.bin`（魔数/版本/尺寸校验，任何不符返回 `CollisionError`，**不 panic**）。
2. `CollisionData`（取代 M0-007 的空壳）：`empty()`、`insert_chunk`、`remove_chunk`（对应客户端卸载）、`from_dir`（仅服务器/测试用，读取 `/data/maps/m0/chunks/`，依赖文件 I/O → 放 `#[cfg(feature="std-io")]` 或交由调用方传字节，**`sim` 本体不做 I/O**，I/O 在 `server`/测试里）。
3. Rapier 构建：每个已载入区块一个 `HeightField` 碰撞体（单位尺度 1 m/样本），放在世界位置 `(bx*64, 0, bz*64)`；块与块之间共享边界样本保证无缝；确定性：按 `(bz,bx)` 升序插入。
4. 查询 API（`WorldView` 也将暴露）：`height_at(x,z)->Option<f32>`（双线性）、`normal_at(x,z)->Option<Vec3>`、`slope_deg_at(x,z)->Option<f32>`、`ground_probe(pos, max_down)->GroundHit{point,normal,slope_deg,material}`、`raycast(origin, dir, max_t)->Option<RayHit{t,point,normal,material}>`、`los(from,to)->bool`（沿线段与高度场相交则 false）、`walkable(x,z)->bool`（slope ≤ 35°，阈值来自 `/data` 的 role.movement 或常量 `MAX_WALK_SLOPE_DEG = 35.0`，在 M0-011 接入 data）。
5. 区块缺失：查询返回 `None`/`false`（不 panic）；"碰撞数据缺失时玩家不得进入该区块"的限制由 M0-011/M0-022 处理（ARCHITECTURE §9.3）。
6. 所有浮点计算只用 `f32` 与 `libm`。
### 接口与数据
- 公开类型与方法如上；`World::new(cfg, &CollisionData)` 沿用 M0-007 的签名。M0-014 将通过 `loadCollision(bx,bz,bytes)` 调用 `CollisionChunk::from_bytes`。

## 不做什么
- 不做角色控制器与移动（M0-011）、弹体扫掠与命中体（M0-012）。
- 不做静态建筑网格碰撞、材质穿透表（M1-012/M1-052）。
- 不做区块的 Meshopt 压缩（M1-035）。

## 验收标准
1. 基准 `cargo bench -p sim collision_raycast`：在 M0 地形上 10 000 条随机射线（固定种子，长度 ≤ 800 m）总耗时中位数 < 5 ms（参考机；写入机型与数字）。
2. 测试 `slope_limit_35deg`：把 Rapier `KinematicCharacterController`（`max_slope_climb_angle = 35°`）直接用于测试，让半径 0.35 m、高 1.8 m 的胶囊以 4.4 m/s 朝 M0-009 地形的 > 35° 陡坡推进 5 s：最终相对起点高度增量 ≤ 坡底以上 0.5 m（无法攀上）；对 ≤ 30° 的坡面同一测试能够攀上（高度增量 ≥ 坡高的 90%）。
3. 反量化精度：对 1000 个随机 `(x,z)`，`height_at` 与由 `heightmap_1m.r16` 直接双线性计算的参考值差 ≤ 1e-4 m。
4. 区块接缝：在 4 个区块交点邻域 1000 个采样点，`height_at` 连续（相邻 1 cm 采样差 ≤ 坡度×1 cm + 1e-4）；缺失区块处 `height_at == None`。
5. 解析器健壮性：对截断/错魔数/错版本/超大 samples 的字节串返回 `Err`，不 panic、不超过 1 MB 分配（`cargo test collision_parse_fuzz` 随机 10^5 例）。
6. 同一 `CollisionData` 构建两次，Rapier 世界内碰撞体顺序与 `state_hash` 贡献相同（确定性单测）。

## 测试要求
- 单元测试：双线性插值边界（块边/角）、法线与坡度（平面、45° 斜面）、`los` 穿山与不穿山各例、`raycast` 从内部发射、命中极远。
- 集成/无头测试：读取真实 `/data/maps/m0/chunks/*`（由 M0-009 产出）的集成测试。
- 性能基准：`collision_raycast`（见验收 1）、`ground_probe`（记录）。
- 回归：无。

## 性能预算
客户端预测内存 ≤ 5 MB（ARCHITECTURE §9.5）：M0 全 16 块 ≈ 16 × 17 KB ≈ 0.3 MB，记录实测；射线批量查询见验收 1。KCC 的 20 µs 预算在 M0-011 验收。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- Rapier `enhanced-determinism` 的互斥特性与 wasm32 兼容性须以安装版本文档为准；若无法在 wasm32 下启用，立即写 PLAN_ISSUES（影响 M0-014/M0-029）。
- 射线实现可用 Rapier `QueryPipeline` 或自写 DDA 步进 1 m 网格；若自写，需与 Rapier 高度场结果交叉验证（测试验收 3/4 覆盖）。选择及理由写进 `collision/mod.rs` 顶部注释。
- 坡度阈值 35° 来自 INDEX 验收；GAMEDESIGN §9 只有 REF M-29 待测，实测后以 `/data` 为准（M0-032 回填）。
