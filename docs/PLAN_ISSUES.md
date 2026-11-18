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

> 编号核对（2026-11-17）：当前 PI-001 至 PI-020 无重复、无缺号；PI-011 的条目位于 PI-012 之后（登记顺序所致，编号未重排）；PI-008 原误编为第二个 PI-007，已更正为 PI-008，其内容未变。新增条目前先看本文件最大 PI 编号再 +1，写入前重新读取文件末尾。

### PI-001 protocol M0 接口细化：Welcome 字段、InputBundle 编码与 FireCmd.origin_hint

- **状态**：resolved
- **类别**：协议/数据
- **提出者 / 日期**：M0-005 / 2026-11-17
- **问题**：(1) ARCHITECTURE §6.8.1 的 `Welcome` 表未列 `data_hash:u64` 与 `client_tick_offset:i16`（§6.2 文字要求下发），已按任务卡加入。(2) §6.8.2 未规定 `InputBundle` 内第 2、3 条 `InputCmd` 的差分格式与 `ack_snapshot_tick`/`n`/`fire_n` 位宽：实现为 ack 16 bit、pred_hash 8 bit、n 2 bit(1..3)、fire_n 2 bit(0..2)；首条完整 144 bit；后续条 tick 必须为前一条 +1（否则编码报错），seq 用 1 bit"前一条 +1"标志，move 用同/异标志，yaw/pitch 用 2 bit 类型（同/8 bit 小差分/完整），其余字段"同/异"1 bit。(3) `FireCmd.origin_hint` 为 3×14 bit 原始码，米制含义（步长/参照点）尚未定义，留给 M1。(4) `move_x/move_z == -128` 在编解码两侧一律判为非法（规格为 -127..127）。(5) pitch 线上 8 bit 的最大误差为 0.3529°，略大于验收文字的 0.35°（9 bit 为 0.176°）；yaw 10 bit 最大误差 0.176°（步长 0.352°）。(6) `InputBundle` 不写 msg_type（C1 通道单一消息，省 1 B）；C0 的 msg_type 两个方向共用一套编号（Hello=1,Welcome=2,Reject=3,Ping=4,Pong=5,Disconnect=6）。
- **证据**：典型 3 条走动输入（yaw 每 tick +37、pitch +5）实测 29 B（预算 ≤30 B）；若 yaw 每 tick 变化 > ±127 码（≈0.7°/tick）则该条退化为完整 yaw，约 +8 bit/条，3 条可能略超 30 B。见 crates/protocol/tests/limits.rs。
- **建议方案**：Fable 把上述位宽写回 ARCHITECTURE §6.8.1/§6.8.2 的"变更说明"；M1 若实测鼠标 yaw 变化常超 ±127 码，可把小差分位宽从 8 改为 10 bit（需升 PROTOCOL_VERSION 前完成，M0 期间无兼容负担）。
- **受影响任务**：M0-006、M0-014、M0-016、M0-018（使用这些编码）。
- **临时处理**：按上述实现并以 golden bytes 固定（crates/protocol/tests/golden/）。
- **裁决（Fable 填写）**：见下方总控裁定（resolved，2026-11-17）。
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

- **状态**：resolved
- **类别**：渲染/性能
- **提出者 / 日期**：M0-020 / 2026-11-17
- **问题**：E0-6 试验（`reports/m0/compute-feasibility.md`）显示：①`ComputeShader.setStorageTexture` 不支持指定 mip level，Hi-Z 金字塔只能每级一张纹理，或自写封装；②深度纹理以 `texture_depth_2d` 绑定到 compute、`drawIndexedIndirect` 驱动 Babylon 网格（ARCH §9.7 Hi-Z + indirect draw）、TAAU 抖动/运动向量/RTT ping-pong 接入均未验证；③本机（无 GPU）的 SwiftShader WebGPU 适配器一 present 画布就丢失 Dawn 实例（裸 WebGPU 即复现），CI/容器里只能用测试 shim 离屏渲染，无法做 WebGPU 路径的像素校验与真实帧时间。
- **证据**：`client/src/render/compute-smoke.ts`、`client/e2e/webgpu-swiftshader-shim.js`、报告第"缺口"节。
- **建议方案**：M1 渲染任务（Hi-Z/froxel/TAAU）开工前在参考机（H-021）各做 ≤ 1 天 spike（缺口 2/3/5）；Hi-Z 提前规划"自写薄封装"工作量；WebGPU 视觉回归（M2-050）需 GPU runner 或参考机。
- **受影响任务**：M0-020（E0-6 结论）、M0-030、M1-043/044、M2-032、M2-050。
- **临时处理**：仅验证数据通路；WebGPU 帧率/画面类验收标"需在参考机上测"。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：Babylon 的 WebGPU compute 用自写薄封装（client/src/render 内）；逐 mip 视图、深度纹理绑定、indirect draw 留到各自用到的任务里验证。本机 WebGPU 只有 SwiftShader 软件适配器，present 画布会丢失 Dawn 实例：e2e 通过 client/e2e/webgpu-swiftshader-shim.js 把输出换成离屏纹理（WebGPU 路径无像素校验）；像素/截图校验走 WebGL2 路径。真 GPU 的验收项标"需在参考机上测（H-021）"，复测入口 `/?renderer=webgpu&smoke=compute&debug=1`。　文档回写：未记录。

### PI-006 FixedStepper 步长常量取 1000/30 而非字面 33.333

- **状态**：resolved
- **类别**：文档冲突
- **提出者 / 日期**：M0-020 / 2026-11-17
- **问题**：任务文件写 `step_ms=33.333`，ARCH §6.1 写 `dt = 1/30 s = 33.333 ms`。字面 33.333 与 30 Hz 每小时漂移约 0.3 tick，故实现用 `1000/30`（约 33.3333…），构造参数仍可覆盖。验收（`advance(100)=3`、`advance(1000)=5` 且 clamp+1、1 h 随机 dt 偏差 ≤ 1）均已通过；内部带 1e-6 ms 容差避免 `3*(1000/30)` 浮点误差少算一步。
- **建议方案**：接受。
- **受影响任务**：M0-022。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：固定步长取 1000/30 ms。　文档回写：未记录。

### PI-007 数据 schema 细化：weapon.part_mult.pelvis（可选）与 weapon.fx 块

- **状态**：resolved
- **类别**：协议/数据
- **提出者 / 日期**：M0-008 / 2026-11-17
- **问题**：GAMEDESIGN §6.6 部位倍率含“盆骨/下腹 ×0.9”，M0-008 要求写入 `part_mult`，但 M0-004 的 weapon schema 的 `part_mult` 只有 head/neck/torso/limb；任务 M0-008 还要求 `fx{muzzle_flash_ms, tracer}`，schema 无此块。
- **证据**：data/schemas/weapon.schema.json（新增可选 `gameplay.part_mult.pelvis` 与顶层可选 `fx{muzzle_flash_ms,tracer}`）；crates/sim/src/data_defs.rs 同步（`PartMult.pelvis: Option<f64>`、`WeaponDef.fx: Option<WeaponFx>`，均 `#[serde(default)]`）；校验器 42/42、fixtures 8 valid / 23 invalid 仍通过。
- **建议方案**：接受（均为可选字段，向后兼容）。`pelvis` 缺省时 sim 应回退到 `torso` 倍率或 0.9（由 M0-012 在读取处决定并写测试）；`fx` 仅供表现层，sim 逻辑不得读取。
- **受影响任务**：M0-012（伤害结算读取 pelvis）、M0-025（曳光走客户端调试开关）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：weapon schema 的可选字段 `part_mult.pelvis` 与 `fx{...}` 保留；`pelvis` 缺省时回退为躯干（torso）倍率。　文档回写：未记录。

### PI-008 M0-009 细化：远景环与碰撞区块一并产出；meta/spawns schema 增加可选字段；二进制不入库

- **状态**：resolved
- **类别**：范围细化 / 数据格式
- **提出者 / 日期**：M0-009 / 2026-11-17
- **问题**：①INDEX 只列 257² 高度图与 meta.json，本任务按任务卡一并产出 `heightmap_far_8m.r16`、`chunks/b_*`、`chunks/c_*`。②为写入 `generator`/`dequant`/`layout`/`chunks`，`map_meta.schema.json` 新增可选字段；`spawns.schema.json` 的 `hq.*` 新增可选 `y_m`、顶层新增可选 `patrol_points[{id,pos,y_m}]`（均为可选，sim 尚未解析 maps，故无需改 `data_defs.rs`）。③`units.mjs` 的无量纲白名单追加 `seed`。④总控约定"大二进制不入 git"：`*.r16` 与 `chunks/` 加入 .gitignore，仅 `meta/spawns/manifest.json` 入库（manifest 含各二进制的字节数与 SHA-256，`validate` 会核对）；重生成命令见 DEVELOPING.md。因此 M0-010/M0-023 在干净检出上须先运行重生成命令（`DATA_HASH` 只含 .json，不受二进制影响）。⑤M0 的 meta.crs 对程序化地形是占位（epsg 32630、原点 0,0）；`far_ring` 与近景共用同一 hmin/hmax（整数米，覆盖两者）。⑥spawns 的 hq 键沿用 schema 的 `US`/`GER`（大写），与"数据里 faction 小写"约定不一致，仅因既有 schema 如此。
- **建议方案**：接受；若希望二进制入库（~2.4 MB），删除 .gitignore 末尾两行即可。
- **受影响任务**：M0-010、M0-023、M0-004（schema）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：地形二进制不入库，干净检出先运行 `PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0`（约 1 s，见 DEVELOPING.md）；约定 x 东、z 北，数组行=z、列=x，`h = hmin + u16/65535*(hmax−hmin)`；map_meta / spawns 的新增可选字段保留。　文档回写：未记录。

### PI-009 IBM Plex 带 Reserved Font Name "Plex"：子集化/WOFF2 产物不能沿用原名（M0-003）

- **状态**：resolved
- **类别**：许可证 / 素材
- **提出者 / 日期**：M0-003 / 2026-11-17
- **问题**：IBM Plex 的 LICENSE.txt 首行为 "Copyright © 2017 IBM Corp. with Reserved Font Name Plex"。OFL FAQ 2.6 称子集化属修改，2.2 称格式转换一般属修改，修改版不得使用 RFN（2.7 仅在功能等价时例外）。UI_SPEC §2.3 要求子集化 Plex Sans/Mono 并以原名提供。Noto Sans SC 的 RFN 为 "Source"，族名不含该词，无冲突；Barlow Semi Condensed 无 RFN。
- **证据**：licenses/texts/AST-V-FNT-004-01.txt；https://openfontlicense.org/ofl-faq/ （2026-11-17 访问）；LICENSES.md 台账 AST-V-FNT-004-01/02（状态 PENDING）。
- **建议方案**：A）子集产物改族名（name 表与 CSS 均不含 Plex），保留许可证文本（推荐，零沟通成本）；B）不子集化，直接用 IBM 官方 WOFF2（体积可能超预算）；C）向 IBM 取书面许可。另建议 LICENSES §2 新增处理码 `SUBSET`（子集化）与 `RENAME`（改族名），当前只能用 `CONVERT`。
- **受影响任务**：M1-053、M1-054。
- **临时处理**：台账 PENDING，不入库任何 Plex 子集产物。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：采用方案 A——IBM Plex 的子集/WOFF2 产物改族名（不含保留字 "Plex"，用 `Frontline UI Sans` / `Frontline UI Mono`；CSS 变量与 font-family 引用这两个名字）；Noto Sans SC 子集产物的 name 表不得出现 "Source"。LICENSES.md 的 allowed_processing 增加处理码 `SUBSET`、`RENAME`（由 M1-053 落实并把 Plex 两行转 OK）。　文档回写：未记录。

### PI-010 snapshot 位宽与基线语义细化（M0-006）

- **状态**：resolved
- **类别**：协议/数据
- **提出者 / 日期**：M0-006 / 2026-11-17
- **问题**：ARCHITECTURE §6.5.1/§6.5.2 的快照布局有若干未定细节，M0-006 作如下细化（均写在 `crates/protocol/src/snapshot/*` 模块文档里，golden bytes 固定）：(1) 头部 `flags(4b)` 前移到 `type` 之后（读者须先知道 tick 是 32 还是 16 bit）；flags = FULL_TICK / CRC / HAS_SELF / HAS_BASELINE；`baseline_tick_delta` 存 `delta-1`（5 bit 表示 1..32），故 ack 落后 32 tick 仍可 delta，33 起全关键帧。16 bit tick 取"离解码器最新 tick 最近的值"，仅在客户端已 ack 过之后使用。(2) 一个包只引用**一个**基线 tick（该包内被 ack 的最新快照）；只有"在那一包里出现过"的实体才发 delta，其余发关键帧——比 §6.5.2 的"每实体基线 tick"简单，且丢包下编解码端天然一致（实测 30% 丢包 60 s 0 次 MissingBaseline）。AOI 若让实体隔包出现，会多发关键帧（12.5 B vs 4–6 B）：M0-019 应尽量让近/中景实体每包都出现。(3) 记录类型：`id:8` 后 1 bit `1`=delta（后随 `field_mask:8`）、`01`=关键帧、`00`=移除（`removed_ids`）；mask 位 7 为 near 标志（rot 宽度）、位 2 vel 保留（M0 置位判非法）。(4) 位置差分大小类按 |步数| `<32/<256/<2048/其余(绝对值)` 对应 `6/9/12 bit/绝对 16 bit`（x、z 共用一个类；y 单独，绝对 13 bit）；原文 `<8/<64/<512` 与 6/9/12 bit 不自洽，已改。(5) 远景记录 yaw 8 bit、**不发 pitch**（保持基线值）。(6) EntityState：yaw 为 10 bit 码、pitch 为 6 bit 码、anim<16、stance<8、weapon<64；id 为 u8（与 EntityId u16 的差异见 M0-005）。(7) self_state：ammo_in_mag 8、ammo_reserve 12、stamina 6、suppression 4、cooldown 8，pos_corr 标签 4 bit（0=ok，1=后随 45 bit 位置）。(8) 事件类型码 Shot=1/Impact=2/HitConfirm=3/DamageDir=4（5 bit）；位置类字段为原始 14 bit 码，米制含义同 origin_hint 留 M1。(9) CRC-16/CCITT-FALSE（0x1021，init 0xFFFF），大端附在末尾。
- **证据**：tests/snapshot_size.rs、tests/snapshot_prop.rs 的实测输出（见 tasks/STATUS.md M0-006 行）。
- **建议方案**：Fable 把上述写回 ARCHITECTURE §6.5/§6.8.4 的"变更说明"。
- **受影响任务**：M0-014、M0-019、M0-016、M0-018、M0-027（使用快照编解码）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：一个快照包只引用一个基线 tick，只有在该基线包里出现过的实体才发 delta，其余发关键帧——所以 AOI（M0-019）应让近/中景实体每包都出现，避免隔包出现导致反复发关键帧；远景记录只发 yaw 8 bit、不发 pitch；位置差分大小类阈值 32/256/2048 步；基线环 32 tick，超出即全关键帧。快照实测：近景 delta 4–6 B/记录、关键帧 12.5 B/记录、40 实体包约 282 B。模糊测试：`cargo +nightly fuzz run <target> -- -max_total_time=60`（crates/protocol/fuzz 为独立 workspace）。　文档回写：未记录。

### PI-012 M0-031：weapon.historical._source 改为结构化数组；_meta 增加 unverified / deviation_reason；M1 实用射速 40 → 24

