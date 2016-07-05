use heap::immix::MUTATORS;
use heap::immix::N_MUTATORS;
use heap::immix::ImmixMutatorLocal;
use heap::immix::ImmixSpace;
use heap::immix::ImmixLineMarkTable;
use heap::freelist::FreeListSpace;
use objectmodel;

use common;
use common::{Address, ObjectReference};
use common::AddressMap;

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, Condvar, RwLock};

extern crate crossbeam;

#[cfg(feature = "mt-trace")]
use self::crossbeam::sync::chase_lev::*;
#[cfg(feature = "mt-trace")]
use std::sync::mpsc;
#[cfg(feature = "mt-trace")]
use std::sync::mpsc::channel;
#[cfg(feature = "mt-trace")]
use std::thread;

use std::sync::atomic;

lazy_static! {
    static ref STW_COND : Arc<(Mutex<usize>, Condvar)> = {
        Arc::new((Mutex::new(0), Condvar::new()))
    };
    
    static ref GET_ROOTS : RwLock<Box<Fn()->Vec<ObjectReference> + Sync + Send>> = RwLock::new(Box::new(get_roots));
    
    static ref GC_CONTEXT : RwLock<GCContext> = RwLock::new(GCContext{immix_space: None, lo_space: None});
    
    static ref ROOTS : RwLock<Vec<ObjectReference>> = RwLock::new(vec![]);
}

static CONTROLLER : AtomicIsize = atomic::ATOMIC_ISIZE_INIT;
const  NO_CONTROLLER : isize    = -1;

pub struct GCContext {
    immix_space : Option<Arc<ImmixSpace>>,
    lo_space    : Option<Arc<RwLock<FreeListSpace>>>
}

fn get_roots() -> Vec<ObjectReference> {
    vec![]
}

pub fn init(immix_space: Arc<ImmixSpace>, lo_space: Arc<RwLock<FreeListSpace>>) {
    CONTROLLER.store(NO_CONTROLLER, Ordering::SeqCst);
    let mut gccontext = GC_CONTEXT.write().unwrap();
    gccontext.immix_space = Some(immix_space);
    gccontext.lo_space = Some(lo_space);
}

pub fn init_get_roots(get_roots: Box<Fn()->Vec<ObjectReference> + Sync + Send>) {
    *GET_ROOTS.write().unwrap() = get_roots;
}

pub fn trigger_gc() {
    trace!("Triggering GC...");
    
    for mut m in MUTATORS.write().unwrap().iter_mut() {
        if m.is_some() {
            m.as_mut().unwrap().set_take_yield(true);
        }
    }
}

extern crate libc;
#[cfg(target_arch = "x86_64")]
#[link(name = "gc_clib_x64")]
extern "C" {
    pub fn malloc_zero(size: libc::size_t) -> *const libc::c_void;
    fn immmix_get_stack_ptr() -> Address;
    pub fn set_low_water_mark();
    fn get_low_water_mark() -> Address;
    fn get_registers() -> *const Address;
    fn get_registers_count() -> i32;
}

#[inline(always)]
pub fn is_valid_object(addr: Address, start: Address, end: Address, live_map: &AddressMap<u8>) -> bool {
    if addr >= end || addr < start {
        return false;
    }
    
    common::test_nth_bit(live_map.get(addr), objectmodel::OBJ_START_BIT)
}

pub fn stack_scan() -> Vec<ObjectReference> {
    let stack_ptr : Address = unsafe {immmix_get_stack_ptr()};
    let low_water_mark : Address = unsafe {get_low_water_mark()};
    
    let mut cursor = stack_ptr;
    let mut ret = vec![];
    
    let gccontext = GC_CONTEXT.read().unwrap();
    let immix_space = gccontext.immix_space.as_ref().unwrap();
    
    while cursor < low_water_mark {
        let value : Address = unsafe {cursor.load::<Address>()};
        
        if is_valid_object(value, immix_space.start(), immix_space.end(), &immix_space.alloc_map) {
            ret.push(unsafe {value.to_object_reference()});
        }
        
        cursor = cursor.plus(common::POINTER_SIZE);
    }
    
    let roots_from_stack = ret.len();
    
    let registers_count = unsafe {get_registers_count()};
    let registers = unsafe {get_registers()};
    
    for i in 0..registers_count {
        let value = unsafe {*registers.offset(i as isize)};
        
        if is_valid_object(value, immix_space.start(), immix_space.end(), &immix_space.alloc_map) {
            ret.push(unsafe {value.to_object_reference()});
        }
    }
    
    let roots_from_registers = ret.len() - roots_from_stack;
    
    trace!("roots: {} from stack, {} from registers", roots_from_stack, roots_from_registers);
    
    ret
}

