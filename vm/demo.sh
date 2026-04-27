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

# Paths to toolchain binaries (built by cargo)
PA24R="../target/debug/pa24r"
P24LOAD="../target/debug/p24-load"

# check_multiunit NAME APP_SPC LIB_SPC EXPECTED
# Full multi-unit pipeline: assemble units → link → load .p24m on pvm.s
check_multiunit() {
    local name="$1" app_file="$2" lib_file="$3" expected="$4"

    if [ ! -f "$app_file" ] || [ ! -f "$lib_file" ]; then
        printf "  ${YELLOW}SKIP${NC}  %-20s  (file not found)\n" "$name"
        SKIP=$((SKIP + 1))
        return
    fi

    if [ ! -x "$PA24R" ] || [ ! -x "$P24LOAD" ]; then
        printf "  ${YELLOW}SKIP${NC}  %-20s  (cargo build needed)\n" "$name"
        SKIP=$((SKIP + 1))
        return
    fi

    local tmpdir
    tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" RETURN

    # Assemble each unit
    "$PA24R" "$app_file" -o "$tmpdir/app.p24" 2>/dev/null || { printf "  ${RED}FAIL${NC}  %-20s  (pa24r app failed)\n" "$name"; FAIL=$((FAIL+1)); return; }
    "$PA24R" "$lib_file" -o "$tmpdir/lib.p24" 2>/dev/null || { printf "  ${RED}FAIL${NC}  %-20s  (pa24r lib failed)\n" "$name"; FAIL=$((FAIL+1)); return; }

    # Link
    "$P24LOAD" "$tmpdir/app.p24" "$tmpdir/lib.p24" -o "$tmpdir/image.p24m" 2>/dev/null || { printf "  ${RED}FAIL${NC}  %-20s  (p24-load failed)\n" "$name"; FAIL=$((FAIL+1)); return; }

    # Assemble pvm.s and find code_ptr
    cor24-run --assemble pvm.s "$tmpdir/pvm.bin" "$tmpdir/pvm.lst" >/dev/null 2>&1
    local code_ptr
    code_ptr=$(grep -A1 "code_ptr:" "$tmpdir/pvm.lst" | tail -1 | awk '{print $1}' | tr -d ':')

    # Run
    local raw
    raw=$(cor24-run --load-binary "$tmpdir/pvm.bin@0" --load-binary "$tmpdir/image.p24m@0x010000" --patch "0x${code_ptr}=0x010000" --entry 0 --speed 0 -n "$MAX_INSN" --terminal 2>&1)

    # Extract: everything between "PVM OK" and "HALT"
    local actual
    actual=$(echo "$raw" | awk '
        /^PVM OK$/ { found=1; next }
        /^HALT$/ { found=0; next }
        /^TRAP [0-9]/ && found { print; found=0; next }
        found { print }
    ')

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
    echo "${BOLD}Memory block operations:${NC}"
    check "t11-memcpy"        "tests/t11-memcpy.spc"        "ABCABC"
    check "t12-memset"        "tests/t12-memset.spc"        "AAAA"
    check "t13-memcpy-overlap" "tests/t13-memcpy-overlap.spc" "ABABC"
    check "t14-memcmp"        "tests/t14-memcmp.spc"        "OKLTOKGT"
    check "t15-jmp_ind"       "tests/t15-jmp_ind.spc"       "ABOK"
    check "t16-xcall"         "tests/t16-xcall.spc"         "XOK"
    check "t19-malloc-reuse"  "tests/t19-malloc-reuse.spc"  "OK"

    echo ""
    echo "${BOLD}Linked programs:${NC}"
    check "merged-int"   "tests/merged_int.spc"   "42"
    check "merged-str"   "tests/merged_str.spc"   "Hello"

    echo ""
    echo "${BOLD}Multi-unit (.p24m):${NC}"
    check "t18-static"   "tests/t18-static.spc"   "6720"
    check_multiunit "t18-multiunit" "tests/t18-app.spc" "tests/t18-mathlib.spc" "6720"

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
