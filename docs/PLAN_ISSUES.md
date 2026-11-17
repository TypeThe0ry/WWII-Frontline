# 计划问题登记（PLAN_ISSUES.md）

> 用途：执行任务时发现的**计划、架构、数值、素材、预算**层面的问题登记处。
> **执行者不改架构文档**（`ARCHITECTURE.md`、`GAMEDESIGN.md`、`UI_SPEC.md`、`MAPS.md`、`REFERENCE_MEASUREMENTS.md`、`MILESTONES.md`、`tasks/INDEX.md` 的结论/接口/预算/章节编号）；遇到问题在本文新增条目，在任务备注引用编号，并按现有文档继续做能做的部分。
> 审阅：Fable 在每个里程碑的审阅检查点（`docs/MILESTONES.md` §0.4）逐条裁决；采纳的修订由 Fable 写回对应文档，并在本条目写明"已回写到哪个文档/章节/日期"。
> 规则：只追加，不删除；执行者不得改写他人条目；已裁决条目保留作为决策记录。

---

## 1. 条目模板

复制下面模板到 §3 的条目列表末尾（编号 `PI-{三位序号}` 顺延）：

```markdown
### PI-{NNN} {一句话标题}

- **状态**：open | accepted | rejected | resolved | deferred
- **类别**：架构 | 玩法数值 | 协议/数据 | 渲染/性能 | 素材/许可证 | 地图/数据源 | 测试/运维 | 计划/任务粒度 | 文档冲突 | 其它
- **提出者 / 日期**：{任务 ID 或人名} / {YYYY-MM-DD}
- **问题**：{发生了什么、哪个文档/章节的要求做不到或相互冲突；一两段，具体到章节号与数字}
- **证据**：{实测数字与命令、日志/报告路径、失败用例、截图路径；"未核实"的外部事实要明说}
- **建议方案**：{至少一个可执行方案；有多个时列出优缺点与推荐项；注明是否需要改哪份文档的哪一节}
- **受影响任务**：{任务 ID 列表；已受影响的状态（partial/blocked）}
- **临时处理**：{在裁决前执行者采取的做法，如"按原文档实现并标记 partial"}
- **裁决（Fable 填写）**：{采纳/驳回/部分采纳 + 理由；已回写的文档与章节；日期}
```

状态含义：

| 状态 | 含义 |
|---|---|
| `open` | 已登记，待裁决 |
| `accepted` | 已采纳，修订已回写（或任务已调整） |
| `rejected` | 驳回，按原文档执行 |
| `resolved` | 问题已不存在（如被后续任务解决），保留记录 |
| `deferred` | 推迟到指定里程碑或 Backlog |

## 2. 登记指引

必须登记的情形（不限于）：

- 实测数字不达 ARCHITECTURE/MILESTONES 的预算或退出标准（附实测与参考机型号）。
- 文档间冲突（如 GAMEDESIGN 旧表述与总控裁定 R1–R11 冲突），或文档引用的章节/ID 不存在。
- 任务超出一次会话需要拆分（建议拆分方案；续作任务规则见 `tasks/README.md` §5.6）。
- 素材/许可证问题：需求 ID 无合规候选、许可证条款矛盾、可提取性风险；数据源许可证不明。
- 接口细化（落地时必须补全的字段位宽、枚举值等），供 Fable 回写文档。
- 外部事实无法联网核实（价格、浏览器支持、条款）——写"未核实"并说明影响。
- 对标测量结果与史实基线严重冲突（REFERENCE_MEASUREMENTS §6.4、§5.3 的取舍需要人类裁决）。

不要登记：纯实现细节、已有文档明确规定的做法、个人偏好。

## 3. 条目列表

（暂无条目。首个条目编号为 `PI-001`。）

### PI-001 protocol M0 接口细化：Welcome 字段、InputBundle 编码与 FireCmd.origin_hint

