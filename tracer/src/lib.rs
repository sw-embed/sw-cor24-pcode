// Core VM for the P-code Trace Interpreter (pv24t) and the Debugger (pv24d).
//
// Semantics match pvm.s exactly: dual-stack architecture, 24-bit word size,
// signed arithmetic, same frame layout, same trap behavior.

use pa24r::Opcode;
use std::io::{self, Read, Write};

// 24-bit mask and sign extension
pub const MASK24: i32 = 0x00FF_FFFF;

pub fn sign_extend_24(v: i32) -> i32 {
    let v = v & MASK24;
    if v & 0x0080_0000 != 0 {
        v | !MASK24 // sign-extend negative
    } else {
        v
    }
}

pub fn wrap24(v: i32) -> i32 {
    sign_extend_24(v)
}

// VM state
pub struct Vm {
    // Memory: code, data, globals, call stack, eval stack, heap — all in one flat array
    pub mem: Vec<u8>,

    // Registers
    pub pc: usize,
    pub esp: usize,   // eval stack pointer (points to next free slot)
    pub csp: usize,   // call stack pointer (points to next free slot)
    pub fp_vm: usize, // frame pointer
    pub gp: usize,    // globals base
    pub hp: usize,    // heap pointer

    // Layout boundaries
    pub code_size: usize,
    pub data_size: usize,
    pub global_count: usize,
    pub globals_base: usize,
    pub call_stack_base: usize,
    pub eval_stack_base: usize,
    pub heap_base: usize,

    // Status
    pub status: u8, // 0=running, 1=halted, 2=trapped
    pub trap_code: u8,

    // I/O
    pub stdin_buf: Vec<u8>,
    pub stdin_pos: usize,

    // Trace
    pub trace: bool,
    pub instruction_count: u64,
    pub max_instructions: u64,
}

pub const CALL_STACK_WORDS: usize = 256;
pub const EVAL_STACK_WORDS: usize = 256;
pub const HEAP_WORDS: usize = 256;
pub const WORD: usize = 3;

impl Vm {
    pub fn new(
        image: pa24r::LoadedImage,
        stdin_data: Vec<u8>,
        trace: bool,
        max_instructions: u64,
    ) -> Self {
        let code_size = image.code.len();
        let data_size = image.data.len();
        let global_count = image.global_count as usize;

        let globals_base = code_size + data_size;
        let globals_size = global_count * WORD;
        let call_stack_base = globals_base + globals_size;
        let call_stack_size = CALL_STACK_WORDS * WORD;
        let eval_stack_base = call_stack_base + call_stack_size;
        let eval_stack_size = EVAL_STACK_WORDS * WORD;
        let heap_base = eval_stack_base + eval_stack_size;
        let heap_size = HEAP_WORDS * WORD;

        let total = heap_base + heap_size;
        let mut mem = vec![0u8; total];

        // Load code
        mem[..code_size].copy_from_slice(&image.code);
        // Load data
        mem[code_size..code_size + data_size].copy_from_slice(&image.data);

        Vm {
            mem,
            pc: image.entry_point as usize,
            esp: eval_stack_base,
            csp: call_stack_base,
            fp_vm: call_stack_base,
            gp: globals_base,
            hp: heap_base,
            code_size,
            data_size,
            global_count,
            globals_base,
            call_stack_base,
            eval_stack_base,
            heap_base,
            status: 0,
            trap_code: 0,
            stdin_buf: stdin_data,
            stdin_pos: 0,
            trace,
            instruction_count: 0,
            max_instructions,
        }
    }

    // Memory access: 24-bit words stored little-endian in 3 bytes
    pub fn read_word(&self, addr: usize) -> i32 {
        if addr + 2 >= self.mem.len() {
            return 0;
        }
        let lo = self.mem[addr] as i32;
        let mid = self.mem[addr + 1] as i32;
        let hi = self.mem[addr + 2] as i32;
        sign_extend_24(lo | (mid << 8) | (hi << 16))
    }

    pub fn write_word(&mut self, addr: usize, val: i32) {
        let v = val & MASK24;
        if addr + 2 >= self.mem.len() {
            self.trap(2); // stack overflow
            return;
        }
        self.mem[addr] = v as u8;
        self.mem[addr + 1] = (v >> 8) as u8;
        self.mem[addr + 2] = (v >> 16) as u8;
    }

