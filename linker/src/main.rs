use std::fs;
use std::io::{self, Write};
use std::process;

mod linker;
mod parser;
mod symbols;

struct Args {
    inputs: Vec<String>,
    output: Option<String>,
    verbose: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut inputs = Vec::new();
    let mut output = None;
    let mut verbose = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                if i >= args.len() {
                    return Err("-o requires an output path".to_string());
                }
                output = Some(args[i].clone());
            }
            "--verbose" | "-v" => {
                verbose = true;
            }
            "--help" | "-h" => {
                return Err(String::new()); // triggers usage
            }
            arg if arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            }
            _ => {
                inputs.push(args[i].clone());
            }
        }
        i += 1;
    }

    if inputs.is_empty() {
        return Err(String::new());
    }

    Ok(Args {
        inputs,
        output,
        verbose,
    })
}

fn usage() {
    eprintln!("pl24r — COR24 p-code linker");
    eprintln!();
    eprintln!("Usage: pl24r [OPTIONS] <file.spc>...");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -o <path>    Write output to file (default: stdout)");
    eprintln!("  -v, --verbose  Print diagnostics to stderr");
    eprintln!("  -h, --help     Show this help");
}

fn run() -> Result<(), String> {
    let args = parse_args().inspect_err(|_| {
        usage();
    })?;

    // Parse all input files.
    let mut modules = Vec::new();
    for path in &args.inputs {
        let source = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let module = parser::parse(&source, path).map_err(|e| format!("{path}: {e}"))?;
        if args.verbose {
            eprintln!(
                "[pl24r] parsed '{}': module='{}', {} items, {} exports, {} externs, metadata={}",
                path,
                module.name,
                module.items.len(),
                module.exports.len(),
                module.externs.len(),
                module.has_metadata,
            );
        }
        modules.push(module);
    }

    // Build symbol table and validate.
    let symbol_table = symbols::build_symbol_table(&modules).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| format!("error: {e}")).collect();
        msgs.join("\n")
    })?;

    if args.verbose {
        eprintln!(
            "[pl24r] symbol table: {} exports",
            symbol_table.exports.len()
        );
        let mut export_names: Vec<&String> = symbol_table.exports.keys().collect();
        export_names.sort();
        for name in &export_names {
            let sym = &symbol_table.exports[*name];
            eprintln!("  {} ({}) from '{}'", name, sym.kind, sym.module);
        }
    }

    for w in &symbol_table.warnings {
        eprintln!("warning: {w}");
    }

    // Link modules.
    let linked = linker::link(&modules).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| format!("error: {e}")).collect();
        msgs.join("\n")
    })?;

    if args.verbose {
        eprintln!(
            "[pl24r] linked: {} procs, {} globals, {} data, {} consts",
            linked.procs.len(),
            linked.globals.len(),
            linked.data.len(),
            linked.consts.len(),
        );
    }

    // Emit output.
    let output_text = linker::emit(&linked);

    match args.output {
        Some(ref path) => {
            fs::write(path, &output_text).map_err(|e| format!("{path}: {e}"))?;
            if args.verbose {
                eprintln!("[pl24r] wrote {path}");
            }
        }
        None => {
            io::stdout()
                .write_all(output_text.as_bytes())
                .map_err(|e| format!("stdout: {e}"))?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        if !e.is_empty() {
            eprintln!("{e}");
        }
        process::exit(1);
    }
}