- **状态**：resolved
- **类别**：协议/数据
- **提出者 / 日期**：M0-005 / 2026-11-17
- **问题**：(1) ARCHITECTURE §6.8.1 的 `Welcome` 表未列 `data_hash:u64` 与 `client_tick_offset:i16`（§6.2 文字要求下发），已按任务卡加入。(2) §6.8.2 未规定 `InputBundle` 内第 2、3 条 `InputCmd` 的差分格式与 `ack_snapshot_tick`/`n`/`fire_n` 位宽：实现为 ack 16 bit、pred_hash 8 bit、n 2 bit(1..3)、fire_n 2 bit(0..2)；首条完整 144 bit；后续条 tick 必须为前一条 +1（否则编码报错），seq 用 1 bit"前一条 +1"标志，move 用同/异标志，yaw/pitch 用 2 bit 类型（同/8 bit 小差分/完整），其余字段"同/异"1 bit。(3) `FireCmd.origin_hint` 为 3×14 bit 原始码，米制含义（步长/参照点）尚未定义，留给 M1。(4) `move_x/move_z == -128` 在编解码两侧一律判为非法（规格为 -127..127）。(5) pitch 线上 8 bit 的最大误差为 0.3529°，略大于验收文字的 0.35°（9 bit 为 0.176°）；yaw 10 bit 最大误差 0.176°（步长 0.352°）。(6) `InputBundle` 不写 msg_type（C1 通道单一消息，省 1 B）；C0 的 msg_type 两个方向共用一套编号（Hello=1,Welcome=2,Reject=3,Ping=4,Pong=5,Disconnect=6）。
- **证据**：典型 3 条走动输入（yaw 每 tick +37、pitch +5）实测 29 B（预算 ≤30 B）；若 yaw 每 tick 变化 > ±127 码（≈0.7°/tick）则该条退化为完整 yaw，约 +8 bit/条，3 条可能略超 30 B。见 crates/protocol/tests/limits.rs。
- **建议方案**：Fable 把上述位宽写回 ARCHITECTURE §6.8.1/§6.8.2 的"变更说明"；M1 若实测鼠标 yaw 变化常超 ±127 码，可把小差分位宽从 8 改为 10 bit（需升 PROTOCOL_VERSION 前完成，M0 期间无兼容负担）。
- **受影响任务**：M0-006、M0-014、M0-016、M0-018（使用这些编码）。
- **临时处理**：按上述实现并以 golden bytes 固定（crates/protocol/tests/golden/）。
- **裁决（Fable 填写）**：
- **裁定（总控）**：采纳全部细化项：Welcome 增加 `data_hash`/`client_tick_offset`；InputBundle 的差分位宽；`move_x/move_z == -128` 判非法；C0 `msg_type` 双向共用编号、InputBundle 不写 `msg_type`；`FireCmd.origin_hint` 米制含义留到 M1。唯一修改：**pitch 量化由 8 bit 改为 9 bit**（8 bit 最大误差 0.353° 超出 0.35° 阈值）。M0-006 落实：新增 `quant::PITCH_BITS_EVENT = 9`，用于 `Shot` 事件 `dir_pitch`（原 §6.8.4 的 pitch8，是 M0 中唯一的 8 bit pitch）；实体快照 pitch 仍为 ARCHITECTURE §6.5.2 的 6 bit（远端玩家外观用，非瞄准关键）。InputBundle 的 pitch 是 i16 + 8 bit 小差分（非量化），无需调整；重测典型 3 条走动输入 InputBundle = 29 B（≤30），最坏 49 B，golden 无变化。

### PI-002 文档冲突：数据字段命名（ID 与单位后缀）以 GAMEDESIGN §0.3 / §6.4–6.6 口径统一