- **状态**：resolved
- **类别**：数据格式细化 / 史实数值差异
- **提出者 / 日期**：M0-031 / 2026-11-17
- **问题**：①任务要求 `historical._source` 为结构化数组：`data/schemas/weapon.schema.json` 改为 `string | source_record[]`（`initial` 数据仍可写字符串），`_meta` 新增可选 `unverified[]`、`deviation_reason{}`；`crates/sim/src/data_defs.rs` 同步（`HistSource` untagged 枚举、`Meta.unverified/deviation_reason`、`SourceRecord`）。新规则 `tools/data-validate/rules/historical.mjs`（`hist-source-required`，status=reviewed/measured 时强制）。vehicle schema 尚未改（M3-051 做）。②数值差异：`us_m1_garand.historical.rate_of_fire_rpm_practical` 40 → 24（FM 23-5 Ch.1 par.4 的 16-24 发/分瞄准射击；GAMEDESIGN §6.2 的 40 出自记忆）；新增 `effective_range_m` = 460（FM 23-5）。GAMEDESIGN §6.2 / ARCHITECTURE §8.5.1 示例中的 853 与 760 经核对一致或未被推翻（853 有 FM 23-5 支持；Kar98k 760 未能核实，二手来源有 755/760/745）。`gameplay` 数值一律未改。③来源争议需人类裁决：Kar98k 有效射程（TM-E 30-451 约 732 m vs 二手 500 m 铁瞄）；FM 23-5 转写页未标版次，为战后版；Kar98k 的 s.S. 弹种名与初速/射速无一手来源（HDv/Merkblatt、Hogg、博物馆馆藏本次均未检索到）。
- **证据**：reports/m0/hist-review-m0.md；data/weapons/*.json；`node tools/data-validate/index.mjs` 退出 0，`pnpm -C tools/data-validate test` 47/47。
- **建议方案**：接受 schema 细化；M1-070 补 Kar98k 的一手来源并复核 M1 的射速取值口径（24 为手册瞄准射速上限，另有"速射"口径）。
- **受影响任务**：M1-070、M2-053、M3-051（沿用 `_source` 数组格式）、M0-032。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：weapon 的 `historical._source` 用结构化数组（每字段的文献 + 段落定位 + 核实日期），`_meta.unverified[]` 登记查不到一手来源的字段，`_meta.deviation_reason{}` 记录游戏性数值偏离史实的原因；`historical` 只描述史实，`gameplay` 数值不因史实复核而改动。M1 Garand 史实瞄准射速 16–24 发/分（FM 23-5），游戏内射速上限是 gameplay 值。后续武器/载具数据沿用同一格式。　文档回写：未记录。

### PI-011 M0-007：bevy_ecs 在 wasm32 上传递依赖 wasm-bindgen/tokio，须锁 0.16.1 且不开 `std` feature；其它细化

- **状态**：resolved
- **类别**：依赖 / 接口细化
- **问题**：任务要求 `sim` 依赖图不含 `wasm-bindgen`/`tokio`（check-deps 与验收 3）。实测（`cargo metadata` 解析图，scratchpad 隔离工程 + 仓库内）：`bevy_ecs` 0.17/0.18/0.19/0.20 在 `cfg(target_arch="wasm32")` 下经 `bevy_platform`/`bevy_tasks`（web-task）传递引入 `wasm-bindgen`、`wasm-bindgen-futures`（→`tokio`）；0.16.x 开 `std` feature 也引入 `wasm-bindgen`；**仅 0.16.1 且 `default-features=false`（不开 `std`）图干净**，`node tools/ci/check-deps.mjs` 通过。
- **建议方案**：`bevy_ecs = "=0.16.1", default-features = false`（已采用）；单线程 executor 用 `ExecutorKind::SingleThreaded`（该版本也无 `multi_threaded` feature）。升级 bevy_ecs 前须重新跑 check-deps；如必须升级，需放宽 check-deps 对 `cfg(wasm32)` 传递依赖的判定（总控/Fable 决定）。
- **细化**：①`WorldView::entity_state` 返回 `Option<EntityStateView>`（旧 id 须得 `None`；`protocol::EntityState` 当前未导出，待 M0-015 接入时替换）；②`EntityId`/`PlayerId` 为类型别名，`Tick` 为不实现 `Ord` 的 newtype（环绕比较仅 `lt/le/diff`）；③`GameMode` 增加 `Sandbox`；④`test-util` 特性下的 determinism 集成测试以 `required-features` 声明，**CI 必须用 `cargo test -p frontline-sim --features test-util`**（自引用 dev-dependency 会被 check-deps 判为 sim->sim 禁止边，故未用）；⑤未使用 `indexmap`（全部走 `BTreeMap`）；⑥`rand_core` 0.10 的 `RngCore` 已更名 `Rng`，sim 里 `rng::RngCore` 为其重导出别名，`rng::Rng` 是 `Rng::for_` 的命名空间 struct。
- **受影响任务**：M0-010/011/012/013（沿用 World 接口）、M0-014（sim-wasm 自带 wasm-bindgen，不受影响）、M0-029/M0-030（CI 命令）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：`bevy_ecs` 锁 `=0.16.1`、`default-features=false` 且不开 `std`（更高版本在 wasm32 上会传递引入 wasm-bindgen/tokio，使 check-deps 失败）；升级前必须重跑 `node tools/ci/check-deps.mjs`。sim 的测试命令是 `cargo test -p frontline-sim --features test-util`（determinism 集成测试 required-features=["test-util"]，不带会静默跳过）。sim 内遍历用 BTreeMap，保持确定性。sim 的占位接口：`WorldView::los`、`height_at`（M0-010 落实）、`team_resources`（M0-013）；`entity_state` 目前返回 sim 自己的 `EntityStateView`（M0-015 接入 protocol 类型时替换）。　文档回写：未记录。

### PI-013 M0-010：Rapier 0.36.1 精确锁定与特性取舍；查询自写、Rapier 仅作碰撞世界；若干接口细化

- **状态**：resolved
- **类别**：依赖 / 接口细化
- **提出者 / 日期**：M0-010 / 2026-11-17
- **问题**：①依赖：`rapier3d = "=0.36.1"`，`default-features=false`，features = `dim3, f32, std, enhanced-determinism`（不开 `simd-*`、`parallel`、`profiler`（会引入 web-time）、`serde-serialize`）。`enhanced-determinism` = `simba/libm_force + parry3d/enhanced-determinism`，在 wasm32-unknown-unknown 下可构建，`check-deps` 仍通过（传递依赖 nalgebra 0.35 / parry3d 0.31 / glamx 0.3 / simba / wide / spade 等，许可均为 Apache-2.0 或 MIT/Zlib 系，在 deny 白名单内；本机无 cargo-deny，未实跑）。注意：Rapier 0.36 的 API 已不同于任务书假定的旧版（`PhysicsWorld`、`QueryPipeline<'a>` 借用式、数学类型为 glamx），KCC 为 `rapier3d::control::KinematicCharacterController::move_shape`。②native 与 wasm32 的逐位一致性：本机只能验证 native；`CollisionData::state_hash()` 已纳入区块样本与碰撞体顺序，M0-014/M0-029 应在 wasm 与 native 上各算一次比对。③任务书写"射线可用 Rapier QueryPipeline 或自写 DDA"：采用自写 1 m 网格 DDA（两三角形/格，对角线与 Rapier 非 zigzag 高度场一致），`height_at` 取双线性（验收 3 的参考），法线/坡度/射线用三角形面。实测 Rapier 0.36.1 对个别跨多个高度场碰撞体的长射线会漏检（3000 条随机射线中 1 条，原点(219.72,27.71,241.73)、方向(0.254,-0.200,-0.946)；自写结果经暴力步进验证正确），故不以 Rapier 射线作为权威。④接口：`CollisionChunk::from_bytes(bx, bz, bytes)`（文件头不含 bx/bz，须调用方传入；与任务书 `from_bytes(&[u8])` 的差异）；`WorldView::height_at` 改为返回 `Option<f32>`（缺块为 None）；`los` 在线段进入未载入区块/起点在地下时返回 false；`World::collision()` 暴露 `&CollisionData`；`World::new` 内部 `clone()`（Clone 重建 Rapier 世界，按 (bz,bx) 升序）。⑤基准未用 criterion（保持与 `world_step_empty` 相同的 `Instant` 手写 harness，避免新增 dev 依赖）。⑥`state_hash()` 为 `CollisionData` 的独立摘要，未并入 `World::state_hash`（静态数据，且避免改动 determinism golden）。
- **证据**：crates/sim/src/collision/*；tests/collision_*.rs；`cargo bench -p frontline-sim --bench collision`；tasks/STATUS.md M0-010 行。
- **建议方案**：接受；M0-011 的 KCC 直接用 `CollisionData::physics().world().query_pipeline()`。
- **受影响任务**：M0-011（KCC）、M0-012（弹道用 `raycast`）、M0-014/M0-029（wasm 构建与 native/wasm 一致性）、M0-022（缺块限制）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：`rapier3d =0.36.1`（default-features=false；dim3, f32, std, enhanced-determinism）只作碰撞世界；高度/法线/射线/LOS 查询为 sim 自写（crates/sim/src/collision/）。`CollisionChunk::from_bytes(bx, bz, bytes)`；`WorldView::height_at` 返回 `Option<f32>`；`los` 遇缺块或起点在地下返回 false；KCC 用法见 crates/sim/tests/collision_kcc.rs。地形二进制缺失时相关测试打印重建命令后跳过，设 `FRONTLINE_REQUIRE_TERRAIN=1` 则改为失败（验收时请带上该环境变量跑一遍）。native 与 wasm 的逐位一致性待 M0-014 / M0-029 两端比对。　文档回写：未记录。

### PI-014 M0-023：CDLOD 地形——纯距离 morph 在 §9.2 参数下会裂缝（加边邻接掩码）；节点数与验收 2 下限不符；高度纹理 RG8；近远景融合带

- **状态**：resolved
- **类别**：算法 / 验收口径 / 数据格式
- **提出者 / 日期**：M0-023 / 2026-11-17
- **问题**：
  ①**纯距离 morph 不能保证无裂缝**。§9.2 的范围按 2× 递增、`morph_k = smoothstep(0.7r, r, d)`、节点边长 64·2^L（r_L = 110·2^L = 1.72×节点边长）。相邻的细节点 F（LOD L）与粗节点 N'（LOD L+1）的共享边上点到相机距离可达 r_L + diag(F) ≈ 1.82·r_L，而 N' 自身的 morph 在 0.7·r_{L+1} = 1.4·r_L 就已开始，于是 N' 的奇数边顶点已在向 L+2 网格插值，而 F 的偶数顶点仍停在 L+1 网格上——出现裂缝。要靠距离保证无裂缝，morph 区只能留最后 ≈9 %（须 start_{L+1} ≥ r_L + diag(F_L)）。单元测试 `terrain_morph_seam.test.ts`（在 `cdlod-quadtree.test.ts` 的 "control" 用例）用真实选择结果复现：去掉掩码后最大共享边高度差 1.67 m（合成粗糙曲面），带掩码为 9.8e-15 m。**处理**：保留 §9.2 的范围表与 0.7 起点，另在每个实例上带两个邻接掩码（该边邻居更粗 `coarserMask` / 更细 `finerMask`）：邻居更细的边上粗节点不 morph（k=0），邻居更粗的边上细节点强制 k=1（奇数边顶点取相邻偶数顶点平均，正好落在粗节点的直线边上）。偶数/角点顶点从不移动，所以两侧折线完全重合（测试：同级与差 1 级的任意相邻对，共享边 10 000 点高度差 ≤ 1e-4 m，实测最大 9.8e-15 m）。需要四叉树满足 2:1 限制，选择后做一次"邻居差 ≥2 级则拆粗者"的平衡（0.08 ms 量级）。未用裙边。
  ②**节点数低于验收 2 的下限**。验收 2 要求节点数中位 ∈ [150, 450]（来自 §9.2 的"≈200–400 节点/帧"估计）。按 §9.2 的同一张表 + 共享 65×65 网格，每个 LOD 环只有约 10–12 个节点，1 000 个随机位姿（块内、高 1.7–32 m、随机朝向、FOV 57°）实测：切分叶子（视锥前）中位 **76**，可见节点（视锥后）中位 **27**（15–46），即 ≈ 0.22 M 三角形（远低于 ARCHITECTURE §13.5.3 的 Medium 2.0 M 总预算）。该估计与表不自洽（估计值更像 33×33 网格或 32 m 叶子）。**未调参去凑数**：上界（≤450）是真实预算守卫，下界不适用；若要 200–400 须改 LOD 表或网格分辨率，属架构决策。
  ③**高度纹理格式**：验收 6 要求近景+远景纹理上传 < 4 MB，而 R32F 的 1025² 远景图单张 4.2 MB。改为把 `.r16` 原样（小端字节）上传为 **RG8**（R=低字节、G=高字节，shader 精确还原 u16 并手写双线性），近景+远景共 2.13 MB，且无需浮点纹理过滤/渲染扩展（WebGL2 与 WebGPU 的 `rg8unorm` 都可过滤、`textureLoad`/`texelFetch` 直读），精度等同 u16（优于 R16F）。
  ④**近景/远景交界**：近景块外 16 m 内，高度由"远景 8 m 双线性"平滑过渡到"近景块边缘值"（smoothstep 权重），块内严格等于 1 m 近景图（与 sim `height_at` 一致）；两张图在块边的原始差实测最大 0.446 m（≤0.5），融合后曲面在块边无跳变。
  ⑤**近景块外最小 LOD3 与四叉树对齐冲突**：LOD≥2 的节点（256 m 及以上）边界以块中心为原点对齐，会跨过 256 m 块边界，故"块外不细于 LOD3"只能表述为"不与近景块相交的节点不再细分到 LOD3 以下"；跨边界节点被拆分时其块外兄弟（LOD0–2）会作为被动兄弟出现（每帧至多几个）。
  ⑥**与 sim 一致性（验收 4）**：sim-wasm（M0-014）尚未提供，故自动化测试对照独立的 numpy float64 双线性参考（`client/scripts/gen-height-ref.py`，1000 点块内最大误差 4.6e-7 m），并另用一个临时 Rust 工程（仓库外）调用 `frontline_sim::CollisionData::height_at` 对同 1000 点做一次性比对（1000 点最大误差 6.7e-6 m）；M0-014 后应把这一步改成 vitest 里调 wasm。
- **证据**：client/src/render/terrain/cdlod-quadtree.test.ts（seam / control / 选择统计 / 计时）、terrain-data.test.ts、client/e2e/terrain.spec.ts、docs/measurements/M0-023/*.png。
- **建议方案**：接受①③④⑤；②由总控/Fable 裁决是否改验收口径（建议：验收 2 改为"可见节点中位 ≤ 450 且 < 0.3 ms"，下限删除）。
- **受影响任务**：M1-049/050（区块槽位/材质沿用实例属性与掩码约定）、M0-030（地形三个机位的节点数/三角形数口径）、M0-024/025（用 `Terrain.heightAt`）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：CDLOD 用每实例邻接掩码 + 2:1 平衡保证无裂缝；高度纹理用 RG8（原样 u16）；近远景有融合带；**节点数验收改为"可见节点 ≤ 450"（删去下限 150——M0 的小地形只有约 27 个可见节点，原下限与 §9.2 参数表不自洽）**。地形入口 `?scene=terrain`，机位占位 `?cam=0|1|2`，素材同步 `pnpm -C client sync-map`。`?debug=1` 的 GPU timer 在 SwiftShader 上极慢，e2e 避开。CPU `heightAt` 与 sim `height_at` 的一致性比对在 M0-014 之后应改成 vitest 调 wasm（由 M0-029 或 M0-025 顺带落实）。　文档回写：未记录。

### PI-015 M0-011：移动/KCC 的数据字段、Rapier 自带 snap 超预算改为解析 snap、aux 约定、速度偏序校验、加减速模型

- **状态**：resolved
- **类别**：接口细化 / 性能 / 数据
- **提出者 / 日期**：M0-011 / 2026-11-17
- **问题**：①**数据字段**：任务要求的移动数值在 role schema 里没有位置。已按"先 schema 可选字段、再 `data_defs.rs`"追加 `movement.{crouch_walk_mps, jump_height_m, stance_transition_ms, air_control_mult, gravity_mps2, max_slope_deg, autostep_max_m, autostep_min_width_m, snap_to_ground_m}`（全部可选，`MoveParams::from_role` 要求齐备）。重力 9.81（GAMEDESIGN §0.5 全局常量）暂放在 role 里，因 `/data/rules/` 尚无 schema 与载入；后续有规则文件时迁移。`faction_overrides.*.loadout_ids` 的 `minItems` 由 1 放宽为 0（任务要求 `rifleman` 暂填 `[]`，M0-012 填充）。②**KCC 耗时**：Rapier KCC 默认 `snap_to_ground` 多一次 shape cast，单次 move 中位数 ≈ 41 µs（>20 µs，`kcc_100_players_tick` ≈ 4.2 ms >3 ms）。关闭 Rapier 的 snap 后改用 `CollisionData::ground_probe` 的解析 snap（胶囊停靠平面的接触高度公式见 `movement/kcc.rs`），单次 ≈ 18–19 µs、100 人一 tick ≈ 2.2 ms（本机、非参考机、测量时负载均值 ≈ 6.7/4 核）。余量仅约 5–10 %：参考机复测前不视为稳；若超标再启用 §5.3 的完全自写 KCC。③**`aux` 约定**：bit0 = 走路修饰键（Shift；站 2.0 / 蹲 1.5 m/s），其余位保留（ARCHITECTURE §6.3 只写"开火模式切换/标记"，开火模式位由 M0-012 在 bit1 起分配）。冲刺需前向分量 > 0 且目标姿态为站；蹲着不能冲刺、不能起跳；`PRONE` 键 M0 忽略。④**速度偏序校验**：`tools/data-validate/rules/ranges.mjs` 原要求 `prone < crouch < walk < run < sprint`，与 GAMEDESIGN §9.1（蹲姿跑 2.4 > 站立走 2.0）矛盾，改为偏序 `prone < crouch_walk < crouch < run < sprint` 且 `prone < walk < run`；无效夹具 `role.speed_order` 改为 walk ≥ run。⑤**加减速**：M0 取"即时到目标速度 + 即时停止"（首 tick 即达稳态速度，停止 1 tick），空中保持水平速度并按 `air_control_mult`（0.3/tick）逼近愿望速度；平滑加速待 REF M-24–M-26。⑥姿态过渡 250 ms @30 Hz = 7.5 tick：线性混合每 tick 前进 1/7.5，clamp 后第 8 tick 完成（`stance_timer` 为剩余 tick 数）。⑦无输入的 tick 重复上一条指令（输入缓冲/校验是 M0-018）。⑧空碰撞（`CollisionData::empty()`）或区块外一律 `blocked=true` 且不动（任务 §6；`server` 用空碰撞时玩家不会移动）。⑨`state_hash` 折入 `Motion`（速度/姿态/标志），`determinism_basic` golden 重钉为 0x63ac47f15c2331d8。⑩`test-util` feature 现在蕴含 `client-predict`（使 `PredictSim` 对拍测试在 `--features test-util` 下运行）。
- **证据**：crates/sim/src/movement/*；crates/sim/tests/movement_*.rs；`cargo bench -p frontline-sim --bench kcc`；reports/baselines/sim-m0.json；tasks/STATUS.md M0-011 行。
- **建议方案**：接受；M0-014/M0-022 按 `PlayerAuthState::to_bytes`（39 B，布局见源码注释）对接；M0-012 在 `aux` bit1 起分配开火模式位；参考机上重测 KCC。
- **受影响任务**：M0-012、M0-014、M0-015（空碰撞下不移动）、M0-018、M0-022、M0-030（基线 JSON）、M1-011（趴姿/体力扩展 `Stance`/状态）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：采纳 + 补充裁定：加减速数据化（role `movement.accel_time_s` 0.25 / `decel_time_s` 0.20，【待调参】【待测】；字段缺省保持瞬时；由 M0-012 落实，见 PI-016 与 GAMEDESIGN §9.1）。

### PI-016 M0-012：加减速落实方式、步枪射击/弹道/命中体的数据与接口细化

- **状态**：resolved
- **类别**：接口细化 / 数据
- **提出者 / 日期**：M0-012 / 2026-11-17
- **问题**：①**加减速（PI-015 补充裁定）**：`MoveParams` 增加 `accel_time_s` / `decel_time_s`（缺省 0 = 瞬时）。地面水平速度每 tick 向愿望速度线性逼近，步长 = 参考速度 × dt / 时间（参考速度 = 所选移动模式的满偏速度，故静止→目标恒为 `accel_time_s`；松手时取按住的按键所对应的模式速度）；愿望速度小于当前速度时用减速时间。**`PlayerAuthState.vel` 语义变化**：地面上存"指令速度"（斜坡/擦墙造成的实际位移损失不回灌到下一 tick 的斜坡逼近，否则 25° 坡速从 82% 掉到 62%，违反 M0-011 验收 5），仅当本次位移不足指令的一半（撞墙、过陡）时存实际速度；空中仍存实际速度。速度类测试改为"稳态后"测量，`movement_replay` golden 重钉（脚本命令保持 30 tick 以免净位移不足）、`determinism_basic` golden 重钉。②**数据**：role `capsule` 增加可选 `eye_height_stand_m` 1.65 / `eye_height_crouch_m` 1.05（出膛点用，`MoveParams::from_role` 要求齐备）；新增 `data/loadouts/{us,ger}_rifleman_1.json`（primary Garand / Kar98k，`ammo_mags` 6 = 6 个备用弹夹/夹子），`rifleman.faction_overrides.*.loadout_ids` 填入；出生武器 = 角色在该阵营的第一个 loadout，弹匣容量取 `historical.magazine_capacity`（无独立 gameplay 字段；Garand 8 / Kar98k 5），备弹 = 容量 × `ammo_mags`。③**命中体**（头球+躯干+大腿+小腿，站/蹲两套随 `stance_blend` 线性插值）写在 `weapon/hitbox.rs` 的常量表，标【待调参】【待测 → M-15】；躯干胶囊最下 25% 记为盆骨（×0.9），颈部 M0 无独立体积（M1-014 细化），大腿/小腿为肢体（×0.65）。④**射击规则细节**：冲刺判定 = SPRINT 位且前向分量 > 0 且非蹲（与移动同一条件）；站↔蹲过渡中（`stance_timer>0`）不可射击（GAMEDESIGN §9.2）；半自动/栓动都要扳机上升沿，开火按下发生在不可射击期间则丢失（M0 无输入缓冲）；空膛上升沿产生 `SimEvent::DryFire`；装填可在任意非装填状态开始（弹匣未满且有备弹），装填中不可射击；ADS 在持续按住 `ads_ms` 后生效，松开/冲刺/装填立即清零。移动散布类别按水平速度：<0.3 m/s 静止 1.0，≤3.2 m/s 走 1.5，其余跑 2.5。后坐全部自动回落（§6.5 的"40% 留给玩家下压"由客户端相机体现，sim 不模拟）。⑤**时间量化**：毫秒×tick_hz 为"毫刻"（1 tick = 1000），余数以不超过 1 tick 的信用带入下一间隔，故任意射速平均间隔精确（140 rpm 的 12.857 tick 实测均值 12.856）。⑥**弹道**：GAMEDESIGN §6.4 的半隐式欧拉在 4 子步/tick 下系统性多算下坠 ≈ ½·g·h·t（h=1/120 s，100 m 处 5 mm、500 m 处 4.4 cm）：对 RK4 的相对误差 500 m 2.0%（Garand）/1.7%（Kar98k）满足 <3%，100/200/300 m 的相对误差 7.7/4.1/2.9%（绝对误差 < 1.3 cm），验收只要求 500 m。弹丸离开已载入碰撞区域即静默销毁（空碰撞世界里弹丸第一子步后消失）；同队与射手自身不命中，已无生命（hp≤0）的玩家不再是目标（死亡处理 M0-013）。⑦**事件**：`SimEvent` 新增 `Shot{shooter,weapon,muzzle,dir}`、`DryFire{shooter}`、`Impact{pos,normal,material}`、`Hit{shooter,target,zone,damage_hp,dist_m}`；因含 f32，`SimEvent`/`StepOutput` 不再实现 `Eq`。`weapon` 为 `GameData::weapons` 下标（u16）。⑧**API**：`World::{weapon_state, health, projectiles, spawn_projectile}`；`PredictSim::{set_entity, reset_weapon, weapon, last_weapon_out}`（`reset_weapon(Some(state))` 在收到 `self_state` 时调用；`set_entity` 提供射击 RNG 流所需的服务端 EntityId）；`WeaponState::BYTES = 33`（布局见源码）。`state_hash` 折入生命、武器状态与弹丸列表。⑨`aux` 位：任务原计划在 bit1 起分配开火模式位，M0 两把枪无模式切换，未分配。
- **证据**：crates/sim/src/{weapon,projectile.rs}；crates/sim/tests/{weapon_*,ballistics_vs_reference}.rs；`cargo bench -p frontline-sim --bench ballistics`；tasks/STATUS.md M0-012 行。
- **建议方案**：接受；M0-014 在 Predictor 里按 PredictSim 的新 API 接武器状态；M0-015 的 self_state 带 `WeaponState::to_bytes`；M0-013 在 `Health` 上做死亡/重生；M1-014 细化命中区（颈/盆骨/头盔）与友伤；参考机上复测命中体尺寸（M-15）。
- **受影响任务**：M0-013、M0-014、M0-015、M0-018、M0-022、M1-012、M1-013、M1-014。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：`SimEvent` 含 `Shot`/`DryFire`/`Impact`/`Hit`（含 f32，不再实现 Eq）；`World::{weapon_state, health, projectiles, spawn_projectile}`、`PredictSim::{set_entity, reset_weapon, weapon, last_weapon_out}`、`WeaponState`（33 B，to_bytes/from_bytes）；地面 `vel` 存指令速度（位移不足一半时存实际速度）；弹丸飞出已载入碰撞区域即销毁；不可射击期间的扳机上升沿丢失（M0 无输入缓冲）；命中体尺寸、眼高、移动散布阈值为初值【待测】。弹道按 GAMEDESIGN §6.4 的半隐式欧拉 4 子步：500 m 下坠对 RK4 误差约 2%，近距离相对误差更大但绝对误差 ≤1.3 cm——保持不变。　文档回写：未记录。

### PI-017 M0-013：规则数据 `data/rules/` 与 `rules.schema.json`（INDEX 未列）、死亡/重生的接口细化

- **状态**：resolved
- **类别**：数据 / 接口细化
- **提出者 / 日期**：M0-013 / 2026-11-17
- **问题**：①`data/rules/respawn.json` 与 `data/schemas/rules.schema.json` 是 M0-004 schema 列表之外的新增（任务文件已说明属细化）：字段 `death_cam_ms / base_wait_ms / wave_interval_ms / wave_aligned / _meta`，另加可选 `spawn_jitter_m`（初值 2.5 m，上限 3 m，使 3D 距离在缓坡上也 ≤3 m；任务写的是"≤3 m 偏移"）。data-validate 新增文件种类 `rules`（`data/rules/*.json`，需 `_meta`，无 id）；`GameData.rules: &'static RulesDef`（`RulesDef{respawn}`）、`build.rs` 对 `rules/respawn.json` 做类型化解析并嵌入；`tests/common::data_with` 的结构体字面量因此多一个字段（外部 crate 若手写 `GameData{..}` 需补 `rules`）。②**死亡状态**：死者保留实体（`EntityId` 不变、尸体位置冻结）、`LifeState::Dead{t_kia,killer,weapon,headshot,respawn_tick}`；`WorldView::players()` 只列存活者（与其文档一致），新增 `WorldView::{health, alive}`（带默认实现，不破坏其它实现者）、`EntityStateView` 增加 `health_q`（8 bit，0=死亡，正 hp 至少 1）与 `alive`；`World::{player_life, set_team_spawn, set_map_center, respawn_pending}`；`SimEvent::{Kill{victim,killer,weapon,headshot,dist_m}, Spawn{player,pos}}`。`headshot` 仅指 Head 区（M0 颈部无体积）。③**时间口径**：波次 tick 为从 tick 0 起每 `wave_interval` 的整数倍；所有时长按 `ceil(ms·hz/1000)` 取整为 tick（30 Hz 下 5/3/15 s = 150/90/450 tick 精确，出生 tick 与公式误差 0）。④**出生点**：sim 不读 spawns.json；服务器启动时对每队调用 `World::set_team_spawn(team, pos)`（HQ 点 xz，y 取地形高 +0.02），未设置时回到该玩家加入时的位置；朝向取已载入碰撞区块的中心（或 `set_map_center`）。偏移用 `Rng::for_(.., Purpose::Respawn)`，圆盘均匀；Y 取地形高度 +0.02 m。⑤**state_hash** 只在死亡状态下追加字节，无死亡场景的既有 golden（determinism_basic / movement_replay）不变；`weapon_duel` 的事件序列 golden 因对射现在以击杀+重生收尾而重钉（0x2ced… → 0x09a7e29170609f65）。
- **证据**：crates/sim/src/{health,respawn}.rs、schedule.rs（`deaths` / `respawns`）、crates/sim/tests/damage_death.rs；data/rules/respawn.json；tasks/STATUS.md M0-013 行。
- **建议方案**：接受；M0-018 在载入地图后用 spawns.json 的 HQ 点调用 `set_team_spawn`；M0-015/019 的快照用 `EntityStateView.health_q` / `alive`（死亡玩家当前不在 `players()` 中，若要在快照里保留尸体由 M0-019 决定）；M1-016 在 `LifeState` 上追加倒地。
- **受影响任务**：M0-014、M0-015、M0-018、M0-019、M1-010、M1-016、M1-019。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：规则数据在 `data/rules/respawn.json`（schema `rules.schema.json`），`GameData.rules`；死者保留实体（EntityId 不变）但不在 `view().players()` 里；`EntityStateView` 有 `health_q`（8 bit）与 `alive`；事件 `SimEvent::{Kill, Spawn}`；`World::{player_life, set_team_spawn, set_map_center, respawn_pending}`。**M0-018 载入地图后必须按 spawns.json 的 HQ 点对每队调用 `World::set_team_spawn`**，否则玩家在加入位置重生。新增数据文件会改变 DATA_HASH——不要在客户端写死哈希字面量。　文档回写：未记录。

### PI-018 M0-014：sim-wasm 的 JS 接口形态、工作区依赖、数据 JSON 来源、构建 profile

- **状态**：resolved
- **类别**：协议/数据 / 接口细化
- **提出者 / 日期**：M0-014 / 2026-11-17
- **问题**：①任务要求 `client/package.json` 加对 `crates/sim-wasm/pkg` 的 workspace 依赖——但 pkg 是被 git 忽略的构建产物，干净检出里不存在，`pnpm install` 会因此失败，并拖垮其它执行者/CI 的 TS 作业。②`decode(bytes): DecodedSnapshot` 若每帧返回 wasm-bindgen 对象会产生 GC；`encodeHello({...})` 的对象参数需要 serde-wasm-bindgen（增大体积）。③`weaponDef/roleDef` 要 JSON bytes，但 `sim` 的 `*Def` 只实现 `Deserialize`，`embedded` 模块私有。④任务写在 sim-wasm/Cargo.toml 的 release profile 在工作区成员里无效。
- **证据**：crates/sim-wasm/{build.rs,src/*}、client/src/net/wasm/、根 Cargo.toml `[profile.wasm-release]`、reports/m0/M0-014-wasm-size.json。
- **建议方案（已按此实现）**：①不加 workspace 依赖；`client/src/net/wasm/{types,layout,index}.ts` 手写类型与 `LAYOUT_*` 常量表（由 `layoutJson()` 与 Rust golden `tests/golden/layout.json` 双向校验），模块对象由 M0-022 注入/导入（pkg 构建后再把 `crates/sim-wasm/pkg` 列入 pnpm-workspace 即可）。②`SnapshotDecoder.decode(bytes)` 返回 tick，列数据用 `ids()/pos()/…` 取零拷贝视图（TS 的 `decodeSnapshot()` 组装成 DecodedSnapshot 形状）；消息函数用位置参数（TS 封装成对象）。③sim-wasm 的 build.rs 读 /data/weapons、/data/roles，去掉 `_meta` 与 `historical._source` 后嵌入（与 sim 同一批文件，哈希一致）；不改 sim。④根 Cargo.toml 新增 `[profile.wasm-release]`（opt-level z、lto、codegen-units 1、panic abort），构建用 `wasm-pack build crates/sim-wasm --target web --profile wasm-release`，不影响服务器/测试 profile。`Predictor.reset` 增第三参（武器状态）、增 `resetWeapon/equipWeapon/setEntity/authState/tick`。
- **受影响任务**：M0-022（接入 wasm、对账）、M0-029（漂移比较）、M0-030。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：构建 `wasm-pack build crates/sim-wasm --target web --profile wasm-release`，体积检查 `node tools/ci/check-wasm-size.mjs`（当前 brotli 约 241 KB）；client 不把 pkg 作为 workspace 依赖（pkg 是忽略的构建产物），用 `client/src/net/wasm/` 的手写类型与 `LAYOUT_*` 常量，wasm 模块由 M0-022 注入；`SnapshotDecoder.decode` 返回 tick，列数据走零拷贝视图访问器；`Predictor.step`/`state` 返回 wasm 内存视图，须在下次调用前用完；消息函数用位置参数。native 与 wasm 已实测 3000 tick 逐字节一致（`crates/sim-wasm/examples/trace.rs` + `tests/native-vs-wasm.mjs`，M0-029 复用）。　文档回写：未记录。

### PI-019 M0-029：漂移度量定义、输入文件不入库、CI 严格模式建议

- **状态**：resolved
- **类别**：测试 / 度量定义
- **提出者 / 日期**：M0-029 / 2026-11-17
- **问题**：①"RMS 漂移 < 2 cm/s"（ARCHITECTURE §20.1-3）未精确定义，按任务备注落为 `err(t)=|pos_native-pos_wasm|`（3D）对时间的最小二乘斜率 ×100（cm/s），同时报告 `rms_m`、`max_m`、`over_0_05m_ratio`、每 60 s 分段最大误差（`tools/determinism/{stats,compare}.mjs`），请确认。②任务要求把 3×18 000 条输入入库（`testdata/determinism/inputs_seed*.bin`）；改为由固定种子的 PCG（`crates/sim-wasm/tests/support/determinism.rs::gen_inputs`，不依赖 `rand`）在运行时生成，并把生成结果写入 `target/determinism/inputs_seed{N}.bin`，golden 终值哈希进 `determinism_native.rs`——避免约 1 MB 二进制入库，且生成器本身有单测。③三个种子的出生点各不相同（平地 / 南坡脚 / 北高地），以覆盖陡坡与区块边界；出生高度取地形高度。④为一致性测试给 sim-wasm 的 `Predictor` 追加了只读查询 `heightAt(x,z)`（NaN 表示该处无碰撞块），无 sim 逻辑改动。
- **证据**：reports/m0/determinism/summary.json（native×2、wasm×2 各 54 000/54 000 tick 一致；native 与 wasm 逐 tick 哈希与位置逐位相同，漂移全为 0）；client/src/render/terrain/terrain-vs-sim.test.ts（506 032 个采样点最大误差 6.9e-6 m）。
- **建议方案**：CI 的 native-vs-wasm 比较建议**开严格模式**（仓库变量 `DETERMINISM_STRICT=1`）：当前 3 种子×18 000 tick 实测逐位一致，严格模式零误报；一旦将来引入 libm 之外的平台差异，会在 PR 上立刻暴露而不是被告警淹没。R5 的"不阻断"原则保留为开关默认值，若出现合理的小幅漂移再关掉严格模式并以 `drift.cm_per_s` 为准。native×2 / wasm×2 / World×2 永远阻断。
- **受影响任务**：M0-022（对账阈值 0.05 m 目前在 18 000 tick 内零次触发）、M0-030。
- **临时处理**：按此实现；默认警告模式，严格模式由开关启用。
- **裁决（Fable 填写）**：总控裁定（2026-11-17 resolved）：入口 `tools/determinism/run.sh [--strict]`；3 种子 × 18000 tick 的 native×2 / wasm×2 / World×2 无论何种模式都阻断；native vs wasm 的比较**裁定 CI 默认开严格模式**（当前实测逐位一致；将来若出现合理的平台差异再关闭并改看 drift.cm_per_s）——下一个改动 .github/workflows/determinism.yml 的执行者请把严格模式设为默认开启。`Predictor.heightAt(x,z)` 已暴露；客户端地形与 sim 的高度一致性测试在 client/src/render/terrain/terrain-vs-sim.test.ts。　文档回写：未记录。


### PI-020 M0-024：植被实例表 §9.6 的"6 B"与字段宽度不自洽；植被入口、perf 导出与瞬移后的重分桶

- **状态**：open
- **类别**：接口 / 数据格式
- **提出者 / 日期**：M0-024 / 2026-11-17
- **问题**：①ARCHITECTURE §9.6 写 `(x:u16, z:u16, species:u8, scale:u8, rot:u8)` = 6 B，但字段宽度之和是 7 B（2+2+1+1+1）；任务验收 3 与 §9.6 都以"每条 6 B"为硬约束。②入口：`?scene=terrain` 保持纯地形（M0-023 的 e2e 断言地形 draw call = 1），植被另设 `?scene=vegetation`（= 地形 + 植被）。③`window.__perf.dump()` 增加顶层字段 `vegetation{drawCalls, instances, tris}`（无植被时为 0；schema 号不变，属加字段）。④相机瞬移（出生/复活/调试 setPose）后，默认的 0.45 ms 重分桶预算会把全量重分类摊到数帧，期间远处区块沿用旧 LOD。
- **证据**：client/src/render/vegetation/{scatter,vegetation}.ts；scatter.test.ts（round-trip）；e2e/vegetation.spec.ts。
- **建议方案（已按此实现）**：①保留 6 B，把 x、z 各压成 12 bit（块内 1/64 m = 1.5625 cm 步长，块边 4096 步），species/scale/rot 仍各 u8：`b0=x[7:0] b1=x[11:8]|z[3:0]<<4 b2=z[11:4] b3=species b4=scale b5=rot`（`packInstance`/`unpackInstance`）。撒点在量化后的位置上判泊松间距，所以 4 m/1.5 m 间距对存储值严格成立。**M1-035 的 map-pipeline 烘焙必须输出同一布局**（或先改本条再改烘焙）；也可改为 x/z 各 u16 + 7 B，需同步 §9.6 与验收 3。②③按此实现，M0-030 用 `?scene=vegetation&cam=0|1|2`。④瞬移后调用 `Vegetation.settle(camera)`（无预算，≈ 1–3 ms 一次）。
- **受影响任务**：M0-030（入口与 perf 字段）、M1-035（实例表布局）、M1-051（真素材替换占位，接口以实例表字节为界不变）、M1-052。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-021 M0-028：ARCHITECTURE §5.0 浏览器支持表的实测核对（只写实测事实）与互通测试的前提

- **状态**：resolved
- **类别**：测试 / 事实核对
- **提出者 / 日期**：M0-028 / 2026-11-17
- **问题**：§5.0 的 WebTransport 行（Chrome/Edge 2021 末起；Firefox 114 起；Safari 26.4 起）与 §5.5/§18.2 的"`serverCertificateHashes` 各浏览器支持未核实"需要实测。本机只有 Playwright 预装的 Chromium 153.0.8010.12，没有 Firefox / WebKit 构建，且不允许运行 `playwright install`。
- **证据**（reports/m0/browser-matrix.json，`scripts/run-interop.sh`）：
  - **已被实测证实**：Chromium 153.0.8010.12（headless，Linux）可用 `WebTransport` 连接 `wtransport =0.7.2` 服务器；`serverCertificateHashes`（sha-256、13 天 ECDSA P-256 自签名证书）被接受；datagram（C1 上行 / C2 回显下行）与可靠流（C0/C4）互通；10 s 内 199/199 个 C2 包（WT 与 WS 各一次），0 断线，握手后 RTT 中位约 1.0 ms（loopback）。WS 兜底同样通过。
  - **已被实测证实**：`delete window.WebTransport` 与"服务器不监听 WT 端口"两种情况下，`auto` 在 Chromium 上都回退到 WS，耗时约 10–14 ms（多次运行；≤ 5000 ms）；UDP 黑洞（QUIC 握手无响应）情形见 M0-021（3.2 s 回退）。
  - **未验证（本机无该浏览器构建，不是失败）**：Firefox（WT 自 114、`serverCertificateHashes` 支持）、WebKit/Safari 26.4（WT、`serverCertificateHashes`、datagram 细节）。表格中这些格子写"未验证"及原因；复测步骤见 DEVELOPING.md "Browser interop matrix"，人工项 H-103。
  - **未推翻任何 §5.0 条目**；§5.0 中 WebGPU 与覆盖率条目不在本任务范围。
  - 注意：当前服务器尚无 AOI/快照（M0-019），探针数到的"快照"是 `--dev-echo` 的 C1→C2 回显（JSON 中 `snapshotSource: "dev-echo"`）；M0-019 完成后复跑 `scripts/run-interop.sh` 即得到真实快照的计数（`--dev-echo` 被移除时探针自动记为 `server`）。
- **建议方案**：①§5.0 暂不改文字，待 H-103 或 CI 矩阵（`.github/workflows/interop.yml`，nightly）给出 Firefox/WebKit 数据后，由总控把"未核实"改成实测结果；②M0-019 完成后复跑一次矩阵。
- **受影响任务**：M0-019（复跑）、M4-042（矩阵起点）、M0-018（若移除 `--dev-echo`）。
- **临时处理**：Firefox / WebKit 格记"未验证"，不记失败。
- **裁决（Fable 填写）**：按建议方案执行；本条在 H-103 / CI 矩阵出数前保持 resolved（事实已登记），§5.0 的"未核实"字样由总控在拿到数据后更新。

### PI-022 M0-018：8000°/s 转速上限在 30 Hz 下无法触发；缺失输入时清除边沿位（§6.3 细化）；减速与"第 6 tick 速度为 0"；Hello 无昵称

- **状态**：open
- **类别**：数值 / 接口细化
- **提出者 / 日期**：M0-018 / 2026-11-17
- **问题**：①任务与 ARCHITECTURE §15.1 的"yaw/pitch 角速度上限 8000°/s"折算到 1 tick 是 266.7°，但 yaw 是 16 位环绕码、pitch 是 16 位覆盖 180°，两个相邻 tick 间**可表达的最大变化只有 180°**（yaw 取最短弧；pitch 全量程 180°）。所以默认上限在 tick 间隔 1 时永远不会触发，验收 3 "yaw 在 1 tick 内变化 > 266° → 被钳制"在线格式下不可构造。校验器已按 `max_turn_dps * Δtick / 30` 实现并有测试（把上限降到 2000–3000°/s 即触发并计数 `yaw_rate`/`pitch_rate`），但默认值在 M0 等于不设防。②§6.3 原文只说"重复上一条移动输入"。实现为：重复时**清除边沿位**（FIRE/RELOAD/JUMP/USE/MELEE/GADGET/SWAP/VEHICLE_SEAT 与 `aux` 的非走路位），保留电平位（AIM、姿态、冲刺、探头、走路修饰），gap>5 起清零移动并去掉 SPRINT，保持视角（防止一次性动作被无限重放）。③验收 2 写"第 6 tick 起速度为 0"：输入侧确实从第 6 tick 起是零移动（已断言），但 sim 按 PI-015 的 `decel_time_s`（0.20 s）减速，实测速度 4.4 m/s 在第 6 tick 起线性降到 0.73，**第 11 tick 才精确为 0**，此后位移为 0。④`Hello` 没有昵称字段，`Join` 命令只带 `player/team`（任务接口写了 nick）。⑤新增服务器参数：`--data-dir`（默认自动向上查找 `data/maps/<map>`）、隐藏的 `--empty-world`（仅测试）；`--dev-echo` 保留为隐藏的测试脚手架，改为"C1 回显取代会话层"并隐含 `--empty-world`。
- **证据**：crates/server/src/validate.rs（`rate_limit_with_default_cfg_is_unreachable_for_one_tick`、`turn_rate_over_the_limit_is_clamped_and_counted_with_a_lower_cap`）；tests/input_buffer.rs（`ten_silent_ticks_repeat_five_then_hold_still_and_never_replay_fire` 打印速度序列、`each_rejection_reason_has_its_own_counter`）。
- **建议方案**：①M1 实测鼠标后改用"滑动窗口累计转角"（如 1 s 内 |Δ| 总和 ≤ 8000°→ 允许 26 圈/s 以上才拦），或把单 tick 上限降到 ≤ 150°（4500°/s）使其真正生效；M0 保持不收紧（任务备注）。②把"边沿位清除"写进 ARCHITECTURE §6.3 变更说明。③验收 2 改成"输入从第 6 tick 起为零移动，速度在 decel_time_s 内降为 0"。④若要昵称需在协议加字段（M1-040 前）。
- **受影响任务**：M0-019（读取 `PlayerInputState.last_ack_snapshot_tick`）、M0-026（bot 走同一入口 `validate_input`）、M0-027、M1-025、M1-027（反作弊）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：


### PI-023 M0-022：pred_hash / pos_corr / ack_snapshot_tick 的精确语义；事件位置码米制；Shift 键位冲突

- **状态**：open
- **类别**：接口 / 协议语义 / 键位
- **提出者 / 日期**：M0-022 / 2026-11-17
- **问题**：①ARCHITECTURE §6.5.3 与 M0-019/022 任务卡只写"位置量化后 CRC8"，没说：哈希对应**哪个 tick/命令**的位置、`ack_snapshot_tick`（"16 bit 增量"）相对谁取增量、`pos_corr` 是哪一刻的位置、哈希函数的量化与字节序；任务卡要求 sim-wasm 暴露 `predHash(posBytes)`，但 wasm 包没有（且本任务不改 crates/）。②Shot / Impact 事件的 14 位位置码"米制含义由消费者定义"（协议注释），客户端必须先定一个。③任务验收 4（"Shift = walk 位"，M0-011/GAMEDESIGN §9.1）与 UI_SPEC §5.2（Left Shift = 冲刺）冲突；UI_SPEC §5.1 另禁止 Ctrl 作核心键。④`HitConfirm` 事件无击杀位；快照实体记录无"种类"字段；本地玩家在快照实体表里的线上 id 与 `player_id` 的对应关系未定。
- **证据**：client/src/game/{pred-hash,events,snapshot-apply,game-loop}.ts、client/src/input/keymap.ts、client/src/game/{pred-hash,reconcile,game-loop}.test.ts（黄金值：`predHash(128,2.4,56)=66`、`(100.40625,12,300)=175`、`(255.97,200,3.5)=151`、`(0,0,0)=0`，由独立 Python 实现交叉核对）。
- **建议方案（已按此实现，M0-019 请照此实现或回复修改）**：①`ack_snapshot_tick` = 客户端最近一次收到的快照 `server_tick` 的**低 16 位**（服务器用环绕比较还原）；`pred_hash` = 客户端在该快照的 `ack_input_seq` **之后**的预测位置（若该快照带了被接受的 `pos_corr`，则取该权威位置）的哈希；服务器与自己**发出该快照时**的玩家位置（即执行完 `ack_input_seq` 对应命令后的位置）比较，不等才在下一包的 `self_state` 带 `pos_corr`，`pos_corr` = 下一包自身 `ack_input_seq` 之后的权威位置（1/32 m 量化）。哈希 = CRC-8（多项式 0x07，初值 0，无反射/异或）over 6 字节小端 `[x u16, y u16(13 bit), z u16]`，码 = `floor(v*32+0.5)` 钳位到 0..65535 / 0..8191（同 `protocol::quant::quant_pos_*`）。若 M0-019 愿意，可在 sim-wasm 增加同名 `predHash(x,y,z)` 并让客户端改调用（pred-hash.test.ts 的黄金值即对拍样本）。启动时客户端起点是猜的，`ack_input_seq=0`（无条目）+ 哈希不匹配会让服务器在第 1 个快照里带 `pos_corr`，客户端把它当 initial sync（不计纠正）。②M0 暂把事件 14 位位置码当作覆盖 256 m 的 M0 地图（`EVENT_POS_STEP_M = 256/16384`），方向 yaw 10 位 / pitch 9 位（`PITCH_BITS_EVENT`）；M1 定米制时改这一个常量。③Shift = walk（任务验收与 sim `AUX_WALK` 一致），冲刺暂放 Right Shift，等 M2-041 重绑定界面；UI_SPEC §5.2 的默认键位表需要裁定"Shift 到底是走还是冲刺"。④`hit.confirm.killed` 恒 false（击杀走规则事件，M1）；`entity.spawned.kind` 恒 `'player'`；`selfWireId` 默认取 `player_id`（线上实体 id 若不同，M0-019/025 调整）。
- **受影响任务**：M0-019（pred_hash 比较与 pos_corr 内容）、M0-025（`GameState` / 事件接线）、M0-027、M1-041（载具预测的哈希）、M2-041（键位）、M1-025（FireCmd）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：
- **M0-025 落实**（2026-11-17）：①键位：冲刺 = Left Shift（按住）；慢走（`aux` bit0）默认不绑定，M0 调试键 `CapsLock`；Right Shift 不再绑定；`client/src/input/{keymap,input.test}.ts` 已按此更新（原 "Shift = walk" 测试作废）。②`predHash` 已由 sim-wasm 导出（`crates/sim-wasm/src/lib.rs`，调用 protocol 的唯一实现），`client/src/game/pred-hash.ts` 已删除，GameLoop / 测试 harness 改调 wasm，`pred-hash.test.ts` 经 wasm 对拍同一组黄金值。
- **M0-019 对齐情况**（2026-11-17，按 PI-023 实现，无修改）：①`ack_snapshot_tick` 低 16 位用"不超过本客户端最近发出的快照 tick 的、低 16 位相同的最新 tick"还原（`ClientBook::extend_ack`，环绕测试）；只承认服务器真发出过的 tick。②服务器为每个发出的包记录 `(tick, CRC-8(该包所告知的自身位置))`，客户端 `pred_hash` 与"其 ack 所指快照的记录"比较；**每个客户端的第 1 个包必带 `pos_corr`**，之后只在不匹配时置位；置位后**每个包**都带 `pos_corr`（位置 = 该包 `ack_input_seq` 对应命令之后的权威位置），直到收到 ack ≥ 首个携带包的 tick 且哈希匹配才清除（丢包安全）；不匹配/清除各只在"更新的 ack"到来时评估一次，旧 ack 不会重新触发。③哈希只有一份实现：`crates/protocol/src/snapshot/pred_hash.rs`（`crc8`、`pred_hash(x,y,z)`、`pred_hash_q(QPos)`，纯函数，无新依赖），单元测试用 `pred-hash.test.ts` 的 5 个黄金值（66、175、151、0、16）+ CRC-8 检验值 0xF4 对拍通过；**后续应由 sim-wasm 导出 `predHash` 并替换 `client/src/game/pred-hash.ts`（接线留给 M0-025）**。④线上自身实体 id = `player_id`（实体记录的 id 即 PlayerId，0..127；服务器不向客户端发它自己的记录，自身状态走 `self_state`），客户端 `selfWireId = player_id` 的假定成立，无需映射。⑤`ack_input_seq` = sim 本 tick 实际用于该玩家的命令的 seq（重复/保持输入时仍为最后一条真实命令的 seq）；`input_slack` = 最近 8 条被接受命令的"到达时距执行 tick 的提前量（tick）"的最小值，钳到 0..15，无样本时为 1。⑥`HitConfirm` 无击杀位、事件 14 位位置码 256 m/16384（1/64 m）均按 crates/protocol 现状实现，未扩展协议。

### PI-024 M0-019：AOI v0 / 快照发送的实现细化，以及与 PI-010 提示相悖的实测

- **状态**：open
- **类别**：接口细化 / 实测偏离
- **提出者 / 日期**：M0-019 / 2026-11-17
- **问题 / 做法**（`crates/server/src/snapshot/*`）：
  1. **R3 现在就做**（总控裁定，任务卡原定 M1-024）：敌方实体仅当 ≤96 m，或 ≤400 m 且在视锥 +30°（半角 100°，已可见则保持到 110°）内才进候选；友军 ≤400 m（迟滞 440 m）；离开候选的实体立即发 `entity_remove`，连续 3 个包重复（客户端对已删除 id 的 remove 是空操作）。Shot 事件按同一规则路由（敌方视野外的枪声不下发）；`HitConfirm` 只给射手，`DamageDir` 只给被害者，`Impact` 给 400 m 内所有人。团队通道（C3）与载具不在本任务。
  2. **PI-010 的"近/中景每包都出现"提示被实测推翻**：`SnapshotScheduler::near_every_packet`（≤96 m 实体每包出现、全部走 delta）默认**关**。40 人、30 s 同场景对比（`snapshot_bandwidth::near_every_packet_policy_comparison`）：开 = 线路估计 P50 36.7 kbps、关键帧记录 9.4 %；关（规格速率 20/10/5/2 Hz）= 30.2 kbps、关键帧记录 75.7 %。原因：真实移动玩家的近景 delta 是 8–10 B（M0-006 的 4–6 B 是匀速直线），关键帧 12.5 B，10 Hz 关键帧（125 B/s）比 20 Hz delta（≥160 B/s）便宜。取舍：默认遵守 §6.5.4 的速率；需要更平滑的 T1 时打开该选项，代价约 +20 % 字节。关键帧占比因此是 65–85 %（按记录数），但字节数低于预算很多。
  3. **累加器**：只在该客户端出包的 tick 评估（`acc += rate × Δt`，Δt = 距上次评估的时间，等价于每 tick 累加），速率再被该客户端的包速率（20 或 15 Hz）封顶以免积压；`acc` 上限 20；`acc ≥ 10` 者强制先发；同分按 id 升序；新进入候选的实体 `acc` 置 1（首包必发）。"新进入实体首包全量关键帧"不单独强制：若它出现在 ack 基线包里（≤32 tick 内离开又回来）则以 delta 发送，解码结果与关键帧等价。
  4. **线上实体记录**：id = PlayerId；`flags` bit0 正在开火（近 6 tick 有 Shot）、bit1 近 1 s 受伤/击杀相关、bit7 阵营（0 US / 1 GER）；`anim` bit0–2 移动类别（0 静止 1 走 2 跑 3 冲刺 4 空中）、bit3 换弹；`stance` = sim 的 `Stance`（0 站 / 1 蹲）；`weapon` = `def_id & 63`；`health_q` 8 位。`self_state`：`cooldown_bits` bit0–2 = 武器 FSM 码（0 就绪 1 冷却 2 拉栓 3 战术换弹 4 空仓换弹 5 空仓）、bit3 扳机按住、bit4 开镜；`stamina/suppression/deploy_state` 恒 0。
  5. **事件**：全局递增 `event_id: u16`（客户端 128 个窗口去重）；每个事件每客户端最多随 3 个包发送，收到携带它的包的 ack 即停；位置码 1/64 m（PI-023）；法线 12 位八面体（6+6）；`Shot` 不发给射手自己（客户端本地已预测）；`DamageDir.angle` = 攻击者方位相对被害者朝向，0 = 正前，顺时针，`/256 × 2π`。实测 30 % 下行丢包时事件送达率 98.1 %（2748/2800，≥97 % 的目标；理论 1 − 0.3³ = 97.3 %，边缘值）。
  6. **出包节奏**：20 Hz = 除 `tick % 3 == player_id % 3` 以外的 tick；WS（无 datagram 路径）= 每 2 tick 一包（15 Hz，`(tick + player_id) % 2 == 0`）且不带 CRC；连续 3 次 tick 超时 → 全体降为 15 Hz，持续 90 tick 并随超时刷新，计数 `snapshot_degraded_ticks_total`。`PACKET_BUDGET = min(1150, max_datagram_size − 1)`。机器人（无连接的会话）不出包。
  7. **输入侧小改**（crates/server/src/session.rs，M0-018 的文件）：`PlayerInput::{last_applied_seq, input_slack}`；`SimBridge::{net, sessions}` 访问器。`sim_thread` 的 `tick_body` 现在保存 `StepOutput.events`（`step_events`）供快照与 bot 使用。
  8. **C2 联调**：`client/e2e/net-support.ts` 的 `startServer` 默认不再传 `--dev-echo`（改为 `echo: true` 才传），`interop/server.ts` 注释同步；`reports/m0/browser-matrix.*` 重跑后 C2 来源记为 `server`，STATUS M0-028 行的 Chromium 格已按新结果更新（`scripts/check-interop-table.mjs` 一致）。`--dev-echo` 保留为隐藏的纯测试开关。
- **证据**：tests/{aoi_priority,snapshot_bandwidth,snapshot_loss,snapshot_net}.rs、benches/aoi_encode.rs、reports/baselines/server-m0.json；数字见 tasks/STATUS.md M0-019 行与 docs/PROGRESS.md。
- **建议方案**：①M1-024 的 AOI v1 在此基础上加团队通道与标记；②若 M1 要把 T1 提到 20 Hz 平滑，打开 `near_every_packet` 并重测带宽；③C3 团队通道、载具记录、带宽调节器（M2-020）未做。
- **受影响任务**：M0-025（`predHash` 换 wasm、flags/anim 位含义）、M0-027、M1-024、M1-010、M2-020。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-025 M0-025：灰盒武器尺寸与第一人称视图参数放在 data/placeholders/（而非 weapon.schema 的 fx.dimensions）；占位命名统一为 `__` 分隔

- **状态**：open
- **类别**：数据位置 / 命名
- **提出者 / 日期**：M0-025 / 2026-11-17
- **问题**：任务卡要求枪的长度、枪口位置等"写在 /data 的 fx/dimensions 字段"，但 `weapon.schema.json` 的 `fx` 目前只有 `muzzle_flash_ms`/`tracer`，且 `crates/sim/src/data_defs.rs`（deny_unknown_fields）要同步修改；本任务被限定不改 crates/sim。另：M0-023/024 的占位名（`PH_V-NAT-007_terrain`、`PH_V-NAT-004_shrub` 等）用单下划线，不匹配验收 2 的 `^PH_V-[A-Z]+-[A-Z0-9-]+(__.+)?$`。
- **证据**：client/src/render/actors/actor-data.ts；data/placeholders/weapon_models.json（`build.rs` 对未知目录只计入 DATA_HASH，不做类型检查；`tools/data-validate` 不扫该目录）；client/e2e/actors.spec.ts 的命名检查。
- **建议方案**：现状：灰盒尺寸（枪各部件尺寸、第一人称握持/开镜偏移、ADS FOV 倍率、`hip_fov_deg`）放 `data/placeholders/weapon_models.json`，客户端运行时校验（正数/三元组）并有单测；胶囊、眼高、站蹲过渡、枪口焰时长、阵营、初速、ads_ms 都来自 role/weapon 数据。M1-046 换真模型时删除该文件；若要保留为正式字段，再在 sim 侧加 `fx.dimensions`。命名：把 M0-023/024 的网格/材质名改为 `__` 分隔（`PH_V-NAT-007__terrain`、`PH_V-NAT-004__shrub`、`…__impostor`），对应 e2e 引用同步。
- **受影响任务**：M0-030（读 `window.__placeholders`）、M1-001（缺失素材报告）、M1-046。
- **临时处理**：按此实现；新增该文件会改变 DATA_HASH（服务器与 wasm 需用同一份 data 重建）。
- **裁决（Fable 填写）**：

### PI-026 M0-026：bot 数值表 `data/rules/bots.json`、`WorldView::slope_deg_at`、默认射击距离与实测不符、bot 的 PlayerId/队伍约定

- **状态**：open
- **类别**：接口细化 / 数据新增 / 数值偏离
- **提出者 / 日期**：M0-026 / 2026-11-17
- **问题 / 做法**：
  1. **数据表**（任务卡要求"难度表放 `data/rules/bots.json` 并加 schema"）：新增 `data/rules/bots.json`（难度三档 Recruit/Regular/Veteran 取自 GAMEDESIGN §20.6，另含 `think_interval_ms`、`perception`（FOV 120°、视距上限 500 m、最小识别概率）、`nav`、`combat`、`stuck`）、`data/schemas/bots.schema.json`、校验器新种类 `bots`（`rules/bots.json` 专用；`reaction_ms` 必须 min ≤ max）、两份无效夹具；`sim` 的 `build.rs` 做 `deny_unknown_fields` 类型检查（`BotsRulesDef`），`GameData::rules.bots` 暴露给 `bots` crate。**这是 sim 里唯一的改动，没有逻辑变化**；代价：**DATA_HASH 改变**（客户端 wasm 与服务器必须同一份 data 重建）。
  2. **`WorldView::slope_deg_at(x, z) -> Option<f32>`**（任务卡"如缺则补"）：默认实现用 `height_at` 的 1 m 中心差分，`World` 覆盖为碰撞世界的三角面斜率（与 KCC 同一份）。纯只读追加，现有实现者不受影响。
  3. **默认射击距离与实测不符**（数值偏离，数据里可调）：任务卡写"≤ 80 m 盲射，更远 ADS"。M0 武器数据的盲射散布半角是 3.5°（Garand）/ 4.0°（Kar98k），15 m 处已有约 0.9 m 的 1σ，80 m 处 4.9 m；固定靶 15 m 盲射 18 发只命中 2 发，ADS（0.15°，起瞄 300 ms）同距离全中。因此 `combat.blind_fire_max_m` 取 **12 m**（不是 80 m），`approach_beyond_m`（超过则先接近再射击，新字段）取 50 m，`crouch_min_m` 35 m；机动靶的瞄准提前量由 bot 用两次目击估速度实现。这些数字全部在 `_meta.tunable` 里，M1 武器数据复核后重调。
  4. **bot 的 PlayerId/队伍**：`--bots N` 的 bot 占用 127、126、…（真人取最小空闲 id，`Sessions::reserve_bot` 把 bot 的槽永久移出分配并计入队伍座位，所以真人加入仍按"人少的一队"平衡）；队伍按序交替 US、GER。`server_players` 只数真人，bot 数在 `server_bots`；bot 昵称 `[BOT] NN` 只保存在 `BotHost`（Hello 无昵称，M0 没有昵称线上字段，M1 的名牌/记分板再用）。
  5. **感知的一处严格化**：目标从玩家列表里消失（死亡/重生）不算"可见"，bot 不知道它是死了还是躲了，按记忆（`memory_s`）追到最后已知位置；视野内才更新位置与速度。M0 **没有听觉**（B-05 的枪声方向留给 M1-038）、没有掩体/手雷/弹药补给——弹药打光（6 个弹匣，无补给）的 bot 退回巡逻，直到死亡重生。
  6. **A\*** 用 3/2 加权的八向启发（路径最多 1.5 倍最优，平滑后实际近似最优），换最坏情况扩展节点数 1556 → 993、单次最坏 1.4 ms → 0.2–0.6 ms，满足"路径查询 ≤ 1 ms/tick"的逐 tick 上限；不加权版本在 100 bot 下偶有单 tick 2.6 ms。
  7. **宿主接线**：`sim_thread::{build_loop_with_bots, spawn_with_bots}`（`SimThreadConfig` 与 `build_loop`/`spawn` 签名不变，避免打断并行任务）；`bots_think` 在 `World::step` 之后运行，bot 的第 T+1 tick 指令经 `SimBridge::ingest_cmd`（窗口检查 + `validate_input` + 输入环）进入，与真人同一条路径；`SimLoop::{bot_host, tally}` 供测试读取（`EventTally`：shots/hits/kills/headshots/spawns，读自 M0-019 保存的 `step_events`）。
- **证据**：crates/bots/、crates/server/src/bot_host.rs、tests（crates/bots/tests/{nav,perception,behaviour,golden}.rs、crates/server/tests/{bots_soak,bots_validation}.rs）、crates/bots/benches/think.rs、reports/baselines/bots-m0.json；数字见 tasks/STATUS.md M0-026 行与 docs/PROGRESS.md。
- **建议方案**：①M1-038 的行为树/效用 AI 替换 `SoldierBrain`，保持 `BotBrain` 与 `BotHost`；②M1-036 的 navmesh 替换 `grid_nav`，`BotMap` 的接口（`nav`、`patrol`）可保留；③武器数据复核后重调 `combat.*`（尤其 `blind_fire_max_m`）；④真人/bot 昵称上线字段由 M1 决定。
- **受影响任务**：M0-027（10 headless + 10 bot 联测）、M1-036/037/038/039、M2-028、M1 武器复核。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-027 M0-032：对标测量工具链的口径细化、`_meta.ref` 字段、M0 sim 缺失的测量项、docs/measurements 里的非测量图片

- **状态**：open
- **类别**：接口细化 / 数据新增（待办）/ 验收口径
- **提出者 / 日期**：M0-032 / 2026-11-17
- **问题 / 做法**：
  1. **录像缺失，B 部分做不了**（H-011/H-025/H-026）：A 部分（工具链、模板、我方复测、回归脚手架、人类清单）已完成，参考作品的测量值保持"待测"。STATUS 标 `partial`（而非任务卡 §验收 6 的 `blocked`），因为 A 部分及我方数值已交付；B 部分的唯一缺口是录像。
  2. **`/data` 的 `_meta.ref` 字段尚未加入 schema**：`ref_regression.rs` 约定 `_meta.ref{<字段路径>: {ref_id, ref_version, ref_median, kind, state: calibrated|locked, ref_unc?, tol_abs?}}`（见该文件头注释）。现在没有任何测量值，加字段只会白白改变 DATA_HASH，所以**推迟到第一批测量回填时**，与 `data/schemas/*.schema.json` 的可选 `_meta.ref` 和 `crates/sim/src/data_defs.rs`（deny_unknown_fields）同一改动落地。当前该测试对 0 个 locked 项通过（并用我方自测值构造的夹具证明判定逻辑：容限内通过、容限外/不可测/未知 ID/未知 kind 失败）。
  3. **口径细化（行 ID 与 §4 不变，只把"统计量"写死，使录像与我方同定义）**：H-01 按近/中/远拆成 `H-01.<w>@50m/@150m/@300m` 三行（批 M0 共 78 行而非 74）；W-19/W-20 的统计量 = 弹着点相对瞄准点的**角偏差中位数**（P95 记备注）；W-21 = 跑动/静止中位角偏差之比（单位改"比值"）；W-16 = 每发水平位移**绝对值**的中位数；W-22 = 飞行平均速度 `v = d/t`（≥200 m，不是初速，drag 使它低于初速约 9 %：Garand 773 对 853，Kar98k 696 对 760）；W-12/M-12/M-13 等用线性过渡的斜率换算连续时间。
  4. **M0 sim 里不存在、我方复测只能报"不可测"的项**：M-05、M-14（匍匐，M1-011）、M-17（翻越）、W-05（逐发装填，M1-013）、W-06（装填打断）、W-09（FOV，客户端量）、W-11/W-12（切枪）、W-27–W-29（晃动/屏息）、W-26（枚举）。若 B 部分测得这些项，对应行 `pass=na`，不能满足验收 3 的"所有 measured 项 pass==yes"——建议把"我方不可测"从该条里豁免，待对应功能（M1）实现后再复测。
  5. **`docs/measurements/` 里已有其它任务的验收截图**（M0-023/024/025 的 PNG 共 20 个），与验收 5 "`docs/measurements/` 内无图像文件"字面冲突；它们不是参考作品图像、也不是本任务产物，本任务未动。建议总控把这三个目录迁到 `reports/m0/screens/`（或改验收 5 为"无参考作品图像"）。
- **证据**：tools/ref-analysis/（69 项测试全绿）、crates/sim/tests/{own_remeasure,ref_regression}.rs、docs/measurements/{own_m0.json,own_m0.md,CAPTURE_GUIDE_M0.md}。
- **建议方案**：按上；B 部分由 M0-032b 在录像到位后执行（先 `status -v` 看缺项）。
- **受影响任务**：M0-032（B 部分）、M0-033（验收时参考作品数值"待测"是预期状态）、M1-011/M1-013（实现后补测 4 中的项）、M4-044（版本漂移复测复用工具）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-028 M0-027：headless 客户端必须做预测（任务卡"不预测"不可行）、初始同步的量化偏移、`step_overhead` 改为 release 才断言

- **状态**：open
- **类别**：接口细化 / 验收口径
- **提出者 / 日期**：M0-027 / 2026-11-17
- **问题 / 做法**：
  1. 任务卡写"`pred_hash` 固定使用权威位置量化 CRC8——不预测"。服务器不发自身实体记录，客户端只有在 `pos_corr` 出现时才知道权威位置，无预测就无法给出匹配的哈希（服务器会每包都带 `pos_corr`）。做法：headless 默认复用 `sim::PredictSim` 预测并按 PI-023 对账（`crates/headless-client/src/predictor.rs`，与 TS 版同构）；报告新增 `corrections{total,snaps,around_death,tolerance_kept,...}`。
  2. 死亡期间服务器不消费输入，预测需冻结，否则每次死亡产生数次回滚；重生是传送（≥1 m），回滚应从全新的 `PlayerAuthState::spawn` 重放而非沿用死前速度/姿态。浏览器客户端（M0-025/M1）需同样处理：死亡后停止预测、重生按初始同步处理。
  3. 初始同步用量化的 `pos_corr`（xz 1/32 m）作起点，预测轨迹与服务器的 f32 轨迹相差 ≤1.6 cm 且永不收敛，于是约 30–50 % 的快照仍带 `pos_corr`（误差全在 5 cm 容差内、不触发回滚，实测 `tolerance_max_err_m` ≈ 0.03 m），每包多 6 B（约 +1 kbps）。可接受；若要消除，服务器端可在"容差内连续 N 次匹配"后停发，或哈希改用更粗的量化。留给 M1 对账调优。
  4. `crates/server/tests/step_overhead.rs` 的 `hundred_players_empty_world_p99_under_1ms` 是毫秒级计时断言，debug 构建在负载高的虚机上偶发失败：现仅 release 启用（`cfg_attr(debug_assertions, ignore)`），并新增在所有构建都运行的功能性用例 `hundred_players_loop_runs_150_ticks`。
  5. **服务器 RSS 缓慢增长（仅 WT 路径）**：30 min WT 联测（10 headless + 10 bot）服务器 RSS 11.9→17.2 MB，近似线性约 1.9 KB/s，最后 10 min 增长 7.8 %（验收 4 要求 <2 %）；同配置 WS 10 min 无增长。绝对量很小，未定位来源（疑 wtransport/quinn 每连接状态或某个无上限容器），建议 M1-067 前用 heaptrack/计数器查证；已把数据留在 reports/m0/loadtest-wt.json（`rss_samples`）。
  6. 服务器每 IP 最多 4 个连接，同机多客户端须各绑 `127.x.y.z`（`--no-bind-spread` 可关）；服务器 `--report` 增加 `events{shots,hits,kills,headshots,spawns}`（`sim_thread::Report` 追加一个字段）以便联测报告击杀/重生。
- **证据**：crates/headless-client/、scripts/loadtest-m0*.sh、scripts/summarize-loadtest.mjs、reports/m0/。
- **建议方案**：按上；M1-068（netem）与 M2-049 复用 `scripts/loadtest-m0.sh`。
- **受影响任务**：M0-025/M1-040（死亡/重生时的预测处理）、M1-068、M2-049、M0-030/033（对照基线）。
- **临时处理**：按此实现。
- **裁决（Fable 填写）**：

### PI-029 M0-030：低帧率浏览器客户端被误踢（输入 tick 窗口的 `too_far` 不应计入踢出连击）、服务器 RSS 增长的根因（统计样本向量）、浏览器死亡/重生预测

- **状态**：resolved（总控交办的三项问题已在 M0-030 内修正；基线与回归脚本见 reports/m0/perf-baseline.md）
- **类别**：缺陷修正 / 接口细化
- **提出者 / 日期**：M0-030 / 2026-11-17
- **问题 / 做法**：
  1. **误踢（演示服务器 tick 5987，`input_violation_streak`）**。服务器把"连续 30 条违规"定义为 `TooFar | ReservedBits | WeaponSlot | BadMove | MoveClamped | YawRate | PitchRate`。逐类排除：该客户端的输出不可能产生 `YawRate/PitchRate`（`validate_input` 的转速上限 8000°/s ÷ 30 Hz = 266°/tick，而一条指令最多改变偏航 180°、俯仰 180°）、`MoveClamped`（斜向已按 127/√2 取整，长度 125.9 < 128）、`BadMove/ReservedBits/WeaponSlot`（客户端常量）——**只剩 `TooFar`（tick 超过 now+10 的窗口）**。根因在**两端**：客户端 `ClockSync.inputLeadTicks()` 按 RTT 的 EWMA 取提前量，而 1–5 fps 的软件渲染页面主线程被阻塞，Pong 晚一帧才处理，RTT 被测成 300–1000 ms（本机复现：700 ms 帧阻塞下 RTT 1060 ms、提前量 16 tick），提前量超过服务器窗口（10 tick）后整帧的 3–5 条指令（同一时刻连发，tick 连续递增）都落在窗口外，没有任何一条被接受 → input_slack 反馈也停了 → 提前量降不下来，连击 30 次即被踢。服务器侧的问题是：窗口外的 tick 本来就被丢弃、不产生任何效果，却被当作"滥用"计数。**修正**：①服务器 `TooFar` 不再计入连击（仍丢弃、仍计入 `input_rejected_total{reason="too_far"}`）；超过 `now + FAR_FUTURE_TICKS`（300 tick = 10 s）才算伪造 tick，新增 `far_future` 原因并计入连击；`ReservedBits/WeaponSlot/BadMove/MoveClamped/YawRate/PitchRate` 不变。②客户端 `inputLeadTicks()` 封顶 `MAX_INPUT_LEAD_TICKS = 9`（窗口 10 减 1），超出窗口的提前量只会丢输入。恶意输入的防护没有放松：窗口外的输入依然一律丢弃，伪造的远未来 tick 与其它畸形值依然 30 连击踢出。
  2. **WT 路径服务器 RSS 线性增长**。用 `valgrind --tool=massif` 跑 20 个 WT 客户端 150 s：存活堆从 6.89 MB 涨到 7.36 MB（≈3.6 KB/s），增长点是 `SnapshotScheduler::tick` 里的 `Vec<f32>`（budget_util，262 144 B）与 `Vec<u16>`（sizes，131 072 B）——`SchedStats` 的**诊断样本**（每包 6 B，每客户端每秒另有两个 f32，每 tick 一个 u32）默认 `collect: true`，而生产二进制里调度器在 `sim` 线程内，样本从未被读取（只有 `Report` 出线程）。上限 400 000 个样本/向量，所以 10 客户端要约 33 min 才停止增长，100 客户端约 3 min——这就是 "30 min 线性增长"。WS 路径同样累积（15 Hz，每包 6 B），M0-027 的 WS 10 min 窗口内增长未超 2 %，未单独复核。**修正**：`sim_thread::spawn_with_bots` 关闭 `snap.collect`（测试与基准直接用 `SimLoop`，不受影响）；`tick_ns/err_ns`（退出报告用，原先每 tick 8 B、上限 400 万）改为 32 768 样本的均匀蓄水池 + 精确最大值（`Samples`），内存恒定。实测见 reports/m0/perf-baseline.md。
  3. **浏览器客户端的死亡/重生**（PI-028 交接）：此前没有处理——死后继续预测（harness 里死亡 5 s 内本地模拟多走 33.7 m，整段 53 次纠正）。现与 `crates/headless-client` 一致：自身 `health == 0` 时 `LocalPredictor.frozen`（`step/replay` 不动，不产生射击事件）；解冻后 ≥1 m 的纠正是重生传送，回滚从全新的站立状态（`freshLife`：清零速度/姿态、保留 yaw、重新装备起始武器）重放。
- **证据**：client/e2e/low-fps.spec.ts（真服务器 + Chromium，主线程每帧阻塞 280/330/700 ms，60 s 不被踢）、crates/server/src/session.rs 的 `clock_skew_is_not_a_violation_but_forged_ticks_are` 与 `low_fps_burst_pattern_is_never_kicked_for_60_s`、client/src/net/clock.test.ts、client/src/game/death-respawn.test.ts、massif 报告摘要见 PI 正文。
- **建议方案**：按上；M1-040（自动重连）与 M1-068（netem）沿用 `low-fps.spec.ts` 作低帧率回归；M1 若把验证改成滑动窗口（PI-022）保持"窗口外丢弃但不计违规"的语义。
- **受影响任务**：M0-033（验收）、M1-040、M1-068、M2-049。
- **临时处理**：已按此实现。
- **裁决（Fable 填写）**：总控交办项（EXECUTOR 守则"已观察到的问题"与 PI-028 的浏览器交接），按此实现；待总控确认。


### PI-030 M0-033：KCC 单次 ≤ 20 µs 在本机零余量（19.3–20.8 µs，约一半的运行越线）；E0-3 保持 partial，M1 开工前需参考机复测并决定是否启用 §5.3 降级

- **状态**：open
- **类别**：渲染/性能
- **提出者 / 日期**：M0-033 / 2026-11-17
- **问题**：E0-3（ARCH §20.1-2、MILESTONES §1.3）要求 KCC 单次 ≤ 20 µs。M0-011 起历次实测 18.6–19.9 µs，验收时复测同一基准（`cargo bench -p frontline-sim --bench kcc`，release，干净构建，机器空闲）三次：19.27 / 20.18（基准自身判 FAIL）/ 19.94 µs；M0-033 的 `perf-regress check --quick` 里一次 20.81 µs（与 M0-030 基线 19.22 µs 相比 +8.3 %，仍在回归容差 +25 % 内，但已越过 20 µs 预算）。100 个 KCC 一 tick 的 `World::step` 中位 2.24–2.29 ms（预算 3 ms，余量 24 %）无问题，p99 3.2–3.9 ms 偶超 3 ms。结论：单次耗时的 20 µs 判据在共享云虚机上处于测量噪声之内，既不能判通过也不能判失败；服务器预算（100 人 tick P99 实测 1.2–8.3 ms ≪ 28 ms）远未受威胁，所以这是"判据过紧"而非"服务器预算失守"。
- **证据**：`reports/baselines/sim-m0.json`（M0-011 基线）、`reports/m0/perf-baseline.md` §3、本任务三次 bench 输出（见 `reports/m0/ACCEPTANCE.md` §3）、`/tmp` 日志不入库。
- **建议方案**：①（推荐）在参考机（H-021）上用同一命令复测 5 次取中位数，若 ≤ 20 µs 则 E0-3 转 done；②若参考机仍 > 20 µs 或无参考机：把判据改为"100 个 KCC 一 tick ≤ 3 ms（World::step 中位）且单次中位 ≤ 25 µs"（需 Fable 回写 MILESTONES §1.3 E0-3 与 ARCH §20.1-2，写变更说明），因为服务器预算由前者而非后者约束；③不推荐立即启用 ARCH §5.3 降级（降级会牺牲移动手感，且数据不支持其必要性）。M1 可顺带做低风险优化（静止玩家跳过 KCC 的 ground probe、复用上一 tick 的接触缓存）以换取余量，但须通过 determinism 黄金哈希。
- **受影响任务**：M0-011（partial，已有）、M0-033；M1-068（压测门禁沿用 tick P99 判据而非 KCC µs）。
- **临时处理**：E0-3 在验收报告中记"部分"；不改代码、不放宽判据。
- **裁决（Fable 填写）**：

### PI-031 M0-033：验收中发现并修正的三处小缺陷（Playwright 主配置误收 vitest 文件、conn.rs 格式、determinism workflow 未默认严格）

- **状态**：resolved
- **类别**：测试/运维
- **提出者 / 日期**：M0-033 / 2026-11-17
- **问题 / 做法**：①`pnpm -C client test:e2e`（主 Playwright 配置，`testDir: 'e2e'`）自 M0-028 起会把 `e2e/interop/fixtures.test.ts`（vitest 文件）当作 Playwright 用例载入并整体报错退出码 1，所有主配置 e2e 因此无法运行——`client/playwright.config.ts` 增加 `testIgnore: ['**/*.test.ts']`，之后主配置 44 通过 / 8 跳过 / 0 失败；②`crates/server/src/net/conn.rs` 一处遗留 rustfmt 差异（M0-030 报告），已 `rustfmt` 修正，`cargo fmt --check` 退出 0；③PI-019 裁定"CI 默认严格模式"尚未落到 `.github/workflows/determinism.yml`，已改为默认严格（`DETERMINISM_STRICT=0` 或手动触发时取消勾选才降为警告），本机 `tools/determinism/run.sh --strict` 3 种子 × 18 000 tick native/wasm 逐位一致。另：任务卡写的 `pytest tools/map-pipeline tools/ref-analysis` 单次调用会因同名 `tests/` 目录与包路径失败，须分目录运行（见 `reports/m0/ACCEPTANCE.md` §3）；DEVELOPING.md 里 `scene=empty` 的示例 URL 应为 `empty=1`，已更正。
- **证据**：`reports/m0/ACCEPTANCE.md` §3。
- **建议方案**：按上；M1 的 CI 作业（H-102 后）把 `pnpm -C client test:e2e` 与各专用配置分别列为作业。
- **受影响任务**：M0-002（CI）、M0-028、M0-029、M0-030。
- **临时处理**：已修正。
- **裁决（Fable 填写）**：验收内的最小修补，待总控确认。

