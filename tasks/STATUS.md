# 任务状态表（tasks/STATUS.md）

> 执行者每完成/推进/阻塞一个任务，必须更新本表对应行与顶部摘要，并随该任务的提交一起提交。
> 状态枚举：`todo` / `in-progress` / `done` / `partial` / `blocked`（定义见 `tasks/README.md` §4）。
> 任务清单与 `tasks/INDEX.md` 一致；新增/拆分任务须**先改 INDEX 再改本表**。

## 摘要

- **当前里程碑**：M0 技术验证
- **下一个任务**：`M0-001` repo-scaffold（依赖已满足、编号最小的 todo 任务）
- **blocked 清单**：（无）
  - 格式：`任务 ID — 所缺条目 — 登记日期 — 谁来解除（人类/素材/其它任务）`
- **partial 清单**：（无）
  - 格式：`任务 ID — 已完成部分 — 遗留项 — 对应 PLAN_ISSUES 编号`
- **最近更新**：（执行者填写：日期 / 任务 ID）

## 列说明

- **实测结果**：验收标准要求的**实测数字**（如"tick_ms P99 = 21.4 ms，100 客户端×30 min"），附测量命令或报告路径；不得只写"通过"。
- **遗留问题**：未解决的偏差/风险，写 `docs/PLAN_ISSUES.md` 条目编号；无则留空。
- **完成提交**：完成该任务的 commit 短哈希（多个提交用逗号分隔）。

## M0 技术验证（M0-001 – M0-033）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M0-001 | 仓库脚手架与工具链 | todo | | | |
| M0-002 | GitHub Actions 基线与依赖图检查 | todo | | | |
| M0-003 | 字体许可证核实与登记（R10） | todo | | | |
| M0-004 | /data schema 与校验工具 | todo | | | |
| M0-005 | protocol：位流、量化与 M0 消息集 | todo | | | |
| M0-006 | protocol：快照编码与基线增量 | todo | | | |
| M0-007 | sim：World/step/确定性底座 | todo | | | |
| M0-008 | 数据：M0 步枪（Kar98k、M1 Garand） | todo | | | |
| M0-009 | 地图：M0 单块地形（256 m）数据管线 | todo | | | |
| M0-010 | sim：高度场碰撞 | todo | | | |
| M0-011 | sim：Rapier 角色控制器与基础移动 | todo | | | |
| M0-012 | sim：步枪射击、弹道与装填 | todo | | | |
| M0-013 | sim：生命、死亡与简易重生 | todo | | | |
| M0-014 | sim-wasm：WASM 封装 | todo | | | |
| M0-015 | server：进程骨架与 30 Hz 主循环 | todo | | | |
| M0-016 | server：WebSocket 传输与握手 | todo | | | |
| M0-017 | server：WebTransport 传输 | todo | | | |
| M0-018 | server：会话、输入缓冲与校验 | todo | | | |
| M0-019 | server：AOI v0 与快照发送 | todo | | | |
| M0-020 | client/render：引擎初始化与调试叠层 | todo | | | |
| M0-021 | client/net：传输与时钟同步 | todo | | | |
| M0-022 | client/game：输入、预测、对账与插值 | todo | | | |
| M0-023 | client/render：CDLOD 地形 | todo | | | |
| M0-024 | client/render：植被实例化与 impostor | todo | | | |
| M0-025 | client/render：灰盒角色、第一人称枪与射击反馈 | todo | | | |
| M0-026 | bots：M0 基础 bot | todo | | | |
| M0-027 | headless-client v0 与 10 客户端联测 | todo | | | |
| M0-028 | 测试：浏览器 WT/WS 互通矩阵 | todo | | | |
| M0-029 | 测试：确定性与 WASM 漂移 | todo | | | |
| M0-030 | 测试：M0 性能基线与报告 | todo | | | |
| M0-031 | 史实参数复核：Kar98k 与 M1 Garand（R8） | todo | | | |
| M0-032 | 对标测量：M0 批回填 | todo | | | |
| M0-033 | M0 里程碑验收与 STATUS 汇总 | todo | | | |

