#define _GNU_SOURCE
#include "runtime_api.h"

#include <dlfcn.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

const OotRuntimeApi *oot_runtime_api(void);

static int fail(const char *message) {
    fprintf(stderr, "shared-library bridge probe: %s\n", message);
    return EXIT_FAILURE;
}

int main(int argc, char **argv) {
    if (argc != 2 || strchr(argv[1], '/') == NULL) {
        fprintf(stderr, "usage: %s LIBRARY_PATH\n", argv[0]);
        return EXIT_FAILURE;
    }
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return EXIT_FAILURE;
    }

    dlerror();
    oot_shared_run_fn run = (oot_shared_run_fn)dlsym(library, "oot_shared_run");
    const char *error = dlerror();
    if (error != NULL || run == NULL) {
        fprintf(stderr, "scenario lookup failed: %s\n", error ? error : "null symbol address");
        (void)dlclose(library);
        return EXIT_FAILURE;
    }
    dlerror();
    oot_shared_version_fn version = (oot_shared_version_fn)dlsym(library, "oot_shared_version");
    error = dlerror();
    if (error != NULL || version == NULL || version() != OOT_RUNTIME_API_VERSION) {
        (void)dlclose(library);
        return fail("missing or unsupported shared-library version");
    }

    OotRuntimeApi api = *oot_runtime_api();
    if (run(NULL) != OOT_SHARED_INVALID) {
        (void)dlclose(library);
        return fail("null runtime table was not rejected");
    }
    api.version++;
    if (run(&api) != OOT_SHARED_INVALID) {
        (void)dlclose(library);
        return fail("wrong runtime table version was not rejected");
    }
    api = *oot_runtime_api();
    api.write = NULL;
    if (run(&api) != OOT_SHARED_INVALID) {
        (void)dlclose(library);
        return fail("runtime table with a missing function was not rejected");
    }
    if (run(oot_runtime_api()) != OOT_SHARED_OK) {
        (void)dlclose(library);
        return fail("Rust ABI scenario or invalid-request control flow failed");
    }
    if (dlclose(library) != 0) {
        fprintf(stderr, "dlclose failed: %s\n", dlerror());
        return EXIT_FAILURE;
    }
    puts("PASS: loaded C library called Rust ABI through version 1 function table; callback, scratch bounds, invalid requests, shutdown and dlclose verified");
    return EXIT_SUCCESS;
}
