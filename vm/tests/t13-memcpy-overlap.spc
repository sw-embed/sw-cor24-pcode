; t13-memcpy-overlap.spc — MEMCPY with overlapping regions (memmove)
; Expected output: ABABC
;
; buf starts as: A B C 0 0
; Copy 3 bytes from buf to buf+2 (dst > src, overlapping)
; With memmove semantics (backward copy): result is A B A B C

.data buf 65, 66, 67, 0, 0, 0

.proc main 0
    ; Overlapping copy: src=buf, dst=buf+2, len=3
    push buf
    push buf
    push_s 2
    add
    push_s 3
    memcpy
    ; Print 5 bytes of buf
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
    push buf
    push_s 4
    add
    loadb
    sys 1
    push_s 10
    sys 1
    halt
.end