- **状态**：resolved
- **类别**：文档冲突
- **提出者 / 日期**：M0-004 / 2026-11-17
- **问题**：ARCHITECTURE §8.5.1 示例用 `m1_garand`、绝对伤害 `damage_head`、`drag_k_per_m`；GAMEDESIGN §6.1/§6.3 用 `us_m1_garand`、`part_mult`、`k_drag`、`rpm_cyclic`。ARCH §8.5.2/§8.5.4 的 `health`、`max_speed_kph`、`stiffness` 等也不带单位后缀或使用 kph，违反 §8.5 通则"所有数值带单位后缀"。
- **证据**：`data/README.md` "Field-name mapping" 表逐项列出；校验器 `tools/data-validate/rules/units.mjs` 强制。
- **建议方案**：Fable 把 ARCH §8.5.1–§8.5.4 示例改为 `data/schemas/*.schema.json` 的字段名（ID 取 GAMEDESIGN §0.3）。
- **受影响任务**：M0-008、M0-011、M0-013（按本 schema 写数据）。
- **临时处理**：按 data/schemas 与 data/README.md 的映射实现。
- **裁决（Fable 填写）**：采纳。数据字段名以 data/schemas/*.schema.json 与 data/README.md 映射表为准；ARCHITECTURE §8.5 示例已按此更新（见 §8.5 开头“变更说明”与 §8.5.1 示例）。2026-11-17

### PI-003 数据 schema 细化：M0 字段集、可选 _meta、loadout 引用口径

- **状态**：resolved
- **类别**：协议/数据
- **提出者 / 日期**：M0-004 / 2026-11-17
- **问题**：落地时补全了文档未定的细节：①weapon `class`/vehicle `type` 为封闭枚举，新类别须先改 schema；②`historical` 数值可写 `"未核实"`，`_source` 必填；③map_meta/spawns 按 MAPS §6.2 原样，`_meta` 可选（其余四类必填）；④loadout 的 `gadget_1/2`、`melee` 的 id 都必须指向 weapon id（GAMEDESIGN §6.1 把手雷等列为 weapon）；⑤role 增加可选 `capsule{radius_m,height_stand_m,height_crouch_m}`；⑥校验器额外规则：role 速度需 prone<crouch<walk<run<sprint，role.loadout_ids 所指 loadout 的 role/faction 必须匹配，文件名须等于 id；⑦faction 枚举小写 `us|ger`（data 文件），`US|GER`（MAPS spawns）。`rules/*.json`、`maps/*/map.json` 暂无 schema（只进哈希）。
- **证据**：data/schemas/*.schema.json、tools/data-validate/rules/*.mjs。
- **建议方案**：Fable 在 ARCH §8.5 加"变更说明"指向 data/README.md；后续任务新增字段一律追加可选字段并同步 `crates/sim/src/data_defs.rs`。
- **受影响任务**：M0-008、M0-011、M0-013、M0-032。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：采纳全部 7 点。后续新增字段一律先改 schema（可选字段）并同步 crates/sim/src/data_defs.rs；ARCHITECTURE §8.5 已加“变更说明”指向 data/README.md。2026-11-17

### PI-004 依赖登记：sim 新增 serde / serde_json / sha2；crate 包名为 frontline-sim

- **状态**：resolved
- **类别**：其它
- **提出者 / 日期**：M0-004 / 2026-11-17
- **问题**：`crates/sim` 新增 dependency 与 build-dependency：serde 1（derive）、serde_json 1、sha2 0.10（均 MIT/Apache-2.0）。ARCH §4.3.2 允许 serde（仅数据解析）；sha2 若 INDEX 未列出，在此登记。另：任务验收写 `cargo test -p sim`，实际包名是 `frontline-sim`（M0-001），应使用 `cargo test -p frontline-sim data_hash_stable`。`sim` 的 wasm32 构建需确认 sha2/serde_json 可编译（M0-014 验证）。
- **证据**：`cargo test -p frontline-sim` 通过；Cargo.lock。
- **建议方案**：接受；任务/文档里的 `-p sim` 改 `-p frontline-sim`。
- **受影响任务**：M0-007、M0-014。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：采纳。serde / serde_json / sha2 依赖批准；任务与文档里的 `-p sim` 一律理解为 `-p frontline-sim`。2026-11-17

### PI-005 渲染：Babylon 8.56 compute 需自写薄封装（Hi-Z 逐 mip 视图/深度纹理绑定/indirect draw）；无头 WebGPU 无法 present

- **状态**：open
- **类别**：渲染/性能
- **提出者 / 日期**：M0-020 / 2026-11-17
- **问题**：E0-6 试验（`reports/m0/compute-feasibility.md`）显示：①`ComputeShader.setStorageTexture` 不支持指定 mip level，Hi-Z 金字塔只能每级一张纹理，或自写封装；②深度纹理以 `texture_depth_2d` 绑定到 compute、`drawIndexedIndirect` 驱动 Babylon 网格（ARCH §9.7 Hi-Z + indirect draw）、TAAU 抖动/运动向量/RTT ping-pong 接入均未验证；③本机（无 GPU）的 SwiftShader WebGPU 适配器一 present 画布就丢失 Dawn 实例（裸 WebGPU 即复现），CI/容器里只能用测试 shim 离屏渲染，无法做 WebGPU 路径的像素校验与真实帧时间。
- **证据**：`client/src/render/compute-smoke.ts`、`client/e2e/webgpu-swiftshader-shim.js`、报告第"缺口"节。
- **建议方案**：M1 渲染任务（Hi-Z/froxel/TAAU）开工前在参考机（H-021）各做 ≤ 1 天 spike（缺口 2/3/5）；Hi-Z 提前规划"自写薄封装"工作量；WebGPU 视觉回归（M2-050）需 GPU runner 或参考机。
- **受影响任务**：M0-020（E0-6 结论）、M0-030、M1-043/044、M2-032、M2-050。
- **临时处理**：仅验证数据通路；WebGPU 帧率/画面类验收标"需在参考机上测"。
- **裁决（Fable 填写）**：

### PI-006 FixedStepper 步长常量取 1000/30 而非字面 33.333

- **状态**：open
- **类别**：文档冲突
- **提出者 / 日期**：M0-020 / 2026-11-17
- **问题**：任务文件写 `step_ms=33.333`，ARCH §6.1 写 `dt = 1/30 s = 33.333 ms`。字面 33.333 与 30 Hz 每小时漂移约 0.3 tick，故实现用 `1000/30`（约 33.3333…），构造参数仍可覆盖。验收（`advance(100)=3`、`advance(1000)=5` 且 clamp+1、1 h 随机 dt 偏差 ≤ 1）均已通过；内部带 1e-6 ms 容差避免 `3*(1000/30)` 浮点误差少算一步。
- **建议方案**：接受。
- **受影响任务**：M0-022。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-007 数据 schema 细化：weapon.part_mult.pelvis（可选）与 weapon.fx 块

- **状态**：open
- **类别**：协议/数据
- **提出者 / 日期**：M0-008 / 2026-11-17
- **问题**：GAMEDESIGN §6.6 部位倍率含“盆骨/下腹 ×0.9”，M0-008 要求写入 `part_mult`，但 M0-004 的 weapon schema 的 `part_mult` 只有 head/neck/torso/limb；任务 M0-008 还要求 `fx{muzzle_flash_ms, tracer}`，schema 无此块。
- **证据**：data/schemas/weapon.schema.json（新增可选 `gameplay.part_mult.pelvis` 与顶层可选 `fx{muzzle_flash_ms,tracer}`）；crates/sim/src/data_defs.rs 同步（`PartMult.pelvis: Option<f64>`、`WeaponDef.fx: Option<WeaponFx>`，均 `#[serde(default)]`）；校验器 42/42、fixtures 8 valid / 23 invalid 仍通过。
- **建议方案**：接受（均为可选字段，向后兼容）。`pelvis` 缺省时 sim 应回退到 `torso` 倍率或 0.9（由 M0-012 在读取处决定并写测试）；`fx` 仅供表现层，sim 逻辑不得读取。
- **受影响任务**：M0-012（伤害结算读取 pelvis）、M0-025（曳光走客户端调试开关）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-008 M0-009 细化：远景环与碰撞区块一并产出；meta/spawns schema 增加可选字段；二进制不入库

