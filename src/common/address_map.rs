extern crate libc;

use std::mem;
use common::LOG_POINTER_SIZE;
use common::Address;
use heap::gc::malloc_zero;

#[derive(Clone)]
pub struct AddressMap<T: Copy> {
    start : Address,
    end   : Address,
    
    pub ptr   : *mut T,
    len   : usize
}

impl <T> AddressMap<T> where T: Copy{
    pub fn new(start: Address, end: Address) -> AddressMap<T> {
        let len = end.diff(start) >> LOG_POINTER_SIZE;
        let ptr = unsafe{malloc_zero((mem::size_of::<T>() * len) as libc::size_t)} as *mut T;
        
        AddressMap{start: start, end: end, ptr: ptr, len: len}
    }
    
    #[inline(always)]
    pub fn set(&self, addr: Address, value: T) {
        let index = (addr.diff(self.start) >> LOG_POINTER_SIZE) as isize;
        unsafe{*self.ptr.offset(index) = value};
    }

    #[inline(always)]
    pub fn get(&self, addr: Address) -> T {
        let index = (addr.diff(self.start) >> LOG_POINTER_SIZE) as isize;
        unsafe {*self.ptr.offset(index)}
    }
}