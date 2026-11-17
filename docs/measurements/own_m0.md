# 我方（Frontline sim）批 M0 同法复测数值

由 `cargo test -p frontline-sim --features test-util --test own_remeasure`（`OWN_REMEASURE_OUT=docs/measurements/own_m0.json`）导出，tick 30 Hz，数据哈希见 JSON。
"不可测" = M0 sim 没有该功能（原因在备注）。这是**我方**数值，不是参考作品数值；参考作品的测量值仍全部"待测"。

| ID | 项目 | 我方实测中位数 | 单位 | 范围 [min, max] | n | /data 当前值 | 方法 | 备注 |
|---|---|---|---|---|---|---|---|---|
| W-01.us_m1_garand | 实际射速（长连射平均） | 120 | 发/分 | [120, 120] | 1 | 120 | tick log of max-rate clicker, 20 s, N-1 intervals over the span | 40 shots in 20 s; semi/bolt weapons are limited by the pacing interval, so this is the cap, not a cyclic rate |
| W-02.us_m1_garand | 半自动最快点射间隔 | 0.5 | 秒 | [0.5, 0.5] | 39 | 0.5 | tick log, intervals between consecutive shots of the fastest clicker | 30 Hz quantisation: 15 ticks |
| W-03.us_m1_garand | 空仓换弹时间 | 2.8 | 秒 | [2.8, 2.8] | 1 | 2.8 | ticks from reload press to reload_finished (empty magazine) | whole-clip reload in M0 |
| W-04.us_m1_garand | 战术换弹时间 | 3.4 | 秒 | [3.4, 3.4] | 1 | 3.4 | ticks from reload press to reload_finished (1 round fired) |  |
| W-05.us_m1_garand | 逐发装填每发时间与装满总时间 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | Per-round loading is M1-013; M0 reloads whole clips (data per_round_ms is not used yet) |
| W-06.us_m1_garand | 打断换弹的最早可开火时刻 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | Reload cannot be interrupted in M0 (weapon_fsm: reload_cannot_be_restarted_or_swapped_midway) |
| W-07.us_m1_garand | 开镜进入时间 | 0.3 | 秒 | [0.3, 0.3] | 1 | 0.3 | ticks from AIM press to ads_progress == 1 | sim state; the client camera/FOV blend is a presentation timing to record separately |
| W-08.us_m1_garand | 开镜退出时间 | 0 | 秒 | [0, 0] | 1 |  | AIM release resets ads progress | M0 sim releases ADS instantly (0 s); client camera blend is presentation |
| W-09.us_m1_garand | ADS 放大倍率/FOV | 不可测 | 度 |  | 0 |  | none (feature absent in M0 sim) | FOV is a client rendering value; the sim has none (measure with the client at M0-025) |
| W-11.us_m1_garand | 切枪时间：拔出 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | No weapon switching in M0 (single weapon, weapon_slot ignored) |
| W-12.us_m1_garand | 切枪时间：收起 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | No weapon switching in M0 (single weapon, weapon_slot ignored) |
| W-15.us_m1_garand | 后坐模式：竖直（前10发） | 1.60286 | 度/发 | [1.36068, 1.83976] | 1600 | 1.6 | view-pitch delta across each shot, 200 runs x 10 shots (hip, standing) | kick = vkick x U(0.85, 1.15), identical for every shot index (no pattern in M0) |
| W-16.us_m1_garand | 后坐模式：水平（前10发） | 0.167084 | 度/发 | [0.000260524, 0.974554] | 1600 | 0.25 | |view-yaw delta| across each shot, 200 runs x 10 shots | median of /yaw kick/; yaw kick is N(0, hsigma); signed series = per-index mean |
| W-17.us_m1_garand | 后坐恢复：停火后回正时间与方式 | 0.233333 | 秒 | [0.233333, 0.233333] | 1 | 0.253333 | ticks from the shot until the view recoil offset is 0 | automatic linear return after a 120 ms delay (approx., kick jitter +-15 %) |
| W-18.us_m1_garand | ADS 与腰射后坐倍率 | 1.00097 | 比值 | [1.00097, 1.00097] | 1 |  | mean ADS kick / mean hip kick, 2000 shots each | no ADS recoil factor in M0 (stance factor only) |
| W-19.us_m1_garand | 腰射扩散 | 2.45028 | 度 | [0.0197823, 3.49984] | 10000 | 3.5 | angular deviation of 10 000 hip shots from the aim line (median; P95 and max in p95/max) | uniform draw in a cone of half angle hip_deg; median ~ 0.71 x half angle |
| W-20.us_m1_garand | 开镜扩散（站/蹲/趴） | 0.104678 | 度 | [0, 0.149353] | 10000 | 0.15 | angular deviation of 10 000 ADS shots, standing | crouching ADS median 0.0839 deg; prone is M1-011 |
| W-21.us_m1_garand | 扩散随移动/姿态/连射的增长与恢复 | 2.49208 | 比值 | [2.49208, 2.49208] | 1 |  | median hip deviation while running (4.4 m/s) / standing still | walk (2 m/s) x1.496, crouch x0.798; bloom per shot 0.4 deg decays at 3 deg/s |
| W-22.us_m1_garand | 弹头速度（曳光飞行时间法，>= 200 m） | 773.168 | 米/秒 | [758.15, 788.185] | 2 | 853 | v = d / t_flight at 200 m and 300 m (average velocity over the flight, as read from a recording) | time of flight 200 m 0.2537 s, 300 m 0.3957 s; the average is below the muzzle velocity because of drag |
| W-23.us_m1_garand | 下坠：100 m 落点偏移 | 0.0763901 | 米 | [0.0763901, 0.0763901] | 1 |  | integrated projectile height below the bore line at that range |  |
| W-24.us_m1_garand | 下坠：200 m 落点偏移 | 0.311006 | 米 | [0.311006, 0.311006] | 1 |  | integrated projectile height below the bore line at that range |  |
| W-25.us_m1_garand | 下坠：300 m 落点偏移 | 0.729628 | 米 | [0.729628, 0.729628] | 1 |  | integrated projectile height below the bore line at that range | 400 m 1.362 m, 500 m 2.243 m; relative to the bore line, level shot |
| W-26.us_m1_garand | 弹头飞行形式（hitscan/弹丸） | 不可测 | 枚举 |  | 0 |  | code inspection / projectile spawn | Enumeration, no number: simulated projectile (semi-implicit Euler, 4 sub-steps/tick, drag k), not hitscan |
| W-27.us_m1_garand | 准星晃动幅度与频率（ADS 站立，无屏息） | 不可测 | 度 |  | 0 |  | none (feature absent in M0 sim) | Sway and breath holding are not implemented in M0 (no sway state in WeaponState) |
| W-28.us_m1_garand | 晃动随姿态变化 | 不可测 | 度 |  | 0 |  | none (feature absent in M0 sim) | Sway and breath holding are not implemented in M0 (no sway state in WeaponState) |
| W-29.us_m1_garand | 屏息：抑制比、持续、恢复 | 不可测 | 度 |  | 0 |  | none (feature absent in M0 sim) | Sway and breath holding are not implemented in M0 (no sway state in WeaponState) |
| W-30.us_m1_garand | 弹匣容量与携弹量 | 8 | 发 | [8, 8] | 1 | 8 | magazine rounds of a fresh weapon | spare rounds with 6 spare magazines: 48; data mag_reserve 6 |
| W-37.us_m1_garand | 伤害：各距离所需发数与部位倍率 | 2 | 发 | [2, 2] | 1 |  | ceil(health / damage(zone, distance)) from projectile::damage; torso at 100 m | torso 70.00 HP at 100 m, 62.16 HP at 300 m; see H-01 |
| H-01.us_m1_garand@50m | 致死发数（躯干） | 2 | 发 | [2, 2] | 1 |  | ceil(100 HP / damage(torso, d)) | damage 70.00 HP |
| H-01.us_m1_garand@150m | 致死发数（躯干） | 2 | 发 | [2, 2] | 1 |  | ceil(100 HP / damage(torso, d)) | damage 68.04 HP |
| H-01.us_m1_garand@300m | 致死发数（躯干） | 2 | 发 | [2, 2] | 1 |  | ceil(100 HP / damage(torso, d)) | damage 62.16 HP |
| H-02.us_m1_garand | 头部致死发数 | 1 | 发 | [1, 1] | 1 |  | ceil(100 HP / damage(head, 50 m)) | head multiplier 3 |
| H-03.us_m1_garand | 四肢致死发数/伤害倍率 | 3 | 发 | [3, 3] | 1 |  | ceil(100 HP / damage(limb, 50 m)) | limb multiplier 0.65 |
| W-01.ger_kar98k | 实际射速（长连射平均） | 40 | 发/分 | [40, 40] | 1 | 40 | tick log of max-rate clicker, 20 s, N-1 intervals over the span | 14 shots in 20 s; semi/bolt weapons are limited by the pacing interval, so this is the cap, not a cyclic rate |
| W-13.ger_kar98k | 栓动单次循环时间 | 1.5 | 秒 | [1.5, 1.5] | 13 | 1.5 | tick log, fire -> next fire at the earliest | 45 ticks @30 Hz |
| W-03.ger_kar98k | 空仓换弹时间 | 3 | 秒 | [3, 3] | 1 | 3 | ticks from reload press to reload_finished (empty magazine) | whole-clip reload in M0 |
| W-04.ger_kar98k | 战术换弹时间 | 3 | 秒 | [3, 3] | 1 | 3 | ticks from reload press to reload_finished (1 round fired) |  |
| W-05.ger_kar98k | 逐发装填每发时间与装满总时间 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | Per-round loading is M1-013; M0 reloads whole clips (data per_round_ms is not used yet) |
| W-06.ger_kar98k | 打断换弹的最早可开火时刻 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | Reload cannot be interrupted in M0 (weapon_fsm: reload_cannot_be_restarted_or_swapped_midway) |
| W-07.ger_kar98k | 开镜进入时间 | 0.3 | 秒 | [0.3, 0.3] | 1 | 0.3 | ticks from AIM press to ads_progress == 1 | sim state; the client camera/FOV blend is a presentation timing to record separately |
| W-08.ger_kar98k | 开镜退出时间 | 0 | 秒 | [0, 0] | 1 |  | AIM release resets ads progress | M0 sim releases ADS instantly (0 s); client camera blend is presentation |
| W-09.ger_kar98k | ADS 放大倍率/FOV | 不可测 | 度 |  | 0 |  | none (feature absent in M0 sim) | FOV is a client rendering value; the sim has none (measure with the client at M0-025) |
| W-11.ger_kar98k | 切枪时间：拔出 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | No weapon switching in M0 (single weapon, weapon_slot ignored) |
| W-12.ger_kar98k | 切枪时间：收起 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | No weapon switching in M0 (single weapon, weapon_slot ignored) |
| W-15.ger_kar98k | 后坐模式：竖直（前10发） | 3.19196 | 度/发 | [2.7203, 3.67917] | 1000 | 3.2 | view-pitch delta across each shot, 200 runs x 10 shots (hip, standing) | kick = vkick x U(0.85, 1.15), identical for every shot index (no pattern in M0) |
| W-16.ger_kar98k | 后坐模式：水平（前10发） | 0.138645 | 度/发 | [0.000413513, 0.704049] | 1000 | 0.2 | |view-yaw delta| across each shot, 200 runs x 10 shots | median of /yaw kick/; yaw kick is N(0, hsigma); signed series = per-index mean |
| W-17.ger_kar98k | 后坐恢复：停火后回正时间与方式 | 0.433333 | 秒 | [0.433333, 0.433333] | 1 | 0.44 | ticks from the shot until the view recoil offset is 0 | automatic linear return after a 120 ms delay (approx., kick jitter +-15 %) |
| W-18.ger_kar98k | ADS 与腰射后坐倍率 | 0.999386 | 比值 | [0.999386, 0.999386] | 1 |  | mean ADS kick / mean hip kick, 2000 shots each | no ADS recoil factor in M0 (stance factor only) |
| W-19.ger_kar98k | 腰射扩散 | 2.80027 | 度 | [0.034264, 3.99983] | 10000 | 4 | angular deviation of 10 000 hip shots from the aim line (median; P95 and max in p95/max) | uniform draw in a cone of half angle hip_deg; median ~ 0.71 x half angle |
| W-20.ger_kar98k | 开镜扩散（站/蹲/趴） | 0.0559529 | 度 | [0, 0.0791294] | 10000 | 0.08 | angular deviation of 10 000 ADS shots, standing | crouching ADS median 0.0442 deg; prone is M1-011 |
| W-21.ger_kar98k | 扩散随移动/姿态/连射的增长与恢复 | 2.49175 | 比值 | [2.49175, 2.49175] | 1 |  | median hip deviation while running (4.4 m/s) / standing still | walk (2 m/s) x1.496, crouch x0.798; bloom per shot 0 deg decays at 0 deg/s |
| W-22.ger_kar98k | 弹头速度（曳光飞行时间法，>= 200 m） | 695.545 | 米/秒 | [683.338, 707.752] | 2 | 760 | v = d / t_flight at 200 m and 300 m (average velocity over the flight, as read from a recording) | time of flight 200 m 0.2826 s, 300 m 0.4390 s; the average is below the muzzle velocity because of drag |
| W-23.ger_kar98k | 下坠：100 m 落点偏移 | 0.0949734 | 米 | [0.0949734, 0.0949734] | 1 |  | integrated projectile height below the bore line at that range |  |
| W-24.ger_kar98k | 下坠：200 m 落点偏移 | 0.38605 | 米 | [0.38605, 0.38605] | 1 |  | integrated projectile height below the bore line at that range |  |
| W-25.ger_kar98k | 下坠：300 m 落点偏移 | 0.902137 | 米 | [0.902137, 0.902137] | 1 |  | integrated projectile height below the bore line at that range | 400 m 1.676 m, 500 m 2.745 m; relative to the bore line, level shot |
| W-26.ger_kar98k | 弹头飞行形式（hitscan/弹丸） | 不可测 | 枚举 |  | 0 |  | code inspection / projectile spawn | Enumeration, no number: simulated projectile (semi-implicit Euler, 4 sub-steps/tick, drag k), not hitscan |
| W-27.ger_kar98k | 准星晃动幅度与频率（ADS 站立，无屏息） | 不可测 | 度 |  | 0 |  | none (feature absent in M0 sim) | Sway and breath holding are not implemented in M0 (no sway state in WeaponState) |
| W-28.ger_kar98k | 晃动随姿态变化 | 不可测 | 度 |  | 0 |  | none (feature absent in M0 sim) | Sway and breath holding are not implemented in M0 (no sway state in WeaponState) |
| W-29.ger_kar98k | 屏息：抑制比、持续、恢复 | 不可测 | 度 |  | 0 |  | none (feature absent in M0 sim) | Sway and breath holding are not implemented in M0 (no sway state in WeaponState) |
| W-30.ger_kar98k | 弹匣容量与携弹量 | 5 | 发 | [5, 5] | 1 | 5 | magazine rounds of a fresh weapon | spare rounds with 6 spare magazines: 30; data mag_reserve 6 |
| W-37.ger_kar98k | 伤害：各距离所需发数与部位倍率 | 1 | 发 | [1, 1] | 1 |  | ceil(health / damage(zone, distance)) from projectile::damage; torso at 100 m | torso 100.00 HP at 100 m, 96.67 HP at 300 m; see H-01 |
| H-01.ger_kar98k@50m | 致死发数（躯干） | 1 | 发 | [1, 1] | 1 |  | ceil(100 HP / damage(torso, d)) | damage 100.00 HP |
| H-01.ger_kar98k@150m | 致死发数（躯干） | 1 | 发 | [1, 1] | 1 |  | ceil(100 HP / damage(torso, d)) | damage 100.00 HP |
| H-01.ger_kar98k@300m | 致死发数（躯干） | 2 | 发 | [2, 2] | 1 |  | ceil(100 HP / damage(torso, d)) | damage 96.67 HP |
| H-02.ger_kar98k | 头部致死发数 | 1 | 发 | [1, 1] | 1 |  | ceil(100 HP / damage(head, 50 m)) | head multiplier 3 |
| H-03.ger_kar98k | 四肢致死发数/伤害倍率 | 2 | 发 | [2, 2] | 1 |  | ceil(100 HP / damage(limb, 50 m)) | limb multiplier 0.65 |
| M-01 | 行走速度 | 2.00003 | 米/秒 | [2.00003, 2.00003] | 1 | 2 | displacement over 3 s after 20 warm-up ticks (flat ground) |  |
| M-02 | 奔跑速度 | 4.40002 | 米/秒 | [4.40002, 4.40002] | 1 | 4.4 | same |  |
| M-03 | 冲刺速度 | 6.39999 | 米/秒 | [6.39999, 6.39999] | 1 | 6.4 | same | M0 has unlimited stamina |
| M-04 | 蹲行速度 | 2.4 | 米/秒 | [2.4, 2.4] | 1 | 2.4 | same (crouch, not walk-modified) | crouch+walk 1.500 m/s |
| M-05 | 匍匐速度 | 不可测 | 米/秒 |  | 0 |  | none (feature absent in M0 sim) | Prone is M1-011; the sim ignores the PRONE button (data prone_mps unused) |
| M-06 | 后退/侧移速度倍率 | 0.999997 | 比值 | [0.999994, 1] | 2 |  | steady speed backwards / sideways divided by forward run speed | min/max = the two ratios (backward 1.000, strafe 1.000) |
| M-12 | 姿态切换：站->蹲 | 0.25 | 秒 | [0.25, 0.25] | 1 | 0.25 | blend slope over 3 ticks (full transition = 1 / slope) | linear blend, 7.500 ticks = 0.250 s; the transition completes on whole tick 8 (0.267 s) |
| M-13 | 姿态切换：蹲->站 | 0.25 | 秒 | [0.25, 0.25] | 1 | 0.25 | same |  |
| M-14 | 姿态切换：站/蹲->趴；趴->站/蹲 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | Prone is M1-011 |
| M-17 | 翻越矮墙时间与最高可翻越高度 | 不可测 | 秒 |  | 0 |  | none (feature absent in M0 sim) | No vault in M0 (autostep only: max step height in data autostep_max_m) |
| M-19 | 跳跃高度与滞空时间 | 0.598678 | 米 | [0.598678, 0.598678] | 1 | 0.6 | max foot height above the take-off point, sampled per tick | air time 0.700 s (21 ticks); peak sampled at 30 Hz (<= 1 cm low) |
| M-24 | 加速时间（静止->95 % 稳定速度） | 0.24167 | 秒 | [0.24167, 0.24167] | 1 | 0.25 | per-tick speed, linear interpolation of the 95 % crossing | steady 4.400 m/s; data accel_time_s is the 0 -> 100 % time, 95 % = 0.95 x |
| M-25 | 减速时间与滑行距离 | 0.19 | 秒 | [0.19, 0.19] | 1 | 0.2 | per-tick speed after releasing the key, 5 % crossing (interpolated) | stopping distance 0.367 m; the 5 % crossing of a linear ramp is 0.95 x decel_time_s |
| M-26 | 转向惯性：180 度反向时间 | 0.25 | 秒 | [0.25, 0.25] | 1 |  | time until the velocity along the old heading reaches 0 (direction flips) | 95 % of the speed reached in the new direction after 0.488 s |
