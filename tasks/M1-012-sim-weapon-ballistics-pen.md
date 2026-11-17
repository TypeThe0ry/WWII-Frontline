# M1-012 sim：弹道细化与穿透

## 元数据
- 里程碑：M1
- 工作流：sim
- 规模：M
- 依赖：M0-012
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §6.4; GAMEDESIGN §6.7; ARCHITECTURE §6.7.3; ARCHITECTURE §6.10; ARCHITECTURE §15.1

## 背景
M0-012 已实现步枪弹道：段扫描弹体（重力 + 阻力）、命中体、伤害衰减、500 m 下坠与解析值误差 < 3%。本任务做 M1 所需的细化：**阻力参数化**（按弹种 `k_drag`）、**材质穿透表**、**最简跳弹**、**穿透后衰减**。弹道细则（GD §6.4）：
- 服务器权威弹丸，每 tick 线段扫掠；内部子步 4 次/tick（等效 120 Hz），`h = Δt/4`：`a = (0,−g,0) − k_drag·|v|·v`；`v += a·h`；`x += v·h`；寿命 > 4 s 或速度 < 150 m/s 销毁。
- 阻力系数（1/m）：.30-06 0.00075、7.92×57 0.00068、7.92×33 0.00120、.30 Carbine 0.00150、.45 ACP 0.00170、9×19 0.00200、.50 BMG 0.00055。
- 穿透（GD §6.7）：`pen_power`（9×19=60、.45=55、.30 Carbine=90、Kurz=150、.30-06/7.92×57=220–230、.50=520、12 号 00 弹单粒=25）；材质 cost/mm：肉 0.5、树叶/树篱 0.3、玻璃 0.2、薄木/篱笆 1.0、厚木/木门 1.5、砖 3.0、石/混凝土 6.0、土/沙(沙袋) 5.0、薄钢(≤10 mm) 8.0、厚钢 20、雪 2.0、水(≥0.5 m) 15。
- 算法：`P=pen_power`；逐层 `need=t×cost`，`P<need` 则停止，否则 `P-=need`、`D_dealt *= max(0.25, P/pen_power)`、方向偏转 ±1.5°（硬）/±0.5°（软）；**最多穿透 2 层**（⇒ 射线最多 3 段）；身体命中后不再继续（简化），`pen ≥ 150` 且命中非头躯干肢体时"穿身"伤害 ×0.5。
- 伤害衰减 `F0/F1/min` 由 M1-008 数据提供。服务器每弹丸最多 2 次穿透检测。
- 预算：阶段 4 武器与弹道 ≤ 400 活动弹体段扫描，单段约 5 µs，合计 ≤ 2.5 ms/tick（ARCHITECTURE §6.10）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/ballistics/integrate.rs` —— 4 子步积分（`libm`，禁 `f32::powf` 等平台函数）
- `crates/sim/src/ballistics/penetration.rs` —— `trace_with_penetration(origin, dir, ammo, world) -> Vec<TraceSegment>`
- `crates/sim/src/ballistics/ricochet.rs` —— 最简跳弹
- `crates/sim/src/ballistics/materials.rs` —— 材质表（从 `/data/materials.json` 读取）
- `data/materials/surface_materials.json` + schema（cost_per_mm、stop_mm、deflect_deg、`sound_class`/`impact_fx_class`）
- `crates/sim/tests/ballistics_*.rs`；`crates/sim/benches/projectile_step.rs`
### 要实现的行为
1. 弹种参数化：`AmmoDef{k_drag_per_m, mass_g, pen_power, caliber_mm}` 由 M1-008 的 ammo 数据加载；M0 的硬编码阻力移除。
2. 积分：每 tick 4 子步；寿命/最低速度销毁；输出 `ProjectileStep{from,to,v_end}` 供命中检测与曳光（每 5 发 1 发曳光为纯视觉标记）。
3. 材质层识别：射线命中碰撞体的 `material_id` 与厚度（入/出点距离）；对建筑体用其材质 tag，对地形/水面特殊处理（水 ≥0.5 m 深用 cost 15）。
4. 穿透：按上述算法逐层；穿透后伤害系数 `max(0.25, P/pen_power)`；输出每段 `{seg_index, material, remaining_pen, damage_mult}`。
5. 跳弹（最简）：入射角（与表面夹角）< 15° 且材质硬（砖/石/钢）→ 反射，动能 ×0.4，计 1 段；其余按穿透/停止。跳弹后弹丸继续受重力/阻力。**不是**装甲判定（载具走 M1-023）。
6. 伤害：返回 `PenResult{dealt_mult, stopped_at, segments≤3}`；命中结算（部位倍率）由 M1-014 完成，本任务只给穿透后伤害系数。
7. 事件：每段对 `Impact{pos,normal,material}` 产出（供客户端弹着点特效/声音）。
### 接口与数据
```rust
pub struct PenResult { pub segments: [Option<TraceSegment>; 3], pub dealt_mult: f32, pub stopped: bool }
pub fn trace_with_penetration(w: &dyn CollisionWorld, p: &ProjectileState, ammo: &AmmoDef) -> PenResult
```
材质表字段单位：`cost_per_mm`（穿透点/mm）、`stop_mm`、`deflect_deg`。

## 不做什么
- 命中部位/头盔/伤害公式：M1-014；
- 载具装甲判定与 AP/HEAT：M1-023；
- 压制（弹丸擦过检测）：M1-017；
- 爆炸伤害：M1-015；
- 延迟补偿回溯：M1-025；
- 客户端弹着点特效：M1-047。

## 验收标准
1. 阻力积分：Garand 853 m/s、`k=0.00075` 在 500 m 的速度 ≈ 586 m/s、飞行时间 ≈ 0.72 s、下坠 ≈ 2.5 m（GD §6.4 示例），误差 < 3%（`cargo test ballistics_500m`）。
2. 木板（薄木 1.0/mm）厚 30 mm：.30-06（pen 220）穿透后 `dealt_mult ≈ (220−30)/220 = 0.864`，误差 < 1%；9×19（pen 60）穿透后 `0.5`（(60−30)/60），误差 < 1%。
3. 沙袋一层 200 mm（cost 5 → need 1000）：.30-06（220）被完全阻挡，`stopped==true`；.50 BMG（520）同样阻挡（单测）。
4. 砖墙 150 mm（cost 3 → need 450）：.30-06 阻挡；.50 BMG（520）穿透，剩余 70，`dealt_mult = max(0.25, 70/520)=0.25`（单测）。
5. 穿透射线最多 3 段：叠放 4 层薄木板时第 3 层后停止，`segments.len()≤3`（单测）。
6. 跳弹：入射 10° 打砖面跳弹、20° 不跳（边界单测）；跳弹后速度为入射的 ×√0.4 ≈ 0.632（动能 ×0.4）±1%。
7. 性能：`cargo bench -p sim projectile_step`：单段（含 4 子步 + 1 次射线）中位数 ≤ 5 µs；400 弹体一 tick ≤ 2.5 ms（ARCHITECTURE §6.10 阶段 4）。
8. 确定性：含随机偏转（`Rng::for_(tick,entity,purpose)`）的 1 万 tick 回放 `state_hash` 两次一致。

## 测试要求
- 单元测试：每种材质的厚度边界、跳弹角度边界、偏转范围 ±1.5°/±0.5°、`stop_mm` 优先级。
- 集成：`headless-client` 向建筑墙射击，`Impact` 事件材质 ID 正确。
- 性能基准：criterion `projectile_step` 基线入库。
- 回归：M0 的 500 m 误差测试保持通过。

## 性能预算
- 引用 ARCHITECTURE §6.10 阶段 4：≤ 400 活动弹体、单段约 5 µs、合计 ≤ 2.5 ms/tick。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- INDEX 验收写"穿透射线不超 3 段"，与 GD §6.7"最多穿透 2 层"一致（2 层 = 3 段）。
- 跳弹"最简模型"阈值（15°、×0.4）GD 未给，属本任务细化，写入 PLAN_ISSUES；调参标【待调参】。
- 材质表的世界贴图/碰撞体 tag 来源依赖地图管线（M1-034/M1-035）；M1 灰盒建筑按 style 映射到默认材质（砖/木）。
- INDEX 验收"木板/沙袋穿透深度与 /data 一致"：此处以材质表数据（`surface_materials.json`）为准。