### PI-032 M1-001：闸门与规范化的几处接口细化（FLAGGED 与 D-LIC 的关系、空 allowed_processing、源朝向约定、requirements.json 最小版本、退出码 5–8）

- **状态**：open
- **类别**：素材/许可证 | 协议/数据（接口细化）
- **提出者 / 日期**：M1-001 / 2026-11-17
- **问题**：任务卡与 LICENSES.md 的几处表述需要落地时取舍，均按"最保守、最易回滚"实现，请 Fable 确认或改写：
  1. **D-LIC 与 LICENSES §6（简化登记，当前生效）**：任务卡要求"D-LIC 渠道且 `confirmation_path = —` → 退出码 4"，而 §6 说免费素材 `OK` 无需 `confirmation_path`。实现：渠道命中 D-LIC 清单（TurboSquid、CGTrader、Sketchfab、KitBash3D、ActorCore、A Sound Effect）且 `confirmation_path` 为空时拒绝，**例外**：`order_id` 以 `FREE` 开头（LICENSES §2 对免费素材的约定）或 Sketchfab 且许可证名以 `CC` 开头（规则原文"Sketchfab 非 CC"）。付费行不受影响。
  2. **`FLAGGED`**：LICENSES §6.2 在三值枚举外新增了 `FLAGGED`。解析器接受四值（其它值仍报错并给行号）；闸门把 `FLAGGED` 与 `PENDING` 同样处理（退出码 3，提示文字区分）。
  3. **`allowed_processing` 为空/`—`**：§6.5 说该列对免费素材可选填，但任务卡的 `assertAllowed` 以白名单为准。实现：空列 = D-MOD 全部处理码都允许；显式列出则只允许所列（越权退出码 8）；未知码在解析台账时报错。另外认可 §2 的音频码 `EQCOMP/CONVOLVE`（任务卡清单未列）与 PI-009 的 `SUBSET/RENAME`。
  4. **源朝向约定**：ARCHITECTURE §11.1 只规定输出为"Y-up、前向 -Z"，没规定源的约定。实现（均可用 `--up/--forward` 或 glTF `asset.extras.{unit,up_axis,forward_axis}` 覆盖）：Y-up 源默认前向 `+Z`（glTF 2.0 规范：资产正面朝 +Z），故默认 Y-up 资产会绕 Y 转 180° 到 -Z；Z-up 源默认前向 `-Y`（Blender 前视图约定）。**若总控/Fable 认为默认应为"不转"，把 normalize.ts 中的默认前向改为 `-Z` 即可（一行）**；对称件（立方体）不受影响。
  5. **`assets/requirements.json` 最小版本**：由 `asset-pipeline gen-requirements` 从 ASSET_SHOPPING_LIST §4.1–4.5 各表第一列抽取需求 ID（共 168 个，里程碑取所在小节、重复 ID 取最早里程碑；`kind` 仅按 `V-` 前缀粗分 visual/audio）。局限：只有 ID→里程碑映射，没有规格/数量；表格之外（正文里提到但没有表格行）的需求不会出现；`V-ANM-001（M1` 这类单元格中的括号注记靠正则容错。M1 内 72 个 ID（M1 小节首次出现者）：real 0 / placeholder 1（仅 `data/placeholders` 中带 `requirement_id` 的灰盒）/ missing 71。
  6. **退出码**：任务卡只定了 2/3/4。其余：1 用法/内部错误、5 台账引用的 `license_text_path`/`confirmation_path` 文件不存在（LICENSES §4.4）、6 台账无法解析、7 逻辑 ID 不合 §11.2、8 处理动作越权（D-MOD）。
  7. **运行方式**：CLI 入口 `tools/asset-pipeline/bin/asset-pipeline.mjs` 直接用 Node 的 type stripping 跑 `src/*.ts`（Node 22.18+ 默认开启，仓库固定 22.23.4）；`engines` 写的是 `>=22.12`，22.12–22.17 需加 `--experimental-strip-types`，或先 `pnpm -C tools/asset-pipeline build` 生成 `dist/`（入口优先用 dist）。
