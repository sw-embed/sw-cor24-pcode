# pv24a — Architecture

## System Layers

```
┌─────────────────────────────────────┐
│  Layer 4: Language Front-Ends       │  (future, separate projects)
│  Pascal compiler → .pasm / bytecode │
│  Lisp, BASIC, etc.                  │
├─────────────────────────────────────┤
│  Layer 3: Language-Specific Runtime  │  (future, separate projects)
│  Pascal: new/dispose, writeln, sets │
├─────────────────────────────────────┤
│  Layer 2: Base Runtime Services     │  ← this project (runtime portion)
│  Heap allocator, bounds checking,   │
│  nil/div-zero traps, I/O stubs      │
├─────────────────────────────────────┤
│  Layer 1: P-Code VM                 │  ← this project (core)
│  Stack machine interpreter          │
│  Eval stack + call stack + globals  │
│  ~30 opcodes, frame model, traps    │
├─────────────────────────────────────┤
│  Layer 0: Target Substrate          │  (existing, external)
│  COR24 ISA, cor24-run assembler,    │
│  emulator, FPGA hardware            │
└─────────────────────────────────────┘
```

**This project (pv24a)** implements Layers 1 and 2, plus a p-code assembler
tool for testing.

## Component Map

```
pv24a/
├── pvm.s              P-Code VM interpreter (Layer 1)
├── runtime.s          Base runtime services (Layer 2)
├── pasm.s             P-Code assembler tool
├── tests/
│   ├── t01-arith.pasm     Arithmetic tests
│   ├── t02-stack.pasm     Stack operation tests
│   ├── t03-branch.pasm    Control flow tests
│   ├── t04-proc.pasm      Procedure call tests
│   ├── t05-memory.pasm    Memory access tests
│   ├── t06-uart.pasm      UART I/O tests
│   ├── t07-led.pasm       LED tests
│   ├── t08-recurse.pasm   Recursion tests
│   ├── t09-heap.pasm      Heap allocation tests
│   └── t10-traps.pasm     Error/trap tests
├── examples/
│   ├── hello.pasm         Hello World
│   ├── blink.pasm         LED blink
│   └── echo.pasm          UART echo
├── docs/
│   ├── prd.md
│   ├── architecture.md
│   ├── design.md
│   └── research.txt
└── demo.sh                Test harness script
```

## Memory Map (COR24 address space)

```
0x000000 ┌──────────────────────┐
         │ COR24 boot / entry   │  cor24-run entry point
         ├──────────────────────┤
         │ P-Code VM code       │  pvm.s assembled COR24 instructions
         │ (interpreter loop,   │
         │  opcode dispatch,    │
         │  runtime services)   │
         ├──────────────────────┤
         │ P-Code bytecode      │  loaded/assembled p-code program
         │ (code segment)       │
         ├──────────────────────┤
         │ Constants / strings  │  literal pool
         ├──────────────────────┤
         │ Globals segment      │  p-code global variables
         ├──────────────────────┤
         │ Call stack            │  frames: ret-pc, dyn-link, static-link,
         │ (grows upward)       │  proc-id, locals
         ├──────────────────────┤
         │ Eval stack            │  expression temporaries
         │ (grows upward)       │  (separate from call stack)
         ├──────────────────────┤
         │ Free space            │
         ├──────────────────────┤
         │ Heap                  │  dynamic allocation (grows upward)
         │ (bump allocator v1)  │
         ├──────────────────────┤
0x0FFFFF │ End of SRAM           │
         ├──────────────────────┤
0xFEEC00 │ EBR (3KB)             │  COR24 hardware stack (sp)
         │                      │  Used as scratch / COR24-level stack
0xFEF7FF └──────────────────────┘
0xFF0100   UART data register
0xFF0101   UART status register
0xFF0000   LED/Switch port (write=LED D2, read=button S2)
```

## Register Allocation (COR24 → VM)

The COR24 has very few registers. For the VM interpreter:

| COR24 Reg | VM Role         | Notes |
|-----------|-----------------|-------|
| r0        | W (work/scratch)| General scratch, opcode dispatch |
| r1        | VM state ptr    | Points to VM state struct in memory |
| r2        | Scratch #2      | Secondary work register |
| sp        | COR24 stack     | Hardware push/pop (EBR), used sparingly |
| fp        | Memory base     | Used for indexed loads/stores |

VM state (pc, esp, csp, gp, hp, etc.) lives in a memory struct because
COR24 doesn't have enough registers to hold it all.

## Execution Model

### Interpreter Loop (fetch-decode-execute)

```
vm_loop:
    ; fetch opcode byte from p-code[pc]
    ; pc += 1
    ; decode: index into dispatch table
    ; jump to handler
    ; handler executes, modifies VM state
    ; jump back to vm_loop
```

### Dual Stack Architecture

- **Eval stack**: expression temporaries, pushed/popped by arithmetic and
  load/store instructions. Grows upward in SRAM.
- **Call stack**: procedure activation records (frames). Grows upward in
  a separate SRAM region. Each frame contains:
  - Return PC (p-code address)
  - Dynamic link (previous frame pointer)
  - Static link (lexically enclosing frame, for nested scopes)
  - Procedure ID (for debugger)
  - Local variable slots

Separate stacks make debugging much easier and prevent eval stack corruption
from damaging frame data.

### Opcode Encoding

P-code instructions are variable-length byte sequences:

| Format | Bytes | Example |
|--------|-------|---------|
| op     | 1     | `add`, `dup`, `ret` |
| op imm8 | 2   | `push_small 42` |
| op imm24 | 4  | `push 100000`, `jmp label` |
| op u8 u8 | 3  | `loadl 2` (local at offset 2) |

## I/O Model

The VM provides two I/O paths:

1. **sys instructions** — abstract device calls (portable)
   - `sys PUTC` — write byte to output
   - `sys GETC` — read byte from input
   - `sys LED` — write LED state
   - `sys HALT` — stop execution

2. **Memory-mapped I/O** — direct UART/LED access via load/store
   - Available but discouraged for portable p-code programs
   - Runtime library wraps MMIO behind sys calls

## Build & Test

```bash
# Assemble and run VM with a test program
cor24-run --run pvm.s -u "$(cat tests/t01-arith.pasm)" --speed 0 -n 5000000

# Or: assemble pasm source, then run VM with bytecode
# (exact mechanism TBD — may use UART pipe or preloaded memory)

# Interactive mode
cor24-run --run pvm.s --terminal --echo

# Regression test
./demo.sh test
```

## Relationship to Other Projects

| Project | Role | Location |
|---------|------|----------|
| cor24-rs/cor24-bin | COR24 assembler + emulator | ~/github/sw-embed/cor24-rs |
| tf24a | Pattern reference (Forth in COR24 asm) | ~/github/sw-cli-tools/tf24a |
| pv24a | **This project** — P-Code VM + pasm | ~/github/sw-vibe-coding/pv24a |
| (future) | Pascal compiler → p-code | TBD separate project |
| (future) | Web debugger UI (Yew/WASM) | TBD separate project |
