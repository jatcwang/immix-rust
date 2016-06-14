extern crate libc;

use heap;
use heap::immix::ImmixMutatorLocal;
use heap::immix::ImmixSpace;
use heap::freelist::FreeListSpace;

use std::sync::RwLock;

pub const OBJECT_SIZE : usize = 24;
pub const OBJECT_ALIGN: usize = 8;

pub const ALLOCATION_TIMES : usize = 50000000;

#[allow(unused_variables)]
pub fn exhaust_alloc() {
    use std::sync::{Arc};
    use std::sync::atomic::Ordering;
    
    let shared_space : Arc<ImmixSpace> = {
        let space : ImmixSpace = ImmixSpace::new(heap::IMMIX_SPACE_SIZE.load(Ordering::SeqCst));
        
        Arc::new(space)
    };
    let lo_space : Arc<RwLock<FreeListSpace>> = {
        let space : FreeListSpace = FreeListSpace::new(heap::LO_SPACE_SIZE.load(Ordering::SeqCst));
        Arc::new(RwLock::new(space))
    };
    heap::gc::init(shared_space.clone(), lo_space.clone());

    let mut mutator = ImmixMutatorLocal::new(shared_space.clone());
    
    println!("Trying to allocate {} objects of (size {}, align {}). ", ALLOCATION_TIMES, OBJECT_SIZE, OBJECT_ALIGN);
    const ACTUAL_OBJECT_SIZE : usize = OBJECT_SIZE;
    println!("Considering header size of {}, an object should be {}. ", 0, ACTUAL_OBJECT_SIZE);
    println!("This would take {} bytes of {} bytes heap", ALLOCATION_TIMES * ACTUAL_OBJECT_SIZE, heap::IMMIX_SPACE_SIZE.load(Ordering::SeqCst));
    
    alloc_loop(&mut mutator);
}

#[cfg(target_os = "linux")]
#[inline(never)]
fn alloc_loop(mutator: &mut ImmixMutatorLocal) {
    use common::perf;
    
    let perf = unsafe {perf::start_perf_events()};
    unsafe {perf::perf_read_values(perf);}
    let t_start = unsafe {perf::cur_time()};
    
    for _ in 0..ALLOCATION_TIMES {
//        mutator.yieldpoint();
        
        let res = mutator.alloc(OBJECT_SIZE, OBJECT_ALIGN);
        mutator.init_object(res, 0b1100_0011);  
    }
    
    let t_end = unsafe {perf::cur_time()};
    unsafe {perf::perf_read_values(perf);}
    
    println!("time used: {} msec", unsafe {perf::diff_in_ms(t_start, t_end)});
    unsafe {perf::perf_print(perf);}
}

#[cfg(not(target_os = "linux"))]
#[allow(unused_variables)]
fn alloc_loop(mutator: &mut ImmixMutatorLocal) {
    unimplemented!()  
}