- **状态**：open
- **类别**：范围细化 / 数据格式
- **提出者 / 日期**：M0-009 / 2026-11-17
- **问题**：①INDEX 只列 257² 高度图与 meta.json，本任务按任务卡一并产出 `heightmap_far_8m.r16`、`chunks/b_*`、`chunks/c_*`。②为写入 `generator`/`dequant`/`layout`/`chunks`，`map_meta.schema.json` 新增可选字段；`spawns.schema.json` 的 `hq.*` 新增可选 `y_m`、顶层新增可选 `patrol_points[{id,pos,y_m}]`（均为可选，sim 尚未解析 maps，故无需改 `data_defs.rs`）。③`units.mjs` 的无量纲白名单追加 `seed`。④总控约定"大二进制不入 git"：`*.r16` 与 `chunks/` 加入 .gitignore，仅 `meta/spawns/manifest.json` 入库（manifest 含各二进制的字节数与 SHA-256，`validate` 会核对）；重生成命令见 DEVELOPING.md。因此 M0-010/M0-023 在干净检出上须先运行重生成命令（`DATA_HASH` 只含 .json，不受二进制影响）。⑤M0 的 meta.crs 对程序化地形是占位（epsg 32630、原点 0,0）；`far_ring` 与近景共用同一 hmin/hmax（整数米，覆盖两者）。⑥spawns 的 hq 键沿用 schema 的 `US`/`GER`（大写），与"数据里 faction 小写"约定不一致，仅因既有 schema 如此。
- **建议方案**：接受；若希望二进制入库（~2.4 MB），删除 .gitignore 末尾两行即可。
- **受影响任务**：M0-010、M0-023、M0-004（schema）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-009 IBM Plex 带 Reserved Font Name "Plex"：子集化/WOFF2 产物不能沿用原名（M0-003）

