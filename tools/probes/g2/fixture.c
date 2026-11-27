#include <stdint.h>

typedef struct {
    uint64_t x[32];
    uint64_t pc;
    uint8_t memory[16];
    int fault;
} Ctx;

static const uint64_t g_module_base = 0;

/* Synthetic guest memory is little-endian and addresses are byte offsets. */
static void recomp_store64(Ctx *c, uint64_t address, uint64_t value) {
    if (address > sizeof(c->memory) - 8) {
        c->fault = 1;
        return;
    }
    for (unsigned i = 0; i < 8; ++i) {
        c->memory[address + i] = (uint8_t)(value >> (8 * i));
    }
}

static uint64_t recomp_load64(Ctx *c, uint64_t address) {
    uint64_t value = 0;
    if (address > sizeof(c->memory) - 8) {
        c->fault = 1;
        return 0;
    }
    for (unsigned i = 0; i < 8; ++i) {
        value |= (uint64_t)c->memory[address + i] << (8 * i);
    }
    return value;
}

static void add(Ctx *c) {
#include "add.inc"
}

static void branch(Ctx *c) {
#include "branch.inc"
}

/* Dispatch only the two addresses produced by this synthetic branch. */
static int branch_destination(Ctx *c) {
    branch(c);
    switch (c->pc) {
    case 0x10008: return 1;
    case 0x10004: return 2;
    default: return 0;
    }
}

static void store(Ctx *c) {
#include "store.inc"
}

static void load(Ctx *c) {
#include "load.inc"
}

int main(void) {
    Ctx c = {0};
    c.x[0] = 35;
    add(&c);
    if (c.x[0] != 42) return 1;
    c.x[0] = UINT64_MAX;
    add(&c);
    if (c.x[0] != 6) return 2;

    c.x[0] = 0;
    if (branch_destination(&c) != 1) return 3;
    c.x[0] = 1;
    if (branch_destination(&c) != 2) return 4;

    c.x[0] = UINT64_C(0x0123456789abcdef);
    c.x[1] = 8;
    store(&c);
    if (c.fault || c.memory[8] != 0xef || c.memory[15] != 0x01) return 5;
    load(&c);
    if (c.fault || c.x[2] != c.x[0]) return 6;
    c.x[1] = 9;
    store(&c);
    if (!c.fault || c.memory[8] != 0xef || c.memory[15] != 0x01) return 7;
    c.fault = 0;
    load(&c);
    if (!c.fault || c.x[2] != 0) return 8;
    return 0;
}
