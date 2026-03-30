#!/bin/bash
# pl24r pipeline example — full build flow from .spc to COR24 executable
#
# This script demonstrates the intended build pipeline. It requires:
#   - pl24r (this project, built with `cargo build --release`)
#   - pasm  (from pv24a — the COR24 p-code assembler)
#   - pv24a (from pv24a — the COR24 p-code VM/emulator)
#
# Usage:
#   ./scripts/pipeline.sh runtime.spc app.spc
#
# The pipeline:
#   1. pl24r links runtime + app → combined.spc
#   2. pasm assembles combined.spc → combined.p24
#   3. pv24a runs combined.p24

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Paths — adjust if tools are installed elsewhere
PL24R="${PL24R:-$PROJECT_DIR/target/release/pl24r}"
PASM="${PASM:-pasm}"
PV24A="${PV24A:-pv24a}"

if [ $# -lt 2 ]; then
    echo "Usage: $0 <runtime.spc> <app.spc> [extra.spc...]"
    echo ""
    echo "Example:"
    echo "  $0 ~/github/softwarewrighter/pr24p/src/runtime.spc my_app.spc"
    exit 1
fi

COMBINED=$(mktemp /tmp/pl24r_combined_XXXXXX.spc)
BINARY=$(mktemp /tmp/pl24r_output_XXXXXX.p24)
trap 'rm -f "$COMBINED" "$BINARY"' EXIT

echo "=== Step 1: Link ==="
echo "  pl24r $* -o $COMBINED"
"$PL24R" "$@" -o "$COMBINED" -v

echo ""
echo "=== Step 2: Assemble ==="
echo "  pasm $COMBINED -o $BINARY"
if command -v "$PASM" &>/dev/null; then
    "$PASM" "$COMBINED" -o "$BINARY"
else
    echo "  [SKIP] pasm not found — install pv24a to assemble"
    echo "  The combined .spc is at: $COMBINED"
    exit 0
fi

echo ""
echo "=== Step 3: Run ==="
echo "  pv24a $BINARY"
if command -v "$PV24A" &>/dev/null; then
    "$PV24A" "$BINARY"
else
    echo "  [SKIP] pv24a not found — install pv24a emulator to run"
    echo "  The .p24 binary is at: $BINARY"
fi
