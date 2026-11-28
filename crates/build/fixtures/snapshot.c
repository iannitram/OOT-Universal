#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>

static int matches_file(const char *path, const unsigned char *expected, size_t size) {
    unsigned char actual[32];
    FILE *file = fopen(path, "rb");
    if (file == NULL || size > sizeof(actual)) {
        if (file != NULL) {
            fclose(file);
        }
        return 0;
    }

    const size_t read_size = fread(actual, 1, size, file);
    const int eof = fgetc(file) == EOF;
    const int failed = ferror(file);
    const int close_failed = fclose(file) != 0;
    if (read_size != size || !eof || failed || close_failed) {
        return 0;
    }
    for (size_t i = 0; i < size; ++i) {
        if (actual[i] != expected[i]) {
            return 0;
        }
    }
    return 1;
}

int main(void) {
    static const unsigned char manifest[] = "oot-synthetic-fixture-v1\n";
    static const unsigned char payload[] = {0x4f, 0x4f, 0x54, 0x00, 0x01, 0x7f, 0x80, 0xff};
    if (!matches_file("fixture/manifest.bin", manifest, sizeof(manifest) - 1) ||
        !matches_file("fixture/payload.bin", payload, sizeof(payload))) {
        fputs("synthetic snapshot mismatch\n", stderr);
        return 23;
    }
    puts("synthetic-snapshot-ok");
    return 0;
}
