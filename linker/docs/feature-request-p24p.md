# Feature Request: p24p — Emit Module Metadata in .spc Output

**Target repo:** [softwarewrighter/p24c](https://github.com/softwarewrighter/p24c) (a.k.a. p24p)
**Depends on:** pl24r linker module metadata convention
**Priority:** Required for pl24r to be useful in the real toolchain

## Problem

The Pascal compiler (p24p) currently emits plain `.spc` files containing only `.proc`, `.global`, `.data`, and `.const` directives. There is no module boundary or symbol visibility information in the output.

The pl24r linker ([softwarewrighter/pl24r](https://github.com/softwarewrighter/pl24r)) defines a module metadata protocol using four new `.spc` directives for linking. Without compiler support, pl24r falls back to "export-all" mode — treating every symbol as public — which defeats the purpose of having a linker that can validate cross-module dependencies.

## Requested Change

When p24p compiles a Pascal source file, wrap the output `.spc` in module metadata:

```spc
.module <module-name>
.export main
.export <any-other-public-symbols>
.extern _p24p_write_int
.extern _p24p_write_ln
.extern _p24p_write_bool

; ... existing .global, .data, .const, .proc output ...

.endmodule
```

### Specific requirements

1. **`.module <name>`** — Emit at the top of the `.spc` output. The module name should be derived from the source filename (e.g., `hello.pas` → `.module hello`).

2. **`.export <sym>`** — Emit for every symbol that should be visible to other modules. For a typical app this is just `main`. For a library module, this would be the public procedure names.

3. **`.extern <sym>`** — Emit for every symbol the module calls but does not define. These are the runtime library functions and any user library functions. The compiler already knows which calls are unresolved — emit them as `.extern` directives.

4. **`.endmodule`** — Emit at the end of the `.spc` output.

### Example

Given this Pascal source:
```pascal
program hello;
begin
  writeln(42);
end.
```

Current p24p output:
```spc
.proc main 0
    push 42
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end
```

Desired p24p output:
```spc
.module hello
.export main
.extern _p24p_write_int
.extern _p24p_write_ln

.proc main 0
    push 42
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end

.endmodule
```

## Context

- **Module metadata spec:** [pl24r CLAUDE.md](https://github.com/softwarewrighter/pl24r/blob/main/CLAUDE.md) — "Module metadata format" section
- **Linker design research:** [pl24r docs/research.txt](https://github.com/softwarewrighter/pl24r/blob/main/docs/research.txt)
- **pasm compatibility:** pasm silently skips unknown directives, so adding metadata to `.spc` output is backward-compatible. Files with metadata still assemble correctly even without pl24r in the pipeline.

## Notes

- The metadata directives are language-agnostic — they're part of the `.spc` format, not Pascal-specific. Any future COR24 compiler frontend should emit the same directives.
- pl24r's export-all fallback means existing `.spc` files without metadata continue to work. This change is additive, not breaking.
- The compiler already tracks which symbols are defined vs. called but not defined — the information needed for `.export`/`.extern` should already be available internally.
