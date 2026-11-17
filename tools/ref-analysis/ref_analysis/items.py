"""Catalogue of the M0 batch (REFERENCE_MEASUREMENTS 9.1) with the 4.1-4.3 table columns.

`kind` is the 7.3 tolerance class; `ext` marks series results that go to series/{ID}.csv.
Reference *values* never appear here: only what to measure and how."""

from __future__ import annotations

from dataclasses import dataclass

RIFLES = ("us_m1_garand", "ger_kar98k")
ROLE = "rifleman"


@dataclass(frozen=True)
class Item:
    id: str
    category: str
    item: str
    unit: str
    method: str
    scene: str
    data_field: str
    kind: str
    subjects: tuple[str, ...]
    reps: int = 5
    protocol: str = ""
    series: bool = False

    def rows(self) -> list[tuple[str, str]]:
        """(row id, subject id) pairs: weapon items get the `.weapon_id` suffix (8.2)."""
        if not self.subjects:
            return [(self.id, "")]
        if self.subjects == (ROLE,):
            return [(self.id, ROLE)]
        return [(f"{self.id}.{s}", s) for s in self.subjects]


W = "武器"
M = "移动"
H = "生存"
BOTH = RIFLES
GAR = ("us_m1_garand",)
KAR = ("ger_kar98k",)

