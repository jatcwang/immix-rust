use common::Address;
use heap::immix;

extern crate aligned_alloc;

use std::collections::LinkedList;
use std::sync::Arc;
use std::sync::RwLock;

pub struct FreeListSpace {
    current_nodes : LinkedList<Box<FreeListNode>>,
    
    node_id: usize,
    
    size       : usize,
    used_bytes : usize
}

impl FreeListSpace {
    pub fn new(size: usize) -> FreeListSpace {
        FreeListSpace {
            current_nodes: LinkedList::new(),
            node_id: 0,
            size: size,
            used_bytes: 0
        }
    }
    
    pub fn mark(&mut self, obj: Address) {
        
    }
    
    pub fn alloc(&mut self, size: usize, align: usize) -> Option<Address> {
        if self.used_bytes + size > self.size {
            None
        } else {
            let ret = self::aligned_alloc::aligned_alloc(size, align);
            
            let addr = Address::from_ptr::<()>(ret);
            
            self.current_nodes.push_front(Box::new(FreeListNode{id: self.node_id, start: addr, size: size, mark: NodeMark::FreshAlloc}));
            self.node_id += 1;
            self.used_bytes += size;
            
            Some(addr)
        }
    }
    
    pub fn sweep(&mut self) {
        let (new_nodes, new_used_bytes) = {
            let mut ret = LinkedList::new();
            let nodes = &mut self.current_nodes;
            let mut used_bytes = 0;
            
            while !nodes.is_empty() {
                let mut node = nodes.pop_front().unwrap();
                match node.mark {
                    NodeMark::Live => {
                        node.set_mark(NodeMark::PrevLive);
                        used_bytes += node.size;
                        ret.push_back(node);
                    },
                    NodeMark::PrevLive | NodeMark::FreshAlloc => {
                        let ptr = node.start.to_ptr::<()>() as *mut ();
                        // free the memory
                        unsafe {self::aligned_alloc::aligned_free(ptr);}
                        // do not add this node into new linked list
                    }
                }
            }
            
            (ret, used_bytes)
        };
        
        self.current_nodes = new_nodes;
        self.used_bytes = new_used_bytes;        
    }
    
    pub fn current_nodes(&self) -> &LinkedList<Box<FreeListNode>> {
        &self.current_nodes
    }
    pub fn current_nodes_mut(&mut self) -> &mut LinkedList<Box<FreeListNode>> {
        &mut self.current_nodes
    }
}

pub struct FreeListNode {
    id: usize,
    start : Address,
    size  : usize,
    mark  : NodeMark
}

impl FreeListNode {
    pub fn set_mark(&mut self, mark: NodeMark) {
        self.mark = mark;
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum NodeMark {
    FreshAlloc,
    PrevLive,
    Live,
}
unsafe impl Sync for NodeMark {}

#[inline(never)]
pub fn alloc_large(size: usize, align: usize, mutator: &mut immix::ImmixMutatorLocal, space: Arc<RwLock<FreeListSpace>>) -> Address {
    loop {
        mutator.yieldpoint();
        
        let ret_addr = {
            let mut lo_space_lock = space.write().unwrap();            
            lo_space_lock.alloc(size, align)
        };
        
        match ret_addr {
            Some(addr) => {
                return addr;
            },
            None => {
                use heap::gc;
                gc::trigger_gc();
            }
        }
    }
}

use std::fmt;

impl fmt::Display for FreeListSpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FreeListSpace\n").unwrap();
        write!(f, "{} used, {} total\n", self.used_bytes, self.size).unwrap();
        write!(f, "nodes:\n").unwrap();
        
        for node in self.current_nodes() {
            write!(f, "  {}\n", node).unwrap();
        }
        
        write!(f, "done\n")
    }
}

impl fmt::Display for FreeListNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FreeListNode#{}(start={:#X}, size={}, state={:?})", self.id, self.start, self.size, self.mark)
    }
}