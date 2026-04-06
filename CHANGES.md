# Changelog

## 2026-04-06 -- XLOADG/XSTOREG cross-unit global access

- Added XLOADG (0x75) and XSTOREG (0x76) opcodes for cross-unit global access
- Encoding: D8_O8 (opcode + unit_id + offset = 3 bytes)
- VM handlers in pvm.s/pvmasm.s: look up unit_table[unit_id].global_base,
  compute gp + (global_base + offset) * 3, load/store eval stack TOS
- Added to pa24r (opcode enum, from_mnemonic, opcode_size), pasm.s and
  pvmasm.s mnemonic tables (type=D8_O8), pv24t tracer (decode + trap)
- Extended dispatch tables from 117 to 119 entries, bounds check updated
- All 188 Rust tests + 18 VM tests pass

## 2026-04-06 -- Linker --unit mode

- Added --unit flag to pl24r for unit compilation mode
- Unit mode preserves .unit/.import/.export/.extern directives in output
  (static-link mode strips them as before)
- Validates: every .export references a defined proc/global, every .extern
  is either resolved internally (removed) or has a matching .import
- Parser now handles .unit, .import, .endunit directives
- .export/.extern accept optional nargs parameter (stripped by linker)
- link_unit() and emit_unit() public API for programmatic use
- 6 new tests: directive preservation, import/extern handling, internal
  resolution, error cases, end-to-end assembly to v2 .p24
- All 188 Rust tests pass

## 2026-04-06 -- VM .p24m multi-unit image support

- pvm.s _start detects .p24m magic ("P24M") at code_ptr and parses the
  header: reads entry_point, unit_count, unit_table, IRT base, code
  segment base, globals segment base
- Extended vm_state from 36 to 42 bytes (14 words): added unit_table_ptr
  (offset 36) and p24m_base (offset 39) for runtime lookups
- Extended .p24m format: header now 27 bytes (was 21), added code_offset
  and globals_offset fields; unit table entries now 9 bytes (was 6),
  added per-unit irt_off for direct IRT lookup
- Fixed critical op_xcall bug: return_pc was being overwritten by slot
  value due to incorrect COR24 stack ordering — save return_pc to
  xcall_temps before popping slot
- Backward compatible: v1 .p24 and raw bytecode still work (magic check
  falls through to init_raw_code)
- End-to-end verified: app unit xcalls mathlib.double(21)→42='*'
  across two independently compiled .p24 units via .p24m image
- pvmasm.s vm_state kept in sync (42 bytes)
- Added t17 multi-unit test .spc files (app + lib)
- All 176 Rust tests + 18 VM tests pass

## 2026-04-06 -- P24 multi-unit loader (p24-load)

- New Rust binary crate loader/ (p24-load) in workspace
- Reads multiple v2 .p24 files, resolves cross-unit imports against
  exports, produces a .p24m multi-unit image
- Features: sequential code layout, global segment partitioning,
  import resolution via name matching, IRT construction, LOADG/STOREG/
  ADDRG operand patching with global partition offsets
- .p24m format: magic "P24M", unit table (base_addr + global_base),
  per-unit IRT (import_count + absolute addresses), concatenated
  code/data/globals
- parse_p24m() for loading and verifying .p24m images
- Error handling: unresolved imports, duplicate unit names, hash
  collisions, address overflow
- Added Debug derive to pa24r::LoadedImage
- 10 loader tests: two-unit linking, IRT resolution, global patching,
  entry point, error cases, round-trip, v1 compatibility
- All 176 Rust tests pass

## 2026-04-06 -- XCALL VM handler and IRT dispatch

- Added op_xcall handler to pvm.s and pvmasm.s: reads 16-bit IRT slot
  from code stream, looks up absolute target address from IRT[slot],
  builds call frame with caller unit_id encoded in static_link high byte,
  sets pc=target
- Extended vm_state from 27 to 36 bytes (9→12 words): added irt_base,
  unit_count, current_unit fields
- Extended dispatch table from 116 to 117 entries (0x00-0x74)
- Updated op_ret to detect cross-unit returns: checks static_link high
  byte, restores current_unit on cross-unit return
- Extended ret_temps from 12 to 15 bytes (added static_link save slot)
- Added sys 7 (SET_IRT_BASE): pops address from eval stack, sets
  vm_state.irt_base — enables p-code programs to configure the IRT
- Added xcall_temps (6 bytes) for handler scratch storage
- Fixed COR24 ISA issues: shr→sra, mov fp,r0→push r0;pop fp
- Added sys_led trampoline (sys_led_j) for branch distance overflow
- Added t16-xcall VM test: allocates IRT on heap, writes target offset,
  sets irt_base via sys 7, xcalls through slot 0, verifies return
- All 165 Rust tests + 18 VM tests pass

## 2026-04-05 -- P24 v2 binary format with export/import tables

- Extended .p24 binary format to v2 with unit export and import tables
  for cross-unit procedure calls (load-time resolution design)
- Added XCall opcode (0x74) with Imm16 encoding (3 bytes: opcode + slot16)
  to pa24r, pasm.s, pvmasm.s, and pv24t (tracer traps; VM handler in Phase 2)
- New assembler directives in unit mode: .unit, .import, .export, .extern
  - .unit <name>: declares source as a named unit (triggers v2 output)
  - .import <unit>: declares dependency on external unit
  - .export <sym> [nargs]: marks procedure for export
  - .extern <sym> [nargs]: declares imported procedure, assigns slot index
- V2 .p24 header adds: export table (hash + offset), import table
  (unit_hash + name_hash + slot), string table, unit name
- Backward compatible: v1 files assemble/load unchanged; v2 only emitted
  when .unit directive is present
- Added Imm16 encoding type (type=5) to pasm.s and pvmasm.s mnemonic tables
  and both pass 1 (size) / pass 2 (emit) handlers
- Added FNV-1a 16-bit hash function for export/import name matching
- Made opcode_size() public for use in tests
- 10 new integration tests for v2 format, xcall encoding, round-trip
- Added design document: docs/load-time-resolution.md
- All 165 Rust tests pass

## 2026-04-01 -- JMP_IND VM opcode (Tier 2)

- Added JMP_IND (0x73) as VM opcode across all implementations:
  pvm.s, pvmasm.s, pasm.s, pa24r, and pv24t
- JMP_IND: ( addr -- ) indirect jump to address on eval stack
- Enables efficient dispatch tables for interpreters and computed gotos
- Updated dispatch tables (116 entries), mnemonic tables, opcode enums
- Added t15-jmp_ind test: basic indirect jump, chained jump, computed target
- Updated design.md opcode table and control flow section
  (reserved range now 0x74-0xFF)
- Fix: pvmasm.s was missing jmp_ind handler, dispatch entry, and mnemonic
  (original commit only added to pvm.s and pasm.s)
- All 152 Rust tests + 17 VM tests pass

## 2026-04-01 -- MEMCMP VM opcode (Tier 1)

- Added MEMCMP (0x72) as VM opcode across all implementations:
  pvm.s, pvmasm.s, pasm.s, pa24r, and pv24t
- MEMCMP: ( a b len -- result ) lexicographic byte comparison
  pushes 0 if equal, -1 if a<b, 1 if a>b
- Zero-length compare returns 0 (equal)
- Updated dispatch tables (115 entries), mnemonic tables, opcode enums
- Added t14-memcmp test: equal, less-than, greater-than, zero-length
- Updated design.md opcode table (reserved range now 0x73-0xFF)
- All 152 Rust tests + 16 VM tests pass

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
