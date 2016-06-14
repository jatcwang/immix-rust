#![allow(dead_code)]
use std::env;
use std::sync::atomic::Ordering;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod common;
mod objectmodel;
mod heap;

mod exhaust;
mod mark;
mod trace;
mod mt_trace;
mod gcbench;
mod mt_gcbench;
mod obj_init;

fn init() {
    objectmodel::init();
}

fn main() {
    use heap;
    init();
    
    match env::var("HEAP_SIZE") {
        Ok(val) => {
            if val.ends_with("M") {
                let (num, _) = val.split_at(val.len() - 1);
                let heap_size = num.parse::<usize>().unwrap() << 20;
                
                let immix_space_size : usize = (heap_size as f64 * heap::IMMIX_SPACE_RATIO) as usize;
                heap::IMMIX_SPACE_SIZE.store(immix_space_size, Ordering::SeqCst);
                
                let lo_space_size : usize = (heap_size as f64 * heap::LO_SPACE_RATIO) as usize;
                heap::LO_SPACE_SIZE.store(lo_space_size, Ordering::SeqCst);
                 
                println!("heap is {} bytes (immix: {} bytes, lo: {} bytes) . ", heap_size, immix_space_size, lo_space_size);
            } else {
                println!("unknow heap size variable: {}, ignore", val);
                println!("using default heap size: {} bytes. ", heap::IMMIX_SPACE_SIZE.load(Ordering::SeqCst));
            }
        },
        Err(_) => {
            println!("using default heap size: {} bytes. ", heap::IMMIX_SPACE_SIZE.load(Ordering::SeqCst));
        }
    }
    
    match env::var("N_GCTHREADS") {
        Ok(val) => {
            heap::gc::GC_THREADS.store(val.parse::<usize>().unwrap(), Ordering::SeqCst);
        },
        Err(_) => {
            heap::gc::GC_THREADS.store(8, Ordering::SeqCst);
        }
    }
    
    if cfg!(feature = "exhaust") {
        exhaust::exhaust_alloc();
    } else if cfg!(feature = "initobj") {
        obj_init::alloc_init();
    } else if cfg!(feature = "gcbench") {
        gcbench::start();
    } else if cfg!(feature = "mt-gcbench") {
        mt_gcbench::start();
    } else if cfg!(feature = "mark") {
        mark::alloc_mark();
    } else if cfg!(feature = "trace") {
        trace::alloc_trace();
    } else if cfg!(feature = "mt-trace") {
        mt_trace::alloc_trace();
    }
    else {
        println!("unknown features: build with 'cargo build --release --features \"exhaust\"");
    }
}
