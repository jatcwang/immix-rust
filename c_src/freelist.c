#include "freelist.h"
#include "immix.h"
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>

int LO_SPACE_SIZE    = DEFAULT_HEAP_SIZE * LO_SPACE_RATIO;

FreeListSpace* lo_space;

/*
 * freelist space (now using global malloc)
 */
FreeListSpace* FreeListSpace_new(int space_size) {
    FreeListSpace* ret = (FreeListSpace*) malloc(sizeof(FreeListSpace));

    ret->head = NULL;
    ret->last = NULL;
    ret->total_size = space_size;
    ret->used_size  = 0;
    pthread_mutex_init(&(ret->lock), NULL);

    return ret;
}

Address FreeListSpace_alloc(FreeListSpace* flSpace, int size, int align) {
    pthread_mutex_lock( &(flSpace->lock));

    if (flSpace->used_size + size > flSpace->total_size) {
        pthread_mutex_unlock( &(flSpace->lock));
        return 0;
    }

    // actually allocation
    Address addr;
    int ret = posix_memalign((void*)&addr, align, size);
    if (ret != 0) {
        printf("trying to alloc from freelist space: size=%d, align=%d\n", size, align);
        printf("failed posix_memalign alloc");
        exit(1);
    }

    // metadata
    FreeListNode* node = (FreeListNode*) malloc(sizeof(FreeListNode));
    node->next = NULL;
    node->addr = addr;
    node->size = size;

    // update freelist space
    if (flSpace->last == NULL) {
        // first node
        flSpace->head = node;
        flSpace->last = node;
    } else {
        flSpace->last->next = node;
    }

    flSpace->used_size = flSpace->used_size + size;

    pthread_mutex_unlock( &(flSpace->lock));

    return addr;
}

Address alloc_large(int size, int align, ImmixMutatorLocal* mutator, FreeListSpace* space) {
    while (true) {
        yieldpoint(mutator);

        Address ret = FreeListSpace_alloc(space, size, align);

        if (ret != 0) {
            return ret;
        } else {
            printf("Space exhausted for large alloc\n");
            printf("gc should happen here\n");
            exit(1);
        }
    }
}
