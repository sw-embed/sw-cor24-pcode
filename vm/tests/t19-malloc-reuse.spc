; t19-malloc-reuse.spc — exercise sys_alloc/sys_free reuse via free list.
; Expected output: OK
;
; Allocates a 600-byte block and immediately frees it 200 times in a row.
; With the bump-only allocator this would consume 200 * (600+6) = 121,200
; bytes and trip TRAP 5 (HEAP_OVERFLOW) well before completion.  With the
; free-list allocator the block is reused every iteration, so total live
; heap stays ~606 B and the loop runs to completion.

.proc main 1
    push 200
    storel 0
loop:
    loadl 0
    jz done
    ; alloc(600); write a byte at ptr to make sure ptr is valid; free
    push 600
    sys 4               ; alloc → ptr
    dup                 ; ptr ptr
    push_s 88           ; 'X'
    swap                ; ptr 'X' ptr
    storeb              ; mem[ptr] = 'X'  ( pops ptr 'X' ptr → leaves ptr )
    sys 5               ; free
    ; counter--
    loadl 0
    push 1
    sub
    storel 0
    jmp loop
done:
    push_s 79           ; 'O'
    sys 1
    push_s 75           ; 'K'
    sys 1
    push_s 10           ; '\n'
    sys 1
    halt
.end
