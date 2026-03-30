# pa24r

Rust-native two-pass p-code assembler for the COR24 toolchain. Reads linked `.spc` (symbolic p-code) source and produces `.p24` binary files.

## Pipeline Position

```
Pascal source -> p24p (compile) -> .spc -> pl24r (link) -> pa24r (assemble) -> .p24 -> pvm.s (execute)
```

pa24r replaces the COR24-hosted `pasm.s` for build-time assembly. By the time pa24r sees the `.spc`, all symbols are resolved by the linker (`pl24r`).

## Usage

### CLI

```bash
pa24r input.spc -o output.p24
pa24r input.spc -o output.p24 --verbose    # print assembly stats
pa24r input.spc -o output.p24 --dump        # hex dump of output
```

### Library API

```rust
use pa24r::{assemble, assemble_to_p24, load_p24, relocate_data_refs};

// Assemble .spc source to structured result
let result = assemble(source);

// Assemble .spc source to .p24 binary
let binary = assemble_to_p24(source)?;

// Load a .p24 binary back into segments
let image = load_p24(&binary)?;

// Relocate data references when loading into emulator memory
relocate_data_refs(&mut code, code_size, data_size, load_addr);
```

## .p24 Binary Format

18-byte header followed by code bytes then data bytes:

| Offset | Size | Field |
|--------|------|-------|
| 0x00 | 4 | Magic: `P24\0` |
| 0x04 | 1 | Version (1) |
| 0x05 | 3 | Entry point (LE) |
| 0x08 | 3 | Code size (LE) |
| 0x0B | 3 | Data size (LE) |
| 0x0E | 3 | Global count (LE) |
| 0x11 | 1 | Flags (0x00) |

All 3-byte fields are little-endian, matching the COR24 24-bit word size.

## Building

```bash
cargo build
cargo test                     # 52 tests (32 unit + 20 integration)
cargo clippy --all-targets --all-features -- -D warnings
```

Rust edition 2024. No external dependencies.

## Related Projects

- [pv24a](https://github.com/sw-vibe-coding/pv24a) -- P-code VM and assembler (opcode ground truth)
- [web-dv24r](https://github.com/softwarewrighter/web-dv24r) -- P-code debugger (browser UI, consumer of pa24r)
- [p24p](https://github.com/softwarewrighter/p24p) -- Pascal compiler (emits .spc)
- [pl24r](https://github.com/softwarewrighter/pl24r) -- Text-level .spc linker
- [pr24p](https://github.com/softwarewrighter/pr24p) -- Pascal runtime (.spc sources)
