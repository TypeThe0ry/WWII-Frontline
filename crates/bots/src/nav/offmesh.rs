//! Off-mesh links (windows, ladders, stairs, vaults). The M1 bake ships none (PI-085); the type
//! and the state machine skeleton exist so M1-038 and the grey-box interiors can plug in.

use super::V2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Window,
    Ladder,
    Stairs,
    Vault,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Link {
    pub kind: LinkKind,
    pub start: V2,
    pub end: V2,
    pub radius: f32,
    pub bidirectional: bool,
    pub cost: f32,
}

/// Which action bit the steering needs while crossing a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinkAction {
    pub jump: bool,
    pub vault: bool,
}

pub fn action_for(kind: LinkKind) -> LinkAction {
    match kind {
        LinkKind::Window | LinkKind::Vault => LinkAction {
            jump: false,
            vault: true,
        },
        LinkKind::Ladder => LinkAction {
            jump: false,
            vault: false,
        },
        LinkKind::Stairs => LinkAction {
            jump: false,
            vault: false,
        },
    }
}
