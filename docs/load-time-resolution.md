# Load-Time Resolution: P-Code Unit Loading

Design document for loadable P-code units with load-time symbol
resolution.  Modeled after the COR24 monitor/sws/swye multi-binary
architecture, adapted for the P-code VM.

## Overview

Currently the P-code toolchain produces a single monolithic image: all
modules are merged by the linker, assembled into one `.p24`, and loaded
as one blob.  This design adds **units** -- independently assembled
`.p24` binaries that are combined by a loader and can call each other
at runtime through a VM-managed unit table.

```
 .spc sources            .spc sources
 (unit: app)             (unit: mathlib)
      |                       |
   pl24r --unit            pl24r --unit
      |                       |
   pa24r                   pa24r
      |                       |
  app.p24                mathlib.p24
      \                     /
       \                   /
        p24-load (loader)
              |
        memory image + unit table
              |
           pvm.s
```

Resolution is **load-time only**: every cross-unit reference is
resolved before the first instruction executes.  There is no lazy
binding or runtime symbol lookup.

---

## 1. Concepts

### Unit

A unit is an independently compiled and assembled `.p24` binary.  Each
unit has:

- A **code segment** (bytecode).
- A **data segment** (read-only byte data).
- An **export table** listing procedures visible to other units.
- An **import table** listing procedures this unit calls in other units.
- A **global reservation** (number of global words it needs).

At load time, each unit is assigned a **base address** in the VM's code
space and a **global partition** in the shared globals segment.

### Unit Table

A small array in VM memory, populated by the loader before execution
starts.  Each entry holds the base address and export-table pointer for
one unit.

```
unit_table[unit_id]:
    base_addr   (3 bytes)   code segment start in VM address space
    export_ptr  (3 bytes)   pointer to resolved export offsets
```

Maximum 256 units (unit_id is 1 byte).

### Import Resolution Table (IRT)

Each unit's imports are resolved into a per-unit array of absolute
target addresses.  The `XCALL` instruction indexes into the calling
unit's IRT to find the target PC.

---

## 2. .P24 Binary Format (v2)

### Current Format (v1, 18 bytes)

```
Offset  Size  Field
  0       4   Magic           "P24\0" (0x50 0x32 0x34 0x00)
  4       1   Version         0x01
  5       3   Entry point     LE24, byte offset in code segment
  8       3   Code size       LE24, bytes
 11       3   Data size       LE24, bytes
 14       3   Global count    LE24, words (each word = 3 bytes)
 17       1   Flags           0x00 (reserved)
 18+      N   Code segment
 18+N     M   Data segment
```

### Extended Format (v2)

Version byte becomes `0x02`.  New fields follow the v1 header:

```
Offset  Size  Field
  0       4   Magic           "P24\0"
  4       1   Version         0x02
  5       3   Entry point     LE24
  8       3   Code size       LE24
 11       3   Data size       LE24
 14       3   Global count    LE24
 17       1   Flags           bit 0: has_exports, bit 1: has_imports
 18       2   Export count    LE16 (max 65535; 0 if no exports)
 20       2   Import count    LE16 (max 65535; 0 if no imports)
 22       2   Name length     LE16 (unit name, for loader diagnostics)
 24       N   Unit name       UTF-8, not null-terminated

--- Export table (export_count entries, 5 bytes each) ---
 24+N     5*E  Exports        For each: name_hash(2) + offset(3)

--- Import table (import_count entries, 5 bytes each) ---
           5*I  Imports       For each: unit_hash(2) + name_hash(2) + slot(1)

--- String table (for diagnostics; optional, indicated by flags) ---
           var  Strings       Null-terminated export/import names

--- Code segment ---
           C    Code

--- Data segment ---
           D    Data
```

### Name Hashing

Export and import names are identified by a 16-bit FNV-1a hash.  The
full string names are stored in the optional string table for
diagnostics and collision detection but are not required at runtime.

The loader matches imports to exports by (unit_name_hash,
proc_name_hash) pairs.  On hash collision (detected via the string
table), the loader emits an error.

