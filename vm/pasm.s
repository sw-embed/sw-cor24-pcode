; pasm.s — P-Code Assembler for COR24
;
; Two-pass assembler: reads .spc source from UART, assembles to bytecode.
;   Pass 1: collect symbols (labels, procs, consts, globals, data), compute sizes
;   Pass 2: emit bytecode, resolve symbol references
;
; Token types:
;   0 = TOK_EOF     end of input
;   1 = TOK_NL      newline
;   2 = TOK_NUM     integer literal (value in tok_value)
;   3 = TOK_IDENT   identifier/mnemonic/name (string in tok_buf)
;   4 = TOK_DIR     directive without '.' (string in tok_buf)
;   5 = TOK_LABEL   label definition without ':' (string in tok_buf)
;   6 = TOK_COMMA   comma separator
;
; Symbol types:
;   0 = SYM_CONST   named constant (value = literal)
;   1 = SYM_LABEL   code label (value = code offset)
;   2 = SYM_PROC    procedure entry (value = code offset)
;   3 = SYM_GLOBAL  global variable (value = global segment offset)
;   4 = SYM_DATA    data block (value = data segment offset)
;
; Operand types (mnemonic table):
;   0 = NONE    1-byte instruction
;   1 = IMM8    2-byte: opcode + byte
;   2 = IMM24   4-byte: opcode + word
;   3 = D8_A24  5-byte: opcode + byte + word (calln)
;   4 = D8_O8   3-byte: opcode + byte + byte (loadn/storen)
;
; Register allocation:
;   r0 = work/scratch, parameter, return value
;   r1 = return address (jal) or scratch
;   r2 = scratch, function address for jal
;   fp = memory base for indexed loads/stores
;   sp = COR24 hardware stack (EBR)
;
; UART: data at -65280 (0xFF0100), status at -65279 (0xFF0101)
;   TX busy = status bit 7 (sign bit via lb sign-extend)
;   RX ready = status bit 0

; ============================================================
; Entry point
; ============================================================
_start:
    ; Print boot message
    la r0, msg_boot
    la r2, uart_puts
    jal r1, (r2)

    ; Read all UART input into input_buf
    la r2, read_all_input
    jal r1, (r2)

    ; ---- Pass 1: collect symbols, compute sizes ----
    lc r0, 0
    la r2, input_pos
    sw r0, 0(r2)
    ; Prime first character
    la r2, lex_advance
    jal r1, (r2)
    ; Set pass number = 1
    lc r0, 1
    la r2, pass_num
    sb r0, 0(r2)
    ; Reset counters
    lc r0, 0
    la r2, code_addr
    sw r0, 0(r2)
    la r2, global_offset
    sw r0, 0(r2)
    la r2, data_offset
    sw r0, 0(r2)
    la r2, sym_count
    sw r0, 0(r2)
    ; Initialize name_pool_ptr
    la r0, name_pool
    la r2, name_pool_ptr
    sw r0, 0(r2)
    ; Run pass 1
    la r2, parse_program
    jal r1, (r2)

    ; Save code size after pass 1
    la r2, code_addr
    lw r0, 0(r2)
    la r2, code_size
    sw r0, 0(r2)
    ; Save data size
    la r2, data_offset
    lw r0, 0(r2)
    la r2, total_data_size
    sw r0, 0(r2)

    ; Patch data and global symbols with absolute offsets
    la r2, patch_symbols
    jal r1, (r2)

    ; ---- Pass 2: emit bytecode ----
    lc r0, 0
    la r2, input_pos
    sw r0, 0(r2)
    ; Prime first character
    la r2, lex_advance
    jal r1, (r2)
    ; Set pass number = 2
    lc r0, 2
    la r2, pass_num
    sb r0, 0(r2)
    ; Initialize code output pointer
    la r0, code_buf
    la r2, code_ptr
    sw r0, 0(r2)
    ; Initialize data output pointer
    la r0, data_buf
    la r2, data_ptr
    sw r0, 0(r2)
    ; Run pass 2
    la r2, parse_program
    jal r1, (r2)

    ; ---- Dump bytecode for verification ----
    la r2, dump_bytecode
    jal r1, (r2)

    ; Done
    la r0, msg_done
    la r2, uart_puts
    jal r1, (r2)
halt_loop:
    bra halt_loop

; ============================================================
; UART helpers
; ============================================================

; uart_putc — send byte in r0 to UART
; Leaf function. Clobbers: r0, r2. Preserves: r1.
uart_putc:
    push r0
    la r2, -65280
uart_putc_wait:
    lb r0, 1(r2)
    cls r0, z
    brt uart_putc_wait
    pop r0
    sb r0, 0(r2)
    jmp (r1)

; uart_puts — print null-terminated string at address in r0
; Non-leaf. Clobbers: r0, r1, r2.
uart_puts:
    push r1
    mov r1, r0
uart_puts_loop:
    lbu r0, 0(r1)
    ceq r0, z
    brt uart_puts_done
    push r1
    push r0
    la r2, -65280
uart_puts_tx:
    lb r0, 1(r2)
    cls r0, z
    brt uart_puts_tx
    pop r0
    sb r0, 0(r2)
    pop r1
    add r1, 1
    bra uart_puts_loop
uart_puts_done:
    pop r1
    jmp (r1)

; ============================================================
; read_all_input — read UART into input_buf until EOT (0x04)
; ============================================================
; Non-leaf. Clobbers: r0, r1, r2.
read_all_input:
    push r1
    ; Initialize write pointer and length
    la r0, input_buf
    la r2, rai_ptr
    sw r0, 0(r2)
    lc r0, 0
    la r2, input_len
    sw r0, 0(r2)

rai_loop:
    ; Poll UART RX ready
    la r2, -65280
rai_poll:
    lbu r0, 1(r2)
    push r2
    lc r2, 1
    and r0, r2
    pop r2
    ceq r0, z
    brt rai_poll
    ; Read byte
    lbu r0, 0(r2)
    ; Check for EOT (0x04)
    lc r2, 4
    ceq r0, r2
    brf rai_not_eot
    la r0, rai_done
    jmp (r0)
rai_not_eot:
    ; Store byte in buffer
    la r2, rai_ptr
    lw r2, 0(r2)
    sb r0, 0(r2)
    ; Advance pointer
    la r2, rai_ptr
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    ; Increment length
    la r2, input_len
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    bra rai_loop

rai_done:
    ; Null-terminate buffer
    la r2, rai_ptr
    lw r2, 0(r2)
    lc r0, 0
    sb r0, 0(r2)
    pop r1
    jmp (r1)

; ============================================================
; Lexer — character input (buffer-based)
; ============================================================

; lex_advance — read next character from input_buf
; If past end, sets lex_char = 0 (EOF).
; Leaf function. Clobbers: r0, r2. Preserves: r1.
lex_advance:
    la r2, input_pos
    lw r0, 0(r2)
    ; Save position for comparison
    push r0
    la r2, input_len
    lw r2, 0(r2)
    pop r0
    clu r0, r2
    brf lex_adv_eof
    ; Read byte from input_buf[position]
    la r2, input_buf
    add r2, r0
    lbu r0, 0(r2)
    ; Advance position
    push r0
    la r2, input_pos
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    pop r0
    ; Store in lex_char
    la r2, lex_char
    sb r0, 0(r2)
    jmp (r1)
lex_adv_eof:
    lc r0, 0
    la r2, lex_char
    sb r0, 0(r2)
    jmp (r1)

; ============================================================
; Lexer — tokenizer
; ============================================================

; next_token — consume input and produce the next token
; Sets tok_type, tok_buf/tok_len (for string tokens), tok_value (for numbers).
; Non-leaf. Clobbers: r0, r1, r2, fp.
next_token:
    push r1

    ; Skip spaces (32) and tabs (9)
nt_skip_ws:
    la r2, lex_char
    lbu r0, 0(r2)
    lc r2, 32
    ceq r0, r2
    brt nt_is_ws
    lc r2, 9
    ceq r0, r2
    brt nt_is_ws
    bra nt_classify

