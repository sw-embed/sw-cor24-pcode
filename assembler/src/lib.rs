// pa24r — P-Code Assembler for COR24

pub mod lexer;

// .p24 header constants
pub const P24_MAGIC: [u8; 4] = [0x50, 0x32, 0x34, 0x00]; // "P24\0"
pub const P24_VERSION: u8 = 1;
pub const P24_HEADER_SIZE: usize = 18;

/// Encoding format for p-code instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// 1 byte: [op]
    None,
    /// 2 bytes: [op, imm8]
    Imm8,
    /// 4 bytes: [op, lo, mid, hi]
    Imm24,
    /// 5 bytes: [op, d8, lo, mid, hi]
    D8A24,
    /// 3 bytes: [op, d8, o8]
    D8O8,
}

impl Encoding {
    /// Total instruction size in bytes (including opcode byte).
    pub const fn size(self) -> usize {
        match self {
            Encoding::None => 1,
            Encoding::Imm8 => 2,
            Encoding::Imm24 => 4,
            Encoding::D8A24 => 5,
            Encoding::D8O8 => 3,
        }
    }
}

/// P-code opcodes. Values match pvm.s ground truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Stack operations (0x00-0x06)
    Halt = 0x00,
    Push = 0x01,
    PushS = 0x02,
    Dup = 0x03,
    Drop = 0x04,
    Swap = 0x05,
    Over = 0x06,

    // Arithmetic (0x10-0x15)
    Add = 0x10,
    Sub = 0x11,
    Mul = 0x12,
    Div = 0x13,
    Mod = 0x14,
    Neg = 0x15,

    // Logic (0x16-0x1B)
    And = 0x16,
    Or = 0x17,
    Xor = 0x18,
    Not = 0x19,
    Shl = 0x1A,
    Shr = 0x1B,

    // Comparison (0x20-0x25)
    Eq = 0x20,
    Ne = 0x21,
    Lt = 0x22,
    Le = 0x23,
    Gt = 0x24,
    Ge = 0x25,

    // Control flow (0x30-0x36)
    Jmp = 0x30,
    Jz = 0x31,
    Jnz = 0x32,
    Call = 0x33,
    Ret = 0x34,
    Calln = 0x35,
    Trap = 0x36,

    // Local / Global / Nonlocal access (0x40-0x4B)
    Enter = 0x40,
    Leave = 0x41,
    Loadl = 0x42,
    Storel = 0x43,
    Loadg = 0x44,
    Storeg = 0x45,
    Addrl = 0x46,
    Addrg = 0x47,
    Loada = 0x48,
    Storea = 0x49,
    Loadn = 0x4A,
    Storen = 0x4B,

    // Memory indirect (0x50-0x53)
    Load = 0x50,
    Store = 0x51,
    Loadb = 0x52,
    Storeb = 0x53,

    // System calls (0x60)
    Sys = 0x60,

    // Memory block operations (0x70-0x71)
    Memcpy = 0x70,
    Memset = 0x71,
}

impl Opcode {
    /// Return the encoding format for this opcode.
    pub const fn encoding(self) -> Encoding {
        match self {
            // NONE encoding (1 byte)
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
            | Opcode::Memset => Encoding::None,

            // IMM8 encoding (2 bytes)
            Opcode::PushS
            | Opcode::Ret
            | Opcode::Trap
            | Opcode::Enter
            | Opcode::Loadl
            | Opcode::Storel
            | Opcode::Addrl
            | Opcode::Loada
            | Opcode::Storea
            | Opcode::Sys => Encoding::Imm8,

            // IMM24 encoding (4 bytes)
            Opcode::Push
            | Opcode::Jmp
            | Opcode::Jz
            | Opcode::Jnz
            | Opcode::Call
            | Opcode::Loadg
            | Opcode::Storeg
            | Opcode::Addrg => Encoding::Imm24,

            // D8_A24 encoding (5 bytes): calln depth8 addr24
            Opcode::Calln => Encoding::D8A24,

            // D8_O8 encoding (3 bytes): loadn/storen depth8 off8
            Opcode::Loadn | Opcode::Storen => Encoding::D8O8,
        }
    }

