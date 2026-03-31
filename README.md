# sw-cor24-pcode -- P-Code Tools for COR24

A Rust workspace containing the p-code toolchain for the COR24 ISA:

| Directory | What | Impl |
|-----------|------|------|
| `vm/` | P-code VM + standalone assembler (pasm) | COR24 assembly |
| `assembler/` | P-code binary assembler (.spc -> .p24) | Rust |
| `linker/` | P-code linker (merges .spc modules) | Rust |

## Pipeline

```
.spc source(s) --> linker (pl24r) --> combined.spc --> assembler (pa24r) --> .p24 --> VM (pvm.s)
```

## Quick Start

```bash
# Build and test the Rust workspace (assembler + linker)
./scripts/build.sh

# Run the VM test suite (requires cor24-run from sw-cor24-emulator)
cd vm && ./demo.sh
```

## Dependencies

- [sw-cor24-emulator](https://github.com/sw-embed/sw-cor24-emulator) -- `cor24-run` binary for VM execution

## Sibling Repos

- [sw-cor24-x-assembler](https://github.com/sw-embed/sw-cor24-x-assembler) -- COR24 assembler
- [sw-cor24-emulator](https://github.com/sw-embed/sw-cor24-emulator) -- COR24 emulator + ISA
- [sw-cor24-pascal](https://github.com/sw-embed/sw-cor24-pascal) -- Pascal compiler (emits .spc)
- [web-sw-cor24-pcode](https://github.com/sw-embed/web-sw-cor24-pcode) -- browser-based p-code debugger

## License

MIT -- see [LICENSE](LICENSE) for details.

Copyright (c) 2026 Michael A Wright
