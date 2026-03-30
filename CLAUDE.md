# sw-cor24-pcode -- Claude Instructions

## Project Overview

P-code toolchain for the COR24 ISA. Rust workspace with two crates
plus the COR24-assembly VM.

## Layout

- `vm/` -- P-code VM (pvm.s), standalone assembler (pasm.s), integrated
  assembler+VM (pvmasm.s), test suite (demo.sh + tests/)
- `assembler/` -- Rust crate `pa24r`: two-pass .spc -> .p24 binary assembler
- `linker/` -- Rust crate `pl24r`: merges multiple .spc modules into one

## Build & Test

```bash
# Rust workspace
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings

# VM tests (requires cor24-run)
cd vm && ./demo.sh
```

## Pipeline

```
.spc sources -> pl24r (link) -> combined.spc -> pa24r (assemble) -> .p24 -> pvm.s (execute)
```

## CRITICAL: pasm.s and pa24r must stay in sync

`pasm.s` (COR24-native assembler) and `pa24r` (Rust assembler) are two
implementations of the same two-pass .spc assembler. They MUST remain
bug-compatible and at feature parity. Any opcode, directive, or encoding
change in one must be mirrored in the other. When modifying either:

1. Make the equivalent change in both implementations
2. Run both test suites to verify identical output
3. If adding a new opcode/directive, update the opcode table in both

The COR24-native pasm.s proves the toolchain is self-hosting on real
hardware; pa24r provides 4,447x faster assembly for development builds.
Neither is optional.

## Dependencies

- `cor24-run` from sw-cor24-emulator (for VM execution)
- No Rust crate dependencies (both pa24r and pl24r are pure Rust)