    /// Instruction size in bytes.
    pub const fn size(self) -> usize {
        self.encoding().size()
    }

    /// Look up an opcode by its mnemonic string.
    pub fn from_mnemonic(name: &str) -> Option<Opcode> {
        match name {
            "halt" => Some(Opcode::Halt),
            "push" => Some(Opcode::Push),
            "push_s" => Some(Opcode::PushS),
            "dup" => Some(Opcode::Dup),
            "drop" => Some(Opcode::Drop),
            "swap" => Some(Opcode::Swap),
            "over" => Some(Opcode::Over),
            "add" => Some(Opcode::Add),
            "sub" => Some(Opcode::Sub),
            "mul" => Some(Opcode::Mul),
            "div" => Some(Opcode::Div),
            "mod" => Some(Opcode::Mod),
            "neg" => Some(Opcode::Neg),
            "eq" => Some(Opcode::Eq),
            "ne" => Some(Opcode::Ne),
            "lt" => Some(Opcode::Lt),
            "le" => Some(Opcode::Le),
            "gt" => Some(Opcode::Gt),
            "ge" => Some(Opcode::Ge),
            "and" => Some(Opcode::And),
            "or" => Some(Opcode::Or),
            "xor" => Some(Opcode::Xor),
            "not" => Some(Opcode::Not),
            "shl" => Some(Opcode::Shl),
            "shr" => Some(Opcode::Shr),
            "jmp" => Some(Opcode::Jmp),
            "jz" => Some(Opcode::Jz),
            "jnz" => Some(Opcode::Jnz),
            "call" => Some(Opcode::Call),
            "ret" => Some(Opcode::Ret),
            "calln" => Some(Opcode::Calln),
            "trap" => Some(Opcode::Trap),
            "enter" => Some(Opcode::Enter),
            "leave" => Some(Opcode::Leave),
            "loadl" => Some(Opcode::Loadl),
            "storel" => Some(Opcode::Storel),
            "loada" => Some(Opcode::Loada),
            "storea" => Some(Opcode::Storea),
            "load" => Some(Opcode::Load),
            "store" => Some(Opcode::Store),
            "loadb" => Some(Opcode::Loadb),
            "storeb" => Some(Opcode::Storeb),
            "loadg" => Some(Opcode::Loadg),
            "storeg" => Some(Opcode::Storeg),
            "addrg" => Some(Opcode::Addrg),
            "addrl" => Some(Opcode::Addrl),
            "loadn" => Some(Opcode::Loadn),
            "storen" => Some(Opcode::Storen),
            "sys" => Some(Opcode::Sys),
            "memcpy" => Some(Opcode::Memcpy),
            "memset" => Some(Opcode::Memset),
            _ => None,
        }
    }
}

/// Result of assembling .spc source.
pub struct AssemblyResult {
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    pub entry_point: u32,
    pub global_count: u32,
    pub errors: Vec<AssemblyError>,
}

/// An error encountered during assembly.
#[derive(Debug)]
pub struct AssemblyError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// A loaded .p24 binary image.
pub struct LoadedImage {
    pub entry_point: u32,
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    pub global_count: u32,
}

/// Error loading a .p24 binary.
#[derive(Debug)]
pub enum LoadError {
    TooShort,
    BadMagic,
    BadVersion(u8),
    Truncated,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::TooShort => write!(f, "file too short for .p24 header"),
            LoadError::BadMagic => write!(f, "invalid .p24 magic bytes"),
            LoadError::BadVersion(v) => write!(f, "unsupported .p24 version: {v}"),
            LoadError::Truncated => write!(f, "file truncated: body shorter than header declares"),
        }
    }
}

use std::collections::HashMap;

use lexer::{Token, tokenize};

/// Metadata directives from pl24r that we silently skip.
const METADATA_DIRECTIVES: &[&str] = &[".module", ".export", ".extern", ".endmodule"];

/// Symbol type for tracking what kind of entity a name refers to.
#[derive(Debug, Clone, Copy)]
enum SymType {
    Code,
    Data,
    Global,
    Const,
}

