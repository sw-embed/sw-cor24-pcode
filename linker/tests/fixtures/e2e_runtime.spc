.module runtime
.export _p24p_write_int
.export _p24p_write_bool
.export _p24p_write_ln

; pr24p — Pascal Runtime Library
; Phase 0: Hand-written .spc stubs for p-code VM syscall wrappers

; _p24p_write_int ( n -- )
; Print signed integer to UART as decimal.
.proc _p24p_write_int 1
    enter 1
    loada 0              ; load argument n
    ; check for negative
    dup                  ; n n
    push 0               ; n n 0
    lt                   ; n (n<0?)
    jz positive
    ; print minus sign
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
print:
    dup
    jz done
    sys 1                ; PUTC digit
    jmp print
done:
    drop                 ; discard sentinel
    leave
    ret 1
.end

; _p24p_write_bool ( b -- )
; Print TRUE or FALSE to UART.
.proc _p24p_write_bool 1
    enter 0
    loada 0              ; load argument b
    jz print_false
    ; print "TRUE"
    push 84              ; 'T'
    sys 1
    push 82              ; 'R'
    sys 1
    push 85              ; 'U'
    sys 1
    push 69              ; 'E'
    sys 1
    jmp bool_done
print_false:
    ; print "FALSE"
    push 70              ; 'F'
    sys 1
    push 65              ; 'A'
    sys 1
    push 76              ; 'L'
    sys 1
    push 83              ; 'S'
    sys 1
    push 69              ; 'E'
    sys 1
bool_done:
    leave
    ret 1
.end

; _p24p_write_ln ( -- )
; Print newline (LF) to UART via sys 1 (PUTC).
.proc _p24p_write_ln 0
    enter 0
    push 10              ; LF
    sys 1
    leave
    ret 0
.end

.endmodule
