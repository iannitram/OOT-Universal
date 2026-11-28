#include "runtime_api.h"

static const OotRuntimeApi runtime_api = {
    .version = OOT_RUNTIME_API_VERSION,
    .create = oot_abi_probe_create,
    .call = oot_abi_probe_call,
    .read = oot_abi_probe_read,
    .write = oot_abi_probe_write,
    .shutdown = oot_abi_probe_shutdown,
};

const OotRuntimeApi *oot_runtime_api(void) {
    return &runtime_api;
}
