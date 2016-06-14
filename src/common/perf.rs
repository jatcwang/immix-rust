extern crate libc;
use self::libc::c_void;

use heap::immix::ImmixMutatorLocal;

#[link(name = "gc_perf")]
#[link(name = "pfm")]
extern "C" {
    pub fn start_perf_events() -> *const c_void;
    pub fn perf_read_values(events: *const c_void);
    pub fn perf_print(events: *const c_void);
    
    pub fn cur_time() -> *const c_void;
    pub fn diff_in_ms(t_start: *const c_void, t_end: *const c_void) -> f64;
    
    pub fn nop(anything: *mut ImmixMutatorLocal);
}