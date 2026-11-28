#include "oot_abi_probe.h"
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdint.h>

static int32_t add_context(void *context, int32_t value) {
    return value + *(const int32_t *)context;
}

int main(void) {
    OotAbiProbe *probe = 0, *other = 0;
    int32_t delta = 7, result = 0;
    uint8_t bytes[4] = {0}, replacement[2] = {0xaa, 0xbb};
    assert(oot_abi_probe_create(add_context, &delta, 0) == OOT_ABI_INVALID);
    assert(oot_abi_probe_create(0, &delta, &probe) == OOT_ABI_INVALID && probe == 0);
    assert(oot_abi_probe_create(add_context, &delta, &probe) == OOT_ABI_OK);
    assert(probe != 0);
    assert(oot_abi_probe_call(probe, 35, &result) == OOT_ABI_OK && result == 42);
    assert(oot_abi_probe_read(probe, 0, 4, bytes) == OOT_ABI_OK);
    assert(bytes[0] == 0 && bytes[1] == 0 && bytes[2] == 0 && bytes[3] == 0);
    assert(oot_abi_probe_write(probe, 62, 2, replacement) == OOT_ABI_OK);
    assert(oot_abi_probe_read(probe, 62, 2, bytes) == OOT_ABI_OK);
    assert(bytes[0] == 0xaa && bytes[1] == 0xbb);
    assert(oot_abi_probe_read(probe, 64, 0, 0) == OOT_ABI_OK);
    assert(oot_abi_probe_write(probe, 64, 0, 0) == OOT_ABI_OK);
    assert(oot_abi_probe_read(probe, 0, 1, 0) == OOT_ABI_INVALID);
    assert(oot_abi_probe_write(probe, 0, 1, 0) == OOT_ABI_INVALID);
    assert(oot_abi_probe_write(probe, 63, 2, replacement) == OOT_ABI_INVALID);
    assert(oot_abi_probe_read(probe, 63, 2, bytes) == OOT_ABI_INVALID);
    assert(bytes[0] == 0xaa && bytes[1] == 0xbb);
    assert(oot_abi_probe_write(probe, UINT64_MAX, 1, replacement) == OOT_ABI_INVALID);
    assert(oot_abi_probe_read(probe, 62, 2, bytes) == OOT_ABI_OK);
    assert(bytes[0] == 0xaa && bytes[1] == 0xbb);
    assert(oot_abi_probe_create(add_context, &delta, &other) == OOT_ABI_OK);
    assert(oot_abi_probe_read(other, 62, 2, bytes) == OOT_ABI_OK);
    assert(bytes[0] == 0 && bytes[1] == 0);
    assert(oot_abi_probe_write(other, 62, 2, replacement) == OOT_ABI_OK);
    assert(oot_abi_probe_read(probe, 62, 2, bytes) == OOT_ABI_OK);
    assert(bytes[0] == 0xaa && bytes[1] == 0xbb);
    assert(oot_abi_probe_shutdown(&other) == OOT_ABI_OK && other == 0);
    assert(oot_abi_probe_call(0, 1, &result) == OOT_ABI_INVALID);
    assert(oot_abi_probe_call(probe, 1, 0) == OOT_ABI_INVALID);
    OotAbiProbe *original = probe;
    assert(oot_abi_probe_create(add_context, &delta, &probe) == OOT_ABI_INVALID);
    assert(probe == original);
    assert(oot_abi_probe_shutdown(&probe) == OOT_ABI_OK && probe == 0);
    assert(oot_abi_probe_shutdown(&probe) == OOT_ABI_INVALID);
    assert(oot_abi_probe_shutdown(0) == OOT_ABI_INVALID);
    return 0;
}