### Backward Compatibility

A v1 `.p24` is still valid.  Tools check the version byte:

- **v1**: No exports or imports; can only be loaded as a standalone unit
  or as the single unit in a monolithic image.
- **v2**: Full unit support.

---

## 3. Source Language (.spc) Changes

### New Directives

#### `.unit <name>`

Declares the source file as a named unit.  Replaces `.module` in unit
mode.  A file may contain either `.module` (for static linking) or
`.unit` (for unit compilation), not both.

```spc
.unit mathlib
```

#### `.import <unit_name>`

Declares a dependency on an external unit.  The assembler uses this to
validate `.extern` references and assign import slot indices.

```spc
.import mathlib
```

#### `.export <symbol> [<nargs>]`

Marks a procedure as visible to other units.  The optional `<nargs>`
parameter documents the argument count (for use by higher-level
compilers; the assembler stores it in the export table but does not
enforce it).

Already exists as a linker directive; now also recognized by the
assembler in unit mode.

```spc
.export gcd 2
.export factorial 1
```

#### `.extern <symbol> [<nargs>]`

Declares an imported procedure.  The assembler allocates an import
slot for it.  `xcall` references this symbol by slot index.

Already exists as a linker directive; now also recognized by the
assembler.

```spc
.extern gcd 2
```

### New Instruction: `xcall`

Cross-unit procedure call.  Operand is a symbol name that must have
been declared `.extern`.

```spc
.extern gcd 2

.proc main 0
    push 12
    push 8
    xcall gcd       ; cross-unit call
    ; result on eval stack
.end
```

**Encoding** (3 bytes):

```
  0x74   slot_lo   slot_hi
```

- Opcode `0x74` (next available after `JMP_IND` at `0x73`).
- `slot` is a 16-bit LE index into the unit's IRT.
- At load time, `IRT[slot]` contains the absolute target PC.

The assembler assigns slot indices sequentially in declaration order of
`.extern` directives (first extern = slot 0, second = slot 1, etc.).

### Example: Two-Unit Program

**mathlib.spc** (library unit):
```spc
.unit mathlib
.export gcd 2
.export factorial 1

.proc gcd 2
    loada 1             ; b
    jz gcd_base
    loada 0             ; a
    loada 1             ; b
    mod
    loada 1             ; b
    call gcd            ; intra-unit call (regular)
    ret 2
gcd_base:
    loada 0
    ret 2
.end

.proc factorial 1
    loada 0
    push 1
    le
    jz fact_recurse
    push 1
    ret 1
fact_recurse:
    loada 0
    loada 0
    push 1
    sub
    call factorial      ; intra-unit call
    mul
    ret 1
.end
```

**app.spc** (main unit):
```spc
.unit app
.import mathlib
.extern gcd 2
.extern factorial 1

.proc main 0
    push 48
    push 18
    xcall gcd           ; cross-unit call -> mathlib.gcd
    sys 1               ; putc result

    push 6
    xcall factorial     ; cross-unit call -> mathlib.factorial
    sys 1               ; putc result

    halt
.end
```

---

## 4. Assembler Changes (pa24r + pasm.s)

### pa24r (Rust Assembler)

#### Unit Mode

When the source contains `.unit`, the assembler operates in **unit
mode**:

1. **Pass 1** collects symbols as before, plus:
   - Export list from `.export` directives.
   - Import list from `.extern` directives (assigned slot indices).
   - Unit name from `.unit`.

2. **Pass 2** emits bytecode.  `xcall <symbol>` looks up the symbol in
   the import list and emits `0x74 <slot_lo> <slot_hi>`.

3. **Emit** writes a v2 `.p24` with export and import tables.

#### New Opcode Registration

Add `XCall = 0x74` to the opcode enum.  Encoding class: `IMM16`
(opcode + 2-byte LE operand).

Update the opcode bounds check comment: valid opcodes 0x00..0x74.

#### Error Conditions

