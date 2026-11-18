//! Tree nodes, the builder (enforces the node cap) and the tick function.

use super::blackboard::Blackboard;

/// ARCHITECTURE 16.3: at most 80 nodes per bot tree.
pub const MAX_NODES: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Success,
    Failure,
    Running,
}

/// A node. `first..first+len` indexes [`Tree`]'s child array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    Sequence { first: u16, len: u16 },
    Selector { first: u16, len: u16 },
    Parallel { first: u16, len: u16, need: u16 },
    Condition(u16),
    Action(u16),
    Cooldown { child: u16, ticks: u32 },
    Until { child: u16 },
    Invert { child: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    TooManyNodes(usize),
    BadChild(u16),
    Empty,
}

/// The leaf implementations of one bot.
pub trait Leaves {
    fn condition(&mut self, id: u16) -> bool;
    fn action(&mut self, id: u16) -> Status;
}

#[derive(Debug, Clone)]
pub struct Tree {
    nodes: Vec<Node>,
    children: Vec<u16>,
}

impl Tree {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Ticks the root (node 0). `tick` drives the cooldown clocks.
    pub fn tick(&self, bb: &mut Blackboard, leaves: &mut dyn Leaves, tick: u32) -> Status {
        self.run(0, bb, leaves, tick)
    }

    fn run(&self, id: u16, bb: &mut Blackboard, lv: &mut dyn Leaves, tick: u32) -> Status {
        let i = id as usize;
        match self.nodes[i] {
            Node::Condition(c) => {
                if lv.condition(c) {
                    Status::Success
                } else {
                    Status::Failure
                }
            }
            Node::Action(a) => lv.action(a),
            Node::Invert { child } => match self.run(child, bb, lv, tick) {
                Status::Success => Status::Failure,
                Status::Failure => Status::Success,
                Status::Running => Status::Running,
            },
            Node::Cooldown { child, ticks } => {
                if tick < bb.ready_at[i] {
                    return Status::Failure;
                }
                let s = self.run(child, bb, lv, tick);
                if s != Status::Running {
                    bb.ready_at[i] = tick.saturating_add(ticks);
                }
                s
            }
            Node::Until { child } => match self.run(child, bb, lv, tick) {
                Status::Success => Status::Success,
                _ => Status::Running,
            },
            Node::Sequence { first, len } => {
                let start = bb.cursor[i];
                for k in start..len {
                    let c = self.children[(first + k) as usize];
                    match self.run(c, bb, lv, tick) {
                        Status::Success => {}
                        Status::Failure => {
                            bb.cursor[i] = 0;
                            return Status::Failure;
                        }
                        Status::Running => {
                            bb.cursor[i] = k;
                            return Status::Running;
                        }
                    }
                }
                bb.cursor[i] = 0;
                Status::Success
            }
            Node::Selector { first, len } => {
                let start = bb.cursor[i];
                for k in start..len {
                    let c = self.children[(first + k) as usize];
                    match self.run(c, bb, lv, tick) {
                        Status::Failure => {}
                        Status::Success => {
                            bb.cursor[i] = 0;
                            return Status::Success;
                        }
                        Status::Running => {
                            bb.cursor[i] = k;
                            return Status::Running;
                        }
                    }
                }
                bb.cursor[i] = 0;
                Status::Failure
            }
            Node::Parallel { first, len, need } => {
                let (mut ok, mut fail) = (0u16, 0u16);
                for k in 0..len {
                    let c = self.children[(first + k) as usize];
                    match self.run(c, bb, lv, tick) {
                        Status::Success => ok += 1,
                        Status::Failure => fail += 1,
                        Status::Running => {}
                    }
                }
                if ok >= need {
                    Status::Success
                } else if len - fail < need {
                    Status::Failure
                } else {
                    Status::Running
                }
            }
        }
    }
}

