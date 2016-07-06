#include <pthread.h>
#include <stdio.h>
#include <stdbool.h>
#include "immix_rust.h"

#define N_ALLOCATION_THREADS 5

void* new_allocation_thread(void*);
void allocation_once(struct Mutator* m);
void allocation_loop(struct Mutator* m);

int main() {
    // init gc with size of immix space and large object space, and number of gc threads
    gc_init(1 << 20, 1 << 20, 8);

    pthread_t threads[N_ALLOCATION_THREADS];
    int i;

    for (i = 0; i < N_ALLOCATION_THREADS; i++) {
        pthread_create(&threads[i], NULL, new_allocation_thread, NULL);
    }

    for (i = 0; i < N_ALLOCATION_THREADS; i++) {
        pthread_join(threads[i], NULL);
    }
}

void* new_allocation_thread(void* arg) {
    // creating a new mutator
    struct Mutator* m = new_mutator();

    // set current stack pointer as a limit for conservative stack scanning
    // (stack scanning won't traverse beyond the limit)
    set_low_water_mark();

    // main allocation loop
    allocation_loop(m);

    // if the allocation finishes, we destroy the mutator
    drop_mutator(m);

    return NULL;
}

void print_mutator(struct Mutator* m) {
    printf("mutator:\n");
    printf("id    =%lld\n", m->id);
    printf("space =0x%llx\n", m->space_start);
    printf("cursor=0x%llx\n", m->cursor);
    printf("limit =0x%llx\n", m->limit);
    printf("yield?=%d\n", *(m->yield));
}

void allocation_once(struct Mutator* m) {
    print_mutator(m);
    uint64_t addr = alloc(&m, 32, 8);
    printf("RETURN1 = %llx\n", addr);
    print_mutator(m);
    addr = alloc(&m, 32, 8);
    printf("RETURN2 = %llx\n", addr);
    print_mutator(m);
}

void allocation_loop(struct Mutator* m) {
    bool* yield = m->yield;
    struct Mutator** m_ref = &m;

    while(1) {
        // yieldpoint
        yieldpoint(yield, m_ref);

        uint64_t addr = alloc(&m, 32, 8);
    }
}
