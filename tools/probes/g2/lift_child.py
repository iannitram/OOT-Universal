"""Run the pinned function lifter on an authored AArch64 control-flow graph."""
import importlib
import sys
import types
from pathlib import Path

# The parent built the upstream emitter; avoid its build helper's host assumptions.
build = types.ModuleType("build_emit")
build.build = lambda: sys.argv[2]
sys.modules["build_emit"] = build
sys.path.insert(0, sys.argv[1])
lift = importlib.import_module("a64_lift")
emit = importlib.import_module("a64emit")

class Image:
    entries = {0x10000, 0x20000, 0x20020}
    hooks = {}
    # cbz x0, zero; add x0, x0, #7; ret; zero: add x0, x0, #3; ret
    words = {
        **dict(zip(range(0x10000, 0x10014, 4),
                   (0xb4000060, 0x91001c00, 0xd65f03c0, 0x91000c00, 0xd65f03c0))),
        # Save the entry LR, call the separate callee, then restore LR and return.
        0x20000: 0xaa1e03e9,  # mov x9, x30
        0x20004: 0x91001421,  # add x1, x1, #5
        0x20008: 0x94000006,  # bl 0x20020
        0x2000c: 0x91001c00,  # add x0, x0, #7
        0x20010: 0xaa0903fe,  # mov x30, x9
        0x20014: 0xd65f03c0,  # ret
        0x20020: 0x91001400,  # add x0, x0, #5
        0x20024: 0xaa1e03e3,  # mov x3, x30
        0x20028: 0xd65f03c0,  # ret
    }
    def in_code(self, pc):
        return pc in self.words
    def word(self, pc):
        return self.words[pc]

lifter = lift.Lifter(Image())
lifter.discover()
expected = {
    0x10000: {0x10000, 0x10004, 0x10008, 0x1000c, 0x10010},
    0x20000: set(range(0x20000, 0x20018, 4)),
    0x20020: {0x20020, 0x20024, 0x20028},
}
for entry, addresses in expected.items():
    body, _ = lifter.funcs[entry]
    if set(body) != addresses:
        raise RuntimeError(f"function discovery missed an instruction at {entry:#x}")
ops = [(pc, word) for entry in sorted(expected)
       for pc, (kind, word) in lifter.funcs[entry][0].items() if kind[0] == "op"]
texts, flags = emit.translate(ops)
if len(texts) != len(ops) or len(flags) != len(ops) or any(flags):
    raise RuntimeError("emitter rejected an instruction")
lookup = {(word, pc) if lift.pc_relative(word) else word: (lift.op_text(text, set()), flag)
          for (pc, word), text, flag in zip(ops, texts, flags)}
function, refs = lifter.emit_function(0x10000, lookup)
if refs or lifter.unhandled:
    raise RuntimeError("unexpected function references or unhandled instructions")
caller, caller_refs = lifter.emit_function(0x20000, lookup)
callee, callee_refs = lifter.emit_function(0x20020, lookup)
expected_callee = "f_00020020"
if caller_refs != {expected_callee} or callee_refs or lifter.unhandled:
    raise RuntimeError("direct BL did not emit exactly the discovered callee reference")
Path(sys.argv[3]).write_text(function + callee + caller, encoding="ascii")