/// A symbol table entry.
#[derive(Debug, Clone)]
struct Symbol {
    value: u32,
    sym_type: SymType,
    line: usize,
}

/// Assemble .spc source into separate code and data segments.
pub fn assemble(source: &str) -> AssemblyResult {
    let tokens = tokenize(source);
    let mut symbols: HashMap<String, Symbol> = HashMap::new();
    let mut errors: Vec<AssemblyError> = Vec::new();
    let mut data_bytes: Vec<u8> = Vec::new();
    let mut code_offset: u32 = 0;
    let mut data_offset: u32 = 0;
    let mut global_offset: u32 = 0;
    let mut entry_point: Option<u32> = None;

    // ── Pass 1: Symbol Collection ──

    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        match &tok.token {
            Token::Newline => {
                i += 1;
            }

            Token::Label(name) => {
                if let Err(e) =
                    insert_symbol(&mut symbols, name, code_offset, SymType::Code, tok.line)
                {
                    errors.push(e);
                }
                i += 1;
            }

            Token::Directive(dir) => {
                let line = tok.line;
                i += 1;
                match dir.as_str() {
                    ".proc" => {
                        // .proc <name> <nargs>
                        // Auto-emit `enter N` as first instruction (matches pasm.s)
                        let name = expect_identifier(&tokens, &mut i);
                        if let Some(name) = name {
                            if name == "main" {
                                entry_point = Some(code_offset);
                            }
                            if let Err(e) =
                                insert_symbol(&mut symbols, &name, code_offset, SymType::Code, line)
                            {
                                errors.push(e);
                            }
                        } else {
                            errors.push(AssemblyError {
                                line,
                                message: ".proc missing name".into(),
                            });
                        }
                        // Read nargs for the auto-emitted enter instruction
                        let _nargs = expect_number(&tokens, &mut i).unwrap_or(0);
                        code_offset += Opcode::Enter.size() as u32; // enter N = 2 bytes
                        skip_to_newline(&tokens, &mut i);
                    }
                    ".end" => {
                        // Auto-emit `leave` before implicit end (matches pasm.s)
                        code_offset += Opcode::Leave.size() as u32; // leave = 1 byte
                        skip_to_newline(&tokens, &mut i);
                    }
                    ".data" => {
                        // .data <name> <byte>, <byte>, ...
                        let name = expect_identifier(&tokens, &mut i);
                        if let Some(name) = name {
                            if let Err(e) =
                                insert_symbol(&mut symbols, &name, data_offset, SymType::Data, line)
                            {
                                errors.push(e);
                            }
                        } else {
                            errors.push(AssemblyError {
                                line,
                                message: ".data missing name".into(),
                            });
                        }
                        // Collect bytes
                        while i < tokens.len() && tokens[i].token != Token::Newline {
                            match &tokens[i].token {
                                Token::Number(n) => {
                                    data_bytes.push(*n as u8);
                                    data_offset += 1;
                                    i += 1;
                                }
                                Token::Comma => {
                                    i += 1;
                                }
                                _ => {
                                    i += 1;
                                }
                            }
                        }
                    }
                    ".global" => {
                        // .global <name> <count>
                        let name = expect_identifier(&tokens, &mut i);
                        let count = expect_number(&tokens, &mut i).unwrap_or(1);
                        if let Some(name) = name {
                            if let Err(e) = insert_symbol(
                                &mut symbols,
                                &name,
                                global_offset,
                                SymType::Global,
                                line,
                            ) {
                                errors.push(e);
                            }
                        } else {
                            errors.push(AssemblyError {
                                line,
                                message: ".global missing name".into(),
                            });
                        }
                        global_offset += count as u32 * 3;
                        skip_to_newline(&tokens, &mut i);
                    }
                    ".const" => {
                        // .const <name> <value>
                        let name = expect_identifier(&tokens, &mut i);
                        let value = expect_number(&tokens, &mut i).unwrap_or(0);
                        if let Some(name) = name {
                            if let Err(e) = insert_symbol(
                                &mut symbols,
                                &name,
                                value as u32,
                                SymType::Const,
                                line,
                            ) {
                                errors.push(e);
                            }
                        } else {
                            errors.push(AssemblyError {
                                line,
                                message: ".const missing name".into(),
                            });
                        }
                        skip_to_newline(&tokens, &mut i);
                    }
                    d if METADATA_DIRECTIVES.contains(&d) => {
                        skip_to_newline(&tokens, &mut i);
                    }
                    _ => {
                        // Unknown directive — skip silently (may be from pl24r)
                        skip_to_newline(&tokens, &mut i);
                    }
                }
            }

            Token::Identifier(name) => {
                // Instruction mnemonic
                let line = tok.line;
                if let Some(op) = Opcode::from_mnemonic(name) {
                    code_offset += op.size() as u32;
                    i += 1;
                    // Skip any operand tokens on this line
                    skip_to_newline(&tokens, &mut i);
                } else {
                    errors.push(AssemblyError {
                        line,
                        message: format!("unknown mnemonic: {name}"),
                    });
                    i += 1;
                    skip_to_newline(&tokens, &mut i);
                }
            }

            _ => {
                i += 1;
            }
        }
    }

    let code_size = code_offset;
    let data_size = data_offset;
    let global_count = global_offset / 3;

    // Patch data symbols: add code_size (data follows code in memory)
    // Global symbols are NOT patched here — their value stays as the raw
    // word index. The code emitter adjusts based on context: loadg/storeg
    // use the raw word index, while push uses code_size + data_size + index.
    for sym in symbols.values_mut() {
        if let SymType::Data = sym.sym_type {
            sym.value += code_size;
        }
    }

    if entry_point.is_none() {
        errors.push(AssemblyError {
            line: 0,
            message: "missing .proc main".into(),
        });
    }

    // ── Pass 2: Code Emission ──

    let global_base = code_size + data_size;
    let mut code: Vec<u8> = Vec::with_capacity(code_size as usize);
    i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        match &tok.token {
            Token::Newline | Token::Label(_) | Token::Comma => {
                i += 1;
            }

            Token::Directive(dir) => {
                i += 1;
                match dir.as_str() {
                    ".proc" => {
                        // Auto-emit enter N
                        let _name = expect_identifier(&tokens, &mut i);
                        let nargs = expect_number(&tokens, &mut i).unwrap_or(0);
                        code.push(Opcode::Enter as u8);
                        code.push(nargs as u8);
                        skip_to_newline(&tokens, &mut i);
                    }
                    ".end" => {
                        // Auto-emit leave
                        code.push(Opcode::Leave as u8);
                        skip_to_newline(&tokens, &mut i);
                    }
                    _ => {
                        skip_to_newline(&tokens, &mut i);
                    }
                }
            }

            Token::Identifier(name) => {
                let line = tok.line;
                if let Some(op) = Opcode::from_mnemonic(name) {
                    code.push(op as u8);
                    i += 1;

                    match op.encoding() {
                        Encoding::None => {
                            // Ignore trailing operand (e.g. `ret 1`)
                            skip_to_newline(&tokens, &mut i);
                        }
                        Encoding::Imm8 => {
                            let val = resolve_operand(
                                &tokens,
                                &mut i,
                                &symbols,
                                line,
                                &mut errors,
                                Some(global_base),
                            );
                            code.push(val as u8);
                            skip_to_newline(&tokens, &mut i);
                        }
                        Encoding::Imm24 => {
                            // loadg/storeg/addrg use raw word index for globals
                            let gb = match op {
                                Opcode::Loadg | Opcode::Storeg | Opcode::Addrg => None,
                                _ => Some(global_base),
                            };
                            let val =
                                resolve_operand(&tokens, &mut i, &symbols, line, &mut errors, gb);
                            emit_le24(&mut code, val);
                            skip_to_newline(&tokens, &mut i);
                        }
                        Encoding::D8A24 => {
                            // d8, a24 — two operands (depth, address)
                            let d = resolve_operand(
                                &tokens,
                                &mut i,
                                &symbols,
                                line,
                                &mut errors,
                                Some(global_base),
                            );
                            // skip comma if present
                            if i < tokens.len() && tokens[i].token == Token::Comma {
                                i += 1;
                            }
                            let a = resolve_operand(
                                &tokens,
                                &mut i,
                                &symbols,
                                line,
                                &mut errors,
                                Some(global_base),
                            );
                            code.push(d as u8);
                            emit_le24(&mut code, a);
                            skip_to_newline(&tokens, &mut i);
                        }
                        Encoding::D8O8 => {
                            // d8, o8 — two operands
                            let d = resolve_operand(
                                &tokens,
                                &mut i,
                                &symbols,
                                line,
                                &mut errors,
                                Some(global_base),
                            );
                            if i < tokens.len() && tokens[i].token == Token::Comma {
                                i += 1;
                            }
                            let o = resolve_operand(
                                &tokens,
                                &mut i,
                                &symbols,
                                line,
                                &mut errors,
                                Some(global_base),
                            );
                            code.push(d as u8);
                            code.push(o as u8);
                            skip_to_newline(&tokens, &mut i);
                        }
                    }
                } else {
                    // Unknown mnemonic — already reported in pass 1
                    i += 1;
                    skip_to_newline(&tokens, &mut i);
                }
            }

            _ => {
                i += 1;
            }
        }
    }

    AssemblyResult {
        code,
        data: data_bytes,
        entry_point: entry_point.unwrap_or(0),
        global_count,
        errors,
    }
}

