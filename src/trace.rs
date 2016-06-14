extern crate time;

use heap;
use heap::immix::ImmixMutatorLocal;
use heap::immix::ImmixSpace;
use heap::freelist::FreeListSpace;
use common::Address;

use std::sync::{Arc};
use std::sync::atomic::Ordering;
use std::sync::RwLock;

use exhaust::OBJECT_SIZE;
use exhaust::OBJECT_ALIGN;
use exhaust::ALLOCATION_TIMES;

const TRACE_TIMES : usize = ALLOCATION_TIMES;

struct Node<'a> {
    hdr  : u64,
    next : &'a Node<'a>,
    unused_ptr : usize,
    unused_int : i32,
    unused_int2: i32
}

#[allow(unused_variables)]
pub fn alloc_trace() {
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
    
    println!("Trying to allocate 1 object of (size {}, align {}). ", OBJECT_SIZE, OBJECT_ALIGN);
    const ACTUAL_OBJECT_SIZE : usize = OBJECT_SIZE;
    println!("Considering header size of {}, an object should be {}. ", 0, ACTUAL_OBJECT_SIZE);
    
    println!("Trying to allocate {} objects, which will take roughly {} bytes", TRACE_TIMES, TRACE_TIMES * ACTUAL_OBJECT_SIZE);
    let root = mutator.alloc(ACTUAL_OBJECT_SIZE, OBJECT_ALIGN);
    mutator.init_object(root, 0b1100_0001);
    
    let mut prev = root;
    for _ in 0..TRACE_TIMES - 1 {
        let res = mutator.alloc(ACTUAL_OBJECT_SIZE, OBJECT_ALIGN);
        mutator.init_object(res, 0b1100_0001);
        
        // set prev's 1st field (offset 0) to this object
        unsafe {prev.store::<Address>(res)};
        
        prev = res;
    }
    
    trace_loop(root, shared_space, lo_space);
}

#[cfg(target_os = "linux")]
#[inline(never)]
fn trace_loop(root: Address, shared_space: Arc<ImmixSpace>, lo_space: Arc<RwLock<FreeListSpace>>) {
    use common::perf;    
    
    println!("Start tracing");
    let mut roots = vec![unsafe {root.to_object_reference()}];
    
    let perf = unsafe {perf::start_perf_events()};
    unsafe {perf::perf_read_values(perf);}
    let t_start = time::now_utc();
    
    heap::gc::start_trace(&mut roots, shared_space, lo_space);
    
    let t_end = time::now_utc();
    unsafe {perf::perf_read_values(perf);}
    
    println!("time used: {} msec", (t_end - t_start).num_milliseconds());
    unsafe {perf::perf_print(perf);}
}

#[cfg(not(target_os = "linux"))]
#[allow(unused_variables)]
fn trace_loop(root: Address, shared_space: Arc<ImmixSpace>, lo_space: Arc<RwLock<FreeListSpace>>) {
    unimplemented!()
}