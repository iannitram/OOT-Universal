# oot-settings-codec

A bounded JSON codec for the dependency-free `oot-contracts` settings contract. It performs no file access. `encode` and `decode` both enforce `MAX_SETTINGS_BYTES` (4096 bytes).

Schema version 1 has this exact shape; every value is required and unknown fields are rejected:

```json
{"schema_version":1,"resolution":{"kind":"explicit","width":1280,"height":720},"frame_cap":60,"upscaler":"off","extras_enabled":false}
```

`resolution.kind` is `native` (with no other resolution fields) or `explicit` (with positive `width` and `height`). `frame_cap` is the number `30` or `60`; `upscaler` is `off` or `platform_default`. Defaults are explicitly encoded as native resolution, 60 FPS, off and false. Unsupported schema versions and malformed or invalid values are rejected.
