# M2-007 protocol：M2 消息（建造、资源、投票、Ping）

## 元数据
- 里程碑：M2
- 工作流：net
- 规模：S
- 依赖：M1-010
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §6.8; ARCHITECTURE §8.2; ARCHITECTURE §6.6; GAMEDESIGN §16.4; GAMEDESIGN §19.3; GAMEDESIGN §16.3

## 背景
M1-010 定义了 C0（Deploy/SquadOp/Chat/Marker/Report）、C4、C2.events、载具与团队通道。M2 的规则任务（建造 M2-010、资源 M2-009、投票 M2-018、Ping M2-019）需要新消息；本任务**先行**加入 `crates/protocol`，避免规则/UI 返工（MILESTONES §3.8 风险）。协议演进规则（ARCH §8.2）：`PROTOCOL_VERSION` 握手时严格相等；字段只在末尾追加，破坏性修改升版本；解码路径禁用 `unsafe`，所有长度/范围检查（ARCH §19.1，字符串：昵称 ≤ 24、聊天 ≤ 200）。
已有通道（ARCH §6.6.1）：C0 control 可靠有序；C4 可靠事件（S→C uni stream）；C2.events 不可靠冗余；`WorldState`（1 Hz）已含 `resources:[manpower:16b, munitions:16b, fuel:16b]` 与 5 个据点状态（ARCH §6.8.2）。敌方标记 `Marker.pos` 量化到 5 m、最高 1 Hz（R3）。

## 具体内容
### 要创建/修改的文件
- `crates/protocol/src/msg/build.rs` —— 建造类消息
- `crates/protocol/src/msg/vote.rs`、`ping.rs`、`resource.rs`
- `crates/protocol/src/lib.rs` —— `PROTOCOL_VERSION` 递增（在 M1 值上 +1）；`ClientMsgC0`/`ServerMsgC0`/`ServerEventC4` 枚举追加变体
- `crates/protocol/tests/` —— 往返、proptest、fuzz 目标
- `crates/protocol/fuzz/fuzz_targets/m2_messages.rs`
### 要实现的行为
1. C0（C→S，可靠）新增：
   - `BuildPlace{ kind:u8, pos:q(16×2), yaw:u8(256 档), nonce:u16 }`、`BuildCancel{ id:id16 }`、`BuildJoin{ id:id16 }`（加入施工）、`Demolish{ id:id16 }`
   - `PingPlace{ kind:u8{attention,enemy,need_medic,need_ammo,need_engineer,need_at,need_supplies,thanks}, pos:q(16×2), target:id16 }`（GD §16.4 的 8 项；自动识别由服务器确定 `target`）
   - `VoteStart{ kind:u8{kick,remove_commander,mute}, target:u8 }`、`VoteCast{ vote_id:u8, yes:bool }`、`TeamSwitchRequest{ team:u8 }`、`ForgivePunish{ victim_event:u16, punish:bool }`（TK 15 s 选择，GD §19.2）
2. C4（S→C，可靠）新增：`StructureState{ id:id16, kind:u8, team:u8, pos:q, state:u8{ghost,building,done,destroyed}, hp_q:u8 }`、`ResourceDelta` 已有（M1），本任务补 `reason:u8`；`VoteState{ vote_id, kind, target, yes, no, ends_in_s }`、`VoteResult{ vote_id, passed }`、`TkPrompt{ killer:u8, expires_s:u8 }`、`TeamBalanceNotice{ code:u8, args:[i32;2] }`
3. C2.events 新增 `PingMarker{ kind:u8, pos:q, ttl:u8, owner:u8 }`（友军可见范围由服务器筛选，客户端不过滤）；`Marker` 的 `kind` 枚举扩展 Team Mark / Squad Order / Fire Request 等（GD §16.3），仅追加枚举值。
4. 位级量化遵循 ARCH §6.8 约定（`q(n,r)`）；`kind` 超出枚举 → `CodecError::OutOfRange` 并断开前先计数。
5. 所有新增消息进入 `Message` 的编码/解码表；消息 ID 在末尾追加，不改动已有 ID。
### 接口与数据
- 沿用 `Codec` trait（ARCH §8.2）：`encode(&self, &mut BitWriter) -> Result<(), CodecError>`、`decode(&mut BitReader)`。
- `BuildKind` 枚举取值：`barbed_wire, tank_trap, sandbag, foxhole, mg_nest, repair_station, resource_node_manpower, resource_node_munitions, resource_node_fuel, at_gun, outpost`（与 GD §12.4 清单一致；Garrison 沿用 M1 的 Deploy/放置消息）。

## 不做什么
- 消息的服务器处理与校验 → M2-010/M2-018/M2-019（规则在 sim，处理器在 server）；速率限制在 M2-019。
- 客户端 UI → M2-037/M2-039/M2-043；语音相关消息不新增（走 LiveKit，ARCH §14）。
- Commander 相关消息（`CommanderOrder` 已在 M1 草案）的新能力 → M3。

## 验收标准
1. `cargo test -p protocol` 中新增消息往返测试 100% 通过：每个新变体至少 1 个正常样例 + 2 个边界样例（最大字符串长度、最大 id）。
2. `proptest`：对每个新消息 10 000 个随机样例，`decode(encode(x)) == x`，量化误差在文档给定上界内。
3. `cargo fuzz run m2_messages -- -max_total_time=300`（5 min）无 panic、无 OOM、无越界。
4. `PROTOCOL_VERSION` 比 M1 末值大 1；新旧版本互握手被拒（`Reject` 版本原因）的测试通过。
5. 解码路径 `unsafe` 数量 = 0（`rg unsafe crates/protocol/src` 输出为空）。
6. `BuildPlace`/`PingPlace` 的编码长度：分别 ≤ 8 B 与 ≤ 8 B（测试断言）。

## 测试要求
- 单元测试：上述往返；非法 `kind`、超长字符串、截断缓冲的错误路径。
- 集成/无头：headless-client 连接 M1 server，发送新消息被丢弃或忽略而不崩溃（server 处理器未实现时回 `Reject`/忽略并计数）。
- 性能基准：`cargo bench -p protocol m2_encode`，记录每条编码耗时（预期 µs 级，无阈值）。
- 回归：协议版本快照测试更新。

## 性能预算
不得使 ARCH §6.9 的带宽推导回归：新增消息均为事件驱动，`StructureState`/`PingMarker` 平均每客户端 ≤ 2 条/s（计入"可靠事件 300 B/s 均值"内，ARCH §6.9.2）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 字段位宽为"草案细化"，落地后在 PLAN_ISSUES 记一条供 Fable 回写 ARCH §6.8。
- 若后续任务需新字段，**只能追加到末尾**，不得重排。M2-018/M2-019 若发现缺字段，在本任务备注追加说明并升版本。
