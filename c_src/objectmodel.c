#include <inttypes.h>
#include <string.h>
#include <stdio.h>
#include "objectmodel.h"

uint8_t MARK_STATE = 1;

//Header Header_new(uint8_t gcbits, uint8_t ref_bits, uint16_t size, uint32_t ty_id) {
//    Header header;
//    header.gcbits = gcbits;
//    header.ref_bits = ref_bits;
//    header.size = size;
//    header.ty_id = ty_id;
//    return header;
//}

const char *byte_to_binary(int x) {
    static char b[9];
    b[0] = '\0';

    int z;
    for (z = 128; z > 0; z >>= 1)
    {
        strcat(b, ((x & z) == z) ? "1" : "0");
    }

    return b;
}

//void Header_print(Header* header) {
//    uint64_t header_u64 = Header_as_u64(*header);
//    printf("0x%" PRIx64 "(0b%s)\n", header_u64, byte_to_binary(header_u64));
//    printf("gcbits: 0b%s\n", byte_to_binary((int) header->gcbits));
//    printf("refbits: 0b%s\n", byte_to_binary((int) header->ref_bits));
//}

//void __attribute__ ((noinline)) mark_as_traced(ObjectReference obj) {
//    set_gcbits_as(obj, MARK_BIT_INDEX, MARK_STATE);
//}