#[inline(never)]
pub fn sync_barrier(mutator: &mut ImmixMutatorLocal) {
    let controller_id = CONTROLLER.compare_and_swap(-1, mutator.id() as isize, Ordering::SeqCst);
    
    trace!("Mutator{} saw the controller is {}", mutator.id(), controller_id);
    
    // prepare the mutator for gc - return current block (if it has)
    mutator.prepare_for_gc();
    
    // scan its stack
    let mut thread_roots = stack_scan();
    ROOTS.write().unwrap().append(&mut thread_roots);
    
    // user thread call back to prepare for gc
//    USER_THREAD_PREPARE_FOR_GC.read().unwrap()();
    
    if controller_id != NO_CONTROLLER {
        // this thread will block
        block_current_thread();
        
        // reset current mutator
        mutator.reset();
    } else {
        // this thread is controller
        // other threads should block
        
        // wait for all mutators to be blocked
        let &(ref lock, ref cvar) = &*STW_COND.clone();
        let mut count = 0;
        
        trace!("expect {} mutators to park", *N_MUTATORS.read().unwrap() - 1);
        while count < *N_MUTATORS.read().unwrap() - 1 {
            let new_count = {*lock.lock().unwrap()};
            if new_count != count {				
                count = new_count;
                trace!("count = {}", count);
            }
        }
        
        trace!("everyone stopped, gc will start");
        
        // roots->trace->sweep
        gc();
        
        // mutators will resume
        CONTROLLER.store(NO_CONTROLLER, Ordering::SeqCst);
        for mut t in MUTATORS.write().unwrap().iter_mut() {
            if t.is_some() {
                t.as_mut().unwrap().set_take_yield(false);
            }
        }
        // every mutator thread will reset themselves, so only reset current mutator here
        mutator.reset();

        // resume
        {
            let mut count = lock.lock().unwrap();
            *count = 0;
            cvar.notify_all();
        }
    }
}

fn block_current_thread() {
    let &(ref lock, ref cvar) = &*STW_COND.clone();
    let mut count = lock.lock().unwrap();
    *count += 1;
    
    while *count != 0 {
        count = cvar.wait(count).unwrap();
    }
}

pub static GC_COUNT : atomic::AtomicUsize = atomic::ATOMIC_USIZE_INIT;

fn gc() {
    GC_COUNT.store(GC_COUNT.load(atomic::Ordering::SeqCst) + 1, atomic::Ordering::SeqCst);
    
    trace!("GC starts");
    
    // creates root deque
    let mut roots : &mut Vec<ObjectReference> = &mut ROOTS.write().unwrap();
    
    // mark & trace
    {
        let gccontext = GC_CONTEXT.read().unwrap();
        let (immix_space, lo_space) = (gccontext.immix_space.as_ref().unwrap(), gccontext.lo_space.as_ref().unwrap());
        
        start_trace(&mut roots, immix_space.clone(), lo_space.clone());
    }
    
    trace!("trace done");
    
    // sweep
    {
        let mut gccontext = GC_CONTEXT.write().unwrap();
        let immix_space = gccontext.immix_space.as_mut().unwrap();
        
        immix_space.sweep();
    }
    
    objectmodel::flip_mark_state();
    trace!("GC finishes");
}

pub const MULTI_THREAD_TRACE_THRESHOLD : usize = 10;

pub const PUSH_BACK_THRESHOLD : usize = 50;
pub static GC_THREADS : atomic::AtomicUsize = atomic::ATOMIC_USIZE_INIT;

