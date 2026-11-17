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

