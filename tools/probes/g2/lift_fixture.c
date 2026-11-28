#include <stdint.h>
#include <stdio.h>
typedef struct { uint64_t x[32]; } Ctx;
static int icall_error;
static uint64_t icall_expected_target;
static uint64_t icall_expected_site;
static void a64_icall(Ctx *c, uint64_t target);
#define A64_ICALL(c, target) a64_icall((c), (target))
#define A64_NOESC
#include "lifted.inc"

static void a64_icall(Ctx *c, uint64_t target) {
    if (c->x[30] != icall_expected_site) {
        icall_error = 1;
        return;
    }
    if (target != icall_expected_target) {
        icall_error = 2;
        return;
    }
    if (target == UINT64_C(0x20020)) {
        f_00020020(c);
        return;
    }
    icall_error = 3;
}

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
    c.x[30] = 0x5678;
    f_00020020(&c);
    if (c.x[0] != 40 || c.x[3] != 0x5678) return 4;

    c = (Ctx){0};
    c.x[0] = 35;
    c.x[30] = 0x12345678;
    f_00020000(&c);
    if (c.x[0] != 47 || c.x[1] != 5) return 5;
    if (c.x[3] != 0x2000c || c.x[30] != 0x12345678) return 6;

    c = (Ctx){0};
    c.x[0] = 35;
    c.x[9] = 0xabc;
    c.x[16] = UINT64_C(0x20020);
    c.x[30] = UINT64_C(0x87654321);
    icall_expected_target = UINT64_C(0x20020);
    icall_expected_site = UINT64_C(0x20048);
    f_00020040(&c);
    if (icall_error || c.x[0] != 47 || c.x[3] != icall_expected_site
            || c.x[9] != UINT64_C(0x87654321) || c.x[30] != UINT64_C(0x87654321)) return 7;

    c = (Ctx){0};
    c.x[0] = 35;
    c.x[9] = 0xdef;
    c.x[16] = UINT64_C(0xdeadbeef);
    c.x[30] = UINT64_C(0x13579bdf);
    icall_error = 0;
    icall_expected_target = UINT64_C(0x20020);
    icall_expected_site = UINT64_C(0x20048);
    f_00020040(&c);
    if (icall_error != 2 || c.x[0] != 42 || c.x[3] != 0
            || c.x[9] != UINT64_C(0x13579bdf) || c.x[30] != UINT64_C(0x13579bdf)) return 8;

    c = (Ctx){0};
    c.x[0] = 35;
    c.x[9] = 0x456;
    c.x[16] = UINT64_C(0x20020);
    c.x[30] = UINT64_C(0x2468ace0);
    icall_error = 0;
    icall_expected_target = UINT64_C(0x20020);
    icall_expected_site = UINT64_C(0x2004c);
    f_00020040(&c);
    if (icall_error != 1 || c.x[0] != 42 || c.x[3] != 0
            || c.x[9] != UINT64_C(0x2468ace0) || c.x[30] != UINT64_C(0x2468ace0)) return 9;

    puts("function branch, direct and indirect calls, dispatcher target/site, post-call result and guest link registers passed");
    return 0;
}
