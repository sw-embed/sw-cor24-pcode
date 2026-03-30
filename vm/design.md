# pv24a — Design Document

## 1. VM State

The p-code VM maintains the following state in a memory struct:

```
vm_state:
    .word 0     ; pc   — p-code instruction pointer
    .word 0     ; esp  — eval stack pointer (top of eval stack)
    .word 0     ; csp  — call stack pointer (top of call stack)
    .word 0     ; fp   — current frame pointer (call stack)
    .word 0     ; gp   — globals base address
    .word 0     ; hp   — heap pointer (next free)
    .word 0     ; code — base address of p-code segment
    .word 0     ; status — 0=running, 1=halted, 2=trapped
    .word 0     ; trap_code — last trap reason
```

## 2. P-Code Instruction Set

### 2.1 Stack / Constants

| Opcode | Encoding | Stack Effect | Description |
|--------|----------|-------------|-------------|
| `push` imm24 | 4 bytes | ( -- n ) | Push 24-bit immediate |
| `push_s` imm8 | 2 bytes | ( -- n ) | Push sign-extended 8-bit immediate |
| `dup` | 1 byte | ( a -- a a ) | Duplicate top |
| `drop` | 1 byte | ( a -- ) | Discard top |
| `swap` | 1 byte | ( a b -- b a ) | Swap top two |
| `over` | 1 byte | ( a b -- a b a ) | Copy second to top |

### 2.2 Arithmetic / Logic

| Opcode | Encoding | Stack Effect | Description |
|--------|----------|-------------|-------------|
| `add` | 1 byte | ( a b -- a+b ) | Add |
| `sub` | 1 byte | ( a b -- a-b ) | Subtract |
| `mul` | 1 byte | ( a b -- a*b ) | Multiply |
| `div` | 1 byte | ( a b -- a/b ) | Divide (trap on b=0) |
| `mod` | 1 byte | ( a b -- a%b ) | Modulo (trap on b=0) |
| `neg` | 1 byte | ( a -- -a ) | Negate |
| `and` | 1 byte | ( a b -- a&b ) | Bitwise AND |
| `or` | 1 byte | ( a b -- a\|b ) | Bitwise OR |
| `xor` | 1 byte | ( a b -- a^b ) | Bitwise XOR |
| `not` | 1 byte | ( a -- ~a ) | Bitwise NOT |
| `shl` | 1 byte | ( a n -- a<<n ) | Shift left |
| `shr` | 1 byte | ( a n -- a>>n ) | Arithmetic shift right |

### 2.3 Comparison

All comparisons push 1 (true) or 0 (false):

| Opcode | Encoding | Stack Effect | Description |
|--------|----------|-------------|-------------|
| `eq` | 1 byte | ( a b -- flag ) | Equal |
| `ne` | 1 byte | ( a b -- flag ) | Not equal |
| `lt` | 1 byte | ( a b -- flag ) | Less than (signed) |
| `le` | 1 byte | ( a b -- flag ) | Less or equal (signed) |
| `gt` | 1 byte | ( a b -- flag ) | Greater than (signed) |
| `ge` | 1 byte | ( a b -- flag ) | Greater or equal (signed) |

### 2.4 Control Flow

| Opcode | Encoding | Stack Effect | Description |
|--------|----------|-------------|-------------|
| `jmp` addr24 | 4 bytes | ( -- ) | Unconditional jump |
| `jz` addr24 | 4 bytes | ( flag -- ) | Jump if zero |
| `jnz` addr24 | 4 bytes | ( flag -- ) | Jump if nonzero |
| `call` addr24 | 4 bytes | ( args... -- ) | Call procedure (static link = 0) |
| `calln` depth8 addr24 | 5 bytes | ( args... -- ) | Call with static link (depth=0: nested, depth=N: N levels up) |
| `ret` nargs8 | 2 bytes | ( [retval] -- [retval] ) | Return, clean nargs |
| `halt` | 1 byte | ( -- ) | Stop VM |
| `trap` code8 | 2 bytes | ( -- ) | Trigger trap |

### 2.5 Local / Global / Nonlocal Access

