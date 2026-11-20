use crate::components::{PlayerIndex, Team, Transform};
use crate::health::LifeState;
use crate::respawn::{ms_to_ticks, RespawnQueue};
use crate::spawn::garrison::{GarrisonState, Garrisons, SpawnRequestError};
use crate::spawn::outpost::{self, OutpostError, OutpostPlace, OutpostState, Outposts, PlaceRules};
use crate::{PlayerId, Tick, Vec3, World};

impl World {
    pub fn outpost_list(&self) -> impl Iterator<Item = &OutpostState> {
        self.ecs.resource::<Outposts>().list.values()
    }

    pub fn preview_outpost(&self, who: PlayerId, pos: Vec3) -> Result<(), OutpostError> {
        self.outpost_placement(who, pos, true).map(|_| ())
    }

    pub fn place_outpost(&mut self, who: PlayerId, pos: Vec3) -> Result<(), OutpostError> {
        let p = self.outpost_placement(who, pos, false)?;
        self.ecs
            .resource_mut::<Outposts>()
            .placing
            .insert(p.squad, p);
        Ok(())
    }

    fn outpost_placement(
        &self,
        who: PlayerId,
        pos: Vec3,
        preview: bool,
    ) -> Result<OutpostPlace, OutpostError> {
        let def = self.cfg.data.outpost();
        let squad = self
            .squads
            .squad_of(who)
            .ok_or(OutpostError::NeedSquadmates)?;
        if squad.leader != who || !self.outpost_officer(who) {
            return Err(OutpostError::WrongRole);
        }
        let players = &self.ecs.resource::<PlayerIndex>().map;
        let &(entity, e) = players.get(&who).ok_or(OutpostError::NotAlive)?;
        let body = self.ecs.entity(e);
        if !body.get::<LifeState>().is_some_and(outpost::alive) {
            return Err(OutpostError::NotAlive);
        }
        let t = body.get::<Transform>().ok_or(OutpostError::NotAlive)?;
        let y = self
            .collision()
            .height_at(pos.x, pos.z)
            .ok_or(OutpostError::Slope)?;
        let pos = Vec3::new(pos.x, y, pos.z);
        if !pos.is_finite() || t.pos.distance_squared(pos) > def.placement_reach_m.powi(2) {
            return Err(OutpostError::Unreachable);
        }
        let team = squad.team;
        let outposts = self.ecs.resource::<Outposts>();
        if outposts.placing.contains_key(&squad.id) {
            return Err(OutpostError::AlreadyPlacing);
        }
        if outpost::ID_START
            .checked_add(outposts.next)
            .is_none_or(|id| id > outpost::ID_END)
        {
            return Err(OutpostError::IdExhausted);
        }
        let current = self.tick().0;
        let mut nearby_mates = 0;
        let mut enemy_nearby = false;
        for (&peer, &(_, e)) in players {
            if peer == who {
                continue;
            }
            let body = self.ecs.entity(e);
            if !body.get::<LifeState>().is_some_and(outpost::alive) {
                continue;
            }
            let (Some(t), Some(&peer_team)) = (body.get::<Transform>(), body.get::<Team>()) else {
                continue;
            };
            let d = outpost::distance_sq(t.pos, pos);
            if peer_team == team && squad.has(peer) && d <= def.squadmate_radius_m.powi(2) {
                nearby_mates += 1;
            } else if !preview && peer_team != team && d <= def.blocked_radius_m.powi(2) {
                enemy_nearby = true;
            }
        }
        let friendly_spawns: Vec<_> = self
            .ecs
            .resource::<Garrisons>()
            .list
            .values()
            .filter(|g| g.team == team && g.state != GarrisonState::Destroyed)
            .map(|g| g.pos)
            .chain(
                outposts
                    .list
                    .values()
                    .filter(|p| p.team == team && p.squad != squad.id)
                    .map(|p| p.pos),
            )
            .collect();
        let enemy_points: Vec<_> = self
            .objective_list()
            .iter()
            .filter(|o| o.owner.is_some_and(|owner| owner != team))
            .map(|o| (o.pos, o.radius_m))
            .collect();
        outpost::check_place(
            def,
            &PlaceRules {
                pos,
                nearby_mates,
                enemy_nearby,
                friendly_spawns: &friendly_spawns,
                enemy_points: &enemy_points,
                slope: self.collision().slope_deg_at(pos.x, pos.z),
                mounted: self.seat_of(who).is_some(),
                now: current,
                cooldown_until: outposts.cooldown.get(&squad.id).copied().unwrap_or(0),
            },
        )?;
        Ok(OutpostPlace {
            builder: who,
            entity,
            team,
            squad: squad.id,
            pos,
            anchor: t.pos,
            last_hp: self.player_hp(who).unwrap_or(0.0),
            complete_at: current + ms_to_ticks(def.place_ms, self.cfg.tick_hz),
        })
    }

