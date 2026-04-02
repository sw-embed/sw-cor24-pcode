; t12-memset.spc — MEMSET opcode test
; Expected output: AAAA

.data buf 0, 0, 0, 0, 0, 0

.proc main 0
    ; Zero-length set (should be a no-op)
    push buf
    push_s 88
    push_s 0
    memset
    ; Fill 4 bytes with 'A' (65)
    push buf
    push_s 65
    push_s 4
    memset
    ; Print buf[0..3]
    push buf
    loadb
    sys 1
    push buf
    push_s 1
    add
    loadb
    sys 1
    push buf
    push_s 2
    add
    loadb
    sys 1
    push buf
    push_s 3
    add
    loadb
    sys 1
    push_s 10
    sys 1
    halt
.end