## M1 垂直切片（M1-001 – M1-072）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M1-001 | 素材管线：ingest 与许可证闸门 | todo | | | |
| M1-002 | 素材管线：网格 LOD、纹理与碰撞简化 | todo | | | |
| M1-003 | 素材管线：.fpak 分块容器（D-PAK） | todo | | | |
| M1-004 | 素材管线：音频处理与打包 | todo | | | |
| M1-005 | 素材管线：manifest 与体积预算门禁 | todo | | | |
| M1-006 | client：fpak 加载、解码 Worker 与缓存 | todo | | | |
| M1-007 | 数据：M1 五兵种、loadout 与基础规则 | todo | | | |
| M1-008 | 数据：M1 武器与投掷物 | todo | | | |
| M1-009 | 数据：M4A1 Sherman（M1 的 1 辆坦克） | todo | | | |
| M1-010 | protocol：M1 全消息集 | todo | | | |
| M1-011 | sim：完整移动（姿态、体力、翻越、探身） | todo | | | |
| M1-012 | sim：弹道细化与穿透 | todo | | | |
| M1-013 | sim：冲锋枪、卡宾、手枪与近战 | todo | | | |
| M1-014 | sim：命中区与伤害模型 | todo | | | |
| M1-015 | sim：手雷与烟雾 | todo | | | |
| M1-016 | sim：倒地、救起与击杀归属 | todo | | | |
| M1-017 | sim：压制 | todo | | | |
| M1-018 | sim：据点占领与对局状态机 | todo | | | |
| M1-019 | sim：HQ 出生与 Garrison | todo | | | |
| M1-020 | sim：Supplies 与补给箱 | todo | | | |
| M1-021 | sim：小队、兵种名额与部署校验 | todo | | | |
| M1-022 | sim：载具物理（悬挂、履带、座位） | todo | | | |
| M1-023 | sim：载具武器与装甲判定 | todo | | | |
| M1-024 | server：AOI v1（敌友规则、团队通道） | todo | | | |
| M1-025 | server：延迟补偿 | todo | | | |
| M1-026 | server：部署/小队处理与可靠事件 | todo | | | |
| M1-027 | server：基础校验（速度/射速/视角） | todo | | | |
| M1-028 | gateway：服务器列表、游客鉴权与 ticket | todo | | | |
| M1-029 | server：注册、心跳与 ticket 验证 | todo | | | |
| M1-030 | 地图：Carentan 数据源许可证与锚点对账 | todo | | | |
| M1-031 | 地图：Carentan 1/3 区域高度图 | todo | | | |
| M1-032 | 地图：OSM 矢量提取与清洗 | todo | | | |
| M1-033 | 地图：1944 修正（M1 区域） | todo | | | |
| M1-034 | 地图：Carentan 1/3 关卡标注与建筑足迹 | todo | | | |
| M1-035 | 地图：区块烘焙、splat 与植被分布 | todo | | | |
| M1-036 | 地图：Recast 导航网格烘焙 | todo | | | |
| M1-037 | bots：navmesh 寻路与避障 | todo | | | |
| M1-038 | bots：行为树与效用 AI | todo | | | |
| M1-039 | bots：小队长 bot、兵种选择与填充 | todo | | | |
| M1-040 | client/game：游戏状态与事件总线类型 | todo | | | |
| M1-041 | client/net：载具驾驶预测与同步 | todo | | | |
| M1-042 | render：PBR 材质系统与变体管理 | todo | | | |
| M1-043 | render：光照、CSM 阴影与天空 | todo | | | |
| M1-044 | render：TAA、曝光与色调映射 | todo | | | |
| M1-045 | render：骨骼角色与动画状态机 | todo | | | |
| M1-046 | render：第一人称武器 pass | todo | | | |
| M1-047 | render：粒子与贴花基础 | todo | | | |
| M1-048 | render：坦克渲染 | todo | | | |
| M1-049 | world：区块流式加载 | todo | | | |
| M1-050 | world：地形 splat 材质与宏观色 | todo | | | |
| M1-051 | world：树篱、植被与道具 | todo | | | |
| M1-052 | world：建筑与可进入室内 | todo | | | |
| M1-053 | UI：框架、设计 token 与 i18n | todo | | | |
| M1-054 | UI：启动加载与主菜单 | todo | | | |
| M1-055 | UI：服务器浏览器与阵营选择 | todo | | | |
| M1-056 | UI：部署界面 | todo | | | |
| M1-057 | UI：核心 HUD | todo | | | |
| M1-058 | UI：战术地图 | todo | | | |
| M1-059 | UI：倒地与阵亡界面 | todo | | | |
| M1-060 | UI：Officer/Support 放置与载具进出 | todo | | | |
| M1-061 | UI：暂停菜单与基础设置 | todo | | | |
| M1-062 | 音频：引擎核心 | todo | | | |
| M1-063 | 音频：枪声分层、脚步与命中 | todo | | | |
| M1-064 | 素材接入：M1 视觉素材 | todo | | | |
| M1-065 | 素材接入：M1 音频与字体 | todo | | | |
| M1-066 | 运维：本地开发栈 | todo | | | |
| M1-067 | 运维：预发布部署 | todo | | | |
| M1-068 | 测试：20 客户端联测 | todo | | | |
| M1-069 | 测试：M1 性能基准 | todo | | | |
| M1-070 | 史实参数复核：M1 武器与 Sherman（R8） | todo | | | |
| M1-071 | 对标测量：M1 批回填与我方复测 | todo | | | |
| M1-072 | M1 里程碑验收与 STATUS 汇总 | todo | | | |

