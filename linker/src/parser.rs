//! .spc file parser for the pl24r COR24 p-code linker.
//!
//! Parses symbolic p-code assembler text files into a structured AST,
//! including module metadata directives for linking. Language-agnostic:
//! handles .spc files from any compiler targeting the COR24 p-code VM.

/// A parsed .spc module.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module name (from `.module` directive or inferred from filename).
    pub name: String,
    /// Symbols explicitly exported via `.export`.
    pub exports: Vec<String>,
    /// Symbols declared as external dependencies via `.extern`.
    pub externs: Vec<String>,
    /// Whether this module had explicit `.module`/`.endmodule` metadata.
    pub has_metadata: bool,
    /// The items (procs, globals, data, constants, comments) in order.
    pub items: Vec<Item>,
}

/// A top-level item in a .spc file.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Proc(Proc),
    Global(Global),
    Data(Data),
    Const(Const),
    Comment(String),
}

/// A procedure definition (`.proc NAME nlocals` ... `.end`).
#[derive(Debug, Clone, PartialEq)]
pub struct Proc {
    pub name: String,
    pub nlocals: u32,
    pub body: Vec<Statement>,
}

/// A statement inside a procedure body.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Label(String),
    Instruction(Instruction),
    Comment(String),
    Blank,
}

/// A single p-code instruction with optional operand.
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub mnemonic: String,
    pub operand: Option<String>,
    pub comment: Option<String>,
}

/// A global variable declaration (`.global NAME nwords`).
#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    pub name: String,
    pub nwords: u32,
}

/// A data declaration (`.data NAME byte,byte,...`).
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// A constant declaration (`.const NAME value`).
#[derive(Debug, Clone, PartialEq)]
pub struct Const {
    pub name: String,
    pub value: String,
}

