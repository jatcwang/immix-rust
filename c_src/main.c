#include "common.h"
#include "immix.h"
#include "freelist.h"
#include "objectmodel.h"
#include "gc.h"

#include <sys/time.h>
#include <stdio.h>
#include <inttypes.h>
#include <stdlib.h>

#define OBJECT_SIZE 24
#define OBJECT_ALIGN 8
#define ACTUAL_OBJECT_SIZE (OBJECT_SIZE + HEADER_SIZE)

#define ALLOCATION_TIMES 50000000
#define INIT_TIMES       50000000
#define MARK_TIMES       50000000
#define TRACE_TIMES      50000000

struct PerfEvents;
extern struct PerfEvents* start_perf_events();
extern void perf_read_values(struct PerfEvents* events);
extern void perf_print(struct PerfEvents* events);

void alloc_loop(ImmixMutatorLocal* mutator);
void exhaust_alloc() {
    gc_init();

    ImmixMutatorLocal* mutator = ImmixMutatorLocal_new(immix_space);

    printf("Trying to allocate %d objects of (size %d, align %d).\n", ALLOCATION_TIMES, OBJECT_SIZE, OBJECT_ALIGN);
    printf("Considering header size of %d, an object should be %d.\n", HEADER_SIZE, ACTUAL_OBJECT_SIZE);
    printf("This would take %d bytes of %d bytes heap.\n", ALLOCATION_TIMES * ACTUAL_OBJECT_SIZE, IMMIX_SPACE_SIZE);

    alloc_loop(mutator);
}

extern void nop(void* anything);
void alloc_loop(ImmixMutatorLocal* mutator) {
    struct timeval t_start, t_end;

    struct PerfEvents* perf = start_perf_events();
    perf_read_values(perf);
    gettimeofday(&t_start, NULL);

    int i = 0;
    for (; i < ALLOCATION_TIMES; i++) {
//        yieldpoint(mutator);

        Address res = ImmixMutatorLocal_alloc(mutator, ACTUAL_OBJECT_SIZE, OBJECT_ALIGN);
        init_object(mutator, res, 0b11000011);
    }

    gettimeofday(&t_end, NULL);
    perf_read_values(perf);

    double duration = (double) (t_end.tv_usec - t_start.tv_usec) / 1000 + (double) (t_end.tv_sec - t_start.tv_sec) * 1000;
    printf("time used: %f msec\n", duration);
    perf_print(perf);
}

void alloc_init() {
    gc_init();

    ImmixMutatorLocal* mutator = ImmixMutatorLocal_new(immix_space);

    printf("Trying to allocate 1 object of (size %d, align %d).\n", OBJECT_SIZE, OBJECT_ALIGN);
    printf("Considering header size of %d, an object should be %d.\n", HEADER_SIZE, ACTUAL_OBJECT_SIZE);

    printf("Trying to allocate %d objects, which will take roughly %d bytes. \n", INIT_TIMES, INIT_TIMES * ACTUAL_OBJECT_SIZE);
    Address* objs = (Address*) malloc(sizeof(Address) * INIT_TIMES);
    int i = 0;
    for (; i < INIT_TIMES; i++) {
        Address res = ImmixMutatorLocal_alloc(mutator, ACTUAL_OBJECT_SIZE, OBJECT_ALIGN);

        objs[i] = res;
    }

    struct timeval t_start, t_end;

    printf("Start init objects\n");
    struct PerfEvents* perf = start_perf_events();
    perf_read_values(perf);
    gettimeofday(&t_start, NULL);

    for (i = 0; i < MARK_TIMES; i++) {
//        yieldpoint(mutator);
        init_object_no_inline(mutator, objs[i], 0b11000011);
        init_object_no_inline(mutator, objs[i], 0b11000011);
    }

    gettimeofday(&t_end, NULL);
    perf_read_values(perf);

    double duration = (double) (t_end.tv_usec - t_start.tv_usec) / 1000 + (double) (t_end.tv_sec - t_start.tv_sec) * 1000;
    printf("time used: %f msec\n", duration);
    perf_print(perf);
}

void mark_loop(Address* objs, ImmixSpace* immix_space);
void alloc_mark() {
    gc_init();

    ImmixMutatorLocal* mutator = ImmixMutatorLocal_new(immix_space);

    printf("Trying to allocate 1 object of (size %d, align %d).\n", OBJECT_SIZE, OBJECT_ALIGN);
    printf("Considering header size of %d, an object should be %d.\n", HEADER_SIZE, ACTUAL_OBJECT_SIZE);

    printf("Trying to allocate %d objects, which will take roughly %d bytes. \n", MARK_TIMES, MARK_TIMES * ACTUAL_OBJECT_SIZE);
    Address* objs = (Address*) malloc(sizeof(Address) * MARK_TIMES);
    int i = 0;
    for (; i < MARK_TIMES; i++) {
        Address res = ImmixMutatorLocal_alloc(mutator, ACTUAL_OBJECT_SIZE, OBJECT_ALIGN);
        init_object(mutator, res, 0b11000011);

        objs[i] = res;
    }

    mark_loop(objs, immix_space);
}

