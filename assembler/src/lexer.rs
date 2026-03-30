/// Token types produced by the .spc lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// End of line (tracks line boundaries for error reporting).
    Newline,
    /// An identifier: mnemonic, symbol name, or operand reference.
    Identifier(String),
    /// A decimal integer literal (may be negative).
    Number(i32),
    /// A directive: .proc, .end, .data, .global, .const, or metadata directives.
    Directive(String),
    /// A label definition (the name without trailing colon).
    Label(String),
    /// Comma separator (used in .data byte lists).
    Comma,
}

/// A token with its source line number (1-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub token: Token,
    pub line: usize,
}

/// Tokenize .spc source into a sequence of located tokens.
///
/// Handles semicolon comments (discards to end of line), negative numbers,
/// label definitions (identifier followed by colon), and all directive forms.
/// Unknown directives (metadata from pl24r) are still emitted as Directive
/// tokens so the parser can skip them.
pub fn tokenize(source: &str) -> Vec<Located> {
    let mut tokens = Vec::new();

    for (line_idx, raw_line) in source.lines().enumerate() {
        let line_num = line_idx + 1;

        // Strip comments: everything from `;` onward.
        let line = match raw_line.find(';') {
            Some(pos) => &raw_line[..pos],
            None => raw_line,
        };

        let chars = line.as_bytes();
        let mut pos = 0;

        while pos < chars.len() {
            let ch = chars[pos];

            // Skip whitespace (spaces and tabs).
            if ch == b' ' || ch == b'\t' {
                pos += 1;
                continue;
            }

            // Comma.
            if ch == b',' {
                tokens.push(Located {
                    token: Token::Comma,
                    line: line_num,
                });
                pos += 1;
                continue;
            }

            // Directive: starts with '.'.
            if ch == b'.' {
                let start = pos;
                pos += 1;
                while pos < chars.len()
                    && (chars[pos].is_ascii_alphanumeric() || chars[pos] == b'_')
                {
                    pos += 1;
                }
                let name = std::str::from_utf8(&chars[start..pos]).unwrap().to_string();
                tokens.push(Located {
                    token: Token::Directive(name),
                    line: line_num,
                });
                continue;
            }

            // Number: starts with digit, or '-' followed by digit.
            if ch.is_ascii_digit()
                || (ch == b'-' && pos + 1 < chars.len() && chars[pos + 1].is_ascii_digit())
            {
                let start = pos;
                if ch == b'-' {
                    pos += 1;
                }
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                let text = std::str::from_utf8(&chars[start..pos]).unwrap();
                let value: i32 = text.parse().unwrap();
                tokens.push(Located {
                    token: Token::Number(value),
                    line: line_num,
                });
                continue;
            }

            // Identifier or label: starts with alpha or '_'.
            if ch.is_ascii_alphabetic() || ch == b'_' {
                let start = pos;
                pos += 1;
                while pos < chars.len()
                    && (chars[pos].is_ascii_alphanumeric() || chars[pos] == b'_')
                {
                    pos += 1;
                }
                let text = std::str::from_utf8(&chars[start..pos]).unwrap();

                // If followed by ':', it's a label.
                if pos < chars.len() && chars[pos] == b':' {
                    tokens.push(Located {
                        token: Token::Label(text.to_string()),
                        line: line_num,
                    });
                    pos += 1; // consume the colon
                } else {
                    tokens.push(Located {
                        token: Token::Identifier(text.to_string()),
                        line: line_num,
                    });
                }
                continue;
            }

            // Unknown character — skip it.
            pos += 1;
        }

        // Emit a newline token to mark line boundaries.
        tokens.push(Located {
            token: Token::Newline,
            line: line_num,
        });
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn blank_lines() {
        let tokens = tokenize("\n\n");
        // "\n\n" splits into ["", ""] via .lines() → 2 newline tokens
        let newlines: Vec<_> = tokens
            .iter()
            .filter(|t| t.token == Token::Newline)
            .collect();
        assert_eq!(newlines.len(), 2);
    }

    #[test]
    fn comment_only_line() {
        let tokens = tokenize("; this is a comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, Token::Newline);
    }

    #[test]
    fn inline_comment() {
        let tokens = tokenize("push 42 ; push the answer");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 2);
        assert_eq!(non_nl[0].token, Token::Identifier("push".into()));
        assert_eq!(non_nl[1].token, Token::Number(42));
    }

    #[test]
    fn simple_instruction() {
        let tokens = tokenize("halt");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 1);
        assert_eq!(non_nl[0].token, Token::Identifier("halt".into()));
    }

    #[test]
    fn instruction_with_operand() {
        let tokens = tokenize("push 255");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 2);
        assert_eq!(non_nl[0].token, Token::Identifier("push".into()));
        assert_eq!(non_nl[1].token, Token::Number(255));
    }

    #[test]
    fn negative_number() {
        let tokens = tokenize("push -1");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 2);
        assert_eq!(non_nl[1].token, Token::Number(-1));
    }

    #[test]
    fn label_definition() {
        let tokens = tokenize("loop_start:");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 1);
        assert_eq!(non_nl[0].token, Token::Label("loop_start".into()));
    }

    #[test]
    fn label_with_instruction() {
        let tokens = tokenize("start: push 0");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 3);
        assert_eq!(non_nl[0].token, Token::Label("start".into()));
        assert_eq!(non_nl[1].token, Token::Identifier("push".into()));
        assert_eq!(non_nl[2].token, Token::Number(0));
    }

    #[test]
    fn proc_directive() {
        let tokens = tokenize(".proc main 0");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 3);
        assert_eq!(non_nl[0].token, Token::Directive(".proc".into()));
        assert_eq!(non_nl[1].token, Token::Identifier("main".into()));
        assert_eq!(non_nl[2].token, Token::Number(0));
    }

    #[test]
    fn end_directive() {
        let tokens = tokenize(".end");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 1);
        assert_eq!(non_nl[0].token, Token::Directive(".end".into()));
    }

    #[test]
    fn data_directive() {
        let tokens = tokenize(".data hello 72, 101, 108");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 7);
        assert_eq!(non_nl[0].token, Token::Directive(".data".into()));
        assert_eq!(non_nl[1].token, Token::Identifier("hello".into()));
        assert_eq!(non_nl[2].token, Token::Number(72));
        assert_eq!(non_nl[3].token, Token::Comma);
        assert_eq!(non_nl[4].token, Token::Number(101));
        assert_eq!(non_nl[5].token, Token::Comma);
        assert_eq!(non_nl[6].token, Token::Number(108));
    }

    #[test]
    fn global_directive() {
        let tokens = tokenize(".global count 1");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 3);
        assert_eq!(non_nl[0].token, Token::Directive(".global".into()));
        assert_eq!(non_nl[1].token, Token::Identifier("count".into()));
        assert_eq!(non_nl[2].token, Token::Number(1));
    }

    #[test]
    fn const_directive() {
        let tokens = tokenize(".const MAX 100");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 3);
        assert_eq!(non_nl[0].token, Token::Directive(".const".into()));
        assert_eq!(non_nl[1].token, Token::Identifier("MAX".into()));
        assert_eq!(non_nl[2].token, Token::Number(100));
    }

    #[test]
    fn metadata_directives_emitted() {
        // Metadata directives from pl24r should still be tokenized
        // (the parser will skip them).
        for dir in [".module", ".export", ".extern", ".endmodule"] {
            let input = format!("{dir} some_name");
            let tokens = tokenize(&input);
            let non_nl: Vec<_> = tokens
                .iter()
                .filter(|t| t.token != Token::Newline)
                .collect();
            assert_eq!(
                non_nl[0].token,
                Token::Directive(dir.into()),
                "metadata directive {dir} should be tokenized"
            );
        }
    }

    #[test]
    fn symbol_reference_as_operand() {
        let tokens = tokenize("call main");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 2);
        assert_eq!(non_nl[0].token, Token::Identifier("call".into()));
        assert_eq!(non_nl[1].token, Token::Identifier("main".into()));
    }

    #[test]
    fn line_numbers_tracked() {
        let tokens = tokenize("halt\npush 1\nadd");
        let idents: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.token, Token::Identifier(_)))
            .collect();
        assert_eq!(idents[0].line, 1); // halt
        assert_eq!(idents[1].line, 2); // push
        assert_eq!(idents[2].line, 3); // add
    }

    #[test]
    fn tabs_as_whitespace() {
        let tokens = tokenize("\t\tpush\t42");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 2);
        assert_eq!(non_nl[0].token, Token::Identifier("push".into()));
        assert_eq!(non_nl[1].token, Token::Number(42));
    }

    #[test]
    fn underscore_identifiers() {
        let tokens = tokenize("_foo_bar");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl.len(), 1);
        assert_eq!(non_nl[0].token, Token::Identifier("_foo_bar".into()));
    }

    #[test]
    fn multiline_program() {
        let source = "\
.proc main 0
    push 1
    push 2
    add
    sys 1
    halt
.end
";
        let tokens = tokenize(source);
        let directives: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.token, Token::Directive(_)))
            .collect();
        assert_eq!(directives.len(), 2); // .proc, .end
        let numbers: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.token, Token::Number(_)))
            .collect();
        assert_eq!(numbers.len(), 4); // 0, 1, 2, 1
    }

    #[test]
    fn large_negative_number() {
        let tokens = tokenize("push -8388608");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl[1].token, Token::Number(-8_388_608));
    }

    #[test]
    fn zero_value() {
        let tokens = tokenize("push 0");
        let non_nl: Vec<_> = tokens
            .iter()
            .filter(|t| t.token != Token::Newline)
            .collect();
        assert_eq!(non_nl[1].token, Token::Number(0));
    }
}