/// Parse error with line number and description.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a .spc file's text content into a `Module`.
///
/// `filename` is used to infer the module name when no `.module` directive is present.
pub fn parse(source: &str, filename: &str) -> Result<Module, ParseError> {
    let mut parser = Parser::new(source, filename);
    parser.parse()
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    filename: &'a str,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, filename: &'a str) -> Self {
        Self {
            lines: source.lines().collect(),
            pos: 0,
            filename,
        }
    }

    fn parse(&mut self) -> Result<Module, ParseError> {
        let mut module_name: Option<String> = None;
        let mut exports = Vec::new();
        let mut externs = Vec::new();
        let mut items = Vec::new();
        let mut has_metadata = false;
        let mut in_module = false;

        while self.pos < self.lines.len() {
            let line_num = self.pos + 1;
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            if trimmed.is_empty() {
                self.pos += 1;
                continue;
            }

            if trimmed.starts_with(';') {
                items.push(Item::Comment(trimmed.to_string()));
                self.pos += 1;
                continue;
            }

            if let Some(directive) = trimmed.strip_prefix('.') {
                let parts: Vec<&str> = directive.splitn(2, char::is_whitespace).collect();
                let dir_name = parts[0];
                let dir_arg = parts.get(1).map(|s| s.trim());

                match dir_name {
                    "module" => {
                        let name = dir_arg.ok_or_else(|| ParseError {
                            line: line_num,
                            message: ".module requires a name".to_string(),
                        })?;
                        // Strip inline comment from module name
                        let name = name.split(';').next().unwrap().trim();
                        module_name = Some(name.to_string());
                        has_metadata = true;
                        in_module = true;
                        self.pos += 1;
                    }
                    "endmodule" => {
                        if !in_module {
                            return Err(ParseError {
                                line: line_num,
                                message: ".endmodule without matching .module".to_string(),
                            });
                        }
                        in_module = false;
                        self.pos += 1;
                    }
                    "export" => {
                        let sym = dir_arg.ok_or_else(|| ParseError {
                            line: line_num,
                            message: ".export requires a symbol name".to_string(),
                        })?;
                        let sym = sym.split(';').next().unwrap().trim();
                        exports.push(sym.to_string());
                        self.pos += 1;
                    }
                    "extern" => {
                        let sym = dir_arg.ok_or_else(|| ParseError {
                            line: line_num,
                            message: ".extern requires a symbol name".to_string(),
                        })?;
                        let sym = sym.split(';').next().unwrap().trim();
                        externs.push(sym.to_string());
                        self.pos += 1;
                    }
                    "proc" => {
                        items.push(Item::Proc(self.parse_proc()?));
                    }
                    "global" => {
                        items.push(Item::Global(self.parse_global()?));
                        self.pos += 1;
                    }
                    "data" => {
                        items.push(Item::Data(self.parse_data()?));
                        self.pos += 1;
                    }
                    "const" => {
                        items.push(Item::Const(self.parse_const()?));
                        self.pos += 1;
                    }
                    _ => {
                        return Err(ParseError {
                            line: line_num,
                            message: format!("unknown directive: .{dir_name}"),
                        });
                    }
                }
            } else {
                return Err(ParseError {
                    line: line_num,
                    message: format!("unexpected line: {trimmed}"),
                });
            }
        }

        if in_module {
            return Err(ParseError {
                line: self.lines.len(),
                message: ".module without matching .endmodule".to_string(),
            });
        }

        let name = module_name.unwrap_or_else(|| module_name_from_filename(self.filename));

        Ok(Module {
            name,
            exports,
            externs,
            has_metadata,
            items,
        })
    }

    fn parse_proc(&mut self) -> Result<Proc, ParseError> {
        let line_num = self.pos + 1;
        let line = self.lines[self.pos].trim();

        // .proc NAME nlocals
        let rest = line.strip_prefix(".proc").unwrap().trim();
        // Strip inline comment
        let rest = rest.split(';').next().unwrap().trim();
        let parts: Vec<&str> = rest.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(ParseError {
                line: line_num,
                message: ".proc requires NAME and nlocals".to_string(),
            });
        }

        let name = parts[0].to_string();
        let nlocals: u32 = parts[1].parse().map_err(|_| ParseError {
            line: line_num,
            message: format!("invalid nlocals: {}", parts[1]),
        })?;

        self.pos += 1;
        let mut body = Vec::new();

        while self.pos < self.lines.len() {
            let bline = self.lines[self.pos].trim();

            if bline == ".end" {
                self.pos += 1;
                return Ok(Proc {
                    name,
                    nlocals,
                    body,
                });
            }

            if bline.is_empty() {
                body.push(Statement::Blank);
                self.pos += 1;
                continue;
            }

            if bline.starts_with(';') {
                body.push(Statement::Comment(bline.to_string()));
                self.pos += 1;
                continue;
            }

            // Label: ends with ':'
            if let Some(label) = bline.strip_suffix(':')
                && !label.contains(' ')
                && !label.contains('\t')
            {
                body.push(Statement::Label(label.to_string()));
                self.pos += 1;
                continue;
            }

            // Instruction
            body.push(Statement::Instruction(self.parse_instruction(bline)?));
            self.pos += 1;
        }

        Err(ParseError {
            line: line_num,
            message: format!(".proc {name} missing .end"),
        })
    }

    fn parse_instruction(&self, line: &str) -> Result<Instruction, ParseError> {
        // Split off inline comment
        let (code, comment) = if let Some(idx) = line.find(';') {
            let (c, rest) = line.split_at(idx);
            (c.trim(), Some(rest.to_string()))
        } else {
            (line.trim(), None)
        };

        let parts: Vec<&str> = code.splitn(2, char::is_whitespace).collect();
        let mnemonic = parts[0].to_string();
        let operand = parts
            .get(1)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Ok(Instruction {
            mnemonic,
            operand,
            comment,
        })
    }

    fn parse_global(&self) -> Result<Global, ParseError> {
        let line_num = self.pos + 1;
        let line = self.lines[self.pos].trim();

        let rest = line.strip_prefix(".global").unwrap().trim();
        let rest = rest.split(';').next().unwrap().trim();
        let parts: Vec<&str> = rest.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(ParseError {
                line: line_num,
                message: ".global requires NAME and nwords".to_string(),
            });
        }

        let name = parts[0].to_string();
        let nwords: u32 = parts[1].parse().map_err(|_| ParseError {
            line: line_num,
            message: format!("invalid nwords: {}", parts[1]),
        })?;

        Ok(Global { name, nwords })
    }

    fn parse_data(&self) -> Result<Data, ParseError> {
        let line_num = self.pos + 1;
        let line = self.lines[self.pos].trim();

        let rest = line.strip_prefix(".data").unwrap().trim();
        let rest = rest.split(';').next().unwrap().trim();
        let parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();

        if parts.len() < 2 {
            return Err(ParseError {
                line: line_num,
                message: ".data requires NAME and byte values".to_string(),
            });
        }

        let name = parts[0].to_string();
        let bytes: Result<Vec<u8>, _> = parts[1]
            .split(',')
            .map(|b| {
                b.trim().parse::<u8>().map_err(|_| ParseError {
                    line: line_num,
                    message: format!("invalid byte value: {}", b.trim()),
                })
            })
            .collect();

        Ok(Data {
            name,
            bytes: bytes?,
        })
    }

    fn parse_const(&self) -> Result<Const, ParseError> {
        let line_num = self.pos + 1;
        let line = self.lines[self.pos].trim();

        let rest = line.strip_prefix(".const").unwrap().trim();
        let rest = rest.split(';').next().unwrap().trim();
        let parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();

        if parts.len() < 2 {
            return Err(ParseError {
                line: line_num,
                message: ".const requires NAME and value".to_string(),
            });
        }

        Ok(Const {
            name: parts[0].to_string(),
            value: parts[1].trim().to_string(),
        })
    }
}