nt_is_ws:
    la r2, lex_advance
    jal r1, (r2)
    bra nt_skip_ws

    ; ---- Character classification ----
    ; Uses inverted-branch + far-jump pattern to avoid branch range issues.

nt_classify:
    ; r0 = current char (non-space, non-tab)

    ; EOF (char == 0)
    ceq r0, z
    brf nt_not_eof
    la r0, nt_eof
    jmp (r0)
nt_not_eof:

    ; Newline (char == 10)
    lc r2, 10
    ceq r0, r2
    brf nt_not_nl
    la r0, nt_newline
    jmp (r0)
nt_not_nl:

    ; Carriage return (char == 13) — skip silently
    lc r2, 13
    ceq r0, r2
    brf nt_not_cr
    la r0, nt_cr
    jmp (r0)
nt_not_cr:

    ; Comment (char == ';' = 59)
    lc r2, 59
    ceq r0, r2
    brf nt_not_cmt
    la r0, nt_comment
    jmp (r0)
nt_not_cmt:

    ; Comma (char == ',' = 44)
    lc r2, 44
    ceq r0, r2
    brf nt_not_comma
    la r0, nt_comma
    jmp (r0)
nt_not_comma:

    ; Directive (char == '.' = 46)
    lc r2, 46
    ceq r0, r2
    brf nt_not_dir
    la r0, nt_directive
    jmp (r0)
nt_not_dir:

    ; Negative sign (char == '-' = 45) → number
    lc r2, 45
    ceq r0, r2
    brf nt_not_neg
    la r0, nt_number
    jmp (r0)
nt_not_neg:

    ; Digit check: '0'(48) <= r0 <= '9'(57)
    lc r2, 48
    clu r0, r2
    brt nt_check_alpha
    lc r2, 58
    clu r0, r2
    brf nt_check_alpha
    la r0, nt_number
    jmp (r0)

nt_check_alpha:
    ; Underscore (95)
    lc r2, 95
    ceq r0, r2
    brf nt_not_under
    la r0, nt_ident
    jmp (r0)
nt_not_under:

    ; Uppercase: 'A'(65) to 'Z'(90)
    lc r2, 65
    clu r0, r2
    brt nt_not_upper
    lc r2, 91
    clu r0, r2
    brf nt_not_upper
    la r0, nt_ident
    jmp (r0)
nt_not_upper:

    ; Lowercase: 'a'(97) to 'z'(122)
    lc r2, 97
    clu r0, r2
    brt nt_do_skip
    lc r2, 123
    clu r0, r2
    brf nt_do_skip
    la r0, nt_ident
    jmp (r0)

nt_do_skip:
    ; Unknown character — skip and retry
    la r2, lex_advance
    jal r1, (r2)
    la r0, nt_skip_ws
    jmp (r0)

; ---- Token handlers ----

nt_eof:
    la r2, tok_type
    lc r0, 0
    sb r0, 0(r2)
    pop r1
    jmp (r1)

nt_newline:
    ; Advance past \n
    la r2, lex_advance
    jal r1, (r2)
    la r2, tok_type
    lc r0, 1
    sb r0, 0(r2)
    pop r1
    jmp (r1)

nt_cr:
    ; Skip \r, continue scanning
    la r2, lex_advance
    jal r1, (r2)
    la r0, nt_skip_ws
    jmp (r0)

nt_comment:
    ; Skip to end of line or EOF
nt_cmt_loop:
    la r2, lex_advance
    jal r1, (r2)
    la r2, lex_char
    lbu r0, 0(r2)
    ; EOF?
    ceq r0, z
    brf nt_cmt_not_eof
    la r0, nt_eof
    jmp (r0)
nt_cmt_not_eof:
    ; Newline?
    lc r2, 10
    ceq r0, r2
    brf nt_cmt_loop
    ; Found \n — produce newline token
    la r0, nt_newline
    jmp (r0)

nt_comma:
    la r2, lex_advance
    jal r1, (r2)
    la r2, tok_type
    lc r0, 6
    sb r0, 0(r2)
    pop r1
    jmp (r1)

nt_directive:
    ; Advance past '.'
    la r2, lex_advance
    jal r1, (r2)
    ; Read directive name into tok_buf
    la r2, read_name
    jal r1, (r2)
    ; Set tok_type = 4 (TOK_DIR)
    la r2, tok_type
    lc r0, 4
    sb r0, 0(r2)
    pop r1
    jmp (r1)

nt_number:
    ; Parse number (starts with '-' or digit)
    la r2, parse_number
    jal r1, (r2)
    ; tok_type and tok_value set by parse_number
    pop r1
    jmp (r1)

nt_ident:
    ; Read name into tok_buf
    la r2, read_name
    jal r1, (r2)
    ; Check if followed by ':' (label definition)
    la r2, lex_char
    lbu r0, 0(r2)
    lc r2, 58
    ceq r0, r2
    brf nt_ident_done
    ; Label: consume ':'
    la r2, lex_advance
    jal r1, (r2)
    la r2, tok_type
    lc r0, 5
    sb r0, 0(r2)
    pop r1
    jmp (r1)
nt_ident_done:
    la r2, tok_type
    lc r0, 3
    sb r0, 0(r2)
    pop r1
    jmp (r1)

; ============================================================
; read_name — read alphanumeric/underscore chars into tok_buf
; ============================================================
; Precondition: lex_char is first char of name (alpha or underscore)
; Postcondition: tok_buf filled and null-terminated, tok_len set
; Non-leaf. Clobbers: r0, r1, r2, fp.
read_name:
    push r1
    ; Initialize write pointer
    la r0, tok_buf
    la r2, tok_ptr
    sw r0, 0(r2)

rn_loop:
    la r2, lex_char
    lbu r0, 0(r2)

    ; Check underscore (95)
    lc r2, 95
    ceq r0, r2
    brt rn_store

    ; Check digit: 48 <= r0 <= 57
    lc r2, 48
    clu r0, r2
    brt rn_not_digit
    lc r2, 58
    clu r0, r2
    brt rn_store
rn_not_digit:

    ; Check uppercase: 65 <= r0 <= 90
    lc r2, 65
    clu r0, r2
    brt rn_done
    lc r2, 91
    clu r0, r2
    brt rn_store

    ; Check lowercase: 97 <= r0 <= 122
    lc r2, 97
    clu r0, r2
    brt rn_done
    lc r2, 123
    clu r0, r2
    brf rn_done
    ; Fall through: is lowercase letter

rn_store:
    ; Store char at tok_ptr, advance ptr
    la r2, tok_ptr
    push r2
    pop fp
    lw r2, 0(fp)
    sb r0, 0(r2)
    add r2, 1
    sw r2, 0(fp)
    ; Advance lexer
    la r2, lex_advance
    jal r1, (r2)
    la r0, rn_loop
    jmp (r0)

rn_done:
    ; Null-terminate tok_buf
    la r2, tok_ptr
    lw r2, 0(r2)
    lc r0, 0
    sb r0, 0(r2)
    ; Calculate length: end - tok_buf
    la r0, tok_buf
    sub r2, r0
    ; Store tok_len
    la r0, tok_len
    sb r2, 0(r0)
    pop r1
    jmp (r1)

; ============================================================
; parse_number — parse decimal integer from UART input
; ============================================================
; Precondition: lex_char is first char ('-' or digit)
; Postcondition: tok_type = 2 (TOK_NUM), tok_value = parsed value
; Non-leaf. Clobbers: r0, r1, r2, fp.
parse_number:
    push r1
    ; Check for negative sign
    la r2, lex_char
    lbu r0, 0(r2)
    lc r2, 45
    ceq r0, r2
    brf pn_positive
    ; Negative: set flag, advance past '-'
    lc r0, 1
    la r2, num_neg
    sb r0, 0(r2)
    la r2, lex_advance
    jal r1, (r2)
    bra pn_start
pn_positive:
    lc r0, 0
    la r2, num_neg
    sb r0, 0(r2)
pn_start:
    ; Initialize accumulator to 0
    lc r0, 0
    la r2, tok_value
    sw r0, 0(r2)

