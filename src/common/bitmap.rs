extern crate libc;

use std::mem;

#[derive(Clone)]
pub struct Bitmap {    
    bitmap     : *mut u64,
    bitmap_len : usize    
}

impl Bitmap {
    pub fn new(length: usize) -> Bitmap {
        let bitmap_len = length;
        let bitmap = unsafe {
            // secretly reserve one more word
            libc::malloc((mem::size_of::<u64>() * ((bitmap_len >> 6) + 1)) as libc::size_t) as *mut u64
        };
        
        Bitmap{bitmap: bitmap, bitmap_len: bitmap_len}
    }
    
    #[inline(always)]
    pub fn set_bit(&mut self, index: usize) {
        let word = unsafe{self.bitmap.offset((index >> 6) as isize)};
        unsafe {*word = *word | (1 << (index & 63))};
    }
    
    #[inline(always)]
    pub fn clear_bit(&mut self, index: usize) {
        let word = unsafe {self.bitmap.offset((index >> 6) as isize)};
        unsafe {*word = *word & (0 << (index & 63))};
    }
    
    #[inline(always)]
    pub fn test_bit(&self, index: usize) -> bool{
        let word = unsafe {self.bitmap.offset((index >> 6) as isize)};
        unsafe {(*word & (1 << (index & 63))) != 0}
    }
    
    #[inline(always)]
    pub fn length_until_next_bit(&self, index: usize) -> usize {
        let mut len = 1;
        while index + len < self.bitmap_len {
            if self.test_bit(index + len) {
                return len;
            } else {
                len += 1;
                continue;
            }
        }
        
        return 0;
    }
    
    #[inline(always)]
    pub fn set(&mut self, index: usize, value: u64, length: usize) {
        if cfg!(debug_assertions) {
            assert!(index < self.bitmap_len);
            assert!(length <= 64);
        }
        let nth_u64 = index >> 6;
        let nth_bit = index & 63;
        
        let word = unsafe {self.bitmap.offset(nth_u64 as isize)};
        
        if length <= 64 - nth_bit {
            unsafe {
                *word = *word | (value << nth_bit);
            }
        } else {
            unsafe {
                let next_word = self.bitmap.offset(nth_u64 as isize + 1);
                *word = *word | (value.wrapping_shl(nth_bit as u32));
                *next_word = *next_word | (value >> (64 - nth_bit));
            }
        }   
    }
    
    #[inline(always)]
    pub fn get(&self, index: usize, length: usize) -> u64 {
        if cfg!(debug_assertions) {
            assert!(index < self.bitmap_len);
            assert!(length <= 64);
        }
        
        let nth_u64 = index >> 6;
        let nth_bit = index % 64;
        
        let word = unsafe {self.bitmap.offset(nth_u64 as isize)};
        
        if length <= 64 - nth_bit {
            ((unsafe {*word}) >> nth_bit) & ((1 << length) - 1)  
        } else {
            unsafe {
                let next_word = self.bitmap.offset(nth_u64 as isize + 1);
                
                let part1 = *word >> nth_bit;
                let part2 = (*next_word & ( (1 << (nth_bit + length - 64)) - 1 )) << (64 - nth_bit);
                
                part1 | part2
            }
        }
    }
    
    pub fn print(&self) {
        let mut ptr = self.bitmap;
        let nwords = {
            if self.bitmap_len / 64 == 0 {
                1
            } else {
                self.bitmap_len / 64
            }
        };
        
        for i in 0..nwords {
            println!("{}\t0b{:64b}", i * 64, unsafe {*ptr});
            ptr = unsafe{ptr.offset(1)};
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn test_bit() {
        let mut bitmap = Bitmap::new(64);
        
        bitmap.set_bit(3);
        bitmap.print();
        assert!(bitmap.test_bit(3));
        
        bitmap.clear_bit(3);
        assert!(!bitmap.test_bit(3));
        
        bitmap.set_bit(3);
        bitmap.set_bit(4);
        bitmap.set_bit(6);
        
        bitmap.print();
        
        assert_eq!(bitmap.length_until_next_bit(3), 1);
        assert_eq!(bitmap.length_until_next_bit(4), 2);
        assert_eq!(bitmap.length_until_next_bit(5), 1);
        assert_eq!(bitmap.length_until_next_bit(6), 0);
    }
}