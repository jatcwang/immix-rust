#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>

// the layout of this struct needs to be the same as src/heap/immix/immix_mutator.rs
struct Mutator {
    uint64_t id;
    void* alloc_map;
    uint64_t space_start;
    uint64_t cursor;
    uint64_t limit;
    uint64_t line;

    bool* yield;
    // we do not care about the rest
};

extern void gc_init(uint64_t immix_size, uint64_t lo_size, uint64_t n_gcthreads);
extern void set_low_water_mark();
extern struct Mutator* new_mutator();
extern void drop_mutator(struct Mutator* mutator);
extern void yieldpoint_slow(struct Mutator** mutator);
extern uint64_t alloc_slow(struct Mutator** mutator, uint64_t size, uint64_t align);
extern uint64_t alloc_large(struct Mutator** mutator, uint64_t size);

inline void yieldpoint(bool* take_yield, struct Mutator** m) __attribute__((always_inline));
inline void yieldpoint(bool* take_yield, struct Mutator** m) {
    if (*take_yield) {
        yieldpoint_slow(m);
    }
}

inline uint64_t align_up(uint64_t addr, uint64_t align) __attribute__((always_inline));
inline uint64_t align_up(uint64_t addr, uint64_t align) {
    return (addr + align - 1) & ~(align - 1);
}

inline uint64_t alloc(struct Mutator** mutator, uint64_t size, uint64_t align) __attribute__((always_inline));
inline uint64_t alloc(struct Mutator** mutator, uint64_t size, uint64_t align) {
    struct Mutator* self = *mutator;
    uint64_t start = align_up(self->cursor, align);
    uint64_t end = start + size;

    if (end > self->limit)
        return alloc_slow(mutator, size, align);
    else {
        self->cursor = end;
        return start;
    }
}