pn_loop:
    ; Get current char
    la r2, lex_char
    lbu r0, 0(r2)
    ; Check if digit: 48 <= r0 <= 57
    lc r2, 48
    clu r0, r2
    brt pn_loop_done
    lc r2, 58
    clu r0, r2
    brf pn_loop_done
    ; Convert digit char to value
    add r0, -48
    ; acc = acc * 10 + digit
    push r0
    la r2, tok_value
    lw r0, 0(r2)
    lc r2, 10
    mul r0, r2
    pop r2
    add r0, r2
    la r2, tok_value
    sw r0, 0(r2)
    ; Advance lexer
    la r2, lex_advance
    jal r1, (r2)
    bra pn_loop

pn_loop_done:
    ; Check negative flag
    la r2, num_neg
    lbu r0, 0(r2)
    ceq r0, z
    brt pn_set_type
    ; Negate tok_value
    la r0, tok_value
    push r0
    pop fp
    lw r0, 0(fp)
    lc r2, 0
    sub r2, r0
    sw r2, 0(fp)

pn_set_type:
    la r2, tok_type
    lc r0, 2
    sb r0, 0(r2)
    pop r1
    jmp (r1)

; ============================================================
; Parser — main program loop
; ============================================================

; parse_program — parse all tokens, dispatching by type
; Shared by pass 1 and pass 2 (behavior differs based on pass_num).
; Non-leaf. Clobbers: r0, r1, r2, fp.
parse_program:
    push r1

pp_loop:
    ; Get next token
    la r2, next_token
    jal r1, (r2)

    ; Load token type
    la r2, tok_type
    lbu r0, 0(r2)

    ; EOF → done
    ceq r0, z
    brf pp_not_eof
    pop r1
    jmp (r1)
pp_not_eof:

    ; NL → skip
    lc r2, 1
    ceq r0, r2
    brf pp_not_nl
    la r0, pp_loop
    jmp (r0)
pp_not_nl:

    ; COMMA → skip
    lc r2, 6
    ceq r0, r2
    brf pp_not_comma
    la r0, pp_loop
    jmp (r0)
pp_not_comma:

    ; DIR (4) → handle directive
    lc r2, 4
    ceq r0, r2
    brf pp_not_dir
    la r2, handle_dir
    jal r1, (r2)
    la r0, pp_loop
    jmp (r0)
pp_not_dir:

    ; LABEL (5) → handle label
    lc r2, 5
    ceq r0, r2
    brf pp_not_label
    la r2, handle_label
    jal r1, (r2)
    la r0, pp_loop
    jmp (r0)
pp_not_label:

    ; IDENT (3) → handle instruction
    lc r2, 3
    ceq r0, r2
    brf pp_skip
    la r2, handle_instr
    jal r1, (r2)
    la r0, pp_loop
    jmp (r0)
pp_skip:

    ; Unexpected token — skip
    la r0, pp_loop
    jmp (r0)

; ============================================================
; Parser — directive handler
; ============================================================

; handle_dir — dispatch by directive name in tok_buf
; Non-leaf. Clobbers: r0, r1, r2.
handle_dir:
    push r1

    ; Check "const"
    la r0, tok_buf
    la r2, str_eq_a
    sw r0, 0(r2)
    la r0, dir_const_str
    la r2, str_eq_b
    sw r0, 0(r2)
    la r2, str_eq
    jal r1, (r2)
    ceq r0, z
    brt hd_not_const
    la r2, dir_const
    jal r1, (r2)
    pop r1
    jmp (r1)
hd_not_const:

    ; Check "global"
    la r0, tok_buf
    la r2, str_eq_a
    sw r0, 0(r2)
    la r0, dir_global_str
    la r2, str_eq_b
    sw r0, 0(r2)
    la r2, str_eq
    jal r1, (r2)
    ceq r0, z
    brt hd_not_global
    la r2, dir_global
    jal r1, (r2)
    pop r1
    jmp (r1)
hd_not_global:

    ; Check "data"
    la r0, tok_buf
    la r2, str_eq_a
    sw r0, 0(r2)
    la r0, dir_data_str
    la r2, str_eq_b
    sw r0, 0(r2)
    la r2, str_eq
    jal r1, (r2)
    ceq r0, z
    brt hd_not_data
    la r2, dir_data
    jal r1, (r2)
    pop r1
    jmp (r1)
hd_not_data:

    ; Check "proc"
    la r0, tok_buf
    la r2, str_eq_a
    sw r0, 0(r2)
    la r0, dir_proc_str
    la r2, str_eq_b
    sw r0, 0(r2)
    la r2, str_eq
    jal r1, (r2)
    ceq r0, z
    brt hd_not_proc
    la r2, dir_proc
    jal r1, (r2)
    pop r1
    jmp (r1)
hd_not_proc:

    ; Check "end" — emit leave in pass 2, advance code_addr in pass 1
    la r0, tok_buf
    la r2, str_eq_a
    sw r0, 0(r2)
    la r0, dir_end_str
    la r2, str_eq_b
    sw r0, 0(r2)
    la r2, str_eq
    jal r1, (r2)
    ceq r0, z
    brt hd_unknown
    ; Check pass number
    la r2, pass_num
    lbu r0, 0(r2)
    lc r2, 1
    ceq r0, r2
    brf hd_end_p2
    ; Pass 1: advance code_addr by 1 (leave opcode)
    la r2, code_addr
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)
hd_end_p2:
    ; Pass 2: emit leave opcode (0x41 = 65)
    lc r0, 65
    la r2, emit_byte
    jal r1, (r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)
hd_unknown:
    ; Unknown directive — skip
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)

; ============================================================
; Parser — directive implementations
; ============================================================

; dir_const — .const NAME value
; Non-leaf.
dir_const:
    push r1
    ; Read name token
    la r2, next_token
    jal r1, (r2)
    ; Only register in pass 1
    la r2, pass_num
    lbu r0, 0(r2)
    lc r2, 1
    ceq r0, r2
    brf dc_skip
    ; Copy name to name pool
    la r2, sym_name_copy
    jal r1, (r2)
    la r2, sym_add_name
    sw r0, 0(r2)
    ; Read value token
    la r2, next_token
    jal r1, (r2)
    la r2, tok_value
    lw r0, 0(r2)
    la r2, sym_add_val
    sw r0, 0(r2)
    ; Type = 0 (SYM_CONST)
    lc r0, 0
    la r2, sym_add_type
    sb r0, 0(r2)
    ; Add symbol
    la r2, sym_add
    jal r1, (r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)
dc_skip:
    ; Pass 2: skip value token and rest of line
    la r2, next_token
    jal r1, (r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)

; dir_global — .global NAME nwords
; Non-leaf.
dir_global:
    push r1
    ; Read name token
    la r2, next_token
    jal r1, (r2)
    ; Only register in pass 1
    la r2, pass_num
    lbu r0, 0(r2)
    lc r2, 1
    ceq r0, r2
    brf dg_skip
    ; Copy name to name pool
    la r2, sym_name_copy
    jal r1, (r2)
    la r2, sym_add_name
    sw r0, 0(r2)
    ; Value = current global_offset
    la r2, global_offset
    lw r0, 0(r2)
    la r2, sym_add_val
    sw r0, 0(r2)
    ; Type = 3 (SYM_GLOBAL)
    lc r0, 3
    la r2, sym_add_type
    sb r0, 0(r2)
    ; Add symbol
    la r2, sym_add
    jal r1, (r2)
    ; Read nwords
    la r2, next_token
    jal r1, (r2)
    ; Advance global_offset by nwords * 3
    la r2, tok_value
    lw r0, 0(r2)
    lc r2, 3
    mul r0, r2
    push r0
    la r2, global_offset
    lw r0, 0(r2)
    pop r2
    add r0, r2
    la r2, global_offset
    sw r0, 0(r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)
dg_skip:
    la r2, next_token
    jal r1, (r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)

