Implement MEMCMP as a VM opcode in the p-code VM (Tier 1 primitive from vm/docs/feature-request-interpreter-primitives.md).

## What to implement

### MEMCMP (opcode 0x72)
- Stack effect: ( a b len -- result )
- Compare len bytes at a and b
- Push 0 if equal, negative if a<b, positive if a>b (lexicographic byte comparison)
- If len is 0, push 0

## Where to implement

1. **pvm.s** (COR24 assembly VM) — add opcode handler for 0x72 in the dispatch table
2. **pasm.s** (COR24 native assembler) — add `memcmp` mnemonic to the opcode table
3. **pa24r** (Rust assembler) — add `memcmp` mnemonic to the opcode table
4. **vm/design.md** — document the new opcode

## CRITICAL: pasm.s and pa24r must stay in sync

## Testing
- Add .spc test program in vm/tests/ for memcmp (equal, less-than, greater-than, zero-length)
- Run both assembler test suites and vm/demo.sh