M0_ITEMS: tuple[Item, ...] = (
    Item("W-01", W, "实际射速（长连射平均）", "发/分", "M-AUD;M-FRM", "S-RANGE", "weapons/{w}.json:fire.rpm", "timing", BOTH, 5,
         "近距离(<=5 m)连续点射/拉栓打完弹匣的长序列，单独音轨；半自动/栓动取最快可持续节奏", False),
    Item("W-02", W, "半自动最快点射间隔", "秒", "M-AUD", "S-RANGE", "weapons/{w}.json:fire.semi_min_interval_s", "timing", GAR, 5,
         "尽可能快地连点开火键，>= 20 发（弹匣打空换弹后再继续）"),
    Item("W-03", W, "空仓换弹时间", "秒", "M-FRM", "S-RANGE", "weapons/{w}.json:reload.empty_s", "timing", BOTH, 5,
         "打空后按换弹键 -> 可再次开火的首帧"),
    Item("W-04", W, "战术换弹时间", "秒", "M-FRM", "S-RANGE", "weapons/{w}.json:reload.tactical_s", "timing", BOTH, 5,
         "弹匣未空时换弹 -> 可再次开火"),
    Item("W-05", W, "逐发装填每发时间与装满总时间", "秒", "M-FRM", "S-RANGE", "weapons/{w}.json:reload.per_round_s", "timing", KAR, 5,
         "栓动/逐发装填：记录每发装入的间隔与总时长"),
    Item("W-06", W, "打断换弹的最早可开火时刻", "秒", "M-FRM", "S-RANGE", "weapons/{w}.json:reload.interruptible_after_s", "timing", KAR, 5,
         "逐发装填中途按开火键"),
    Item("W-07", W, "开镜进入时间", "秒", "M-FRM;M-ANG", "S-RANGE", "weapons/{w}.json:ads.in_s", "timing", BOTH, 5,
         "按下开镜键 -> FOV/视图稳定的首帧"),
    Item("W-08", W, "开镜退出时间", "秒", "M-FRM;M-ANG", "S-RANGE", "weapons/{w}.json:ads.out_s", "timing", BOTH, 5,
         "松开开镜键 -> FOV/视图恢复稳定"),
    Item("W-09", W, "ADS 放大倍率/FOV", "度", "M-ANG", "S-RANGE", "weapons/{w}.json:ads.fov_deg", "angle", BOTH, 5,
         "腰射与开镜各做一次指南针/远景标定（3.5）"),
    Item("W-11", W, "切枪时间：拔出", "秒", "M-FRM", "S-RANGE", "weapons/{w}.json:equip.draw_s", "timing", BOTH, 5,
         "按下切枪键 -> 可开火首帧"),
    Item("W-12", W, "切枪时间：收起", "秒", "M-FRM", "S-RANGE", "weapons/{w}.json:equip.holster_s", "timing", BOTH, 5,
         "按下切枪键 -> 可拔下一把"),
    Item("W-13", W, "栓动单次循环时间", "秒", "M-FRM", "S-RANGE", "weapons/{w}.json:bolt.cycle_s", "timing", KAR, 5,
         "开火 -> 上膛完成 -> 可再开火"),
    Item("W-15", W, "后坐模式：竖直（前10发）", "度/发", "M-TPL;M-ANG", "S-RANGE", "weapons/{w}.json:recoil.pattern_pitch_deg[]", "angle", BOTH, 10,
         "固定远景地物，腰射连发，n>=10 次；先做 FOV 标定", True),
    Item("W-16", W, "后坐模式：水平（前10发）", "度/发", "M-TPL;M-ANG", "S-RANGE", "weapons/{w}.json:recoil.pattern_yaw_deg[]", "angle", BOTH, 10,
         "同 W-15", True),
    Item("W-17", W, "后坐恢复：停火后回正时间与方式", "秒", "M-TPL;M-FRM", "S-RANGE", "weapons/{w}.json:recoil.recovery_s", "timing", BOTH, 5,
         "单发/连发后视角回正曲线"),
    Item("W-18", W, "ADS 与腰射后坐倍率", "比值", "M-TPL", "S-RANGE", "weapons/{w}.json:recoil.ads_mult", "ratio", BOTH, 10,
         "同一武器腰射 vs 开镜各 n>=10"),
    Item("W-19", W, "腰射扩散", "度", "M-STAT", "S-RANGE", "weapons/{w}.json:spread.hip_deg", "stat", BOTH, 30,
         "固定靶，>= 30 发弹着点，转角度；每发间隔足够使后坐/扩散恢复", False),
    Item("W-20", W, "开镜扩散（站/蹲/趴）", "度", "M-STAT", "S-RANGE", "weapons/{w}.json:spread.ads_deg", "stat", BOTH, 30,
         "同 W-19，开镜"),
    Item("W-21", W, "扩散随移动/姿态/连射的增长与恢复", "度", "M-STAT;M-FRM", "S-RANGE", "weapons/{w}.json:spread.move_mult", "stat", BOTH, 30,
         "静止/走/跑 x 站/蹲 各一组", True),
    Item("W-22", W, "弹头速度（曳光飞行时间法，>= 200 m）", "米/秒", "M-CAL;M-FRM", "S-RANGE", "weapons/{w}.json:ballistics.muzzle_velocity_mps", "speed", BOTH, 5,
         "枪口火光首帧 -> 命中靶标首帧，>= 200 m"),
    Item("W-23", W, "下坠：100 m 落点偏移", "米", "M-CAL-F", "S-RANGE", "weapons/{w}.json:ballistics.drop_table_m[100]", "distance", BOTH, 5,
         "固定瞄准基准，带垂直刻度的靶"),
    Item("W-24", W, "下坠：200 m 落点偏移", "米", "M-CAL-F", "S-RANGE", "weapons/{w}.json:ballistics.drop_table_m[200]", "distance", BOTH, 5, "同 W-23"),
    Item("W-25", W, "下坠：300 m（及 400/500 m）落点偏移", "米", "M-CAL-F", "S-RANGE", "weapons/{w}.json:ballistics.drop_table_m[300]", "distance", BOTH, 5, "同 W-23"),
    Item("W-26", W, "弹头飞行形式（hitscan/弹丸）", "枚举", "M-FRM", "S-RANGE", "weapons/{w}.json:ballistics.model", "count", BOTH, 5,
         "观测记录：是否可见弹丸、命中反馈时间"),
    Item("W-27", W, "准星晃动幅度与频率（ADS 站立，无屏息）", "度", "M-TPL;M-ANG", "S-RANGE", "weapons/{w}.json:sway.amp_deg", "angle", BOTH, 5,
         "ADS 静止 >= 10 s", True),
    Item("W-28", W, "晃动随姿态变化", "度", "M-TPL", "S-RANGE", "weapons/{w}.json:sway.stance_mult[]", "ratio", BOTH, 5,
         "站/蹲/趴各一"),
    Item("W-29", W, "屏息：抑制比、持续、恢复", "秒", "M-TPL;M-FRM", "S-RANGE", "weapons/{w}.json:sway.hold_breath_mult", "timing", BOTH, 5,
         "按住屏息至耗尽，再观察恢复"),
    Item("W-30", W, "弹匣容量与携弹量", "发", "M-UI;M-DOC", "S-RANGE", "weapons/{w}.json:mag.capacity", "count", BOTH, 5,
         "读 HUD 弹药数"),
    Item("W-37", W, "伤害：各距离所需发数与部位倍率", "发", "M-STAT", "S-DUMMY", "weapons/{w}.json:damage.falloff[]", "count", BOTH, 10,
         "与 H-01..H-03 同批录制（共用录像）"),
    Item("M-01", M, "行走速度", "米/秒", "M-CAL", "S-FIELD", "movement.json:walk.speed_mps", "speed", (ROLE,), 5, "沿 >= 100 m 直线匀速走过两个网格点"),
    Item("M-02", M, "奔跑速度", "米/秒", "M-CAL", "S-FIELD", "movement.json:run.speed_mps", "speed", (ROLE,), 5, "同 M-01"),
    Item("M-03", M, "冲刺速度", "米/秒", "M-CAL", "S-FIELD", "movement.json:sprint.speed_mps", "speed", (ROLE,), 5, "同 M-01；体力允许的距离内"),
    Item("M-04", M, "蹲行速度", "米/秒", "M-CAL", "S-FIELD", "movement.json:crouch.speed_mps", "speed", (ROLE,), 5, "同 M-01"),
    Item("M-05", M, "匍匐速度", "米/秒", "M-CAL", "S-FIELD", "movement.json:prone.speed_mps", "speed", (ROLE,), 5, "同 M-01"),
    Item("M-06", M, "后退/侧移速度倍率", "比值", "M-CAL", "S-FIELD", "movement.json:backward_mult", "ratio", (ROLE,), 5, "后退与侧移各一"),
    Item("M-12", M, "姿态切换：站->蹲", "秒", "M-FRM", "S-FIELD", "movement.json:stance.stand_to_crouch_s", "timing", (ROLE,), 5, "按键 -> 视角/眼高稳定"),
    Item("M-13", M, "姿态切换：蹲->站", "秒", "M-FRM", "S-FIELD", "movement.json:stance.crouch_to_stand_s", "timing", (ROLE,), 5, "同 M-12"),
    Item("M-14", M, "姿态切换：站/蹲->趴；趴->站/蹲", "秒", "M-FRM", "S-FIELD", "movement.json:stance.to_prone_s", "timing", (ROLE,), 5, "同 M-12"),
    Item("M-17", M, "翻越矮墙时间与最高可翻越高度", "秒", "M-FRM;M-CAL", "S-OBST", "movement.json:vault.time_s", "timing", (ROLE,), 5, "不同高度矮墙逐个尝试"),
    Item("M-19", M, "跳跃高度与滞空时间", "米", "M-FRM;M-CAL", "S-OBST", "movement.json:jump.height_m", "distance", (ROLE,), 5, "原地起跳，侧面机位+已知高度参照"),
    Item("M-24", M, "加速时间（静止->95 % 稳定速度）", "秒", "M-TPL;M-CAL", "S-FIELD", "movement.json:accel.to_run_s", "timing", (ROLE,), 5, "从静止直线起步，帧级位移"),
    Item("M-25", M, "减速时间与滑行距离", "秒", "M-TPL;M-CAL", "S-FIELD", "movement.json:accel.stop_s", "timing", (ROLE,), 5, "稳定速度下松键"),
    Item("M-26", M, "转向惯性：180 度反向时间", "秒", "M-TPL", "S-FIELD", "movement.json:accel.reverse_s", "timing", (ROLE,), 5, "稳定前进时瞬间反向"),
    Item("H-01", H, "致死发数（躯干，近/中/远）", "发", "M-STAT", "S-DUMMY", "weapons/{w}.json:damage.shots_to_kill[dist]", "count", BOTH, 10, "近/中/远至少各一个距离，每距离 n>=10"),
    Item("H-02", H, "头部致死发数", "发", "M-STAT", "S-DUMMY", "weapons/{w}.json:damage.headshot_stk", "count", BOTH, 10, "同 H-01，瞄准头部"),
    Item("H-03", H, "四肢致死发数/伤害倍率", "发", "M-STAT", "S-DUMMY", "weapons/{w}.json:damage.limb_mult", "count", BOTH, 10, "同 H-01，瞄准四肢"),
)


def all_rows() -> list[tuple[Item, str, str]]:
    """Every (item, row id, subject id) of the M0 batch."""
    return [(it, rid, sid) for it in M0_ITEMS for rid, sid in it.rows()]


def by_id() -> dict[str, tuple[Item, str]]:
    return {rid: (it, sid) for it, rid, sid in all_rows()}