- **证据**：`tools/asset-pipeline/src/{gate,ledger,allowed,normalize,report}.ts`；`pnpm -C tools/asset-pipeline test` 80 项通过；`assets/requirements.json`。
- **建议方案**：按上采纳；若 1 的例外不被接受，删 `gate.ts` 里 `dlicChannel` 的两条 return 即回到任务卡原文（此时免费 CGTrader/Sketchfab 素材也需要 `confirmation_path`）。第 4 条请在 M1-002 / 素材接入任务拿到第一个真实素材时用肉眼核对朝向。
- **受影响任务**：M1-002、M1-003、M1-004、M1-005（沿用 `assertAllowed` 与 `work/` 目录布局）；M1 素材接入类任务（朝向、需求 ID 表）。
- **临时处理**：已按上实现；STATUS 标 partial 的原因只有验收 7（CI 墙钟需 GitHub 实跑，H-102）。
- **裁决（Fable 填写）**：

### PI-033 M1-002：纹理编码器、法线两通道布局、碰撞与处理权限、体积比判据的几处实现取舍

- **状态**：open
- **类别**：素材/管线（接口细化）
- **提出者 / 日期**：M1-002 / 2026-11-17
- **问题 / 做法**（均按最保守、最易回滚实现，请 Fable 确认或改写）：
  1. **编码器**：任务卡写 `toktx`（KTX-Software 原生程序，CI 镜像需另装）。实现改用 npm 的 `ktx2-encoder`（Basis Universal 编码器的 WASM 构建，MIT 包装 + Apache-2.0 编码器，与 `toktx` 同一编码器与同一 KTX2 容器），无需原生二进制，任何构建机 `pnpm install` 即可；依赖已在上一提交 919d1d7 加入。若坚持 `toktx`，只需替换 `textures.ts` 的 `encodeKtx2`。参数：ETC1S quality 128 / effort 1；UASTC level 1 + zstd；UASTC RDO 实测过（1024² 法线 23–31 s/张）太慢，未启用。
  2. **法线贴图"RG 双通道"**：输入先转成 RRRG 布局（X 写 RGB、Y 写 A），再按 UASTC 编码；KTX2 key/value 里写 `fl_layout=RRRG`、`fl_channels=2`，验收 4 的"通道数 = 2"按这两个 tag 校验（`inspectKtx2`），格式按 DFD 色彩模型（UASTC=166、ETC1S=163）校验。运行时（M1-006）需要按 RRRG 还原 Z；若 Fable 想要真正的 2 通道 KTX2（如 BC5/EAC_RG 目标），需改转码目标，属 M1-006。
  3. **量化后 normal**：`quantize` 把 normal 写成 8-bit snorm（xyz），再经 meshopt FILTER 方法（含八面体滤波）压缩；"8-bit oct" 即此滤波路径。position/uv 为 16-bit（SHORT/UNSIGNED_SHORT，normalized）。包围盒误差实测 ~2e-6 到 4e-6（< 0.1 % 判据）。
  4. **处理权限**：LOD 链、网格压缩、碰撞简化统一要求 `LOD`；纹理要求 `TEXCOMP`（仅当文档含纹理时才检查，无纹理资产不需要）。碰撞没有独立处理码，沿用 `LOD`。
  5. **碰撞**：trimesh 先 `simplify`（Prune），达不到预算再 `simplifySloppy`；convex 用"三角形质心沿最长轴递归二分 ≤ 32 簇 + 每簇增量凸包"，不做 V-HACD 式凹度细化（任务卡备注：新依赖先写 PLAN_ISSUES，故自实现）；退化簇回退为 AABB 盒。
  6. **ORM**：材质同时有 occlusion 与 metallicRoughness 且尺寸相同时，打包成一张 R=AO、G=roughness、B=metallic 的图，两个槽共用；尺寸不同则跳过并告警；只有 AO 无 MR 时 G/B 填 255（保持"因子 × 1"）。
  7. **NPOT**：宽或高不是 4 的倍数时边缘复制补到 4 的倍数并告警，**UV 不重缩放**（补边在右/下，影响最多 3 像素）。
  8. **验收 1 的体积比取决于内容**：ETC1S 颜色图压缩比很高（2048² 合成 albedo PNG 7.7 MB → ≈ 0.6 MB），UASTC 法线图只有约 35 % 的节省（1024² PNG 1.7 MB → 1.1 MB）。合成样例 A（2 万三角 + 2048² albedo + 3 张 1024²）为 18.3 %；样例 B（5 万三角 + 4 张 1024²，法线占比大）为 31.5 %，**超过 25 %**。真实 CC0 样例（带 2048² PNG 的材质模型）尚未入库，该验收按"合成样例通过、真实样例待测"记 partial；若总控要求法线图也 ≤ 25 %，需允许法线用 ETC1S 或降分辨率（改 `selectFormat` 表一行）。
  9. **"原始大小"**：`stats.source_bytes` = `normalized.gltf` 的 json + .bin + 图片字节（normalize 不改变图片与几何精度，与原始 glTF+PNG 等价）；`check-size-ratio` 用 `lods[0].bytes / source_bytes`。
- **证据**：`tools/asset-pipeline/src/{textures,mesh-compress,collision,process,stats}.ts`；`pnpm -C tools/asset-pipeline test` 102 项、`test:e2e` 5 项通过。
- **受影响任务**：M1-003（读取 `lodN{_fp,_tp}.glb`、`collision.glb`、`stats.json`）、M1-005（`stats.chains[].lods[]`）、M1-006（RRRG 法线与 KTX2 转码）、M1-064。
- **临时处理**：已按上实现。
- **裁决（Fable 填写）**：

