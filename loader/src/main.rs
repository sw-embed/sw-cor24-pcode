// p24-load — CLI for linking multiple .p24 units into a .p24m image

use p24_load::LinkOptions;
use pa24r::load_p24;
use std::{fs, process};

fn parse_addr(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "Usage: p24-load <unit1.p24> [unit2.p24 ...] -o <output.p24m> [--load-addr <addr>]"
        );
        eprintln!();
        eprintln!("Links multiple .p24 unit files into a single .p24m multi-unit image.");
        eprintln!("The first file is the entry unit (execution starts at its main).");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -o <file>            Output .p24m file (required)");
        eprintln!("  --load-addr <addr>   Runtime VM load address (hex or dec, default 0).");
        eprintln!("                       Baked into push <data_ref> operands so strings");
        eprintln!("                       and other data refs dereference correctly at");
        eprintln!("                       runtime. Use 0x010000 for standard cor24-run");
        eprintln!("                       --load-binary placement.");
        process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    // Parse args: collect input files and -o output
    let mut inputs: Vec<String> = Vec::new();
    let mut output: Option<String> = None;
    let mut load_addr: u32 = 0;
    let mut load_addr_specified = false;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "-o" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: -o requires an argument");
                process::exit(1);
            }
            output = Some(args[i].clone());
        } else if args[i] == "--load-addr" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: --load-addr requires an argument");
                process::exit(1);
            }
            load_addr = parse_addr(&args[i]).unwrap_or_else(|| {
                eprintln!("error: invalid --load-addr value: '{}'", args[i]);
                process::exit(1);
            });
            load_addr_specified = true;
        } else {
            inputs.push(args[i].clone());
        }
        i += 1;
    }

    let output = output.unwrap_or_else(|| {
        eprintln!("error: -o <output.p24m> is required");
        process::exit(1);
    });

    if inputs.is_empty() {
        eprintln!("error: at least one input .p24 file is required");
        process::exit(1);
    }

    // Read all input files
    let mut binaries: Vec<(String, Vec<u8>)> = Vec::new();
    for path in &inputs {
        let data = fs::read(path).unwrap_or_else(|e| {
            eprintln!("error: cannot read '{path}': {e}");
            process::exit(1);
        });
        binaries.push((path.clone(), data));
    }

    let refs: Vec<(&str, &[u8])> = binaries
        .iter()
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();

    // Warn if any unit has a .data section but --load-addr was not specified.
    // Without the flag, push <data_ref> operands aren't relocated to their
    // final VM addresses and loadb/loadw will read wrong memory at runtime.
    if !load_addr_specified {
        let units_with_data: Vec<&str> = refs
            .iter()
            .filter_map(|(name, data)| match load_p24(data) {
                Ok(img) if !img.data.is_empty() => Some(*name),
                _ => None,
            })
            .collect();
        if !units_with_data.is_empty() {
            eprintln!("p24-load: warning: linking units with .data sections without --load-addr.");
            eprintln!("  Data-ref pushes will not be relocated and loadb/loadw will read");
            eprintln!("  wrong memory at runtime. Use --load-addr 0x<runtime load address>");
            eprintln!("  (e.g. 0x010000 for cor24-run --load-binary).");
            eprintln!("  Units with data: {}", units_with_data.join(", "));
            eprintln!("  To silence this warning, pass --load-addr 0 explicitly.");
        }
    }

    let opts = LinkOptions { load_addr };
    match p24_load::link_units_with_opts(&refs, opts) {
        Ok(image) => {
            fs::write(&output, &image).unwrap_or_else(|e| {
                eprintln!("error: cannot write '{output}': {e}");
                process::exit(1);
            });
            eprintln!(
                "Linked {} unit(s) → {} ({} bytes, load_addr=0x{:06X})",
                inputs.len(),
                output,
                image.len(),
                load_addr
            );
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}