- **状态**：open
- **类别**：许可证 / 素材
- **提出者 / 日期**：M0-003 / 2026-11-17
- **问题**：IBM Plex 的 LICENSE.txt 首行为 "Copyright © 2017 IBM Corp. with Reserved Font Name Plex"。OFL FAQ 2.6 称子集化属修改，2.2 称格式转换一般属修改，修改版不得使用 RFN（2.7 仅在功能等价时例外）。UI_SPEC §2.3 要求子集化 Plex Sans/Mono 并以原名提供。Noto Sans SC 的 RFN 为 "Source"，族名不含该词，无冲突；Barlow Semi Condensed 无 RFN。
- **证据**：licenses/texts/AST-V-FNT-004-01.txt；https://openfontlicense.org/ofl-faq/ （2026-11-17 访问）；LICENSES.md 台账 AST-V-FNT-004-01/02（状态 PENDING）。
- **建议方案**：A）子集产物改族名（name 表与 CSS 均不含 Plex），保留许可证文本（推荐，零沟通成本）；B）不子集化，直接用 IBM 官方 WOFF2（体积可能超预算）；C）向 IBM 取书面许可。另建议 LICENSES §2 新增处理码 `SUBSET`（子集化）与 `RENAME`（改族名），当前只能用 `CONVERT`。
- **受影响任务**：M1-053、M1-054。
- **临时处理**：台账 PENDING，不入库任何 Plex 子集产物。
- **裁决（Fable 填写）**：

### PI-010 snapshot 位宽与基线语义细化（M0-006）

