use heap::immix;
use heap::immix::ImmixSpace;
use heap::immix::immix_space::ImmixBlock;
use heap::gc;

use common::LOG_POINTER_SIZE;
use common::Address;

use std::*;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_MUTATORS : usize = 1024;
lazy_static! {
    pub static ref MUTATORS : RwLock<Vec<Option<Arc<ImmixMutatorGlobal>>>> = {
        let mut ret = Vec::with_capacity(MAX_MUTATORS);
        for _ in 0..MAX_MUTATORS {
            ret.push(None);
        }
        RwLock::new(ret)
    };
    
    pub static ref N_MUTATORS : RwLock<usize> = RwLock::new(0);
}

#[repr(C)]
pub struct ImmixMutatorLocal {
    id        : usize,
    
    // use raw pointer here instead of AddressMapTable
    // to avoid indirection in fast path    
    alloc_map : *mut u8,
    space_start: Address,
    
    // cursor might be invalid, but Option<Address> is expensive here
    // after every GC, we set both cursor and limit
    // to Address::zero() so that alloc will branch to slow path    
    cursor    : Address,
    limit     : Address,
    line      : usize,
    
    space     : Arc<ImmixSpace>,
    block     : Option<Box<ImmixBlock>>,
    
    // globally accessible per-thread fields
    global    : Arc<ImmixMutatorGlobal>
}

pub struct ImmixMutatorGlobal {
    take_yield : AtomicBool
}

impl ImmixMutatorLocal {
    pub fn reset(&mut self) -> () {
        unsafe {
            // should not use Address::zero() other than initialization
            self.cursor = Address::zero();
            self.limit = Address::zero();
        }
        self.line = immix::LINES_IN_BLOCK;
        
        self.block = None;
    }
    
    pub fn new(space : Arc<ImmixSpace>) -> ImmixMutatorLocal {
        let global = Arc::new(ImmixMutatorGlobal::new());
        
        let mut id_lock = N_MUTATORS.write().unwrap();
        {
            let mut mutators_lock = MUTATORS.write().unwrap();
            mutators_lock.remove(*id_lock);
            mutators_lock.insert(*id_lock, Some(global.clone()));
        }
        
        let ret = ImmixMutatorLocal {
            id : *id_lock,
            cursor: unsafe {Address::zero()}, limit: unsafe {Address::zero()}, line: immix::LINES_IN_BLOCK,
            block: None,
            alloc_map: space.alloc_map.ptr,
            space_start: space.start(),
            global: global,
            space: space, 
        };
        *id_lock += 1;
        
        ret
    }
    
    pub fn destroy(&mut self) {
        {
            self.return_block();
        }
        
        let mut mutator_count_lock = N_MUTATORS.write().unwrap();
        
        let mut mutators_lock = MUTATORS.write().unwrap();
        mutators_lock.push(None);
        mutators_lock.swap_remove(self.id);
        
        *mutator_count_lock = *mutator_count_lock - 1;
        
        if cfg!(debug_assertions) {
            println!("destroy mutator. Now live mutators = {}", *mutator_count_lock);
        }
    }
    
    #[inline(always)]
    pub fn yieldpoint(&mut self) {
        if self.global.take_yield() {
            self.yieldpoint_slow();
        }
    }
    
    #[inline(never)]
    fn yieldpoint_slow(&mut self) {
        gc::sync_barrier(self);
    }
    
    #[inline(always)]
    pub fn alloc(&mut self, size: usize, align: usize) -> Address {
        // println!("Fastpath allocation");
        let start = self.cursor.align_up(align);
        let end = start.plus(size);
        
        // println!("cursor = {:#X}, after align = {:#X}", c, start);
        
        if end > self.limit {
            self.try_alloc_from_local(size, align)
        } else {
//            fill_alignment_gap(self.cursor, start);
            self.cursor = end;
            
            start            
        } 
    }
    
    #[inline(always)]
    pub fn init_object(&mut self, addr: Address, encode: u8) {
        unsafe {
            *self.alloc_map.offset((addr.diff(self.space_start) >> LOG_POINTER_SIZE) as isize) = encode;
        }
    }
    