### PI-034 M1-003：`.fpak` 的几处格式取舍（条目命名、header/footer 字段、网格/贴图条目内容、分包）

- **状态**：open
- **类别**：素材/管线（格式）
- **提出者 / 日期**：M1-003 / 2026-11-17
- **问题 / 做法**：
  1. **条目命名**：任务卡用 `wpn_us_m1_garand@lod0`，ARCHITECTURE §11.3 的 manifest 示例用 `veh_us_m4a1_sherman/lod0`。实现按任务卡用 `@`（`^[a-z][a-z0-9_]*(@[a-z0-9_]+)?$`），M1-005 写 manifest 的 `entry` 字段时用同一写法；§11.3 的斜杠写法视为示意。
  2. **header/footer**：header 32 B（magic、version、flags、entry_count、index_offset，其余保留），条目起点 32、16 B 对齐；footer 32 B，`version` 放在 footer（打开时不必读 header），CRC32 校验索引。任务卡写"fnv/crc32"，取 CRC32。
  3. **条目布局 v1**：网格条目 = 自有头 + 每图元 `meshopt` 压缩的交错顶点缓冲 + 索引缓冲 + 4×4 矩阵（带 `quantize` 的反量化变换）+ 材质表（含贴图条目的逻辑 ID）；贴图条目 = KTX2 去掉标准头后按"最小 mip 在前"重排，保留 DFD 与 ETC1S 全局数据。**当前不支持蒙皮/动画网格**（M1-045 在新 `layout_version` 里扩展）。运行期解码属 M1-006。
  4. **分包**：默认按类别（逻辑 ID 前缀）分组，单包超过 16 MiB 在资产之间切分；64 m 地图区块的分组通过 `pack({group})` 回调由地图任务传入（M1-049/M1-035），本任务不生成区块。
  5. **`pack` 的权限**：对每个资产先调用 `assertAllowed(id, 'CONVERT')`（台账行必须存在且 OK；normalize 已要求过 CONVERT）；打包本身没有独立的 D-MOD 处理码。
  6. **meta 条目**：每个资产一个 `@meta` JSON 条目（逻辑 ID、各 LOD 三角/顶点数、碰撞与贴图摘要），不含台账 ID、源文件名或渠道名；已核对产物中不含这些字符串（e2e 断言）。
  7. **不透明性**：`assert-opaque` 检查魔数、GLB/glTF JSON 解析、`ktx-parse`、`file(1)`；对照组（真实 KTX2）会被识别，保证检查有效。这只说明"不是标准文件"，不是保护，SPEC.md 与代码注释均已写明。
- **证据**：`tools/asset-pipeline/src/fpak/`、`src/pack.ts`、`tools/asset-pipeline/src/fpak/SPEC.md`；`pnpm -C tools/asset-pipeline test` 123 项、`test:e2e` 7 项通过。
- **受影响任务**：M1-005（`packs.json` → manifest）、M1-006（读取器与解码 Worker）、M1-004（音频条目 `kind:'audio'`、codec 3）、M1-045（蒙皮网格布局）。
- **临时处理**：已按上实现。
- **裁决（Fable 填写）**：

### PI-035 M1-004：音频条目命名（`@opus`/`@aac`）、Opus 码率模式、短剪辑响度回退与条目头

- **状态**：open
- **类别**：素材/管线（格式）
- **提出者 / 日期**：M1-004 / 2026-11-17
- **问题 / 做法**（请 Fable 确认或改写）：
  1. **命名**：任务卡写 `…#opus` / `…#aac`；`.fpak` 逻辑 ID 的变体分隔符只允许 `@`（PI-034），所以用 `snd_…@opus` / `snd_…@aac`。M1-006/M1-062 按此取包；PROBE.md 已固定。
  2. **Opus 用 CBR**（`-vbr off`）：1 s 单声道 64 kbps 正弦用 VBR 得 10.6 KB（> 9 KB 验收线），CBR 得 9.0 KB。代价是复杂音频的质量/体积折中不如 VBR；若需要可对 `ambience` 单独改回 VBR（`encode.ts` 一处）。
  3. **条目不是裸 WebM/MP4**：音频条目 = 32 字节自有头 + 容器字节（`decodeAudioData` 需要完整容器）。头里放 `preload`、`loop`、时长等；条目因此不以 EBML/ftyp 魔数开头，但容器内容仍可被提取后识别，这只是"非独立文件分发"，不是保护。
  4. **响度**：目标 LUFS 表 near -18 / ambience -24 / ui -20 取自任务卡；其余类别（gun_mid -20、gun_far -22、foley -20、music -20、voice -18）为【待调参】的暂定值。True Peak 目标 -1.5 dBTP（验收 ≤ -1）。ffmpeg `loudnorm` 对短于约 400 ms 的剪辑得不到积分响度（-inf）：回退为峰值归一到 -3 dBFS，报告里 `loudness_measured:false`、`integrated_lufs:-70`。
  5. **AAC 容器**：MP4（`-movflags +faststart`，`-map_metadata -1`，bitexact），不是裸 ADTS，以便 Safari 的 `decodeAudioData`；解码侧 AAC 有编码器延迟/补零，往返时长判据只对 Opus 取 ±20 ms。
  6. **fixtures**：任务卡写 `fixtures/sine_1s.wav`；测试里用自写的 WAV 写入器现场生成（含注入 `bext`/`iXML` 的版本），不新增二进制 fixture。
  7. **依赖**：用系统 `ffmpeg`（GitHub `ubuntu-latest` 自带；本机 6.1.1），不新增 npm 依赖。许可证见 LICENSES §5 新增行（未核实，不随游戏分发）。
- **证据**：`tools/asset-pipeline/src/audio/`、`src/fpak/entry-audio.ts`、`PROBE.md`；`pnpm -C tools/asset-pipeline test` 134 项、`test:e2e` 9 项通过。
- **受影响任务**：M1-005（体积门禁读 `audio-report.json`）、M1-006/M1-062（探测与取包）、M1-065（真实音频接入）。
- **临时处理**：已按上实现。
- **裁决（Fable 填写）**：

### PI-036 M1-005：manifest 由 `.fpak` 反推、`core` 只认外部测量、bundle 通配与分档规则、`audio-report` 资产 ID
- **问题 / 做法**（请 Fable 确认或改写）：
  1. **manifest 来源**：`manifest` 命令直接打开 `assets/processed/**/*.fpak` 读索引并计算每条目 sha256（任务卡写"汇总 stats.json、索引、台账"）。`pak/entry/offset/bytes/sha256` 因此天然正确；`stats.json` 只补 `tris` 与资产 ID；`consistency` 用同一读取路径逐条复核（偏移 +1 可被定位到条目）。
  2. **`@` 变体 → manifest 字段**：`X@lodN[_fp|_tp]` → `lods[]`（带 `suffix`）；`@collision`、`@tex_*`、`@opus|@aac`（`audio[]`，`codec`）、`@meta`。与 ARCHITECTURE §11.3 示例相比，`entry` 是 `.fpak` 逻辑 ID（`veh_x@lod0`，不是 `veh_x/lod0`）、纹理放在资产内联 `textures[]`；示例仅追加 `placeholder`/`req_id`/`index_bytes`。
  3. **`placeholder`**：台账行存在且 `status=OK` → `false`，否则 `true`；`--overrides` 可强制，但 `placeholder:false` 对应非 OK 台账一律失败（`checkPlaceholders`）。音频资产 ID 取自 `audio-report.json` 的 `asset_id`（`audio-pack` 已写入 item 的 `asset_id` 时才有）。
  4. **`tier`**：stats 不含 asset-class 的档位，缺省为全档 `low..ultra`；用 overrides（通配）收窄。`*_medium|high|ultra` 的 bundle 只计 `tier` 含该档的资产；bundle 通配 `*` 不跨 `/`，`**` 跨。
  5. **预算口径**：bundle 字节 = 命中资产全部条目 `bytes` + 每条目索引字节（26 + id 长度）；不含 16 字节对齐填充与头尾（≤ 约 0.1 %，在 1 % 容差内）。`core` 不由 manifest 计算，只读 `--core-size-file`（`core_bytes|br_bytes|total_br_bytes`），缺失即 `skipped`。M0 的 Vite 构建尚未输出 size-report（M1-054 前登记），所以 CI 当前 `core` 为 skipped。
  6. **默认 bundles.json**：Medium/High/Ultra 暂用同一组类别通配（`bld_* chr_* prop_* snd_* tex_* veg_* veh_* wpn_*`），靠 `tier` 区分；`terrain_carentan/*` 区块（M1-035）使用 `/`，而 `.fpak` 逻辑 ID 不允许 `/`，M1-035 须决定用 `terrain_carentan_b_{bx}_{bz}` 命名并相应改通配。High/Ultra 预算按 350/600 MiB（367001600 / 629145600 B）。
  7. **基线 manifest**：CI 取目标分支的 `assets/manifests/manifest.json`（`git show origin/<base>:…`），不存在则只做绝对预算；`budget-approved` 标签经环境变量 `BUDGET_APPROVED=1` 传入。
- **证据**：`tools/asset-pipeline/src/{manifest,budget,consistency}.ts`、`.github/workflows/manifest-budget.yml`、`test/manifest.test.ts`。
- **受影响任务**：M1-006（消费 manifest；`entry` 为 `.fpak` 逻辑 ID）、M1-035（区块命名）、M1-054（core size-report）、M1-064（按类别汇总）。
- **临时处理**：已按上实现。
- **裁决（Fable 填写）**：

### PI-037 M1-006：加载器先交付核心逻辑（partial），Worker 内 KTX2/Meshopt 解码与 Playwright 指标拆为 M1-006b
- **问题 / 做法**（请 Fable 确认或改写）：
  1. **拆分**：任务卡自己建议超规模时拆 `M1-006b`。本次（M1-006）交付与环境无关的核心：`fpak-reader`（Range/200 降级/index crc）、`manifest.resolve`、`CacheLayer`（内存 LRU + OPFS/Cache API 探测与自动降级）、`WorkerPool`（优先级/取消/池大小 `min(4, cores-2)`）、`UploadScheduler`（1.5 ms / 2 MB 每帧）、占位命名 `PH_{reqId}`/`PH_{reqId}__{instance}`、静音回退、去重、重试、节流进度。**未交付（M1-006b）**：`decode-worker.ts`（BasisU transcoder WASM + meshopt 解码，设备格式选择）、Playwright 长任务/冷热启动/100 MB 样例指标（验收 1、4 的浏览器部分）、OPFS `FileSystemSyncAccessHandle`（现用异步 `createWritable`，Safari 旧版本可能不支持，失败即降级内存）。解码默认是恒等解码器（`Decoder` 注入点），因此现在返回的 `MeshData` 仍是 `.fpak` 条目原始字节。
  2. **重试语义**：「重试 3 次（200/400/800 ms）」解释为 1 次首发 + 3 次重试 = 共 4 次尝试，失败后返回占位。404 也重试（任务卡未区分）。
  3. **占位未被缓存**：占位结果不进入会话级去重表，下一次请求会重试，避免一次网络抖动永久占位。
  4. **缓存键**：用条目 `sha256`（manifest 提供），内容寻址，跨版本安全；`pak` 路径本身已内容哈希。
  5. **测试位置**：沿用 client 的 `src/**/*.test.ts`（vite 配置不改），任务卡的 `client/test/assets/*.test.ts` 不使用。
  6. **与 M1-005 manifest 的对应**：`entry` 为 `.fpak` 逻辑 ID（`x@lod0`），音频 `x@opus|aac`；`manifest.ts` 按 tier 过滤。
- **证据**：`client/src/render/assets/*`，`pnpm -C client test` 全绿（新增 20 项），`tsc`/`eslint` 干净。
- **受影响任务**：M1-006b（解码 Worker + 浏览器指标）、M1-049（区块流式用 `loadBytes`/`loadMesh` + 取消）、M1-062（`loadAudioBytes`）。
- **临时处理**：已按上实现。
- **裁决（Fable 填写）**：

### PI-038 M1-006b：解码 Worker 已做网格（meshopt）与纹理条目解析，BasisU 转码留给 M1-006c
- **问题 / 做法**（请 Fable 确认或改写）：
  1. **已做**：`entry-decode.ts`（网格布局 v1 → 顶点/索引/材质，纹理布局 v1 → KTX2 层数据 + DFD/SGD + `RRRG` 布局标记）、`decode-worker.ts`（Worker 消息处理，Transferable 回传）、`worker-decoder.ts`（按池大小建 Worker 的 `Decoder`，以及同线程 `inlineDecoder` 作测试/无 Worker 回退）。client 新增依赖 `meshoptimizer@1.3.0`（MIT，已在 workspace 中被素材管线使用）。
  2. **未做（M1-006c）**：KTX2 → BC7/ASTC/ETC2/RGBA 转码需要 BasisU transcoder WASM（或经 Babylon `KhronosTextureContainer2`，需把条目重新拼回 KTX2 文件）；来源与许可证须登记 LICENSES §5，且影响 core 体积（约 0.4 MB br）。需要先定"自带 transcoder"还是"用 Babylon 内置"（后者默认从 CDN 取 WASM，与自托管/离线策略冲突）。裁决前 `loadTexture` 返回已解析的 `DecodedTexture`。
  3. **Playwright 指标**（长任务 = 0、100 MB 冷热启动）仍未做：需要参考机与 ≥100 MB 样例包，并入 H-021/H-102 之后。
- **证据**：`client/src/render/assets/{entry-decode,decode-worker,worker-decoder}.ts`、`decode.test.ts`（4 项；网格往返：位置/UV/索引/材质/纹理引用逐项一致）。
- **受影响任务**：M1-006c（转码）、M1-042（PBR 材质消费 `DecodedMesh`/`DecodedTexture`）、M1-049。
- **临时处理**：已按上实现。
- **裁决（Fable 填写）**：

### PI-039 CI 上两个确定性哈希与本机不一致的根因：`f32::atan2` 走平台 libm、`native.json` 数据哈希过期

- **状态**：resolved（待 CI 复核）
- **类别**：测试/运维
- **提出者 / 日期**：用户指派的修复线程 / 2026-11-17
- **问题**：PR 1 上 `test-rust`（`bot_input_hash_is_pinned`）与 `wasm-build`（`node-smoke.mjs` 数据哈希）在 GitHub runner 失败，本机通过。
- **证据**：
  1. `wasm-build`：不是平台问题。干净克隆上 `cargo run --example trace` 得到 `dataHash=17150076847956766228`，与 CI 的 wasm 值一致，而入库的 `crates/sim-wasm/tests/golden/native.json` 记的是旧值 `16955763235307373660`（该文件在 M1-002 提交时没有随 `data/` 重新生成）。用 `gen_golden` 重新生成即可。
  2. `test-rust`：`crates/bots/src/soldier.rs` 里两处 `dir[0].atan2(dir[1])` 是 sim/bots 中仅有的未走 `libm` 的超越函数调用（其余全是 `libm::sinf/cosf/sqrtf`）。`f32::atan2` 在 Linux 上调用系统 glibc 的 `atan2f`，不同 glibc 版本/CPU 变体可差 1 ulp，进而改变机器人输入哈希（CI `0xb6050c0e229e2aa1` vs 本机 `0x6f09df7c5f721374`）。
- **补充（第二轮 CI）**：`crates/sim/tests/damage_death.rs` 的测试 harness 里同样有 `d.x.atan2(d.z)`，使 `GOLDEN_SKIRMISH_FINAL` 在 runner 上不同（CI `0xb1054f5a473b9951`）；`bots/tests/common/mod.rs` 的 `cos/sin` 摆位同理。均改为 `libm::*f`，金哈希不变。
- **做法**：改用 `libm::atan2f`（与其余确定性代码一致）；重新生成 `native.json`。bots 金哈希**不改**（本机换 libm 后仍为 `0x6f09df7c5f721374`）。
- **规则**：sim / bots / protocol 内禁止使用 `f32::{sin,cos,tan,atan,atan2,exp,ln,powf,...}`，一律用 `libm::*f`。
- **受影响任务**：M0 确定性闸门（determinism / wasm-build / test-rust）。
- **裁决（Fable 填写）**：

### PI-041 M1-008 分两步：先交付 6 个枪械（现有 schema 内），手雷/火箭/近战/弹药表拆 M1-008b
- **问题 / 做法**（请 Fable 确认或改写）：
  1. **本次交付**：`us_m1_carbine`、`us_thompson_m1a1`、`us_m3_grease`、`ger_mp40`、`us_m1911a1`、`ger_p38`，数值逐项取自任务卡（GD §6.2/§6.3），`_meta.status=initial`，`historical._source` 为占位字符串（"待 M1-070 按 REF §6 复核"）；`historical` 值（口径、初速、整备质量等）来自记忆，未联网复核。手枪 `muzzle_flash_ms` 取 30（任务卡未给，取小于步枪的暂定值）。冲锋枪/手枪的 `spread.hip_deg`/`ads_deg` 用任务卡"盲射/ADS"两列。
  2. **推迟（M1-008b）**：手雷（`us_mk2`、`ger_m24`、`us_m18`、`ger_nb39`）、`ger_panzerfaust_60`、6 个近战、`data/ammo/`。现有 `weapon.schema.json` 的 `gameplay` 是 `additionalProperties:false` 且要求射击字段（`falloff/fire/reload/ads_ms/spread/recoil/ballistics`），不能表达引信/烟雾/后焰/近战前摇；需要扩展 schema（按 `class` 分支）与 `data.rs` 解析，属 M1-015 / M1-013 的前置，单独成 PR。
  3. **金标重打**（总控指示：同一 PR 内重打并说明）：新增武器改变 `DATA_HASH` 与武器下标，`sim` 的 `GOLDEN_SKIRMISH_FINAL`、`GOLDEN_DUEL_EVENTS`、`sim-wasm` 的 `GOLDEN_SEED1_18000` 与 `tests/golden/native.json`（`gen_golden` 重生成）按新数据重打；`bots` 金标未变。合并哈希线 PR #2（libm）与 PR #10（`state_hash` 按武器字符串 id，`damage_death`/`determinism_basic` 由其重定标）后，`cargo test --workspace --locked` 与 `cargo test -p frontline-sim --features test-util`（CI 的两条命令）本地均通过。
  4. **名称规范**：武器 ID 沿用已落地的 `us_/ger_` 前缀（GD §0.3）。
- **证据**：`data/weapons/*.json`（8 个文件）、`tools/data-validate` 0 错误；金标新值见各测试常量。
- **受影响任务**：M1-007（loadout 引用这 6 个枪械；手雷/火箭/近战引用需等 M1-008b）、M1-012/013、M1-070。
- **临时处理**：已按上实现。
- **裁决（Fable 填写）**：


### PI-040 金哈希不再依赖武器在 `GameData::weapons` 中的下标

- **状态**：resolved
- **类别**：测试/运维
- **提出者 / 日期**：确定性修复线程（响应 M1 线程的预告）/ 2026-11-17
- **问题**：M1 数据任务会新增武器、改变 `weapons/*.json` 的顺序，而 `World::state_hash` 把下标（`WeaponState.def_id`、弹丸 `weapon`、死亡记录里的 `weapon`）直接写进哈希，一加武器所有 sim/bots/server/wasm 金哈希都会移位。
- **做法**：`state_hash` 改为写武器字符串 id 的 FNV-1a（`World::weapon_key`），不再写下标；线协议里的 `weapon:6` 与 `WeaponState::to_bytes` 不变（下标仍是运行时/线上的表示）。因此只有"武器的实际数值/新增武器改变了战斗结果"才会改金哈希。一次性重新定标：`damage_death` `GOLDEN_SKIRMISH_FINAL`、`determinism_basic` `GOLDEN_FINAL_HASH`（其余金哈希不含武器状态，未变）。`node-smoke.mjs` 的 `dataHash` 改为优先对比 `target/wasm-trace/meta.json`（原生 trace 的当次值），不再每次改 /data 就要重生成 `native.json`。
- **仍然会变的**：`DATA_HASH`（按设计随 /data 变，现在测试里按当次值比对）、与武器数值有关的行为型金哈希（bots、skirmish 等，改数值就是行为变了，需按注释说明重新定标）。
- **受影响任务**：M1 数据任务（武器/角色数据）。
- **裁决（Fable 填写）**：

### PI-042 M1-008b：手雷/烟雾/火箭/近战/弹药各成独立 kind 与目录，不塞进 `weapons`

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：数据/架构
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **问题**：M1-008 卡要求把 `us_mk2`、`ger_m24`、`us_m18`、`ger_nb39`、`ger_panzerfaust_60`、6 个近战、`data/ammo/` 放进 `data/weapons`，但 `weapon.schema.json` 的 `gameplay` 必填 `damage_torso_hp/recoil/spread/mag_reserve…`（枪械专属），且 `GameData::weapons` 被 sim/bots/sim-wasm 以下标消费（`WeaponParams::from_def` 等），把无后坐/无弹匣的投掷物塞进去要么放松全部必填、要么让每个消费方判 class。
- **做法**：新增三个数据 kind：`explosive`（`data/explosives/`，class `frag_grenade|smoke_grenade|at_rocket`，按 class 的 `if/then` 约束 `gameplay`）、`melee`（`data/melee/`）、`ammo`（`data/ammo/`），各有 JSON Schema、Rust 类型（`ExplosiveDef/MeleeDef/AmmoDef`，`GameData::{explosives,melee,ammo}`）与 build.rs 类型化解析。`weapons/` 与 `GameData::weapons` 不变，因此武器下标与金哈希行为不受影响（只有 `DATA_HASH` 随 /data 变）。引用完整性：`data-validate` 的 `weapon` id 空间 = weapons ∪ explosives ∪ melee，loadout 引用手雷/近战 id 照常解析，且跨目录 id 唯一。单位：弹重用 `mass_kg`（`_g` 在白名单外），压制系数用 `suppression_mult`，弹性/摩擦用 `*_mult`。近战 `faction` 允许 `any`（枪托）。
- **与卡的偏差**：目录名；`rate_of_fire_rpm_cap/rpm_cyclic` 对手雷/近战无意义，未加。
- **受影响任务**：M1-007（loadout 引用）、M1-015（投掷物行为）、M1-013（近战）、M1-012（弹药阻力）。
- **裁决（Fable 填写）**：

### PI-043 M1-007：gadget 与三类规则作为新 kind；`ammo_mags` 含枪内一个；loadout 增补字段

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：数据/规则
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①新 kind `gadget`（`data/gadgets/`，8 个：绷带、医疗包、望远镜、Garrison/Outpost/Ammo Crate 套件、Supply Crate 搬运、维修工具箱；`gameplay` 为带单位后缀的扁平数值表，校验器强制单位）。②`data/rules/` 下 `capture.json`、`spawn.json`、`supplies.json` 各成独立 kind（`rule_capture/rule_spawn/rule_supplies`，各有 schema；按文件名分派，`respawn.json`/`bots.json` 不变）。`spawn.json` 只放卡中新增项（Garrison、出生保护、HQ/Garrison 的 base_wait）；波次间隔仍以 `respawn.json` 为运行时真源，`spawn.json.wave_interval_s` 为文档性副本，M1-016 接入时二选一合并。这三类规则暂只做 schema 校验与哈希，未做 Rust 类型化（消费它们的 sim 任务 M1-016..021 各自加）。③loadout 增补 `unlock_level`（必填）、`secondary_mags`、`tools[]`（≤4，非武器工具含绷带）、`tags`（仅 `m1_only`，且只允许 `ger_rifleman_pf60`，由 data-validate 强制）；`gadget_1/2` 作为手雷/发射器栏（可引用 explosives 的 id）。Outpost 套件在 GD 5.5 提到但卡的 gadget 清单没有，补了 `outpost_kit`。④单位白名单新增后缀 `_points/_supplies/_players/_slots/_uses/_crates` 与无量纲 `unlock_level/secondary_mags`。
- **`ammo_mags` 语义（行为变更）**：GD 5.3 的"N 弹匣含枪内一个"——`starting_weapon` 的备弹由 `cap × ammo_mags` 改为 `cap × (ammo_mags − 1)`；M0 数据里 `ammo_mags: 6` 原本是"6 个备弹匣"。`us_rifleman_1`/`ger_rifleman_1` 现为 10/16（卡与 GD 5.6）。因此 `GOLDEN_SKIRMISH_FINAL`、`determinism_basic::GOLDEN_FINAL_HASH` 重新定标，`weapon_duel`/`damage_death` 的弹药守恒断言按新数值改写。
- **未决/假设**：Crewman 的 `unlock` 省略（无小队编制，全队上限 8）；Officer 的近战取匕首（GD 未给，待 M1-070）；`ger_rifleman_pf60` 的步枪取 Kar98k ×12、另带 M24 ×1（卡只说"加 Panzerfaust 60"）；望远镜无数值。
- **受影响任务**：M1-016..021（规则消费）、M1-014（弹药/换弹）、M1-064。
- **裁决（Fable 填写）**：

### PI-044 M1-006c：自带 Basis Universal 转码器（Apache-2.0），不用 Babylon 的 CDN 转码器

- **状态**：resolved（实现者裁决，待 Fable 复核；回应 PI-038 第 2 点）
- **类别**：客户端/许可证
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：选"自带 transcoder"。把 Binomial 的 `basis_transcoder.{js,wasm}`（Apache-2.0，527 KB wasm + 57 KB js）原样放进 `client/public/basis/`，取自 `three@0.187.0` 的预编译产物（上游 Emscripten 构建，无需本仓库编译 C++），登记 LICENSES.md §5（含 SHA-256）。Babylon 默认从 `cdn.babylonjs.com` 拉 WASM，与自托管/离线策略冲突，不用。`babylonjs-ktx2decoder`（Apache-2.0）包内不含 wasm，也不适用。
- **实现**：`ktx2-rebuild.ts` 把纹理条目（layout v1，不含容器头、最小 mip 在前）重新拼回标准 KTX2；`transcode.ts` 按 GPU 能力选 BC7 > ASTC 4×4 > ETC2 > BC3 > RGBA8 并转码全部 mip；`decode-worker.ts` 收到 `{init:{caps,basisBase}}` 后首次用到才加载模块（模块 Worker 无 `importScripts`，用 fetch 文本 + `new Function` 求值），之后纹理条目直接返回 `TranscodedTexture`（大 mip 在前，Transferable）；未配置时仍返回 `DecodedTexture`。`DecodedTexture` 补了 `typeSize/depth/layerCount/faceCount`。
- **测试**：真实编码器产出的 16×16 UASTC 与 ETC1S fixtures（`fixtures/*.entry|*.ktx2`），两种源 × 五种目标格式的尺寸与完整 mip 链、RGBA8 梯度还原误差 ≤ 24/255、损坏数据抛错、模块只加载一次。
- **体积**：不计入 `core`（按需加载，不在首包）；`manifest-budget` 的 `core` 由外部 size-report 提供（PI-036），应只统计首包。
- **未做**：法线图 `RRRG` 布局的 BC5 路径（目前走 RGBA 格式，法线重建由材质着色器处理，M1-042）；Playwright 真机 GPU 上传与耗时（H-021）。
- **裁决（Fable 填写）**：