    pub fn read_byte(&self, addr: usize) -> i32 {
        if addr >= self.mem.len() {
            0
        } else {
            self.mem[addr] as i32
        }
    }

    pub fn write_byte(&mut self, addr: usize, val: i32) {
        if addr >= self.mem.len() {
            self.trap(2);
            return;
        }
        self.mem[addr] = val as u8;
    }

    // Fetch operand bytes from code at pc
    pub fn fetch_u8(&mut self) -> u8 {
        let v = if self.pc < self.code_size {
            self.mem[self.pc]
        } else {
            0
        };
        self.pc += 1;
        v
    }

    pub fn fetch_i8(&mut self) -> i32 {
        let v = self.fetch_u8() as i8;
        v as i32
    }

    pub fn fetch_u24(&mut self) -> u32 {
        let lo = self.fetch_u8() as u32;
        let mid = self.fetch_u8() as u32;
        let hi = self.fetch_u8() as u32;
        lo | (mid << 8) | (hi << 16)
    }

    // Eval stack operations
    pub fn push_eval(&mut self, val: i32) {
        if self.esp >= self.heap_base {
            self.trap(2); // stack overflow
            return;
        }
        self.write_word(self.esp, val);
        self.esp += WORD;
    }

    pub fn pop_eval(&mut self) -> i32 {
        if self.esp <= self.eval_stack_base {
            self.trap(3); // stack underflow
            return 0;
        }
        self.esp -= WORD;
        self.read_word(self.esp)
    }

    pub fn peek_eval(&self) -> i32 {
        if self.esp <= self.eval_stack_base {
            0
        } else {
            self.read_word(self.esp - WORD)
        }
    }

    pub fn eval_depth(&self) -> usize {
        (self.esp - self.eval_stack_base) / WORD
    }

    pub fn trap(&mut self, code: u8) {
        self.status = 2;
        self.trap_code = code;
    }

    pub fn run(&mut self) {
        while self.status == 0 {
            if self.max_instructions > 0 && self.instruction_count >= self.max_instructions {
                eprintln!(
                    "pv24t: instruction limit reached ({} instructions)",
                    self.max_instructions
                );
                self.status = 1;
                break;
            }
            self.step_one();
        }

        // Print trap message (matches pvm.s behavior)
        if self.status == 2 {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            let _ = writeln!(out, "TRAP {}", self.trap_code);
        }
    }

    /// Execute exactly one instruction. Increments instruction_count.
    /// Used by the debugger for step-by-step execution.
    pub fn step_one(&mut self) {
        if self.status != 0 {
            return;
        }
        let instr_pc = self.pc;
        let op_byte = self.fetch_u8();

        let op = match decode_opcode(op_byte) {
            Some(op) => op,
            None => {
                self.trap(4); // invalid opcode
                return;
            }
        };

        if self.trace {
            self.trace_instruction(instr_pc, op);
        }

        self.execute(op);
        self.instruction_count += 1;
    }

