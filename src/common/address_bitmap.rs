use common::LOG_POINTER_SIZE;
use common::Address;
use common::bitmap::Bitmap;

#[derive(Clone)]
pub struct AddressBitmap {
    start : Address,
    end   : Address,
    
    bitmap: Bitmap    
}

impl AddressBitmap {
    pub fn new(start: Address, end: Address) -> AddressBitmap {
        let bitmap_len = end.diff(start) >> LOG_POINTER_SIZE;
        let bitmap = Bitmap::new(bitmap_len);
        
        AddressBitmap{start: start, end: end, bitmap: bitmap}
    }
    
    #[inline(always)]
    #[allow(mutable_transmutes)]
    pub unsafe fn set_bit(&self, addr: Address) {
        use std::mem;
        let mutable_bitmap : &mut Bitmap = mem::transmute(&self.bitmap);
        mutable_bitmap.set_bit(addr.diff(self.start) >> LOG_POINTER_SIZE);
    }
    
    #[inline(always)]
    #[allow(mutable_transmutes)]
    pub unsafe fn clear_bit(&self, addr: Address) {
        use std::mem;
        let mutable_bitmap : &mut Bitmap = mem::transmute(&self.bitmap);        
        mutable_bitmap.clear_bit(addr.diff(self.start) >> LOG_POINTER_SIZE);
    }
    
    #[inline(always)]
    pub fn test_bit(&self, addr: Address) -> bool {
        self.bitmap.test_bit(addr.diff(self.start) >> LOG_POINTER_SIZE)
    }
        
    #[inline(always)]
    pub fn length_until_next_bit(&self, addr: Address) -> usize {
        self.bitmap.length_until_next_bit(addr.diff(self.start) >> LOG_POINTER_SIZE)
    }
    
    #[inline(always)]
    #[allow(mutable_transmutes)]
    pub unsafe fn set(&self, addr: Address, value: u64, length: usize) {
        use std::mem;
        
        if cfg!(debug_assertions) {
            assert!(addr >= self.start && addr <= self.end);
        }
        
        let index = addr.diff(self.start) >> LOG_POINTER_SIZE;
        let mutable_bitmap : &mut Bitmap = mem::transmute(&self.bitmap);
        mutable_bitmap.set(index, value, length);
    }
    
    #[inline(always)]
    pub fn get(&self, addr: Address, length: usize) -> u64 {
        if cfg!(debug_assertions) {
            assert!(addr >= self.start && addr <= self.end);
        }
        
        let index = addr.diff(self.start) >> LOG_POINTER_SIZE;
        self.bitmap.get(index, length)
    }

    pub fn print(&self) {
        self.bitmap.print();
    }
}