### PI-045 M1-009：炮弹内联在车辆 `weapons[]`，车辆 schema 向后兼容地扩展；ARCH 8.5.4 的 hull_health 1000 以 GD 的 1600 为准

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：数据/文档不一致
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①卡允许"独立 `ammo_shells/` 文件或并入车辆 `weapons[]` 二选一"，选并入：M1 只有一种炮，单独成 kind 要再加 schema、Rust 类型与引用规则，收益到 M3 才有（M4A3E2 共用 75 mm M3 时再抽出，届时在 `weapons[].shells` 改为 `shell_ids`）。②`vehicle.schema.json` 新增均为可选字段：`historical.dimensions_m`、`gameplay.{max_speed_mps, hull_turn_deg_s, accel_time_s, max_slope_deg, armor_zones[], modules[], weapons[], spawn, sockets}`，旧夹具与 M0 数据不受影响。③`data-validate` 车辆规则：座位 id 唯一、坦克类 `armor_zones` 覆盖 9 区且 id 不重复、模块/武器 id 唯一、武器座位须存在、`max_speed_mps = 0.75 × 史实路速`（±0.5%）。单位白名单增加后缀 `_pct`、`_pct_per_100m`、`_rounds`。④`hull_health_hp = 1600`（GD 14.8），ARCH 8.5.4 示例的 1000 仅为结构示例。⑤`armor_mm{front,side,rear,top}` 汇总保留（必填，取车体前上/侧/后/顶），细粒度以 `armor_zones` 为准。
- **假设**：车载 M1919 的伤害取 40（GD 6.3 单兵 MG34 49 × 0.8 的类推，GD 无 M1919 行），M2HB 取表内 85；M1919 初速 853 m/s、500 rpm 取 GD 6.2 表。Sherman 最高速 28.5 km/h = 7.9167 m/s 以 `max_speed_mps` 存（`_kph` 不在单位白名单）。悬挂/履带参数沿用 M0 夹具初值，待 M1-022 实测。
- **未做**：Rust 侧 `weapons`/`spawn` 暂为松散 `serde_json::Value`，由 M1-022/M1-023 在消费时定型；`GOLDEN`（sim/bots）未受影响（`weapons` 表未动）。
- **受影响任务**：M1-022、M1-023、M1-048、M1-064。
- **裁决（Fable 填写）**：

### PI-046 M1-010：协议任务拆为 A（C0/C4/事件）与 b（载具/团队通道/fuzz/bench），并补充若干字段

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：任务范围/协议
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：M1-010 卡为 L 规模，拆两个 PR：**A**（本次）= C0 新消息、C4 全部事件、事件段 5–7（Explosion/Suppress/Marker）、`PROTOCOL_VERSION` 1→2、`CHANGELOG.md`、往返/错误用例；**b** = 载具记录与 delta（≤24 B 均值）、`TeamState/WorldState`（49 人 ≤200 B）、fuzz 靶 `c0,c4,events,snapshot_vehicle`、criterion 基准。验收 3/4/7 与性能基准归 b。
- **补充/细化**：①`SquadOp` 加 `lock/unlock/invite`（9 种，数值 1–9）。②`Chat` 末尾加 `sender:u8`（S→C 才有意义，C→S 忽略）；C→S 与 S→C 共用 type 0x13。③`Settings` 定为 `view_dist_m:u16, flags:u8`。④`PlayerRoster.flags` 位：倒地/Commander/队长/离场/Bot；昵称 ≤24 字节。⑤`Kill.dist_q` 为 12 位原始码（0.25 m/步），不在协议层转米。⑥`SpawnPointState.pos` 为 2×16 位原始码，度量含义由消费方（M1-040/Map）定义，与 `MarkerPlace` 一致。⑦C4 `msg_type` 编号 0x01–0x09，C0 新增 0x10–0x16，事件类型 5/6/7。⑧字符串上限按**字节**计（chat 200 字节 ≈ 66 个汉字）。
- **下游处理**：服务器 C0 收到新客户端消息先接受并丢弃（M1-026 实现）；sim-wasm `pack_event` 增加三种事件的 13 字节记录，JS 暴露归 M1-040。`hello.hex` 与 `native.json` 金样已重定标。
- **受影响任务**：M1-024/025/026/040。
- **裁决（Fable 填写）**：

### PI-047 M1-010b：载具实体记录、团队通道的线格式取舍，协议版本 3

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：协议
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①实体记录 tag 扩为 `1`=delta、`01`=玩家关键帧、`001`=载具关键帧、`000`=移除（移除 10→11 bit），kind 由基线实体决定，delta 不带 kind；这是对版本 2 的破坏性改动，故 `PROTOCOL_VERSION` 2→3（版本 2 仅存在于刚合并的 PR #15）。②载具含速度（3×i11，1/64 m/s，±16 m/s）以供客户端外推；delta 中 pos 沿用玩家的尺寸类，quat/vel/turret/crew 为整字段替换（不做差分），实测均值 15.8 B、最大 20 B，远低于 24/28 B。③`VehicleExtra` 去掉 M0 占位的 `u8` 炮塔字段，改为 12+12 bit，新增 `damage_flags`；`seats` 为 3 位乘员位图。④`TeamState` 不带 player_id（卡限定每人 32 bit、49 人 ≤200 B）：第 i 项对应本队按 player_id 升序的第 i 名成员，花名册来自 C4 `PlayerRoster`，`state==0` 表示缺席/阵亡；超 49 人按 id 序截断并返回被丢弃数。⑤位置坐标 x/z 各 11 bit，含义由消费方按地图尺寸定。⑥`WorldState` M1 子集 16 B。
- **验证**：proptest 1e5 往返（车辆状态/团队/世界），四元数 ≤0.35°（1e5 样本），炮塔/炮管 12 bit ≤0.35°；`vehicle_delta_size`（1e4 步）；60 实体包（含 8 辆载具）编码 15 µs（≤25），单载具 0.33 µs（≤2）。cargo-fuzz 靶 `c0,c4,events,snapshot_vehicle` 已加入 `fuzz/`，但本环境无 nightly/cargo-fuzz，改以 `tests/mutation.rs`（同属性、每靶 ≥5e4 变异输入，随 CI 跑）替代；10 分钟 fuzz 记录待参考机/CI nightly（H-102）。
- **受影响任务**：M1-024/025（发送策略）、M1-040（客户端解码与 JS 暴露）。
- **裁决（Fable 填写）**：

### PI-048 M1-011a：移动拆为 a（趴姿/体力/修正/验证）与 b（翻越/攀爬/探身），以及若干模型取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim/数据
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①拆分：碰撞世界目前只有地形高度场，没有可翻越的障碍物，翻越/攀爬/探身依赖的几何检测放 011b（用合成高度场台阶做测试）。②`stance_blend` 改为姿态坐标 0 站/1 蹲/2 趴；转换时间表由 `stance_from`（上一个指令姿态）+ 目标决定，中途反向从当前坐标继续，胶囊高度不跳变；站→趴取 0.70 s 直达表项而非 0.25+0.50。趴姿胶囊高 0.7 m（= 2×半径，KCC 最小值；GD 的 0.5 m 是命中盒高度，另存于 `hitbox_height_m`，命中盒 `PRONE` 表见 hitbox.rs）。③起身顶部空间检查用向上射线（目前只有地形遮挡），不足则退回较矮姿态。④输入：`PRONE` 键 > `CROUCH` 键 > 站；趴姿 `SPRINT` 为快爬（1.3 m/s，8/s 体力），`AUX_WALK` 强制慢爬；ADS 乘数在冲刺外生效；装填乘数和负重通过 `MoveCtx` 传入（`step_player` 保持旧签名，等价于默认上下文；`World` 与 `PredictSim` 用 `step_player_ex`，负重暂为 `None`，待 loadout 重量数据）。⑤体力：冲刺/快爬仅在有移动输入时扣；归零（≤0.001）锁定冲刺与跳跃直到 ≥30；跳跃需 ≥15 且未锁定。⑥`PlayerAuthState` 45 B：追加 `stance_from`、`stamina`（f32 位与 100.0 的位异或，使全零尾部等于满体力的新玩家且往返精确）、`stamina_delay`、flags bit2 `sprint_locked`；世界 `state_hash` 纳入这些字段，三处金哈希（skirmish、determinism_basic、movement_replay）重定标并连跑两次稳定。⑦所有新数值进 `data/roles/*.json`（schema 同步、列入 `_meta.tunable`），武器侧 `stance_recoil/stance_spread` 常量仍保留一份（已加测试断言与数据一致，M1-014 再并入数据驱动）。
- **未做**：翻越/攀爬/探身（011b）；`self_state.stamina` 已接线（6 bit）；客户端 `LAYOUT_STATE_*` 暴露 stamina 留 M1-040；基准与 100 玩家 3 ms 需参考机；headless 回放对比待 011b。
- **受影响任务**：M1-014（命中盒/修正表）、M1-016、M1-027（`check_displacement`）、M1-040/045。
- **裁决（Fable 填写）**：

### PI-049 M1-011b：翻越/攀爬/探身的几何与状态取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim/数据
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①碰撞世界仍只有地形高度场，检测函数 `vault::detect` 接受"表面高度闭包"（现取 `height_at`，日后叠加静态道具），沿视线前向每 0.05 m 采样：0.7 m 内出现超过自动台阶（0.3 m）的陡面，继续找到顶面平台（≥0.2 m 宽，容差 5 cm），按顶面相对脚下地面的高度分类：≤1.3 m 翻越、≤1.8 m 攀爬、更高拒绝；落点取顶面或障碍后方可站立处。头顶 0.6 m 空间目前恒成立（无顶棚），数据字段 `climb_headroom_m` 已留。②触发：站立、已完成姿态过渡、地面可走、前进输入、`USE` 或 `JUMP`；体力须未锁定且足额，否则按普通跳跃处理。③运动是 tick 计数的纯函数：水平线性插值 + 抛物线抬升（顶面上方 0.1 m 过顶）；期间无输入、不开火、体力只走恢复延迟。④探身是 −1..1 的标量（±1 = `lean_deg` 20°），按 `1/lean_ticks` 每 tick 趋近；`LEAN_L/LEAN_R` 同按取消；探身键禁用冲刺（简化"不可并用"）；趴姿只抬头 `prone_peek_m`；射击起点加 `head_offset`，散布乘 `lean_spread_mult`（趴姿窥视不乘）。⑤`PlayerAuthState` 45→80 字节（lean f32 @45，翻越 ticks/total @49/@50，start/target/top 共 7×f32 @51，climb 位在 flags bit3），客户端布局常量与 wasm 金样同步；世界哈希只在探身/翻越期间多写字段，故其余场景金哈希不变。⑥新数值（15 项）进 `data/roles/*.json`，`vault_time_mult` 留给 Assault（M2）。
- **未做**：参考机基准（kcc_step、100 玩家 3 ms）；headless 回放对比（M1-044 后）；障碍物实体几何（需 M1 地图道具碰撞）。
- **受影响任务**：M1-014（head_offset）、M1-027、M1-040/045/046。
- **裁决（Fable 填写）**：

### PI-050 M1-012：穿透模块的层来源、材质数据与跳弹阈值

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim/数据
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①碰撞世界目前只有地形高度场，没有建筑/道具碰撞体，因此穿透算法对 `LayerWorld` trait 编程（给定射线返回第一个材质层的入/出点与法线），测试用合成平板；`ballistics::{integrate, materials, penetration, ricochet}` 是纯函数，随机偏转用调用方给的 `Pcg64Mcg`（新增 `Purpose::Penetration`）。接入 `projectile.rs` 命中结算归 M1-014，层来源归地图道具碰撞（M1-034/035）。②材质一材质一文件进 `data/materials/`（新 kind `material`，schema、校验器、夹具齐全）：肉/树叶/玻璃/薄木/厚木/砖/石与混凝土（合并为 `stone_concrete`）/沙袋/薄钢/厚钢/雪/水共 12 种，费用与 GD §6.7 一致；新增字段 `stop_mm`（超过即阻挡，0 = 无上限，优先于穿透力）、`min_mm`（薄于此的层不计，水 500 mm）、`ricochet`（硬材质可跳弹）、`deflect_deg`（硬 1.5/软 0.5）。薄钢与厚钢是两种材质，由调用方按厚度（≤10 mm）选取。③厚度按射线路径长度计（斜射更厚）。④跳弹：入射角（与表面夹角）< 15° 且材质 `ricochet` → 反射，动能 ×0.4（速度 ×0.632），记 1 段并结束本次追踪（继续飞行由调用方用返回的位置/方向/速度系数重新积分）；阈值【待调参】。⑤弹药数据补 `us_30_06`、`ger_792x57`、`us_50_bmg`（阻力与穿透力取 GD §6.4/§6.7，质量取公开值，待 M1-070 复核）。⑥穿透最多 2 层：第 3 层无论如何停在入口。
- **验证**：`crates/sim/tests/ballistics_pen.rs` 8 项；`cargo bench -p frontline-sim --bench projectile_step`（开发机 133 ns/段）。金样：sim-wasm 的 `native.json`（含 data hash）已重生；`GOLDEN_SEED1_18000` 与其余金哈希未变。
- **受影响任务**：M1-014、M1-017、M1-023、M1-034/035、M1-047。
- **裁决（Fable 填写）**：

### PI-051 M1-013：拆为 a（射击/装填/后坐验证）与 b（近战/副武器切换），备弹按发数记

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①M0-012 的状态机已覆盖全自动/半自动/栓动、时间累加器和装填，013a 补验证与缺口：打断装填（冲刺或翻越，取消不耗弹，`reload_cancelled`）、`ShotSpawn.spread_seed`（`rng::spread_seed(match_seed, tick, entity)` 的 16 位，对应 `FireCmd.spread_seed`）、`ammo::mag_stack`、六把武器的速率/装填/后坐测试与 `fire_rate_probe` 示例、`weapon_step` 基准。②备弹仍按"发数"记（`reserve: u16`），不引入弹匣栈：战术装填是"补满枪内弹匣"，结果与"换入满弹匣、旧弹匣带余弹回包"在 mag+reserve 总数上完全一致；`mag_stack(reserve, cap)` 给出 (整弹匣数, 部分弹匣发数) 供 UI。这避免改动 33 字节的 `WeaponState` 线格式。③013b（近战、`swap_in_ms` 副武器切换）需要武器槽/loadout 状态与目标命中体，另开 PR。④切枪打断装填待 013b（目前无武器切换）。
- **受影响任务**：M1-014、M1-016、M1-026/027、M1-040。
- **裁决（Fable 填写）**：

### PI-052 M1-013b：近战做成独立状态机；副武器切换拆为 013c

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①`weapon/melee.rs`：`MeleeState`（Idle/Windup/Cooldown + 剩余 tick + 是否冲刺突刺）、`step_melee`、纯函数 `melee_trace`；调度顺序 `movement -> melee -> weapons -> projectiles -> deaths`，近战期间屏蔽 FIRE。②近战武器按阵营取数据里的刺刀（`MeleeTable`，美 `us_m1_bayonet`、德 `ger_s8498_bayonet`）；刀、枪托、手枪托的数据与参数已就绪，等武器槽（013c）再接入。③判定用水平距离（≤ range_m）+ ±35° 扇形 + 脚高差 ≤1 m，角度比较留 0.01° 余量（大于 16 位偏航步长 0.0055°），使数据值 120°/35° 恰好命中。④背刺：攻击者在目标朝向反方向 ≥120° 内，直接击杀（伤害取 max(hp, damage)）。⑤冲刺中开始挥刺刀 → 射程 ×1.2（规格写"冲刺突刺 +20%"，取射程而非伤害）。⑥友军目标直接跳过（ff_mult=0）。⑦击杀事件 weapon 索引用 0xFF（`MELEE_WEAPON_IDX`），新增 `SimEvent::MeleeHit`；哈希仅在近战进行中写入，空闲时不改变既有 golden。⑧`swap_in_ms` 副武器切换需要武器槽/loadout 状态，拆为 013c。
- **受影响任务**：M1-013c、M1-014、M1-016、M1-040。
- **裁决（Fable 填写）**：

### PI-053 M1-014：拆为 a（伤害模型、归属窗口、命中体历史）与 b（Downed 接入）；倍率仍取武器数据

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①新增 `combat/`：`damage`（`compute_damage`/`damage_amount`、`ff_mult`、`DamageOutcome::{Hit,Downed,Killed(reason)}`，Downed 血量 40、超杀 −60）、`credit`（15 s 近期伤害窗口，供 M1-016）、`lagcomp_history`（16 tick 环，`record`/`hitboxes_at` 线性插值，回溯逻辑仍归 M1-025）。②部位倍率沿用各武器数据里的 `part_mult`（已含颈/盆骨/肢体），不另建 `hit_zones.json`；上肢/下肢同为 ×0.65，不拆 `UpperLimb/LowerLimb`。③头盔：M1 全员视为戴盔，手枪/冲锋枪头部倍率封顶 2.0（取数据值与 2.0 的较小者），步枪不受影响；无头盔数据，待装备系统。④友军伤害默认开启：子弹 ×0.5（爆炸 0.7、近战 0、载具 1.0、坦克炮 0.5），`projectile.rs` 不再跳过队友，命中队友的子弹在此终止；穿透系数 `pen_mult` 暂为 1.0（M1-012 的层来源待地图道具）。⑤Downed 状态机属 M1-016：`deaths` 仍按 `hp ≤ 0` 判死，`compute_damage` 的 Downed 结果先不被消费。⑥姿态命中盒总高 1.79/1.19/0.51 m（含半径）与 GD 9.5 的 1.8/1.2/0.5 m 在 2 cm 内，未改体积。⑦golden 重定：`bots` 因友军伤害与头盔规则改变。
- **受影响任务**：M1-015、M1-016、M1-017、M1-025、M1-040。
- **裁决（Fable 填写）**：

### PI-054 M1-015：手雷与烟雾的输入映射、公式取值与简化

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①输入：按住 `GADGET` 烹饪破片雷，开始时同时按住 `AIM` 则烹饪烟雾弹；松开投出，持雷超过引信（可烹饪的雷）在手中爆炸（持有者自伤 ×1.0）；持有者死亡则在尸体处放下继续倒计时。初始数量固定：破片 2、烟雾 1（loadout 的 gadget 计数待 M1-020）。②爆炸伤害按公式 `Dmax×(1−(r/R)²)`：任务文字的 r=2→211.7、r=6→121 与公式不符（应为 206.25、96.25），以公式为准；r=3 为 189 ⇒ ≥150 KIA 成立。③遮挡：目前只有地形，爆心（离地 5 cm）到受害者胸（脚上 1.2 m）被地形挡 ×0.25；薄木 ×0.6 与实体的材质判定待 M1-034/035 地图道具碰撞，`Occlusion` 枚举已就绪。④队友 ×0.7，投掷者自己 ×1.0；伤害进 `HitLog`（weapon 索引 0xFE）以便归属击杀。⑤引信从烹饪开始起算：烹饪 45 tick 后松开，爆炸在松开后 74 tick；新投出的雷下一 tick 才开始飞行与扣引信。⑥飞行：4 子步、重力 9.81、反弹系数 0.4/切向摩擦 0.6，静止判定 = 贴地且水平速度 <0.3 m/s、竖直速度 < 0.3 + 一 tick 重力，连续 3 tick。45° 投出 18 m/s 落点约 33 m。⑦烟雾：球体（不做地面截头），半径 `R·min(1,t/expand)·浓度`，浓度 1 直到最后 `decay_s` 起线性降为 0，浓度 ≤0.5 不再遮挡（即末 4 s 起透明）；`World::los` 先判地形再判烟雾，弹丸与穿透不读烟雾。⑧不做：手雷与玩家/墙体的体积碰撞（只与地形碰撞）、载具内乘员暴露度（M1-023）、压制输入事件（M1-017）、`ThrowableDef` 经 `/data` 的 `smoke_concentration` 字段（阈值 0.5 写在代码常量）、`target_visibility_through_smoke`（M1-016 用到时加）。⑨golden 重定：`determinism_basic`（随机输入会按 GADGET）。
- **受影响任务**：M1-016、M1-017、M1-020、M1-034/035、M1-038。
- **裁决（Fable 填写）**：

### PI-055 M1-016：拆为 a（状态机/流血/放弃/救起/归属）与 b（绷带、自然恢复、处决、环境伤害、爬行）

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①`LifeState` 新增 `Downed{t_down, killer, weapon, fatigue, give_up_ticks}`，原 `Dead` 即 KIA；倒地血量就是 `Health.hp`（40），流血与再受击都扣它，≤0 即 KIA。`is_alive()` 仍只指 Alive，新增 `is_downed()`/`is_dead()`；倒地者不能开火/近战/投掷/移动（爬行 0.6 m/s 在 016b），但保留输入以便放弃与被救。②转移：Alive 受致命击 → 超杀（hp≤−60）、爆头子弹、爆炸/载具、近战背刺 → Dead；否则 → Downed。Downed 受击 hp≤0 → Dead（`Finish`）。③流血 40 hp/80 s（0.5 hp/s），每次 120 s 窗口内的被救起使下次流血时长 −20 s，下限 20 s。④放弃：倒地满 5 s 后按住 `RELOAD` 3 s → Dead，无归属；输入键为实现者选定。⑤救起：`Medic` 标记（`World::set_medic`；玩家目前没有角色字段，兵种系统随 M1-020/loadout 补）按住 `USE`，目标是 1.5 m 内、有地形视线的最近倒地队友，5.0 s（150 tick）；松手后 3 s 内续上进度，之后重置；救起者 hp 损失 ≥20 重置；被救起者 hp 40、2 s 不能开火。⑥归属：`SimEvent::KillCredit{kind: Down|Finish|Direct, assists, team_kill}`，助攻 = 15 s 内对该受害者累计 ≥25 hp 的其他敌方玩家；友军击倒/击杀记 `team_kill`；流血耗尽/放弃无归属。保留 `SimEvent::Kill` = 进入 Dead。⑦数值暂为 `medical.rs` 常量，未进 `/data/health.json`（schema/构建管线改动留给后续）。⑧golden 重定：`damage_death` skirmish、`weapon_duel` 事件序列（致命击现在先倒地，多出事件）；M0 测试里"击杀所需发数"改为数到倒地事件。⑨`players()` 视图仍只列 Alive（倒地者不在内），bots 暂不会去处决/救助。
- **受影响任务**：M1-017、M1-019、M1-025、M1-038、M1-040、M1-056。
- **裁决（Fable 填写）**：

### PI-056 M1-016b：绷带/自然恢复/处决/摔落/爬行的实现取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①`USE` 键一键多用：持 `USE` 时，医疗兵身边有倒地队友（1.5 m）→ 救起（`medical`）；否则医疗兵给最近的受伤/流血队友绑绷带（2.5 s，+40 至 100，止血）；非医疗兵遇倒地队友 → 稳定（2.5 s，止血不回血，`Downed.stable`，不再流血）；身边无人可救 → 给自己绑（3.5 s，+25 至 75，止血）。绷带数：普通 2、医疗兵 6，用尽无效，死亡重置。②自然恢复：hp<60 且 10 s 内未受伤、未在流血 → +1 hp/s，至 60 封顶。③流血：单次伤害 ≥30 或爆炸且仍存活 → 流血 1 hp/s，到 5 hp 为止（不致死），绷带止血。④处决：对倒地敌人的近战命中不造成伤害，开始 1.2 s 处决（攻击者不能移动/冲刺/跳跃）；攻击者 hp 下降、目标不再倒地或相距 >2.5 m 则取消；完成时目标 hp 清零（`Melee` 归属 Finish）。⑤摔落：滞空峰值到落地 >3 m，每米 12 hp，≥12 m 直接 KIA（超杀同理）；伤害进 `HitLog`（weapon 0xFD，shooter=自己，无归属），不受友伤倍率。⑥爬行：倒地强制俯卧，速度 `crawl_mps`（0.6）。⑦推迟：越界/溺水/载具碾压（待地图水体、载具 M2）、`/data/health.json`（数值仍在 `medical.rs` 常量）、医疗兵绷带与 loadout 的衔接（M1-020）。⑧测试辅助：`World::{debug_set_hp, debug_teleport, debug_down}`（feature `test-util`）。⑨golden 重定：`determinism_basic`、`damage_death` skirmish、`weapon_duel`（倒地后爬行俯卧，站姿瞄准打不中，射手改为见倒地即停）；`health_m1` 的"补枪/疲劳"测试改瞄俯卧高度/等站起；重复运行稳定。
- **受影响任务**：M1-017、M1-020、M1-025、M1-038、M1-040。
- **裁决（Fable 填写）**：

### PI-057 M1-017：拆为 a（检测/累积/衰减/事件/反刷分）与 b（对移速、ADS、散布、晃动的作用；`/data/combat/suppression.json`；bench）

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①检测在弹丸每个子步内完成：该子步实际飞过的线段（被命中截断）到每名敌方玩家胶囊轴线（脚下 0.2 m ~ 头上 0.2 m，随姿态）的最近距离 ≤5 m，`s = 权重 × (1−d/5)^1.2 × min(1,v/500)`；同一发子弹对同一玩家只记一次（`SuppressSeen` 集合，弹丸消失即清理）；被命中的玩家和友军不受影响；每玩家每 tick 最多处理 12 发。②权重按武器 `class`：pistol/smg 0.8、carbine 1.5、rifle_* 2.2、mg 2.8、hmg 6.0（M1 数据里只有前几类）。③衰减：窗口（每次受压重置，15 tick）内不降，之后每 tick `(6+0.12S)×mult/hz`；`mult` 取姿态（站 1.0/蹲 ≥0.5 为 1.15/趴 ≥1.5 为 1.3）与掩体（上一个压制源到胸口地形不通视）1.4 中的较大者，不相乘。④**GAMEDESIGN 示例与公式不符**：MG42 d=2 m 1200 rpm 持续 1.5 s 示例写"≈35"，但每发都会重置 0.5 s 窗口，爆发期间无衰减，结果为 30×1.52≈45.5（4 s 钳到 100）；以公式为准，测试按 40–50 断言。⑤爆炸压制读本 tick 的 `Explosion`（kind 0）事件，`40(1−r/15)²`，r 从脚到爆心，地形挡住胸口则 ×0.5，友军爆炸同样压制；爆炸没有来源玩家，故不产生反刷分记录。⑥事件：`SimEvent::Suppress{target, source_dir, intensity}`，≤10 Hz 节流，`source_dir` = 世界方位 256 级（0=+z，顺时针向 +x），`intensity = S×2.55`；`SuppressionAssist{suppressor, victim}`：victim 被**压制者的队友**（非压制者本人）在 5 s 内击杀时，每个压制者一次（记录最多 8 条）。⑦`self_state.suppression` = `quantise4(S)`，服务器组包仍待 M0-019 之后的网络任务，这里提供 `World::suppression(id)`。⑧阶段顺序：projectiles → suppression → deaths → suppression_assists；死亡玩家在 `LifeState::Dead` 后清零（致命击当 tick 仍保留供反刷分）。⑨数值暂在代码常量，`/data` 管线与 schema 在 017b 做。⑩golden 重定：`damage_death` skirmish、`weapon_duel`（多了 Suppress 事件）；`determinism_basic`、bots 未变（状态仅在受压时进哈希）。
- **受影响任务**：M1-013（散布读 `supp_spread`）、M1-038、M1-057/063、M2 计分。
- **裁决（Fable 填写）**：

