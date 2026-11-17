//! Magazine / reserve bookkeeping (GAMEDESIGN 6.3 reload, M0: whole-clip loading only).

use super::state::{ReloadKind, WeaponState};

/// Reload kind for the current magazine content (`empty_ms` when empty, else `tactical_ms`).
pub fn reload_kind(w: &WeaponState) -> ReloadKind {
    if w.mag == 0 {
        ReloadKind::Empty
    } else {
        ReloadKind::Tactical
    }
}

/// A reload makes sense: the magazine is not full and spare rounds exist.
pub fn can_reload(w: &WeaponState, mag_capacity: u16) -> bool {
    w.mag < mag_capacity && w.reserve > 0
}

/// Moves rounds from the reserve into the magazine up to its capacity; returns how many.
/// `mag + reserve` is unchanged.
pub fn complete_reload(w: &mut WeaponState, mag_capacity: u16) -> u16 {
    let take = (mag_capacity.saturating_sub(w.mag)).min(w.reserve);
    w.mag += take;
    w.reserve -= take;
    take
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_fills_and_conserves() {
        let mut w = WeaponState::new(0, 3, 48);
        assert_eq!(reload_kind(&w), ReloadKind::Tactical);
        assert!(can_reload(&w, 8));
        assert_eq!(complete_reload(&mut w, 8), 5);
        assert_eq!((w.mag, w.reserve), (8, 43));
        assert!(!can_reload(&w, 8));
        let mut e = WeaponState::new(0, 0, 3);
        assert_eq!(reload_kind(&e), ReloadKind::Empty);
        assert_eq!(
            complete_reload(&mut e, 8),
            3,
            "partial fill from a small reserve"
        );
        assert_eq!((e.mag, e.reserve), (3, 0));
        assert!(
            !can_reload(&WeaponState::new(0, 0, 0), 8),
            "no reserve: invalid"
        );
    }
}
