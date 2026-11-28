#include <dlfcn.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef int32_t (*transform_fn)(int32_t);
typedef uint32_t (*version_fn)(void);

static int fail(const char *message) {
    fprintf(stderr, "shared-library probe: %s\n", message);
    return EXIT_FAILURE;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s LIBRARY_PATH\n", argv[0]);
        return EXIT_FAILURE;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return EXIT_FAILURE;
    }

    dlerror();
    transform_fn transform = (transform_fn)dlsym(library, "oot_shared_transform");
    const char *error = dlerror();
    if (error != NULL || transform == NULL) {
        fprintf(stderr, "required symbol lookup failed: %s\n",
                error != NULL ? error : "null symbol address");
        (void)dlclose(library);
        return EXIT_FAILURE;
    }

    dlerror();
    version_fn version = (version_fn)dlsym(library, "oot_shared_version");
    error = dlerror();
    if (error == NULL && version != NULL && version() != UINT32_C(1)) {
        (void)dlclose(library);
        return fail("unexpected optional library version");
    }

    const int32_t input = INT32_C(42);
    const int32_t expected = input ^ INT32_C(0x13579bdf);
    if (transform(input) != expected) {
        (void)dlclose(library);
        return fail("operation returned an unexpected value");
    }

    dlerror();
    (void)dlsym(library, "oot_shared_symbol_that_does_not_exist");
    error = dlerror();
    if (error == NULL) {
        (void)dlclose(library);
        return fail("missing symbol lookup unexpectedly succeeded");
    }

    if (dlclose(library) != 0) {
        fprintf(stderr, "dlclose failed: %s\n", dlerror());
        return EXIT_FAILURE;
    }

    printf("PASS: transform(%" PRId32 ") = %" PRId32 "; missing symbol rejected; library unloaded\n",
           input, expected);
    return EXIT_SUCCESS;
}