### PI-058 M1-017b：压制对移动/射击的作用与 `/data` 推迟

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①移速：在 `movement` 里把摇杆轴按 `1−0.10e` 缩放（走/跑/冲刺实测 S=100 为 0.90 倍）。②ADS 时间 ×(1+0.30e)、腰射/ADS 散布 ×(1+1.5e)：在 `weapons` 里对该玩家的 `WeaponParams` 副本缩放（S=100 时开火角偏差 RMS 为 2.5 倍）。③晃动：`Rng::for_(tick, entity, SuppressShake)` 的 12 Hz 值噪声线性插值，幅度 `0.6°·e`，叠加到该 tick 射击用的 yaw/pitch 副本（不改玩家的视角）。④`pinned`：某次受压使 S≥85 时 `pinned_until = tick+15`，期间 `weapons` 屏蔽 AIM 位（再次受压重新计时）。⑤**客户端预测**：`PredictSim` 目前不带压制输入，被压制时预测的移速/散布会与服务器有微小偏差；`self_state.suppression`（4 bit）接入预测留给 M1 预测/和解任务（M1-019 一带）。⑥推迟：`/data/combat/suppression.json` 与 schema（与 PI-055 同样处理，系数在 `suppression/mod.rs` 常量），载具炮弹近失压制（M1-023）。⑦性能：近失广相用 16 m 均匀网格，50 射手 + 100 玩家每 tick 50 发近失弹，压制额外开销 0.14 ms（预算 0.3，开发机非参考机）。⑧golden 重定：bots、`damage_death` skirmish、`weapon_duel`。
- **受影响任务**：M1-019（预测）、M1-038、M1-057/063。
- **裁决（Fable 填写）**：


### PI-059 M1-018：简化胜负规则、阵营符号与预热 API

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①占领进度 p：美军为正、德军为负；有效人数 `min(d,4)+0.5·max(0,min(d,10)−4)`，速率 0.2 %/s×有效人数，僵持不动，无人时以 0.15 %/s 回归上一任主人（中立点不回归）。②对局阶段 Idle→Warmup→Live→Ended，M1 简化胜负：全占=FullControl，否则限时比点数、再比 p 之和，仍平=Draw（无延时/加时，Overtime 仅占位）。③新增 API `begin_warmup`、`set_human_players`（真人≥2 提前开始）、`configure_match`，阶段映射到线协议 `Phase` 留待 M1-019/网络任务。④常量进 `/data/rules/match_m1.json`（schema `rule_match`，字段 `early_start_players` 带单位后缀）与既有 `capture.json`；状态按需入哈希，既有 golden 不变。⑤推迟：预热期复活 ×0.5、`zones.json` 载入（M1-034）、载具乘员计入（M2）。
- **受影响任务**：M1-019、M1-034、M1-038、网络层 Phase 映射。
- **裁决（Fable 填写）**：

### PI-060 M1-019：拆为 a（HQ）与 b（Garrison）；HQ 出生规则按需开启，常量进代码

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①M1-019 工作量大，拆 019a（HQ 出生计时/保护/选点/保护区）与 019b（Garrison、槽位、吞吐、`SupplyProvider`）。②规则仅在 `World::set_hq` 后启用，未设 HQ 的世界走 M0 老路径，既有 golden 不变；状态按需入哈希（0xD1）。③惩罚在出生到期时判定：到期时若自杀/洪水惩罚使应出生 tick 更晚则重新入队（效果等价于死亡时加 base_wait，且无需改 `deaths` 签名）；洪水只统计该队 HQ 近 10 s 的出生。④热身 ×0.5 通过 `World::step` 每 tick 调整 `RespawnPolicy.base_wait_ms`（仅 HQ 规则开启时）。⑤出生保护实现为“撤销本 tick 对受保护玩家的伤害”（恢复满血并删去其 `HitLog` 记录），所有伤害源无需改动；移动/FIRE 位取消。⑥HQ 区伤害走 `HitRec(shooter=target, kind=Explosion, direct_kia)`，无击杀归属，会引发流血（约 1 HP/s）。⑦常量（40 m 出生区、150 m 区、3 s 警告、40 HP/s、20 s/5 s 自杀、10 s/8 人/2 s 洪水、8 m 抖动、10 m 防杀半径）写在 `spawn/mod.rs`，`spawn.json` 现有字段留待 019b 读取；出生点位置抖动 8 m（任务写“出生区 40 m、点周围 8 m”，取点周围 8 m）。⑧推迟：`SpawnPointState` 事件与 `DeployResult`（随 M1-010/M1-026）、主动重新部署（随 M1-021）。
- **受影响任务**：M1-019b、M1-021、M1-026、M1-060。
- **裁决（Fable 填写）**：

### PI-061 M1-019b：Garrison 的实现取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-17
- **做法**：①放置者兵种：M1 尚无兵种系统，用 `Officer(bool)` 组件（默认 false，`World::set_officer`），M1-021 换成真实兵种校验。②“Sector”=离放置点最近的据点；该据点归敌方则花 100，否则 50；每 Sector 2 个、全队 6 个（先判队伍上限）；无据点时全部算同一 Sector。③建造：持 USE 且在 6 m 内的己方玩家算施工者，速率 1+0.6(n−1)，n≤3；生命值随进度从 20 % 线性增至 100 %。④伤害：Garrison 暂无弹丸/爆炸碰撞体，经 `World::damage_garrison(nominal, Bullet|Explosion)`（×0.05/×1.0）；后续碰撞接入随 M1-020/M1-056。⑤封锁/Overrun 以“存活且未倒地”的玩家 XZ 距离判断；封锁=最近一次有敌人 30 m 内后 3 s。⑥出生：`request_spawn` 校验后把玩家重新排队到 `max(t_kia+8 s+惩罚, now)`；到期时若点失效（毁/封锁/换队）则回落 HQ，无槽位则顺延到下一个槽回复；吞吐 ≤2 人/s=同一点相邻出生至少间隔 hz/2 tick；Garrison 出生无保护。⑦推迟：个人冷却 8 s、拆除返还（`SupplyProvider` 只有 available/consume 两个函数，返还随 M1-020 扩接口）、`DeployResult`/重新部署（M1-021/026）。⑧新增事件 `SimEvent::SpawnPointState`（建成/摧毁）。
- **受影响任务**：M1-020、M1-021、M1-026、M1-056、M1-060。
- **裁决（Fable 填写）**：

### PI-062 M1-020：拆为 a（箱/Depot/搬运/Provider）与 b（Ammo Crate/补弹/e2e）；若干取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①拆 020a / 020b。②`SupplyProvider::available/consume` 只有位置与半径，不带队伍，故 15 m 内**任一队**的地面箱都计入（双方箱在 15 m 内同时出现的情形极少，M2 若要区分再扩接口）；携带中的箱不计入。③搬运交互：持 USE 满 1.5 s 触发一次，需松开才能再次触发；优先拾取 2 m 内己方地面箱，其次 HQ Depot，再次己方据点补给点；放下位置为脚下。④Support 用 `Support(bool)` 标记（默认 false，`set_support`），M1-021 换兵种。⑤携箱移速 ×0.75 通过摇杆轴缩放（与压制同法）；“仅手枪/拒绝切换主武器”需要武器槽（M1-013c），本任务只暴露 `World::carrying`。⑥地面箱上限 40 按队计、每 tick 淘汰最早放下的；携带中的不计。⑦携带者倒地/死亡时箱落在原地。⑧新事件 `CratePlaced/CrateDestroyed/SupplyNotice{code}`（代码 1=此处不可放置）。⑨`supplies.json` 现已类型化（deny_unknown_fields），Ammo 单价表补充（MG/火箭/地雷/载具炮弹）随 020b。
- **受影响任务**：M1-020b、M1-021、M1-024（箱体下发）、M1-057/060。
- **裁决（Fable 填写）**：

### PI-063 M1-020b：补弹与 Ammo Crate 的取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①补弹目标=玩家默认兵种 loadout 的初始量（备弹 = 弹匣容量 × (弹匣数−1)，手雷 2/烟 1、绷带 2）；缺口按“夹”向上取整计价，点数不够则只补买得起的整夹（补进的发数不超过缺口）。②价格按主武器 class 映射：pistol→pistol_mag、smg→smg_mag、bar→bar_mag、mg*/hmg→mg_belt、其余步枪/卡宾→rifle_clip；`supplies.json` 与 schema 新增 bar_mag 5、mg_belt 12、rocket 20、mine 10、vehicle_shell 40（后四项暂无对应物品）。③顺序：主武器 → 手雷 → 烟雾 → 绷带；“副武器”待武器槽（M1-013c）。④Ammo Crate 立即放置（无 1.5 s 动画，客户端动画属 M1-057）；每命计数在 Support 死亡时清零，冷却跨命保留；同队友军 4 m 内持 USE 满 1.5 s 补弹，中途松开重计；空箱/到期移除并发 `CrateDestroyed`。⑤箱 HP 100，枪弹 ×0.1/爆炸 ×1（同补给箱）。⑥新事件 `Resupplied{player,crate_id,points}`、`SupplyNotice` 代码 2（无可补）。⑦数据变化后重生成 sim-wasm `native.json` 的 dataHash。
- **受影响任务**：M1-021、M1-024、M1-057/060、M1-013c。
- **裁决（Fable 填写）**：

### PI-064 M1-021a：小队、名册与平衡的取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①拆 021a（名册/小队/名额/平衡）与 021b（`validate_deploy`、loadout、换兵种冷却、bench）。②`SquadManager` 是不碰 ECS 的纯状态，持有 `PlayerRoster`（队伍、小队、兵种、得分、在线、阵亡时刻）；`World` 提供包装并发 `SimEvent::Roster(RosterDelta)`；名册为空时不参与哈希，旧 golden 不变。③平衡规则 `H_t − H_other ≤ 2` 按“加入后”差值保守解读（已领先 2 人的队不可再进人；换队同理按移动后计算）。④名册上限每队 50（`team_max_players`）。⑤兵种规则：Armor Crew 内只能是 `m1_override.armor_crew_leader_role`（crewman，每车组 ≤2）；Recon 内 M1 只有 Rifleman（Spotter/Sniper 属 M3）；步兵队不可选 Crewman；Unassigned 仅 Rifleman；Medic 补全队上限 12（GD §4.4）。⑥队长离线或 KIA 后 120 s 才换人（离队/被踢立即换）；AFK 检测需输入时间戳，留给 M1-026；无继任者则解散。⑦主动移交：队长 `Promote`，被选者 10 s 内 `AcceptLead`。⑧速率窗口按滚动 tick：60 s 前的记录恰在满 60 s 时失效。⑨小队名由类型+序号生成（Alpha…Zulu 之后 `Squad-N`、`Armor-N`、`Recon-N`）+ ≤16 字符后缀，敏感词过滤在 M1-026。⑩单位后缀白名单新增 `_squads`。
- **受影响任务**：M1-021b、M1-026、M1-039、M1-056。
- **裁决（Fable 填写）**：

### PI-065 M1-021b：部署校验的取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①`DeployReq` 沿用线上 `DeployRequest` 字段：`role`/`loadout` 为 `GameData::{roles,loadouts}` 下标，`squad` 为小队 id（0=Unassigned，小队 id 在 1..=255 内循环分配以适配 `squad:u8`），`spawn_point` 为 Garrison id，`0xFFFF` 表示己方 HQ（Garrison id 从 0 起）。②校验顺序按任务：玩家状态（无实体=DEPLOY 界面，或已阵亡）→ 小队须与名册一致 → 兵种名额 → loadout（兵种、阵营、`unlock_level ≤ 3`，M1 无 XP 视为全解锁）→ 出生点 → 换兵种冷却；返回首个失败码，码表 1..23 固定。③换兵种冷却：与上次兵种不同且距上次更换 < 30 s 才拒绝，沿用同兵种不算更换。④`World::deploy` 对没有实体的玩家立即在 HQ 生成（倒计时与 `DeployResult.spawn_tick` 属 M1-026）；阵亡者选 HQ 走既有波次，选 Garrison 调用 `request_spawn`。⑤所选 loadout 下标记入名册（`RosterEntry.loadout`），实际装备的落实随武器槽/loadout 应用任务（M1-013c 之后）。⑥`SpawnRequestError::NotDead` 在点位校验中不会出现，映射为 `NoSuchPoint`。
- **受影响任务**：M1-026、M1-056、M1-013c。
- **裁决（Fable 填写）**：

### PI-066 M1-022：载具物理的取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①不使用 Rapier 刚体：车体是一个小型确定性刚体积分器（质心、四元数、盒惯量，每 tick 3 个子步，按 `EntityId` 升序），悬挂射线直接采样高度场；避免 `enhanced-determinism` 刚体开销与第二套世界。②`vehicles/us_m4a1_75.json` 调参：悬挂 `stiffness_n_per_m` 38000→100000、`damping_ns_per_m` 3800→25000（原值 12 条射线总刚度 456 kN/m，静态下沉 0.65 m > 行程 0.6 m，会触底；新值下沉约 0.25 m、阻尼比约 0.8）。③驱动：每侧一个速度目标（前进目标按 `accel_time_s[0]+25%×范围` 斜坡上升，使 0→90% 落在 6–12 s 内，8–10 s 窗口均值≈极速），接触点力 = 速度误差 × 增益，受 `2×max_torque/0.3 m`（链轮半径常量，数据里暂无）与 `μ_f·N` 限制；倒车上限 40%；`boost` 暂无独立效果（满油门即游戏极速）。④侧向抓地按质心侧滑速度计算（不含偏航带来的滑移），否则 `friction_lateral=2.2` 会使原地转向不可能；转向 = 左右履带差速（右转=负偏航率）。⑤坡度：≤25° 满驱动，线性降到 30° 为 0；驱动归零时履带锁止（目标 0），保持在 32° 坡上（轻微蠕动）。⑥车体碰撞：沙盒里没有静态障碍，用车头（倒车为车尾）一排探针对地形；陡于履带可爬的地形算撞墙，伤害 `0.5·v²`（v>25 km/h，v 为撞向障碍的速度），每次接触只结算一次；游戏极速 28.5 km/h，30 km/h 的验收值用 28.5/24 km/h 两例代替。⑦碾压：车身足迹内、速度≥5 km/h 的活人，≥15 km/h 直接 KIA、否则 70 HP，同一人 2 s 内不重复；友军同样受伤（GD 7.6 未区分）。⑧乘员保留身体实体，仅加 `InVehicle` 组件：`movement` 跳过，位置每 tick 跟随座位；车上的射击/受击留给 M1-023；无座位时（乘员死亡）补位逻辑留给 M1-023。⑨M1 仅 Crewman 可上车；没有驾驶员且非 `set_vehicle_ctrl` 脚本控制的车辆自动手刹。⑩驾驶映射：`move_z`=油门、`move_x`=转向、Sprint=boost、Jump=手刹；换座/离车按键走 M1-026 的网络层（本任务只提供 `World` API）。⑪`PredictSim::step_vehicle_driver` 推迟到 M1-041：`vehicle::step_one` 已是服务器与预测共用的单函数。⑫`VehicleExtra` 为 12 B（最小三元数 4 B + 平面速度 4 B + 偏航率 2 B + 血量 1 B + 标志 1 B）。
- **受影响任务**：M1-023、M1-024、M1-041、M1-048、M1-071。
- **裁决（Fable 填写）**：

### PI-067 M1-023a：装甲判定的取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①`armor_hit` 严格按 GD §14.4.1 步骤实现；GD §14.4.2 手算示例（φ=30° 未做 5° normalization）与步骤 4 不一致，以步骤为准，示例仅参考。②过匹配（口径 ≥ 3×板厚）先于跳弹检查：不跳弹且 `φ_eff=min(φ_eff,45°)`。③跳弹判据为 `φ_eff ≥ 阈值`（AP 68°、HEAT 75°）。④`ShellParams` 需要口径，故在 `us_m4a1_75.json` 三种炮弹加 `caliber_mm: 75`（金哈希已重生成）。⑤手算夹具用 Python 独立计算，几何为 n=(0,0,1)、v̂=(sinφ,0,−cosφ)；随机因子由调用方注入（`luck_factor(u)=0.95+0.1u`）。
- **受影响任务**：M1-023b、M1-071。
- **裁决（Fable 填写）**：

### PI-068 M1-023b：炮塔与主炮状态机

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①炮塔 `Turret{yaw,pitch}`（相对车体）以目标方向逼近，水平速率 24°/s×模块倍率，传动损坏（≤50%）×0.5，卡死 3°/s；走最短路径。②俯仰限位取数据 −10/+25°，俯仰速率数据里没有，取水平速率的 50%（`ELEVATION_RATE_FRACTION`，待调）。③`Gun` 弹药按 AP/HE/WP 计数；开火后装填 `reload_s×倍率`，切换弹种 `switch_reload_s×倍率`（同弹种不计）；车长缺位倍率 1.3（3.9 s）；换弹种与装填均四舍五入到 tick。④本批只提供纯状态机与单测；与 `Vehicle` 的接线（炮手输入、弹丸发射、命中结算）在 023c。
- **受影响任务**：M1-023c、M1-060。
- **裁决（Fable 填写）**：

### PI-069 M1-023c：模块与车内路径

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①`ModuleSpecs` 直接读 `vehicles/*.json` 的 `modules`（HP、`at_50pct`/`at_0` 数值与布尔），`Modules::effects` 合并为速度/加速/履带推力/炮塔倍率、装填/散布倍率、`can_fire`、`immobilized`、炮塔卡死；多模块取最不利值。②殉爆与引擎起火的概率读 `cookoff_chance_pct`/`fire_chance_pct`，随机数由调用方给（新增 `Purpose::Armor=7`）。③数据没有车内几何：模块盒、乘员位置与舱室（车体/炮塔）作为 Sherman 占位常量放在 `vehicle/internal.rs`，待数据补充（另议）。④内部路径 `0.6 + P_rem/40`（≤3 m）；沿线段的模块 ×1.0、乘员直接 ×0.25（球半径 0.3 m）、同舱室其他乘员 ×0.08、车体 `0.5×dmg_pen + 0.2×模块伤害`。⑤Track L 毁后"只能转圈 ×0.3"通过 `speed_mult=0.3` 暴露，物理接线在后续批次。
- **受影响任务**：M1-023d、M1-060。
- **裁决（Fable 填写）**：

### PI-070 M1-023c/d：炮弹、击毁与 Panzerfaust 的取舍

- **状态**：resolved（实现者裁决，待 Fable 复核）
- **类别**：sim
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①载具炮弹独立于步兵弹丸：`Shells` 资源，每 tick 3 子步、仅重力（穿深衰减已含空气阻力的效果），段扫描同时测地形与敌方载具，同队载具不受击；命中后 HE/HEAT/WP 就地起爆（`explode` 抽出为 `pub(crate)`，参数 `dmax/radius/kind`，Explosion kind 1 HE、2 殉爆、3 烧毁、4 HEAT）。②载具外形碰撞体 = 车体盒（至顶甲高度 0.35 m）+ 随炮塔旋转的炮塔盒（±1.1×±1.2 m）；命中面→装甲区：车体前（以 −0.3 m 分上/下前甲）/侧/后/顶、炮塔前/侧/后/顶；板法线 = 面法线按 `slope_deg`（自竖直）向上倾。③HE 以 `he_pen_mm`（15）当穿深、`explosion_damage_max_hp` 当结构伤害走同一条判定；WP 在落点放烟（扩 3 s、衰 8 s，沿用手榴弹烟云）。④炮手输入：炮塔朝向跟踪世界 yaw/pitch，FIRE 开火，SWAP 上升沿轮换 AP→HE→WP；炮手缺位炮塔不动，车长缺位装填 ×1.3。⑤击毁：车体 HP ≤ 0 或引擎 0 HP 起火（引擎 `fire_chance_pct` 判定）→ `VehicleKnockedOut`，Burning 8 s（乘员每秒 20 HP）后爆炸 R8/Dmax200，残骸 60 s 后移除（`reap_wrecks` 回收实体 id）；殉爆（弹药架 0 HP 且 60% 判定）→ 乘员全灭、爆炸 R12/Dmax350、残骸立即计时。⑥乘员在车内由玩家身体承担伤害（直接 ×0.25、同舱 ×0.08），经 `HitLog` 记 `DamageKind::Tank`；车内伤害不分友军（同队载具本就不被击中）。⑦Panzerfaust：`Gadgets.rocket`（由部署所选 loadout 的 `at_rocket` 小工具数量决定，`spawn_player` 读取），FIRE 上升沿发射（车内不可），结构伤害取数据 1000（任务文写 900，以数据为准），爆炸 Dmax150/R5，后焰 5 m 30°半角锥、30 HP（对全部非射手生效），超过 `max_range_m`（80 m）移除；弹道无推力，60 m 有效射程需玩家自行抬高，未做瞄具补偿。⑧新增事件 `ArmorHit/VehicleDamaged/VehicleFired/VehicleKnockedOut/VehicleAmmoExplosion/CrewKilled/RocketFired`，`Purpose::Armor=7`。⑨未做（拆续）：车载 M1919/M2HB 的弹丸接线（沿用步兵自动武器，待 M1-013c）、乘员死亡后换座补位、车内几何入数据、同轴 MG 对载具/步兵的伤害（小口径对装甲 `t≥12 mm` 不击穿）、重生波次上的 rocket 授予（仅 `spawn_player` 路径）。
- **受影响任务**：M1-024、M1-026、M1-041、M1-048、M1-060、M1-071。
- **裁决（Fable 填写）**：


### PI-071 M1-024 AOI v1 范围裁剪
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①审计以离开阈值作硬上界；②转身并集取 100 ms（3 tick@30 Hz）；③3D 视锥；④参数全部来自 `aoi.json`；⑤团队通道已实现但传输接线推迟到 M1-026/029；⑥载具快照实体/24 B 记录/调度 ×1.5、LoS 缓存、netem 带宽、`reports/m1-aoi.json`、criterion `aoi_select` 拆为 M1-024b。
- **受影响任务**：M1-026、M1-029、M1-024b。
- **裁决（Fable 填写）**：

### PI-072 M1-025 延迟补偿范围与落点
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①历史记录与回溯时间选择放在 `sim`（弹丸阶段需要读取，且预测端同源），`server` 只做计数；②`view_tick==0` 视为“未上报视角”，不补偿（脚本/机器人/预测开火不受影响）；③弹丸在飞行全程使用恒定偏移 `tick−rewind`（弹速下飞行 ≤ 数 tick）；④起点始终取射手权威位姿，0.5 m 校验仅作函数与单测，起点校正指标待 025b；⑤墙体保护、`lagcomp.json`、载具历史、netem 命中率场景、criterion、20 客户端 P99 拆为 M1-025b。
- **受影响任务**：M1-026、M1-027、M3-022、M1-025b。
- **裁决（Fable 填写）**：

### PI-073 M1-026 范围与 NFKC
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①NFKC 需要未列入依赖白名单（ARCHITECTURE §4.5）的表 crate，M1 只折叠全角 ASCII 与全角空格，其余 Unicode 原样保留，待白名单决定后换成完整 NFKC；②`Command::C0` 经 sim 线程串行处理，回复直接写连接句柄；③`Command` 不再 `Copy`；④聊天 `Command` 频道与 `MarkerPlace` 暂丢弃（记 `chat_rejected{channel}`）；⑤小队操作失败暂无线上错误码（仅计数），`DeployResult.reason` 即 sim 码；⑥C4 总线/名册增量/重连/丢包测试/背压/规则泄漏脚本拆为 M1-026b。
- **受影响任务**：M1-026b、M1-040、M1-056。
- **裁决（Fable 填写）**：

### PI-074 M1-027 范围、视角限幅与 view_tick 语义
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①协议文档把 `view_tick` 定义为相对 tick 的差值，M1-025a 误按绝对 tick 实现，本次改为差值（`rewind_ticks(delta)`，0 = 未上报，>8 夹紧）；②8000°/s 限幅在 30 Hz 下永远不会触发（单 tick 最大转角 180° = 5400°/s），验收 3 的“180° 瞬转被钳制到 266.7°/tick”自相矛盾；限幅保持可配置，测试用 3600°/s，建议 Fable 裁定新的默认值（合法甩枪峰值 3000°/s = 100°/tick）；③“射速拒绝”定义为武器仍在冷却/拉栓时的扳机上升沿且 sim 未发射（连发持枪不计）；④`suspicion_move` 计移动向量超限（`move_clamped`/`bad_move`），因协议不含位置；⑤`anticheat.json` 数据化、`pred_hash` 统计、origin_hint 校正计数、分级断开、headless 注入器、20 客户端混合与 criterion 拆为 M1-027b。
- **受影响任务**：M1-025b、M1-027b、M1-040。
- **裁决（Fable 填写）**：

### PI-075 M1-028 范围与实现选择
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①首个切片用内存注册表/结果表，SQLite（sqlx）迁移、`guests` 表与指标拆 M1-028b（sqlx/libsqlite3 编译体量大，先验证 HTTP 面）；②游客 token 手写 HS256 JWT（hmac+sha2+base64，不引 `jsonwebtoken`），ticket 为 Ed25519（`ed25519-dalek`）；③`ServerInfo`/`TicketClaims` 在 `protocol::dto`，JSON 形态由 gateway 自带镜像结构，protocol 保持无 serde；④`ticket::verify`+`JtiCache` 目前在 gateway，M1-029 需要同样的验签：优先复制到 server（避免 server 依赖 gateway，§4.5），或抽出共享小 crate，待 Fable 裁定；⑤ticket 的 `player_id` 由 gateway 按对局轮转分配（0..127），与 server 的 `alloc_player_team` 需在 M1-029 对账；⑥所有鉴权先于请求体解析（未认证返回 401 而非 422）；⑦内部 API 的传输安全属部署（M1-066/067）。
- **受影响任务**：M1-029、M1-028b、M1-055。
- **裁决（Fable 填写）**：

