; merged_int.spc — Linked program: main + runtime (write_int, write_ln)
; Simulates pl24r linker output: main first, runtime follows
; Expected output: 42\n

; --- module: app ---
.proc main 0
    push 42
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end

; --- module: runtime ---

; _p24p_write_int ( n -- )
; Print signed integer to UART as decimal.
.proc _p24p_write_int 1
    loada 0              ; load argument n
    dup                  ; n n
    push 0               ; n n 0
    lt                   ; n (n<0?)
    jz positive
    push 45              ; '-'
    sys 1                ; n
    neg                  ; -n
positive:
    storel 0             ; local0 = abs(n)
    push 0               ; push sentinel
extract:
    loadl 0              ; load n
    push 10
    mod                  ; n % 10
    push 48
    add                  ; n % 10 + '0' = digit char
    loadl 0
    push 10
    div                  ; n / 10
    storel 0             ; update n = n / 10
    loadl 0
    jnz extract          ; if n != 0, extract more digits
wi_print:
    dup
    jz wi_done
    sys 1                ; PUTC digit
    jmp wi_print
wi_done:
    drop                 ; discard sentinel
    ret 1
.end

; _p24p_write_ln ( -- )
.proc _p24p_write_ln 0
    push 10              ; LF
    sys 1
    ret 0
.end
