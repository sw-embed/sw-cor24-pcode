.module app
.export main
.extern _p24p_write_int
.extern _p24p_write_bool
.extern _p24p_write_ln

; Simple Pascal-like app: writeln(42); writeln(true); writeln(-7)
; Expected output:
;   42
;   TRUE
;   -7

.proc main 0
    ; writeln(42)
    push 42
    call _p24p_write_int
    call _p24p_write_ln
    ; writeln(true)
    push 1
    call _p24p_write_bool
    call _p24p_write_ln
    ; writeln(-7)
    push -7
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end

.endmodule
