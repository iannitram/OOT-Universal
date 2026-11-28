#ifndef OOT_SHARED_RUNTIME_API_H
#define OOT_SHARED_RUNTIME_API_H

#include "../abi/oot_abi_probe.h"

typedef struct OotRuntimeApi {
    uint32_t version;
    int32_t (*create)(OotAbiCallback, void *, OotAbiProbe **);
    int32_t (*call)(OotAbiProbe *, int32_t, int32_t *);
    int32_t (*read)(OotAbiProbe *, uint64_t, uint64_t, uint8_t *);
    int32_t (*write)(OotAbiProbe *, uint64_t, uint64_t, const uint8_t *);
    int32_t (*shutdown)(OotAbiProbe **);
} OotRuntimeApi;

enum { OOT_RUNTIME_API_VERSION = 1, OOT_SHARED_OK = 0, OOT_SHARED_INVALID = 1 };

typedef int32_t (*oot_shared_run_fn)(const OotRuntimeApi *);
typedef uint32_t (*oot_shared_version_fn)(void);

#endif