#[allow(unused_variables)]
#[inline(never)]
#[cfg(feature = "mt-trace")]
pub fn start_trace(work_stack: &mut Vec<ObjectReference>, immix_space: Arc<ImmixSpace>, lo_space: Arc<RwLock<FreeListSpace>>) {
    // creates root deque
    let (mut worker, stealer) = deque();
    
    while !work_stack.is_empty() {
        worker.push(work_stack.pop().unwrap());
    }

    loop {
        let (sender, receiver) = channel::<ObjectReference>();        
        
        let mut gc_threads = vec![];
        for _ in 0..GC_THREADS.load(atomic::Ordering::SeqCst) {
            let new_immix_space = immix_space.clone();
            let new_lo_space = lo_space.clone();
            let new_stealer = stealer.clone();
            let new_sender = sender.clone();
            let t = thread::spawn(move || {
                start_steal_trace(new_stealer, new_sender, new_immix_space, new_lo_space);
            });
            gc_threads.push(t);
        }
        
        // only stealers own sender, when all stealers quit, the following loop finishes
        drop(sender);
        
        loop {
            let recv = receiver.recv();
            match recv {
                Ok(obj) => worker.push(obj),
                Err(_) => break
            }
        }
        
        match worker.try_pop() {
            Some(obj_ref) => worker.push(obj_ref),
            None => break
        }
    }
}

#[allow(unused_variables)]
#[inline(never)]
#[cfg(not(feature = "mt-trace"))]
pub fn start_trace(local_queue: &mut Vec<ObjectReference>, immix_space: Arc<ImmixSpace>, lo_space: Arc<RwLock<FreeListSpace>>) {
    use objectmodel;
    let mark_state = objectmodel::MARK_STATE.load(Ordering::SeqCst) as u8;
    
    while !local_queue.is_empty() {
        trace_object(local_queue.pop().unwrap(), local_queue, immix_space.alloc_map.ptr, immix_space.trace_map.ptr, &immix_space.line_mark_table, immix_space.start(), immix_space.end(), mark_state);
    }    
}

#[allow(unused_variables)]
#[cfg(feature = "mt-trace")]
fn start_steal_trace(stealer: Stealer<ObjectReference>, job_sender:mpsc::Sender<ObjectReference>, immix_space: Arc<ImmixSpace>, lo_space: Arc<RwLock<FreeListSpace>>) {
    use objectmodel;
    
    let mut local_queue = vec![];
    
    let line_mark_table = &immix_space.line_mark_table;
    let (alloc_map, trace_map) = (immix_space.alloc_map.ptr, immix_space.trace_map.ptr);
    let (space_start, space_end) = (immix_space.start(), immix_space.end());
    let mark_state = objectmodel::MARK_STATE.load(Ordering::SeqCst) as u8;
    
    loop {
        let work = {
            if !local_queue.is_empty() {
                local_queue.pop().unwrap()
            } else {
                let work = stealer.steal();
                match work {
                    Steal::Empty => return,
                    Steal::Abort => continue,
                    Steal::Data(obj) => obj
                }
            }
        };
        
        steal_trace_object(work, &mut local_queue, &job_sender, alloc_map, trace_map, line_mark_table, space_start, space_end, mark_state, &lo_space);
    }
} 