| Opcode | Encoding | Stack Effect | Description |
|--------|----------|-------------|-------------|
| `enter` nlocals8 | 2 bytes | ( -- ) | Set up frame, reserve N local slots |
| `leave` | 1 byte | ( -- ) | Tear down frame |
| `loadl` off8 | 2 bytes | ( -- val ) | Load local at offset |
| `storel` off8 | 2 bytes | ( val -- ) | Store local at offset |
| `loadg` off24 | 4 bytes | ( -- val ) | Load global at offset |
| `storeg` off24 | 4 bytes | ( val -- ) | Store global at offset |
| `addrl` off8 | 2 bytes | ( -- addr ) | Push address of local |
| `addrg` off24 | 4 bytes | ( -- addr ) | Push address of global |
| `loadn` depth8 off8 | 3 bytes | ( -- val ) | Load nonlocal via static link chain |
| `storen` depth8 off8 | 3 bytes | ( val -- ) | Store nonlocal via static link chain |

### 2.6 Indirect Memory Access

| Opcode | Encoding | Stack Effect | Description |
|--------|----------|-------------|-------------|
| `load` | 1 byte | ( addr -- val ) | Load word from address |
| `store` | 1 byte | ( val addr -- ) | Store word to address |
| `loadb` | 1 byte | ( addr -- byte ) | Load byte (zero-extended) |
| `storeb` | 1 byte | ( byte addr -- ) | Store byte |

### 2.7 System / I/O

| Opcode | Encoding | Stack Effect | Description |
|--------|----------|-------------|-------------|
| `sys` id8 | 2 bytes | (varies) | System call |

System call IDs:

| ID | Name | Stack Effect | Description |
|----|------|-------------|-------------|
| 0 | HALT | ( -- ) | Stop execution |
| 1 | PUTC | ( char -- ) | Write byte to UART |
| 2 | GETC | ( -- char ) | Read byte from UART (blocking) |
| 3 | LED | ( state -- ) | Write LED D2 state (bit 0) |
| 4 | ALLOC | ( size -- ptr ) | Bump-allocate heap block |
| 5 | FREE | ( ptr -- ) | Free heap block (no-op in bump mode) |
| 6 | READ_SWITCH | ( -- state ) | Read button S2 state (bit 0: 1=released, 0=pressed) |

### 2.8 Argument Access

| Opcode | Encoding | Stack Effect | Description |
|--------|----------|-------------|-------------|
| `loada` idx8 | 2 bytes | ( -- val ) | Load argument by index |
| `storea` idx8 | 2 bytes | ( val -- ) | Store argument by index |

## 3. Call Frame Layout

When `call` executes, a new frame is pushed onto the call stack:

```
         ┌──────────────────┐  ← new csp (after enter)
         │ local[N-1]       │  offset = +(N-1)*3 from fp
         │ ...              │
         │ local[0]         │  offset = 0 from fp
fp ───→  ├──────────────────┤
         │ procedure ID     │  offset = -3
         │ static link      │  offset = -6  (lexically enclosing frame)
         │ dynamic link     │  offset = -9  (caller's fp)
         │ return PC        │  offset = -12
         ├──────────────────┤
         │ arg[N-1]         │  offset = -15  (from fp)
         │ ...              │
         │ arg[0]           │  offset = -15 - (N-1)*3
         └──────────────────┘
```

- **Caller** pushes arguments onto eval stack, then `call addr`
- **call** instruction: pops args from eval stack into call stack frame header,
  pushes return PC, dynamic link, static link, proc ID
- **enter N**: advances csp by N*3 to reserve local slots, sets fp
- **leave**: restores fp to dynamic link, restores csp
- **ret N**: pushes return value (if any) to eval stack, pops frame, cleans N args

## 4. Calling Convention

Simple and explicit:

1. Caller pushes arguments left-to-right onto eval stack
2. Caller executes `call <proc_addr>`
3. Callee executes `enter <nlocals>` (reserves local slots)
4. Callee accesses args via `loada 0`, `loada 1`, etc.
5. Callee accesses locals via `loadl 0`, `loadl 1`, etc.
6. For functions: callee pushes return value onto eval stack
7. Callee executes `ret <nargs>` (cleans args from frame)
8. Caller finds return value (if any) on eval stack

