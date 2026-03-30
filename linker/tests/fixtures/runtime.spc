.module runtime
.export _p24p_write_int
.export _p24p_write_ln

.proc _p24p_write_int 1
    enter 1
    loada 0
    dup
    push 0
    lt
    jz positive
    push 45              ; '-'
    sys 1                ; PUTC
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
print:
    dup
    jz done
    sys 1                ; PUTC
    jmp print
done:
    drop
    leave
    ret 1
.end

.proc _p24p_write_ln 0
    push 10
    sys 1                ; PUTC
    ret 0
.end

.endmodule