## M2 完整 Frontline 模式（M2-001 – M2-055）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M2-001 | 地图：Carentan 全图高度图与矢量 | todo | | | |
| M2-002 | 地图：Carentan 1944 修正（全图） | todo | | | |
| M2-003 | 地图：建筑样式与地标（全图） | todo | | | |
| M2-004 | 地图：Carentan Frontline 关卡标注 | todo | | | |
| M2-005 | 地图：全图区块、碰撞与导航烘焙 | todo | | | |
| M2-006 | 数据：M2 全步兵兵种、武器与基础载具 | todo | | | |
| M2-007 | protocol：M2 消息（建造、资源、投票、Ping） | todo | | | |
| M2-008 | sim：Frontline 模式规则 | todo | | | |
| M2-009 | sim：战略资源与资源节点 | todo | | | |
| M2-010 | sim：Engineer 建造系统 | todo | | | |
| M2-011 | sim：Outpost、维修站与反坦克炮 | todo | | | |
| M2-012 | sim：M2 兵种特有能力 | todo | | | |
| M2-013 | sim：M2 武器类（自动步枪、机枪、霰弹） | todo | | | |
| M2-014 | sim：火箭、地雷与炸药 | todo | | | |
| M2-015 | sim：M2 载具（轻型车、卡车、Panzer IV）与刷新 | todo | | | |
| M2-016 | sim：载具模块损伤与维修 | todo | | | |
| M2-017 | sim：计分与经验 | todo | | | |
| M2-018 | sim：自动平衡、TK、投票与挂机 | todo | | | |
| M2-019 | sim：标记、Ping 与聊天频道规则 | todo | | | |
| M2-020 | server：带宽调节器 | todo | | | |
| M2-021 | server：100 玩家 CPU 优化 | todo | | | |
| M2-022 | gateway：战绩、设置与可选 OAuth | todo | | | |
| M2-023 | gateway：管理 API 与举报 | todo | | | |
| M2-024 | 语音：LiveKit 自托管与 token | todo | | | |
| M2-025 | 语音：服务器订阅集管理 | todo | | | |
| M2-026 | 语音：客户端接入与空间化 | todo | | | |
| M2-027 | bots：Frontline 目标与战术行为 | todo | | | |
| M2-028 | bots：规则式指挥官 | todo | | | |
| M2-029 | bots：100 bot 规模优化 | todo | | | |
| M2-030 | render：GTAO/SSAO | todo | | | |
| M2-031 | render：体积雾 | todo | | | |
| M2-032 | render：TAAU 时间性超分 | todo | | | |
| M2-033 | render：烟雾、火焰、弹坑与血迹 | todo | | | |
| M2-034 | world：遮挡剔除 | todo | | | |
| M2-035 | world：水面与沼泽 | todo | | | |
| M2-036 | world：全图流式与内存预算 | todo | | | |
| M2-037 | UI：径向菜单与快捷指令 | todo | | | |
| M2-038 | UI：记分板 | todo | | | |
| M2-039 | UI：建造界面 | todo | | | |
| M2-040 | UI：文字聊天与语音状态 | todo | | | |
| M2-041 | UI：完整设置与可访问性基础 | todo | | | |
| M2-042 | UI：结算界面 | todo | | | |
| M2-043 | UI：投票、举报、换队与系统提示 | todo | | | |
| M2-044 | UI：战术地图的 Frontline 叠层 | todo | | | |
| M2-045 | 音频：遮挡低通与压制效果 | todo | | | |
| M2-046 | 音频：混响总线、环境与载具声 | todo | | | |
| M2-047 | 素材接入：M2 批 | todo | | | |
| M2-048 | 运维：监控与日志 | todo | | | |
| M2-049 | 测试：100 客户端压测（WT 与 WS） | todo | | | |
| M2-050 | 测试：视觉回归（Playwright） | todo | | | |
| M2-051 | 测试：balance-sim v0 | todo | | | |
| M2-052 | 测试：M2 性能基准 | todo | | | |
| M2-053 | 史实参数复核：M2 武器与载具（R8） | todo | | | |
| M2-054 | 对标测量：M2 批回填 | todo | | | |
| M2-055 | M2 里程碑验收与 STATUS 汇总 | todo | | | |