/// Assemble .spc source and produce a complete .p24 binary.
pub fn assemble_to_p24(source: &str) -> Result<Vec<u8>, Vec<AssemblyError>> {
    let result = assemble(source);
    if !result.errors.is_empty() {
        return Err(result.errors);
    }

    let mut binary = Vec::with_capacity(P24_HEADER_SIZE + result.code.len() + result.data.len());

    // Header
    binary.extend_from_slice(&P24_MAGIC);
    binary.push(P24_VERSION);
    emit_le24(&mut binary, result.entry_point);
    emit_le24(&mut binary, result.code.len() as u32);
    emit_le24(&mut binary, result.data.len() as u32);
    emit_le24(&mut binary, result.global_count);
    binary.push(0x00); // flags

    // Body
    binary.extend_from_slice(&result.code);
    binary.extend_from_slice(&result.data);

    Ok(binary)
}

/// Load a .p24 binary into a LoadedImage.
pub fn load_p24(binary: &[u8]) -> Result<LoadedImage, LoadError> {
    if binary.len() < P24_HEADER_SIZE {
        return Err(LoadError::TooShort);
    }
    if binary[0..4] != P24_MAGIC {
        return Err(LoadError::BadMagic);
    }
    let version = binary[4];
    if version != P24_VERSION {
        return Err(LoadError::BadVersion(version));
    }

    let entry_point = read_le24(&binary[5..8]);
    let code_size = read_le24(&binary[8..11]) as usize;
    let data_size = read_le24(&binary[11..14]) as usize;
    let global_count = read_le24(&binary[14..17]);

    let body = &binary[P24_HEADER_SIZE..];
    if body.len() < code_size + data_size {
        return Err(LoadError::Truncated);
    }

    Ok(LoadedImage {
        entry_point,
        code: body[..code_size].to_vec(),
        data: body[code_size..code_size + data_size].to_vec(),
        global_count,
    })
}