; dir_data — .data NAME byte, byte, ...
; Non-leaf.
dir_data:
    push r1
    ; Read name token
    la r2, next_token
    jal r1, (r2)
    ; Only register in pass 1
    la r2, pass_num
    lbu r0, 0(r2)
    lc r2, 1
    ceq r0, r2
    brf dd_pass2
    ; Pass 1: register name, count bytes
    la r2, sym_name_copy
    jal r1, (r2)
    la r2, sym_add_name
    sw r0, 0(r2)
    la r2, data_offset
    lw r0, 0(r2)
    la r2, sym_add_val
    sw r0, 0(r2)
    lc r0, 4
    la r2, sym_add_type
    sb r0, 0(r2)
    la r2, sym_add
    jal r1, (r2)
    ; Count bytes until NL/EOF
    lc r0, 0
    la r2, dd_count
    sw r0, 0(r2)
dd_p1_loop:
    la r2, next_token
    jal r1, (r2)
    la r2, tok_type
    lbu r0, 0(r2)
    ; NL or EOF → done
    lc r2, 1
    ceq r0, r2
    brf dd_p1_not_nl
    la r0, dd_p1_done
    jmp (r0)
dd_p1_not_nl:
    ceq r0, z
    brf dd_p1_not_eof
    la r0, dd_p1_done
    jmp (r0)
dd_p1_not_eof:
    ; COMMA → skip
    lc r2, 6
    ceq r0, r2
    brt dd_p1_loop
    ; NUM → count
    lc r2, 2
    ceq r0, r2
    brf dd_p1_loop
    la r2, dd_count
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    la r0, dd_p1_loop
    jmp (r0)
dd_p1_done:
    ; Add byte count to data_offset
    la r2, dd_count
    lw r0, 0(r2)
    push r0
    la r2, data_offset
    lw r0, 0(r2)
    pop r2
    add r0, r2
    la r2, data_offset
    sw r0, 0(r2)
    pop r1
    jmp (r1)

dd_pass2:
    ; Pass 2: emit bytes to data_buf
dd_p2_loop:
    la r2, next_token
    jal r1, (r2)
    la r2, tok_type
    lbu r0, 0(r2)
    ; NL or EOF → done
    lc r2, 1
    ceq r0, r2
    brf dd_p2_not_nl
    la r0, dd_p2_done
    jmp (r0)
dd_p2_not_nl:
    ceq r0, z
    brf dd_p2_not_eof
    la r0, dd_p2_done
    jmp (r0)
dd_p2_not_eof:
    ; COMMA → skip
    lc r2, 6
    ceq r0, r2
    brt dd_p2_loop
    ; NUM → emit byte to data_buf
    la r2, tok_value
    lw r0, 0(r2)
    la r2, data_ptr
    lw r2, 0(r2)
    sb r0, 0(r2)
    ; Advance data_ptr
    la r2, data_ptr
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    la r0, dd_p2_loop
    jmp (r0)
dd_p2_done:
    pop r1
    jmp (r1)

; dir_proc — .proc NAME nlocals
; Non-leaf.
dir_proc:
    push r1
    ; Read name token
    la r2, next_token
    jal r1, (r2)
    ; Only register in pass 1
    la r2, pass_num
    lbu r0, 0(r2)
    lc r2, 1
    ceq r0, r2
    brf dp_skip
    ; Copy name to name pool
    la r2, sym_name_copy
    jal r1, (r2)
    la r2, sym_add_name
    sw r0, 0(r2)
    ; Value = current code_addr
    la r2, code_addr
    lw r0, 0(r2)
    la r2, sym_add_val
    sw r0, 0(r2)
    ; Type = 2 (SYM_PROC)
    lc r0, 2
    la r2, sym_add_type
    sb r0, 0(r2)
    la r2, sym_add
    jal r1, (r2)
    ; Read nlocals
    la r2, next_token
    jal r1, (r2)
    ; Advance code_addr by 2 (enter opcode + nlocals byte)
    la r2, code_addr
    lw r0, 0(r2)
    add r0, 2
    sw r0, 0(r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)
dp_skip:
    ; Pass 2: read nlocals, emit enter + nlocals
    la r2, next_token
    jal r1, (r2)
    la r2, resolve_operand
    jal r1, (r2)
    push r0
    ; Emit enter opcode (0x40 = 64)
    lc r0, 64
    la r2, emit_byte
    jal r1, (r2)
    ; Emit nlocals byte
    pop r0
    la r2, emit_byte
    jal r1, (r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)

; ============================================================
; Parser — label handler
; ============================================================

; handle_label — register label in symbol table (pass 1 only)
; Non-leaf.
handle_label:
    push r1
    ; Only in pass 1
    la r2, pass_num
    lbu r0, 0(r2)
    lc r2, 1
    ceq r0, r2
    brf hl_done
    ; Copy name to name pool
    la r2, sym_name_copy
    jal r1, (r2)
    la r2, sym_add_name
    sw r0, 0(r2)
    ; Type = 1 (SYM_LABEL)
    lc r0, 1
    la r2, sym_add_type
    sb r0, 0(r2)
    ; Value = current code_addr
    la r2, code_addr
    lw r0, 0(r2)
    la r2, sym_add_val
    sw r0, 0(r2)
    la r2, sym_add
    jal r1, (r2)
hl_done:
    pop r1
    jmp (r1)

; ============================================================
; Parser — instruction handler
; ============================================================

; handle_instr — look up mnemonic, emit or count instruction
; Non-leaf. Clobbers: r0, r1, r2.
handle_instr:
    push r1
    ; Look up mnemonic
    la r2, mnem_lookup
    jal r1, (r2)
    ; r0 = 1 if found, 0 if not
    ceq r0, z
    brf hi_found
    ; Unknown mnemonic
    la r0, msg_err_mnem
    la r2, uart_puts
    jal r1, (r2)
    la r0, tok_buf
    la r2, uart_puts
    jal r1, (r2)
    lc r0, 10
    la r2, uart_putc
    jal r1, (r2)
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)

hi_found:
    ; cur_opcode and cur_optype are set
    ; Check pass number
    la r2, pass_num
    lbu r0, 0(r2)
    lc r2, 1
    ceq r0, r2
    brf hi_do_pass2
    ; ---- Pass 1: count instruction size ----
    la r0, hi_pass1
    jmp (r0)
hi_do_pass2:
    ; ---- Pass 2: emit opcode byte first ----
    la r2, cur_opcode
    lbu r0, 0(r2)
    la r2, emit_byte
    jal r1, (r2)
    la r0, hi_p2_operand
    jmp (r0)

; ---- Pass 1: add instruction size to code_addr ----
hi_pass1:
    ; code_addr += 1 (opcode byte)
    la r2, code_addr
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    ; Now add operand size based on type
    la r2, cur_optype
    lbu r0, 0(r2)
    ; type 0 (NONE): +0
    ceq r0, z
    brf hi_p1_not_t0
    la r0, hi_p1_done
    jmp (r0)
hi_p1_not_t0:
    ; type 1 (IMM8): +1
    lc r2, 1
    ceq r0, r2
    brf hi_p1_not_t1
    la r2, code_addr
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    la r0, hi_p1_done
    jmp (r0)
hi_p1_not_t1:
    ; type 2 (IMM24): +3
    lc r2, 2
    ceq r0, r2
    brf hi_p1_not_t2
    la r2, code_addr
    lw r0, 0(r2)
    add r0, 3
    sw r0, 0(r2)
    la r0, hi_p1_done
    jmp (r0)
hi_p1_not_t2:
    ; type 3 (D8_A24): +4
    lc r2, 3
    ceq r0, r2
    brf hi_p1_not_t3
    la r2, code_addr
    lw r0, 0(r2)
    add r0, 4
    sw r0, 0(r2)
    la r0, hi_p1_done
    jmp (r0)
hi_p1_not_t3:
    ; type 4 (D8_O8): +2
    la r2, code_addr
    lw r0, 0(r2)
    add r0, 2
    sw r0, 0(r2)

hi_p1_done:
    ; Skip remaining tokens on line
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)

