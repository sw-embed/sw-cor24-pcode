# Changelog

## 2026-04-01 -- MEMCPY and MEMSET VM opcodes (Tier 1)

- Added MEMCPY (0x70) and MEMSET (0x71) as VM opcodes across all
  implementations: pvm.s, pvmasm.s, pasm.s, pa24r, and pv24t
- MEMCPY: ( src dst len -- ) with memmove semantics (overlapping-safe)
- MEMSET: ( dst val len -- ) fills len bytes with val
- Zero-length operations are no-ops (only pop arguments)
- Updated dispatch tables, mnemonic tables, and opcode enums
- Added 3 test programs: basic copy, basic fill, overlapping copy
- Updated design.md opcode table
- Fixed pre-existing clippy warnings in tracer (dead_code, write_with_newline, map_clone)
- All 152 Rust tests + 15 VM tests pass

## 2026-03-29 -- Repository created from fork + merge

- Forked [pv24a](https://github.com/softwarewrighter/pv24a) to
  [sw-cor24-pcode](https://github.com/sw-embed/sw-cor24-pcode)
- Moved VM files (pvm.s, pasm.s, pvmasm.s, demo.sh, tests/, examples/)
  into `vm/` subdirectory
- Copied [pa24r](https://github.com/softwarewrighter/pa24r) (p-code
  assembler) into `assembler/`
- Copied [pl24r](https://github.com/softwarewrighter/pl24r) (p-code
  linker) into `linker/`
- Set up Rust workspace with assembler and linker as members
- Updated assembler integration test paths to reference `vm/` fixtures
- Added `scripts/build.sh` for workspace build + clippy + test
- All 152 tests pass (52 assembler + 100 linker)
