# pl24r — P-Code Linker for COR24

A Rust CLI tool that combines multiple `.spc` (symbolic p-code assembler) text files into one merged `.spc` file. This is a language-agnostic offline text-level linker for the COR24 p-code VM.

pl24r operates on `.spc` files regardless of which high-level language compiler produced them. Pascal (via p24p) is the first frontend, but the linker supports any language targeting COR24.

## Usage

```
pl24r [OPTIONS] <file.spc>...

Options:
  -o <path>      Write output to file (default: stdout)
  -v, --verbose  Print diagnostics to stderr
  -h, --help     Show this help
```

Link a runtime library with an application module:

```bash
pl24r runtime.spc app.spc -o combined.spc
```

## Pipeline

pl24r is one stage in the COR24 build pipeline:

```
HLL source(s) → compiler → .spc file(s)  ─┐
Runtime .spc                               ├→ pl24r → combined .spc → pasm → .p24 → COR24 VM
Library .spc modules                       ┘
```

A convenience script runs the full pipeline (link, assemble, run):

```bash
./scripts/pipeline.sh runtime.spc app.spc
```

## Module Metadata

pl24r uses language-agnostic `.spc` directives to control linking:

```asm
.module app
.export main
.extern _p24p_write_int

.proc main 0
    push 42
    call _p24p_write_int
    halt
.end

.endmodule
```

| Directive    | Purpose                              |
|------------- |--------------------------------------|
| `.module`    | Declare module identity              |
| `.export`    | Mark symbol as visible to other modules |
| `.extern`    | Declare external symbol dependency   |
| `.endmodule` | End module boundary                  |

Files without metadata use an export-all fallback: every `.proc`, `.global`, and `.data` symbol is exported automatically.

## Linking Behavior

- The main module (containing `halt` or the entry point) is emitted first so the VM starts execution at code offset 0.
- Duplicate symbol definitions across modules produce an error.
- Unresolved `.extern` references produce an error.
- Verbose mode (`-v`) prints module discovery, symbol resolution, and merge diagnostics to stderr.

## Build

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

## Related Projects

- [p24p](https://github.com/softwarewrighter/p24p) — Pascal compiler targeting COR24, emits `.spc`
- [pr24p](https://github.com/softwarewrighter/pr24p) — Pascal runtime library (`.spc`)
- [pv24a](https://github.com/sw-vibe-coding/pv24a) — P-code VM and `pasm` assembler

## License

See repository for license details.
