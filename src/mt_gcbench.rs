#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#![allow(unused_variables)]

use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use std::thread;

use heap;
use heap::immix::ImmixMutatorLocal;
use heap::immix::ImmixSpace;
use heap::freelist;
use heap::freelist::FreeListSpace;
use std::mem::size_of as size_of;

extern crate time;

const kStretchTreeDepth   : i32 = 18;
const kLongLivedTreeDepth : i32 = 16;
const kArraySize          : i32 = 500000;
const kMinTreeDepth       : i32 = 4;
const kMaxTreeDepth       : i32 = 16;

struct Node {
    hdr : u64,
    left : *mut Node,
    right : *mut Node,
    i : i32,
    j : i32
}

struct Array {
    hdr : u64,
    value : [f64; kArraySize as usize]
}

fn init_Node(me: *mut Node, l: *mut Node, r: *mut Node) {
    unsafe {
        (*me).left = l;
        (*me).right = r;
    }
}

fn TreeSize(i: i32) -> i32{
    (1 << (i + 1)) - 1
}

fn NumIters(i: i32) -> i32 {
    2 * TreeSize(kStretchTreeDepth) / TreeSize(i)
}

fn Populate(iDepth: i32, thisNode: *mut Node, mutator: &mut ImmixMutatorLocal) {
    if iDepth <= 0 {
        return;
    } else {
        unsafe {
            (*thisNode).left = alloc(mutator);
            (*thisNode).right = alloc(mutator);
            Populate(iDepth - 1, (*thisNode).left, mutator);
            Populate(iDepth - 1, (*thisNode).right, mutator);
        }
    }
}

fn MakeTree(iDepth: i32, mutator: &mut ImmixMutatorLocal) -> *mut Node {
    if iDepth <= 0 {
        alloc(mutator)
    } else {
        let left = MakeTree(iDepth - 1, mutator);
        let right = MakeTree(iDepth - 1, mutator);
        let result = alloc(mutator);
        init_Node(result, left, right);
        
        result
    }
}

fn PrintDiagnostics() {
    
}

fn TimeConstruction(depth: i32, mutator: &mut ImmixMutatorLocal) {
    let iNumIters = NumIters(depth);
    println!("creating {} trees of depth {}", iNumIters, depth);
    
    let tStart = time::now_utc();
    for _ in 0..iNumIters {
        let tempTree = alloc(mutator);
        Populate(depth, tempTree, mutator);
        
        // destroy tempTree
    }
    let tFinish = time::now_utc();
    println!("\tTop down construction took {} msec", (tFinish - tStart).num_milliseconds());
    
    let tStart = time::now_utc();
    for _ in 0..iNumIters {
        let tempTree = MakeTree(depth, mutator);
    }
    let tFinish = time::now_utc();
    println!("\tButtom up construction took {} msec", (tFinish - tStart).num_milliseconds());
}

fn run_one_test(immix_space: Arc<ImmixSpace>, lo_space: Arc<RwLock<FreeListSpace>>) {
    unsafe {heap::gc::set_low_water_mark();}
    let mut mutator = ImmixMutatorLocal::new(immix_space);
    
    let mut d = kMinTreeDepth;
    while d <= kMaxTreeDepth {
        TimeConstruction(d, &mut mutator);
        d += 2;
    }
    
    mutator.destroy();
}

#[inline(always)]
fn alloc(mutator: &mut ImmixMutatorLocal) -> *mut Node {
    let addr = mutator.alloc(size_of::<Node>(), 8);
    mutator.init_object(addr, 0b1100_0011);
    addr.to_ptr_mut::<Node>()
}

use std::env;

pub fn start() {
    unsafe {heap::gc::set_low_water_mark();}
    
    let n_threads : i32 = {
        let args : Vec<_> = env::args().collect();
        if args.len() > 1 {
            i32::from_str_radix(&args[1], 10).unwrap()
        } else {
            8
        }
    };
    
    let immix_space : Arc<ImmixSpace> = {
        let space : ImmixSpace = ImmixSpace::new(heap::IMMIX_SPACE_SIZE.load(Ordering::SeqCst));
        Arc::new(space)
    };
    let lo_space : Arc<RwLock<FreeListSpace>> = {
        let space : FreeListSpace = FreeListSpace::new(heap::LO_SPACE_SIZE.load(Ordering::SeqCst));
        Arc::new(RwLock::new(space))
    };
    heap::gc::init(immix_space.clone(), lo_space.clone());
    
    let mut mutator = ImmixMutatorLocal::new(immix_space.clone());
    
    println!("Garbage Collector Test");
    println!(" Live storage will peak at {} bytes.\n", 
        2 * (size_of::<Node>() as i32) * n_threads * TreeSize(kLongLivedTreeDepth) + 
        (size_of::<Array>() as i32));
    
    println!(" Stretching memory with a binary tree or depth {}", kStretchTreeDepth);
    PrintDiagnostics();
    
    let tStart = time::now_utc();
    
    // Stretch the memory space quickly
    let tempTree = MakeTree(kStretchTreeDepth, &mut mutator);
    // destroy tree
    
    println!(" Creating a long-lived binary tree of depth {}", kLongLivedTreeDepth);
    let longLivedTree = alloc(&mut mutator);
    Populate(kLongLivedTreeDepth, longLivedTree, &mut mutator);
    
    println!(" Creating a long-lived array of {} doubles", kArraySize);
    freelist::alloc_large(size_of::<Array>(), 8, &mut mutator, lo_space.clone());
  
    let mut threads = vec![];
    for i in 0..n_threads {
        let immix_space_clone = immix_space.clone();
        let lo_space_clone = lo_space.clone();
        let t = thread::spawn(move || {
            run_one_test(immix_space_clone, lo_space_clone);
        });
        threads.push(t);
    }
    
    // run one test locally
    let mut d = kMinTreeDepth;
    while d <= kMaxTreeDepth {
        TimeConstruction(d, &mut mutator);
        d += 2;
    }      
  
    mutator.destroy();
    
    for t in threads {
        t.join().unwrap();
    }
    
    if longLivedTree.is_null() {
        println!("Failed(long lived tree wrong)");
    }
    
    let tFinish = time::now_utc();
    let tElapsed = (tFinish - tStart).num_milliseconds();
    
    PrintDiagnostics();
    println!("Completed in {} msec", tElapsed);
    println!("Finished with {} collections", heap::gc::GC_COUNT.load(Ordering::SeqCst)); 
}