| Error | When |
|-------|------|
| `xcall` on non-extern symbol | Symbol exists but was not declared `.extern` |
| `call` on `.extern` symbol | Must use `xcall` for external symbols |
| `.extern` without `.import` | No unit declared for the external symbol |
| Duplicate `.export` | Same symbol exported twice |

### pasm.s (COR24-Native Assembler)

Mirror the pa24r changes:

- Recognize `.unit`, `.import` directives.
- Assemble `xcall` as opcode `0x74` + 16-bit slot.
- Emit v2 `.p24` header with export/import tables.

This keeps the two assemblers at feature parity per the project rule.

---

## 5. Linker Changes (pl24r)

### Current Behavior

The linker merges all `.spc` modules into a single `.spc`:

1. Find the module containing `main`.
2. Reorder: main first, others follow.
3. Merge globals (de-duplicate by name), constants (error on conflict),
   procs (concatenate), data (concatenate).
4. Strip `.module`/`.export`/`.extern`/`.endmodule` directives.
5. Output a single `.spc` ready for the assembler.

This remains unchanged and is the **static link** path.

### New: Unit Link Mode (`--unit`)

```bash
pl24r --unit -o mathlib.spc  module_a.spc module_b.spc
```

In unit mode, the linker:

1. Merges input modules into one `.spc` as before.
2. **Preserves** `.unit`, `.export`, `.extern`, `.import` directives
   in the output (does not strip them).
3. Validates that every `.extern` symbol is either:
   - Defined in the merged modules (resolve internally, emit as regular
     `call`), or
   - Declared via `.import` (leave as `.extern` for the assembler).
4. Validates that every `.export` symbol is defined.

The output `.spc` is then assembled with pa24r, which produces a v2
`.p24` with the export/import tables.

### Mixed Mode

A unit can statically link several modules internally while still
exporting/importing across units:

```bash
# Link runtime modules into mathlib unit
pl24r --unit -o mathlib.spc  gcd.spc factorial.spc helpers.spc

# Assemble the unit
pa24r mathlib.spc -o mathlib.p24

# Link app modules
pl24r --unit -o app.spc  main.spc

# Assemble
pa24r app.spc -o app.p24

# Load both units
p24-load app.p24 mathlib.p24 -o image.p24m
```

---

## 6. Loader (p24-load) -- New Tool

A new Rust binary crate in the workspace: `loader/`.

### Input

One or more `.p24` files (v1 or v2).  The first file listed is the
**entry unit** (execution starts at its entry point).

```bash
p24-load app.p24 mathlib.p24 -o image.p24m
```

### Processing

1. **Parse headers** for all input `.p24` files.

2. **Assign unit IDs** (0..N-1) in command-line order.

3. **Layout code segments** sequentially:
   ```
   Unit 0 (app):      base = 0x000000, size = 0x001A00
   Unit 1 (mathlib):  base = 0x001A00, size = 0x000800
   ```

4. **Layout global segments**:
   ```
   Unit 0 globals:  offset 0, count 5    (words 0-4)
   Unit 1 globals:  offset 5, count 3    (words 5-7)
   Total globals:   8 words
   ```

5. **Resolve imports**: For each unit's import table, find the matching
   export in the target unit and compute the absolute address:
   ```
   app.import[0] "gcd" in "mathlib"
     -> mathlib.export["gcd"] = offset 0x0000
     -> absolute = mathlib.base + 0x0000 = 0x001A00
   ```

6. **Build the IRT** for each unit: an array of absolute 3-byte
   addresses, one per import slot.

7. **Build the unit table**: array of (base_addr, irt_ptr) entries.

8. **Patch global offsets**: Each unit's global references assume
   offset 0.  The loader adds the unit's global partition offset to
   every `LOADG`/`STOREG`/`ADDRG` operand in the unit's code.

   Alternatively, defer this to the VM by storing per-unit global
   base offsets in the unit table.  This avoids code patching at the
   cost of a VM lookup on every global access.  **Recommended:
   loader-side patching** (one-time cost, no runtime overhead).

