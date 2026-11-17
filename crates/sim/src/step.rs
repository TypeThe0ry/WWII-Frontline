//! Output of one simulation step. Variants are appended by later tasks.

use crate::ids::{EntityId, PlayerId};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SimEvent {
    PlayerSpawned { player: PlayerId, entity: EntityId },
    PlayerRemoved { player: PlayerId, entity: EntityId },
}

/// Objective / resource / phase changes (M0-013+; empty for now).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleChange {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepOutput {
    pub events: Vec<SimEvent>,
    pub rule_changes: Vec<RuleChange>,
}
