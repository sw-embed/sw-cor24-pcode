Implement MEMCPY and MEMSET as VM opcodes in the p-code VM (Tier 1 primitives from vm/docs/feature-request-interpreter-primitives.md).

## What to implement

### MEMCPY (opcode 0x70)
- Stack effect: ( src dst len -- )
- Copy len bytes from src to dst with memmove semantics (handle overlapping regions correctly)
- If len is 0, do nothing

### MEMSET (opcode 0x71)
- Stack effect: ( dst val len -- )
- Fill len bytes at dst with byte value val
- If len is 0, do nothing

## Where to implement

1. **pvm.s** (COR24 assembly VM) — add opcode handlers for 0x70 and 0x71 in the dispatch table
2. **pasm.s** (COR24 native assembler) — add `memcpy` and `memset` mnemonics to the opcode table
3. **pa24r** (Rust assembler) — add `memcpy` and `memset` mnemonics to the opcode table
4. **vm/design.md** — document the new opcodes in the instruction set table

## CRITICAL: pasm.s and pa24r must stay in sync
Both assemblers must recognize the new mnemonics and emit identical bytecodes. Update opcode tables in both.

## Testing
- Add .spc test programs in vm/tests/ that exercise both opcodes (basic copy, overlapping copy, zero-length, basic fill)
- Run both assembler test suites
- Run vm/demo.sh