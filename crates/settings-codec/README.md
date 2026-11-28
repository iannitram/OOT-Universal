# oot-settings-codec

A bounded JSON codec and caller-directed file persistence for the dependency-free `oot-contracts` settings contract. `encode` and `decode` both enforce `MAX_SETTINGS_BYTES` (4096 bytes).

`file::read` reads no more than 4097 bytes before decoding. `file::write_atomic` creates a unique short-name sibling staging file with `create_new`, writes and syncs it, closes it, then renames it over the destination. On Unix the staging file is created with mode `0600`. Failed staging or replacement attempts remove the temporary file without deleting the existing destination. Callers must supply a trusted destination directory; this module does not select platform settings directories or enforce a path sandbox. The file is synced before replacement, but the parent directory is not synced, so portable standard-library APIs do not guarantee the rename survives sudden power loss.

Schema version 1 has this exact shape; every value is required and unknown fields are rejected:

```json
{"schema_version":1,"resolution":{"kind":"explicit","width":1280,"height":720},"frame_cap":60,"upscaler":"off","extras_enabled":false}
```

`resolution.kind` is `native` (with no other resolution fields) or `explicit` (with positive `width` and `height`). `frame_cap` is the number `30` or `60`; `upscaler` is `off` or `platform_default`. Defaults are explicitly encoded as native resolution, 60 FPS, off and false. Unsupported schema versions and malformed or invalid values are rejected.