9. **Emit the image** (`.p24m` = P24 multi-unit image).

### Output Format (.p24m)

```
Offset  Size    Field
  0       4     Magic           "P24M"
  4       1     Version         0x01
  5       3     Entry point     LE24, absolute address (unit 0's entry)
  8       1     Unit count
  9       3     Total code      LE24, bytes
 12       3     Total globals   LE24, words
 15       3     Unit table off  LE24, offset to unit table in image
 18       3     IRT offset      LE24, offset to import resolution tables

--- Unit Table (6 bytes per unit) ---
  base_addr(3) + global_base(3)

--- IRT (variable, 3 bytes per import slot per unit) ---
  Per unit: import_count(2) + [abs_addr(3)] * import_count

--- Code (concatenated, no gaps) ---
  Unit 0 code | Unit 1 code | ...

--- Data (concatenated) ---
  Unit 0 data | Unit 1 data | ...

--- Globals (zeroed, total_globals * 3 bytes) ---
```

### Error Conditions

| Error | When |
|-------|------|
| Unresolved import | Import references a unit or symbol not found |
| Hash collision | Two exports in the same unit have the same name hash |
| Duplicate unit name | Two input files declare the same `.unit` name |
| No entry unit | First input file has no entry point |
| Address overflow | Combined code exceeds 24-bit address space (16 MB) |

---

## 7. VM Changes (pvm.s + pvmasm.s)

### New VM State Fields

Extend `vm_state` from 27 bytes to 39 bytes:

| Offset | Size | Field | Purpose |
|--------|------|-------|---------|
| 0-26 | 27 | (existing) | pc, esp, csp, fp_vm, gp, hp, code, status, trap_code |
| 27 | 3 | unit_table | Pointer to unit table in memory |
| 30 | 3 | irt_base | Pointer to IRT base in memory |
| 33 | 1 | unit_count | Number of loaded units |
| 34 | 1 | current_unit | Unit ID of currently executing code |
| 35 | 3 | gp_base | Global base for current unit (cached) |
| 38 | 1 | (padding) | Alignment |

### XCALL Handler

New opcode `0x74`, encoding: `opcode(1) + slot(2)` = 3 bytes.

```
op_xcall:
    ; 1. Fetch slot index (16-bit LE) from code[pc]
    ;    slot = code[pc] | (code[pc+1] << 8)

    ; 2. Look up IRT for current unit
    ;    irt_entry = irt_base + unit_irt_offset[current_unit] + slot * 3

    ; 3. Read absolute target address from IRT
    ;    target_pc = IRT[slot]  (3 bytes, LE24)

    ; 4. Determine target unit from address
    ;    Scan unit_table to find which unit owns target_pc
    ;    (or store unit_id in IRT alongside the address)

    ; 5. Push extended call frame (15 bytes):
    ;    return_pc(3) + dynamic_link(3) + static_link(3) +
    ;    saved_esp(3) + saved_unit_id(1) + saved_gp_base(3)
    ;    Note: 2 extra bytes vs regular CALL frame

    ; 6. Update current_unit and gp_base for the target unit

    ; 7. Set pc = target_pc - target_unit.base_addr
    ;    (pc is relative to current unit's code base)

    ; 8. Set code = target_unit.base_addr
    ;    (update vm_state.code to point at new unit's code)

    ; 9. Jump to vm_loop
```

**Alternative (simpler): absolute PC model.**  If the loader
concatenates all code into one flat region, `pc` can be an absolute
offset into the combined code and `vm_state.code` stays constant.
This eliminates steps 7-8 and simplifies the handler significantly.
**Recommended.**

With the absolute-PC model, the handler reduces to:

```
op_xcall:
    ; 1. Fetch 16-bit slot from code[pc], advance pc by 2
    ; 2. target = IRT[current_unit][slot]   (absolute address)
    ; 3. Build call frame (same as op_call, plus saved_unit_id)
    ; 4. Update current_unit (for global access)
    ; 5. Set pc = target
    ; 6. Jump to vm_loop
```

