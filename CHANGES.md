# Changelog

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