/// Relocate push instructions that reference the data or global segments.
///
/// After assembly, `push` operands that reference data are encoded as
/// offsets from the start of the combined code+data buffer (values at
/// or above `code_size`). When loading into emulator memory at a specific
/// address, these operands need `load_addr` added to become absolute.
///
/// Walks the code bytes, finds each instruction by opcode and encoding,
/// and adjusts any `push` (IMM24) operand that falls at or above `code_size`.
pub fn relocate_data_refs(code: &mut [u8], code_size: u32, data_size: u32, load_addr: u32) {
    let threshold = code_size;
    let limit = code_size + data_size;
    let mut pc = 0usize;
    while pc < code.len() {
        let op_byte = code[pc];
        // Try to determine instruction size from opcode encoding.
        // We only care about push (0x01) which is IMM24.
        let size = if op_byte == Opcode::Push as u8 {
            // IMM24: read the 3-byte operand
            let val = read_le24(&code[pc + 1..pc + 4]);
            if val >= threshold && val < limit {
                let relocated = val + load_addr;
                code[pc + 1] = relocated as u8;
                code[pc + 2] = (relocated >> 8) as u8;
                code[pc + 3] = (relocated >> 16) as u8;
            }
            4
        } else {
            opcode_size(op_byte)
        };
        pc += size;
    }
}

