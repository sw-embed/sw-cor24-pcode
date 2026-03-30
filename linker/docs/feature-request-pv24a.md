# Feature Request: pv24a/pasm — Document Module Metadata Directives as Reserved

**Target repo:** [softwarewrighter/pv24a](https://github.com/softwarewrighter/pv24a)
**Component:** pasm (p-code assembler)
**Depends on:** pl24r linker module metadata convention
**Priority:** Low — pasm already handles this correctly by skipping unknown directives

## Current Behavior

pasm currently silently skips any `.`-directive it doesn't recognize. This means `.module`, `.export`, `.extern`, and `.endmodule` are ignored without error. This is the correct behavior — pl24r strips these directives from its output before pasm sees them in the normal pipeline, but if someone feeds an unlinked `.spc` file directly to pasm, it should not break.

## Requested Changes

### 1. Document the metadata directives in `docs/design.md`

The `.spc` format specification in `docs/design.md` should document the four linking directives as part of the format, even though pasm doesn't process them:

```
## Module Metadata (for linking)

The following directives are used by the pl24r linker for cross-module
symbol resolution. pasm ignores them — they are stripped by the linker
before assembly. They are documented here as part of the .spc format.

  .module <name>     — declare module identity
  .export <sym>      — mark symbol as visible to other modules
  .extern <sym>      — declare external symbol dependency
  .endmodule         — end module boundary
```

This ensures the `.spc` format is documented in one authoritative place and future pasm maintainers understand why these directives appear in `.spc` files.

### 2. (Optional) Warn on unrecognized directives instead of silent skip

Currently pasm silently ignores unknown directives. A possible improvement:

- **Known linker directives** (`.module`, `.export`, `.extern`, `.endmodule`) — skip silently (these are expected in unlinked `.spc` files)
- **Truly unknown directives** — emit a warning to stderr

This would catch typos like `.proce` or `.golbal` that currently pass silently. This is optional and low priority.

## Context

- **Module metadata spec:** [pl24r CLAUDE.md](https://github.com/softwarewrighter/pl24r/blob/main/CLAUDE.md) — "Module metadata format" section
- **Linker design research:** [pl24r docs/research.txt](https://github.com/softwarewrighter/pl24r/blob/main/docs/research.txt)
- **Current .spc spec:** `pv24a/docs/design.md` — the authoritative reference for the `.spc` format

## The Full Pipeline

```
Pascal source → p24p → .spc (with metadata) → pl24r → combined .spc (metadata stripped) → pasm → .p24 → pv24a
```

pasm sits downstream of pl24r, so it normally sees clean `.spc` without metadata. The documentation change ensures the format is fully specified even for the upstream stages.

## Notes

- This is primarily a documentation change, not a code change.
- pasm's current "skip unknown directives" behavior is already correct for forward compatibility.
- The `.spc` format is language-agnostic — these directives support any compiler frontend targeting COR24, not just Pascal.
