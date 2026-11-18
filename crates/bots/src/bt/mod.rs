//! Behaviour-tree engine (ARCHITECTURE 16.3, M1-038).
//!
//! A tree is a flat `Vec<Node>` plus a flat child-index array: no boxes, no recursion over heap
//! objects, and the per-bot mutable state ([`Blackboard`]) is two preallocated vectors, so a tick
//! allocates nothing. Leaves are `u16` ids resolved by the caller through [`Leaves`].
//!
//! Node semantics (all "memory" nodes resume at the child that returned `Running`):
//! * `Sequence`: first non-`Success` child decides; all succeed -> `Success`.
//! * `Selector`: first non-`Failure` child decides; all fail -> `Failure`.
//! * `Parallel{need}`: ticks every child each tick; `Success` once `need` succeed,
//!   `Failure` once `need` can no longer be reached, else `Running`.
//! * `Cooldown{ticks}`: after the child finishes (either way) it is `Failure` for `ticks` ticks.
//! * `Until`: `Running` while the child fails, `Success` the tick it succeeds.
//! * `Invert`: swaps `Success`/`Failure`.

mod blackboard;
mod node;

pub use blackboard::Blackboard;
pub use node::{Builder, Leaves, Node, Status, Tree, TreeError, MAX_NODES};