    pub fn trace_instruction(&self, pc: usize, op: Opcode) {
        let stderr = io::stderr();
        let mut err = stderr.lock();

        // Format: PC=XXXX OP=name [operands] | ESP=depth [top values] | FP=XXXX CSP=XXXX
        let _ = write!(err, "#{:<8} PC={:04X} ", self.instruction_count, pc);

        // Decode operands for display (peek ahead without advancing pc)
        let code = &self.mem[..self.code_size];
        let next = self.pc; // pc already advanced past opcode

        match op {
            // No operand
            Opcode::Halt
            | Opcode::Dup
            | Opcode::Drop
            | Opcode::Swap
            | Opcode::Over
            | Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Neg
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Not
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Eq
            | Opcode::Ne
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge
            | Opcode::Leave
            | Opcode::Load
            | Opcode::Store
            | Opcode::Loadb
            | Opcode::Storeb
            | Opcode::Memcpy
            | Opcode::Memset
            | Opcode::Memcmp
            | Opcode::JmpInd => {
                let _ = write!(err, "{:6}", format!("{op:?}").to_lowercase());
            }
            // Imm8
            Opcode::PushS
            | Opcode::Ret
            | Opcode::Trap
            | Opcode::Enter
            | Opcode::Loadl
            | Opcode::Storel
            | Opcode::Addrl
            | Opcode::Loada
            | Opcode::Storea
            | Opcode::Sys => {
                let imm = if next < code.len() { code[next] } else { 0 };
                let _ = write!(err, "{:6} {}", format!("{op:?}").to_lowercase(), imm);
            }
            // Imm24
            Opcode::Push
            | Opcode::Jmp
            | Opcode::Jz
            | Opcode::Jnz
            | Opcode::Call
            | Opcode::Loadg
            | Opcode::Storeg
            | Opcode::Addrg => {
                let imm = if next + 2 < code.len() {
                    code[next] as u32
                        | ((code[next + 1] as u32) << 8)
                        | ((code[next + 2] as u32) << 16)
                } else {
                    0
                };
                let _ = write!(err, "{:6} 0x{:06X}", format!("{op:?}").to_lowercase(), imm);
            }
            // D8A24
            Opcode::Calln => {
                let d = if next < code.len() { code[next] } else { 0 };
                let a = if next + 3 < code.len() {
                    code[next + 1] as u32
                        | ((code[next + 2] as u32) << 8)
                        | ((code[next + 3] as u32) << 16)
                } else {
                    0
                };
                let _ = write!(err, "{:6} {} 0x{:06X}", "calln", d, a);
            }
            // Imm16
            Opcode::XCall => {
                let imm = if next + 1 < code.len() {
                    code[next] as u16 | ((code[next + 1] as u16) << 8)
                } else {
                    0
                };
                let _ = write!(err, "{:6} slot={}", "xcall", imm);
            }
            // D8O8
            Opcode::Loadn | Opcode::Storen | Opcode::XLoadg | Opcode::XStoreg => {
                let d = if next < code.len() { code[next] } else { 0 };
                let o = if next + 1 < code.len() {
                    code[next + 1]
                } else {
                    0
                };
                let _ = write!(err, "{:6} {} {}", format!("{op:?}").to_lowercase(), d, o);
            }
        }

        // Stack state: show depth and top 4 values
        let depth = self.eval_depth();
        let _ = write!(err, " | ESP={}", depth);
        if depth > 0 {
            let _ = write!(err, " [");
            let show = depth.min(4);
            for i in 0..show {
                let addr = self.esp - (i + 1) * WORD;
                let val = self.read_word(addr);
                if i > 0 {
                    let _ = write!(err, ", ");
                }
                let _ = write!(err, "{}", val);
            }
            if depth > 4 {
                let _ = write!(err, ", ...");
            }
            let _ = write!(err, "]");
        }

        let _ = write!(err, " | FP={:04X}", self.fp_vm);
        let _ = writeln!(err);
    }

    pub fn execute(&mut self, op: Opcode) {
        match op {
            Opcode::Halt => {
                self.status = 1;
            }

            Opcode::Push => {
                let val = self.fetch_u24() as i32;
                self.push_eval(sign_extend_24(val));
            }

            Opcode::PushS => {
                let val = self.fetch_i8();
                self.push_eval(wrap24(val));
            }

            Opcode::Dup => {
                let v = self.peek_eval();
                self.push_eval(v);
            }

            Opcode::Drop => {
                self.pop_eval();
            }

            Opcode::Swap => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(b);
                self.push_eval(a);
            }

            Opcode::Over => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(a);
                self.push_eval(b);
                self.push_eval(a);
            }

