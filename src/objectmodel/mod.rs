use std::sync::atomic;
pub static MARK_STATE : atomic::AtomicUsize = atomic::ATOMIC_USIZE_INIT;

use common::ObjectReference;

pub fn init() {
    MARK_STATE.store(1, atomic::Ordering::SeqCst);
}

pub fn flip_mark_state() {
    let mark_state = MARK_STATE.load(atomic::Ordering::SeqCst);
    if mark_state == 0 {
        MARK_STATE.store(1, atomic::Ordering::SeqCst);
    } else {
        MARK_STATE.store(0, atomic::Ordering::SeqCst);
    }
}

use common::Address;
use common::LOG_POINTER_SIZE;

#[inline(always)]
pub fn mark_as_traced(trace_map: *mut u8, space_start: Address, obj: ObjectReference, mark_state: u8) {
    unsafe {
        *trace_map.offset((obj.to_address().diff(space_start) >> LOG_POINTER_SIZE) as isize) = mark_state;
    }
}

#[inline(always)]
pub fn is_traced(trace_map: *mut u8, space_start: Address, obj: ObjectReference, mark_state: u8) -> bool {
    unsafe {
        (*trace_map.offset((obj.to_address().diff(space_start) >> LOG_POINTER_SIZE) as isize)) == mark_state
    }
}

pub const REF_BITS_LEN    : usize = 6;
pub const OBJ_START_BIT   : usize = 6;
pub const SHORT_ENCODE_BIT : usize = 7;

#[inline(always)]
pub fn get_ref_byte(alloc_map:*mut u8, space_start: Address, obj: ObjectReference) -> u8 {
    unsafe {*alloc_map.offset((obj.to_address().diff(space_start) >> LOG_POINTER_SIZE) as isize)}
}