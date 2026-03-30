; t06-compare.spc — Comparison ops
; Expected output: YNYNYN\n

.proc main 0
    push_s 5
    push_s 5
    eq
    call pyn
    push_s 5
    push_s 5
    ne
    call pyn
    push_s 3
    push_s 7
    lt
    call pyn
    push_s 3
    push_s 7
    gt
    call pyn
    push_s 4
    push_s 4
    le
    call pyn
    push_s 3
    push_s 7
    ge
    call pyn
    push_s 10
    sys 1
    halt
.end

.proc pyn 1
    loada 0
    jz no
    push_s 89
    sys 1
    ret 1
no:
    push_s 78
    sys 1
    ret 1
.end
