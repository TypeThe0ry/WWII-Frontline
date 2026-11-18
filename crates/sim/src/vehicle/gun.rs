//! Main-gun state machine (M1-023b; GD 14.2): ammunition per shell type, loading, shell
//! switching and the reload-time multipliers.

/// Shell families of the Sherman's 75 mm (AP/HE/WP order matches the data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShellSlot {
    Ap = 0,
    He = 1,
    Wp = 2,
}

pub const SHELL_SLOTS: usize = 3;

/// Reload multiplier without a commander in the turret (GD 14.2).
pub const NO_COMMANDER_RELOAD_MULT: f32 = 1.3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GunParams {
    pub reload_s: f32,
    pub switch_reload_s: f32,
    pub ammo: [u16; SHELL_SLOTS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireError {
    NoGun,
    Loading,
    OutOfAmmo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gun {
    pub ammo: [u16; SHELL_SLOTS],
    /// Shell currently in the breech (or being loaded).
    pub loaded: ShellSlot,
    /// Ticks left before the gun can fire (0 = ready).
    pub reload_left: u32,
}

impl Gun {
    pub fn new(p: &GunParams) -> Self {
        Gun {
            ammo: p.ammo,
            loaded: ShellSlot::Ap,
            reload_left: 0,
        }
    }

    pub fn ready(&self) -> bool {
        self.reload_left == 0
    }

    /// Selects another shell: a ready gun must reload for `switch_reload_s` (x multipliers).
    pub fn select(&mut self, p: &GunParams, s: ShellSlot, mult: f32, hz: f32) {
        if s == self.loaded {
            return;
        }
        self.loaded = s;
        self.reload_left = secs_to_ticks(p.switch_reload_s * mult, hz);
    }

    /// Fires the loaded shell; starts the normal reload. `mult` folds the commander and
    /// module effects into the reload time.
    pub fn fire(&mut self, p: &GunParams, mult: f32, hz: f32) -> Result<ShellSlot, FireError> {
        if !self.ready() {
            return Err(FireError::Loading);
        }
        let i = self.loaded as usize;
        if self.ammo[i] == 0 {
            return Err(FireError::OutOfAmmo);
        }
        self.ammo[i] -= 1;
        self.reload_left = secs_to_ticks(p.reload_s * mult, hz);
        Ok(self.loaded)
    }

    pub fn tick(&mut self) {
        self.reload_left = self.reload_left.saturating_sub(1);
    }
}

fn secs_to_ticks(s: f32, hz: f32) -> u32 {
    libm::roundf(s * hz) as u32
}
