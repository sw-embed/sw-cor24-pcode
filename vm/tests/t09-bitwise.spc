; t09 — Bitwise ops
; Expected output: OKOKOKOK\n
.proc main 0
push 255
push 15
and
push_s 15
eq
call chk
push_s 48
push_s 5
or
push_s 53
eq
call chk
push 1
push_s 3
shl
push_s 8
eq
call chk
push 255
push 255
xor
push_s 0
eq
call chk
push_s 10
sys 1
halt
.end
.proc chk 1
loada 0
jz no
push_s 79
sys 1
push_s 75
sys 1
ret 1
no:
push_s 78
sys 1
push_s 79
sys 1
ret 1
.end
