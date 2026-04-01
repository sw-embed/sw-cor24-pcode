Implement JMP_IND as a VM opcode in the p-code VM (Tier 2 primitive from vm/docs/feature-request-interpreter-primitives.md).

## What to implement

### JMP_IND (opcode 0x73)
- Stack effect: ( addr -- )
- Jump to address on top of eval stack (indirect/computed jump)
- Enables efficient dispatch tables for interpreters (statement dispatch, opcode dispatch) and computed gotos

## Where to implement

1. **pvm.s** (COR24 assembly VM) — add opcode handler for 0x73 in the dispatch table
2. **pasm.s** (COR24 native assembler) — add `jmp_ind` mnemonic to the opcode table
3. **pa24r** (Rust assembler) — add `jmp_ind` mnemonic to the opcode table
4. **vm/design.md** — document the new opcode

## CRITICAL: pasm.s and pa24r must stay in sync

## Testing
- Add .spc test program in vm/tests/ for jmp_ind (basic indirect jump, dispatch table pattern)
- Run both assembler test suites and vm/demo.sh