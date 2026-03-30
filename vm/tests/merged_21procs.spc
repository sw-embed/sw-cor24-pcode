; merged_21procs.spc — Linked program with 21 runtime procedures
; Reproduces original merged_int.spc symbol count (~58)
; Expected output: 42\n

.proc main 0
    push 42
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end


; pr24p — Pascal Runtime Library
; Phase 0: Hand-written .spc stubs for p-code VM syscall wrappers

; _p24p_write_int ( n -- )
; Print signed integer to UART as decimal.
; Handles negative numbers (prints '-' prefix).
; Uses div/mod by 10 to extract digits, pushes onto eval stack
; in reverse order with a zero sentinel, then prints from most
; significant to least significant via sys 1 (PUTC).
.proc _p24p_write_int 1
    loada 0              ; load argument n
    ; check for negative
    dup                  ; n n
    push 0               ; n n 0
    lt                   ; n (n<0?)
    jz positive
    ; print minus sign
    push 45              ; n '-'
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

; _p24p_write_bool ( b -- )
; Print TRUE or FALSE to UART.
; Compares argument to 0: if zero prints FALSE, otherwise prints TRUE.
; Each character is pushed and output via sys 1 (PUTC).
.proc _p24p_write_bool 0
    loada 0              ; load argument b
    jz wb_false
    ; print "TRUE"
    push 84              ; 'T'
    sys 1
    push 82              ; 'R'
    sys 1
    push 85              ; 'U'
    sys 1
    push 69              ; 'E'
    sys 1
    jmp wb_done
wb_false:
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
wb_done:
    ret 1
.end

; _p24p_write_ln ( -- )
; Print newline (LF) to UART via sys 1 (PUTC).
.proc _p24p_write_ln 0
    push 10              ; LF
    sys 1
    ret 0
.end

; _p24p_write_str ( addr -- )
; Print null-terminated string from data segment to UART.
; Walks bytes via loadb, calls sys 1 (PUTC) for each until null.
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

; Phase 1: Standard functions
; Hand-written .spc until p24c supports function compilation.
; Will be replaced with Pascal-compiled versions when p24c Phase 1 lands.

; _p24p_abs ( x -- |x| )
; Absolute value. Returns x if x >= 0, else -x.
.proc _p24p_abs 0
    loada 0              ; load x
    dup
    push 0
    lt                   ; x < 0?
    jz abs_done
    neg                  ; negate if negative
abs_done:
    ret 1
.end

; _p24p_odd ( x -- x mod 2 <> 0 )
; Returns true (1) if x is odd, false (0) if even.
.proc _p24p_odd 0
    loada 0              ; load x
    push 2
    mod                  ; x mod 2
    push 0
    ne                   ; result != 0 -> true
    ret 1
.end

; _p24p_ord ( c -- c )
; Character to integer. Identity on this VM (chars are integers).
.proc _p24p_ord 0
    loada 0
    ret 1
.end

; _p24p_chr ( n -- n )
; Integer to character. Identity on this VM (chars are integers).
.proc _p24p_chr 0
    loada 0
    ret 1
.end

; _p24p_succ ( x -- x+1 )
; Next ordinal value.
.proc _p24p_succ 0
    loada 0
    push 1
    add
    ret 1
.end

; _p24p_pred ( x -- x-1 )
; Previous ordinal value.
.proc _p24p_pred 0
    loada 0
    push 1
    sub
    ret 1
.end

; _p24p_sqr ( x -- x*x )
; Square of integer.
.proc _p24p_sqr 0
    loada 0
    dup
    mul
    ret 1
.end

; Phase 1: Runtime checks
; Hand-written .spc until p24c supports procedure compilation.
; Pascal source: src/checks.pas

; _p24p_bounds_check ( index low high -- )
; Array bounds violation handler. Called by compiler-generated
; array access code. If index < low or index > high, prints
; "BOUNDS" to UART and halts.
.proc _p24p_bounds_check 0
    loada 2              ; load index (first pushed)
    loada 1              ; load low (second pushed)
    lt                   ; index < low?
    jnz bc_fail
    loada 2              ; load index
    loada 0              ; load high (last pushed)
    gt                   ; index > high?
    jnz bc_fail
    ret 3                ; in range, return normally