/// Derive module name from filename: strip path and extension.
fn module_name_from_filename(filename: &str) -> String {
    let base = filename.rsplit('/').next().unwrap_or(filename);
    base.strip_suffix(".spc").unwrap_or(base).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comment() {
        let src = "; this is a comment\n";
        let m = parse(src, "test.spc").unwrap();
        assert_eq!(m.items.len(), 1);
        assert_eq!(m.items[0], Item::Comment("; this is a comment".to_string()));
    }

    #[test]
    fn test_parse_global() {
        let src = ".global count 1\n";
        let m = parse(src, "test.spc").unwrap();
        assert_eq!(m.items.len(), 1);
        assert_eq!(
            m.items[0],
            Item::Global(Global {
                name: "count".to_string(),
                nwords: 1
            })
        );
    }

    #[test]
    fn test_parse_data() {
        let src = ".data msg 72, 101, 108, 108, 111, 10, 0\n";
        let m = parse(src, "test.spc").unwrap();
        assert_eq!(m.items.len(), 1);
        match &m.items[0] {
            Item::Data(d) => {
                assert_eq!(d.name, "msg");
                assert_eq!(d.bytes, vec![72, 101, 108, 108, 111, 10, 0]);
            }
            _ => panic!("expected Data"),
        }
    }

    #[test]
    fn test_parse_const() {
        let src = ".const MAX 255\n";
        let m = parse(src, "test.spc").unwrap();
        assert_eq!(m.items.len(), 1);
        assert_eq!(
            m.items[0],
            Item::Const(Const {
                name: "MAX".to_string(),
                value: "255".to_string()
            })
        );
    }

    #[test]
    fn test_parse_simple_proc() {
        let src = "\
.proc main 0
    push 42
    halt
.end
";
        let m = parse(src, "test.spc").unwrap();
        assert_eq!(m.items.len(), 1);
        match &m.items[0] {
            Item::Proc(p) => {
                assert_eq!(p.name, "main");
                assert_eq!(p.nlocals, 0);
                assert_eq!(p.body.len(), 2);
                assert_eq!(
                    p.body[0],
                    Statement::Instruction(Instruction {
                        mnemonic: "push".to_string(),
                        operand: Some("42".to_string()),
                        comment: None,
                    })
                );
                assert_eq!(
                    p.body[1],
                    Statement::Instruction(Instruction {
                        mnemonic: "halt".to_string(),
                        operand: None,
                        comment: None,
                    })
                );
            }
            _ => panic!("expected Proc"),
        }
    }

    #[test]
    fn test_parse_proc_with_labels() {
        let src = "\
.proc puts 1
    loada 0
    storel 0
loop:
    loadl 0
    loadb
    dup
    jz done
    sys 1
    jmp loop
done:
    drop
    ret 1
.end
";
        let m = parse(src, "test.spc").unwrap();
        match &m.items[0] {
            Item::Proc(p) => {
                assert_eq!(p.name, "puts");
                assert_eq!(p.nlocals, 1);
                // Check that labels are parsed
                assert!(
                    p.body
                        .iter()
                        .any(|s| *s == Statement::Label("loop".to_string()))
                );
                assert!(
                    p.body
                        .iter()
                        .any(|s| *s == Statement::Label("done".to_string()))
                );
            }
            _ => panic!("expected Proc"),
        }
    }

    #[test]
    fn test_parse_instruction_with_comment() {
        let src = "\
.proc test 0
    push 45              ; '-'
    sys 1                ; PUTC
.end
";
        let m = parse(src, "test.spc").unwrap();
        match &m.items[0] {
            Item::Proc(p) => match &p.body[0] {
                Statement::Instruction(i) => {
                    assert_eq!(i.mnemonic, "push");
                    assert_eq!(i.operand.as_deref(), Some("45"));
                    assert!(i.comment.as_ref().unwrap().contains("'-'"));
                }
                _ => panic!("expected instruction"),
            },
            _ => panic!("expected Proc"),
        }
    }

    #[test]
    fn test_parse_hello_spc() {
        let src = "\
; hello.spc — Hello World for pv24a P-Code VM
;
; Expected output: Hello\\n

.data msg 72, 101, 108, 108, 111, 10, 0

.proc main 0
    push msg
    call puts
    halt
.end

.proc puts 1
    loada 0
    storel 0
loop:
    loadl 0
    loadb
    dup
    jz done
    sys 1
    loadl 0
    push 1
    add
    storel 0
    jmp loop
done:
    drop
    ret 1
.end
";
        let m = parse(src, "hello.spc").unwrap();
        assert_eq!(m.name, "hello");
        assert!(!m.has_metadata);
        assert!(m.exports.is_empty());
        assert!(m.externs.is_empty());

        // Should have: 3 comments, 1 data, 2 procs = 6 items
        assert_eq!(m.items.len(), 6);

        // Verify data
        match &m.items[3] {
            Item::Data(d) => {
                assert_eq!(d.name, "msg");
                assert_eq!(d.bytes, vec![72, 101, 108, 108, 111, 10, 0]);
            }
            _ => panic!("expected Data at index 3"),
        }

        // Verify main proc
        match &m.items[4] {
            Item::Proc(p) => {
                assert_eq!(p.name, "main");
                assert_eq!(p.nlocals, 0);
            }
            _ => panic!("expected Proc at index 4"),
        }
    }

    #[test]
    fn test_parse_module_metadata() {
        let src = "\
.module runtime
.export _p24p_write_int
.export _p24p_write_ln
.extern some_helper

.proc _p24p_write_int 1
    enter 1
    loada 0
    halt
.end

.proc _p24p_write_ln 0
    push 10
    sys 1
    ret 0
.end

.endmodule
";
        let m = parse(src, "runtime.spc").unwrap();
        assert_eq!(m.name, "runtime");
        assert!(m.has_metadata);
        assert_eq!(m.exports, vec!["_p24p_write_int", "_p24p_write_ln"]);
        assert_eq!(m.externs, vec!["some_helper"]);
        assert_eq!(m.items.len(), 2);
    }

    #[test]
    fn test_module_name_from_filename() {
        assert_eq!(module_name_from_filename("hello.spc"), "hello");
        assert_eq!(module_name_from_filename("path/to/runtime.spc"), "runtime");
        assert_eq!(module_name_from_filename("noext"), "noext");
    }

    #[test]
    fn test_export_all_fallback() {
        let src = "\
.global x 1
.proc foo 0
    halt
.end
";
        let m = parse(src, "mylib.spc").unwrap();
        assert!(!m.has_metadata);
        // No explicit exports — linker will use export-all fallback
        assert!(m.exports.is_empty());
        assert_eq!(m.name, "mylib");
    }

    #[test]
    fn test_error_unmatched_endmodule() {
        let src = ".endmodule\n";
        let err = parse(src, "test.spc").unwrap_err();
        assert_eq!(err.line, 1);
        assert!(err.message.contains("without matching .module"));
    }

    #[test]
    fn test_error_missing_endmodule() {
        let src = ".module foo\n";
        let err = parse(src, "test.spc").unwrap_err();
        assert!(err.message.contains("without matching .endmodule"));
    }

    #[test]
    fn test_error_missing_proc_end() {
        let src = "\
.proc broken 0
    push 1
";
        let err = parse(src, "test.spc").unwrap_err();
        assert!(err.message.contains("missing .end"));
    }

    #[test]
    fn test_parse_runtime_spc() {
        let src = "\
; pr24p — Pascal Runtime Library
; Phase 0: Hand-written .spc stubs

.proc _p24p_write_int 1
    enter 1
    loada 0
    dup
    push 0
    lt
    jz positive
    push 45
    sys 1
    neg
positive:
    storel 0
    push 0
extract:
    loadl 0
    push 10
    mod
    push 48
    add
    loadl 0
    push 10
    div
    storel 0
    loadl 0
    jnz extract
print:
    dup
    jz done
    sys 1
    jmp print
done:
    drop
    leave
    ret 1
.end
";
        let m = parse(src, "runtime.spc").unwrap();
        assert_eq!(m.name, "runtime");
        assert_eq!(m.items.len(), 3); // 2 comments + 1 proc
        match &m.items[2] {
            Item::Proc(p) => {
                assert_eq!(p.name, "_p24p_write_int");
                assert_eq!(p.nlocals, 1);
                // Should have labels: positive, extract, print, done
                let labels: Vec<_> = p
                    .body
                    .iter()
                    .filter_map(|s| match s {
                        Statement::Label(l) => Some(l.as_str()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(labels, vec!["positive", "extract", "print", "done"]);
            }
            _ => panic!("expected Proc"),
        }
    }

    #[test]
    fn test_parse_blank_lines_in_proc() {
        let src = "\
.proc test 0
    push 1

    push 2
.end
";
        let m = parse(src, "test.spc").unwrap();
        match &m.items[0] {
            Item::Proc(p) => {
                assert_eq!(p.body.len(), 3); // push, blank, push
                assert_eq!(p.body[1], Statement::Blank);
            }
            _ => panic!("expected Proc"),
        }
    }

    #[test]
    fn test_parse_comment_in_proc() {
        let src = "\
.proc test 0
    ; setup
    push 1
    ; done
    halt
.end
";
        let m = parse(src, "test.spc").unwrap();
        match &m.items[0] {
            Item::Proc(p) => {
                assert_eq!(p.body[0], Statement::Comment("; setup".to_string()));
                assert_eq!(p.body[2], Statement::Comment("; done".to_string()));
            }
            _ => panic!("expected Proc"),
        }
    }
}
