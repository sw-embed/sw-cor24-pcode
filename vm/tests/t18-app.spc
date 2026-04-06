; t18-app.spc — App unit that xcalls mathlib.gcd and mathlib.factorial
; Expected output: 6720 (gcd(48,18)=6, factorial(6)=720)
; Prints each digit as ASCII via print_num helper

.unit app
.import mathlib
.extern gcd 2
.extern factorial 1

.proc main 0
    ; gcd(48, 18) = 6
    push_s 48
    push_s 18
    xcall gcd
    call print_num

    ; factorial(6) = 720
    push_s 6
    xcall factorial
    call print_num

    ; newline
    push_s 10
    sys 1
    halt
.end

; print_num(n) — print decimal number (recursive digit extraction)
.proc print_num 2
    loada 0
    storel 0            ; local0 = n
    loadl 0
    push_s 10
    mod
    storel 1            ; local1 = n % 10
    loadl 0
    push_s 10
    div
    storel 0            ; local0 = n / 10
    loadl 0
    jz print_digit      ; if n/10 == 0, just print last digit
    loadl 0
    call print_num      ; recurse for higher digits
print_digit:
    loadl 1
    push_s 48           ; '0'
    add
    sys 1               ; putc digit
    ret 1
.end
