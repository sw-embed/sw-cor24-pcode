// p24-load — CLI for linking multiple .p24 units into a .p24m image

use std::{fs, process};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("Usage: p24-load <unit1.p24> [unit2.p24 ...] -o <output.p24m>");
        eprintln!();
        eprintln!("Links multiple .p24 unit files into a single .p24m multi-unit image.");
        eprintln!("The first file is the entry unit (execution starts at its main).");
        process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    // Parse args: collect input files and -o output
    let mut inputs: Vec<String> = Vec::new();
    let mut output: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "-o" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: -o requires an argument");
                process::exit(1);
            }
            output = Some(args[i].clone());
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

    match p24_load::link_units(&refs) {
        Ok(image) => {
            fs::write(&output, &image).unwrap_or_else(|e| {
                eprintln!("error: cannot write '{output}': {e}");
                process::exit(1);
            });
            eprintln!(
                "Linked {} unit(s) → {} ({} bytes)",
                inputs.len(),
                output,
                image.len()
            );
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}
