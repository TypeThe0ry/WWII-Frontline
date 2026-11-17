# M4-040 测试：三图视觉回归与四档画质

## 元数据
- 里程碑：M4
- 工作流：test
- 规模：M
- 依赖：M3-048（M3 视觉回归扩充）, M4-014, M4-015, M4-017
- 素材需求：无（灰盒即可）；素材状态：n.a.
- 引用文档：ARCHITECTURE §17.4; ARCHITECTURE §17.5; ARCHITECTURE §13.1; UI_SPEC §6.5

## 背景
ARCHITECTURE §17.4：Playwright + Chromium；CI 无 GPU 时使用 SwiftShader/软件 WebGPU（Dawn）或固定 WebGL2 路径，**两条路径分别维护基线**；固定机位：**每张地图 12 个机位 × 3 时段 × 4 画质档（子集）**；固定随机种子、关闭 TAA 抖动（或固定相位）、固定时间；像素比较 `pixelmatch`，阈值 **0.1% 差异像素 + 感知指标**，差异产出报告；基线按"运行器类型"分开存储；跨 GPU 厂商差异用夜间 job（若有自托管 GPU runner）。
M4 的范围：把 Omaha 与 Foy 并入（M2-050/M3-048 已有 Carentan 与 M3 扩充），并为 **四档画质（Low/Medium/High/Ultra）** 建基线子集。时段：Carentan 白天/黄昏/阴天（以现有 M2-050 为准）；Omaha 清晨 06:30（MAPS §4.5）与上午 08:30（可选）；Foy 黎明 08:00、正午、黄昏 16:30（MAPS §5.5）。
"四档子集"含义：不是 12 × 3 × 4 全组合，而是子集——建议：每图每时段在 4 档各取 3 个代表机位（共 3×3×4 = 36 张/图）+ 其余机位仅 High 档（共 9×3 = 27 张/图），合计约 63 张/图；总量控制使 PR 作业 < 20 min（§17.5 `e2e-visual`）。
UI 视觉回归（M4-026 矩阵）并入同一框架但独立 spec，本任务不重复。

## 具体内容
### 要创建/修改的文件
- `tests/e2e/camera_poses.json` —— 补全 Omaha、Foy 各 12 机位（位置、朝向、FOV、时段 id、天气预设 id），沿用既有 schema（单一来源，M4-021 也用）。
- `tests/e2e/visual/m4_maps.spec.ts` —— 三图 × 时段 × 档位子集的截图与比较。
- `tests/e2e/visual/baselines/{swiftshader-webgpu,software-webgl2,gpu-nightly}/…` —— 基线（Git LFS 或对象存储清单 + 哈希；大图不入主 git 的方式按 M2-050 约定）。
- `tests/e2e/visual/subset.json` —— 档位子集定义。
- `.github/workflows/e2e-visual.yml`（更新）：PR 触发（改 `client/src/render`、`data/render`、`data/maps` 时）+ nightly 全量；差异报告 artifact。
- `tools/visual/report.ts` —— 汇总差异（像素比、SSIM/感知指标、热图）。
### 要实现的行为
1. 机位设计：Omaha 12 机位覆盖海滩全景、障碍带近景、海堤、D-1 出口、悬崖冲沟、高原果园、WN 工事、US 登陆艇视角、天空/水面反射机位（SSR 开关在 Ultra 档）、远景海平线；Foy 12 机位覆盖林缘散兵坑、牧场长视线、村庄街道、N30 公路、森林内部、降雪/雾预设、冬季天空。
2. 固定化：固定种子、固定相位 TAA（Halton 相位固定）、固定时间/天气、关闭自动曝光抖动（或固定曝光）、固定粒子种子；水面与降雪使用确定性时间（`t = fixed`）。
3. 比较：`pixelmatch` 阈值 0.1% 差异像素（对 1080p）+ 感知指标（SSIM ≥ 0.98）；失败时输出 diff 图与报告。
4. 基线更新流程：`pnpm -C tests update-baselines --map … --reason "…"`，PR 需人工批准（CODEOWNERS）；基线更新必须附原因。
5. 夜间 job：全量 12×3×4（若有自托管 GPU runner，分厂商基线；无则软件路径）。
6. 与 M4-021 共用机位序列，产生"首次变体"数据（可选）。
### 接口与数据
- `camera_poses.json` 条目：`{"id":"omaha_beach_01","map":"omaha_dog_green","pos":[x,y,z],"yaw":..,"pitch":..,"fov":..,"time":"0630","weather":"overcast"}`。

## 不做什么
- 不做 UI 视觉回归（M4-026）；不做性能测量（M4-041）；不做浏览器兼容矩阵（M4-042）。
- 不修复发现的视觉缺陷（登记 PLAN_ISSUES 并交由对应任务）。
- 不使用真实 GPU 厂商矩阵作为 PR 门禁（只夜间）。

## 验收标准
1. **基线入库**：三图 × 12 机位 × 3 时段 × 四档子集（按 `subset.json`）的基线齐全；`subset.json` 列出子集规模，脚本断言缺失基线数 = 0。
2. **CI 差异阈值生效**：故意改动一个材质参数的 PR 在 `e2e-visual` 上失败（差异像素 > 0.1%），回退后通过；记录一次实际 CI 运行链接/编号。
3. PR 作业总墙钟 < 20 min（§17.5；`e2e-visual`）；夜间全量 job 完成并上传报告 artifact。
4. 两条路径（SwiftShader WebGPU、软件 WebGL2）各自基线；跨路径不比较。
5. 稳定性：同提交连续 5 次运行，差异像素 > 0.1% 的"假阳性"机位数 = 0。
6. 基线更新流程文档化在 `tests/e2e/README.md`（执行者产物）并包含审批要求。

## 测试要求
- 单元测试：`vitest tests/visual-tools`：`pixelmatch` 包装、SSIM 计算、子集解析。
- 集成/无头测试：本任务本身即 e2e；包含冒烟 10 机位 3 min 版（本地快速运行）。
- 性能基准：作业时长记录。
- 回归：基线哈希清单 `baselines/manifest.json` 进 git。

## 性能预算
- CI 作业时长：`e2e-visual` < 20 min（§17.5）。

## 完成定义
- [ ] 全部验收标准有实测数字，已写入 `tasks/STATUS.md`
- [ ] 测试通过（Playwright、`vitest`）；`cargo clippy -D warnings`、`eslint`、`tsc --noEmit` 干净
- [ ] 性能预算已测（或明确标注"参考机不可用，partial"）
- [ ] 无新增禁用依赖（ARCHITECTURE §4.5）；`/data` 校验通过
- [ ] 占位素材已按 ASSET_GAPS §3 命名（`PH_{需求ID}`）；缺失素材需求 ID 已在回报中列出
- [ ] 文档同步：若任务产生了接口/数据变更，已更新对应 docs 的"变更说明"并在 PLAN_ISSUES 登记（不改章节编号）
- [ ] `tasks/STATUS.md` 对应行与顶部摘要已更新；提交信息含任务 ID

## 备注/已知风险
- 灰盒阶段的基线在真素材接入（M4-029）后必然大面积失效，需要在 M4-029 后一次性刷新并记录原因（建议在 STATUS 注明"基线版本 = 灰盒/真素材"）。
- 软件渲染与真 GPU 结果有差异（ARCHITECTURE §17.4），本任务不保证跨 GPU 厂商一致。
- INDEX 与 ARCHITECTURE 的"12 机位×3 时段×4 档"是子集定义的上限，具体子集由本任务确定并记录，避免作业超时。