### Extended Call Frame

Cross-unit calls need to save/restore the caller's unit context:

```
Extended frame (15 bytes):
  +0   return_pc       (3)   absolute PC to return to
  +3   dynamic_link    (3)   caller's fp_vm
  +6   static_link     (3)   lexical parent (0 for xcall)
  +9   saved_esp       (3)   caller's eval stack pointer
  +12  saved_unit_id   (1)   caller's current_unit
  +13  saved_gp_base   (2)   (unused padding, gp_base derivable from unit_id)
```

Or, to avoid a different frame size: store `current_unit` in the
existing frame's static_link field (which is always 0 for cross-unit
calls, since lexical scoping doesn't cross unit boundaries).  This
keeps the frame at 12 bytes.

**Recommended: reuse static_link field.**  The high byte of the 3-byte
static_link can store the caller's unit_id (static_link is 0 for
xcall, so the high byte is free):

```
Frame (12 bytes, same as CALL):
  +0   return_pc       (3)
  +3   dynamic_link    (3)
  +6   unit_id(1) + 0x0000(2)   ; static_link repurposed for xcall
  +9   saved_esp       (3)
```

### XRET Behavior

No new `XRET` instruction is needed.  The existing `RET` instruction
works because:

1. `RET` restores `pc` from the frame's `return_pc` (absolute).
2. `RET` restores `fp_vm` from the frame's dynamic_link.
3. The VM checks: if the frame's static_link high byte is nonzero,
   it's a cross-unit return -- restore `current_unit` and `gp_base`
   from the saved values.

This adds a single byte-check to `RET` (branch on the high byte of
the static_link field).

### Global Access Across Units

Each unit's globals occupy a partition of the shared globals segment.
The VM maintains `gp_base` = globals_seg + (current_unit's
global_offset * 3).

- `LOADG offset` reads from `gp_base + offset * 3` (unchanged logic,
  just different base).
- On unit switch (xcall/ret), `gp_base` is updated from the unit
  table.

No new instructions needed for per-unit globals.

