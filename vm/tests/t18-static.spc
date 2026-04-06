; t18-static.spc — Statically-linked version of t18 (app + mathlib)
; Expected output: 6720
; Same logic as t18-app + t18-mathlib but in a single file

.proc main 0
    ; gcd(48, 18) = 6
    push_s 48
    push_s 18
    call gcd
    call print_num

    ; factorial(6) = 720
    push_s 6
    call factorial
    call print_num

    ; newline
    push_s 10
    sys 1
    halt
.end

.proc gcd 2
    loada 1
    jz gcd_base
    loada 0
    loada 1
    mod
    loada 1
    call gcd
    ret 2
gcd_base:
    loada 0
    ret 2
.end

.proc factorial 1
    loada 0
    push_s 1
    le
    jz fact_recurse
    push_s 1
    ret 1
fact_recurse:
    loada 0
    loada 0
    push_s 1
    sub
    call factorial
    mul
    ret 1
.end

.proc print_num 2
    loada 0
    storel 0
    loadl 0
    push_s 10
    mod
    storel 1
    loadl 0
    push_s 10
    div
    storel 0
    loadl 0
    jz print_digit
    loadl 0
    call print_num
print_digit:
    loadl 1
    push_s 48
    add
    sys 1
    ret 1
.end