/// Builds a tree bottom-up: add children first, then the parent referencing their ids.
/// The root must be added last; [`Builder::finish`] moves it to index 0.
#[derive(Debug, Default)]
pub struct Builder {
    nodes: Vec<Node>,
    children: Vec<u16>,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, n: Node) -> u16 {
        self.nodes.push(n);
        (self.nodes.len() - 1) as u16
    }

    fn group(&mut self, kids: &[u16]) -> (u16, u16) {
        let first = self.children.len() as u16;
        self.children.extend_from_slice(kids);
        (first, kids.len() as u16)
    }

    pub fn condition(&mut self, id: u16) -> u16 {
        self.push(Node::Condition(id))
    }
    pub fn action(&mut self, id: u16) -> u16 {
        self.push(Node::Action(id))
    }
    pub fn sequence(&mut self, kids: &[u16]) -> u16 {
        let (first, len) = self.group(kids);
        self.push(Node::Sequence { first, len })
    }
    pub fn selector(&mut self, kids: &[u16]) -> u16 {
        let (first, len) = self.group(kids);
        self.push(Node::Selector { first, len })
    }
    pub fn parallel(&mut self, need: u16, kids: &[u16]) -> u16 {
        let (first, len) = self.group(kids);
        self.push(Node::Parallel { first, len, need })
    }
    pub fn cooldown(&mut self, child: u16, ticks: u32) -> u16 {
        self.push(Node::Cooldown { child, ticks })
    }
    pub fn until(&mut self, child: u16) -> u16 {
        self.push(Node::Until { child })
    }
    pub fn invert(&mut self, child: u16) -> u16 {
        self.push(Node::Invert { child })
    }

    /// Validates the node cap and child ids, and makes `root` node 0.
    pub fn finish(mut self, root: u16) -> Result<Tree, TreeError> {
        let n = self.nodes.len();
        if n == 0 {
            return Err(TreeError::Empty);
        }
        if n > MAX_NODES {
            return Err(TreeError::TooManyNodes(n));
        }
        if root as usize >= n || self.children.iter().any(|&c| c as usize >= n) {
            return Err(TreeError::BadChild(root));
        }
        // Swap root into slot 0 and remap every reference.
        let remap = |x: u16| -> u16 {
            if x == root {
                0
            } else if x == 0 {
                root
            } else {
                x
            }
        };
        self.nodes.swap(0, root as usize);
        for c in &mut self.children {
            *c = remap(*c);
        }
        for node in &mut self.nodes {
            match node {
                Node::Cooldown { child, .. } | Node::Until { child } | Node::Invert { child } => {
                    if *child as usize >= n {
                        return Err(TreeError::BadChild(*child));
                    }
                    *child = remap(*child);
                }
                _ => {}
            }
        }
        Ok(Tree {
            nodes: self.nodes,
            children: self.children,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct L {
        conds: Vec<bool>,
        acts: Vec<Status>,
        calls: Vec<u16>,
    }
    impl Leaves for L {
        fn condition(&mut self, id: u16) -> bool {
            self.conds[id as usize]
        }
        fn action(&mut self, id: u16) -> Status {
            self.calls.push(id);
            self.acts[id as usize]
        }
    }
    fn leaves(conds: &[bool], acts: &[Status]) -> L {
        L {
            conds: conds.to_vec(),
            acts: acts.to_vec(),
            calls: vec![],
        }
    }

    #[test]
    fn sequence_stops_at_failure_and_resumes_running() {
        let mut b = Builder::new();
        let a0 = b.action(0);
        let a1 = b.action(1);
        let a2 = b.action(2);
        let root = b.sequence(&[a0, a1, a2]);
        let t = b.finish(root).unwrap();
        let mut bb = Blackboard::for_tree(&t);
        let mut l = leaves(&[], &[Status::Success, Status::Running, Status::Success]);
        assert_eq!(t.tick(&mut bb, &mut l, 0), Status::Running);
        assert_eq!(l.calls, [0, 1]);
        // Next tick resumes at child 1 (child 0 is not re-run).
        l.calls.clear();
        l.acts[1] = Status::Success;
        assert_eq!(t.tick(&mut bb, &mut l, 1), Status::Success);
        assert_eq!(l.calls, [1, 2]);
        l.acts[0] = Status::Failure;
        l.calls.clear();
        assert_eq!(t.tick(&mut bb, &mut l, 2), Status::Failure);
        assert_eq!(l.calls, [0]);
    }

    #[test]
    fn selector_takes_first_non_failure() {
        let mut b = Builder::new();
        let c = b.condition(0);
        let a0 = b.action(0);
        let guarded = b.sequence(&[c, a0]);
        let a1 = b.action(1);
        let root = b.selector(&[guarded, a1]);
        let t = b.finish(root).unwrap();
        let mut bb = Blackboard::for_tree(&t);
        let mut l = leaves(&[false], &[Status::Success, Status::Success]);
        assert_eq!(t.tick(&mut bb, &mut l, 0), Status::Success);
        assert_eq!(l.calls, [1]);
        l.conds[0] = true;
        l.calls.clear();
        assert_eq!(t.tick(&mut bb, &mut l, 1), Status::Success);
        assert_eq!(l.calls, [0]);
    }

    #[test]
    fn parallel_threshold() {
        let mut b = Builder::new();
        let a0 = b.action(0);
        let a1 = b.action(1);
        let root = b.parallel(2, &[a0, a1]);
        let t = b.finish(root).unwrap();
        let mut bb = Blackboard::for_tree(&t);
        let mut l = leaves(&[], &[Status::Success, Status::Running]);
        assert_eq!(t.tick(&mut bb, &mut l, 0), Status::Running);
        l.acts[1] = Status::Success;
        assert_eq!(t.tick(&mut bb, &mut l, 1), Status::Success);
        l.acts[0] = Status::Failure;
        assert_eq!(t.tick(&mut bb, &mut l, 2), Status::Failure);
    }

    #[test]
    fn cooldown_blocks_after_completion() {
        let mut b = Builder::new();
        let a = b.action(0);
        let root = b.cooldown(a, 10);
        let t = b.finish(root).unwrap();
        let mut bb = Blackboard::for_tree(&t);
        let mut l = leaves(&[], &[Status::Success]);
        assert_eq!(t.tick(&mut bb, &mut l, 5), Status::Success);
        for k in 6..15 {
            assert_eq!(t.tick(&mut bb, &mut l, k), Status::Failure, "tick {k}");
        }
        assert_eq!(t.tick(&mut bb, &mut l, 15), Status::Success);
        assert_eq!(l.calls.len(), 2);
    }

    #[test]
    fn until_loops_until_child_succeeds() {
        let mut b = Builder::new();
        let c = b.condition(0);
        let root = b.until(c);
        let t = b.finish(root).unwrap();
        let mut bb = Blackboard::for_tree(&t);
        let mut l = leaves(&[false], &[]);
        assert_eq!(t.tick(&mut bb, &mut l, 0), Status::Running);
        assert_eq!(t.tick(&mut bb, &mut l, 1), Status::Running);
        l.conds[0] = true;
        assert_eq!(t.tick(&mut bb, &mut l, 2), Status::Success);
    }

    #[test]
    fn invert_and_root_remap() {
        let mut b = Builder::new();
        let c = b.condition(0);
        let a = b.action(0);
        let inv = b.invert(c);
        // Root is not the last-added node: node 0 (the condition) must be remapped.
        let root = b.sequence(&[inv, a]);
        let t = b.finish(root).unwrap();
        let mut bb = Blackboard::for_tree(&t);
        let mut l = leaves(&[false], &[Status::Success]);
        assert_eq!(t.tick(&mut bb, &mut l, 0), Status::Success);
        l.conds[0] = true;
        assert_eq!(t.tick(&mut bb, &mut l, 1), Status::Failure);
    }

    #[test]
    fn node_cap_enforced() {
        let mut b = Builder::new();
        let kids: Vec<u16> = (0..MAX_NODES as u16).map(|i| b.action(i)).collect();
        let root = b.selector(&kids); // 81 nodes
        assert_eq!(b.finish(root).unwrap_err(), TreeError::TooManyNodes(81));
        let mut b = Builder::new();
        let kids: Vec<u16> = (0..(MAX_NODES as u16 - 1)).map(|i| b.action(i)).collect();
        let root = b.selector(&kids); // exactly 80
        assert!(b.finish(root).is_ok());
    }
}