; ---- Pass 2: emit operand(s) ----
hi_p2_operand:
    la r2, cur_optype
    lbu r0, 0(r2)
    ; type 0 (NONE): no operand
    ceq r0, z
    brf hi_p2_not_t0
    la r0, hi_p2_done
    jmp (r0)
hi_p2_not_t0:
    ; type 1 (IMM8): read token, emit byte
    lc r2, 1
    ceq r0, r2
    brf hi_p2_not_t1
    la r2, next_token
    jal r1, (r2)
    la r2, resolve_operand
    jal r1, (r2)
    la r2, emit_byte
    jal r1, (r2)
    la r0, hi_p2_done
    jmp (r0)
hi_p2_not_t1:
    ; type 2 (IMM24): read token, emit word
    lc r2, 2
    ceq r0, r2
    brf hi_p2_not_t2
    la r2, next_token
    jal r1, (r2)
    la r2, resolve_operand
    jal r1, (r2)
    la r2, emit_word
    jal r1, (r2)
    la r0, hi_p2_done
    jmp (r0)
hi_p2_not_t2:
    ; type 3 (D8_A24): read two tokens, emit byte + word
    lc r2, 3
    ceq r0, r2
    brf hi_p2_not_t3
    ; First: depth byte
    la r2, next_token
    jal r1, (r2)
    la r2, resolve_operand
    jal r1, (r2)
    la r2, emit_byte
    jal r1, (r2)
    ; Second: address word
    la r2, next_token
    jal r1, (r2)
    la r2, resolve_operand
    jal r1, (r2)
    la r2, emit_word
    jal r1, (r2)
    la r0, hi_p2_done
    jmp (r0)
hi_p2_not_t3:
    ; type 4 (D8_O8): read two tokens, emit byte + byte
    la r2, next_token
    jal r1, (r2)
    la r2, resolve_operand
    jal r1, (r2)
    la r2, emit_byte
    jal r1, (r2)
    la r2, next_token
    jal r1, (r2)
    la r2, resolve_operand
    jal r1, (r2)
    la r2, emit_byte
    jal r1, (r2)

hi_p2_done:
    la r2, skip_to_eol
    jal r1, (r2)
    pop r1
    jmp (r1)

; ============================================================
; skip_to_eol — consume tokens until NL or EOF
; ============================================================
; Non-leaf. Clobbers: r0, r1, r2.
skip_to_eol:
    push r1
ste_loop:
    la r2, tok_type
    lbu r0, 0(r2)
    ; NL?
    lc r2, 1
    ceq r0, r2
    brt ste_done
    ; EOF?
    ceq r0, z
    brt ste_done
    ; Consume next token
    la r2, next_token
    jal r1, (r2)
    bra ste_loop
ste_done:
    pop r1
    jmp (r1)

; ============================================================
; Symbol table — add entry
; ============================================================

; sym_add — add symbol from sym_add_name/type/val
; Non-leaf. Clobbers: r0, r1, r2.
sym_add:
    push r1
    ; Check for overflow (max 64 entries)
    la r2, sym_count
    lw r0, 0(r2)
    lc r2, 64
    clu r0, r2
    brt sa_ok
    ; Overflow — print error and skip
    la r0, msg_err_sym
    la r2, uart_puts
    jal r1, (r2)
    la r0, msg_sym_full
    la r2, uart_puts
    jal r1, (r2)
    pop r1
    jmp (r1)
sa_ok:
    ; Calculate entry address: sym_table + sym_count * 9
    la r2, sym_count
    lw r0, 0(r2)
    lc r2, 9
    mul r0, r2
    la r2, sym_table
    add r0, r2
    la r2, sa_entry
    sw r0, 0(r2)
    ; Write word 0: name pool offset
    la r2, sym_add_name
    lw r0, 0(r2)
    la r2, sa_entry
    lw r2, 0(r2)
    sw r0, 0(r2)
    ; Write word 1: type
    la r2, sym_add_type
    lbu r0, 0(r2)
    la r2, sa_entry
    lw r2, 0(r2)
    sw r0, 3(r2)
    ; Write word 2: value
    la r2, sym_add_val
    lw r0, 0(r2)
    la r2, sa_entry
    lw r2, 0(r2)
    sw r0, 6(r2)
    ; Increment sym_count
    la r2, sym_count
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    pop r1
    jmp (r1)

; ============================================================
; Symbol table — find by name
; ============================================================

; sym_find — search symbol table for name in tok_buf
; Returns value in r0. Prints error if not found, returns 0.
; Non-leaf. Clobbers: r0, r1, r2.
sym_find:
    push r1
    ; Check if table is empty
    la r2, sym_count
    lw r0, 0(r2)
    ceq r0, z
    brf sf_start
    la r0, sf_not_found
    jmp (r0)
sf_start:
    ; Init search state
    la r2, sym_count
    lw r0, 0(r2)
    la r2, sf_count
    sw r0, 0(r2)
    lc r0, 0
    la r2, sf_index
    sw r0, 0(r2)
    la r0, sym_table
    la r2, sf_ptr
    sw r0, 0(r2)

sf_loop:
    ; Check index < count
    la r2, sf_index
    lw r0, 0(r2)
    push r0
    la r2, sf_count
    lw r2, 0(r2)
    pop r0
    clu r0, r2
    brf sf_not_found

    ; Get name address: name_pool + entry[0]
    la r2, sf_ptr
    lw r2, 0(r2)
    lw r0, 0(r2)
    la r2, name_pool
    add r0, r2

    ; Compare with tok_buf
    la r2, str_eq_a
    sw r0, 0(r2)
    la r0, tok_buf
    la r2, str_eq_b
    sw r0, 0(r2)
    la r2, str_eq
    jal r1, (r2)
    ; r0 = 1 if match
    ceq r0, z
    brt sf_next

    ; Found! Return value (word 2 at offset 6)
    la r2, sf_ptr
    lw r2, 0(r2)
    lw r0, 6(r2)
    pop r1
    jmp (r1)

sf_next:
    ; Advance pointer by 9
    la r2, sf_ptr
    lw r0, 0(r2)
    add r0, 9
    sw r0, 0(r2)
    ; Increment index
    la r2, sf_index
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    la r0, sf_loop
    jmp (r0)

sf_not_found:
    la r0, msg_err_sym
    la r2, uart_puts
    jal r1, (r2)
    la r0, tok_buf
    la r2, uart_puts
    jal r1, (r2)
    lc r0, 10
    la r2, uart_putc
    jal r1, (r2)
    lc r0, 0
    pop r1
    jmp (r1)

; ============================================================
; Symbol table — copy name to name pool
; ============================================================

; sym_name_copy — copy tok_buf into name_pool, return offset in r0
; Non-leaf. Clobbers: r0, r1, r2.
sym_name_copy:
    push r1
    ; Calculate offset: name_pool_ptr - name_pool
    la r2, name_pool_ptr
    lw r0, 0(r2)
    la r2, name_pool
    sub r0, r2
    push r0
    ; Set up copy source
    la r0, tok_buf
    la r2, snc_src
    sw r0, 0(r2)

snc_loop:
    ; Load byte from source
    la r2, snc_src
    lw r2, 0(r2)
    lbu r0, 0(r2)
    push r0
    ; Store to name_pool_ptr
    la r2, name_pool_ptr
    lw r2, 0(r2)
    pop r0
    sb r0, 0(r2)
    ; Check if null
    push r0
    ; Advance dest
    la r2, name_pool_ptr
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    ; Advance source
    la r2, snc_src
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    ; Check saved byte for null
    pop r0
    ceq r0, z
    brf snc_loop

    ; Return offset
    pop r0
    pop r1
    jmp (r1)

; ============================================================
; String comparison
; ============================================================

; str_eq — compare strings at str_eq_a and str_eq_b
; Returns r0 = 1 if equal, 0 if not.
; Non-leaf. Clobbers: r0, r1, r2.
str_eq:
    push r1

