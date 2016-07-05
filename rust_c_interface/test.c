#include <pthread.h>
#include "immix_rust.h"

#define N_ALLOCATION_THREADS 5

void* new_allocation_thread(void*);
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

void allocation_loop(struct Mutator* m) {
    while(1) {
        yieldpoint(&m);

        uint64_t addr = alloc(&m, 32, 8);
    }
}