## M3 全兵种、全载具、Assault、指挥官能力、火炮（M3-001 – M3-054）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M3-001 | 数据：M3 全兵种补全与剩余武器 | todo | | | |
| M3-002 | 数据：M3 全部载具 | todo | | | |
| M3-003 | 数据：指挥官能力与火炮 | todo | | | |
| M3-004 | 地图：Carentan 炮兵阵地、载具路线与 Assault 测试配置 | todo | | | |
| M3-005 | protocol：M3 消息（能力、炮兵、标记、载具座位） | todo | | | |
| M3-006 | sim：指挥官席位、资源池与指令校验 | todo | | | |
| M3-007 | sim：Supply Drop 与 Ammo Drop | todo | | | |
| M3-008 | sim：Bombing/Strafing/Precision | todo | | | |
| M3-009 | sim：Recon Flight、Rally 与 Airhead | todo | | | |
| M3-010 | sim：Spotter 标记、OP 与暴露规则 | todo | | | |
| M3-011 | sim：狙击步枪与屏息 | todo | | | |
| M3-012 | sim：Commander/Tank Commander/Spotter/Sniper 特有规则 | todo | | | |
| M3-013 | sim：全弹种穿深与装甲判定 | todo | | | |
| M3-014 | sim：轻型/侦察载具 | todo | | | |
| M3-015 | sim：中/重型坦克 | todo | | | |
| M3-016 | sim：运输载具座位与乘员射击 | todo | | | |
| M3-017 | sim：载具呼叫与认领 | todo | | | |
| M3-018 | sim：榴弹炮阵地 | todo | | | |
| M3-019 | sim：炮组操作与反坦克炮 | todo | | | |
| M3-020 | sim：Assault 模式规则 | todo | | | |
| M3-021 | sim：建筑破坏阶段与动态导航障碍 | todo | | | |
| M3-022 | server：载具延迟补偿与 AOI 优先级 | todo | | | |
| M3-023 | bots：载具驾驶与乘员 bot | todo | | | |
| M3-024 | bots：Spotter 与 Sniper bot | todo | | | |
| M3-025 | bots：炮组与车长 bot | todo | | | |
| M3-026 | bots：规则指挥官使用全部能力 | todo | | | |
| M3-027 | bots：LLM 指挥官（可选 feature） | todo | | | |
| M3-028 | bots：Assault 模式 bot 行为 | todo | | | |
| M3-029 | render：全部载具外观与交互 | todo | | | |
| M3-030 | render：空袭飞机与落弹表现 | todo | | | |
| M3-031 | render：炮击落点与战场环境表现 | todo | | | |
| M3-032 | render：载具击毁与残骸 | todo | | | |
| M3-033 | UI：指挥官界面 | todo | | | |
| M3-034 | UI：Spotter 与 OP 工具 | todo | | | |
| M3-035 | UI：载具与火炮操作界面 | todo | | | |
| M3-036 | UI：载具 HUD 细节与 Assault HUD | todo | | | |
| M3-037 | 音频：全部载具与重武器声 | todo | | | |
| M3-038 | 音频：爆炸、炮击与空袭 | todo | | | |
| M3-039 | 音频：战场人声与无线电 | todo | | | |
| M3-040 | 语音：指挥与车组频道 | todo | | | |
| M3-041 | 素材接入：M3 载具 | todo | | | |
| M3-042 | 素材接入：M3 武器与角色 | todo | | | |
| M3-043 | 素材接入：M3 音频 | todo | | | |
| M3-044 | 素材接入：飞机与火炮模型 | todo | | | |
| M3-045 | 测试：M3 全功能 100 客户端压测 | todo | | | |
| M3-046 | 测试：M3 平衡模拟 | todo | | | |
| M3-047 | 数据：M3 首轮数值调参 | todo | | | |
| M3-048 | 测试：M3 视觉回归扩充 | todo | | | |
| M3-049 | 测试：M3 性能基准 | todo | | | |
| M3-050 | 运维：夜间作业 | todo | | | |
| M3-051 | 史实参数复核：全部载具与剩余武器（R8） | todo | | | |
| M3-052 | 对标测量：M3 批回填 | todo | | | |
| M3-053 | UI：简体中文文案与 CJK 字体完整性 | todo | | | |
| M3-054 | M3 里程碑验收与 STATUS 汇总 | todo | | | |