bc_fail:
    ; print "BOUNDS\n"
    push 66              ; 'B'
    sys 1
    push 79              ; 'O'
    sys 1
    push 85              ; 'U'
    sys 1
    push 78              ; 'N'
    sys 1
    push 68              ; 'D'
    sys 1
    push 83              ; 'S'
    sys 1
    push 10              ; LF
    sys 1
    sys 0                ; HALT
.end

; _p24p_nil_check ( ptr -- )
; Nil pointer dereference handler. Called before pointer
; dereference. If ptr = 0, prints "NIL" and halts.
; Otherwise returns with ptr still on stack for use.
.proc _p24p_nil_check 0
    loada 0              ; load ptr
    jnz nc_ok            ; non-nil, safe
    ; print "NIL\n"
    push 78              ; 'N'
    sys 1
    push 73              ; 'I'
    sys 1
    push 76              ; 'L'
    sys 1
    push 10              ; LF
    sys 1
    sys 0                ; HALT
nc_ok:
    loada 0              ; return ptr for caller to use
    ret 1
.end

; Phase 1: Read support
; Hand-written .spc until p24c supports function compilation.
; Pascal source: src/read.pas

; _p24p_read_char ( -- c )
; Read single character from UART via sys 2 (GETC).
.proc _p24p_read_char 0
    sys 2                ; GETC -> push char
    ret 0
.end

; _p24p_read_int ( -- n )
; Read signed integer from UART. Skips leading whitespace,
; handles optional +/- sign, accumulates decimal digits.
; local0 = n (accumulator), local1 = neg flag, local2 = current char
.proc _p24p_read_int 3
    push 0
    storel 0             ; n = 0
    push 0
    storel 1             ; neg = 0
    ; read first char, skip whitespace
    sys 2
    storel 2             ; c = getc
ri_ws:
    loadl 2
    push 32              ; space
    eq
    jnz ri_skip
    loadl 2
    push 9               ; tab
    eq
    jnz ri_skip
    jmp ri_sign
ri_skip:
    sys 2
    storel 2             ; c = getc
    jmp ri_ws
ri_sign:
    ; check for minus
    loadl 2
    push 45              ; '-'
    ne
    jnz ri_plus
    push 1
    storel 1             ; neg = 1
    sys 2
    storel 2             ; c = getc
    jmp ri_digits
ri_plus:
    ; check for plus
    loadl 2
    push 43              ; '+'
    ne
    jnz ri_digits
    sys 2
    storel 2             ; c = getc
ri_digits:
    ; check if c is a digit (48..57)
    loadl 2
    push 48
    lt
    jnz ri_negate        ; c < '0', stop
    loadl 2
    push 57
    gt
    jnz ri_negate        ; c > '9', stop
    ; n = n * 10 + (c - 48)
    loadl 0
    push 10
    mul
    loadl 2
    push 48
    sub
    add
    storel 0             ; n = n * 10 + digit
    sys 2
    storel 2             ; c = getc
    jmp ri_digits
ri_negate:
    loadl 1
    jz ri_done
    loadl 0
    neg
    storel 0             ; n = -n
ri_done:
    loadl 0              ; push result
    ret 0
.end

; _p24p_read_ln ( -- )
; Consume characters from UART until LF (10) is read.
.proc _p24p_read_ln 1
    sys 2
    storel 0             ; c = getc
rl_loop:
    loadl 0
    push 10              ; LF
    eq
    jnz rl_done
    sys 2
    storel 0             ; c = getc
    jmp rl_loop
rl_done:
    ret 0
.end

; Phase 2: Heap management
; Hand-written .spc until p24c supports pointer/record compilation.
; Pascal source: src/heap.pas

; Globals for heap tracking
; _h_ac = allocation count, _h_fc = free count
; _h_pt = pointer tracking table (16 slots)
.global _h_ac 1
.global _h_fc 1
.global _h_pt 16

; _p24p_heap_init ( -- )
; Initialize heap tracking. Zero counters and pointer table.
.proc _p24p_heap_init 1
    push 0
    storeg _h_ac            ; alloc_count = 0
    push 0
    storeg _h_fc            ; free_count = 0
    ; zero tracking table
    push 0
    storel 0                ; i = 0
