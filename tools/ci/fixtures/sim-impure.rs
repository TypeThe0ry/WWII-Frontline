use std::collections::HashMap; // violation: unordered iteration
use std::time::Instant;

// A comment mentioning HashMap is fine.
pub fn bad() -> HashMap<u32, u32> {
    let _t = Instant::now();
    let mut rng = rand::thread_rng();
    HashMap::new()
}
