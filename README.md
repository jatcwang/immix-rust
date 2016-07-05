immix-rust
==========

An Immix implementation written in Rust.

Features
--------

* Immix GC
    * thread local allocation
    * parallel GC
    * non generational
    * non moving
    * no defragment
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

* `pub extern fn gc_init(immix_size: usize, lo_size: usize, n_gcthreads: usize)`

   initialises the GC. First two parameters give sizes of immix space and
   large object space (in bytes), the 3rd parameter defines the number of
   GC threads (only used if multi-thread tracing is turned on via
   `--features mt-trace`)

* `pub extern fn new_mutator() -> Box<ImmixMutatorLocal>`

   creates a mutator. The user is responsible for storing the mutator at thread
   local storage.

* `pub extern fn drop_mutator(mutator: Box<ImmixMutatorLocal>)`

   destroys the mutator. After the call, `mutator` is no longer valid.

* `extern "C" {
    pub fn set_low_water_mark();
  }`

   stores current stack pointer as a limit, so that conservative stack scanning
   will not traverse stack beyond this limit. Call this function after getting
   a new mutator.

### Allocation

* `pub extern fn alloc(mutator: &mut Box<ImmixMutatorLocal>, size: usize, align: usize) -> ObjectReference`

  allocates an object in Immix space (thread local allocation).
  The object size should be smaller than one Immix line (256 bytes).
  Use large object allocation if the object size is larger than 256 bytes.

* `pub extern fn alloc_large(mutator: &mut Box<ImmixMutatorLocal>, size: usize) -> ObjectReference`

  allocates an object in large object space (global synchronisation involved).
  May later move the object size check into `alloc()` and deprecate this function.

*  `pub extern fn yieldpoint(mutator: &mut Box<ImmixMutatorLocal>)`

  checks if current mutator should yield. GC won't be able to stop a mutator
  unless this function is put into code.

Note: `alloc` and `yieldpoint` are fast paths. They are supposed to be inlined
into user code. However, there is no obvious way to inline a Rust function
into C code. This can be resolved either by generating assembly from Rust,
or expressing the fast paths in C code (this will be done in near future).


Usage
-----
Install Rust and Cargo from https://www.rust-lang.org/.
Run `cargo build --release` under the repo directory to build the GC
(add `--features mt-trace` to turn on parallel GC), which
will generate `target/release/libimmix_rust.dylib`.
`rust_c_interface/test.c` gives an example on how to use the GC from
C code. Compile and link `test.c` with the library to test.

Near Future Plan
------

* implementing a more efficient free list allocator
* allowing inlining of fastpaths into C
* removing `alloc_large()`, allocation in large object space will be triggered
in `alloc()` if the object size exceeds a threshold. 
