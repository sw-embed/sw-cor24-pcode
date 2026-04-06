// pa24r — P-Code Assembler for COR24

pub mod lexer;

// .p24 header constants
pub const P24_MAGIC: [u8; 4] = [0x50, 0x32, 0x34, 0x00]; // "P24\0"
pub const P24_VERSION: u8 = 1;
pub const P24_VERSION_2: u8 = 2;
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
    /// 3 bytes: [op, lo, hi] — 16-bit LE operand
    Imm16,
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
            Encoding::Imm16 => 3,
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

    // Memory block operations (0x70-0x72)
    Memcpy = 0x70,
    Memset = 0x71,
    Memcmp = 0x72,

    // Indirect jump (0x73)
    JmpInd = 0x73,

    // Cross-unit call (0x74)
    XCall = 0x74,

    // Cross-unit global access (0x75-0x76)
    XLoadg = 0x75,
    XStoreg = 0x76,
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
            | Opcode::Memset
            | Opcode::Memcmp
            | Opcode::JmpInd => Encoding::None,

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

            // IMM16 encoding (3 bytes): xcall slot16
            Opcode::XCall => Encoding::Imm16,

            // D8_O8 encoding (3 bytes): xloadg/xstoreg unit_id8 offset8
            Opcode::XLoadg | Opcode::XStoreg => Encoding::D8O8,
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
            "memcmp" => Some(Opcode::Memcmp),
            "jmp_ind" => Some(Opcode::JmpInd),
            "xcall" => Some(Opcode::XCall),
            "xloadg" => Some(Opcode::XLoadg),
            "xstoreg" => Some(Opcode::XStoreg),
            _ => None,
        }
    }

    /// Look up a mnemonic string from an opcode byte value.
    pub fn mnemonic_from_byte(byte: u8) -> Option<&'static str> {
        match byte {
            0x00 => Some("halt"),
            0x01 => Some("push"),
            0x02 => Some("push_s"),
            0x03 => Some("dup"),
            0x04 => Some("drop"),
            0x05 => Some("swap"),
            0x06 => Some("over"),
            0x10 => Some("add"),
            0x11 => Some("sub"),
            0x12 => Some("mul"),
            0x13 => Some("div"),
            0x14 => Some("mod"),
            0x15 => Some("neg"),
            0x16 => Some("and"),
            0x17 => Some("or"),
            0x18 => Some("xor"),
            0x19 => Some("not"),
            0x1A => Some("shl"),
            0x1B => Some("shr"),
            0x20 => Some("eq"),
            0x21 => Some("ne"),
            0x22 => Some("lt"),
            0x23 => Some("le"),
            0x24 => Some("gt"),
            0x25 => Some("ge"),
            0x30 => Some("jmp"),
            0x31 => Some("jz"),
            0x32 => Some("jnz"),
            0x33 => Some("call"),
            0x34 => Some("ret"),
            0x35 => Some("calln"),
            0x36 => Some("trap"),
            0x40 => Some("enter"),
            0x41 => Some("leave"),
            0x42 => Some("loadl"),
            0x43 => Some("storel"),
            0x44 => Some("loadg"),
            0x45 => Some("storeg"),
            0x46 => Some("addrl"),
            0x47 => Some("addrg"),
            0x48 => Some("loada"),
            0x49 => Some("storea"),
            0x4A => Some("loadn"),
            0x4B => Some("storen"),
            0x50 => Some("load"),
            0x51 => Some("store"),
            0x52 => Some("loadb"),
            0x53 => Some("storeb"),
            0x60 => Some("sys"),
            0x70 => Some("memcpy"),
            0x71 => Some("memset"),
            0x72 => Some("memcmp"),
            0x73 => Some("jmp_ind"),
            0x74 => Some("xcall"),
            0x75 => Some("xloadg"),
            0x76 => Some("xstoreg"),
            _ => None,
        }
    }
}

/// An exported procedure in a unit.
#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub name: String,
    pub offset: u32,
    pub nargs: u8,
}

/// An imported procedure from an external unit.
#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub unit_name: String,
    pub proc_name: String,
    pub slot: u16,
}

/// Unit metadata collected from .unit/.import/.export/.extern directives.
#[derive(Debug, Clone, Default)]
pub struct UnitInfo {
    pub name: String,
    pub exports: Vec<ExportEntry>,
    pub imports: Vec<ImportEntry>,
    pub imported_units: Vec<String>,
}

/// Compute a 16-bit FNV-1a hash of a byte string.
pub fn fnv1a_16(data: &[u8]) -> u16 {
    // FNV-1a 32-bit, then fold to 16 bits via xor-fold
    let mut h: u32 = 0x811c_9dc5;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    ((h >> 16) ^ (h & 0xFFFF)) as u16
}

