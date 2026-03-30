; t08 — Recursion test
; Expected output: 120\n
.proc main 0
push_s 5
call fact
call pn
push_s 10
sys 1
halt
.end
.proc fact 2
loada 0
storel 0
push 1
storel 1
lp:
loadl 0
push 1
le
jnz dn
loadl 1
loadl 0
mul
storel 1
loadl 0
push 1
sub
storel 0
jmp lp
dn:
loadl 1
ret 1
.end
.proc pn 2
loada 0
storel 0
loadl 0
push_s 10
mod
storel 1
loadl 0
push_s 10
div
storel 0
loadl 0
jz la
loadl 0
call pn
la:
loadl 1
push_s 48
add
sys 1
ret 1
.end
