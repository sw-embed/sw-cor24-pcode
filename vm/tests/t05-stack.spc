; t05-stack.spc — Stack operation test
; Expected output: XXHiCAC\n

.proc main 0
    ; dup test
    push_s 88
    dup
    sys 1
    sys 1
    ; swap test
    push_s 72
    push_s 105
    swap
    sys 1
    sys 1
    ; over test
    push_s 67
    push_s 65
    over
    sys 1
    sys 1
    sys 1
    ; drop test
    push_s 99
    drop
    push_s 10
    sys 1
    halt
.end
