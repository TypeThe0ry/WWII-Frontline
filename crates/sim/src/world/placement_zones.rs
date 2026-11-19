use bevy_ecs::prelude::Mut;

use crate::components::{PlayerIndex, Team, Transform};
use crate::supply::{
    check_supply_crate_place, Carrying, SupplyCfg, SupplyPlaceError, SupplyWorld, Support,
};
use crate::{LifeState, NoBuildZoneError, NoBuildZones, PlayerId, World};

impl World {
    /// Replace the map's no-build zones. Invalid input leaves the installed zones unchanged.
    pub fn set_no_build_zones(
        &mut self,
        zones: Vec<Vec<[f32; 2]>>,
    ) -> Result<(), NoBuildZoneError> {
        self.ecs.insert_resource(NoBuildZones::new(zones)?);
        Ok(())
    }

    pub fn no_build_zones(&self) -> &NoBuildZones {
        self.ecs.resource::<NoBuildZones>()
    }

    pub fn install_no_build_zones(&mut self, zones: NoBuildZones) {
        self.ecs.insert_resource(zones);
    }

    /// Put down the carried Supply Crate at the living Support player's authoritative feet.
    /// This is the direct action equivalent of a completed hold-USE drop.
    pub fn drop_supply_crate(&mut self, who: PlayerId) -> Result<u32, SupplyPlaceError> {
        let &(_, e) = self
            .ecs
            .resource::<PlayerIndex>()
            .map
            .get(&who)
            .ok_or(SupplyPlaceError::NotAlive)?;
        let er = self.ecs.entity(e);
        let life = er.get::<LifeState>().ok_or(SupplyPlaceError::NotAlive)?;
        if life.is_dead() || life.is_downed() {
            return Err(SupplyPlaceError::NotAlive);
        }
        if !er.get::<Support>().is_some_and(|s| s.0) {
            return Err(SupplyPlaceError::NotSupport);
        }
        if er.get::<Carrying>().and_then(|c| c.0).is_none() {
            return Err(SupplyPlaceError::NotCarrying);
        }
        let team = *er.get::<Team>().ok_or(SupplyPlaceError::NotAlive)?;
        let pos = er.get::<Transform>().ok_or(SupplyPlaceError::NotAlive)?.pos;
        let objectives = &self.ecs.resource::<crate::rules::Objectives>().0;
        check_supply_crate_place(
            objectives.iter().map(|o| (o.owner, o.pos)),
            team,
            pos,
            self.no_build_zones(),
        )?;
        let ground_max = self.ecs.resource::<SupplyCfg>().ground_max;
        let events = &mut self.pending_events;
        self.ecs.resource_scope(|ecs, mut sw: Mut<SupplyWorld>| {
            let mut carry = ecs
                .get_mut::<Carrying>(e)
                .ok_or(SupplyPlaceError::NotCarrying)?;
            sw.drop_carried_crate(who, pos, &mut carry, ground_max, events)
        })
    }
}
