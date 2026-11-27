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
    OotAbiProbe *probe = 0;
    int32_t delta = 7, result = 0;
    assert(oot_abi_probe_create(add_context, &delta, 0) == OOT_ABI_INVALID);
    assert(oot_abi_probe_create(0, &delta, &probe) == OOT_ABI_INVALID && probe == 0);
    assert(oot_abi_probe_create(add_context, &delta, &probe) == OOT_ABI_OK);
    assert(probe != 0);
    assert(oot_abi_probe_call(probe, 35, &result) == OOT_ABI_OK && result == 42);
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
