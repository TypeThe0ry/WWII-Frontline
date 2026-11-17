use std::collections::HashMap; // purity-allow: lookup only, never iterated

/* HashSet in a block
   comment, also fine */
pub fn ok() -> HashMap<u32, u32> { // purity-allow: return type of the waived lookup
    HashMap::new() // purity-allow: see above
}
