#include <stdint.h>

struct Mutator;

extern void gc_init(uint64_t immix_size, uint64_t lo_size, uint64_t n_gcthreads);
extern void set_low_water_mark();
extern struct Mutator* new_mutator();
extern void drop_mutator(struct Mutator* mutator);
extern void yieldpoint(struct Mutator** mutator);
extern uint64_t alloc(struct Mutator** mutator, uint64_t size, uint64_t align);
extern uint64_t alloc_large(struct Mutator** mutator, uint64_t size);