- **状态**：open
- **类别**：协议/数据
- **提出者 / 日期**：M0-006 / 2026-11-17
- **问题**：ARCHITECTURE §6.5.1/§6.5.2 的快照布局有若干未定细节，M0-006 作如下细化（均写在 `crates/protocol/src/snapshot/*` 模块文档里，golden bytes 固定）：(1) 头部 `flags(4b)` 前移到 `type` 之后（读者须先知道 tick 是 32 还是 16 bit）；flags = FULL_TICK / CRC / HAS_SELF / HAS_BASELINE；`baseline_tick_delta` 存 `delta-1`（5 bit 表示 1..32），故 ack 落后 32 tick 仍可 delta，33 起全关键帧。16 bit tick 取"离解码器最新 tick 最近的值"，仅在客户端已 ack 过之后使用。(2) 一个包只引用**一个**基线 tick（该包内被 ack 的最新快照）；只有"在那一包里出现过"的实体才发 delta，其余发关键帧——比 §6.5.2 的"每实体基线 tick"简单，且丢包下编解码端天然一致（实测 30% 丢包 60 s 0 次 MissingBaseline）。AOI 若让实体隔包出现，会多发关键帧（12.5 B vs 4–6 B）：M0-019 应尽量让近/中景实体每包都出现。(3) 记录类型：`id:8` 后 1 bit `1`=delta（后随 `field_mask:8`）、`01`=关键帧、`00`=移除（`removed_ids`）；mask 位 7 为 near 标志（rot 宽度）、位 2 vel 保留（M0 置位判非法）。(4) 位置差分大小类按 |步数| `<32/<256/<2048/其余(绝对值)` 对应 `6/9/12 bit/绝对 16 bit`（x、z 共用一个类；y 单独，绝对 13 bit）；原文 `<8/<64/<512` 与 6/9/12 bit 不自洽，已改。(5) 远景记录 yaw 8 bit、**不发 pitch**（保持基线值）。(6) EntityState：yaw 为 10 bit 码、pitch 为 6 bit 码、anim<16、stance<8、weapon<64；id 为 u8（与 EntityId u16 的差异见 M0-005）。(7) self_state：ammo_in_mag 8、ammo_reserve 12、stamina 6、suppression 4、cooldown 8，pos_corr 标签 4 bit（0=ok，1=后随 45 bit 位置）。(8) 事件类型码 Shot=1/Impact=2/HitConfirm=3/DamageDir=4（5 bit）；位置类字段为原始 14 bit 码，米制含义同 origin_hint 留 M1。(9) CRC-16/CCITT-FALSE（0x1021，init 0xFFFF），大端附在末尾。
- **证据**：tests/snapshot_size.rs、tests/snapshot_prop.rs 的实测输出（见 tasks/STATUS.md M0-006 行）。
- **建议方案**：Fable 把上述写回 ARCHITECTURE §6.5/§6.8.4 的"变更说明"。
- **受影响任务**：M0-014、M0-019、M0-016、M0-018、M0-027（使用快照编解码）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-012 M0-031：weapon.historical._source 改为结构化数组；_meta 增加 unverified / deviation_reason；M1 实用射速 40 → 24

