// p24-dump — Pretty-print .p24 and .p24m file layouts

use std::{fs, process};

use p24_load::{P24M_MAGIC, parse_p24m};
use pa24r::{Opcode, P24_MAGIC, load_p24, opcode_size};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 || args[1] == "-h" || args[1] == "--help" {
        eprintln!("Usage: p24-dump <file.p24|file.p24m>");
        eprintln!();
        eprintln!("Pretty-prints the layout of .p24 and .p24m binary files.");
        process::exit(if args.len() != 2 { 1 } else { 0 });
    }

    let path = &args[1];
    let data = fs::read(path).unwrap_or_else(|e| {
        eprintln!("error: cannot read '{path}': {e}");
        process::exit(1);
    });

    if data.len() < 4 {
        eprintln!("error: file too short ({} bytes)", data.len());
        process::exit(1);
    }

    if data[0..4] == P24M_MAGIC {
        dump_p24m(path, &data);
    } else if data[0..4] == P24_MAGIC {
        dump_p24(path, &data);
    } else {
        eprintln!(
            "error: unrecognized magic: {:02X} {:02X} {:02X} {:02X}",
            data[0], data[1], data[2], data[3]
        );
        process::exit(1);
    }
}

fn dump_p24(path: &str, data: &[u8]) {
    let image = match load_p24(data) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    println!("=== {path} (.p24 v{}) ===", image.version);
    println!();
    println!("Header:");
    println!("  Entry point:   0x{:06X}", image.entry_point);
    println!("  Code size:     {} bytes", image.code.len());
    println!("  Data size:     {} bytes", image.data.len());
    println!(
        "  Global count:  {} words ({} bytes)",
        image.global_count,
        image.global_count * 3
    );

    if let Some(ref info) = image.unit_info {
        println!();
        println!("Unit: {}", info.name);

        if !info.exports.is_empty() {
            println!();
            println!("Exports ({}):", info.exports.len());
            for exp in &info.exports {
                println!("  {:20} offset=0x{:06X}", exp.name, exp.offset);
            }
        }

        if !info.imports.is_empty() {
            println!();
            println!("Imports ({}):", info.imports.len());
            for imp in &info.imports {
                println!(
                    "  {:20} from {:12} slot={}",
                    imp.proc_name, imp.unit_name, imp.slot
                );
            }
        }
    }

    if !image.code.is_empty() {
        println!();
        println!("Code ({} bytes):", image.code.len());
        disassemble(&image.code, 0);
    }

    if !image.data.is_empty() {
        println!();
        println!("Data ({} bytes):", image.data.len());
        hex_dump(&image.data, image.code.len());
    }
}

fn dump_p24m(path: &str, data: &[u8]) {
    let image = match parse_p24m(data) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    println!("=== {path} (.p24m v1) ===");
    println!();
    println!("Header:");
    println!("  Entry point:   0x{:06X}", image.entry_point);
    println!("  Unit count:    {}", image.unit_count);
    println!("  Total code:    {} bytes", image.total_code);
    println!(
        "  Total globals: {} words ({} bytes)",
        image.total_globals,
        image.total_globals * 3
    );
    println!("  Code offset:   0x{:06X}", image.code_off);
    println!("  Globals offset:0x{:06X}", image.globals_off);

    println!();
    println!("Unit Table ({} units):", image.unit_count);
    for (i, unit) in image.units.iter().enumerate() {
        println!(
            "  Unit {}: code_base=0x{:06X}  global_base={}  irt_off=0x{:06X}",
            i, unit.base_addr, unit.global_base, unit.irt_off
        );
    }

    println!();
    println!("Import Resolution Tables:");
    for (i, irt) in image.irts.iter().enumerate() {
        if irt.is_empty() {
            println!("  Unit {}: (none)", i);
        } else {
            println!("  Unit {}: {} entries", i, irt.len());
            for (j, addr) in irt.iter().enumerate() {
                println!("    slot {} → 0x{:06X}", j, addr);
            }
        }
    }

    // Extract code from the image
    let code_start = image.code_off;
    let code_end = code_start + image.total_code as usize;
    if code_end <= data.len() {
        println!();
        println!("Code ({} bytes):", image.total_code);

        // Disassemble per-unit
        let code = &data[code_start..code_end];
        for (i, unit) in image.units.iter().enumerate() {
            let start = unit.base_addr as usize;
            let end = if i + 1 < image.units.len() {
                image.units[i + 1].base_addr as usize
            } else {
                image.total_code as usize
            };
            if start < end && end <= code.len() {
                println!();
                println!("  --- Unit {} (0x{:06X}..0x{:06X}) ---", i, start, end);
                disassemble(&code[start..end], start);
            }
        }
    }
}

fn disassemble(code: &[u8], base_offset: usize) {
    let mut pc = 0;
    while pc < code.len() {
        let op = code[pc];
        let size = opcode_size(op);
        let mnemonic = Opcode::mnemonic_from_byte(op).unwrap_or("???");

        print!("  {:06X}: ", base_offset + pc);

        // Print hex bytes
        for i in 0..size {
            if pc + i < code.len() {
                print!("{:02X} ", code[pc + i]);
            }
        }
        // Pad to align
        for _ in size..5 {
            print!("   ");
        }

        // Print mnemonic + operands
        match size {
            1 => println!("{mnemonic}"),
            2 if pc + 1 < code.len() => {
                println!("{mnemonic} {}", code[pc + 1]);
            }
            3 if pc + 2 < code.len() => {
                if op == 0x74 {
                    // xcall: IMM16
                    let slot = code[pc + 1] as u16 | ((code[pc + 2] as u16) << 8);
                    println!("{mnemonic} slot={slot}");
                } else {
                    // D8_O8: two byte operands
                    println!("{mnemonic} {}, {}", code[pc + 1], code[pc + 2]);
                }
            }
            4 if pc + 3 < code.len() => {
                let val = code[pc + 1] as u32
                    | ((code[pc + 2] as u32) << 8)
                    | ((code[pc + 3] as u32) << 16);
                println!("{mnemonic} 0x{val:06X}");
            }
            5 if pc + 4 < code.len() => {
                let d = code[pc + 1];
                let a = code[pc + 2] as u32
                    | ((code[pc + 3] as u32) << 8)
                    | ((code[pc + 4] as u32) << 16);
                println!("{mnemonic} {d}, 0x{a:06X}");
            }
            _ => println!("{mnemonic}"),
        }

        pc += size;
    }
}

fn hex_dump(data: &[u8], base_offset: usize) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("  {:06X}: ", base_offset + i * 16);
        for b in chunk {
            print!("{b:02X} ");
        }
        // Pad
        for _ in chunk.len()..16 {
            print!("   ");
        }
        print!(" |");
        for b in chunk {
            let c = if b.is_ascii_graphic() || *b == b' ' {
                *b as char
            } else {
                '.'
            };
            print!("{c}");
        }
        println!("|");
    }
}
