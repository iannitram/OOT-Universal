#ifndef OOT_ABI_PROBE_H
#define OOT_ABI_PROBE_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct OotAbiProbe OotAbiProbe;
typedef int32_t (*OotAbiCallback)(void *context, int32_t value);
enum { OOT_ABI_OK = 0, OOT_ABI_INVALID = 1, OOT_ABI_CALLBACK_FAILED = 2, OOT_ABI_PANIC = 3 };

/* Non-null slots must be aligned and writable. Pointer slots must be initialized.
 * The callback and borrowed context must remain valid until shutdown.
 * The callback must return normally and must not call or shut down its probe.
 * Calls and shutdown for a probe must be serialized.
 * On success, *out_probe receives the sole owning pointer. It must start as NULL.
 */
int32_t oot_abi_probe_create(OotAbiCallback callback, void *context, OotAbiProbe **out_probe);
/* Requires a live probe from create and an output slot outside the allocation.
 * Calls the callback synchronously and writes its returned value on success.
 */
int32_t oot_abi_probe_call(OotAbiProbe *probe, int32_t value, int32_t *out_value);
/* Requires the owning pointer slot from create, outside the allocation.
 * Frees the probe and clears the slot. A second shutdown returns INVALID.
 * Any copied handles become invalid after shutdown.
 */
int32_t oot_abi_probe_shutdown(OotAbiProbe **probe);
#ifdef __cplusplus
}
#endif
#endif
