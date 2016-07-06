immix-rust
==========

An Immix implementation written in Rust.

Features
--------

* Immix GC
    * thread local allocation
    * parallel GC
    * non generational, non moving, no defragment
* Free list allocation (as large object allocator)
    * linked list + malloc
    * no proper reclamation yet
    * will reimplement this part in near future
* Target platform
    * x86_64 (root tracing including stack scanning is target dependent)

Interface
--------------
`src/lib.rs` defines interface to use the GC. The interface is also
exposed as a C header `rust_c_interface/immix_rust.h`.

### Initialisation and Destroy

* Rust: `pub extern fn gc_init(immix_size: usize, lo_size: usize, n_gcthreads: usize)`
* C: `extern void gc_init(uint64_t immix_size, uint64_t lo_size, uint64_t n_gcthreads)`

   initialises the GC. First two parameters give sizes of immix space and
   large object space (in bytes), the 3rd parameter defines the number of
   GC threads (only used if multi-thread tracing is turned on via
   `--features mt-trace`)

* Rust: `pub extern fn new_mutator() -> Box<ImmixMutatorLocal>`
* C: `extern struct Mutator* new_mutator()`

   creates a mutator. The user is responsible for storing the mutator at thread
   local storage.

* Rust: `pub extern fn drop_mutator(mutator: Box<ImmixMutatorLocal>)`
* C: `extern void drop_mutator(struct Mutator* mutator)`

   destroys the mutator. After the call, `mutator` is no longer valid. This should
   only be useful when used in C (since Rust will identify the liveness of the mutator,
   and drop it when necessary).

* Rust: `extern "C" {pub fn set_low_water_mark();}`
* C: `extern void set_low_water_mark()`

   stores current stack pointer as a limit, so that conservative stack scanning
   will not traverse stack beyond this limit. Call this function after getting
   a new mutator.

### Allocation

* Rust: `pub extern fn alloc(mutator: &mut Box<ImmixMutatorLocal>, size: usize, align: usize) -> ObjectReference`
* C: `inline uint64_t alloc(struct Mutator** mutator, uint64_t size, uint64_t align)`

  allocates an object in Immix space (thread local allocation).
  The object size should be smaller than one Immix line (256 bytes).
  Use large object allocation if the object size is larger than 256 bytes.

* Rust: `pub extern fn alloc_large(mutator: &mut Box<ImmixMutatorLocal>, size: usize) -> ObjectReference`
* C: `extern uint64_t alloc_large(struct Mutator** mutator, uint64_t size)`

  allocates an object in large object space (global synchronisation involved).
  May later move the object size check into `alloc()` and deprecate this function.

* Rust: `pub extern fn yieldpoint(mutator: &mut Box<ImmixMutatorLocal>)`
* C: `inline void yieldpoint(bool* take_yield, struct Mutator** m)`

  checks if current mutator should yield. GC won't be able to stop a mutator
  unless this function is put into code.

Note: `alloc` and `yieldpoint` are fast paths. They are provided in Rust,
and Rust compiler is able to inline them into Rust code. And they are
expressed in C code in the header file, so that C compiler is able to inline them.

Usage
-----
Install Rust and Cargo from https://www.rust-lang.org/.
Run `cargo build --release` under the repo directory to build the GC
(add `--features mt-trace` to turn on parallel GC), which
will generate a dynamic linked library under `target/release/`.
`rust_c_interface/test.c` gives an example on how to use the GC from
C code. Compile and link `test.c` with the library to test.

Near Future Plan
------

* implementing a more efficient free list allocator
* removing `alloc_large()`, allocation in large object space will be triggered
in `alloc()` if the object size exceeds a threshold.
