#include <stdint.h>
#include <stdio.h>
typedef struct { uint64_t x[32]; } Ctx;
#define A64_NOESC
#include "lifted.inc"
int main(void) {
    Ctx c = {0};
    f_00010000(&c);
    if (c.x[0] != 3) return 1;
    c.x[0] = 5;
    f_00010000(&c);
    if (c.x[0] != 12) return 2;
    c.x[0] = UINT64_MAX;
    f_00010000(&c);
    if (c.x[0] != 6) return 3;

    c = (Ctx){0};
    c.x[0] = 35;
    f_00020010(&c);
    if (c.x[0] != 40) return 4;

    c = (Ctx){0};
    c.x[0] = 35;
    f_00020000(&c);
    if (c.x[0] != 40) return 5;
    if (c.x[30] != 0x20008) return 6;

    puts("function branch, direct call, callee result and guest link register passed");
    return 0;
}
