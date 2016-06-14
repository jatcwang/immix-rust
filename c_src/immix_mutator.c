#include "immix.h"
#include "freelist.h"
#include "common.h"

#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <sys/mman.h>
#include <string.h>

pthread_mutex_t id_lock;
int N_MUTATORS;
ImmixMutatorLocal* MUTATORS[MAX_MUTATORS];

void gc_init() {
    immix_space = ImmixSpace_new(IMMIX_SPACE_SIZE);
    lo_space = FreeListSpace_new(LO_SPACE_SIZE);

    pthread_mutex_init(&id_lock, NULL);
    N_MUTATORS = 0;
}

void yieldpoint_slowpath(ImmixMutatorLocal* mutator) {
    printf("yieldpoint slowpath");
}

void ImmixMutatorLocal_reset(ImmixMutatorLocal* mutator) {
    mutator->cursor = 0;
    mutator->limit = 0;
    mutator->line = IMMIX_LINES_IN_BLOCK;

    mutator->block = NULL;
}

ImmixMutatorLocal* ImmixMutatorLocal_new(ImmixSpace* space) {
    ImmixMutatorLocal* ret = (ImmixMutatorLocal*) malloc(sizeof(ImmixMutatorLocal));

    pthread_mutex_lock(&id_lock);

    // set id
    ret->id = N_MUTATORS;
    // global array
    MUTATORS[N_MUTATORS] = ret;
    // increase id
    N_MUTATORS ++;

    // set other fields
    ret->space = space;
    ImmixMutatorLocal_reset(ret);

    // alloc map
    ret->alloc_map = space->alloc_map;
    ret->space_start = space->start;

    pthread_mutex_unlock(&id_lock);
    return ret;
}

Address ImmixMutatorLocal_alloc_from_global(ImmixMutatorLocal* mutator, int size, int align) {
    // we dont need to return block (as in Rust)

    while (true) {
        yieldpoint(mutator);

        ImmixBlock* new_block = ImmixSpace_get_next_usable_block(mutator->space);

        if (new_block != NULL) {
            mutator->block = new_block;
            mutator->cursor = new_block->start;
            mutator->limit = new_block->start;
            mutator->line = 0;

            return ImmixMutatorLocal_alloc(mutator, size, align);
        } else {
            continue;
        }
    }
}

Address ImmixMutatorLocal_try_alloc_from_local(ImmixMutatorLocal* mutator, int size, int align) {
    if (mutator->line < IMMIX_LINES_IN_BLOCK) {
        int next_available_line = ImmixBlock_get_next_avaiable_line(mutator->block, mutator->line);

        if (next_available_line != -1) {
            int end_line = ImmixBlock_get_next_unavailable_line(mutator->block, next_available_line);

            mutator->cursor = mutator->block->start + (next_available_line << IMMIX_LOG_BYTES_IN_LINE);
            mutator->limit  = mutator->block->start + (end_line << IMMIX_LOG_BYTES_IN_LINE);
            mutator->line   = end_line;

            memset((void*)mutator->cursor, 0, mutator->limit - mutator->cursor);

            int line = next_available_line;
            for (; line < end_line; line++) {
                mutator->block->line_mark_ptr[line] = IMMIX_LINE_MARK_FRESH_ALLOC;
            }

            return ImmixMutatorLocal_alloc(mutator, size, align);
        } else {
            return ImmixMutatorLocal_alloc_from_global(mutator, size, align);
        }
    } else {
        return ImmixMutatorLocal_alloc_from_global(mutator, size, align);
    }
}

void init_object_no_inline(ImmixMutatorLocal* mutator, Address addr, uint8_t encode) {
    mutator->alloc_map[(addr - mutator->space_start) >> LOG_POINTER_SIZE] = encode;
}
