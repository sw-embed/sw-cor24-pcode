# Feature Request: pr24p — Add Module Metadata to Runtime .spc Files

**Target repo:** [softwarewrighter/pr24p](https://github.com/softwarewrighter/pr24p)
**Depends on:** pl24r linker module metadata convention
**Priority:** Required for pl24r to resolve runtime symbols properly

## Problem

The Pascal runtime library (`pr24p/src/runtime.spc`) contains hand-written `.spc` stubs for `_p24p_write_int`, `_p24p_write_bool`, and `_p24p_write_ln`, but has no module metadata. The pl24r linker cannot distinguish which symbols are intended to be public exports of the runtime vs. internal implementation details.

Currently `runtime.spc` looks like:
```spc
; pr24p — Pascal Runtime Library
; Phase 0: Hand-written .spc stubs

.proc _p24p_write_int 1
    ...
.end

.proc _p24p_write_bool 1
    ...
.end

.proc _p24p_write_ln 0
    ...
.end
```

Without metadata, pl24r uses export-all fallback — every symbol is treated as public. This works but provides no validation that callers are using the correct symbols.

## Requested Change

Wrap `runtime.spc` in module metadata declaring the public API:

```spc
.module runtime
.export _p24p_write_int
.export _p24p_write_bool
.export _p24p_write_ln

; pr24p — Pascal Runtime Library
; Phase 0: Hand-written .spc stubs

.proc _p24p_write_int 1
    ...
.end

.proc _p24p_write_bool 1
    ...
.end

.proc _p24p_write_ln 0
    ...
.end

.endmodule
```

### What this enables

1. **Symbol validation** — pl24r can verify that app modules only reference symbols the runtime actually exports. Typos like `call _p24p_write_integer` get caught at link time instead of failing silently or crashing at runtime.

2. **Unused export warnings** — pl24r warns when a runtime symbol is exported but no module references it, helping keep the runtime lean.

3. **Future runtime growth** — As phase 1+ runtime functions are added (compiled from Pascal), some may be internal helpers. Module metadata lets the runtime expose only its public API.

### Scope

This change affects only the Phase 0 hand-written stubs in `src/runtime.spc`. When Phase 1+ runtime routines are compiled from Pascal by p24p, the compiler should emit the metadata (see the companion feature request for p24p).

## Context

- **Module metadata spec:** [pl24r CLAUDE.md](https://github.com/softwarewrighter/pl24r/blob/main/CLAUDE.md) — "Module metadata format" section
- **Linker design research:** [pl24r docs/research.txt](https://github.com/softwarewrighter/pl24r/blob/main/docs/research.txt)
- **pasm compatibility:** pasm silently skips unknown directives. Adding `.module`/`.export`/`.endmodule` to `runtime.spc` does not break direct assembly without pl24r.
- **Reference implementation:** [pl24r tests/fixtures/e2e_runtime.spc](https://github.com/softwarewrighter/pl24r/blob/main/tests/fixtures/e2e_runtime.spc) — this is what the updated `runtime.spc` should look like.

## Notes

- This is a ~6-line addition (3 `.export` + `.module` + `.endmodule`) to an existing file. Minimal risk.
- The `.module runtime` name matches the convention used in pl24r's test fixtures.
- No behavior change for the current `cat runtime.spc app.spc | pasm` workflow — pasm skips the metadata directives.
