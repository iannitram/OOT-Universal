#include "runtime_api.h"

#include <stddef.h>

static int32_t add_context(void *context, int32_t value) {
    return value + *(const int32_t *)context;
}

int32_t oot_shared_run(const OotRuntimeApi *api) {
    if (api == NULL || api->version != OOT_RUNTIME_API_VERSION || api->create == NULL ||
        api->call == NULL || api->read == NULL || api->write == NULL || api->shutdown == NULL) {
        return OOT_SHARED_INVALID;
    }

    OotAbiProbe *probe = NULL;
    int32_t delta = 7;
    int32_t result = 0;
    uint8_t initial[2] = { 0xff, 0xff };
    const uint8_t written[2] = { 0xaa, 0xbb };
    uint8_t readback[2] = { 0, 0 };
    int32_t scenario_status = OOT_SHARED_OK;

    if (api->create(NULL, &delta, &probe) != OOT_ABI_INVALID || probe != NULL) {
        if (probe != NULL) (void)api->shutdown(&probe);
        return 10;
    }
    if (api->create(add_context, &delta, &probe) != OOT_ABI_OK || probe == NULL) {
        if (probe != NULL) (void)api->shutdown(&probe);
        return 11;
    }
    if (api->call(probe, 35, &result) != OOT_ABI_OK || result != 42) scenario_status = 12;
    else if (api->read(probe, 62, 2, initial) != OOT_ABI_OK || initial[0] != 0 || initial[1] != 0) scenario_status = 13;
    else if (api->write(probe, 62, 2, written) != OOT_ABI_OK) scenario_status = 14;
    else if (api->write(probe, 63, 2, written) != OOT_ABI_INVALID) scenario_status = 15;
    else if (api->read(probe, 62, 2, readback) != OOT_ABI_OK ||
             readback[0] != written[0] || readback[1] != written[1]) scenario_status = 16;
    else if (api->write(probe, UINT64_MAX, 1, written) != OOT_ABI_INVALID) scenario_status = 17;

    if (api->shutdown(&probe) != OOT_ABI_OK || probe != NULL) return 18;
    if (api->shutdown(&probe) != OOT_ABI_INVALID || probe != NULL) return 19;
    return scenario_status;
}
