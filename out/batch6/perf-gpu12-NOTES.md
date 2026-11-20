# B6-perf-gpu12 · M1-069 / E1-6 / E1-7（2026-11-20）

- Carentan 占位世界：60 个演示实体，三机位各 3 次，10 s 预热 + 60 s 采集；同一张 RTX 3080 20 GiB（GPU1 / `0000:23:00.0` / 驱动 `595.58.03`），启动前每档各三次都为 0%。用 `unshare --mount --propagation private` 私有化设备视图，屏蔽了其余三张卡，Vulkan PCI 选择加 `pmon` 验证，Low 部分窗口出现外部 Chromium GPU 进程，数据如实保留；未改宿主或停止他人作业。原始 6 份 JSON 与实时负载 CSV、逐轮数据和运行命令见 `reports/m1-perf.{md,json}` / `reports/m1-perf-raw/`。
- Low：已有地形/植被档位，新增 `?scene=world&quality=low` 固定 75% 内部比例与 `--angle-vulkan` 硬件 WebGL2 采集选项。High 1920×1080/WebGPU；Low 1280×720/960×540、WebGL2；均保持旧基准缺省（无自写 CSM/TAA，不含完整 HUD），不做画质设计。画面、动态分辨率策略及设置 UI 待 Fable 设计；无新增素材。
- 本机均属 PI-109 工作站数据。RTX 3060 / Iris Xe、最终素材、分 pass GPU 和 wasm 内存未测；E1-6/E1-7 不作 PASS/FAIL，M1-069/M1-072 维持 partial。适配路径差异不能归为某一项优化的收益。
- High：P95 16.70–16.80 ms（3/9 轮高于 16.7 ms），>50 ms 卡顿 0–2 帧/轮，83–211 draw call / 347,532–444,075 三角形。GPU1 利用率 25–45%，216 个采样点未见其他 GPU1 进程。Low：59.88–60.02 rAF FPS，P95 16.70–16.80 ms，卡顿 0–1 帧/轮，71–181 draw call / 125,572–143,173 三角形。GPU1 利用率 2–57%，221 个点中 91 个见外部进程，不是全程独占。
- Xvfb + 仅 NVIDIA ICD 的试跑遇到 WSI / SharedImage 初始化问题，弃用该组合；正式用系统默认 ICD 搜索路径及真实硬件 Chromium Headless Shell，`/tmp/pg12` 内使用各自的临时 profile。
- 合并前本地门槛基线 `cd4ea51`。`FRONTLINE_PLAYWRIGHT_PORT=5227 pnpm -C client exec playwright test world.spec.ts` 3/3 通过（SwiftShader WebGL2），其中 Low 比例测试覆盖 High 960×540、Low 720×405，CSS 布局不变。`node tools/ci/check-acceptance.mjs` 0 错、0 未结任务（按决定跳过 M1-066）；对应验收/性能预算用例 11/11。
- `CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh lint ts determinism`：3 组通过，0 失败、0 跳过；lint 脚本测试 38，材质变体 8，客户端 99 文件 / 760 测试；native 确定性测试 4/4，3 seeds × 18,000 tick native/WASM 逐 tick 哈希和位置完全一致。
- 收尾删除本 worktree 的 `target`、`client/node_modules/.vite` 和专用 `/data/scratch/b6-perf-gpu12`、`/tmp/pg12`。按里程碑流程以 `/data/projects/WWII-Frontline/scripts/dev/land-ms.sh perf-gpu12` 落地；未用 GitHub Actions。