seq_loop:
    la r2, str_eq_a
    lw r2, 0(r2)
    lbu r0, 0(r2)
    push r0
    la r2, str_eq_b
    lw r2, 0(r2)
    lbu r0, 0(r2)
    pop r2
    ; r2 = byte from a, r0 = byte from b
    ceq r0, r2
    brf seq_ne
    ; Same byte — check for null
    ceq r0, z
    brt seq_eq
    ; Advance both pointers
    la r2, str_eq_a
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    la r2, str_eq_b
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    bra seq_loop

seq_ne:
    lc r0, 0
    pop r1
    jmp (r1)

seq_eq:
    lc r0, 1
    pop r1
    jmp (r1)

; ============================================================
; Emit helpers — write to code_buf
; ============================================================

; emit_byte — write byte in r0 to code_buf, advance code_ptr
; Leaf function. Clobbers: r0, r2. Preserves: r1.
emit_byte:
    la r2, code_ptr
    lw r2, 0(r2)
    sb r0, 0(r2)
    add r2, 1
    mov r0, r2
    la r2, code_ptr
    sw r0, 0(r2)
    jmp (r1)

; emit_word — write 24-bit word in r0 to code_buf, advance by 3
; Leaf function. Clobbers: r0, r2. Preserves: r1.
emit_word:
    la r2, code_ptr
    lw r2, 0(r2)
    sw r0, 0(r2)
    add r2, 3
    mov r0, r2
    la r2, code_ptr
    sw r0, 0(r2)
    jmp (r1)

; ============================================================
; Mnemonic lookup
; ============================================================

; mnem_lookup — find tok_buf in mnemonic table
; Sets cur_opcode and cur_optype on match.
; Returns r0 = 1 if found, 0 if not.
; Non-leaf. Clobbers: r0, r1, r2.
mnem_lookup:
    push r1
    ; Init pointer to start of table
    la r0, mnem_table
    la r2, mnem_ptr
    sw r0, 0(r2)

ml_loop:
    ; Check for end sentinel (first byte = 0)
    la r2, mnem_ptr
    lw r2, 0(r2)
    lbu r0, 0(r2)
    ceq r0, z
    brf ml_compare
    ; End of table — not found
    lc r0, 0
    pop r1
    jmp (r1)

ml_compare:
    ; Compare tok_buf with current entry string
    la r0, tok_buf
    la r2, str_eq_a
    sw r0, 0(r2)
    la r2, mnem_ptr
    lw r0, 0(r2)
    la r2, str_eq_b
    sw r0, 0(r2)
    la r2, str_eq
    jal r1, (r2)
    ; r0 = 1 if match
    ceq r0, z
    brt ml_skip
    ; Match found!
    la r0, ml_found
    jmp (r0)

ml_skip:
    ; Skip past string null terminator
    la r2, mnem_ptr
    lw r0, 0(r2)
ml_skip_str:
    mov r2, r0
    lbu r2, 0(r2)
    add r0, 1
    ceq r2, z
    brf ml_skip_str
    ; r0 now points past null (at opcode byte)
    ; Skip opcode + type (2 bytes)
    add r0, 2
    la r2, mnem_ptr
    sw r0, 0(r2)
    la r0, ml_loop
    jmp (r0)

ml_found:
    ; Walk to null terminator of matched string
    la r2, mnem_ptr
    lw r0, 0(r2)
ml_find_null:
    mov r2, r0
    lbu r2, 0(r2)
    add r0, 1
    ceq r2, z
    brf ml_find_null
    ; r0 points to opcode byte (right after null)
    mov r2, r0
    lbu r0, 0(r2)
    push r0
    lbu r0, 1(r2)
    la r2, cur_optype
    sb r0, 0(r2)
    pop r0
    la r2, cur_opcode
    sb r0, 0(r2)
    ; Return success
    lc r0, 1
    pop r1
    jmp (r1)

; ============================================================
; Operand resolution
; ============================================================

; resolve_operand — resolve current token to a value
; If NUM, returns tok_value. If IDENT, looks up symbol.
; Returns value in r0.
; Non-leaf. Clobbers: r0, r1, r2.
resolve_operand:
    push r1
    la r2, tok_type
    lbu r0, 0(r2)
    ; NUM?
    lc r2, 2
    ceq r0, r2
    brf ro_not_num
    la r2, tok_value
    lw r0, 0(r2)
    pop r1
    jmp (r1)
ro_not_num:
    ; IDENT?
    lc r2, 3
    ceq r0, r2
    brf ro_default
    la r2, sym_find
    jal r1, (r2)
    pop r1
    jmp (r1)
ro_default:
    lc r0, 0
    pop r1
    jmp (r1)

; ============================================================
; patch_symbols — fix data/global offsets after pass 1
; ============================================================

; patch_symbols — add code_size to SYM_DATA, code_size+data_size to SYM_GLOBAL
; Non-leaf. Clobbers: r0, r1, r2.
patch_symbols:
    push r1
    la r2, sym_count
    lw r0, 0(r2)
    ceq r0, z
    brf ps_start
    pop r1
    jmp (r1)
ps_start:
    la r2, sym_count
    lw r0, 0(r2)
    la r2, ps_count
    sw r0, 0(r2)
    la r0, sym_table
    la r2, ps_ptr
    sw r0, 0(r2)
    lc r0, 0
    la r2, ps_index
    sw r0, 0(r2)

ps_loop:
    ; Check index < count
    la r2, ps_index
    lw r0, 0(r2)
    push r0
    la r2, ps_count
    lw r2, 0(r2)
    pop r0
    clu r0, r2
    brf ps_done

    ; Load type (word 1 at offset 3)
    la r2, ps_ptr
    lw r2, 0(r2)
    lw r0, 3(r2)
    ; Check SYM_DATA (4)
    lc r2, 4
    ceq r0, r2
    brf ps_not_data
    ; Patch: value += code_size
    la r2, ps_ptr
    lw r2, 0(r2)
    lw r0, 6(r2)
    push r2
    la r2, code_size
    lw r2, 0(r2)
    add r0, r2
    pop r2
    sw r0, 6(r2)
    la r0, ps_next
    jmp (r0)
ps_not_data:
    ; Check SYM_GLOBAL (3)
    lc r2, 3
    ceq r0, r2
    brf ps_next
    ; Patch: value += code_size + total_data_size
    la r2, ps_ptr
    lw r2, 0(r2)
    lw r0, 6(r2)
    push r2
    la r2, code_size
    lw r2, 0(r2)
    add r0, r2
    la r2, total_data_size
    lw r2, 0(r2)
    add r0, r2
    pop r2
    sw r0, 6(r2)

ps_next:
    ; Advance
    la r2, ps_ptr
    lw r0, 0(r2)
    add r0, 9
    sw r0, 0(r2)
    la r2, ps_index
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    la r0, ps_loop
    jmp (r0)

ps_done:
    pop r1
    jmp (r1)

; ============================================================
; dump_bytecode — print assembled bytes to UART
; ============================================================
; Non-leaf. Clobbers: r0, r1, r2.
dump_bytecode:
    push r1
    ; Print header
    la r0, msg_code
    la r2, uart_puts
    jal r1, (r2)

    ; Calculate byte count: code_ptr - code_buf
    la r2, code_ptr
    lw r0, 0(r2)
    la r2, code_buf
    sub r0, r2
    ceq r0, z
    brf db_has_bytes
    ; Empty — just newline
    lc r0, 10
    la r2, uart_putc
    jal r1, (r2)
    pop r1
    jmp (r1)

db_has_bytes:
    la r2, db_count
    sw r0, 0(r2)
    la r0, code_buf
    la r2, db_ptr
    sw r0, 0(r2)

db_loop:
    la r2, db_count
    lw r0, 0(r2)
    ceq r0, z
    brf db_print
    ; Done — newline
    lc r0, 10
    la r2, uart_putc
    jal r1, (r2)
    pop r1
    jmp (r1)

db_print:
    ; Load byte
    la r2, db_ptr
    lw r2, 0(r2)
    lbu r0, 0(r2)
    ; Print as decimal
    la r2, print_num
    jal r1, (r2)
    ; Print space
    lc r0, 32
    la r2, uart_putc
    jal r1, (r2)
    ; Advance pointer
    la r2, db_ptr
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    ; Decrement count
    la r2, db_count
    lw r0, 0(r2)
    add r0, -1
    sw r0, 0(r2)
    la r0, db_loop
    jmp (r0)

