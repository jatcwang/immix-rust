#include "common.h"
#include <pthread.h>
#include <stdbool.h>
#include <string.h>

#ifndef IMMIX_H
#define IMMIX_H

#define DEFAULT_HEAP_SIZE (500 << 20)

typedef uint8_t LineMark;
typedef uint8_t BlockMark;

/*
 * ImmixBlock
 */
typedef struct ImmixBlock {
    struct ImmixBlock* next;
    struct ImmixBlock* prev;

    int id;
    BlockMark state;
    Address start;

    LineMark* line_mark_ptr;
    int line_mark_len;
} ImmixBlock;

/*
 * ImmixSpace
 */
typedef struct ImmixSpace {
    Address start;
    Address end;

    uint8_t* alloc_map;
    uint8_t* trace_map;
    int map_len;

    LineMark* line_mark_table;
    int line_mark_table_len;

    int total_blocks;

    ImmixBlock* usable_blocks;
    ImmixBlock* used_blocks;

    pthread_mutex_t lock;
} ImmixSpace;

#define IMMIX_SPACE_RATIO 0.8

extern int IMMIX_SPACE_SIZE;
extern ImmixSpace* immix_space;

/*
 * Immix Mutator
 */
typedef struct ImmixMutatorLocal {
    uint64_t id;

    uint8_t* alloc_map;
    Address space_start;

    Address cursor;
    Address limit;
    uint64_t line;

    ImmixSpace* space;

    ImmixBlock* block;

    bool take_yield;
} ImmixMutatorLocal;

#define MAX_MUTATORS 1024

extern pthread_mutex_t id_lock;
extern int N_MUTTAORS;
extern ImmixMutatorLocal* MUTATORS[MAX_MUTATORS];

void gc_init();

/*
 * Immix constants
 */
#define IMMIX_LOG_BYTES_IN_BLOCK 16
#define IMMIX_BYTES_IN_BLOCK (1 << IMMIX_LOG_BYTES_IN_BLOCK)

#define IMMIX_LOG_BYTES_IN_LINE 8
#define IMMIX_BYTES_IN_LINE (1 << IMMIX_LOG_BYTES_IN_LINE)

#define IMMIX_LINES_IN_BLOCK (1 << (IMMIX_LOG_BYTES_IN_BLOCK - IMMIX_LOG_BYTES_IN_LINE))

#define IMMIX_LINE_MARK_FREE            0
#define IMMIX_LINE_MARK_LIVE            1
#define IMMIX_LINE_MARK_FRESH_ALLOC     2
#define IMMIX_LINE_MARK_CONSERV_LIVE    3
#define IMMIX_LINE_MARK_PREV_LIVE       4

#define IMMIX_BLOCK_MARK_USABLE         0
#define IMMIX_BLOCK_MARK_FULL           1

#define ALIGNMENT_VALUE                 1

inline Address align_up(Address region, int align) __attribute__((always_inline));
inline Address align_up(Address region, int align) {
    return (region + align - 1) & ~ (align - 1);
}

inline void fill_alignment_gap(Address start, Address end) __attribute__((always_inline));
inline void fill_alignment_gap(Address start, Address end) {
    if (end > start)
        memset((void*) start, ALIGNMENT_VALUE, end - start);
}

ImmixSpace* ImmixSpace_new(int space_size);
ImmixMutatorLocal* ImmixMutatorLocal_new(ImmixSpace* space);

Address ImmixMutatorLocal_try_alloc_from_local(ImmixMutatorLocal* mutator, int size, int align) __attribute__ ((noinline));

inline void init_object(ImmixMutatorLocal* mutator, Address addr, uint8_t encode) __attribute__((always_inline));
inline void init_object(ImmixMutatorLocal* mutator, Address addr, uint8_t encode){
    uint8_t* map = mutator->alloc_map;
    map[(addr - mutator->space_start) >> LOG_POINTER_SIZE] = encode;
}
void init_object_no_inline(ImmixMutatorLocal* mutator, Address addr, uint8_t encode);

inline Address ImmixMutatorLocal_alloc(ImmixMutatorLocal* mutator, int size, int align) __attribute__((always_inline));
inline Address ImmixMutatorLocal_alloc(ImmixMutatorLocal* mutator, int size, int align) {
    Address start = align_up(mutator->cursor, align);
    Address end = start + size;

    if (end > mutator->limit) {
        return ImmixMutatorLocal_try_alloc_from_local(mutator, size, align);
    } else {
//        fill_alignment_gap(mutator->cursor, start);
        mutator->cursor = end;

        return start;
    }
}

void ImmixSpace_print(ImmixSpace* space);

int ImmixBlock_get_next_avaiable_line(ImmixBlock* block, int line);
int ImmixBlock_get_next_unavailable_line(ImmixBlock* block, int line);
ImmixBlock* ImmixSpace_get_next_usable_block(ImmixSpace* space);

void yieldpoint_slowpath(ImmixMutatorLocal* mutator) __attribute__((__noinline__));

inline void yieldpoint(ImmixMutatorLocal* mutator) __attribute__((always_inline));
inline void yieldpoint(ImmixMutatorLocal* mutator) {
    if (mutator->take_yield)
        yieldpoint_slowpath(mutator);
}

inline void mark_line_live(LineMark* line_mark_table, int line_mark_table_len, Address start, Address addr) __attribute__((always_inline));
inline void mark_line_live(LineMark* line_mark_table, int line_mark_table_len, Address start, Address addr) {
    uintptr_t mark_table_index = (addr - start) >> IMMIX_LOG_BYTES_IN_LINE;
    line_mark_table[mark_table_index] = IMMIX_LINE_MARK_LIVE;
    if (mark_table_index < line_mark_table_len - 1)
        line_mark_table[mark_table_index + 1] = IMMIX_LINE_MARK_CONSERV_LIVE;
}

#endif