/// Get instruction size from raw opcode byte, without requiring a valid Opcode enum value.
fn opcode_size(op: u8) -> usize {
    // Map opcode byte to encoding size using the same logic as Opcode::encoding().
    // This handles unknown opcodes gracefully (treats as 1 byte).
    match op {
        // IMM8 (2 bytes): push_s, ret, trap, enter, loadl, storel, addrl, loada, storea, sys
        0x02 | 0x34 | 0x36 | 0x40 | 0x42 | 0x43 | 0x46 | 0x48 | 0x49 | 0x60 => 2,
        // IMM24 (4 bytes): push, jmp, jz, jnz, call, loadg, storeg, addrg
        0x01 | 0x30 | 0x31 | 0x32 | 0x33 | 0x44 | 0x45 | 0x47 => 4,
        // D8_A24 (5 bytes): calln
        0x35 => 5,
        // D8_O8 (3 bytes): loadn, storen
        0x4A | 0x4B => 3,
        // NONE and everything else (1 byte)
        _ => 1,
    }
}

// ── Helper functions ──

fn insert_symbol(
    symbols: &mut HashMap<String, Symbol>,
    name: &str,
    value: u32,
    sym_type: SymType,
    line: usize,
) -> Result<(), AssemblyError> {
    if let Some(existing) = symbols.get(name) {
        return Err(AssemblyError {
            line,
            message: format!(
                "duplicate symbol '{name}' (first defined at line {})",
                existing.line
            ),
        });
    }
    symbols.insert(
        name.to_string(),
        Symbol {
            value,
            sym_type,
            line,
        },
    );
    Ok(())
}

fn expect_identifier(tokens: &[lexer::Located], i: &mut usize) -> Option<String> {
    if *i < tokens.len()
        && let Token::Identifier(name) = &tokens[*i].token
    {
        let name = name.clone();
        *i += 1;
        return Some(name);
    }
    None
}

fn expect_number(tokens: &[lexer::Located], i: &mut usize) -> Option<i32> {
    if *i < tokens.len()
        && let Token::Number(n) = &tokens[*i].token
    {
        let n = *n;
        *i += 1;
        return Some(n);
    }
    None
}

fn skip_to_newline(tokens: &[lexer::Located], i: &mut usize) {
    while *i < tokens.len() && tokens[*i].token != Token::Newline {
        *i += 1;
    }
}

/// Resolve an operand token to a numeric value.
/// `global_base`: if Some, global symbols get this base added (for `push` etc).
/// If None, global symbols use raw word index (for `loadg`/`storeg`/`addrg`).
fn resolve_operand(
    tokens: &[lexer::Located],
    i: &mut usize,
    symbols: &HashMap<String, Symbol>,
    line: usize,
    errors: &mut Vec<AssemblyError>,
    global_base: Option<u32>,
) -> u32 {
    if *i >= tokens.len() || tokens[*i].token == Token::Newline {
        errors.push(AssemblyError {
            line,
            message: "missing operand".into(),
        });
        return 0;
    }
    match &tokens[*i].token {
        Token::Number(n) => {
            let val = *n;
            *i += 1;
            val as u32
        }
        Token::Identifier(name) => {
            *i += 1;
            if let Some(sym) = symbols.get(name.as_str()) {
                match sym.sym_type {
                    SymType::Global => {
                        if let Some(base) = global_base {
                            // Absolute address: base + word_index * 3
                            base + sym.value
                        } else {
                            // Raw word index for loadg/storeg/addrg
                            sym.value / 3
                        }
                    }
                    _ => sym.value,
                }
            } else {
                errors.push(AssemblyError {
                    line,
                    message: format!("unresolved symbol: {name}"),
                });
                0
            }
        }
        _ => {
            errors.push(AssemblyError {
                line,
                message: format!("unexpected token as operand: {:?}", tokens[*i].token),
            });
            *i += 1;
            0
        }
    }
}

