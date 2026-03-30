; merged_noenter.spc — Test write_int WITHOUT enter/leave (full runtime style)
; Expected output: 42\n

.proc main 0
    push 42
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end

; Full runtime version: NO enter/leave
.proc _p24p_write_int 1
    loada 0
    dup
    push 0
    lt
    jz positive
    push 45
    sys 1
    neg
positive:
    storel 0
    push 0
extract:
    loadl 0
    push 10
    mod
    push 48
    add
    loadl 0
    push 10
    div
    storel 0
    loadl 0
    jnz extract
wi_print:
    dup
    jz wi_done
    sys 1
    jmp wi_print
wi_done:
    drop
    ret 1
.end

.proc _p24p_write_ln 0
    push 10
    sys 1
    ret 0
.end
