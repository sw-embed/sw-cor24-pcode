; t17 multi-unit test — app unit
; Calls double(21) from mathlib via xcall, prints result char

.unit app
.import mathlib
.extern double 1

.proc main 0
    push_s 21       ; 21 * 2 = 42 = '*'
    xcall double
    sys 1           ; PUTC '*'
    push_s 10       ; '\n'
    sys 1
    halt
.end
