use common::Address;
use std::sync::atomic::AtomicUsize;

pub mod immix;
pub mod freelist;
pub mod gc;

pub const ALIGNMENT_VALUE : u8 = 1;

pub const IMMIX_SPACE_RATIO : f64 = 1.0 - LO_SPACE_RATIO;
pub const LO_SPACE_RATIO : f64 = 0.2;
pub const DEFAULT_HEAP_SIZE : usize = 500 << 20;

lazy_static! {
    pub static ref IMMIX_SPACE_SIZE : AtomicUsize = AtomicUsize::new( (DEFAULT_HEAP_SIZE as f64 * IMMIX_SPACE_RATIO) as usize );
    pub static ref LO_SPACE_SIZE : AtomicUsize = AtomicUsize::new( (DEFAULT_HEAP_SIZE as f64 * LO_SPACE_RATIO) as usize );
}

#[inline(always)]
pub fn fill_alignment_gap(start : Address, end : Address) -> () {
    debug_assert!(end >= start);
    start.memset(ALIGNMENT_VALUE, end.diff(start));
}