## 5. P-Code Assembler (pasm) Syntax

### 5.0 File Extensions

- `.spc` — p-code assembler source files (input to pasm)
- `.p24` — assembled p-code bytecode files (output of pasm, input to VM)

### 5.1 Source Format

```
; Comment (semicolon to end of line)

.const UART_DATA  -65280      ; named constant
.const UART_STATUS -65279
.const LED_PORT   -65536

.global counter 1             ; 1 word of global storage
.global buffer 32             ; 32 words

.data msg 72, 101, 108, 108, 111, 0   ; "Hello\0" as bytes

.proc main 0                 ; procedure, 0 locals
    push msg                  ; push address of msg
    call puts
    halt
.end

.proc puts 1                 ; 1 local (loop pointer)
    loada 0                   ; arg0 = string address
    storel 0                  ; local0 = current ptr
loop:
    loadl 0
    loadb                     ; load byte at ptr
    dup
    jz done                   ; if zero, done
    sys 1                     ; PUTC
    loadl 0
    push 1
    add
    storel 0                  ; ptr++
    jmp loop
done:
    drop                      ; drop the zero
    ret 1                     ; return, clean 1 arg
.end
```

### 5.2 Directives

| Directive | Syntax | Description |
|-----------|--------|-------------|
| `.const` | `.const NAME value` | Define named constant |
| `.global` | `.global NAME nwords` | Reserve global storage |
| `.data` | `.data NAME byte, byte, ...` | Define byte data (string/array) |
| `.proc` | `.proc NAME nlocals` | Begin procedure (nlocals = local slot count) |
| `.end` | `.end` | End procedure |

### 5.2.1 Module Metadata Directives

These directives are emitted by the linker (pl24r) for cross-module symbol resolution. pasm silently skips them — they do not affect assembly.

| Directive | Syntax | Description |
|-----------|--------|-------------|
| `.module` | `.module NAME` | Declare compilation unit name |
| `.export` | `.export SYMBOL` | Mark symbol as visible to other modules |
| `.extern` | `.extern SYMBOL` | Declare external symbol dependency |
| `.endmodule` | `.endmodule` | End module declaration |

These are part of the .spc format contract for the toolchain pipeline: `p24p → pl24r → pasm/pa24r → pv24a`. See [[P24Toolchain]] on the coordination wiki for details.

### 5.3 Labels

Labels appear on their own line, ending with `:`:

```
loop:
    ; instructions...
    jmp loop
```

Labels are local to the enclosing `.proc` / `.end` block.

### 5.4 Integer Literals

- Decimal: `42`, `-1`, `0`
- Hex: `0xFF` (if supported) or decimal equivalents
- Named constants: `UART_DATA` (substituted by assembler)

### 5.5 Instructions

One instruction per line. Operand follows mnemonic separated by whitespace:

```
push 42
loadl 0
jz done
call puts
sys 1
ret 0
```

## 6. Opcode Encoding Table

Opcodes are assigned sequential byte values:

| Byte | Mnemonic | Operand |
|------|----------|---------|
| 0x00 | halt | — |
| 0x01 | push | imm24 |
| 0x02 | push_s | imm8 |
| 0x03 | dup | — |
| 0x04 | drop | — |
| 0x05 | swap | — |
| 0x06 | over | — |
| 0x10 | add | — |
| 0x11 | sub | — |
| 0x12 | mul | — |
| 0x13 | div | — |
| 0x14 | mod | — |
| 0x15 | neg | — |
| 0x16 | and | — |
| 0x17 | or | — |
| 0x18 | xor | — |
| 0x19 | not | — |
| 0x1A | shl | — |
| 0x1B | shr | — |
| 0x20 | eq | — |
| 0x21 | ne | — |
| 0x22 | lt | — |
| 0x23 | le | — |
| 0x24 | gt | — |
| 0x25 | ge | — |
| 0x30 | jmp | addr24 |
| 0x31 | jz | addr24 |
| 0x32 | jnz | addr24 |
| 0x33 | call | addr24 |
| 0x34 | ret | nargs8 |
| 0x35 | calln | depth8 addr24 |
| 0x36 | trap | code8 |
| 0x40 | enter | nlocals8 |
| 0x41 | leave | — |
| 0x42 | loadl | off8 |
| 0x43 | storel | off8 |
| 0x44 | loadg | off24 |
| 0x45 | storeg | off24 |
| 0x46 | addrl | off8 |
| 0x47 | addrg | off24 |
| 0x48 | loada | idx8 |
| 0x49 | storea | idx8 |
| 0x4A | loadn | depth8 off8 |
| 0x4B | storen | depth8 off8 |
| 0x50 | load | — |
| 0x51 | store | — |
| 0x52 | loadb | — |
| 0x53 | storeb | — |
| 0x60 | sys | id8 |

