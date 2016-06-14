extern crate time;

use heap;
use heap::immix::ImmixMutatorLocal;
use heap::immix::ImmixSpace;
use heap::freelist::FreeListSpace;
use common::Address;

pub const K : usize          = 4;
pub const TREE_DEPTH : usize = 10; // 10
pub const TREE_COUNT : usize = 50; // 50

pub const OBJECT_SIZE : usize = K * 8;
pub const ACTUAL_OBJECT_SIZE : usize = K * 8;
pub const OBJECT_ALIGN : usize = 8;

#[inline(always)]
fn alloc_k_ary_tree(mutator: &mut ImmixMutatorLocal) -> Address {
    let addr = mutator.alloc(ACTUAL_OBJECT_SIZE, 8);
    mutator.init_object(addr, 0b1100_1111);
    addr
}

fn make_tree(depth: usize, mutator: &mut ImmixMutatorLocal) -> Address {
    if depth <= 0 {
        alloc_k_ary_tree(mutator)
    } else {
        let mut children = vec![];
        for _ in 0..K {
            children.push(make_tree(depth - 1, mutator));
        }
        
        let result = alloc_k_ary_tree(mutator);
//        println!("parent node: {:X}", result);
        
        let mut cursor = result;
        for _ in 0..K {
            let child = children.pop().unwrap();
//            println!("  child: {:X} at {:X}", child, cursor);
            unsafe {cursor.store::<Address>(child)};
            cursor = cursor.plus(8);
        }
        
        result
    }
}

#[allow(unused_variables)]
pub fn alloc_trace() {
    use std::sync::{Arc, RwLock};
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
    
    println!("Trying to allocate 1 object of (size {}, align {}). ", K * 8, 8);
    println!("Considering header size of {}, an object should be {}. ", 0, ACTUAL_OBJECT_SIZE);
    
    println!("Trying to allocate {} trees of depth {}, which is {} objects ({} bytes)", 
              TREE_COUNT, TREE_DEPTH, TREE_COUNT * K.pow(TREE_DEPTH as u32), TREE_COUNT * K.pow(TREE_DEPTH as u32) * ACTUAL_OBJECT_SIZE);
    
    let mut roots = vec![];
    
    for _ in 0..TREE_COUNT {
        roots.push(unsafe {make_tree(TREE_DEPTH, &mut mutator).to_object_reference()});
    }
    
    println!("Start tracing");
    
    let t_start = time::now_utc();
    heap::gc::start_trace(&mut roots, shared_space, lo_space);
    let t_end = time::now_utc();
    
    println!("time used: {} msec", (t_end - t_start).num_milliseconds());
}