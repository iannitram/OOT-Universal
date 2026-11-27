#include <stdint.h>

typedef struct {
    uint64_t x[32];
} Ctx;

static void execute(Ctx *c) {
#include "translated.inc"
}

int main(void) {
    Ctx c = {0};
    c.x[0] = 35;
    execute(&c);
    if (c.x[0] != 42) return 1;
    c.x[0] = UINT64_MAX;
    execute(&c);
    return c.x[0] != 6;
}
