#include "immix.h"
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <sys/mman.h>
#include <string.h>

ImmixSpace* immix_space;

int IMMIX_SPACE_SIZE = DEFAULT_HEAP_SIZE * IMMIX_SPACE_RATIO;

void init_blocks(ImmixSpace* space) {
    int id = 0;
    Address cur = space->start;
    LineMark* curLineMark = space->line_mark_table;
    ImmixBlock* last = NULL;

    int totalBlocks = 0;

    for (; cur < space->end; cur += IMMIX_BYTES_IN_BLOCK, curLineMark = &(curLineMark[IMMIX_LINES_IN_BLOCK]), id += 1) {
        ImmixBlock* b = (ImmixBlock*) malloc(sizeof(ImmixBlock));
        totalBlocks ++;

        if (last == NULL) {
            // first one
            space->usable_blocks = b;
            b->prev = NULL;
        } else {
            last->next = b;
            b->prev = last;
        }

        b->id = id;
        b->state = IMMIX_BLOCK_MARK_USABLE;
        b->start = cur;
        b->line_mark_ptr = curLineMark;
        b->line_mark_len = IMMIX_LINES_IN_BLOCK;

        last = b;
    }

    space->total_blocks = id;
}

ImmixSpace* ImmixSpace_new(int space_size) {
    // mmap
    void* mmap_ret = mmap(0, space_size, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANON, -1, 0);
    if (mmap_ret == MAP_FAILED) {
        printf("failed to call mmap\n");
        exit(1);
    }
    Address mmap_start = (Address) mmap_ret;

    // init immix space
    ImmixSpace* ret = (ImmixSpace*) malloc(sizeof(ImmixSpace));

    ret->start = align_up(mmap_start, IMMIX_BYTES_IN_BLOCK);
    ret->end   = ret->start + space_size;

    // init line mark table
    ret->line_mark_table_len = space_size / IMMIX_BYTES_IN_LINE;
    ret->line_mark_table = (LineMark*) malloc(sizeof(LineMark) * ret->line_mark_table_len);
    LineMark* cursor = ret->line_mark_table;
    for (int i = 0; i < ret->line_mark_table_len; i++) {
        *cursor = IMMIX_LINE_MARK_FREE;
        cursor += sizeof(LineMark);
    }

    // init alloc map and trace map
    ret->map_len = space_size >> LOG_POINTER_SIZE;
    int size = sizeof(uint8_t) * ret->map_len;
    ret->alloc_map = (uint8_t*) malloc(size);
    memset((void*)ret->alloc_map, 0, size);
    ret->trace_map = (uint8_t*) malloc(size);
    memset((void*)ret->trace_map, 0, size);

    printf("malloc alloc_map of size %d bytes\n", size);

    pthread_mutex_init(&(ret->lock), NULL);

    init_blocks(ret);

    return ret;
}

int ImmixBlock_get_next_avaiable_line(ImmixBlock* block, int line) {
    int i = line;
    while (i < block->line_mark_len) {
        if (block->line_mark_ptr[i] == IMMIX_LINE_MARK_FREE) {
            return i;
        } else {
            i++;
        }
    }
    return -1;
}

int ImmixBlock_get_next_unavailable_line(ImmixBlock* block, int line) {
    int i = line;
    while (i < block->line_mark_len) {
        if (block->line_mark_ptr[i] == IMMIX_LINE_MARK_FREE) {
            i++;
        } else {
            return i;
        }
    }
    return i;
}

ImmixBlock* ImmixSpace_get_next_usable_block(ImmixSpace* space) {
    // lock
    pthread_mutex_lock(&(space->lock));

    // get a new block
    ImmixBlock* new_block = space->usable_blocks;

    if (new_block != NULL) {
        // success

        // remove from usable_blocks
        space->usable_blocks = new_block->next;
        if (new_block->next != NULL)
            space->usable_blocks->prev = NULL;

        // add to used block
        new_block->prev = NULL;
        new_block->next = space->used_blocks;
        if (space->used_blocks != NULL)
            space->used_blocks->prev = new_block;
        space->used_blocks = new_block;

        // unlock
        pthread_mutex_unlock(&(space->lock));

        return new_block;
    } else {
        pthread_mutex_unlock(&(space->lock));

        printf("Space exhausted\n");
        printf("gc should happen here\n");
        exit(1);

        return NULL;
    }
}

void ImmixSpace_print(ImmixSpace* space) {

}
