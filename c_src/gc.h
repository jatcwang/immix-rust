#include "common.h"
#include "immix.h"
#include "objectmodel.h"

#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#ifndef GC_H
#define GC_H

/*
 * A linked list that would be useful when scanning object
 */
typedef struct AddressVector {
    int current_capacity;

    int length;
    Address* data;
} AddressVector;

AddressVector* AddressVector_new(int init_capacity);
void AddressVector_grow(AddressVector* list);

inline void AddressVector_push(AddressVector* list, Address value) __attribute__((always_inline));
inline void AddressVector_push(AddressVector* list, Address value) {
    if (unlikely(list->length == list->current_capacity)) {
        // grow
        AddressVector_grow(list);
    }

    list->data[list->length] = value;
    list->length ++;
}

inline Address AddressVector_pop(AddressVector* list) __attribute__((always_inline));
inline Address AddressVector_pop(AddressVector* list) {
    Address ret = list->data[list->length - 1];
    list->length--;
    return ret;
}

inline bool AddressVector_is_empty(AddressVector* list) __attribute__((always_inline));
inline bool AddressVector_is_empty(AddressVector* list) {
    return list->length == 0;
}

void start_trace(AddressVector* roots);

inline void process_edge(Address addr, AddressVector* work_queue, uint8_t mark_state) __attribute__((always_inline));
inline void process_edge(Address addr, AddressVector* work_queue, uint8_t mark_state) {
    ObjectReference obj = *((ObjectReference*) addr);

    if (obj != 0 && !is_traced(obj, mark_state)) {
        AddressVector_push(work_queue, obj);
    }
}

void trace_object_no_inline(
        ObjectReference obj,
        AddressVector* work_queue,
        uint8_t mark_state,
        LineMark* line_mark_table, int line_mark_table_len,
        Address space_start, Address space_end
        );

inline void trace_object(
        ObjectReference obj,
        AddressVector* work_queue,
        uint8_t mark_state,
        LineMark* line_mark_table, int line_mark_table_len,
        Address space_start, Address space_end
        ) __attribute__((always_inline));
inline void trace_object(
        ObjectReference obj,
        AddressVector* work_queue,
        uint8_t mark_state,
        LineMark* line_mark_table, int line_mark_table_len,
        Address space_start, Address space_end
        ) {
    mark_as_traced(immix_space->trace_map, obj, space_start, mark_state);

    if (obj >= space_start && obj < space_end) {
        mark_line_live(line_mark_table, line_mark_table_len, space_start, obj);
    } else {
        // freelist mark
    }

    Address base = obj;

    while (true) {
        uint8_t value = get_ref_byte(obj);
        uint8_t ref_bits = lower_bits(value, REF_BITS_LEN);
        bool short_encode = test_nth_bit(value, SHORT_ENCODE_BIT);

        switch (ref_bits) {
        case 0b00000001:
            process_edge(base, work_queue, mark_state); break;
        case 0b00000011:
            process_edge(base, work_queue, mark_state);
            process_edge(base + 8, work_queue, mark_state); break;
        case 0b00001111:
            process_edge(base, work_queue, mark_state);
            process_edge(base + 8, work_queue, mark_state);
            process_edge(base + 16, work_queue, mark_state);
            process_edge(base + 24, work_queue, mark_state); break;
        default:
            printf("unexpected ref_bits patterns"); exit(1);
        }

        if (short_encode) {
            return;
        } else {
            printf("should not use long encode"); exit(1);
            base = base + REF_BITS_LEN * 8;
        }
    }
}

#endif
