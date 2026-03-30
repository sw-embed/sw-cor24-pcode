; t10-traps.spc — Trap handling test (division by zero)
; Expected output: TRAP 1\n
; Trap code 1 = division by zero

.proc main 0
    push_s 42
    push_s 0
    div
    ; Should never reach here
    push_s 63
    sys 1
    halt
.end
