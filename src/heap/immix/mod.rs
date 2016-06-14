mod immix_space;
mod immix_mutator;

pub use self::immix_space::ImmixSpace;
pub use self::immix_mutator::ImmixMutatorLocal;
pub use self::immix_mutator::ImmixMutatorGlobal;
pub use self::immix_space::LineMarkTable as ImmixLineMarkTable;
pub use self::immix_mutator::MUTATORS;
pub use self::immix_mutator::N_MUTATORS;

use std::sync::Arc;
use std::sync::RwLock;

lazy_static!{
    pub static ref SHARED_SPACE : Option<Arc<RwLock<ImmixSpace>>> = None;
}

pub const LOG_BYTES_IN_LINE  : usize = 8;
pub const BYTES_IN_LINE      : usize = (1 << LOG_BYTES_IN_LINE);
pub const LOG_BYTES_IN_BLOCK : usize = 16;
pub const BYTES_IN_BLOCK     : usize = (1 << LOG_BYTES_IN_BLOCK); 
pub const LINES_IN_BLOCK     : usize = (1 << (LOG_BYTES_IN_BLOCK - LOG_BYTES_IN_LINE));

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum LineMark {
    Free,
    Live,
    FreshAlloc,
    ConservLive,
    PrevLive
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum BlockMark {
    Usable,
    Full
}