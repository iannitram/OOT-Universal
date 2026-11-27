#include "oot_abi_probe.h"

/* Freestanding ABI signature/layout compile smoke test; no game interfaces. */
int32_t oot_probe_translation_unit(OotAbiCallback callback, void *context,
                                   OotAbiProbe **probe, int32_t *result) {
    int32_t status = oot_abi_probe_create(callback, context, probe);
    if (status != OOT_ABI_OK) return status;
    status = oot_abi_probe_call(*probe, INT32_C(1234), result);
    (void)oot_abi_probe_shutdown(probe);
    return status;
}
