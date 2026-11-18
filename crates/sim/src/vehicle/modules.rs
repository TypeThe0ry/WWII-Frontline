//! Damageable vehicle modules (M1-023c; GD 14.5): HP per module and the effects of the
//! `<= 50%` and `0 HP` thresholds, read from the vehicle definition's `modules` table.

use crate::data::defs::VehicleDef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModuleId {
    Engine = 0,
    TrackL = 1,
    TrackR = 2,
    TurretDrive = 3,
    MainGun = 4,
    AmmoRack = 5,
}

pub const MODULE_COUNT: usize = 6;

impl ModuleId {
    pub const ALL: [ModuleId; MODULE_COUNT] = [
        ModuleId::Engine,
        ModuleId::TrackL,
        ModuleId::TrackR,
        ModuleId::TurretDrive,
        ModuleId::MainGun,
        ModuleId::AmmoRack,
    ];

    pub fn from_data_id(s: &str) -> Option<ModuleId> {
        Some(match s {
            "engine" => ModuleId::Engine,
            "track_l" => ModuleId::TrackL,
            "track_r" => ModuleId::TrackR,
            "turret_drive" => ModuleId::TurretDrive,
            "main_gun" => ModuleId::MainGun,
            "ammo_rack" => ModuleId::AmmoRack,
            _ => return None,
        })
    }
}

/// One module's data: HP and the two effect maps.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModuleSpec {
    pub max_hp: f32,
    pub at_50: Vec<(String, f32)>,
    pub at_0: Vec<(String, f32)>,
    /// `at_0` flags set to `true`.
    pub flags_0: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleSpecs(pub [ModuleSpec; MODULE_COUNT]);

fn collect(
    m: Option<&std::collections::BTreeMap<String, serde_json::Value>>,
) -> (Vec<(String, f32)>, Vec<String>) {
    let mut nums = Vec::new();
    let mut flags = Vec::new();
    if let Some(m) = m {
        for (k, v) in m {
            if let Some(f) = v.as_f64() {
                nums.push((k.clone(), f as f32));
            } else if v.as_bool() == Some(true) {
                flags.push(k.clone());
            }
        }
    }
    (nums, flags)
}

impl ModuleSpecs {
    pub fn from_def(def: &VehicleDef) -> Self {
        let mut out: [ModuleSpec; MODULE_COUNT] = Default::default();
        for m in &def.gameplay.modules {
            let Some(id) = ModuleId::from_data_id(&m.id) else {
                continue;
            };
            let (a, _) = collect(m.effects.get("at_50pct"));
            let (b, flags) = collect(m.effects.get("at_0"));
            out[id as usize] = ModuleSpec {
                max_hp: m.health_hp as f32,
                at_50: a,
                at_0: b,
                flags_0: flags,
            };
        }
        ModuleSpecs(out)
    }
}

/// Combined effects of all module damage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModuleEffects {
    pub speed_mult: f32,
    pub accel_mult: f32,
    pub thrust_mult_l: f32,
    pub thrust_mult_r: f32,
    pub traverse_mult: f32,
    pub turret_jammed: bool,
    pub reload_mult: f32,
    pub spread_mult: f32,
    pub can_fire: bool,
    pub immobilized: bool,
}

impl Default for ModuleEffects {
    fn default() -> Self {
        ModuleEffects {
            speed_mult: 1.0,
            accel_mult: 1.0,
            thrust_mult_l: 1.0,
            thrust_mult_r: 1.0,
            traverse_mult: 1.0,
            turret_jammed: false,
            reload_mult: 1.0,
            spread_mult: 1.0,
            can_fire: true,
            immobilized: false,
        }
    }
}

/// What a point of module damage triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DamageOutcome {
    /// The module just reached 0 HP.
    pub destroyed: bool,
    /// The ammo rack reached 0 HP and the cook-off roll hit.
    pub cookoff: bool,
    /// The engine reached 0 HP and the fire roll hit.
    pub fire: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Modules {
    pub hp: [f32; MODULE_COUNT],
}

impl Modules {
    pub fn new(specs: &ModuleSpecs) -> Self {
        let mut hp = [0.0; MODULE_COUNT];
        for (h, s) in hp.iter_mut().zip(specs.0.iter()) {
            *h = s.max_hp;
        }
        Modules { hp }
    }

    /// Applies `dmg`; `u_cookoff` / `u_fire` are `[0,1)` rolls from the deterministic RNG,
    /// used only when the module reaches 0 HP on this hit.
    pub fn damage(
        &mut self,
        specs: &ModuleSpecs,
        id: ModuleId,
        dmg: f32,
        u_cookoff: f32,
        u_fire: f32,
    ) -> DamageOutcome {
        let i = id as usize;
        let was = self.hp[i];
        if was <= 0.0 {
            return DamageOutcome::default();
        }
        self.hp[i] = (was - dmg.max(0.0)).max(0.0);
        let mut out = DamageOutcome::default();
        if self.hp[i] <= 0.0 {
            out.destroyed = true;
            let s = &specs.0[i];
            let chance = |key: &str| {
                s.at_0
                    .iter()
                    .find(|(k, _)| k == key)
                    .map_or(0.0, |(_, v)| *v / 100.0)
            };
            match id {
                ModuleId::AmmoRack => out.cookoff = u_cookoff < chance("cookoff_chance_pct"),
                ModuleId::Engine => out.fire = u_fire < chance("fire_chance_pct"),
                _ => {}
            }
        }
        out
    }

    fn stage(&self, specs: &ModuleSpecs, id: ModuleId) -> u8 {
        let i = id as usize;
        let max = specs.0[i].max_hp;
        if max <= 0.0 {
            0
        } else if self.hp[i] <= 0.0 {
            2
        } else if self.hp[i] <= 0.5 * max {
            1
        } else {
            0
        }
    }

    pub fn effects(&self, specs: &ModuleSpecs) -> ModuleEffects {
        let mut e = ModuleEffects::default();
        for id in ModuleId::ALL {
            let st = self.stage(specs, id);
            if st == 0 {
                continue;
            }
            let s = &specs.0[id as usize];
            let mut apply = |k: &str, v: f32| match (id, k) {
                (_, "speed_mult") => e.speed_mult = e.speed_mult.min(v),
                (_, "accel_mult") => e.accel_mult = e.accel_mult.min(v),
                (ModuleId::TrackL, "thrust_mult") => e.thrust_mult_l = e.thrust_mult_l.min(v),
                (ModuleId::TrackR, "thrust_mult") => e.thrust_mult_r = e.thrust_mult_r.min(v),
                (_, "traverse_mult") => e.traverse_mult = e.traverse_mult.min(v),
                (_, "reload_mult") => e.reload_mult = e.reload_mult.max(v),
                (_, "spread_mult") => e.spread_mult = e.spread_mult.max(v),
                _ => {}
            };
            for (k, v) in &s.at_50 {
                apply(k, *v);
            }
            if st == 2 {
                for (k, v) in &s.at_0 {
                    apply(k, *v);
                }
                for f in &s.flags_0 {
                    if f == "immobilized" {
                        e.immobilized = true;
                    }
                }
                if s.at_0.iter().any(|(k, _)| k == "manual_traverse_deg_s") {
                    e.turret_jammed = true;
                }
                if id == ModuleId::MainGun {
                    e.can_fire = false;
                }
            }
        }
        e
    }
}
