#include "gc.h"
#include "immix.h"
#include "objectmodel.h"

#include <stdlib.h>
#include <stdio.h>

AddressVector* AddressVector_new(int init_capacity) {
    Address* data = (Address*) malloc(init_capacity * sizeof(Address));

    AddressVector* ret = (AddressVector*) malloc(sizeof(AddressVector));
    ret->current_capacity = init_capacity;
    ret->length = 0;
    ret->data = data;

    return ret;
}

void AddressVector_grow(AddressVector* list) {
    int old_capacity = list->current_capacity;
    int new_capacity = old_capacity * 2;

    void* new_data = malloc(new_capacity * sizeof(Address));
    memcpy(new_data, (void*)list->data, old_capacity * sizeof(Address));

    free((void*)list->data);

    list->data = (Address*) new_data;
    list->current_capacity = new_capacity;
}

void start_trace(AddressVector* roots) {
    uint8_t mark_state = MARK_STATE;

    LineMark* line_mark_table = immix_space->line_mark_table;
    int line_mark_table_len = immix_space->line_mark_table_len;
    Address space_start = immix_space->start;
    Address space_end   = immix_space->end;

    while (!AddressVector_is_empty(roots)) {
        trace_object(AddressVector_pop(roots), roots, mark_state, line_mark_table, line_mark_table_len, space_start, space_end);
    }
}

void trace_object_no_inline(
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
