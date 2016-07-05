#include <inttypes.h>
#include <stdlib.h>

void* malloc_zero(size_t size) {
    void* ret = malloc(size);
    memset(ret, 0, size);
    return ret;
}



uintptr_t immmix_get_stack_ptr();
uintptr_t immmix_get_stack_ptr() {
    uintptr_t rsp;
    // get current rsp, rbp (this C func frame)
    __asm__(
        "mov %%rsp, %0 \n"
        : "=rm" (rsp)
    );

    return rsp;
}

int get_registers_count() {
    return 16;
}

uintptr_t* get_registers () {
    uintptr_t rax, rbx, rcx, rdx, rbp, rsp, rsi, rdi, r8, r9, r10, r11, r12, r13, r14, r15;

    __asm__(
        "mov %%rax, %0 \n"
        "mov %%rbx, %1 \n"
        "mov %%rcx, %2 \n"
        "mov %%rdx, %3 \n"
        "mov %%rbp, %4 \n"
        "mov %%rsp, %5 \n"
        "mov %%rsi, %5 \n"
        "mov %%rdi, %6 \n"
        "mov %%r8,  %7 \n"
        "mov %%r9,  %8 \n"
        "mov %%r10, %10\n"
        "mov %%r11, %11\n"
        "mov %%r12, %12\n"
        "mov %%r13, %13\n"
        "mov %%r14, %14\n"
        "mov %%r15, %15\n"
        : "=m" (rax),
          "=m" (rbx),
          "=m" (rcx),
          "=m" (rdx),
          "=m" (rbp),
          "=m" (rsp),
          "=m" (rsi),
          "=m" (rdi),
          "=m" (r8),
          "=m" (r9),
          "=m" (r10),
          "=m" (r11),
          "=m" (r12),
          "=m" (r13),
          "=m" (r14),
          "=m" (r15)
          :
          :
        );

    uintptr_t* ret = (uintptr_t*) malloc(sizeof(uintptr_t) * 16);
    ret[0] = rax;
    ret[1] = rbx;
    ret[2] = rcx;
    ret[3] = rdx;
    ret[4] = rbp;
    ret[5] = rsp;
    ret[6] = rsi;
    ret[7] = rdi;
    ret[8] = r8;
    ret[9] = r9;
    ret[10] = r10;
    ret[11] = r11;
    ret[12] = r12;
    ret[13] = r13;
    ret[14] = r14;
    ret[15] = r15;
    return ret;
}

__thread uintptr_t low_water_mark;

void set_low_water_mark () {
    uintptr_t rsp;
    // get current rsp, rbp (this C func frame)
    __asm__(
        "mov %%rsp, %0 \n"
        : "=rm" (rsp)
    );

    low_water_mark = rsp;
}

uintptr_t get_low_water_mark() {
    return low_water_mark;
}
