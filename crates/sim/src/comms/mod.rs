//! Pure simulation rules for player communications (M2-019).

pub mod chat;
pub mod markers;
pub mod ping;

pub use markers::{Marker, MarkerErr, MarkerExpired, MarkerId, MarkerKind, MarkerRegistry, Member};

#[cfg(test)]
mod tests;