void mark_loop(Address* objs, ImmixSpace* immix_space) {
    struct timeval t_start, t_end;

    printf("Start marking\n");
    struct PerfEvents* perf = start_perf_events();
    perf_read_values(perf);
    gettimeofday(&t_start, NULL);

    uint8_t mark_state = MARK_STATE;

    Address space_start = immix_space->start;
    Address space_end = immix_space->end;
    LineMark* line_mark_table = immix_space->line_mark_table;
    int line_mark_table_len = immix_space->line_mark_table_len;
    uint8_t* trace_map = immix_space->trace_map;

    int i = 0;
    for (; i < MARK_TIMES; i++) {
        Address obj = objs[i];

        mark_as_traced(trace_map, obj, space_start, mark_state);

        if (obj >= space_start && obj < space_end) {
            mark_line_live(line_mark_table, line_mark_table_len, space_start, objs[i]);
        }
    }

    gettimeofday(&t_end, NULL);
    perf_read_values(perf);

    double duration = (double) (t_end.tv_usec - t_start.tv_usec) / 1000 + (double) (t_end.tv_sec - t_start.tv_sec) * 1000;
    printf("time used: %f msec\n", duration);
    perf_print(perf);
}

void alloc_trace() {
    gc_init();

    ImmixMutatorLocal* mutator = ImmixMutatorLocal_new(immix_space);

    printf("Trying to allocate 1 object of (size %d, align %d).\n", OBJECT_SIZE, OBJECT_ALIGN);
    printf("Considering header size of %d, an object should be %d.\n", HEADER_SIZE, ACTUAL_OBJECT_SIZE);

    printf("Trying to allocate %d objects, which will take roughly %d bytes. \n", MARK_TIMES, MARK_TIMES * ACTUAL_OBJECT_SIZE);
    Address root = ImmixMutatorLocal_alloc(mutator, ACTUAL_OBJECT_SIZE, OBJECT_ALIGN);
    init_object(mutator, root, 0b11000001);

    Address prev = root;
    int i = 0;
    for (; i < TRACE_TIMES - 1; i++) {
        Address res = ImmixMutatorLocal_alloc(mutator, ACTUAL_OBJECT_SIZE, OBJECT_ALIGN);
        init_object(mutator, res, 0b11000001);

        // set prev's 1st field (offset 0) to this object
        *((Address*)(prev + HEADER_SIZE)) = res;

        prev = res;
    }

    struct timeval t_start, t_end;

    printf("Start tracing\n");
    AddressVector* roots = AddressVector_new(10);
    AddressVector_push(roots, root);

    struct PerfEvents* perf = start_perf_events();
    perf_read_values(perf);
    gettimeofday(&t_start, NULL);

    start_trace(roots);

    gettimeofday(&t_end, NULL);
    perf_read_values(perf);

    double duration = (double) (t_end.tv_usec - t_start.tv_usec) / 1000 + (double) (t_end.tv_sec - t_start.tv_sec) * 1000;
    printf("time used: %f msec\n", duration);
    perf_print(perf);
}

extern void bench_start();

int main() {
    char* heap_size_str = getenv("HEAP_SIZE");
    if (heap_size_str != NULL) {
        int len = strlen(heap_size_str);
        if (heap_size_str[len - 1] == 'M') {
            char heap_size_str_trunc[len];
            strncpy(heap_size_str_trunc, heap_size_str, len - 1);
            heap_size_str_trunc[len - 1] = '\0';
            int heap_size = (atoi(heap_size_str_trunc)) << 20;

            IMMIX_SPACE_SIZE = heap_size * IMMIX_SPACE_RATIO;
            LO_SPACE_SIZE = heap_size * LO_SPACE_RATIO;

            printf("heap size is %d bytes (immix: %d bytes, lo: %d bytes). \n", heap_size, IMMIX_SPACE_SIZE, LO_SPACE_SIZE);
        } else {
            printf("unknown heap size variable: %s, ignore\n", heap_size_str);
            printf("using default heap size: %d bytes. \n", IMMIX_SPACE_SIZE);
        }
    } else {
        printf("using default heap size: %d bytes. \n", IMMIX_SPACE_SIZE);
    }

#ifdef exhaust
    exhaust_alloc();
#elif initobj
    alloc_init();
#elif mark
    alloc_mark();
#elif trace
    alloc_trace();
#elif gcbench
    bench_start();
#endif

    return 0;
}
