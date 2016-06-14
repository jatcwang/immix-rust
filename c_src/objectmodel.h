#include <stdint.h>
#include <stdbool.h>

#include "common.h"
#include "immix.h"

#ifndef OBJECTMODEL_H
#define OBJECTMODEL_H

#define HEADER_SIZE 0

//#define SCALAR_INLINE 0
//#define SCALAR_ID     1
//#define ARRAY_INLINE  2
//#define ARRAY_ID      3
//
//#define MARK_BIT_INDEX 0
extern uint8_t MARK_STATE;

//typedef struct Header {
//    uint8_t gcbits;
//    uint8_t ref_bits;
//    uint16_t size;
//    uint32_t ty_id;
//} Header;
//
//typedef struct Type {
//    int size;
//
//    int* ref_offsets;
//    int ref_offsets_len;
//} Type;
//
//extern Type* types;
//
//Header Header_new(uint8_t gcbits, uint8_t ref_bits, uint16_t size, uint32_t ty_id);
//void Header_print(Header* header);
//
//inline uint64_t Header_as_u64(Header hdr) __attribute__((always_inline));
//inline uint64_t Header_as_u64(Header hdr) {
//    return *((uint64_t*)&hdr);
//}

//inline void* as_object(Address addr) __attribute__((always_inline));
//inline void* as_object(Address addr) {
//    return (void*) (addr + HEADER_SIZE);
//}
//
//inline void set_gcbits_as(ObjectReference obj, int index, uint8_t value) __attribute__((always_inline));
//inline void set_gcbits_as(ObjectReference obj, int index, uint8_t value) {
//    Header* hdr = ((Header*)obj);
//    hdr->gcbits = hdr->gcbits ^ (( -value ^ hdr->gcbits) & (1 << index));
//}
//
//inline bool test_gcbits(ObjectReference obj, int index, uint8_t value) __attribute__((always_inline));
//inline bool test_gcbits(ObjectReference obj, int index, uint8_t value) {
//    return (((Header*)obj)->gcbits & value) == value;
//}

#define REF_BITS_LEN     6
#define OBJ_START_BIT    6
#define SHORT_ENCODE_BIT 7

inline bool is_traced(ObjectReference obj, uint8_t mark_state) __attribute__((always_inline));
inline bool is_traced(ObjectReference obj, uint8_t mark_state) {
    return immix_space->trace_map[(obj - immix_space->start) >> LOG_POINTER_SIZE] == mark_state;
}

inline void mark_as_traced(uint8_t* trace_map, ObjectReference obj, Address space_start, uint8_t mark_state) __attribute__((always_inline));
inline void mark_as_traced(uint8_t* trace_map, ObjectReference obj, Address space_start, uint8_t mark_state) {
    trace_map[(obj - space_start) >> LOG_POINTER_SIZE] = mark_state;
}
//void __attribute__ ((noinline)) mark_as_traced(ObjectReference obj);

inline uint8_t get_ref_byte(ObjectReference obj) __attribute__((always_inline));
inline uint8_t get_ref_byte(ObjectReference obj) {
    return immix_space->alloc_map[(obj - immix_space->start) >> LOG_POINTER_SIZE];
}

#endif
