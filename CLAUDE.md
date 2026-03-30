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

## Dependencies

- `cor24-run` from sw-cor24-emulator (for VM execution)
- No Rust crate dependencies (both pa24r and pl24r are pure Rust)
