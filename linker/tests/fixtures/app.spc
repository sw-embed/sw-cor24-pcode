.module app
.export main
.extern _p24p_write_int
.extern _p24p_write_ln

.proc main 0
    push 42
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end

.endmodule