#[inline(always)]
#[cfg(feature = "mt-trace")]
pub fn steal_trace_object(obj: ObjectReference, local_queue: &mut Vec<ObjectReference>, job_sender: &mpsc::Sender<ObjectReference>, alloc_map: *mut u8, trace_map: *mut u8, line_mark_table: &ImmixLineMarkTable, immix_start: Address, immix_end: Address, mark_state: u8, lo_space: &Arc<RwLock<FreeListSpace>>) {
    use objectmodel;
    
    objectmodel::mark_as_traced(trace_map, immix_start, obj, mark_state);
    
    let addr = obj.to_address();
    
    if addr >= immix_start && addr < immix_end {
        line_mark_table.mark_line_live(addr);        
    } else {
        // freelist mark
    }
    
    let mut base = addr;
    loop {
        let value = objectmodel::get_ref_byte(alloc_map, immix_start, obj);
        let (ref_bits, short_encode) = (common::lower_bits(value, objectmodel::REF_BITS_LEN), common::test_nth_bit(value, objectmodel::SHORT_ENCODE_BIT));
        match ref_bits {
            0b0000_0001 => {
                steal_process_edge(base, local_queue, trace_map, immix_start, job_sender, mark_state);
            },            
            0b0000_0011 => {
                steal_process_edge(base, local_queue, trace_map, immix_start, job_sender, mark_state);
                steal_process_edge(base.plus(8), local_queue, trace_map, immix_start, job_sender, mark_state);
            },
            0b0000_1111 => {
                steal_process_edge(base, local_queue, trace_map, immix_start, job_sender, mark_state);
                steal_process_edge(base.plus(8), local_queue, trace_map, immix_start, job_sender, mark_state);
                steal_process_edge(base.plus(16), local_queue, trace_map, immix_start, job_sender, mark_state);
                steal_process_edge(base.plus(24), local_queue, trace_map, immix_start, job_sender, mark_state);                
            },            
            _ => {
                panic!("unexpcted ref_bits patterns: {:b}", ref_bits);
            }
        }
        
        assert!(short_encode);
        if short_encode {
            return;
        } else {
            base = base.plus(objectmodel::REF_BITS_LEN * 8);
        } 
    }
}

#[inline(always)]
#[cfg(feature = "mt-trace")]
pub fn steal_process_edge(addr: Address, local_queue:&mut Vec<ObjectReference>, trace_map: *mut u8, immix_start: Address, job_sender: &mpsc::Sender<ObjectReference>, mark_state: u8) {
    use objectmodel;
    
    let obj_addr = unsafe{addr.load::<ObjectReference>()};

    if !obj_addr.to_address().is_zero() && !objectmodel::is_traced(trace_map, immix_start, obj_addr, mark_state) {
        if local_queue.len() >= PUSH_BACK_THRESHOLD {
            job_sender.send(obj_addr).unwrap();
        } else {
            local_queue.push(obj_addr);
        }
    } 
}

#[inline(always)]
pub fn trace_object(obj: ObjectReference, local_queue: &mut Vec<ObjectReference>, alloc_map: *mut u8, trace_map: *mut u8, line_mark_table: &ImmixLineMarkTable, immix_start: Address, immix_end: Address, mark_state: u8) {
    use objectmodel;
    
    objectmodel::mark_as_traced(trace_map, immix_start, obj, mark_state);

    let addr = obj.to_address();
    
    if addr >= immix_start && addr < immix_end {
        line_mark_table.mark_line_live(addr);        
    } else {
        // freelist mark
    }
    
    let mut base = addr;
    loop {
        let value = objectmodel::get_ref_byte(alloc_map, immix_start, obj);
        let (ref_bits, short_encode) = (common::lower_bits(value, objectmodel::REF_BITS_LEN), common::test_nth_bit(value, objectmodel::SHORT_ENCODE_BIT));
        
        match ref_bits {
            0b0000_0001 => {
                process_edge(base, local_queue, trace_map, immix_start, mark_state);
            },            
            0b0000_0011 => {
                process_edge(base, local_queue, trace_map, immix_start, mark_state);
                process_edge(base.plus(8), local_queue, trace_map, immix_start, mark_state);
            },
            0b0000_1111 => {
                process_edge(base, local_queue, trace_map, immix_start, mark_state);
                process_edge(base.plus(8), local_queue, trace_map, immix_start, mark_state);
                process_edge(base.plus(16), local_queue, trace_map, immix_start, mark_state);
                process_edge(base.plus(24), local_queue, trace_map, immix_start, mark_state);                
            },
            _ => {
                panic!("unexpcted ref_bits patterns: {:b}", ref_bits);
            }
        }
        
        debug_assert!(short_encode);
        if short_encode {
            return;
        } else {
            base = base.plus(objectmodel::REF_BITS_LEN * 8);
        } 
    }
}

#[inline(always)]
pub fn process_edge(addr: Address, local_queue:&mut Vec<ObjectReference>, trace_map: *mut u8, space_start: Address, mark_state: u8) {
    use objectmodel;
    
    let obj_addr : ObjectReference = unsafe{addr.load()};

    if !obj_addr.to_address().is_zero() && !objectmodel::is_traced(trace_map, space_start, obj_addr, mark_state) {
        local_queue.push(obj_addr);
    }
}