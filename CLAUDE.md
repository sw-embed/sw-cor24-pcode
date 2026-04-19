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

## CRITICAL: Git Branching Workflow (devgroup policy)

This clone is downstream of a coordinator-gated integration model:

- `main` and `dev` are coordinator-only. **Never commit to them
  directly, and never `git push`.** The coordinator (mike) relays
  ready branches into `dev` and pushes.
- Do all work on `feat/<slug>` or `fix/<slug>` branches, based on
  local `dev` (which tracks the integration branch).
- When work is ready for integration, rename the branch to
  `pr/<slug>` so the coordinator's scan picks it up. **The rename
  IS the handoff.** "PR" here means a `pr/<slug>` branch awaiting
  the coordinator, NOT a GitHub pull request opened with `gh pr
  create`.
- The ref name is the contract -- no PR API, no JSON, no tickets,
  no `gh pr create`.

Dev agents (you) have NO remote write access. Do not invoke `git
push`, `gh pr create`, or any other GitHub-side command. The `push`
phase of `/mw-cp` does not apply on `feat/*`, `fix/*`, or `pr/*`
branches -- stop at the commit step.

### Helpers (on `$PATH` via `$SCRIPTROOT`)

```bash
onboarding               # session briefing: paths, policy, repo state
dg-env                   # environment dump
dg-policy                # reprint the branch policy
dg-new-feature <slug>    # switch dev, fetch, create feat/<slug>
dg-new-fix <slug>        # same flavor, fix/<slug>
dg-mark-pr               # rename current feat/*|fix/* -> pr/*
dg-list-pr               # list local pr/* branches (ready signals)
dg-reap                  # fetch; FF dev; delete pr/* merged into origin/dev
```

### Rules

- **Never `git push`** -- the coordinator handles all pushes.
- **Never commit to `main` or `dev`** -- always work on `feat/*`
  or `fix/*`.
- Base new branches on `origin/dev`; fall back to `origin/main`
  only when `origin/dev` does not exist yet.
- No history rewrites on `dev` or `main`. Rebase is fine on your
  own `feat/*` / `fix/*` before marking `pr/*`.
- After the coordinator merges `pr/<slug>` into `origin/dev`, run
  `dg-reap` to fast-forward local `dev` and delete the merged
  branch. ("Reap" here means `branch -D` for `pr/*` already in
  `origin/dev` -- not a GitHub-API cleanup.)

Full policy:
`/disk1/github/softwarewrighter/devgroup/docs/branching-pr-strategy.md`

## CRITICAL: AgentRail Session Protocol (MUST follow exactly)

Each AgentRail step maps to one `feat/<slug>` (or `fix/<slug>`)
branch. Create the branch BEFORE doing the work, and rename it to
`pr/<slug>` AFTER `agentrail complete`.

### 1. START (do this FIRST, before anything else)
```bash
onboarding     # paths, branch policy, helpers, current repo state
agentrail next # current step prompt + plan context
```
Read both outputs carefully. `onboarding` surfaces the branch
policy, the `dg-*` helpers, and any pending `pr/*` branches waiting
for the coordinator. `agentrail next` contains your current step,
prompt, plan context, and any relevant skills/trajectories.

### 2. BRANCH (create a work branch for the step)
```bash
dg-new-feature <slug>    # or dg-new-fix <slug> for a bug fix
```
Use the step's slug as the topic. This switches to `dev`, fetches,
and creates `feat/<slug>`.

### 3. BEGIN (tell AgentRail the step is started)
```bash
agentrail begin
```

### 4. WORK (do what the step prompt says)
Do NOT ask "want me to proceed?". The step prompt IS your
instruction. Execute it directly.

### 5. COMMIT (after the work is done)
Commit your code changes with git on the `feat/<slug>` branch. Do
NOT push -- the coordinator handles pushes. If using `/mw-cp`, stop
at the commit step (skip the push phase).
**Run `/mw-cp` in every repo that was modified during the step.**

### 6. COMPLETE (after committing)
```bash
agentrail complete --summary "what you accomplished" \
  --reward 1 \
  --actions "tools and approach used"
```
- If the step failed: `--reward -1 --failure-mode "what went wrong"`
- If the saga is finished: add `--done`

### 7. MARK PR (signal ready-to-merge)
```bash
dg-mark-pr               # renames feat/<slug> -> pr/<slug>
```

### 8. STOP (after mark-pr, DO NOT continue working)
Do NOT make further code changes after `dg-mark-pr`. Any changes
after complete/mark-pr are outside the step's recorded scope.
Future work belongs in the NEXT step on a NEW branch.

Before starting the next step, fast-forward local `dev`:
```bash
dg-reap     # or: git switch dev && git fetch --all --prune && git merge --ff-only
```

## Key Rules

- **Never push** -- coordinator-only.
- **Never commit to `main` or `dev`** -- always work on `feat/*`
  or `fix/*`.
- **Do NOT skip AgentRail steps** -- the next session depends on
  accurate tracking.
- **Do NOT ask for permission** -- the step prompt is the instruction.
- **Do NOT continue working** after `dg-mark-pr`.
- **Commit before complete** -- always commit first, then record
  completion, then mark-pr.
- **pasm.s and pa24r must stay in sync** -- mirror every opcode,
  directive, or encoding change in both implementations.

## Useful Commands

```bash
agentrail status          # current saga state
agentrail history         # all completed steps
agentrail plan            # view the plan
agentrail next            # current step + context
```

## Cross-Repo Context

All COR24 repos live under `$ORGROOT` (`.../sw-embed/`) as siblings.
Most relevant to this project (downstream consumers of the p-code
VM and toolchain):

- `sw-cor24-emulator` (a.k.a. `cor24-rs`) -- `cor24-run` emulator
  used to execute `pvm.s` and `pvmasm.s`.
- `sw-cor24-pascal` -- Pascal compiler (`p24p`) targeting this VM.
- `sw-cor24-plsw` -- alternate Pascal-to-p-code compiler.
- `sw-cor24-basic` -- BASIC interpreter on the p-code VM.
- `sw-cor24-script` -- sws scripting language on the p-code VM.
- `sw-cor24-project` -- ecosystem umbrella / migration tracking.
