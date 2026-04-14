// pv24t — P-code Trace Interpreter for COR24
//
// Reads a .p24 binary and executes it with optional instruction-level tracing.
//
// Usage:
//   pv24t <file.p24>              # Run, program output to stdout
//   pv24t -t <file.p24>           # Run with trace to stderr
//   pv24t -t -n 1000 <file.p24>  # Trace with instruction limit

use pa24r::load_p24;
use pv24t::Vm;
use std::process;

fn usage() -> ! {
    eprintln!("pv24t — P-code Trace Interpreter for COR24");
    eprintln!();
    eprintln!("Usage: pv24t [options] <file.p24>");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -t          Enable instruction-level tracing to stderr");
    eprintln!("  -n <count>  Maximum instructions to execute (0 = unlimited)");
    eprintln!("  -i <text>   Provide stdin input as a string");
    eprintln!("  -h          Show this help");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut trace = false;
    let mut max_instructions: u64 = 0;
    let mut input_file = None;
    let mut stdin_text = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-t" => {
                trace = true;
                i += 1;
            }
            "-n" => {
                i += 1;
                max_instructions = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 1;
            }
            "-i" => {
                i += 1;
                stdin_text = args.get(i).cloned();
                i += 1;
            }
            "-h" | "--help" => usage(),
            arg if !arg.starts_with('-') => {
                input_file = Some(arg.to_string());
                i += 1;
            }
            other => {
                eprintln!("pv24t: unknown option: {other}");
                usage();
            }
        }
    }

    let path = match input_file {
        Some(p) => p,
        None => {
            eprintln!("pv24t: missing input file");
            usage();
        }
    };

    let binary = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("pv24t: cannot read {path}: {e}");
            process::exit(1);
        }
    };

    let image = match load_p24(&binary) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("pv24t: {path}: {e}");
            process::exit(1);
        }
    };

    if trace {
        eprintln!(
            "pv24t: loaded {path}: code={} data={} globals={} entry=0x{:04X}",
            image.code.len(),
            image.data.len(),
            image.global_count,
            image.entry_point
        );
    }

    let stdin_data = stdin_text.map(|s| s.into_bytes()).unwrap_or_default();
    let mut vm = Vm::new(image, stdin_data, trace, max_instructions);
    vm.run();

    if trace {
        eprintln!(
            "pv24t: {} instructions executed, status={}",
            vm.instruction_count, vm.status
        );
    }

    process::exit(if vm.status == 2 {
        vm.trap_code as i32
    } else {
        0
    });
}
