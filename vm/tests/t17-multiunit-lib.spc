; t17 multi-unit test — mathlib unit
; Exports double(x) = x * 2

.unit mathlib
.export double 1

.proc main 0
    halt
.end

.proc double 1
    loada 0
    dup
    add
    ret 1
.end
