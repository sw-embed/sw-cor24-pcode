; merged_str.spc — Linked program: main + runtime (write_str, write_ln)
; Simulates pl24r linker output with .data directive
; Expected output: Hello\n

.data hello 72, 101, 108, 108, 111, 0

; --- module: app ---
.proc main 0
    push hello
    call _p24p_write_str
    call _p24p_write_ln
    halt
.end

; --- module: runtime ---

; _p24p_write_str ( addr -- )
.proc _p24p_write_str 1
    loada 0              ; load string address
    storel 0             ; local0 = pointer
ws_loop:
    loadl 0              ; load pointer
    loadb                ; load byte at pointer
    dup
    jz ws_done           ; null terminator -> done
    sys 1                ; PUTC
    loadl 0
    push 1
    add
    storel 0             ; pointer++
    jmp ws_loop
ws_done:
    drop                 ; discard null byte
    ret 1
.end

; _p24p_write_ln ( -- )
.proc _p24p_write_ln 0
    push 10              ; LF
    sys 1
    ret 0
.end
