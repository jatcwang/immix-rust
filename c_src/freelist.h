#include "immix.h"

#ifndef FREELIST_H
#define FREELIST_H

#define LO_SPACE_RATIO (1 - IMMIX_SPACE_RATIO)
extern int LO_SPACE_SIZE;

/*
 * FreeList Node, Space
 */
struct FreeListNode;
typedef struct FreeListNode {
    struct FreeListNode* next;
    Address addr;
    int64_t size;
} FreeListNode;

typedef struct FreeListSpace {
    FreeListNode* head;
    FreeListNode* last;

    pthread_mutex_t lock;

    int used_size;
    int total_size;
} FreeListSpace;

extern FreeListSpace* lo_space;

Address alloc_large(int size, int align, ImmixMutatorLocal* mutator, FreeListSpace* space);

FreeListSpace* FreeListSpace_new(int space_size);
Address FreeListSpace_alloc(FreeListSpace* flSpace, int size, int align);

#endif
