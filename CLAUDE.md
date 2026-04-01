# sw-cor24-pcode -- Claude Instructions

## Project Overview

P-code toolchain for the COR24 ISA. Rust workspace with two crates
plus the COR24-assembly VM.

## Layout

- `vm/` -- P-code VM (pvm.s), standalone assembler (pasm.s), integrated
  assembler+VM (pvmasm.s), test suite (demo.sh + tests/)
- `assembler/` -- Rust crate `pa24r`: two-pass .spc -> .p24 binary assembler
- `linker/` -- Rust crate `pl24r`: merges multiple .spc modules into one

## Build & Test

```bash
# Rust workspace
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings

# VM tests (requires cor24-run)
cd vm && ./demo.sh
```

## Pipeline

```
.spc sources -> pl24r (link) -> combined.spc -> pa24r (assemble) -> .p24 -> pvm.s (execute)
```

## CRITICAL: pasm.s and pa24r must stay in sync

`pasm.s` (COR24-native assembler) and `pa24r` (Rust assembler) are two
implementations of the same two-pass .spc assembler. They MUST remain
bug-compatible and at feature parity. Any opcode, directive, or encoding
change in one must be mirrored in the other. When modifying either:

1. Make the equivalent change in both implementations
2. Run both test suites to verify identical output
3. If adding a new opcode/directive, update the opcode table in both

The COR24-native pasm.s proves the toolchain is self-hosting on real
hardware; pa24r provides 4,447x faster assembly for development builds.
Neither is optional.

## Dependencies

- `cor24-run` from sw-cor24-emulator (for VM execution)
- No Rust crate dependencies (both pa24r and pl24r are pure Rust)

## CRITICAL: AgentRail Session Protocol (MUST follow exactly)

### 1. START (do this FIRST, before anything else)
```bash
agentrail next
```
Read the output carefully. It contains your current step, prompt,
plan context, and any relevant skills/trajectories.

### 2. BEGIN (immediately after reading the next output)
```bash
agentrail begin
```

### 3. WORK (do what the step prompt says)
Do NOT ask "want me to proceed?". The step prompt IS your instruction.
Execute it directly.

### 4. COMMIT (after the work is done)
Commit your code changes with git. Use `/mw-cp` for the checkpoint
process (pre-commit checks, docs, detailed commit, push).
**Run `/mw-cp` in every repo that was modified during the step.**

### 5. COMPLETE (LAST thing, after committing)
```bash
agentrail complete --summary "what you accomplished" \
  --reward 1 \
  --actions "tools and approach used"
```
- If the step failed: `--reward -1 --failure-mode "what went wrong"`
- If the saga is finished: add `--done`

### 6. STOP (after complete, DO NOT continue working)
Do NOT make further code changes after running `agentrail complete`.
Any changes after complete are untracked and invisible to the next
session. Future work belongs in the NEXT step, not this one.

## Key Rules

- **Do NOT skip steps** — the next session depends on accurate tracking
- **Do NOT ask for permission** — the step prompt is the instruction
- **Do NOT continue working** after `agentrail complete`
- **Commit before complete** — always commit first, then record completion

## Useful Commands

```bash
agentrail status          # Current saga state
agentrail history         # All completed steps
agentrail plan            # View the plan
agentrail next            # Current step + context
```
