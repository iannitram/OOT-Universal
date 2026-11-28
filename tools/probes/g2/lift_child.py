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
    entries = {0x10000}
    hooks = {}
    # cbz x0, zero; add x0, x0, #7; ret; zero: add x0, x0, #3; ret
    words = dict(zip(range(0x10000, 0x10014, 4),
                     (0xb4000060, 0x91001c00, 0xd65f03c0, 0x91000c00, 0xd65f03c0)))
    def in_code(self, pc):
        return pc in self.words
    def word(self, pc):
        return self.words[pc]

lifter = lift.Lifter(Image())
lifter.discover()
body, _ = lifter.funcs[0x10000]
assert set(body) == set(Image.words), "function discovery missed an instruction"
ops = [(pc, word) for pc, (kind, word) in body.items() if kind[0] == "op"]
texts, flags = emit.translate(ops)
assert not any(flags), "emitter rejected an instruction"
lookup = {(word, pc) if lift.pc_relative(word) else word: (lift.op_text(text, set()), flag)
          for (pc, word), text, flag in zip(ops, texts, flags)}
function, refs = lifter.emit_function(0x10000, lookup)
assert not refs and not lifter.unhandled, "unexpected function references or unhandled instructions"
Path(sys.argv[3]).write_text(function, encoding="ascii")
