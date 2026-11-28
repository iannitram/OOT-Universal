#include <stdint.h>

int32_t oot_shared_transform(int32_t value) {
    return value ^ INT32_C(0x13579bdf);
}

uint32_t oot_shared_version(void) {
    return UINT32_C(1);
}