For **cross-unit global access** (reading another unit's globals),
add a new instruction:

```
XLOADG unit_id(1) offset(1)    ; opcode 0x75, 3 bytes
XSTOREG unit_id(1) offset(1)   ; opcode 0x76, 3 bytes
```

These are optional -- most inter-unit communication should go through
procedure calls.  Cross-unit globals are useful for shared state like
configuration or I/O buffers (analogous to the monitor's shared
buffers at 0x0F0000).

### Dispatch Table Update

Extend the opcode dispatch table from 116 entries (0x00-0x73) to
119 entries (0x00-0x76):

| Opcode | Mnemonic | Encoding | Size |
|--------|----------|----------|------|
| 0x74 | XCALL | IMM16 (slot) | 3 |
| 0x75 | XLOADG | IMM8+IMM8 (unit, offset) | 3 |
| 0x76 | XSTOREG | IMM8+IMM8 (unit, offset) | 3 |

### Initialization

When loading a `.p24m` image, the VM initialization code at `_start`
must:

1. Read the unit table from the image and store its address in
   `vm_state.unit_table`.
2. Read the IRT base and store in `vm_state.irt_base`.
3. Set `vm_state.unit_count` from the image header.
4. Set `current_unit = 0` (entry unit).
5. Set `code` to point at the combined code region.
6. Set `gp_base` from unit_table[0].global_base.

For backward compatibility with v1 `.p24` files (no units), the VM
defaults to `unit_count = 1`, `current_unit = 0`, and the existing
single-unit behavior is unchanged.

---

## 8. Compiler Considerations

For higher-level compilers that generate `.spc` (e.g., a future Pascal
or C subset compiler targeting P-code):

### Interface Files (.spi)

A unit's public interface can be described in a `.spi` (SPC interface)
file:

```spc
; mathlib.spi -- auto-generated or hand-written
.unit mathlib
.export gcd 2           ; proc name, arg count
.export factorial 1
```

The compiler reads `.spi` files to:

1. Know which procedures exist in external units.
2. Validate argument counts at compile time.
3. Generate `.import` and `.extern` directives in the output `.spc`.
4. Emit `xcall` instead of `call` for external symbols.

### Forward Declarations

Within a single unit, the two-pass assembler already handles forward
references (a `call` to a proc defined later in the file).  No change
needed.

Across units, the `.extern` directive serves as the forward
declaration.  The compiler must see either the `.spi` file or explicit
`extern` declarations before generating a call.

### Type Checking (Optional)

The `.spi` file could be extended with argument/return types:

```spc
.export gcd 2 int int -> int
```

This is not required for the assembler or VM but would enable a
type-checking compiler to validate cross-unit calls at compile time.

---

## 9. Implementation Phases

### Phase 1: Extended .p24 Format

- Add v2 header support to pa24r (read and write).
- Add export table emission: when `.unit` + `.export` are present,
  write the export table to the `.p24`.
- Mirror in pasm.s.
- No behavioral change for existing v1 files.

**Test**: assemble a unit with exports, verify the .p24 contains the
export table.

### Phase 2: XCALL Opcode

- Add `XCALL = 0x74` to both assemblers.
- Add `.extern` slot allocation in unit mode.
- Emit `0x74 <slot_lo> <slot_hi>` for `xcall` instructions.
- Add `op_xcall` handler to pvm.s and pvmasm.s.
- Extend dispatch table to 119 entries.

**Test**: single-unit program with a manually-constructed IRT, verify
xcall dispatches correctly.

### Phase 3: Loader (p24-load)

- New Rust crate `loader/` in the workspace.
- Parse v2 `.p24` files.
- Assign base addresses and global partitions.
- Resolve imports against exports (hash match + string verify).
- Build IRT.
- Emit `.p24m` image.

**Test**: two-unit program (app + library), load and verify the image
layout.

### Phase 4: VM Multi-Unit Support

- Extend vm_state with unit_table, irt_base, unit_count, current_unit,
  gp_base.
- Update `_start` to parse `.p24m` header and initialize unit state.
- Update `op_xcall` to use real IRT.
- Update `op_ret` to detect and handle cross-unit returns.
- Update `LOADG`/`STOREG` to use `gp_base`.

**Test**: end-to-end two-unit program runs correctly.

### Phase 5: Linker Unit Mode

- Add `--unit` flag to pl24r.
- Preserve `.unit`/`.import`/`.export`/`.extern` in output.
- Validate export/import consistency.

**Test**: multi-module unit linked and assembled into a v2 `.p24`.

### Phase 6: Cross-Unit Globals (Optional)

- Add `XLOADG` (0x75) and `XSTOREG` (0x76).
- Update assemblers and VM.

### Phase 7: Integration and demo.sh Tests

- Add multi-unit test cases to `vm/tests/` and `vm/demo.sh`.
- Add loader integration tests.
- End-to-end demo: app unit calls library unit, prints results.

---

## 10. Comparison with COR24 Native Pattern

| Aspect | COR24 Native | P-Code Units |
|--------|-------------|--------------|
| Binary format | Raw `.bin` (no header) | `.p24` v2 (structured header) |
| Loading | `--load-binary X@ADDR` | `p24-load` assigns addresses |
| Entry discovery | Memory patch (`--patch`) | Export table in `.p24` header |
| Cross-call | Function pointer via `jal` | `XCALL` opcode + IRT |
| Shared state | Fixed memory addresses | Shared globals segment |
| Service vector | Function pointer table | System calls (unchanged) |
| Return path | Trampoline + saved SP | `RET` with unit-id restore |
| Address model | Absolute (physical) | Absolute (virtual, within VM) |

The key difference: COR24 native programs share a physical address
space and call each other with hardware `jal`/`jmp` instructions.
P-code units share a virtual address space managed by the VM, and
cross-unit calls are mediated by the `XCALL` instruction through the
IRT.  The loader plays the role that `cor24-run --load-binary` and
`--patch` play in the native system.