    #[inline(never)]
    pub fn init_object_no_inline(&mut self, addr: Address, encode: u8) {
        self.init_object(addr, encode);
    }
    
    #[inline(never)]
    fn try_alloc_from_local(&mut self, size : usize, align: usize) -> Address {
        // println!("Trying to allocate from local");
    		
        if self.line < immix::LINES_IN_BLOCK {
            let opt_next_available_line = {
                let cur_line = self.line;
                self.block().get_next_available_line(cur_line)
            };
    
            match opt_next_available_line {
                Some(next_available_line) => {
                    // println!("next available line is {}", next_available_line);
                    
                    // we can alloc from local blocks
                    let end_line = self.block().get_next_unavailable_line(next_available_line);
                    
                    // println!("next unavailable line is {}", end_line);
                    self.cursor = self.block().start().plus(next_available_line << immix::LOG_BYTES_IN_LINE);
                    self.limit  = self.block().start().plus(end_line << immix::LOG_BYTES_IN_LINE);
                    self.line   = end_line;
                    
                    // println!("{}", self);
                    self.cursor.memset(0, self.limit.diff(self.cursor));
                    
                    for line in next_available_line..end_line {
                        self.block().line_mark_table_mut().set(line, immix::LineMark::FreshAlloc);
                    }
                    
                    self.alloc(size, align)
                },
                None => {
                    // println!("no availalbe line in current block");                	
                    self.alloc_from_global(size, align)
                }
            }
        } else {
            // we need to alloc from global space
            self.alloc_from_global(size, align)
        }
    }
    
    fn alloc_from_global(&mut self, size: usize, align: usize) -> Address {
        trace!("slowpath: alloc_from_global");
        
        self.return_block();

        loop {
            // check if yield
            self.yieldpoint();
            
            let new_block : Option<Box<ImmixBlock>> = self.space.get_next_usable_block();
            
            match new_block {
                Some(b) => {
                    self.block    = Some(b);
                    self.cursor   = self.block().start();
                    self.limit    = self.block().start();
                    self.line     = 0;
                    
                    return self.alloc(size, align);
                },
                None => {continue; }
            }
        }
    }
    
    pub fn prepare_for_gc(&mut self) {
        self.return_block();
    }
    
    pub fn id(&self) -> usize {
        self.id
    }

    fn return_block(&mut self) {
        if self.block.is_some() {
            self.space.return_used_block(self.block.take().unwrap());
        }        
    }
    fn block(&mut self) -> &mut ImmixBlock {
        self.block.as_mut().unwrap()
    }
    
    pub fn print_object(&self, obj: Address, length: usize) {
        ImmixMutatorLocal::print_object_static(obj, length);
    }
    
    pub fn print_object_static(obj: Address, length: usize) {
        println!("===Object {:#X} size: {} bytes===", obj, length);
        let mut cur_addr = obj;
        while cur_addr < obj.plus(length) {
            println!("Address: {:#X}   {:#X}", cur_addr, unsafe {cur_addr.load::<u64>()});
            cur_addr = cur_addr.plus(8);
        }
        println!("----");
        println!("=========");        
    }
}

impl ImmixMutatorGlobal {
    pub fn new() -> ImmixMutatorGlobal {
        ImmixMutatorGlobal {take_yield: AtomicBool::new(false)}
    }
    
    pub fn set_take_yield(&self, b : bool) {
        self.take_yield.store(b, Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn take_yield(&self) -> bool{
        self.take_yield.load(Ordering::SeqCst)
    }
}

impl fmt::Display for ImmixMutatorLocal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.cursor.is_zero() {
            write!(f, "Mutator (not initialized)")
        } else {
            write!(f, "Mutator:\n").unwrap();
            write!(f, "cursor= {:#X}\n", self.cursor).unwrap();
            write!(f, "limit = {:#X}\n", self.limit).unwrap();
            write!(f, "line  = {}\n", self.line).unwrap();
            write!(f, "block = {}", self.block.as_ref().unwrap())
        }
    }
}