; merged_mid_int.spc — Medium linked program (8 runtime procs)
; Tests main calling write_int/write_ln, with several unused runtime procs
; Expected output: 42\nTRUE\n-7\n

; --- module: app ---
.proc main 0
    push 42
    call _p24p_write_int
    call _p24p_write_ln
    push 1
    call _p24p_write_bool
    call _p24p_write_ln
    push -7
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end

; --- module: runtime ---

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

.proc _p24p_write_bool 0
    loada 0
    jz wb_false
    push 84
    sys 1
    push 82
    sys 1
    push 85
    sys 1
    push 69
    sys 1
    jmp wb_done
wb_false:
    push 70
    sys 1
    push 65
    sys 1
    push 76
    sys 1
    push 83
    sys 1
    push 69
    sys 1
wb_done:
    ret 1
.end

.proc _p24p_write_ln 0
    push 10
    sys 1
    ret 0
.end

.proc _p24p_write_str 1
    loada 0
    storel 0
ws_loop:
    loadl 0
    loadb
    dup
    jz ws_done
    sys 1
    loadl 0
    push 1
    add
    storel 0
    jmp ws_loop
ws_done:
    drop
    ret 1
.end

.proc _p24p_abs 0
    loada 0
    dup
    push 0
    lt
    jz abs_done
    neg
abs_done:
    ret 1
.end

.proc _p24p_odd 0
    loada 0
    push 2
    mod
    push 0
    ne
    ret 1
.end

.proc _p24p_bounds_check 0
    loada 2
    loada 1
    lt
    jnz bc_fail
    loada 2
    loada 0
    gt
    jnz bc_fail
    ret 3
bc_fail:
    push 66
    sys 1
    push 79
    sys 1
    push 85
    sys 1
    push 78
    sys 1
    push 68
    sys 1
    push 83
    sys 1
    push 10
    sys 1
    sys 0
.end

.proc _p24p_nil_check 0
    loada 0
    jnz nc_ok
    push 78
    sys 1
    push 73
    sys 1
    push 76
    sys 1
    push 10
    sys 1
    sys 0
nc_ok:
    loada 0
    ret 1
.end
