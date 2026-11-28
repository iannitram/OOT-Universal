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
    entries = {0x10000, 0x20000, 0x20010}
    hooks = {}
    # cbz x0, zero; add x0, x0, #7; ret; zero: add x0, x0, #3; ret
    words = {
        **dict(zip(range(0x10000, 0x10014, 4),
                   (0xb4000060, 0x91001c00, 0xd65f03c0, 0x91000c00, 0xd65f03c0))),
        # add x1, x1, #5; bl 0x20010; ret; callee: add x0, x0, #5; ret
        0x20000: 0x91001421,
        0x20004: 0x94000003,
        0x20008: 0xd65f03c0,
        0x20010: 0x91001400,
        0x20014: 0xd65f03c0,
    }
    def in_code(self, pc):
        return pc in self.words
    def word(self, pc):
        return self.words[pc]

lifter = lift.Lifter(Image())
lifter.discover()
expected = {
    0x10000: {0x10000, 0x10004, 0x10008, 0x1000c, 0x10010},
    0x20000: {0x20000, 0x20004, 0x20008},
    0x20010: {0x20010, 0x20014},
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
callee, callee_refs = lifter.emit_function(0x20010, lookup)
expected_callee = "f_00020010"
if caller_refs != {expected_callee} or callee_refs:
    raise RuntimeError("direct BL did not emit exactly the discovered callee reference")
if f"{expected_callee}(c);" not in caller or "c->x[30] = 0x20008ULL;" not in caller:
    raise RuntimeError(f"direct BL emission did not set the guest link register and call the callee: {caller}")
Path(sys.argv[3]).write_text(function + callee + caller, encoding="ascii")
