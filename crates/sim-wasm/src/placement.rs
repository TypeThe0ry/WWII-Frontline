//! Read-only placement preview. WASM supplies the loaded collision and sim owns the rules.

use std::sync::OnceLock;

use frontline_sim::spawn::garrison::{water_material, GarrisonPlacement, PlaceError, TEAM_CAP};
use frontline_sim::supply::{ammo::AmmoCfg, check_supply_crate_place};
use frontline_sim::{GameData, NoBuildZones, ObjectiveSpec, Team, Vec3};
use js_sys::{Object, Reflect};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::js_err;
use crate::predictor::{Predictor, TICK_HZ};

#[wasm_bindgen]
impl Predictor {
    /// Replace the map's no-build polygons (`[[[x, z], ...], ...]`); `[]` clears them.
    #[wasm_bindgen(js_name = setNoBuildZones)]
    pub fn set_no_build_zones(&mut self, zones_json: &str) -> Result<(), JsError> {
        let polygons = serde_json::from_str(zones_json).map_err(js_err)?;
        self.no_build_zones = NoBuildZones::new(polygons).map_err(js_err)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Side {
    Us,
    Ger,
}

impl From<Side> for Team {
    fn from(side: Side) -> Self {
        match side {
            Side::Us => Self::Us,
            Side::Ger => Self::Ger,
        }
    }
}

#[derive(Deserialize)]
struct ObjectiveFact {
    id: u16,
    pos: [f32; 3],
    radius_m: f32,
    owner: Option<Side>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Facts {
    Garrison {
        alive: bool,
        officer: bool,
        team: Side,
        hq: Vec<[f32; 3]>,
        garrisons: Vec<(Side, [f32; 3])>,
        objectives: Vec<ObjectiveFact>,
        enemies: Vec<[f32; 3]>,
        available_supplies: u32,
    },
    SupplyCrate {
        alive: bool,
        support: bool,
        carrying: bool,
        team: Side,
        objectives: Vec<ObjectiveFact>,
    },
    AmmoCrate {
        alive: bool,
        support: bool,
        tick: u32,
        used: u8,
        last_placed_tick: Option<u32>,
        crates: Vec<[f32; 3]>,
    },
}

fn vec3(pos: [f32; 3]) -> Result<Vec3, JsError> {
    let pos = Vec3::from_array(pos);
    if pos.is_finite() {
        Ok(pos)
    } else {
        Err(js_err("placement positions must be finite"))
    }
}

fn answer(
    reason: Option<&str>,
    supply_cost: Option<u32>,
    remaining: Option<u32>,
) -> Result<Object, JsValue> {
    let result = Object::new();
    Reflect::set(&result, &"legal".into(), &reason.is_none().into())?;
    if let Some(reason) = reason {
        Reflect::set(&result, &"reason".into(), &reason.into())?;
    }
    if let Some(cost) = supply_cost {
        Reflect::set(&result, &"supplyCost".into(), &cost.into())?;
    }
    if let Some(remaining) = remaining {
        Reflect::set(&result, &"remaining".into(), &remaining.into())?;
    }
    Ok(result)
}

/// Predict placement without changing the predictor, using a snapshot of locally known facts.
/// `facts_json` is the tagged context for `kind`. Pass the simulated feet position for crates,
/// because crate actions in sim place at the actor. No extra terrain copy is needed.
#[wasm_bindgen]
pub fn placement_check(
    predictor: &Predictor,
    kind: &str,
    pos: &[f32],
    facts_json: &str,
) -> Result<Object, JsValue> {
    let pos = vec3(
        pos.try_into()
            .map_err(|_| js_err("placement position must have 3 coordinates"))?,
    )?;
    let facts = serde_json::from_str::<Facts>(facts_json).map_err(js_err)?;
    match (kind, facts) {
        (
            "garrison",
            Facts::Garrison {
                alive,
                officer,
                team,
                hq,
                garrisons,
                objectives,
                enemies,
                available_supplies,
            },
        ) => {
            if !alive || !officer {
                let reason = if alive {
                    PlaceError::WrongRole
                } else {
                    PlaceError::NotAlive
                };
                return answer(Some(&format!("{reason:?}")), None, None);
            }
            let team = Team::from(team);
            let hq = hq.into_iter().map(vec3).collect::<Result<Vec<_>, _>>()?;
            let garrisons = garrisons
                .into_iter()
                .map(|(side, point)| Ok::<_, JsError>((side.into(), vec3(point)?)))
                .collect::<Result<Vec<_>, _>>()?;
            let objectives = objectives
                .into_iter()
                .map(|o| {
                    if !o.radius_m.is_finite() || o.radius_m < 0.0 {
                        return Err(js_err("invalid placement objective radius"));
                    }
                    Ok(ObjectiveSpec {
                        id: o.id,
                        pos: vec3(o.pos)?,
                        radius_m: o.radius_m,
                        owner: o.owner.map(Team::from),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let enemies = enemies
                .into_iter()
                .map(vec3)
                .collect::<Result<Vec<_>, _>>()?;
            let context = GarrisonPlacement {
                team,
                hq: &hq,
                garrisons: &garrisons,
                objectives: &objectives,
                no_build_zones: &predictor.no_build_zones,
                collision: predictor.sim.collision(),
                water_material: water_material(GameData::embedded()),
                enemies: &enemies,
            };
            match context.check(pos, || available_supplies) {
                Ok(cost) => {
                    let count = garrisons.iter().filter(|(side, _)| *side == team).count();
                    answer(None, Some(cost.supplies), Some((TEAM_CAP - count) as u32))
                }
                Err(reason) => answer(Some(&format!("{reason:?}")), None, None),
            }
        }
        (
            "supply_crate",
            Facts::SupplyCrate {
                alive,
                support,
                carrying,
                team,
                objectives,
            },
        ) => {
            if let Some(reason) = support_error(alive, support) {
                return answer(Some(reason), None, None);
            }
            if !carrying {
                return answer(Some("NotCarrying"), None, None);
            }
            let objectives = objectives
                .into_iter()
                .map(|o| Ok::<_, JsError>((o.owner.map(Team::from), vec3(o.pos)?)))
                .collect::<Result<Vec<_>, _>>()?;
            match check_supply_crate_place(
                objectives.into_iter(),
                team.into(),
                pos,
                &predictor.no_build_zones,
            ) {
                Ok(()) => answer(None, None, None),
                Err(reason) => answer(Some(&format!("{reason:?}")), None, None),
            }
        }
        (
            "ammo_crate",
            Facts::AmmoCrate {
                alive,
                support,
                tick,
                used,
                last_placed_tick,
                crates,
            },
        ) => {
            if let Some(reason) = support_error(alive, support) {
                return answer(Some(reason), None, None);
            }
            static AMMO: OnceLock<AmmoCfg> = OnceLock::new();
            let cfg = AMMO.get_or_init(|| AmmoCfg::from_data(GameData::embedded(), TICK_HZ));
            let crates = crates
                .into_iter()
                .map(vec3)
                .collect::<Result<Vec<_>, _>>()?;
            match cfg.check_place(
                tick,
                used,
                last_placed_tick,
                pos,
                &predictor.no_build_zones,
                crates.into_iter(),
            ) {
                Ok(remaining) => answer(None, None, Some(remaining.into())),
                Err(reason) => answer(Some(&format!("{reason:?}")), None, None),
            }
        }
        _ => Err(js_err("placement kind does not match facts").into()),
    }
}

fn support_error(alive: bool, support: bool) -> Option<&'static str> {
    if !alive {
        Some("NotAlive")
    } else if !support {
        Some("NotSupport")
    } else {
        None
    }
}