; ============================================================
; print_token — print current token to UART (debug output)
; ============================================================
; Non-leaf. Clobbers: r0, r1, r2, fp.
print_token:
    push r1
    la r2, tok_type
    lbu r0, 0(r2)

    ; Type 0: EOF
    ceq r0, z
    brf pt_not_eof
    la r0, pt_eof
    jmp (r0)
pt_not_eof:
    ; Type 1: NL
    lc r2, 1
    ceq r0, r2
    brf pt_not_nl
    la r0, pt_nl
    jmp (r0)
pt_not_nl:
    ; Type 2: NUM
    lc r2, 2
    ceq r0, r2
    brf pt_not_num
    la r0, pt_num
    jmp (r0)
pt_not_num:
    ; Type 3: IDENT
    lc r2, 3
    ceq r0, r2
    brf pt_not_id
    la r0, pt_ident
    jmp (r0)
pt_not_id:
    ; Type 4: DIR
    lc r2, 4
    ceq r0, r2
    brf pt_not_dir
    la r0, pt_dir
    jmp (r0)
pt_not_dir:
    ; Type 5: LABEL
    lc r2, 5
    ceq r0, r2
    brf pt_not_lbl
    la r0, pt_label
    jmp (r0)
pt_not_lbl:
    ; Type 6: COMMA (default)
    la r0, pt_comma
    jmp (r0)

pt_eof:
    la r0, msg_eof
    la r2, uart_puts
    jal r1, (r2)
    pop r1
    jmp (r1)

pt_nl:
    la r0, msg_nl
    la r2, uart_puts
    jal r1, (r2)
    pop r1
    jmp (r1)

pt_num:
    la r0, msg_num
    la r2, uart_puts
    jal r1, (r2)
    ; Print number value
    la r2, tok_value
    lw r0, 0(r2)
    la r2, print_num
    jal r1, (r2)
    ; Print newline
    lc r0, 10
    la r2, uart_putc
    jal r1, (r2)
    pop r1
    jmp (r1)

pt_ident:
    la r0, msg_id
    la r2, uart_puts
    jal r1, (r2)
    la r0, tok_buf
    la r2, uart_puts
    jal r1, (r2)
    lc r0, 10
    la r2, uart_putc
    jal r1, (r2)
    pop r1
    jmp (r1)

pt_dir:
    la r0, msg_dir
    la r2, uart_puts
    jal r1, (r2)
    la r0, tok_buf
    la r2, uart_puts
    jal r1, (r2)
    lc r0, 10
    la r2, uart_putc
    jal r1, (r2)
    pop r1
    jmp (r1)

pt_label:
    la r0, msg_lbl
    la r2, uart_puts
    jal r1, (r2)
    la r0, tok_buf
    la r2, uart_puts
    jal r1, (r2)
    lc r0, 10
    la r2, uart_putc
    jal r1, (r2)
    pop r1
    jmp (r1)

pt_comma:
    la r0, msg_com
    la r2, uart_puts
    jal r1, (r2)
    pop r1
    jmp (r1)

; ============================================================
; print_num — print signed 24-bit integer to UART
; ============================================================
; r0 = value to print
; Non-leaf. Clobbers: r0, r1, r2, fp.
print_num:
    push r1
    ; Handle zero
    ceq r0, z
    brf pnum_not_zero
    la r0, pnum_zero
    jmp (r0)
pnum_not_zero:
    ; Handle negative
    cls r0, z
    brf pnum_positive
    ; Print '-'
    push r0
    lc r0, 45
    la r2, uart_putc
    jal r1, (r2)
    pop r0
    ; Negate: r0 = 0 - r0
    lc r2, 0
    sub r2, r0
    mov r0, r2

pnum_positive:
    ; Store value in num_val
    la r2, num_val
    sw r0, 0(r2)
    ; Push sentinel (0) onto stack
    lc r0, 0
    push r0

    ; Extract digits: divide by 10, push remainders
pnum_extract:
    la r2, num_val
    lw r0, 0(r2)
    ceq r0, z
    brt pnum_output
    ; div10: divides num_val by 10, returns remainder in r0
    la r2, div10
    jal r1, (r2)
    ; r0 = remainder, convert to ASCII
    add r0, 48
    push r0
    bra pnum_extract

    ; Pop and print digits until sentinel (0)
pnum_output:
    pop r0
    ceq r0, z
    brt pnum_ret
    la r2, uart_putc
    jal r1, (r2)
    bra pnum_output

pnum_ret:
    pop r1
    jmp (r1)

pnum_zero:
    lc r0, 48
    la r2, uart_putc
    jal r1, (r2)
    pop r1
    jmp (r1)

; ============================================================
; div10 — divide num_val by 10 using repeated subtraction
; ============================================================
; Updates num_val with quotient, returns remainder (0-9) in r0.
; Leaf function. Clobbers: r0, r2. Preserves: r1.
div10:
    ; Load dividend and init quotient
    la r2, num_val
    lw r0, 0(r2)
    push r0
    la r2, num_div
    lc r0, 0
    sw r0, 0(r2)
    pop r0
    ; Repeated subtraction
d10_loop:
    lc r2, 10
    clu r0, r2
    brt d10_done
    add r0, -10
    ; Increment quotient
    push r0
    la r2, num_div
    lw r0, 0(r2)
    add r0, 1
    sw r0, 0(r2)
    pop r0
    bra d10_loop
d10_done:
    ; r0 = remainder; store quotient to num_val
    push r0
    la r2, num_div
    lw r0, 0(r2)
    la r2, num_val
    sw r0, 0(r2)
    pop r0
    jmp (r1)

; ============================================================
; Mnemonic table
; ============================================================
; Format: null-terminated string, opcode byte, operand type byte
; End sentinel: single 0 byte
;
; Operand types: 0=NONE, 1=IMM8, 2=IMM24, 3=D8_A24, 4=D8_O8