    fn outpost_officer(&self, who: PlayerId) -> bool {
        self.squads
            .entry(who)
            .is_some_and(|e| !e.role_invalid && e.role == Some("officer"))
    }

    /// Refresh permissions before tick, including queued spawns after a roster change.
    pub(super) fn sync_outpost_permissions(&mut self) {
        let idle = {
            let o = self.ecs.resource::<Outposts>();
            o.placing.is_empty() && o.choice.is_empty()
        };
        if idle {
            return;
        }
        let squads = &self.squads;
        let mut o = self.ecs.resource_mut::<Outposts>();
        o.placing.retain(|&sid, p| {
            squads
                .squad_of(p.builder)
                .is_some_and(|s| s.id == sid && s.leader == p.builder)
                && squads
                    .entry(p.builder)
                    .is_some_and(|e| !e.role_invalid && e.role == Some("officer"))
        });
        let o = &mut *o;
        o.choice.retain(|&p, id| {
            o.list.get(id).is_some_and(|op| {
                squads
                    .squad_of(p)
                    .is_some_and(|s| s.id == op.squad && s.team == op.team)
            })
        });
    }

    pub(crate) fn check_outpost_point(
        &self,
        player: PlayerId,
        id: u16,
    ) -> Result<(), SpawnRequestError> {
        let point = self
            .ecs
            .resource::<Outposts>()
            .list
            .get(&id)
            .ok_or(SpawnRequestError::NoSuchPoint)?;
        if !self
            .squads
            .squad_of(player)
            .is_some_and(|s| s.team == point.team && s.id == point.squad)
        {
            return Err(SpawnRequestError::NoSuchPoint);
        }
        if point.spawns_left == 0 {
            return Err(SpawnRequestError::NoSlots);
        }
        if self.tick().0 >= point.expires_tick {
            return Err(SpawnRequestError::NotActive);
        }
        if point.blocked {
            return Err(SpawnRequestError::Blocked);
        }
        Ok(())
    }

    pub(super) fn queue_outpost_spawn(
        &mut self,
        who: PlayerId,
        id: u16,
        t_kia: Tick,
    ) -> Result<u32, SpawnRequestError> {
        self.check_outpost_point(who, id)?;
        self.ecs
            .resource_mut::<crate::spawn::SpawnRules>()
            .choice
            .remove(&who);
        self.ecs.resource_mut::<Outposts>().choice.insert(who, id);
        let now = self.tick().0;
        let due = (t_kia.0 + ms_to_ticks(self.cfg.data.outpost().respawn_ms, self.cfg.tick_hz))
            .max(now + 1);
        let mut q = self.ecs.resource_mut::<RespawnQueue>();
        q.remove_player(who);
        q.schedule(who, Tick(due));
        Ok(due - now)
    }

    /// Sim damage entry point used by the shared structure damage path.
    pub fn damage_outpost(&mut self, id: u16, raw: f32, explosive: bool) {
        let now = self.tick().0;
        let d = self.cfg.data.outpost();
        self.pending_events
            .extend(self.ecs.resource_mut::<Outposts>().damage(
                id,
                raw,
                explosive,
                now,
                d,
                self.cfg.tick_hz,
            ));
    }
}
