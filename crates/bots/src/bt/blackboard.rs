//! Per-bot mutable tree state.

use super::node::Tree;

/// Resume cursors and cooldown deadlines of one bot's tree. Sized once from the tree.
#[derive(Debug, Clone)]
pub struct Blackboard {
    pub(super) cursor: Vec<u16>,
    pub(super) ready_at: Vec<u32>,
}

impl Blackboard {
    pub fn for_tree(t: &Tree) -> Self {
        Blackboard {
            cursor: vec![0; t.len()],
            ready_at: vec![0; t.len()],
        }
    }

    /// Forget every `Running` child (e.g. after death or a goal change).
    pub fn reset(&mut self) {
        self.cursor.iter_mut().for_each(|c| *c = 0);
    }
}