            // Arithmetic
            Opcode::Add => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a.wrapping_add(b)));
            }
            Opcode::Sub => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a.wrapping_sub(b)));
            }
            Opcode::Mul => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a.wrapping_mul(b)));
            }
            Opcode::Div => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                if b == 0 {
                    self.trap(1);
                    return;
                }
                self.push_eval(wrap24(a / b));
            }
            Opcode::Mod => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                if b == 0 {
                    self.trap(1);
                    return;
                }
                self.push_eval(wrap24(a % b));
            }
            Opcode::Neg => {
                let a = self.pop_eval();
                self.push_eval(wrap24(-a));
            }

            // Logic
            Opcode::And => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a & b));
            }
            Opcode::Or => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a | b));
            }
            Opcode::Xor => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a ^ b));
            }
            Opcode::Not => {
                let a = self.pop_eval();
                self.push_eval(wrap24(!a));
            }
            Opcode::Shl => {
                let n = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a << (n & 0x1F)));
            }
            Opcode::Shr => {
                let n = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a >> (n & 0x1F)));
            }

            // Comparison
            Opcode::Eq => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a == b { 1 } else { 0 });
            }
            Opcode::Ne => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a != b { 1 } else { 0 });
            }
            Opcode::Lt => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a < b { 1 } else { 0 });
            }
            Opcode::Le => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a <= b { 1 } else { 0 });
            }
            Opcode::Gt => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a > b { 1 } else { 0 });
            }
            Opcode::Ge => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a >= b { 1 } else { 0 });
            }

            // Control flow
            Opcode::Jmp => {
                let addr = self.fetch_u24() as usize;
                self.pc = addr;
            }

            Opcode::Jz => {
                let addr = self.fetch_u24() as usize;
                let flag = self.pop_eval();
                if flag == 0 {
                    self.pc = addr;
                }
            }

            Opcode::Jnz => {
                let addr = self.fetch_u24() as usize;
                let flag = self.pop_eval();
                if flag != 0 {
                    self.pc = addr;
                }
            }

            Opcode::Call => {
                let addr = self.fetch_u24() as usize;
                // Push frame header on call stack:
                //   [0] return PC
                //   [3] dynamic link (old fp_vm)
                //   [6] static link (= dynamic link for non-nested)
                //   [9] saved esp
                self.write_word(self.csp, self.pc as i32); // return PC
                self.write_word(self.csp + WORD, self.fp_vm as i32); // dynamic link
                self.write_word(self.csp + 2 * WORD, self.fp_vm as i32); // static link (same as dynamic for flat calls)
                self.write_word(self.csp + 3 * WORD, self.esp as i32); // saved esp
                self.csp += 4 * WORD;
                self.pc = addr;
            }

            Opcode::Calln => {
                let depth = self.fetch_u8();
                let addr = self.fetch_u24() as usize;

                // Walk static link chain to find enclosing frame
                let mut sl = self.fp_vm;
                for _ in 0..depth {
                    sl = self.read_word(sl + 2 * WORD) as usize; // follow static links
                }

                self.write_word(self.csp, self.pc as i32);
                self.write_word(self.csp + WORD, self.fp_vm as i32);
                self.write_word(self.csp + 2 * WORD, sl as i32); // static link from chain
                self.write_word(self.csp + 3 * WORD, self.esp as i32);
                self.csp += 4 * WORD;
                self.pc = addr;
            }

            Opcode::Ret => {
                let nargs = self.fetch_u8() as usize;

                // Restore from frame header at fp_vm
                let return_pc = self.read_word(self.fp_vm) as usize;
                let dynamic_link = self.read_word(self.fp_vm + WORD) as usize;
                let saved_esp = self.read_word(self.fp_vm + 3 * WORD) as usize;

                // Check if there's a return value on the eval stack
                // (function result sits above what was saved)
                let has_return = self.esp > self.read_word(self.fp_vm + 3 * WORD) as usize;
                let return_val = if has_return {
                    Some(self.pop_eval())
                } else {
                    None
                };

                // Restore state
                self.csp = self.fp_vm;
                self.fp_vm = dynamic_link;
                self.esp = saved_esp - nargs * WORD; // clean args

                // Push return value if present
                if let Some(rv) = return_val {
                    self.push_eval(rv);
                }

                self.pc = return_pc;
            }

            Opcode::Trap => {
                let code = self.fetch_u8();
                self.trap(code);
            }

            // Frame management
            Opcode::Enter => {
                let nlocals = self.fetch_u8() as usize;
                self.fp_vm = self.csp - 4 * WORD; // point to frame header
                // Allocate locals on call stack
                for _ in 0..nlocals {
                    self.write_word(self.csp, 0);
                    self.csp += WORD;
                }
            }

            Opcode::Leave => {
                self.csp = self.fp_vm + 4 * WORD; // deallocate locals, keep frame header
            }

            // Local variable access: offset from fp_vm + 4*WORD (past frame header)
            Opcode::Loadl => {
                let off = self.fetch_u8() as usize;
                let addr = self.fp_vm + 4 * WORD + off * WORD;
                let val = self.read_word(addr);
                self.push_eval(val);
            }

            Opcode::Storel => {
                let off = self.fetch_u8() as usize;
                let val = self.pop_eval();
                let addr = self.fp_vm + 4 * WORD + off * WORD;
                self.write_word(addr, val);
            }

            Opcode::Addrl => {
                let off = self.fetch_u8() as usize;
                let addr = self.fp_vm + 4 * WORD + off * WORD;
                self.push_eval(addr as i32);
            }

            // Global variable access: offset from gp
            Opcode::Loadg => {
                let off = self.fetch_u24() as usize;
                let addr = self.gp + off * WORD;
                let val = self.read_word(addr);
                self.push_eval(val);
            }

            Opcode::Storeg => {
                let off = self.fetch_u24() as usize;
                let val = self.pop_eval();
                let addr = self.gp + off * WORD;
                self.write_word(addr, val);
            }

            Opcode::Addrg => {
                let off = self.fetch_u24() as usize;
                let addr = self.gp + off * WORD;
                self.push_eval(addr as i32);
            }

            // Argument access: relative to saved_esp in frame
            Opcode::Loada => {
                let idx = self.fetch_u8() as usize;
                let saved_esp = self.read_word(self.fp_vm + 3 * WORD) as usize;
                let addr = saved_esp - (idx + 1) * WORD;
                let val = self.read_word(addr);
                self.push_eval(val);
            }

            Opcode::Storea => {
                let idx = self.fetch_u8() as usize;
                let val = self.pop_eval();
                let saved_esp = self.read_word(self.fp_vm + 3 * WORD) as usize;
                let addr = saved_esp - (idx + 1) * WORD;
                self.write_word(addr, val);
            }

            // Nonlocal access via static link chain
            Opcode::Loadn => {
                let depth = self.fetch_u8() as usize;
                let off = self.fetch_u8() as usize;
                let frame = self.follow_static_links(depth);
                let addr = frame + 4 * WORD + off * WORD;
                let val = self.read_word(addr);
                self.push_eval(val);
            }

            Opcode::Storen => {
                let depth = self.fetch_u8() as usize;
                let off = self.fetch_u8() as usize;
                let val = self.pop_eval();
                let frame = self.follow_static_links(depth);
                let addr = frame + 4 * WORD + off * WORD;
                self.write_word(addr, val);
            }

            // Indirect memory access
            Opcode::Load => {
                let addr = self.pop_eval();
                if addr == 0 {
                    self.trap(6);
                    return;
                } // nil pointer
                let val = self.read_word(addr as usize);
                self.push_eval(val);
            }

            Opcode::Store => {
                let addr = self.pop_eval();
                let val = self.pop_eval();
                if addr == 0 {
                    self.trap(6);
                    return;
                }
                self.write_word(addr as usize, val);
            }

            Opcode::Loadb => {
                let addr = self.pop_eval();
                if addr == 0 {
                    self.trap(6);
                    return;
                }
                let val = self.read_byte(addr as usize);
                self.push_eval(val);
            }

            Opcode::Storeb => {
                let addr = self.pop_eval();
                let val = self.pop_eval();
                if addr == 0 {
                    self.trap(6);
                    return;
                }
                self.write_byte(addr as usize, val);
            }

            // Memory block operations
            Opcode::Memcpy => {
                let len = self.pop_eval() as usize;
                let dst = self.pop_eval() as usize;
                let src = self.pop_eval() as usize;
                if len > 0 {
                    // memmove semantics: handle overlapping regions
                    if src < dst {
                        // Copy backward
                        for i in (0..len).rev() {
                            let b = self.read_byte(src + i);
                            self.write_byte(dst + i, b);
                        }
                    } else {
                        // Copy forward
                        for i in 0..len {
                            let b = self.read_byte(src + i);
                            self.write_byte(dst + i, b);
                        }
                    }
                }
            }

            Opcode::Memset => {
                let len = self.pop_eval() as usize;
                let val = self.pop_eval();
                let dst = self.pop_eval() as usize;
                if len > 0 {
                    for i in 0..len {
                        self.write_byte(dst + i, val);
                    }
                }
            }

            Opcode::Memcmp => {
                let len = self.pop_eval() as usize;
                let b = self.pop_eval() as usize;
                let a = self.pop_eval() as usize;
                let mut result: i32 = 0;
                for i in 0..len {
                    let ba = self.read_byte(a + i) & 0xFF;
                    let bb = self.read_byte(b + i) & 0xFF;
                    if ba != bb {
                        result = if ba < bb { -1 } else { 1 };
                        break;
                    }
                }
                self.push_eval(result);
            }

            // Cross-unit call (not supported in tracer yet)
            Opcode::XCall => {
                let _slot = self.fetch_u8() as u16 | ((self.fetch_u8() as u16) << 8);
                self.trap(4); // invalid opcode in single-unit tracer
            }

            // Cross-unit global access (not supported in tracer yet)
            Opcode::XLoadg | Opcode::XStoreg => {
                let _unit_id = self.fetch_u8();
                let _offset = self.fetch_u8();
                self.trap(4); // invalid opcode in single-unit tracer
            }

            // Indirect jump
            Opcode::JmpInd => {
                let addr = self.pop_eval() as usize;
                self.pc = addr;
            }

            // System calls
            Opcode::Sys => {
                let id = self.fetch_u8();
                self.sys_call(id);
            }
        }
    }

    pub fn follow_static_links(&self, depth: usize) -> usize {
        let mut frame = self.fp_vm;
        for _ in 0..depth {
            frame = self.read_word(frame + 2 * WORD) as usize;
        }
        frame
    }

    pub fn sys_call(&mut self, id: u8) {
        match id {
            0 => {
                // HALT
                self.status = 1;
            }
            1 => {
                // PUTC
                let ch = self.pop_eval() as u8;
                let stdout = io::stdout();
                let mut out = stdout.lock();
                let _ = out.write_all(&[ch]);
                let _ = out.flush();
            }
            2 => {
                // GETC
                let ch = if self.stdin_pos < self.stdin_buf.len() {
                    let c = self.stdin_buf[self.stdin_pos];
                    self.stdin_pos += 1;
                    c as i32
                } else {
                    // Read from actual stdin
                    let mut buf = [0u8; 1];
                    match io::stdin().read_exact(&mut buf) {
                        Ok(()) => buf[0] as i32,
                        Err(_) => -1, // EOF
                    }
                };
                self.push_eval(ch);
            }
            3 => {
                // LED (ignore in tracer)
                self.pop_eval();
            }
            4 => {
                // ALLOC — bump allocator
                let size = self.pop_eval() as usize;
                let ptr = self.hp;
                self.hp += size;
                // Extend memory if needed
                if self.hp > self.mem.len() {
                    self.mem.resize(self.hp, 0);
                }
                self.push_eval(ptr as i32);
            }
            5 => {
                // FREE — no-op
                self.pop_eval();
            }
            6 => {
                // READ_SWITCH (always 0 in tracer)
                self.push_eval(0);
            }
            7 => {
                // SET_IRT_BASE (no-op in single-unit tracer)
                self.pop_eval();
            }
            8 => {
                // DUMP_STATE
                eprintln!(
                    "VM: pc={:06X} esp={:06X} csp={:06X} fp={:06X}",
                    self.pc, self.esp, self.csp, self.fp_vm
                );
                eprintln!("    gp={:06X} hp={:06X} code={:06X}", self.gp, self.hp, 0);
            }
            _ => {
                eprintln!("pv24t: unknown sys call {id}");
                self.trap(4);
            }
        }
    }
}

