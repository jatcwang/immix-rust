use heap::immix;
use heap::gc;
use common::Address;
use common::AddressMap;

extern crate std;
extern crate memmap;
extern crate libc;

use std::*;
use std::collections::LinkedList;
use std::sync::Mutex;
use std::sync::Arc;

// this table will be accessed through unsafe raw pointers. since Rust doesn't provide a data structure for such guarantees:
// 1. Non-overlapping segments of this table may be accessed parallelly from different mutator threads
// 2. One element may be written into at the same time by different gc threads during tracing

#[derive(Clone)]
pub struct LineMarkTable {
    space_start : Address,
    ptr         : *mut immix::LineMark,
    len         : usize,
}

#[derive(Clone)]
pub struct LineMarkTableSlice {
    ptr       : *mut immix::LineMark,
    len       : usize
}

impl LineMarkTable {
    pub fn new(space_start: Address, space_end: Address) -> LineMarkTable {
        let line_mark_table_len = space_end.diff(space_start) / immix::BYTES_IN_LINE;
        let line_mark_table = {
            let ret = unsafe {libc::malloc((mem::size_of::<immix::LineMark>() * line_mark_table_len) as libc::size_t)} as *mut immix::LineMark;
            let mut cursor = ret;
            
            for _ in 0..line_mark_table_len {
                unsafe {*cursor = immix::LineMark::Free;}
                cursor = unsafe {cursor.offset(1)};	
            }
            
            ret
        };
        
        LineMarkTable{space_start: space_start, ptr: line_mark_table, len: line_mark_table_len}
    }
    
    pub fn take_slice(&mut self, start: usize, len: usize) -> LineMarkTableSlice {
        LineMarkTableSlice{ptr: unsafe {self.ptr.offset(start as isize)}, len: len}
    }
    
    #[inline(always)]
    fn get(&self, index: usize) -> immix::LineMark {
        debug_assert!(index <= self.len);
        unsafe {*self.ptr.offset(index as isize)}
    }
    
    #[inline(always)]
    fn set(&self, index: usize, value: immix::LineMark) {
        debug_assert!(index <= self.len);
        unsafe {*self.ptr.offset(index as isize) = value};
    }
    
    #[inline(always)]
    pub fn mark_line_live(&self, addr: Address) {
        let line_table_index = addr.diff(self.space_start) >> immix::LOG_BYTES_IN_LINE;
        
        self.set(line_table_index, immix::LineMark::Live);
        
        if line_table_index < self.len - 1 {
            self.set(line_table_index + 1, immix::LineMark::ConservLive);
        }
    }
    
    #[inline(always)]
    pub fn mark_line_live2(&self, space_start: Address, addr: Address) {
        let line_table_index = addr.diff(space_start) >> immix::LOG_BYTES_IN_LINE;
        
        self.set(line_table_index, immix::LineMark::Live);
        
        if line_table_index < self.len - 1 {
            self.set(line_table_index + 1, immix::LineMark::ConservLive);
        }        
    }
}

impl LineMarkTableSlice {
    #[inline(always)]
    pub fn get(&self, index: usize) -> immix::LineMark {
        debug_assert!(index <= self.len);
        unsafe {*self.ptr.offset(index as isize)}
    }
    #[inline(always)]
    pub fn set(&mut self, index: usize, value: immix::LineMark) {
        debug_assert!(index <= self.len);
        unsafe {*self.ptr.offset(index as isize) = value};
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }
}

#[repr(C)]
pub struct ImmixSpace {
    start            : Address,
    end              : Address,
    
    // these maps are writable at allocation, read-only at collection
    pub alloc_map        : Arc<AddressMap<u8>>,
    
    // these maps are only for collection
    pub trace_map        : Arc<AddressMap<u8>>,
    
    // this table will be accessed through unsafe raw pointers. since Rust doesn't provide a data structure for such guarantees:
    // 1. Non-overlapping segments of this table may be accessed parallelly from different mutator threads
    // 2. One element may be written into at the same time by different gc threads during tracing
    pub line_mark_table  : LineMarkTable,
    
    total_blocks     : usize,  // for debug use
    
    #[allow(dead_code)]
    mmap             : memmap::Mmap,
    usable_blocks    : Mutex<LinkedList<Box<ImmixBlock>>>,
    used_blocks      : Mutex<LinkedList<Box<ImmixBlock>>>,
}

