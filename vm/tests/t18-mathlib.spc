; t18-mathlib.spc — Library unit exporting gcd and factorial
; Used by t18-multiunit-app.spc via xcall

.unit mathlib
.export gcd 2
.export factorial 1

.proc main 0
    halt
.end

; gcd(a, b) — Euclidean algorithm
; Returns gcd of two positive integers
.proc gcd 2
    loada 1             ; b
    jz gcd_base
    loada 0             ; a
    loada 1             ; b
    mod                 ; a % b
    loada 1             ; b
    call gcd            ; gcd(b, a%b)
    ret 2
gcd_base:
    loada 0             ; a
    ret 2
.end

; factorial(n) — recursive factorial
; Returns n!
.proc factorial 1
    loada 0             ; n
    push_s 1
    le                  ; n <= 1?
    jz fact_recurse
    push_s 1            ; base case: return 1
    ret 1
fact_recurse:
    loada 0             ; n
    loada 0             ; n
    push_s 1
    sub                 ; n-1
    call factorial      ; factorial(n-1)
    mul                 ; n * factorial(n-1)
    ret 1
.end
