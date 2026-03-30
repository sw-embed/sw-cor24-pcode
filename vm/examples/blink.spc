; blink.spc — Toggle LED on/off
; Writes alternating 1/0 to LED port
.proc main 0
push 1
sys 3
push_s 0
sys 3
push 1
sys 3
push_s 0
sys 3
halt
.end