pub struct ImmixBlock {
    id    : usize,
    state : immix::BlockMark,
    start : Address,
    
    // a segment of the big line mark table in ImmixSpace
    line_mark_table : LineMarkTableSlice
}

const SPACE_ALIGN : usize = 1 << 19;

impl ImmixSpace {
    pub fn new(space_size : usize) -> ImmixSpace {
        // acquire memory through mmap
        let anon_mmap : memmap::Mmap = match memmap::Mmap::anonymous(space_size + SPACE_ALIGN, memmap::Protection::ReadWrite) {
            Ok(m) => m,
            Err(_) => panic!("failed to call mmap"),
        };
        let start : Address = Address::from_ptr::<u8>(anon_mmap.ptr()).align_up(SPACE_ALIGN);
        let end   : Address = start.plus(space_size);
        
        let line_mark_table = LineMarkTable::new(start, end);
        
        let mut ret = ImmixSpace {
            start: start, 
            end: end, 
            mmap: anon_mmap,

            line_mark_table: line_mark_table,
            trace_map: Arc::new(AddressMap::new(start, end)),
            alloc_map: Arc::new(AddressMap::new(start, end)),
            usable_blocks: Mutex::new(LinkedList::new()),
            used_blocks: Mutex::new(LinkedList::new()),
            total_blocks: 0
        };
        
        ret.init_blocks();
        
        ret
    }
    
    fn init_blocks(&mut self) -> () {
        let mut id = 0;
        let mut block_start = self.start;
        let mut line = 0;
        
        let mut usable_blocks_lock = self.usable_blocks.lock().unwrap();
        
        while block_start.plus(immix::BYTES_IN_BLOCK) <= self.end {
            usable_blocks_lock.push_back(Box::new(ImmixBlock {
                id : id,
                state: immix::BlockMark::Usable,
                start: block_start,
                line_mark_table: self.line_mark_table.take_slice(line, immix::LINES_IN_BLOCK)
            }));
            
            id += 1;
            block_start = block_start.plus(immix::BYTES_IN_BLOCK);
            line += immix::LINES_IN_BLOCK;
        }
        
        self.total_blocks = id;
    }
    
    pub fn return_used_block(&self, old : Box<ImmixBlock>) {
        // Unsafe and raw pointers are used to transfer ImmixBlock to/from each Mutator. 
        // This avoids explicit ownership transferring
        // If we explicitly transfer ownership, the function needs to own the Mutator in order to move the ImmixBlock out of it (see ImmixMutatorLocal.alloc_from_global()),
        // and this will result in passing the Mutator object as value (instead of a borrowed reference) all the way in the allocation   
        self.used_blocks.lock().unwrap().push_front(old);
    }
    
    #[allow(unreachable_code)]
    pub fn get_next_usable_block(&self) -> Option<Box<ImmixBlock>> {
        let res_new_block : Option<Box<ImmixBlock>> = {
            self.usable_blocks.lock().unwrap().pop_front()
        };
        if res_new_block.is_none() {
            // should unlock, and call GC here
            gc::trigger_gc();
                
            None
        } else {
            res_new_block
        }    	
    }
    
    #[allow(unused_variables)]
    pub fn sweep(&self) {
        let mut free_lines = 0;
        let mut usable_blocks = 0;
        let mut full_blocks = 0;
        
        let mut used_blocks_lock = self.used_blocks.lock().unwrap();
        let mut usable_blocks_lock = self.usable_blocks.lock().unwrap();
        
        let mut live_blocks : LinkedList<Box<ImmixBlock>> = LinkedList::new();

        while !used_blocks_lock.is_empty() {
            let mut block = used_blocks_lock.pop_front().unwrap();
            
            let mut has_free_lines = false;
            
            {
                let mut cur_line_mark_table = block.line_mark_table_mut();
                for i in 0..cur_line_mark_table.len() {
                    if cur_line_mark_table.get(i) != immix::LineMark::Live && cur_line_mark_table.get(i) != immix::LineMark::ConservLive {
                        has_free_lines = true;
                        cur_line_mark_table.set(i, immix::LineMark::Free);
                        
                        free_lines += 1;
                    }
                }
                
                // release the mutable borrow of 'block'
            }
            
            if has_free_lines {
                block.set_state(immix::BlockMark::Usable);
                usable_blocks += 1;

                usable_blocks_lock.push_front(block);
            } else {
                block.set_state(immix::BlockMark::Full);
                full_blocks += 1;
                live_blocks.push_front(block);
            }
        }
        
        used_blocks_lock.append(&mut live_blocks);
        
        if cfg!(debug_assertions) {
            println!("free lines    = {} of {} total", free_lines, self.total_blocks * immix::LINES_IN_BLOCK);
            println!("usable blocks = {}", usable_blocks);
            println!("full blocks   = {}", full_blocks);
        }
        
        if full_blocks == self.total_blocks {
            println!("Out of memory in Immix Space");
            std::process::exit(1);
        }
        
        debug_assert!(full_blocks + usable_blocks == self.total_blocks);
    }
    
