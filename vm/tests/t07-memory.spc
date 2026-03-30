; t07-memory.spc — Indirect memory access test
; Expected output: AZ\n

.data buf 0, 0, 0, 0, 0, 0

.proc main 0
    ; Store word 65 at buf, load and print
    push_s 65
    push buf
    store
    push buf
    load
    sys 1
    ; Store byte 90 at buf+3, load and print
    push_s 90
    push buf
    push_s 3
    add
    storeb
    push buf
    push_s 3
    add
    loadb
    sys 1
    push_s 10
    sys 1
    halt
.end
