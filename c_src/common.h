#include <stdint.h>

#ifndef COMMON_H
#define COMMON_H
typedef uintptr_t Address;
typedef uintptr_t ObjectReference;

#define LOG_POINTER_SIZE 3
#define POINTER_SIZE (1 << LOG_POINTER_SIZE)

#define likely(x)       __builtin_expect((x),1)
#define unlikely(x)     __builtin_expect((x),0)

#define Address_store(addr, Ty, value) *((Ty*)addr) = value

#define lower_bits(value, len) (value & ((1 << len) - 1))
#define test_nth_bit(value, index) ((value & (1 << index)) != 0)

#endif