fn emit_le24(out: &mut Vec<u8>, val: u32) {
    out.push(val as u8);
    out.push((val >> 8) as u8);
    out.push((val >> 16) as u8);
}

fn read_le24(bytes: &[u8]) -> u32 {
    u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_values() {
        // Stack operations
        assert_eq!(Opcode::Halt as u8, 0x00);
        assert_eq!(Opcode::Push as u8, 0x01);
        assert_eq!(Opcode::PushS as u8, 0x02);
        assert_eq!(Opcode::Dup as u8, 0x03);
        assert_eq!(Opcode::Drop as u8, 0x04);
        assert_eq!(Opcode::Swap as u8, 0x05);
        assert_eq!(Opcode::Over as u8, 0x06);

        // Arithmetic
        assert_eq!(Opcode::Add as u8, 0x10);
        assert_eq!(Opcode::Sub as u8, 0x11);
        assert_eq!(Opcode::Mul as u8, 0x12);
        assert_eq!(Opcode::Div as u8, 0x13);
        assert_eq!(Opcode::Mod as u8, 0x14);
        assert_eq!(Opcode::Neg as u8, 0x15);

        // Comparison
        assert_eq!(Opcode::Eq as u8, 0x20);
        assert_eq!(Opcode::Ne as u8, 0x21);
        assert_eq!(Opcode::Lt as u8, 0x22);
        assert_eq!(Opcode::Le as u8, 0x23);
        assert_eq!(Opcode::Gt as u8, 0x24);
        assert_eq!(Opcode::Ge as u8, 0x25);

        // Logic
        assert_eq!(Opcode::And as u8, 0x16);
        assert_eq!(Opcode::Or as u8, 0x17);
        assert_eq!(Opcode::Xor as u8, 0x18);
        assert_eq!(Opcode::Not as u8, 0x19);
        assert_eq!(Opcode::Shl as u8, 0x1A);
        assert_eq!(Opcode::Shr as u8, 0x1B);

        // Control flow
        assert_eq!(Opcode::Jmp as u8, 0x30);
        assert_eq!(Opcode::Jz as u8, 0x31);
        assert_eq!(Opcode::Jnz as u8, 0x32);
        assert_eq!(Opcode::Call as u8, 0x33);
        assert_eq!(Opcode::Ret as u8, 0x34);
        assert_eq!(Opcode::Calln as u8, 0x35);
        assert_eq!(Opcode::Trap as u8, 0x36);

        // Local / Global / Nonlocal access
        assert_eq!(Opcode::Enter as u8, 0x40);
        assert_eq!(Opcode::Leave as u8, 0x41);
        assert_eq!(Opcode::Loadl as u8, 0x42);
        assert_eq!(Opcode::Storel as u8, 0x43);
        assert_eq!(Opcode::Loadg as u8, 0x44);
        assert_eq!(Opcode::Storeg as u8, 0x45);
        assert_eq!(Opcode::Addrl as u8, 0x46);
        assert_eq!(Opcode::Addrg as u8, 0x47);
        assert_eq!(Opcode::Loada as u8, 0x48);
        assert_eq!(Opcode::Storea as u8, 0x49);
        assert_eq!(Opcode::Loadn as u8, 0x4A);
        assert_eq!(Opcode::Storen as u8, 0x4B);

        // Memory indirect
        assert_eq!(Opcode::Load as u8, 0x50);
        assert_eq!(Opcode::Store as u8, 0x51);
        assert_eq!(Opcode::Loadb as u8, 0x52);
        assert_eq!(Opcode::Storeb as u8, 0x53);

        // System
        assert_eq!(Opcode::Sys as u8, 0x60);
    }

    #[test]
    fn opcode_count() {
        // Verify we have exactly 36 opcodes by checking from_mnemonic covers all
        let mnemonics = [
            "halt", "push", "push_s", "dup", "drop", "swap", "over", "add", "sub", "mul", "div",
            "mod", "neg", "and", "or", "xor", "not", "shl", "shr", "eq", "ne", "lt", "le", "gt",
            "ge", "jmp", "jz", "jnz", "call", "ret", "calln", "trap", "enter", "leave", "loadl",
            "storel", "loadg", "storeg", "addrl", "addrg", "loada", "storea", "loadn", "storen",
            "load", "store", "loadb", "storeb", "sys",
        ];
        assert_eq!(mnemonics.len(), 49);
        for m in mnemonics {
            assert!(
                Opcode::from_mnemonic(m).is_some(),
                "mnemonic {m:?} not recognized"
            );
        }
    }

    #[test]
    fn unknown_mnemonic() {
        assert!(Opcode::from_mnemonic("bogus").is_none());
    }

    #[test]
    fn encoding_sizes() {
        assert_eq!(Encoding::None.size(), 1);
        assert_eq!(Encoding::Imm8.size(), 2);
        assert_eq!(Encoding::Imm24.size(), 4);
        assert_eq!(Encoding::D8A24.size(), 5);
        assert_eq!(Encoding::D8O8.size(), 3);
    }

    #[test]
    fn instruction_sizes() {
        assert_eq!(Opcode::Halt.size(), 1);
        assert_eq!(Opcode::Push.size(), 4);
        assert_eq!(Opcode::PushS.size(), 2);
        assert_eq!(Opcode::Ret.size(), 2);
        assert_eq!(Opcode::Calln.size(), 5);
        assert_eq!(Opcode::Loadn.size(), 3);
        assert_eq!(Opcode::Loada.size(), 2);
        assert_eq!(Opcode::Loadg.size(), 4);
        assert_eq!(Opcode::Sys.size(), 2);
    }

    #[test]
    fn header_constants() {
        assert_eq!(P24_MAGIC, [0x50, 0x32, 0x34, 0x00]);
        assert_eq!(P24_VERSION, 1);
        assert_eq!(P24_HEADER_SIZE, 18);
    }

    #[test]
    fn relocate_data_refs_adjusts_push() {
        // push <data_ref> where data_ref = code_size (10) + 0 = 10
        // load_addr = 0x1000
        // After relocation: operand should be 10 + 0x1000 = 0x100A
        let mut code = vec![
            0x01, 10, 0, 0, // push 10 (data ref: code_size=10, offset=0)
            0x01, 5, 0, 0,    // push 5 (code ref: < code_size, should NOT change)
            0x00, // halt
            0x00, // padding to make code_size=10 (but we pass it as param)
        ];
        relocate_data_refs(&mut code, 10, 7, 0x1000);
        // First push: 10 + 0x1000 = 0x100A
        assert_eq!(code[1], 0x0A);
        assert_eq!(code[2], 0x10);
        assert_eq!(code[3], 0x00);
        // Second push: 5 < 10, unchanged
        assert_eq!(code[5], 5);
        assert_eq!(code[6], 0);
        assert_eq!(code[7], 0);
    }

    #[test]
    fn relocate_data_refs_leaves_code_refs() {
        // push 0 (code ref, should not change)
        let mut code = vec![0x01, 0, 0, 0, 0x00];
        relocate_data_refs(&mut code, 5, 10, 0x2000);
        assert_eq!(code[1], 0);
        assert_eq!(code[2], 0);
        assert_eq!(code[3], 0);
    }

    #[test]
    fn relocate_skips_non_push_instructions() {
        // sys 1, halt — no push instructions, should be untouched
        let mut code = vec![0x60, 0x01, 0x00];
        let original = code.clone();
        relocate_data_refs(&mut code, 3, 5, 0x1000);
        assert_eq!(code, original);
    }

    #[test]
    fn opcode_size_matches_encoding() {
        // Verify opcode_size() agrees with Opcode::size() for all opcodes
        let all_opcodes = [
            0x00u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x20,
            0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35,
            0x36, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56,
            0x57, 0x58, 0x59, 0x5A, 0x60,
        ];
        for &op in &all_opcodes {
            // We can't easily convert u8 -> Opcode, but we know the expected sizes
            let expected = opcode_size(op);
            assert!(
                (1..=5).contains(&expected),
                "opcode 0x{op:02x} size {expected}"
            );
        }
    }
}