### PI-076 M1-029 范围与实现选择
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①gateway 在注册时分配 `match_id` 并随响应下发 ticket 公钥，server 以此装载验签器（`TICKET_PUBLIC_KEY_FILE` 仅作“钉住”校验：不一致则拒绝启动），server 自己的 `--match-id` 只用于日志；注册完成前到达的连接以 `not_ready` 拒绝；②ticket 无 `kid` 字段，多公钥时逐个尝试（轮换期 ≤2 把，验签 <0.1 ms 量级），不改 ticket 格式；③验收写“容忍时钟偏差 ±2 s”与“61 s 必须过期”互相矛盾：默认偏差容忍为 0（`with_skew` 可配），exp 严格判定；④`Reject.reason` 线上仍只有 `InvalidTicket`（协议码表不改），细分原因（expired/replayed/bad_signature/malformed/wrong_match/not_ready）只进 `ticket_rejected_total{reason}`；⑤server↔gateway 链路用明文 HTTP（reqwest 不开 TLS 特性，避免 webpki-roots 的 CDLA 许可证与 deny 清单冲突），传输安全交部署层（内网/反代，M1-066/067），`https://` 的 GATEWAY_URL 被拒绝；⑥JSON 体（注册/心跳/结果）由 server 自带结构，与 gateway 的 `RegisterReq` 镜像，`MatchRegister` 未放进 protocol（protocol 保持无 serde）；⑦发布构建不强制配置 gateway（M0 压测脚本在 release 下无 gateway 运行），仅 `--dev-no-gateway` 在 release 下为配置错误；未配置 GATEWAY_URL 时启动日志 warn 且不验 ticket；M1-066 部署必须传 GATEWAY_URL；⑧ticket 的 `player_id`/`nick`/`is_admin` 暂不写入会话（server 仍按 `alloc_player_team` 分配 PlayerId，与 PI-075⑤对账结论：以 server 为准），写入会话与重连令牌拆 M1-029b；⑨局终 result 目前只含人数/bot 数/时长/tick，据点数在沙盒图恒为 0，击杀/死亡/得分行待 M1 模式任务。
- **受影响任务**：M1-029b、M1-055、M1-066/067。
- **裁决（Fable 填写）**：

### PI-077 M1-030 范围与实现选择
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **做法**：①UTM 换算用纯 Python Krüger 级数（与 pyproj 差 <1 mm，测试在装有 pyproj 时交叉校验），不把 pyproj 加进管线依赖；②MAPS §3.4 只给出教堂的真实坐标源，其余 7 个锚点的“真实”E/N 暂取设计框架值（框架原点 + 设计坐标），在 `meta.json` 标 `provisional:true`，因此当前相似变换为恒等、`applied:false`，待 H-051 填入真实坐标后重新拟合；锚点 `n13_nw_corner`、`flood_east_edge` 是据 MAPS 文字取的边缘/范围点（±300 m），仅用于凑足 8 个与交叉校验；Augustine 修道院位置未核实，不入锚点；③数据源台账 id 约定为 `AST-M-{LIDAR|ALTI|GLO30|OSM}-01`（req_id `M-CAR-001..004`），草稿行全部 `PENDING`，`verified` 仅在台账 `status=OK` 且存档存在时取台账 `purchase_date`；④任务写的 “`validate-sources` 在 MISSING/PENDING 时返回非 0” 与“人类登记前应被允许”冲突：以 `meta.m1_sources_checked`（默认 false）为开关，false 时只报告不失败，结构错误始终失败；⑤meta 的“JSON Schema”暂用 `carentan.validate_meta` 手写校验，正式 schema 文件与 CI 接入拆 M1-030b；⑥`height.hmin_m/hmax_m` 为 null（M1-031 填）。
- **受影响任务**：M1-031、M1-032、M1-034、H-051/H-052/H-004。
- **裁决（Fable 填写）**：

### PI-078 停用 GitHub Actions，改为本地合并门槛
- **提出者 / 日期**：用户 TT 指示 / 2026-11-18
- **做法**：①`.github/workflows/*.yml` 的所有自动触发（pull_request/push/schedule）已去掉，只保留 `workflow_dispatch`（需要时可手动跑；文件保留作为检查清单的权威定义）；②新增 `scripts/local-ci.sh [组…]`（lint / rust / ts / data / determinism / wasm / assets / map），复刻各工作流的步骤；缺工具的组报告 `SKIPPED` 并以退出码 3 结束（不算绿）；③合并前门槛：本次改动涉及的组必须通过（Rust 改动 → lint+rust+ts，数据/schema → data，地图工具 → map，sim/wasm → determinism+wasm，素材 → assets）；云端容器没有 wasm-pack/cargo-deny，determinism 与 wasm 两组需在 Linux 工作站上跑（Remote Control），在那之前凡改动 sim/sim-wasm 的任务必须先在工作站跑过再合并；④`manifest-budget` 依赖客户端构建产物，已并入 assets 组（有 `client/dist/size-report.json` 时才跑）；⑤CI 实跑里程碑相关的 H-102 事项就此失效（改为本地基线）。
- **受影响任务**：M1-001–004（CI 墙钟判据）、所有后续任务的合并流程。
- **裁决（Fable 填写）**：

### PI-079 H-052 / H-004 / H-051 由执行者处理的结果（用户委托“100% 由你处理”，2026-11-18）
- **提出者 / 日期**：M1 线程 / 2026-11-18（经协调会话转达）
- **H-052 许可证核实**（读了官方页面，存档说明在 `licenses/texts/AST-M-*.txt`，台账行已写入 LICENSES.md §3）：①IGN LiDAR HD：data.gouv 数据集页标注 Licence Ouverte 2.0、生产者 IGN；Etalab 2.0 官方页确认可商用、须署名（至少署名人/来源/最近更新日期）→ `OK`；②OSM：ODbL，署名 “© OpenStreetMap contributors”，署名指南明确游戏可在 splash/游戏内/Credits 署名，且提取 OSM 数据的作品可构成 Derivative Database → `OK`（见 H-004）；③Copernicus GLO-30：官方许可 PDF 已读，授权复制/分发/改作、免费、全球、无国家例外，署名文本与责任声明见存档，但**未写明商用**→ `FLAGGED`（仅远景环兜底用，只有真用到才需决定）；④IGN RGE ALTI：只查到 Occitanie 的再利用页（许可 Other (Attribution)），并非 IGN 官方页 → 保持 `PENDING`，不使用；⑤**仍未核实**：LiDAR HD 在 Calvados/Manche（Carentan）的覆盖与飞行年份（M1-031 取数时核对）、历史影像/老图的条款（只作参考，不入库，无需核实）。以上不是法律意见，公开发布前仍建议一次法律复核。
- **H-004 OSM/ODbL**：按 BRIEF R9 默认执行并确认——接受衍生矢量层按 ODbL 公开、游戏内署名；`data/maps/*/` 矢量层在 `LICENSE_NOTICE.md` 声明 ODbL。若用户日后否决，退路是只作视觉参考、从历史图重新描摹（工时 +30–50%）。
- **H-051 锚点校验：未完成**。云端容器代理拦截了 Overpass/Nominatim（robots 与 403），维基页面不含坐标，所以除教堂（Mérimée PA00110352，±40 m）外的锚点仍是 `provisional`。可行解：在 Linux 工作站（有直连网络）用 Overpass 查教堂/广场/火车站/闸口/N13 桥的 OSM 坐标，填进 `carentan.py` 的 ANCHORS 后重跑 `carentan-meta`；这一步我会在工作站会话里做，不需要用户。
- **受影响任务**：M1-031（取数前核对 Manche 覆盖）、M1-032、M1-034。
- **裁决（Fable 填写）**：

### PI-080 M1-030/031：真实锚点（H-051）与 DEM 阶段取舍
- **提出者 / 日期**：M1 线程 / 2026-11-18
- **H-051 结果**（OSM Overpass 坐标，工作站查询，`reports/m1-anchor-osm.json`，ODbL）：教堂 OSM 位置与 MAPS 目录坐标相差 3.4 m（设计框架成立，恒等变换以教堂平移 dx=3.13 m、dy=−1.36 m 为准，`applied:false`）；共和国广场设计偏差 255 m（>2×±120，标 `design_revised`）、火车站 344 m（±250 内，不参与拟合）；运河闸口真实位置在框外约 (2434, 2211) m，与设计 (1200,1350) 差 1.5 km，说明 MAPS 此处的暂定坐标有误，N13 桥（OSM way 94251398/403）也在框东约 300 m 外，框内没有 N13 桥。**据点/出生点设计应以真实地标（本表）重新校准**，由 M1-034 处理；Augustines 修道院与 Pont de Verdun 的真实位置已入表，作为“真实地标”（无独立设计位置，不参与拟合）。
- **拟合规则细化**：仅 `confidence_m ≤ 150`、设计位置独立且偏差 ≤ 2×confidence 的锚点参与拟合；少于 2 个时用单点平移。避免了让 250 m 级的暂定位置把相似变换拽出虚假旋转。
- **DEM（H-052 补充）**：LiDAR HD 覆盖 Carentan：任务 22LHDFD，飞行 2022-08-29–09-12，MNT 0.5 m（IGN69），15 块、约 229 MB，在工作站 `assets/incoming/maps/carentan/dem/`（不入库）。
- **M1-031 取舍**：①`heightmap_1m.r16` 存的是已切片混合后的 2049² 高度（矩形内原样 1 m，外侧 32 m 羽化到 8 m 远景环上采样）；②LiDAR 瓦片只覆盖约 ±1.5 km，远景环其余区域用“最近有效样本外延”填充（不使用许可证 FLAGGED 的 GLO-30），`meta.height.far_ring_lidar_coverage` 记录覆盖率；③基准偏移暂取 1 m DEM 的 0.1 百分位（尚无水体掩膜，`offset_method` 已标注），待 M1-032 的水体矢量出来后改为最低水面；④验收 1 需要独立 DEM（RGE ALTI 官方页未核实，不用）：`reports/m1-dem-check.csv` 目前是锚点处“重采样 vs 原始 0.5 m 点值”的自洽检查，不是独立来源核对，STATUS 如实标 partial；⑤2049² 不含 256 m 缓冲区（缓冲只用于远景环，已由 8 m 环覆盖）。
- **受影响任务**：M1-031、M1-032、M1-034。
- **裁决（Fable 填写）**：


## PI-081 M1-032 OSM 矢量初稿取舍
- **相似变换不作用于 OSM**：`meta.similarity` 描述 design→real，OSM 本身就是真实坐标；故只做 WGS84→UTM30N→本地米。设计坐标的校准留给 M1-034。
- **堤道**：`motorway/trunk` 标 `causeway_candidate:true`（class 仍 paved），清单写入摘要（2 条）；N13 堤道本身在盒外约 300 m，真正的 `causeway` 类由 M1-033 判定。
- **现代剔除**：零售/工业/学校/医院/火车站/service 建筑、`landuse=retail|industrial`、骑行道、废弃/拆除/站台铁路；`wall`/`fence` 暂不入树篱层（7 条跳过）。
- **字段名**：规格里的 `walkable_depth` 为数值，按单位后缀惯例命名 `walkable_depth_m`；服务道路宽默认 4 m。
- **清洗**：自相交面直接丢弃并计数（真实数据 0 个），不做修复；悬空线延伸未实现（只删 <2 m）。
- **树篱稀疏**：OSM 仅 2 条 hedge + 31 条 tree_row，真实树篱需 M1-033 描摹。
- **受影响任务**：M1-033、M1-034、M1-035。
- **裁决（Fable 填写）**：


## PI-082 M1-033 脚本先行、人工输入缺失
- 人工描摹/洪泛多边形/剔除清单不存在，不用 OSM 或自动生成冒充；`edits-1944` 以退出码 4 报告 partial，M1-034/035 用 OSM 初稿灰盒继续。
- 栅格化不用 GDAL：纯 numpy 偶奇规则 + 到边距离 smoothstep 羽化（`transition_m` 在多边形外侧），无需工作站。
- 洪泛多边形入库时 `depth_m` 0.8、`walkable_depth_m` 1.0（≤1.4）、`vehicle_blocking:true`，为暂定值待人工修正。
- 自动现代剔除 63 项写入 `removed_modern.json`；`remove_ids.txt` 仅作用于 roads/buildings。
- **受影响任务**：M1-034、M1-035。
- **裁决（Fable 填写）**：


## PI-083 M1-034 取舍
- **相似变换未应用**（PI-081）：设计坐标即本地坐标；据点落入建筑/水体时吸附 ≤60 m 并记 `notes`。L3a 教堂周边吸附 14.6 m。
- **入口判据**：只在以据点为圆心、半径 R 的圆内做步行 BFS，统计到达圆周的扇区数（12 扇区），避免绕外圈作弊；HQ 用同一判据，必要时在 ≤60 m 内选最近的开阔位置（US HQ 移动 4.0 m）。
- **Garrison**：采用 MAPS §3.8 的值（US (650,1470)、GER (1340,720)），任务书里的 (420,1550)/(1500,600) 与 MAPS 冲突，以 MAPS 为准。
- **掩体点**：来自建筑外侧 1.5 m 与树篱顶点；无 OSM 要素处（L1a）以 `placeholder_ring` 占位并显式标注，不冒充实测。
- **载具路线**：N13 堤道在盒外，起点取 US HQ 最近的可通行路节点；咽喉点为桥或宽度 <4.6 m 的路段。
- **地标**：只有教堂在 OSM 命中；农场/南桥/拱廊广场未命中，保留设计值待人工核实。
- **受影响任务**：M1-018/019（读取 m1/zones、spawns）、M1-035、M1-052。
- **裁决（Fable 填写）**：


## PI-084 M1-035 区块格式取舍
- **压缩**：环境无 brotli，区块用 zlib-9（高度做 delta+字节平面）；容器每段带 codec 字段，换 brotli 只需改编码器与 FORMAT.txt。浏览器 `DecompressionStream` 原生支持 deflate，也是实际考虑。
- **字节**：单块 15–21 KB，低于规格估计的 30–80 KB，原因是草只存种子、OSM 植被稀疏；真素材/人工树篱到位后重测。
- **对角线**：每个 1 m 格沿 (0,0)-(1,1) 对角线剖分；CDLOD（M1-049）与 Rapier `HeightField` 须同向，不一致时再登记。
- **打包**：`.fpak` 与 manifest 条目未接 M1-003/005，区块先落 `assets/processed/`，`index.json` 带 `data_quality:"draft"`。
- **splat**：dirt 占 21%（建筑 2 m 缓冲 + 小径/铁路），偏高，待 M1-035b 与美术调参。
- **受影响任务**：M1-036、M1-049、M1-050、M1-051。
- **裁决（Fable 填写）**：


## PI-085 M1-036 导航网格取舍
- **依赖**：选 `recast-navigation` 0.44.0（npm，MIT，WASM 内含 Recast/Detour），放在 pnpm 工作区包 `tools/nav-bake`（任务书写的 `tools/map-pipeline/nav/bake.ts` 无法放进 `tools/*` 工作区，Python 侧入口仍是 `map-pipeline nav --slice m1`）；仅构建机使用，已登记 LICENSES.md §5。
- **参数**：步行取 ARCH §16.2（cell 0.3、radius 0.4、climb 0.5、slope 45°），MAPS §6.2（坡度 35°、台阶 0.4）差异在此登记；KCC 实际步行坡度 35° 记在 `navmesh_params.json`。载具网格 cell 取 0.5 m（半径 2.0 m）以提速。
- **建筑**：非可进入建筑在 1 m 地形上挖洞（实心）；可进入建筑保留地面并加 0.3 m 厚、3 m 高的墙，门洞处留 2.4 m 缺口（门宽 1.2 m 不足以通过 Recast 腐蚀后的 0.8 m 直径，1.6 m 缺口有 8/70 栋不可达，2.4 m 全部可达）。只有地面层，楼层/楼梯待灰盒。
- **水**：深水格（`build_grid` 的水体掩膜）从地形三角网中挖洞；涉水区未单独标区域（recast-navigation 的生成器不提供按多边形标区域，需自写 tile 构建，留 M1-036b）。
- **树篱**：`vehicle_blocking` 树篱用 0.6 m 宽、2.5 m 高的盒体整段阻挡（步兵/载具同），无 vault 链接（草稿树篱全是 2.0 m）。
- **header**：`params_hash` 为 `navmesh_params.json` 内容哈希前 8 字节，由 Python 传给烘焙脚本。
- **受影响任务**：M1-037（加载器需 Detour 兼容反序列化）、M1-034b、M1-052。
- **裁决（Fable 填写）**：


## PI-086 M1-037 取舍与对 M1-036 的更正
- **更正（M1-036a）**：#51 合并时的连通性结果有漏洞——Detour `computePath` 在节点池（默认 2,048）耗尽时返回部分路径且 `success=true`，当时 L3a / GER HQ 的“连通”其实未走到终点（报告长度 1,018/894 m 小于直线距离即是征兆）。已改为：终点距目标 <3 m 才算连通，节点池 65,535；重烘焙后 8/8 条路径真实连通（L3a 1,245 m、GER 1,642 m）。同时 `max_edge_len_m` 由 3.6 m 提到 24 m，多边形 22,243→17,839。
- **A* 上限**：规格 4,096 展开上限在老城绕行时不够（GER HQ 需 7,526 次展开），改为 16,384，仍保证终止；60 次随机查询最大展开 6,638。
- **代价模型**：节点=多边形，代价取门户中点到中点，启发 0.999×直线距离，允许重新打开（成本更优时）；与 Detour 路径长度差 <5%。
- **门户朝向**：用多边形中心连线的叉积判定左右，不依赖多边形绕序。
- **范围**：本片不含楼梯 off-mesh（网格无楼层）、未接入 `BotBrain`（M1-038）、未做载具让行接线。
- **受影响任务**：M1-038、M1-034b、M1-036b。
- **裁决（Fable 填写）**：

## PI-087 M1-038 拆分：引擎先行（2026-11-18）
M1-038 体量为 L。先交付与场景无关的引擎（行为树、效用、迟滞、分片）并用单测量化，叶子动作与 `SoldierBrain` 接入、B-xx 场景测试拆到 038b；理由：引擎可独立验证，接入需要 M1-039 的小队指令接口一起设计。状态 `initial`，待 Fable 审阅。

## PI-088 M1-038b：战术逻辑纯函数化，接入延后（2026-11-18）
目标打分与战斗树做成与世界无关的纯函数(`bots::tactics`)，用 Situation 输入做场景测试；接入 `SoldierBrain` 会改变 M0 golden 回放哈希，且需要 M1-039 的 `SquadOrder`，故合并到 038c/M1-039 一起做并重钉哈希。参数(阈值/曲线)均为 `initial`。

## PI-089 M1-039a：纯逻辑先行（2026-11-18）
兵种选择直接复用 sim 的 `role_available`（不重复校验，符合任务备注）；填充/让位/重连宽限与小队长选点做成纯函数，host 接线与 Officer/Support 行为留 039b。`SquadOrder` 暂定义在 `bots::squad_leader`，接线时再决定是否上移到 sim/protocol。

## PI-090 M1-039b：先补指标与常量统一（2026-11-18）
`bots_active{team}` 与 `[BOT]` 前缀共用已落地；FillManager 接入 `sim_thread` 需要动态 spawn/移除 bot 的部署路径（M1-026 处理器）及 20 人无头局，留 039c，与 Officer/Support 行为合做。

## PI-091 M1-040a：先做纯状态模块（2026-11-18）
部署状态机、去重、击杀信息环、roster 合并先作为与 signals/协议解码无关的纯类落地并测试；roster 增量采用 per-player `seq` 取最大（假定 M1-010 的 `PlayerRoster` 增量带可比较序号，若实际字段不同按 protocol 为准再改）。`apply.ts` 与 EventMap 扩展留 040b。

## PI-092 M1-040b：C4 事件无 event_id，去重只用于 C2.events（2026-11-18）
任务写的"C4 按 event_id 去重"与协议不符：`ServerEventC4` 各消息没有 event_id（C4 是可靠流，不会重复），重复投递只发生在 C2.events 的冗余发送。因此 `GameState.apply` 不做去重；`EventDedup` 留给 C2.events 解码层。热路径堆增量验收改为结构性断言（槽位对象复用），数值验收待专用 `--expose-gc` 基准。

## PI-093 M1-041a：先做座位状态机与外推辅助（2026-11-18）
载具预测本体依赖 sim-wasm 导出载具步进（需 wasm 重编与对账接线），在云端无法完成端到端验收(netem/headless)，故先落地与 wasm 无关的纯逻辑。换座 2 s 取 60 tick（30 Hz）；停车阈值 5 km/h 取自 GD 14.7，均为 `initial`。

## PI-094 M1-042a：变体规则（2026-11-18）
任务给"4 宏×4 pass≤60"。规范化规则：depth/shadow 不用 normalMap/detail，alpha 通道不用 alphaTest（混合与裁切互斥）；据此合法变体 32、Low 20，留出 28 个余量给后续宏（如 USE_INSTANCING）。该规则为 `initial`，着色器实现（042b）如发现 alpha 需要 alphaTest 再调整。

## PI-095 M1-043a：纯数学先行（2026-11-18）
光照任务的验收几乎全依赖 GPU/参考机；先落地不依赖渲染器的 CSM 参数与静态级联调度、探针权重与预算、预设混合。Low/Medium 档的分割与分辨率（`low`: [20,80,250]/1024；`medium`: 4 级 1024）为本任务自拟的 `initial` 值，规范只给了 High 与 WebGL2；调参留给 M1-043b 实测。地图光照预设尚未进 `/data`（`parsePreset` 已按期望字段校验），写入随 M1-064。

## PI-096 M1-044a：色调映射选 ACES 近似（2026-11-18）
任务允许 ACES 近似或 AgX 任选；先选 Narkowicz ACES 近似（实现最小、两路径好对齐），写入 `post-math.ts` 作为着色器的参考实现。曝光适应采用速率限幅而非指数平滑，以保证"≤2 s 无超调"；3 EV 以上阶跃会超过 2 s（1.5 EV/s 的直接后果），如需更快再调速率，【待调参】。

## PI-097 M1-045a：站→趴不经俯冲取 0.75 s（2026-11-18）
GD 9.2 只给了俯冲站→趴 0.7 s；非俯冲的站→趴本任务按经过蹲姿处理（0.25+0.5=0.75 s），属 `initial` 假设，待 Fable 确认。步幅 1.6 m/周期、sprint/run 速度阈值 2.6 m/s、LOD 滞回 5 m 均为自拟初值，LOD 距离阈值按任务要求最终应读 `/data` 或配置，当前由调用方传入。

## PI-098 M1-046a：贴墙下限（2026-11-18）
"枪口距墙 ≥5 cm"在相机距墙 < 5 cm 时无法满足（回缩已达枪长上限）；此时枪体整体缩回相机处，由近裁剪面处理。FP 默认 FOV 54° 与 ADS 倍率 1.2（按 tan 半角缩放而非直接除角度）为 `initial`。

## PI-099 M1-047a：驱逐是线性扫描（2026-11-18）
满池驱逐对 ≤1600 槽线性扫描，3 000 请求/s 下约 5M 次比较/s（<2 ms/s 级），M1 可接受；若实测占用主线程预算再换成按优先级分桶的空闲链。优先级数值（255/128/32）与 100 m 合并距离为 `initial`。

## PI-100 M1-048a：socket 命名不一致（2026-11-18）
任务书要求 socket 名 `socket_muzzle`/`socket_seat_*`/`socket_hatch`…，而 `data/vehicles/us_m4a1_75.json` 的 `gameplay.sockets` 只有 `socket_gun_muzzle`/`socket_turret`/`socket_hatch`。渲染侧先按任务书的 9 个名字做名单；数据里的 `socket_gun_muzzle` 需在 M1-048b 映射为 `socket_muzzle`，或由数据侧补齐（M1-070 复核时一并处理）。履带轨距 = 车宽 − 履带宽的具体数值待真模型，当前由调用方传入。

## PI-101 M1-049a：解码视为 fetch 后立即完成（2026-11-18）
核心把 `fetched()` 之后直接置 `uploading`，省略独立的 `decoding` 状态（解码在 Worker 池内、对主线程状态机不可见）；`ChunkState` 类型仍保留 `decoding`/`requested`/`evicting` 以便与任务书接口一致。上传耗时估计系数（2 ms/MB）为 `initial`，M1-049b 用 `stream_upload_ms` 实测后替换。

## PI-102 M1-050a：BC7 下 3 图层材质超 24 MB（2026-11-18）
验收 4 要求 8 层 1024² 含 mip 总显存 ≤24 MB。按 `TerrainLayerDef` 每层 albedo+normal+ORM 三张图计：BC7(8 bpp) 约 32 MB，超出；ASTC 6×6 约 14.2 MB、ETC2 约 16 MB 达标。建议（待 Fable 裁决）：BC7 设备把 normal 用 BC5(8 bpp)并把 ORM 合进 albedo 的 alpha 之外的独立 512² 层，或在 BC7 设备上把层纹理降到 768²/2 张图；M1-050b 实测真实格式后定。当前仅记录核算，未改预算。

## PI-102 裁决（2026-11-18，按全自主规则自行决定）
BC7/ASTC 4×4 等 8 bpp 设备：albedo 保持 1024²，normal 与 ORM 降到 512²（共 16 MB，≤24 MB 预算）；ASTC 6×6/ETC2 保持三张全 1024²（14.2/16 MB）。理由：法线与 ORM 对远距离画质影响小，且不需要额外的通道合并工具；`terrainLayerBytes` 有单测。若 Fable 审阅有异议再改。

## PI-103 M1-051a：M1 区域树篱仅 ~708 m（2026-11-18）
用 `hedgerows.geojson`（OSM 草稿，32 条）按 380–1560 × 520–1700 切片实测树篱总长约 708 m，验收要求 >4 km。这是 M1-033 已登记缺失的"人工描绘树篱"输入；渲染/碰撞管线按段数据驱动，数据补齐后无需改代码。此项在 `docs/NEEDS_HUMAN.md` 已有对应项则并入，否则新增条目。