- **状态**：open
- **类别**：数据格式细化 / 史实数值差异
- **提出者 / 日期**：M0-031 / 2026-11-17
- **问题**：①任务要求 `historical._source` 为结构化数组：`data/schemas/weapon.schema.json` 改为 `string | source_record[]`（`initial` 数据仍可写字符串），`_meta` 新增可选 `unverified[]`、`deviation_reason{}`；`crates/sim/src/data_defs.rs` 同步（`HistSource` untagged 枚举、`Meta.unverified/deviation_reason`、`SourceRecord`）。新规则 `tools/data-validate/rules/historical.mjs`（`hist-source-required`，status=reviewed/measured 时强制）。vehicle schema 尚未改（M3-051 做）。②数值差异：`us_m1_garand.historical.rate_of_fire_rpm_practical` 40 → 24（FM 23-5 Ch.1 par.4 的 16-24 发/分瞄准射击；GAMEDESIGN §6.2 的 40 出自记忆）；新增 `effective_range_m` = 460（FM 23-5）。GAMEDESIGN §6.2 / ARCHITECTURE §8.5.1 示例中的 853 与 760 经核对一致或未被推翻（853 有 FM 23-5 支持；Kar98k 760 未能核实，二手来源有 755/760/745）。`gameplay` 数值一律未改。③来源争议需人类裁决：Kar98k 有效射程（TM-E 30-451 约 732 m vs 二手 500 m 铁瞄）；FM 23-5 转写页未标版次，为战后版；Kar98k 的 s.S. 弹种名与初速/射速无一手来源（HDv/Merkblatt、Hogg、博物馆馆藏本次均未检索到）。
- **证据**：reports/m0/hist-review-m0.md；data/weapons/*.json；`node tools/data-validate/index.mjs` 退出 0，`pnpm -C tools/data-validate test` 47/47。
- **建议方案**：接受 schema 细化；M1-070 补 Kar98k 的一手来源并复核 M1 的射速取值口径（24 为手册瞄准射速上限，另有"速射"口径）。
- **受影响任务**：M1-070、M2-053、M3-051（沿用 `_source` 数组格式）、M0-032。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-011 M0-007：bevy_ecs 在 wasm32 上传递依赖 wasm-bindgen/tokio，须锁 0.16.1 且不开 `std` feature；其它细化

- **状态**：open
- **类别**：依赖 / 接口细化
- **问题**：任务要求 `sim` 依赖图不含 `wasm-bindgen`/`tokio`（check-deps 与验收 3）。实测（`cargo metadata` 解析图，scratchpad 隔离工程 + 仓库内）：`bevy_ecs` 0.17/0.18/0.19/0.20 在 `cfg(target_arch="wasm32")` 下经 `bevy_platform`/`bevy_tasks`（web-task）传递引入 `wasm-bindgen`、`wasm-bindgen-futures`（→`tokio`）；0.16.x 开 `std` feature 也引入 `wasm-bindgen`；**仅 0.16.1 且 `default-features=false`（不开 `std`）图干净**，`node tools/ci/check-deps.mjs` 通过。
- **建议方案**：`bevy_ecs = "=0.16.1", default-features = false`（已采用）；单线程 executor 用 `ExecutorKind::SingleThreaded`（该版本也无 `multi_threaded` feature）。升级 bevy_ecs 前须重新跑 check-deps；如必须升级，需放宽 check-deps 对 `cfg(wasm32)` 传递依赖的判定（总控/Fable 决定）。
- **细化**：①`WorldView::entity_state` 返回 `Option<EntityStateView>`（旧 id 须得 `None`；`protocol::EntityState` 当前未导出，待 M0-015 接入时替换）；②`EntityId`/`PlayerId` 为类型别名，`Tick` 为不实现 `Ord` 的 newtype（环绕比较仅 `lt/le/diff`）；③`GameMode` 增加 `Sandbox`；④`test-util` 特性下的 determinism 集成测试以 `required-features` 声明，**CI 必须用 `cargo test -p frontline-sim --features test-util`**（自引用 dev-dependency 会被 check-deps 判为 sim->sim 禁止边，故未用）；⑤未使用 `indexmap`（全部走 `BTreeMap`）；⑥`rand_core` 0.10 的 `RngCore` 已更名 `Rng`，sim 里 `rng::RngCore` 为其重导出别名，`rng::Rng` 是 `Rng::for_` 的命名空间 struct。
- **受影响任务**：M0-010/011/012/013（沿用 World 接口）、M0-014（sim-wasm 自带 wasm-bindgen，不受影响）、M0-029/M0-030（CI 命令）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：
