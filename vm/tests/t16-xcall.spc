; t16-xcall.spc — XCALL cross-unit call test via IRT
; Expected output: XOK
;
; Main must be FIRST proc (pvmasm.s starts at pc=0).
; print_x follows at a known code offset.
; We allocate IRT on heap, write print_x's offset, set irt_base, xcall.

.unit test_xcall
.import fake_unit
.extern ext_fn

.proc main 0
    ; Allocate 3 bytes for 1 IRT slot on heap
    push_s 3
    sys 4           ; ALLOC → addr

    ; Write print_x's code offset into IRT[0]
    ; main: enter(2) + push_s(2) + sys(2) + dup(1)*3 + push_s(2)*3
    ;        + swap(1)*3 + storeb(1)*3 + add(1)*2 + sys(2) + xcall(3)
    ;        + push_s(2)*2 + sys(2)*2 + halt(1) + leave(1)
    ; Actually, let's just use push with the label
    dup
    push print_x    ; code offset of print_x
    swap
    store           ; mem[addr] = print_x offset (as 3-byte word)

    ; Set irt_base
    sys 7           ; SET_IRT_BASE (pops addr)

    ; xcall slot 0 → print_x
    xcall ext_fn

    ; After return
    push_s 79       ; 'O'
    sys 1
    push_s 75       ; 'K'
    sys 1
    push_s 10       ; '\n'
    sys 1
    halt
.end

.proc print_x 0
    push_s 88       ; 'X'
    sys 1           ; PUTC
    ret 0
.end
