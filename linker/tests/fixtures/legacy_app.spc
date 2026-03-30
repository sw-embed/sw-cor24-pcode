; Legacy app without module metadata
; All symbols are exported by default

.data msg 72,101,108,108,111,0

.proc main 0
    push msg
    call puts
    halt
.end

.proc puts 1
    loada 0
    storel 0
loop:
    loadl 0
    loadb
    dup
    jz done
    sys 1
    loadl 0
    push 1
    add
    storel 0
    jmp loop
done:
    drop
    ret 1
.end
