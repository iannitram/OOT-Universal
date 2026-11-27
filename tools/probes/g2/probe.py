#!/usr/bin/env python3
"""Exercise synthetic AArch64 instructions through both pinned emitters."""
import argparse
import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
FIXTURE = Path(__file__).resolve().with_name("fixture.c")
REVISIONS = {
    "relink": "16cba0845075868d211c63674156fa37401d852e",
    "longshot": "fa2cc03aa5c2369dea53856d4b11bdb9d0299b3b",
}
SOURCE = "third_party/a64-kit/lift/"


def run(*args):
    subprocess.run(args, cwd=ROOT, check=True)


def translate(library):
    lib = ctypes.CDLL(str(library))
    lib.a64_config.argtypes = [ctypes.c_int, ctypes.c_int]
    lib.a64_config.restype = None
    lib.a64_config(1, 1)
    fn = lib.a64_translate_batch
    fn.argtypes = [
        ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_uint64),
        ctypes.c_int, ctypes.POINTER(ctypes.c_char), ctypes.c_longlong,
        ctypes.POINTER(ctypes.c_longlong), ctypes.POINTER(ctypes.c_int),
    ]
    fn.restype = ctypes.c_longlong
    instructions = {
        "add": (0x91001c00, 0),     # add x0, x0, #7
        "branch": (0xb4000040, 2),  # cbz x0, 0x10008
        "store": (0xf9000020, 0),   # str x0, [x1]
        "load": (0xf9400022, 0),    # ldr x2, [x1]
    }
    result = {}
    for name, (word, expected_flags) in instructions.items():
        words = (ctypes.c_uint32 * 1)(word)
        pcs = (ctypes.c_uint64 * 1)(0x10000)
        offsets = (ctypes.c_longlong * 2)()
        flags = (ctypes.c_int * 1)()
        output = ctypes.create_string_buffer(65536)
        size = fn(words, pcs, 1, output, len(output), offsets, flags)
        if (flags[0] != expected_flags or size <= 0 or size > len(output)
                or offsets[0] != 0 or offsets[1] != size):
            raise RuntimeError(f"unexpected translation or flags for {name}")
        result[name] = output.raw[:size].decode("ascii")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--clangxx", default="clang++")
    parser.add_argument("--child", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.child:
        print(json.dumps(translate(args.child)))
        return
    with tempfile.TemporaryDirectory(prefix="oot-g2-") as tmp:
        root = Path(tmp)
        for name, revision in REVISIONS.items():
            directory = root / name
            (directory / "suyu").mkdir(parents=True)
            for item in ("a64emit.cpp", "suyu/arm64_to_c.h"):
                try:
                    data = subprocess.check_output(
                        ["git", "show", f"{revision}:{SOURCE}{item}"], cwd=ROOT
                    )
                except subprocess.CalledProcessError as error:
                    raise RuntimeError(
                        f"cannot read {name} pinned revision {revision}; "
                        "fetch the import branches"
                    ) from error
                (directory / item).write_bytes(data)
            library = directory / ("emitter.dll" if os.name == "nt" else "emitter.so")
            cmd = [args.clangxx, "-std=c++20", "-O2", "-shared", "-Wno-everything"]
            if os.name != "nt":
                cmd.append("-fPIC")
            run(*cmd, str(directory / "a64emit.cpp"), "-o", str(library))
            emitted = json.loads(subprocess.check_output(
                [sys.executable, __file__, "--child", str(library)], cwd=ROOT
            ))
            for instruction, text in emitted.items():
                (directory / f"{instruction}.inc").write_text(text, encoding="ascii")
            exe = directory / ("probe.exe" if os.name == "nt" else "probe")
            common = ["-std=c11", "-Wall", "-Wextra", "-Werror", "-I", str(directory)]
            run(args.clang, *common, str(FIXTURE), "-o", str(exe))
            run(str(exe))
            run(args.clang, "--target=aarch64-none-elf", "-ffreestanding", *common,
                "-c", str(FIXTURE), "-o", str(directory / "probe-aarch64.o"))
            print(f"{name}: ADD, branch and memory native checks passed, "
                  "AArch64 object compiled", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"G2 probe failed: {error}", file=sys.stderr)
        sys.exit(1)
