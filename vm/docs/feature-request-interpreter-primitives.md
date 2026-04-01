# Feature Request: pv24a VM — Interpreter-Assisting Primitives

**Target repo:** sw-cor24-pcode
**Component:** pv24a (p-code VM)
**Requested by:** sw-cor24-basic (COR24 BASIC v1 interpreter)
**Priority:** Medium — implement as p-code library routines first, promote to VM opcodes if profiling shows they are hot paths across multiple languages

## Motivation

The COR24 BASIC interpreter (and future interpreters: Lisp, APL, Forth,
scripting) will run on the p-code VM. Interpreters have repeated
low-level needs that are currently not covered by VM opcodes:

- Byte-level buffer manipulation (shifting program lines on insert/delete)
- Block clearing (NEW command, buffer initialization)
- Dispatch table support (statement handler lookup)
- Block comparison (keyword matching in tokenizer)

These are **language-neutral** primitives useful to any interpreter or
runtime, not BASIC-specific operations.

## Design Principle

From research.txt: "Add a VM feature only if it is useful to more than
one language, fundamental enough that it belongs in the abstract machine,
significantly simpler/faster/safer in the VM than in p-code library
code, and stable enough that you are unlikely to regret the abstraction."

Test: "Could Pascal runtime, BASIC, Lisp, and a future command shell
all plausibly use this?" — if yes, it belongs in the VM.

## Requested Primitives

### Tier 1 — Strongly Recommended

#### MEMCPY (src, dst, len)
- **Stack effect:** `( src dst len -- )`
- **Description:** Copy `len` bytes from `src` to `dst`. Must handle overlapping regions correctly (memmove semantics).
- **Justification:** Used heavily by interpreters for line insertion/deletion (shifting program buffers), string copy, data relocation. Pascal runtime needs it for array assignment and record copy. Every language benefits.
- **Current workaround:** Byte-by-byte loop using `loadb`/`storeb` — functional but O(n) VM dispatch overhead per byte.

#### MEMSET (dst, val, len)
- **Stack effect:** `( dst val len -- )`
- **Description:** Fill `len` bytes at `dst` with byte value `val`.
- **Justification:** Buffer clearing, program area initialization (NEW), zero-fill on allocation. Useful for any language runtime.
- **Current workaround:** Byte-by-byte loop — same overhead issue as MEMCPY.

### Tier 2 — Worth Considering

#### JMP_IND
- **Stack effect:** `( addr -- )`
- **Description:** Jump to address on top of eval stack (indirect/computed jump).
- **Justification:** Enables efficient dispatch tables for interpreters (statement dispatch, opcode dispatch). Also useful for virtual method dispatch, computed gotos, and any future bytecode-on-bytecode interpreter. Current workaround is a chain of comparisons and conditional jumps.

#### MEMCMP (a, b, len)
- **Stack effect:** `( a b len -- result )`
- **Description:** Compare `len` bytes at `a` and `b`. Push 0 if equal, negative if a<b, positive if a>b.
- **Justification:** Keyword matching in tokenizers, string comparison, sorted data structures. Used by every language with strings or symbols.
- **Current workaround:** Byte-by-byte comparison loop.

### Tier 3 — Maybe Later

#### FIND_BYTE (buf, val, len)
- **Stack effect:** `( buf val len -- index )`
- **Description:** Find first occurrence of byte `val` in buffer. Return index or -1 if not found.
- **Justification:** Token scanning, line-end detection, delimiter search. Common in parsers and text processing.

#### CALL_IND
- **Stack effect:** `( args... addr -- [retval] )`
- **Description:** Call procedure at address on eval stack (indirect call).
- **Justification:** Virtual dispatch, function pointer emulation, plugin/callback patterns.

## What Should NOT Be Added

These are interpreter/runtime policy, not abstract-machine primitives:

- BASIC_NEXT_LINE, BASIC_LOOKUP_LINE
- BASIC_PARSE_NUMBER, BASIC_PRINT
- Keyword tokenization opcodes
- Line-number search opcodes
- Any opcode that only one language would use

## Implementation Plan

1. **Now:** Implement MEMCPY, MEMSET, MEMCMP as p-code library routines
   (Pascal procedures or .spc subroutines) in the BASIC interpreter.
2. **After BASIC + one more interpreter exist:** Profile to identify
   actual hot paths. If MEMCPY/MEMSET are significant, promote to VM
   opcodes in the reserved 0x70-0xFF range.
3. **JMP_IND:** Evaluate after writing the BASIC statement dispatcher.
   If the comparison chain is a measurable bottleneck, add JMP_IND.

## Suggested Opcode Assignments (if promoted)

Reserved range 0x70-0xFF is available. Suggested:

| Byte | Mnemonic | Operand | Stack Effect |
|------|----------|---------|-------------|
| 0x70 | memcpy | — | ( src dst len -- ) |
| 0x71 | memset | — | ( dst val len -- ) |
| 0x72 | memcmp | — | ( a b len -- result ) |
| 0x73 | jmp_ind | — | ( addr -- ) |
| 0x74 | call_ind | — | ( args... addr -- [retval] ) |
| 0x75 | find_byte | — | ( buf val len -- index ) |

## Context

- **BASIC design docs:** sw-cor24-basic/docs/design.md (section 11)
- **BASIC architecture:** sw-cor24-basic/docs/architecture.md (section 6.4)
- **VM instruction set:** sw-cor24-pcode/vm/design.md (section 2)
- **Reserved opcode range:** 0x70-0xFF (vm/design.md section 6)
