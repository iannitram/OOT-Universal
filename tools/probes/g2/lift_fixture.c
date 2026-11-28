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
    puts("function branch and returns passed");
    return 0;
}