    pub fn start(&self) -> Address {
        self.start
    }
    pub fn end(&self) -> Address {
        self.end
    }
    
    pub fn line_mark_table(&self) -> &LineMarkTable {
        &self.line_mark_table
    }

    #[inline(always)]
    pub fn addr_in_space(&self, addr: Address) -> bool {
        addr >= self.start && addr < self.end
    }
}

impl ImmixBlock {
    pub fn get_next_available_line(&self, cur_line : usize) -> Option<usize> {
        let mut i = cur_line;
        while i < self.line_mark_table.len {
            match self.line_mark_table.get(i) {
                immix::LineMark::Free => {return Some(i);},
                _ => {i += 1;},
            }
        }
        None
    }
    
    pub fn get_next_unavailable_line(&self, cur_line : usize) -> usize {
        let mut i = cur_line;
        while i < self.line_mark_table.len {
            match self.line_mark_table.get(i) {
                immix::LineMark::Free => {i += 1;}
                _ => {return i; },
            }
        }
        i
    }
    
    pub fn id(&self) -> usize {
        self.id
    }
    pub fn start(&self) -> Address {
        self.start
    }
    pub fn set_state(&mut self, mark: immix::BlockMark) {
        self.state = mark;
    }
    #[inline(always)]
    pub fn line_mark_table(&self) -> &LineMarkTableSlice {
        &self.line_mark_table
    }
    #[inline(always)]
    pub fn line_mark_table_mut(&mut self) -> &mut LineMarkTableSlice {
        &mut self.line_mark_table
    }
}

/// Using raw pointers forbid the struct being shared between threads
/// we ensure the raw pointers won't be an issue, so we allow Sync/Send on ImmixBlock
unsafe impl Sync for ImmixBlock {}
unsafe impl Send for ImmixBlock {}
unsafe impl Sync for ImmixSpace {}
unsafe impl Send for ImmixSpace {}

impl fmt::Display for ImmixSpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ImmixSpace\n").unwrap();
        write!(f, "range={:#X} ~ {:#X}\n", self.start, self.end).unwrap();
        
        // print table by vec
//        write!(f, "table={{\n").unwrap();
//        for i in 0..self.line_mark_table_len {
//            write!(f, "({})", i).unwrap();
//            write!(f, "{:?},", unsafe{*self.line_mark_table.offset(i as isize)}).unwrap();
//            if i % immix::BYTES_IN_LINE == immix::BYTES_IN_LINE - 1 {
//                write!(f, "\n").unwrap();
//            }
//        }
//        write!(f, "\n}}\n").unwrap();
        
        
        write!(f, "t_ptr={:?}\n", self.line_mark_table.ptr).unwrap();
//        write!(f, "usable blocks:\n").unwrap();
//        for b in self.usable_blocks.iter() {
//            write!(f, "  {}\n", b).unwrap();
//        }
//        write!(f, "used blocks:\n").unwrap();
//        for b in self.used_blocks.iter() {
//            write!(f, "  {}\n", b).unwrap();
//        }
        write!(f, "done\n")
    }
}

impl fmt::Display for ImmixBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ImmixBlock#{}(state={:?}, address={:#X}, line_table={:?}", self.id, self.state, self.start, self.line_mark_table.ptr).unwrap();
    
        write!(f, "[").unwrap();
        for i in 0..immix::LINES_IN_BLOCK {
            write!(f, "{:?},", self.line_mark_table.get(i)).unwrap();
        }
        write!(f, "]")
    }
}