hi_loop:
    loadl 0
    push 16
    ge
    jnz hi_done             ; if i >= 16, done
    ; compute address of _h_pt[i] and store 0
    push 0
    addrg _h_pt             ; base address of table
    loadl 0                 ; i
    push 3
    mul                     ; i * 3 (byte offset)
    add                     ; base + i*3
    store                   ; mem[base + i*3] = 0
    loadl 0
    push 1
    add
    storel 0                ; i++
    jmp hi_loop
hi_done:
    ret 0
.end

; _p24p_new ( size -- addr )
; Allocate size words via sys 4 (ALLOC). Track pointer.
.proc _p24p_new 2
    loada 0                 ; load size
    sys 4                   ; ALLOC -> addr on stack
    storel 0                ; local0 = addr
    ; alloc_count++
    loadg _h_ac
    push 1
    add
    storeg _h_ac
    ; find empty slot in tracking table
    push 0
    storel 1                ; i = 0
nw_loop:
    loadl 1
    push 16
    ge
    jnz nw_done             ; table full, skip tracking
    ; load _h_pt[i]
    addrg _h_pt
    loadl 1
    push 3
    mul
    add
    load                    ; _h_pt[i]
    jnz nw_next             ; slot occupied, try next
    ; store addr in empty slot
    loadl 0                 ; addr
    addrg _h_pt
    loadl 1
    push 3
    mul
    add
    store                   ; _h_pt[i] = addr
    jmp nw_done
nw_next:
    loadl 1
    push 1
    add
    storel 1                ; i++
    jmp nw_loop
nw_done:
    loadl 0                 ; push addr as return value
    ret 1
.end

; _p24p_dispose ( ptr -- )
; Free heap memory via sys 5 (FREE). Remove from tracking.
.proc _p24p_dispose 1
    loada 0                 ; load ptr
    sys 5                   ; FREE
    ; free_count++
    loadg _h_fc
    push 1
    add
    storeg _h_fc
    ; find and remove ptr from tracking table
    push 0
    storel 0                ; i = 0
dp_loop:
    loadl 0
    push 16
    ge
    jnz dp_done             ; not found, done
    ; load _h_pt[i]
    addrg _h_pt
    loadl 0
    push 3
    mul
    add
    load                    ; _h_pt[i]
    loada 0                 ; ptr
    ne
    jnz dp_next             ; not this one
    ; clear the slot
    push 0
    addrg _h_pt
    loadl 0
    push 3
    mul
    add
    store                   ; _h_pt[i] = 0
    jmp dp_done
dp_next:
    loadl 0
    push 1
    add
    storel 0                ; i++
    jmp dp_loop
dp_done:
    ret 1
.end

; _p24p_leak_report ( -- )
; Report unfreed allocations. Prints "LEAK:N" or "OK:0".
.proc _p24p_leak_report 1
    loadg _h_ac
    loadg _h_fc
    sub
    storel 0                ; leaks = alloc_count - free_count
    loadl 0
    push 0
    gt
    jz lr_ok
    ; print "LEAK:"
    push 76                 ; 'L'
    sys 1
    push 69                 ; 'E'
    sys 1
    push 65                 ; 'A'
    sys 1
    push 75                 ; 'K'
    sys 1
    push 58                 ; ':'
    sys 1
    loadl 0                 ; push leak count
    jmp lr_print
lr_ok:
    ; print "OK:"
    push 79                 ; 'O'
    sys 1
    push 75                 ; 'K'
    sys 1
    push 58                 ; ':'
    sys 1
    push 0                  ; push 0
lr_print:
    ; print the number using write_int logic inline
    ; (can't call _p24p_write_int from here since it uses ret 1)
    dup
    push 0
    lt
    jz lr_pos
    push 45
    sys 1
    neg
lr_pos:
    storel 0
    push 0                  ; sentinel
lr_ext:
    loadl 0
    push 10
    mod
    push 48
    add                     ; digit char
    loadl 0
    push 10
    div
    storel 0
    loadl 0
    jnz lr_ext
lr_prt:
    dup
    jz lr_end
    sys 1
    jmp lr_prt
lr_end:
    drop                    ; discard sentinel
    push 10                 ; LF
    sys 1
    ret 0
.end

; Phase 2: Write formatting
; Hand-written .spc until p24c supports procedure compilation.
; Pascal source: src/write_fmt.pas

; _p24p_write_char ( c -- )
; Write single character to UART via sys 1 (PUTC).
.proc _p24p_write_char 0
    loada 0              ; load c
    sys 1                ; PUTC
    ret 1
.end
