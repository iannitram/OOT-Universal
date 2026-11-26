#ifndef OOT_ABI_PROBE_H
#define OOT_ABI_PROBE_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct OotAbiProbe OotAbiProbe;
typedef int32_t (*OotAbiCallback)(void *context, int32_t value);
enum { OOT_ABI_OK = 0, OOT_ABI_INVALID = 1, OOT_ABI_CALLBACK_FAILED = 2, OOT_ABI_PANIC = 3 };

/* *out_probe must initially be NULL. Context is borrowed and must outlive the probe. */
int32_t oot_abi_probe_create(OotAbiCallback callback, void *context, OotAbiProbe **out_probe);
/* Calls callback synchronously and writes its returned value on success. */
int32_t oot_abi_probe_call(OotAbiProbe *probe, int32_t value, int32_t *out_value);
/* Frees the probe and nulls *probe. A second shutdown returns INVALID. */
int32_t oot_abi_probe_shutdown(OotAbiProbe **probe);
#ifdef __cplusplus
}
#endif
#endif