Reserved ranges 0x70–0xFF for future opcodes (heap, debug, extended ops).

## 7. Trap Codes

| Code | Name | Trigger |
|------|------|---------|
| 0 | USER_TRAP | `trap 0` instruction |
| 1 | DIV_ZERO | Division/modulo by zero |
| 2 | STACK_OVERFLOW | Eval or call stack exceeds limit |
| 3 | STACK_UNDERFLOW | Eval stack pop when empty |
| 4 | INVALID_OPCODE | Unknown opcode byte |
| 5 | INVALID_ADDRESS | Memory access out of bounds |
| 6 | NIL_POINTER | Dereference of address 0 |
| 7 | BOUNDS_CHECK | Array index out of bounds |

On trap: VM sets status=2, trap_code=N, halts. Diagnostic message to UART.

## 8. Memory Segment Layout

At VM initialization:

1. **Code segment** starts at a fixed base address (after VM code)
2. **Constants/data** follows code (emitted by pasm)
3. **Globals** follow data (allocated per `.global` directives)
4. **Call stack** starts after globals, grows upward
5. **Eval stack** starts after call stack region, grows upward
6. **Heap** starts after eval stack region, grows upward
7. Segment base addresses stored in VM state struct

Sizes are configurable at VM init time. Default allocations:
- Call stack: 4 KB
- Eval stack: 2 KB
- Heap: remaining SRAM up to ~900 KB

## 9. Implementation Strategy

### Phase 1: Minimal VM
- Opcode dispatch loop
- push, dup, drop, add, sub, halt
- UART output via sys PUTC
- Hardcoded test bytecode (no pasm yet)

### Phase 2: Control Flow + Procedures
- jmp, jz, jnz
- call, ret, enter, leave
- loadl, storel, loada, storea
- Frame management

### Phase 3: Full Instruction Set
- All arithmetic/logic/comparison opcodes
- Globals (loadg, storeg)
- Indirect memory (load, store, loadb, storeb)
- Nonlocal access (loadn, storen)
- All sys calls

### Phase 4: P-Code Assembler
- Lexer for pasm syntax (UART input)
- Two-pass assembly (collect labels, emit bytes)
- Directive handling (.const, .global, .data, .proc/.end)
- Output bytecode to memory buffer

### Phase 5: Integration & Test Suite
- pasm assembles test programs
- VM executes assembled bytecode
- Regression test harness (demo.sh)
- Full test coverage per staged plan

## 10. COR24 Assembly Patterns

Reference patterns from tf24a for common idioms:

### Dispatch Table
```asm
; Index into jump table by opcode byte
; r0 = opcode
la r2, dispatch_table
; multiply r0 by 3 (entry size = .word = 3 bytes)
; ... load target address, jmp
```

### UART Output
```asm
; r0 = character to send
la r2, -65280       ; UART base
tx_wait:
    lb r0, 1(r2)   ; status (sign-extended)
    cls r0, z       ; C = (status < 0) = TX busy
    brt tx_wait
    sb r0, 0(r2)   ; write byte
```

### Memory Struct Access
```asm
; Load vm_state.pc into r0
la fp, vm_state
lw r0, 0(fp)       ; pc at offset 0
```
