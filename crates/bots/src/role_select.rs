//! Role choice for bots (M1-039, GAMEDESIGN 4.4): a utility pick among the roles the sim's own
//! quota check ([`role_available`]) allows, so a bot can never ask for a role a human could not.

use frontline_sim::data::defs::RoleDef;
use frontline_sim::roles::quota::{role_available, RoleCtx};
use frontline_sim::squad::SquadKind;

/// Role holders the bot sees in its squad and team (the bot itself excluded).
#[derive(Debug, Clone, Copy)]
pub struct SquadSnapshot<'a> {
    pub kind: Option<SquadKind>,
    /// Squad size including the choosing bot.
    pub size: u32,
    pub is_leader: bool,
    /// `(role id, held in squad, held in team)`.
    pub held: &'a [(&'a str, u32, u32)],
    pub crewman_cap: u32,
}

/// Desirability of a role given the current holders (higher = more needed). Pure.
fn want(id: &str, s: &SquadSnapshot) -> f32 {
    let in_squad = |r: &str| s.held.iter().find(|h| h.0 == r).map_or(0, |h| h.1);
    match id {
        "officer" => 3.0, // leader-only: if allowed, the leader should be the officer
        "medic" => match in_squad("medic") {
            0 => 2.0,
            1 => 1.2,
            _ => 0.2,
        },
        "support" => 1.5 - 0.5 * in_squad("support") as f32,
        "crewman" => 5.0,
        _ => 1.0, // rifleman baseline
    }
}

/// Picks the best allowed role id; falls back to `rifleman` (never fails for a valid squad).
pub fn select_role<'a>(defs: &'a [RoleDef], s: &SquadSnapshot, armor_role: &str) -> &'a str {
    let mut best: Option<(&str, f32)> = None;
    for d in defs {
        let (in_squad, in_team) = s
            .held
            .iter()
            .find(|h| h.0 == d.id)
            .map_or((0, 0), |h| (h.1, h.2));
        let ctx = RoleCtx {
            squad_kind: s.kind,
            squad_size: s.size,
            is_leader: s.is_leader,
            in_squad_of_role: in_squad,
            in_team_of_role: in_team,
            crewman_cap: s.crewman_cap,
            armor_role,
        };
        if role_available(d, &ctx).is_err() {
            continue;
        }
        let w = want(&d.id, s);
        // Ties resolve to the earlier def, so the pick is deterministic.
        if best.is_none_or(|(_, bw)| w > bw) {
            best = Some((&d.id, w));
        }
    }
    best.map_or("rifleman", |b| b.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontline_sim::GameData;

    fn pick(s: &SquadSnapshot) -> String {
        select_role(GameData::embedded().roles, s, "crewman").to_string()
    }

    #[test]
    fn leader_of_big_enough_squad_is_officer() {
        let s = SquadSnapshot {
            kind: Some(SquadKind::Infantry),
            size: 3,
            is_leader: true,
            held: &[],
            crewman_cap: 3,
        };
        assert_eq!(pick(&s), "officer");
    }

    #[test]
    fn non_leader_fills_medic_then_support_then_rifleman() {
        let held0: [(&str, u32, u32); 0] = [];
        let mut s = SquadSnapshot {
            kind: Some(SquadKind::Infantry),
            size: 4,
            is_leader: false,
            held: &held0,
            crewman_cap: 3,
        };
        assert_eq!(pick(&s), "medic");
        let held1 = [("medic", 2, 2)];
        s.held = &held1;
        assert_eq!(pick(&s), "support");
        let held2 = [("medic", 2, 2), ("support", 1, 1)];
        s.held = &held2;
        assert_eq!(pick(&s), "rifleman");
    }

    #[test]
    fn unassigned_is_rifleman() {
        let s = SquadSnapshot {
            kind: None,
            size: 1,
            is_leader: false,
            held: &[],
            crewman_cap: 3,
        };
        assert_eq!(pick(&s), "rifleman");
    }

    /// Acceptance 3: 1000 random situations, never a role the sim's quota check rejects.
    #[test]
    fn role_quota_property() {
        use frontline_sim::rng::Pcg64Mcg;
        use rand_core_shim::next;
        let defs = GameData::embedded().roles;
        let mut rng = Pcg64Mcg::new(99);
        for _ in 0..1000 {
            let kinds = [
                Some(SquadKind::Infantry),
                Some(SquadKind::ArmorCrew),
                Some(SquadKind::Recon),
                None,
            ];
            let kind = kinds[(next(&mut rng) % 4) as usize];
            let held: Vec<(&str, u32, u32)> = defs
                .iter()
                .map(|d| (d.id.as_str(), next(&mut rng) % 4, next(&mut rng) % 12))
                .collect();
            let s = SquadSnapshot {
                kind,
                size: 1 + next(&mut rng) % 6,
                is_leader: next(&mut rng) % 2 == 0,
                held: &held,
                crewman_cap: 3,
            };
            let chosen = select_role(defs, &s, "crewman");
            if chosen == "rifleman" {
                continue; // the documented fallback
            }
            let d = defs.iter().find(|d| d.id == chosen).unwrap();
            let h = held.iter().find(|h| h.0 == chosen).unwrap();
            let ctx = RoleCtx {
                squad_kind: s.kind,
                squad_size: s.size,
                is_leader: s.is_leader,
                in_squad_of_role: h.1,
                in_team_of_role: h.2,
                crewman_cap: 3,
                armor_role: "crewman",
            };
            assert!(role_available(d, &ctx).is_ok(), "{chosen} in {s:?}");
        }
    }

    mod rand_core_shim {
        use frontline_sim::rng::{unit_f32, Pcg64Mcg};
        pub fn next(r: &mut Pcg64Mcg) -> u32 {
            (unit_f32(r) * 1_000_000.0) as u32
        }
    }
}
