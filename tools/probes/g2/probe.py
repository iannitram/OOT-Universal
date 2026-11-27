#!/usr/bin/env python3
"""Exercise one synthetic AArch64 instruction through both pinned emitters."""
import argparse
import ctypes
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
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
    lib.a64_config(1, 1)
    fn = lib.a64_translate_batch
    fn.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p,
                   ctypes.c_longlong, ctypes.c_void_p, ctypes.c_void_p]
    fn.restype = ctypes.c_longlong
    words = (ctypes.c_uint32 * 1)(0x91001c00)  # add x0, x0, #7
    pcs = (ctypes.c_uint64 * 1)(0x10000)
    offsets = (ctypes.c_longlong * 2)()
    flags = (ctypes.c_int * 1)()
    output = ctypes.create_string_buffer(65536)
    size = fn(words, pcs, 1, output, len(output), offsets, flags)
    if flags[0] or size <= 0 or size > len(output) or offsets[0] != 0 or offsets[1] != size:
        raise RuntimeError("instruction was not translated as an ordinary instruction")
    return output.raw[:size].decode("ascii")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--clangxx", default="clang++")
    parser.add_argument("--child", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.child:
        print(translate(args.child), end="")
        return
    with tempfile.TemporaryDirectory(prefix="oot-g2-") as tmp:
        root = Path(tmp)
        for name, revision in REVISIONS.items():
            directory = root / name
            (directory / "suyu").mkdir(parents=True)
            for item in ("a64emit.cpp", "suyu/arm64_to_c.h"):
                data = subprocess.check_output(["git", "show", f"{revision}:{SOURCE}{item}"], cwd=ROOT)
                (directory / item).write_bytes(data)
            library = directory / ("emitter.dll" if os.name == "nt" else "emitter.so")
            cmd = [args.clangxx, "-std=c++20", "-O2", "-shared", "-Wno-everything"]
            if os.name != "nt":
                cmd.append("-fPIC")
            run(*cmd, str(directory / "a64emit.cpp"), "-o", str(library))
            emitted = subprocess.check_output([sys.executable, __file__, "--child", str(library)], cwd=ROOT).decode("ascii")
            # This fixture only models the field touched by the selected integer instruction.
            source = directory / "probe.c"
            source.write_text("typedef unsigned long long uint64_t;\n"
                              "typedef struct { uint64_t x[32]; } Ctx;\n"
                              "static void execute(Ctx *c) {\n" + emitted + "}\n"
                              "int main(void) { Ctx c = {0}; c.x[0] = 35; execute(&c); "
                              "if (c.x[0] != 42) return 1; c.x[0] = ~(uint64_t)0; "
                              "execute(&c); return c.x[0] != 6; }\n", encoding="ascii")
            exe = directory / ("probe.exe" if os.name == "nt" else "probe")
            run(args.clang, "-std=c11", "-Wall", "-Wextra", "-Werror", str(source), "-o", str(exe))
            run(str(exe))
            run(args.clang, "--target=aarch64-none-elf", "-std=c11", "-Wall", "-Wextra", "-Werror",
                "-c", str(source), "-o", str(directory / "probe-aarch64.o"))
            print(f"{name}: translated ADD, native execution passed, AArch64 object compiled", flush=True)


if __name__ == "__main__":
    main()
