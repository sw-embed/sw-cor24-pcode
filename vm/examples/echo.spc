; echo.spc — Echo UART input back
; Reads chars and echoes until newline
.proc main 0
lp:
sys 2
dup
sys 1
push_s 10
eq
jnz dn
jmp lp
dn:
halt
.end
