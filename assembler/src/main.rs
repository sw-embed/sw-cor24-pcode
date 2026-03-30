use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("pa24r — P-Code Assembler for COR24");
        eprintln!();
        eprintln!("Usage: pa24r [OPTIONS] <input.spc> -o <output.p24>");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -o, --output <file>    Output .p24 file (required)");
        eprintln!("  -v, --verbose          Print assembly stats");
        eprintln!("  --dump                 Hex dump of output");
        eprintln!("  -h, --help             Show this help");
        process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    let mut input_file = None;
    let mut output_file = None;
    let mut verbose = false;
    let mut dump = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: -o requires an argument");
                    process::exit(1);
                }
                output_file = Some(args[i].clone());
            }
            "-v" | "--verbose" => verbose = true,
            "--dump" => dump = true,
            arg if !arg.starts_with('-') => {
                input_file = Some(arg.to_string());
            }
            other => {
                eprintln!("error: unknown option: {other}");
                process::exit(1);
            }
        }
        i += 1;
    }

    let input_path = match input_file {
        Some(p) => p,
        None => {
            eprintln!("error: no input file specified");
            process::exit(1);
        }
    };

    let output_path = match output_file {
        Some(p) => p,
        None => {
            eprintln!("error: -o <output.p24> is required");
            process::exit(1);
        }
    };

    let source = match std::fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {input_path}: {e}");
            process::exit(1);
        }
    };

    let result = pa24r::assemble(&source);

    if !result.errors.is_empty() {
        for err in &result.errors {
            eprintln!("{input_path}:{err}");
        }
        process::exit(1);
    }

    if verbose {
        eprintln!("code:    {} bytes", result.code.len());
        eprintln!("data:    {} bytes", result.data.len());
        eprintln!("globals: {} words", result.global_count);
        eprintln!("entry:   0x{:06x}", result.entry_point);
    }

    match pa24r::assemble_to_p24(&source) {
        Ok(binary) => {
            if dump {
                hex_dump(&binary);
            }
            if let Err(e) = std::fs::write(&output_path, &binary) {
                eprintln!("error: cannot write {output_path}: {e}");
                process::exit(1);
            }
            if verbose {
                eprintln!("wrote {} bytes to {output_path}", binary.len());
            }
        }
        Err(errors) => {
            for err in &errors {
                eprintln!("{input_path}:{err}");
            }
            process::exit(1);
        }
    }
}

fn hex_dump(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        eprint!("{:06x}  ", i * 16);
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                eprint!(" ");
            }
            eprint!("{byte:02x} ");
        }
        // Pad if less than 16 bytes
        for j in chunk.len()..16 {
            if j == 8 {
                eprint!(" ");
            }
            eprint!("   ");
        }
        eprint!(" |");
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                eprint!("{}", *byte as char);
            } else {
                eprint!(".");
            }
        }
        eprintln!("|");
    }
}
