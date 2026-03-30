#!/bin/bash
# demo.sh — Test harness for pv24a P-Code VM
#
# Usage:
#   ./demo.sh           Run all tests
#   ./demo.sh test      Run all tests (same as no args)
#   ./demo.sh run FILE  Assemble and run a single .spc file

set -euo pipefail

PVMASM="pvmasm.s"
MAX_INSN=20000000
PASS=0
FAIL=0
SKIP=0

# Colors (if terminal supports them)
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    YELLOW='\033[0;33m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    GREEN='' RED='' YELLOW='' BOLD='' NC=''
fi

# run_spc FILE — assemble and run an .spc file, return UART output
run_spc() {
    local file="$1"
    local input
    input=$(cat "$file")
    cor24-run --run "$PVMASM" -u "${input}"$'\x04' --speed 0 -n "$MAX_INSN" 2>&1
}

# extract_output RAW — extract program output between RUN and HALT/TRAP
extract_output() {
    local raw="$1"
    # Get lines between RUN and HALT (exclusive), or capture TRAP line
    echo "$raw" | awk '
        /^RUN$/ { found=1; next }
        /^HALT$/ { found=0; next }
        /^TRAP [0-9]/ && found { print; found=0; next }
        found { print }
    '
}

# check NAME FILE EXPECTED — run test and compare output
check() {
    local name="$1"
    local file="$2"
    local expected="$3"

    if [ ! -f "$file" ]; then
        printf "  ${YELLOW}SKIP${NC}  %-20s  (file not found)\n" "$name"
        SKIP=$((SKIP + 1))
        return
    fi

    local raw actual
    raw=$(run_spc "$file")
    actual=$(extract_output "$raw")

    if [ "$actual" = "$expected" ]; then
        printf "  ${GREEN}PASS${NC}  %-20s\n" "$name"
        PASS=$((PASS + 1))
    else
        printf "  ${RED}FAIL${NC}  %-20s\n" "$name"
        printf "        expected: %s\n" "$(echo "$expected" | cat -v)"
        printf "        actual:   %s\n" "$(echo "$actual" | cat -v)"
        FAIL=$((FAIL + 1))
    fi
}

# run_file FILE — assemble and run, show output
run_file() {
    local file="$1"
    if [ ! -f "$file" ]; then
        echo "Error: $file not found"
        exit 1
    fi
    echo "Assembling and running: $file"
    local raw
    raw=$(run_spc "$file")
    extract_output "$raw"
}

run_tests() {
    echo "${BOLD}pv24a test suite${NC}"
    echo ""

    echo "${BOLD}Core operations:${NC}"
    check "t01-hello"    "tests/t01-hello.spc"    "Hello"
    check "t02-arith"    "tests/t02-arith.spc"    "*"
    check "t03-globals"  "tests/t03-globals.spc"  "AB"
    check "t04-loop"     "tests/t04-loop.spc"     "54321"

    echo ""
    echo "${BOLD}Stack and logic:${NC}"
    check "t05-stack"    "tests/t05-stack.spc"    "XXHiCAC"
    check "t06-compare"  "tests/t06-compare.spc"  "YNYNYN"

    echo ""
    echo "${BOLD}Memory and procedures:${NC}"
    check "t07-memory"   "tests/t07-memory.spc"   "AZ"
    check "t08-nested"   "tests/t08-nested.spc"   "120"

    echo ""
    echo "${BOLD}Advanced:${NC}"
    check "t09-bitwise"  "tests/t09-bitwise.spc"  "OKOKOKOK"
    check "t10-traps"    "tests/t10-traps.spc"    "TRAP 1"

    echo ""
    echo "${BOLD}Linked programs:${NC}"
    check "merged-int"   "tests/merged_int.spc"   "42"
    check "merged-str"   "tests/merged_str.spc"   "Hello"

    echo ""
    echo "────────────────────────────"
    printf "${GREEN}%d passed${NC}" "$PASS"
    [ "$FAIL" -gt 0 ] && printf ", ${RED}%d failed${NC}" "$FAIL"
    [ "$SKIP" -gt 0 ] && printf ", ${YELLOW}%d skipped${NC}" "$SKIP"
    echo ""

    [ "$FAIL" -gt 0 ] && exit 1
    exit 0
}

# Main
case "${1:-test}" in
    test)
        run_tests
        ;;
    run)
        if [ -z "${2:-}" ]; then
            echo "Usage: $0 run FILE.spc"
            exit 1
        fi
        run_file "$2"
        ;;
    *)
        echo "Usage: $0 [test|run FILE.spc]"
        exit 1
        ;;
esac