/// Decode an opcode byte to an Opcode enum, or None if invalid.
pub fn decode_opcode(op_byte: u8) -> Option<Opcode> {
    Some(match op_byte {
        0x00 => Opcode::Halt,
        0x01 => Opcode::Push,
        0x02 => Opcode::PushS,
        0x03 => Opcode::Dup,
        0x04 => Opcode::Drop,
        0x05 => Opcode::Swap,
        0x06 => Opcode::Over,
        0x10 => Opcode::Add,
        0x11 => Opcode::Sub,
        0x12 => Opcode::Mul,
        0x13 => Opcode::Div,
        0x14 => Opcode::Mod,
        0x15 => Opcode::Neg,
        0x16 => Opcode::And,
        0x17 => Opcode::Or,
        0x18 => Opcode::Xor,
        0x19 => Opcode::Not,
        0x1A => Opcode::Shl,
        0x1B => Opcode::Shr,
        0x20 => Opcode::Eq,
        0x21 => Opcode::Ne,
        0x22 => Opcode::Lt,
        0x23 => Opcode::Le,
        0x24 => Opcode::Gt,
        0x25 => Opcode::Ge,
        0x30 => Opcode::Jmp,
        0x31 => Opcode::Jz,
        0x32 => Opcode::Jnz,
        0x33 => Opcode::Call,
        0x34 => Opcode::Ret,
        0x35 => Opcode::Calln,
        0x36 => Opcode::Trap,
        0x40 => Opcode::Enter,
        0x41 => Opcode::Leave,
        0x42 => Opcode::Loadl,
        0x43 => Opcode::Storel,
        0x44 => Opcode::Loadg,
        0x45 => Opcode::Storeg,
        0x46 => Opcode::Addrl,
        0x47 => Opcode::Addrg,
        0x48 => Opcode::Loada,
        0x49 => Opcode::Storea,
        0x4A => Opcode::Loadn,
        0x4B => Opcode::Storen,
        0x50 => Opcode::Load,
        0x51 => Opcode::Store,
        0x52 => Opcode::Loadb,
        0x53 => Opcode::Storeb,
        0x60 => Opcode::Sys,
        0x70 => Opcode::Memcpy,
        0x71 => Opcode::Memset,
        0x72 => Opcode::Memcmp,
        0x73 => Opcode::JmpInd,
        0x74 => Opcode::XCall,
        0x75 => Opcode::XLoadg,
        0x76 => Opcode::XStoreg,
        _ => return None,
    })
}

