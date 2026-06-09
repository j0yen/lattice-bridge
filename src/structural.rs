//! Structural matching: agreement of already-mapped parents/children.
//!
//! This is a stub for the current iteration. The score is always 0.0 until
//! a full iterative fixpoint loop is implemented in a future tick.

use crate::anchor::BfoAnchor;

/// Compute a structural score in [0, 1] for a candidate mapping (a_idx → b_idx).
///
/// Currently a stub that always returns 0.0.  A future implementation will
/// look at already-committed mappings and check whether parents of `a` map to
/// parents of `b`.
#[allow(unused_variables)]
pub fn score(
    a: &BfoAnchor,
    b: &BfoAnchor,
    _a_all: &[BfoAnchor],
    _b_all: &[BfoAnchor],
    _committed: &[(usize, usize)],
) -> f64 {
    0.0
}
