# pv24a — Product Requirements Document

## Overview

**pv24a** is a p-code virtual machine (VM) targeting the COR24 24-bit RISC ISA.
It provides a language-neutral execution substrate for compilers (Pascal, Lisp,
BASIC, etc.) that emit stack-based p-code bytecode.

The VM runs on the COR24 emulator (`cor24-run`) and eventually on FPGA hardware.

## Goals

1. **Implement a p-code VM in COR24 assembly** — a stack-based interpreter that
   executes p-code bytecode loaded into memory.
2. **Implement a p-code assembler (pasm) in COR24 assembly** — a tool that
   translates human-readable `.pasm` source into p-code bytecode, used to create
   test programs and demos before any high-level compiler exists.
3. **Validate the VM with hand-written test programs** — Hello World, LED toggle,
   UART I/O, arithmetic, procedure calls, recursion, memory operations.
4. **Provide debugger-friendly metadata** — labels, procedure names, source line
   mappings, frame introspection support.

## Non-Goals (This Project)

- Pascal compiler (separate project, targets this VM)
- Other language front-ends (future projects)
- Native code generation / JIT
- Garbage collection
- Floating point
- Web UI debugger (separate project, like web-tf24a)
- Self-hosting

## Target Platform

- **ISA**: COR24 24-bit RISC (3 GPRs: r0/r1/r2, fp, sp, z, iv, ir)
- **Assembler/Emulator**: `cor24-run` from cor24-rs/cor24-bin
- **Memory**: 1 MB SRAM + 3 KB EBR stack + memory-mapped I/O
- **Cell size**: 3 bytes (24-bit words)
- **I/O**: UART at 0xFF0100/0xFF0101, LED/Switch at 0xFF0000

## Deliverables

### D1: P-Code VM (`pvm.s`)
- Stack-based p-code interpreter written in COR24 assembly
- Executes bytecode from a code segment in memory
- Supports: eval stack, call stack (separate), globals, heap (bump allocator)
- Traps for errors: stack overflow/underflow, invalid opcode, division by zero,
  nil pointer, bounds check failure
- ~30 core opcodes (see design.md)
- UART and LED I/O via sys instructions

### D2: P-Code Assembler (`pasm.s`)
- Two-pass assembler written in COR24 assembly
- Reads `.pasm` source text from UART input
- Emits p-code bytecode into a memory buffer
- Supports: labels, procedures, constants, string literals, integer literals,
  comments, directives (.const, .data, .proc/.end, .global)
- Machine-oriented syntax (not Pascal-like)
- Output: bytecode loadable by the VM

### D3: Test Suite (`tests/*.pasm`)
- Hand-written p-code assembly test programs
- Staged: arithmetic → control flow → procedures → memory → I/O → errors
- Run via `cor24-run` with UART input/output verification
- Regression testing pattern (golden output comparison)

### D4: Documentation
- `docs/architecture.md` — layered system architecture
- `docs/design.md` — VM state, opcodes, memory layout, calling convention, pasm syntax
- `docs/prd.md` — this document

## Success Criteria

1. VM executes Hello World (string output via UART)
2. VM executes recursive factorial
3. VM handles procedure calls with locals and arguments
4. VM traps on stack underflow and invalid opcode
5. pasm assembles all test programs correctly
6. LED blink demo runs on emulator
7. All tests pass via `cor24-run` with golden output comparison

## Dependencies

- `cor24-run` CLI (assembler + emulator) from ~/github/sw-embed/cor24-rs/cor24-bin
- COR24 ISA knowledge (instruction encoding, register constraints)
- Pattern reference: ~/github/sw-cli-tools/tf24a (DTC Forth in COR24 assembly)

## Constraints

- COR24 has only 3 GPRs (r0, r1, r2) — register pressure is high
- Branch offsets are ±127 bytes — far jumps need `la r0, label; jmp (r0)`
- No string literals in assembler — use `.byte` sequences
- Labels must be on their own line
- Comments use `;` only
- Integer-only (no floating point in ISA)
- Cell size is 3 bytes — all stack/frame offsets must be multiples of 3
