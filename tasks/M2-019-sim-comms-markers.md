# M2-019 sim：标记、Ping 与聊天频道规则

## 元数据
- 里程碑：M2
- 工作流：sim
- 规模：M
- 依赖：M1-021, M2-007
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：GAMEDESIGN §16; GAMEDESIGN §16.2; GAMEDESIGN §16.3; GAMEDESIGN §16.4; GAMEDESIGN §17.1; GAMEDESIGN §17.2; ARCHITECTURE §19.3; ARCHITECTURE §6.8

## 背景
GD §16 规定"谁能听到谁、谁能放什么标记"；**权限校验在服务器**（客户端只做提示）。M1-010/M1-026 已有基础 Chat/Marker 消息与速率限制；M2-007 加入了 `PingPlace`、`PingMarker`。本任务落实权限矩阵、Ping 系统与频道规则。
**标记矩阵（GD §16.3，数量/生命周期【待调参】）**：Ping（Attention）所有玩家→小队可见，8 s，每人 ≤ 3 同时、间隔 ≥ 2 s；Ping Enemy 小队可见，若放置者为 Spotter/Officer 同时对全队可见，6 s；Need Medic/Ammo/Engineer/AT/Supplies：对应兵种所有友军地图闪烁 + 小队长可见，20 s，每人每类 ≤ 1；Squad Order（Move/Attack/Defend/Hold/Build）仅小队长，小队可见，至替换或 180 s，每类 1、共 ≤ 5；Team Mark（敌步兵/载具/火炮/AT 炮/Garrison）仅 Spotter/Officer/Commander，全队，Spotter ≤ 10、Officer ≤ 5、Commander ≤ 20；Command Order（Attack/Defend）仅 Commander ≤ 12；Fire Request：Officer ≤ 1、Commander ≤ 3，90 s；Garrison Request：Officer/Commander，≤ 3，120 s；Supply Request：Engineer/Officer/Commander，≤ 3，120 s；Fire Mission/Rally Marker 为 M3 条目。
**Ping 系统（§16.4）**：8 类圆盘（Attention / Enemy / Need Medic / Need Ammo / Need Engineer / Need AT / Need Supplies / Thanks）；自动识别：准星指向敌方玩家/载具/建筑 → Enemy；指向友军 → Thanks，倒地者自动 Need Medic；反滥用：被同一小队成员静音超过 3 次的玩家，Ping 对该小队不可见 10 min。
**Spotter 标记显示时长（§17.1）**：步兵 8 s、载具 15 s、火炮/AT 炮/建筑 60 s；敌方标记下发只给**地图图标**（1 Hz、位置量化到 5 m，R3）。
**聊天（§16.2）**：All/Team/Squad/Command 四频道；Command 仅 Commander + Officer；每条 ≤ 200 字符；GD 限"每人每 8 s 最多 5 条"，而 ARCH §19.3 限"2 条/s、10 条/10 s、重复内容检测"——**两者取更严者同时生效**（冲突登记 PLAN_ISSUES）；系统消息走独立队列；服务器聊天保留 24 h（管理员，GD）/ 与举报关联前后 20 条保留 14 天（ARCH §19.4）。

## 具体内容
### 要创建/修改的文件
- `crates/sim/src/comms/markers.rs` —— `MarkerRegistry`：放置/过期/配额/可见集合；`can_place(player, kind) -> Result<(),MarkerErr>`
- `crates/sim/src/comms/ping.rs` —— 自动识别（射线 + 目标类型）、冷却、同时上限、静音反滥用
- `crates/sim/src/comms/chat.rs` —— 频道权限、长度/字符规则（NFKC、拒绝控制/双向覆盖字符，ARCH §19.1）
- `crates/server/src/c0/comms.rs` —— 处理 C0 `Chat/MarkerPlace/PingPlace`：速率限制（令牌桶）、转发 sim、广播（C0 Chat、C2.events `PingMarker/Marker`）
- `data/comms.json` —— 配额/生命周期/冷却
### 要实现的行为
1. `MarkerPlace`/`PingPlace` 权限与配额校验；超限返回 `Notice` 码（i18n key）。
2. 可见性：`visible_to(marker, player)`（小队/兵种/全队）；敌方标记位置量化到 5 m、同标记 ≥ 1 s 才刷新；过期事件广播。
3. Ping 自动识别：用 `WorldView.los` 与目标分类；识别结果写入 `target`。
4. 静音反滥用：记录"被同小队静音"计数（来自 M2-018 之外的客户端静音上报不在此任务；用服务器侧静音事件接口 `report_squad_mute(by,target)` 预留，M2-040 UI 接入）。
5. 聊天：频道成员计算、双限速（取严）、重复内容检测（同文 10 s 内 ≥ 3 次丢弃）、死亡/KIA 可见性按 GD §16.1 的 Dead 规则（文字聊天不受限）。
### 接口与数据
- `MarkerKind` 与 M2-007 的枚举一致（只读使用）；`MarkerRegistry::tick(now) -> Vec<MarkerExpired>`。

## 不做什么
- 语音频道与订阅 → M2-024/M2-025；语音 UI/聊天 UI → M2-040；径向菜单 → M2-037。
- Fire Mission/Rally/Commander 面板 → M3；敏感词表（需 NEEDS_HUMAN H-075，默认仅限速与重复检测）。
- AOI 下发策略本身 → M1-024（本任务只提供 `visible_to`）。

## 验收标准
1. 权限矩阵单测：枚举 `(角色 × 标记类型)` 全组合（≥ 14 角色 × 12 类型），期望表来自 GD §16.3，通过率 100%。
2. Ping 速率限制：同一玩家 1.9 s 内第 2 次被拒、2.0 s 通过；同时 ≤ 3 个；Need* 每类 ≤ 1；以 tick 计 ±1。
3. 生命周期：Ping 8 s（240 tick）、Enemy 6 s、Need* 20 s、Squad Order 180 s、Spotter 标记 步兵 8 s/载具 15 s/建筑 60 s，到期事件 ±1 tick。
4. 敌方标记下发位置量化：任意位置与下发值距离 ≤ 2.5√2 m，且同一标记下发间隔 ≥ 1 s（测试断言）。
5. 聊天：201 字符被拒；2 条/s 超限被丢；10 s 内第 11 条被丢；Command 频道非 Officer/Commander 发送被拒；控制字符/双向覆盖字符被过滤。
6. 反滥用：同小队 4 名成员静音某玩家后其 Ping 对该小队不可见 10 min（18 000 tick），他队可见。
7. 本模块 CPU ≤ 0.05 ms/tick（200 活动标记、100 玩家）。

## 测试要求
- 单元测试：矩阵、配额、窗口边界；proptest：任意放置序列配额不越界。
- 集成/无头：headless 10 客户端发送 Ping/Chat，另一小队与对方队伍收到/不收到。
- 性能基准：`comms_tick`。
- 回归：协议往返由 M2-007 覆盖。

## 性能预算
ARCH §6.10 阶段 5 内 ≤ 0.05 ms/tick；C0 速率限制不得放宽 ARCH §19.3；团队通道敌方标记 ≈ 0.4 kbps（ARCH §6.9.2）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（`cargo test` / `vitest` / CI 作业名）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 聊天限速 GD 与 ARCH 冲突（8 s/5 条 vs 2/s、10/10 s）→ PLAN_ISSUES；本任务取严。
- Spotter/Commander 在 M2 无席位：矩阵测试覆盖，但运行时无人可用。
- "Fire Request→Fire Mission 被 Commander 采纳"属 M3。