## M4 Omaha Beach、Foy、Skirmish、画质与优化、公开测试（M4-001 – M4-046）

| ID | 标题 | 状态 | 实测结果 | 遗留问题 | 完成提交 |
|---|---|---|---|---|---|
| M4-001 | 地图：Omaha 数据源、许可证与锚点对账 | todo | | | |
| M4-002 | 地图：Omaha 高度图、海岸与矢量 | todo | | | |
| M4-003 | 地图：滩头障碍与防御工事摆放 | todo | | | |
| M4-004 | 地图：Omaha Assault 关卡标注 | todo | | | |
| M4-005 | 地图：Omaha 区块、碰撞与导航烘焙 | todo | | | |
| M4-006 | 地图：Foy 数据源许可证与锚点对账 | todo | | | |
| M4-007 | 地图：Foy 高度图与矢量 | todo | | | |
| M4-008 | 地图：Foy 森林与散兵坑布置 | todo | | | |
| M4-009 | 地图：Foy 关卡标注（Frontline 与 Assault） | todo | | | |
| M4-010 | 地图：Foy 区块、碰撞与导航烘焙 | todo | | | |
| M4-011 | sim：登陆艇出生与滩头规则 | todo | | | |
| M4-012 | sim：Skirmish 模式规则 | todo | | | |
| M4-013 | 地图：Skirmish 子区配置 | todo | | | |
| M4-014 | world：海洋、海岸与滩头 | todo | | | |
| M4-015 | world：冬季雪地与天气 | todo | | | |
| M4-016 | bots：新地图与新模式 bot 适配 | todo | | | |
| M4-017 | render：四档画质预设 | todo | | | |
| M4-018 | render：动态分辨率与自动推荐 | todo | | | |
| M4-019 | render：Ultra 屏幕空间反射 | todo | | | |
| M4-020 | render：Low 档与 WebGL2 路径优化 | todo | | | |
| M4-021 | render：着色器变体与预热门禁 | todo | | | |
| M4-022 | render：显存与纹理 mip 预算 | todo | | | |
| M4-023 | UI：模式/地图选择与加载界面更新 | todo | | | |
| M4-024 | UI：画质设置与基准向导 | todo | | | |
| M4-025 | UI：可访问性审计与修复 | todo | | | |
| M4-026 | UI：缩放、安全区与超宽屏测试 | todo | | | |
| M4-027 | UI：新手引导 | todo | | | |
| M4-028 | 音频：分地图混响与环境调校 | todo | | | |
| M4-029 | 素材接入：M4 Omaha/Foy | todo | | | |
| M4-030 | server：统计异常检测与回放 | todo | | | |
| M4-031 | gateway：对局编排与匹配 | todo | | | |
| M4-032 | 安全：加固与审计 | todo | | | |
| M4-033 | 运维：生产部署 | todo | | | |
| M4-034 | 运维：告警、运行手册与发布回滚 | todo | | | |
| M4-035 | 运维：隐私与合规（数据导出/删除） | todo | | | |
| M4-036 | 运维：匿名遥测（首次同意） | todo | | | |
| M4-037 | 运维：公开测试准备 | todo | | | |
| M4-038 | 测试：三图 60 分钟浸泡压测 | todo | | | |
| M4-039 | 测试：三图三模式平衡模拟 | todo | | | |
| M4-040 | 测试：三图视觉回归与四档画质 | todo | | | |
| M4-041 | 测试：四档画质性能矩阵 | todo | | | |
| M4-042 | 测试：浏览器兼容矩阵 | todo | | | |
| M4-043 | 史实参数最终清扫（R8） | todo | | | |
| M4-044 | 对标测量：复测与版本漂移处理 | todo | | | |
| M4-045 | 测试：封闭测试执行与反馈汇总 | todo | | | |
| M4-046 | M4 里程碑验收与 STATUS 汇总 | todo | | | |
