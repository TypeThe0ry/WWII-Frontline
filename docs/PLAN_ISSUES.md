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