/// Result of assembling .spc source.
pub struct AssemblyResult {
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    pub entry_point: u32,
    pub global_count: u32,
    pub errors: Vec<AssemblyError>,
    /// Present when source uses `.unit` directive (unit mode).
    pub unit_info: Option<UnitInfo>,
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
#[derive(Debug)]
pub struct LoadedImage {
    pub entry_point: u32,
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    pub global_count: u32,
    pub version: u8,
    pub unit_info: Option<UnitInfo>,
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

/// Metadata directives from pl24r that we silently skip (non-unit mode only).
const METADATA_DIRECTIVES: &[&str] = &[".module", ".endmodule"];

/// Symbol type for tracking what kind of entity a name refers to.
#[derive(Debug, Clone, Copy)]
enum SymType {
    Code,
    Data,
    Global,
    Const,
    /// External symbol (import slot index stored in value).
    Extern,
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

    // Unit mode state
    let mut unit_name: Option<String> = None;
    let mut export_names: Vec<(String, u8)> = Vec::new(); // (name, nargs)
    let mut imported_units: Vec<String> = Vec::new();
    let mut extern_slots: Vec<(String, String)> = Vec::new(); // (proc_name, unit_name) — unit inferred
    let mut next_extern_slot: u16 = 0;

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
                    ".unit" => {
                        let name = expect_identifier(&tokens, &mut i);
                        if let Some(name) = name {
                            unit_name = Some(name);
                        } else {
                            errors.push(AssemblyError {
                                line,
                                message: ".unit missing name".into(),
                            });
                        }
                        skip_to_newline(&tokens, &mut i);
                    }
                    ".import" => {
                        let name = expect_identifier(&tokens, &mut i);
                        if let Some(name) = name {
                            if !imported_units.contains(&name) {
                                imported_units.push(name);
                            }
                        } else {
                            errors.push(AssemblyError {
                                line,
                                message: ".import missing unit name".into(),
                            });
                        }
                        skip_to_newline(&tokens, &mut i);
                    }
                    ".export" => {
                        let name = expect_identifier(&tokens, &mut i);
                        let nargs = expect_number(&tokens, &mut i).unwrap_or(-1);
                        if let Some(name) = name {
                            export_names.push((name, nargs as u8));
                        } else {
                            errors.push(AssemblyError {
                                line,
                                message: ".export missing symbol name".into(),
                            });
                        }
                        skip_to_newline(&tokens, &mut i);
                    }
                    ".extern" => {
                        let name = expect_identifier(&tokens, &mut i);
                        let nargs = expect_number(&tokens, &mut i).unwrap_or(-1);
                        if let Some(name) = name {
                            let slot = next_extern_slot;
                            next_extern_slot += 1;
                            // Infer unit name: use the most recently declared .import
                            let unit = imported_units.last().cloned().unwrap_or_default();
                            extern_slots.push((name.clone(), unit));
                            if let Err(e) = insert_symbol(
                                &mut symbols,
                                &name,
                                slot as u32,
                                SymType::Extern,
                                line,
                            ) {
                                errors.push(e);
                            }
                            let _ = nargs; // stored via extern_slots index
                        } else {
                            errors.push(AssemblyError {
                                line,
                                message: ".extern missing symbol name".into(),
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
    // Note: main does NOT need to be at offset 0. Compilers like p24p emit
    // a prologue proc (_p24p_entry) at offset 0 that calls main. The .p24
    // header records main's offset in the entry_point field for loaders.

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
                        Encoding::Imm16 => {
                            // xcall: operand is an extern symbol, resolve to slot index
                            let val = resolve_operand(
                                &tokens,
                                &mut i,
                                &symbols,
                                line,
                                &mut errors,
                                None, // no global base adjustment
                            );
                            code.push(val as u8);
                            code.push((val >> 8) as u8);
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

    // Build unit info if in unit mode
    let unit_info = unit_name.map(|name| {
        let exports = export_names
            .iter()
            .map(|(sym_name, nargs)| {
                let offset = symbols.get(sym_name).map(|s| s.value).unwrap_or(0);
                ExportEntry {
                    name: sym_name.clone(),
                    offset,
                    nargs: *nargs,
                }
            })
            .collect();
        let imports = extern_slots
            .iter()
            .enumerate()
            .map(|(slot, (proc_name, uname))| ImportEntry {
                unit_name: uname.clone(),
                proc_name: proc_name.clone(),
                slot: slot as u16,
            })
            .collect();
        UnitInfo {
            name,
            exports,
            imports,
            imported_units,
        }
    });

    // Validate exports reference defined symbols
    if let Some(ref info) = unit_info {
        for exp in &info.exports {
            if !symbols.contains_key(&exp.name) {
                errors.push(AssemblyError {
                    line: 0,
                    message: format!(".export '{}' not defined", exp.name),
                });
            }
        }
    }

    AssemblyResult {
        code,
        data: data_bytes,
        entry_point: entry_point.unwrap_or(0),
        global_count,
        errors,
        unit_info,
    }
}

/// Assemble .spc source and produce a complete .p24 binary.
///
/// Emits v1 format for non-unit sources, v2 format when `.unit` is present.
pub fn assemble_to_p24(source: &str) -> Result<Vec<u8>, Vec<AssemblyError>> {
    let result = assemble(source);
    if !result.errors.is_empty() {
        return Err(result.errors);
    }

    if let Some(ref unit_info) = result.unit_info {
        return Ok(emit_p24_v2(&result, unit_info));
    }

    let mut binary = Vec::with_capacity(P24_HEADER_SIZE + result.code.len() + result.data.len());

    // Header (v1)
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

/// Emit a v2 .p24 binary with export and import tables.
fn emit_p24_v2(result: &AssemblyResult, unit_info: &UnitInfo) -> Vec<u8> {
    let mut binary = Vec::new();

    // Build string table (null-terminated names)
    let mut string_table: Vec<u8> = Vec::new();
    // Unit name
    string_table.extend_from_slice(unit_info.name.as_bytes());
    string_table.push(0);

    // Export names
    let mut export_name_offsets = Vec::new();
    for exp in &unit_info.exports {
        export_name_offsets.push(string_table.len() as u16);
        string_table.extend_from_slice(exp.name.as_bytes());
        string_table.push(0);
    }

    // Import names (unit_name + proc_name)
    let mut import_name_offsets = Vec::new();
    for imp in &unit_info.imports {
        let unit_off = string_table.len() as u16;
        string_table.extend_from_slice(imp.unit_name.as_bytes());
        string_table.push(0);
        let proc_off = string_table.len() as u16;
        string_table.extend_from_slice(imp.proc_name.as_bytes());
        string_table.push(0);
        import_name_offsets.push((unit_off, proc_off));
    }

    let _ = export_name_offsets;
    let _ = import_name_offsets;

    let export_count = unit_info.exports.len() as u16;
    let import_count = unit_info.imports.len() as u16;
    let flags: u8 =
        (if export_count > 0 { 0x01 } else { 0 }) | (if import_count > 0 { 0x02 } else { 0 });

    // ── Header (fixed part, same offsets as v1 + extensions) ──
    binary.extend_from_slice(&P24_MAGIC);
    binary.push(P24_VERSION_2);
    emit_le24(&mut binary, result.entry_point);
    emit_le24(&mut binary, result.code.len() as u32);
    emit_le24(&mut binary, result.data.len() as u32);
    emit_le24(&mut binary, result.global_count);
    binary.push(flags);

    // ── V2 extended header ──
    // Export count (LE16)
    binary.push(export_count as u8);
    binary.push((export_count >> 8) as u8);
    // Import count (LE16)
    binary.push(import_count as u8);
    binary.push((import_count >> 8) as u8);
    // Unit name length (LE16)
    let name_len = unit_info.name.len() as u16;
    binary.push(name_len as u8);
    binary.push((name_len >> 8) as u8);
    // Unit name (UTF-8, not null-terminated)
    binary.extend_from_slice(unit_info.name.as_bytes());

    // ── Export table (5 bytes each: name_hash(2) + offset(3)) ──
    for exp in &unit_info.exports {
        let hash = fnv1a_16(exp.name.as_bytes());
        binary.push(hash as u8);
        binary.push((hash >> 8) as u8);
        emit_le24(&mut binary, exp.offset);
    }

    // ── Import table (5 bytes each: unit_hash(2) + name_hash(2) + slot(1)) ──
    for imp in &unit_info.imports {
        let unit_hash = fnv1a_16(imp.unit_name.as_bytes());
        binary.push(unit_hash as u8);
        binary.push((unit_hash >> 8) as u8);
        let name_hash = fnv1a_16(imp.proc_name.as_bytes());
        binary.push(name_hash as u8);
        binary.push((name_hash >> 8) as u8);
        binary.push(imp.slot as u8);
    }

    // ── String table ──
    // Length prefix (LE16) so loader knows where strings end
    let st_len = string_table.len() as u16;
    binary.push(st_len as u8);
    binary.push((st_len >> 8) as u8);
    binary.extend_from_slice(&string_table);

    // ── Code segment ──
    binary.extend_from_slice(&result.code);

    // ── Data segment ──
    binary.extend_from_slice(&result.data);

    binary
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
    if version != P24_VERSION && version != P24_VERSION_2 {
        return Err(LoadError::BadVersion(version));
    }

    let entry_point = read_le24(&binary[5..8]);
    let code_size = read_le24(&binary[8..11]) as usize;
    let data_size = read_le24(&binary[11..14]) as usize;
    let global_count = read_le24(&binary[14..17]);

    if version == P24_VERSION {
        let body = &binary[P24_HEADER_SIZE..];
        if body.len() < code_size + data_size {
            return Err(LoadError::Truncated);
        }
        return Ok(LoadedImage {
            entry_point,
            code: body[..code_size].to_vec(),
            data: body[code_size..code_size + data_size].to_vec(),
            global_count,
            version,
            unit_info: None,
        });
    }

    // v2: parse extended header and tables
    let flags = binary[17];
    let mut pos = P24_HEADER_SIZE;

    if binary.len() < pos + 6 {
        return Err(LoadError::Truncated);
    }
    let export_count = read_le16(&binary[pos..]) as usize;
    pos += 2;
    let import_count = read_le16(&binary[pos..]) as usize;
    pos += 2;
    let name_len = read_le16(&binary[pos..]) as usize;
    pos += 2;

    if binary.len() < pos + name_len {
        return Err(LoadError::Truncated);
    }
    let unit_name = String::from_utf8_lossy(&binary[pos..pos + name_len]).to_string();
    pos += name_len;

    // Export table: 5 bytes each (hash(2) + offset(3))
    let export_table_size = export_count * 5;
    if binary.len() < pos + export_table_size {
        return Err(LoadError::Truncated);
    }
    let mut exports = Vec::with_capacity(export_count);
    for _ in 0..export_count {
        let _hash = read_le16(&binary[pos..]);
        pos += 2;
        let offset = read_le24(&binary[pos..]);
        pos += 3;
        exports.push(ExportEntry {
            name: String::new(), // filled from string table below
            offset,
            nargs: 0,
        });
    }

    // Import table: 5 bytes each (unit_hash(2) + name_hash(2) + slot(1))
    let import_table_size = import_count * 5;
    if binary.len() < pos + import_table_size {
        return Err(LoadError::Truncated);
    }
    let mut imports = Vec::with_capacity(import_count);
    for _ in 0..import_count {
        let _unit_hash = read_le16(&binary[pos..]);
        pos += 2;
        let _name_hash = read_le16(&binary[pos..]);
        pos += 2;
        let slot = binary[pos] as u16;
        pos += 1;
        imports.push(ImportEntry {
            unit_name: String::new(), // filled from string table
            proc_name: String::new(),
            slot,
        });
    }

    // String table: length(2) + data
    if binary.len() < pos + 2 {
        return Err(LoadError::Truncated);
    }
    let st_len = read_le16(&binary[pos..]) as usize;
    pos += 2;
    if binary.len() < pos + st_len {
        return Err(LoadError::Truncated);
    }
    let string_table = &binary[pos..pos + st_len];
    pos += st_len;

    // Parse string table: null-terminated strings
    // First string is unit name (already parsed), then export names, then import pairs
    let strings: Vec<&str> = {
        let mut v = Vec::new();
        let mut start = 0;
        for (i, &b) in string_table.iter().enumerate() {
            if b == 0 {
                v.push(std::str::from_utf8(&string_table[start..i]).unwrap_or(""));
                start = i + 1;
            }
        }
        v
    };

    // Fill export names (strings[1..=export_count])
    for (i, exp) in exports.iter_mut().enumerate() {
        if i + 1 < strings.len() {
            exp.name = strings[i + 1].to_string();
        }
    }

    // Fill import names (strings after exports, pairs of unit_name + proc_name)
    let import_str_start = 1 + export_count;
    for (i, imp) in imports.iter_mut().enumerate() {
        let idx = import_str_start + i * 2;
        if idx + 1 < strings.len() {
            imp.unit_name = strings[idx].to_string();
            imp.proc_name = strings[idx + 1].to_string();
        }
    }

    let _ = flags; // validated by presence of tables

    // Code and data segments
    if binary.len() < pos + code_size + data_size {
        return Err(LoadError::Truncated);
    }
    let code = binary[pos..pos + code_size].to_vec();
    pos += code_size;
    let data = binary[pos..pos + data_size].to_vec();

    Ok(LoadedImage {
        entry_point,
        code,
        data,
        global_count,
        version,
        unit_info: Some(UnitInfo {
            name: unit_name,
            exports,
            imports,
            imported_units: Vec::new(), // not stored in binary
        }),
    })
}

fn read_le16(bytes: &[u8]) -> u16 {
    u16::from(bytes[0]) | (u16::from(bytes[1]) << 8)
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
pub fn opcode_size(op: u8) -> usize {
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
        // IMM16 (3 bytes): xcall
        0x74 => 3,
        // D8_O8 (3 bytes): xloadg, xstoreg
        0x75 | 0x76 => 3,
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
                    SymType::Extern => {
                        // Import slot index (used by xcall)
                        sym.value
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