/// Disassemble one instruction at `pc` in `code`. Returns (mnemonic, size_in_bytes).
pub fn disasm_one(code: &[u8], pc: usize) -> (String, usize) {
    if pc >= code.len() {
        return ("<eof>".into(), 0);
    }
    let op_byte = code[pc];
    let op = match decode_opcode(op_byte) {
        Some(op) => op,
        None => return (format!(".byte 0x{op_byte:02X}"), 1),
    };
    let name = format!("{op:?}").to_lowercase();
    let size = pa24r::opcode_size(op_byte);
    let rest = &code[pc + 1..];
    let text = match size {
        1 => name,
        2 => {
            let imm = rest.first().copied().unwrap_or(0) as i8;
            format!("{name} {imm}")
        }
        3 => {
            // Could be D8O8 (xloadg/xstoreg/loadn/storen) or imm16 (xcall)
            match op {
                Opcode::XCall => {
                    let slot = rest[0] as u16 | ((rest.get(1).copied().unwrap_or(0) as u16) << 8);
                    format!("xcall slot={slot}")
                }
                Opcode::Loadn | Opcode::Storen | Opcode::XLoadg | Opcode::XStoreg => {
                    let d = rest.first().copied().unwrap_or(0);
                    let o = rest.get(1).copied().unwrap_or(0);
                    format!("{name} {d}, {o}")
                }
                _ => name,
            }
        }
        4 => {
            // IMM24
            let v = rest[0] as u32
                | ((rest.get(1).copied().unwrap_or(0) as u32) << 8)
                | ((rest.get(2).copied().unwrap_or(0) as u32) << 16);
            format!("{name} 0x{v:06X}")
        }
        5 => {
            // D8A24 (calln)
            let d = rest[0];
            let a = rest[1] as u32
                | ((rest.get(2).copied().unwrap_or(0) as u32) << 8)
                | ((rest.get(3).copied().unwrap_or(0) as u32) << 16);
            format!("{name} {d}, 0x{a:06X}")
        }
        _ => format!("{name} <size {size}>"),
    };
    (text, size)
}