mnem_table:
    ; "halt" opcode=0 type=NONE
    .byte 104, 97, 108, 116, 0, 0, 0
    ; "push" opcode=1 type=IMM24
    .byte 112, 117, 115, 104, 0, 1, 2
    ; "push_s" opcode=2 type=IMM8
    .byte 112, 117, 115, 104, 95, 115, 0, 2, 1
    ; "dup" opcode=3 type=NONE
    .byte 100, 117, 112, 0, 3, 0
    ; "drop" opcode=4 type=NONE
    .byte 100, 114, 111, 112, 0, 4, 0
    ; "swap" opcode=5 type=NONE
    .byte 115, 119, 97, 112, 0, 5, 0
    ; "over" opcode=6 type=NONE
    .byte 111, 118, 101, 114, 0, 6, 0
    ; "add" opcode=16 type=NONE
    .byte 97, 100, 100, 0, 16, 0
    ; "sub" opcode=17 type=NONE
    .byte 115, 117, 98, 0, 17, 0
    ; "mul" opcode=18 type=NONE
    .byte 109, 117, 108, 0, 18, 0
    ; "div" opcode=19 type=NONE
    .byte 100, 105, 118, 0, 19, 0
    ; "mod" opcode=20 type=NONE
    .byte 109, 111, 100, 0, 20, 0
    ; "neg" opcode=21 type=NONE
    .byte 110, 101, 103, 0, 21, 0
    ; "and" opcode=22 type=NONE
    .byte 97, 110, 100, 0, 22, 0
    ; "or" opcode=23 type=NONE
    .byte 111, 114, 0, 23, 0
    ; "xor" opcode=24 type=NONE
    .byte 120, 111, 114, 0, 24, 0
    ; "not" opcode=25 type=NONE
    .byte 110, 111, 116, 0, 25, 0
    ; "shl" opcode=26 type=NONE
    .byte 115, 104, 108, 0, 26, 0
    ; "shr" opcode=27 type=NONE
    .byte 115, 104, 114, 0, 27, 0
    ; "eq" opcode=32 type=NONE
    .byte 101, 113, 0, 32, 0
    ; "ne" opcode=33 type=NONE
    .byte 110, 101, 0, 33, 0
    ; "lt" opcode=34 type=NONE
    .byte 108, 116, 0, 34, 0
    ; "le" opcode=35 type=NONE
    .byte 108, 101, 0, 35, 0
    ; "gt" opcode=36 type=NONE
    .byte 103, 116, 0, 36, 0
    ; "ge" opcode=37 type=NONE
    .byte 103, 101, 0, 37, 0
    ; "jmp" opcode=48 type=IMM24
    .byte 106, 109, 112, 0, 48, 2
    ; "jz" opcode=49 type=IMM24
    .byte 106, 122, 0, 49, 2
    ; "jnz" opcode=50 type=IMM24
    .byte 106, 110, 122, 0, 50, 2
    ; "call" opcode=51 type=IMM24
    .byte 99, 97, 108, 108, 0, 51, 2
    ; "ret" opcode=52 type=IMM8
    .byte 114, 101, 116, 0, 52, 1
    ; "calln" opcode=53 type=D8_A24
    .byte 99, 97, 108, 108, 110, 0, 53, 3
    ; "trap" opcode=54 type=IMM8
    .byte 116, 114, 97, 112, 0, 54, 1
    ; "enter" opcode=64 type=IMM8
    .byte 101, 110, 116, 101, 114, 0, 64, 1
    ; "leave" opcode=65 type=NONE
    .byte 108, 101, 97, 118, 101, 0, 65, 0
    ; "loadl" opcode=66 type=IMM8
    .byte 108, 111, 97, 100, 108, 0, 66, 1
    ; "storel" opcode=67 type=IMM8
    .byte 115, 116, 111, 114, 101, 108, 0, 67, 1
    ; "loadg" opcode=68 type=IMM24
    .byte 108, 111, 97, 100, 103, 0, 68, 2
    ; "storeg" opcode=69 type=IMM24
    .byte 115, 116, 111, 114, 101, 103, 0, 69, 2
    ; "addrl" opcode=70 type=IMM8
    .byte 97, 100, 100, 114, 108, 0, 70, 1
    ; "addrg" opcode=71 type=IMM24
    .byte 97, 100, 100, 114, 103, 0, 71, 2
    ; "loada" opcode=72 type=IMM8
    .byte 108, 111, 97, 100, 97, 0, 72, 1
    ; "storea" opcode=73 type=IMM8
    .byte 115, 116, 111, 114, 101, 97, 0, 73, 1
    ; "loadn" opcode=74 type=D8_O8
    .byte 108, 111, 97, 100, 110, 0, 74, 4
    ; "storen" opcode=75 type=D8_O8
    .byte 115, 116, 111, 114, 101, 110, 0, 75, 4
    ; "load" opcode=80 type=NONE
    .byte 108, 111, 97, 100, 0, 80, 0
    ; "store" opcode=81 type=NONE
    .byte 115, 116, 111, 114, 101, 0, 81, 0
    ; "loadb" opcode=82 type=NONE
    .byte 108, 111, 97, 100, 98, 0, 82, 0
    ; "storeb" opcode=83 type=NONE
    .byte 115, 116, 111, 114, 101, 98, 0, 83, 0
    ; "sys" opcode=96 type=IMM8
    .byte 115, 121, 115, 0, 96, 1
    ; End sentinel
    .byte 0

; ============================================================
; Directive name strings
; ============================================================
dir_const_str:
    .byte 99, 111, 110, 115, 116, 0
    ; "const\0"
dir_global_str:
    .byte 103, 108, 111, 98, 97, 108, 0
    ; "global\0"
dir_data_str:
    .byte 100, 97, 116, 97, 0
    ; "data\0"
dir_proc_str:
    .byte 112, 114, 111, 99, 0
    ; "proc\0"
dir_end_str:
    .byte 101, 110, 100, 0
    ; "end\0"

; ============================================================
; String constants
; ============================================================
msg_boot:
    .byte 80, 65, 83, 77, 10, 0
    ; "PASM\n\0"

msg_done:
    .byte 68, 79, 78, 69, 10, 0
    ; "DONE\n\0"

msg_code:
    .byte 67, 79, 68, 69, 32, 0
    ; "CODE \0"

msg_err_mnem:
    .byte 69, 82, 82, 58, 32, 0
    ; "ERR: \0"

msg_err_sym:
    .byte 83, 89, 77, 63, 32, 0
    ; "SYM? \0"

msg_sym_full:
    .byte 70, 85, 76, 76, 10, 0
    ; "FULL\n\0"

msg_eof:
    .byte 69, 79, 70, 10, 0
    ; "EOF\n\0"

msg_nl:
    .byte 78, 76, 10, 0
    ; "NL\n\0"

msg_num:
    .byte 78, 85, 77, 32, 0
    ; "NUM \0"

msg_id:
    .byte 73, 68, 32, 0
    ; "ID \0"

msg_dir:
    .byte 68, 73, 82, 32, 0
    ; "DIR \0"

msg_lbl:
    .byte 76, 66, 76, 32, 0
    ; "LBL \0"

msg_com:
    .byte 67, 79, 77, 10, 0
    ; "COM\n\0"

; ============================================================
; Lexer state
; ============================================================
lex_char:
    .byte 0

; ============================================================
; Token output
; ============================================================
tok_type:
    .byte 0

tok_len:
    .byte 0

tok_value:
    .word 0

tok_buf:
    .byte 0, 0, 0, 0, 0, 0, 0, 0
    .byte 0, 0, 0, 0, 0, 0, 0, 0
    .byte 0, 0, 0, 0, 0, 0, 0, 0
    .byte 0, 0, 0, 0, 0, 0, 0, 0
    ; 32 bytes for token string

; ============================================================
; Temporary variables
; ============================================================
tok_ptr:
    .word 0

num_val:
    .word 0

num_div:
    .word 0

num_neg:
    .byte 0

; ============================================================
; Parser state
; ============================================================
pass_num:
    .byte 0

code_addr:
    .word 0

code_size:
    .word 0

global_offset:
    .word 0

data_offset:
    .word 0

total_data_size:
    .word 0

cur_opcode:
    .byte 0

cur_optype:
    .byte 0

dd_count:
    .word 0

; ============================================================
; Symbol table parameters (for sym_add)
; ============================================================
sym_add_name:
    .word 0

sym_add_type:
    .byte 0

sym_add_val:
    .word 0

; Temp for sym_add
sa_entry:
    .word 0

; Temps for sym_find
sf_count:
    .word 0

sf_index:
    .word 0

sf_ptr:
    .word 0

; Temps for sym_name_copy
snc_src:
    .word 0

; Temps for str_eq
str_eq_a:
    .word 0

str_eq_b:
    .word 0

; Temps for mnem_lookup
mnem_ptr:
    .word 0

; Temps for read_all_input
rai_ptr:
    .word 0

; Temps for dump_bytecode
db_count:
    .word 0

db_ptr:
    .word 0

; Temps for patch_symbols
ps_count:
    .word 0

ps_index:
    .word 0

ps_ptr:
    .word 0

; ============================================================
; Symbol table (max 64 entries, 9 bytes each = 576 bytes)
; ============================================================
sym_count:
    .word 0

sym_table:
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    ; 576 bytes

; ============================================================
; Name pool (1024 bytes for symbol name strings)
; ============================================================
name_pool_ptr:
    .word 0

name_pool:
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    ; 1024 bytes

; ============================================================
; Input buffer (2048 bytes for source input)
; ============================================================
input_len:
    .word 0

input_pos:
    .word 0

input_buf:
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    ; 2048 bytes

; ============================================================
; Code output buffer (1024 bytes)
; ============================================================
code_ptr:
    .word 0

code_buf:
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    ; 1024 bytes

; ============================================================
; Data output buffer (512 bytes)
; ============================================================
data_ptr:
    .word 0

data_buf:
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    ; 512 bytes
