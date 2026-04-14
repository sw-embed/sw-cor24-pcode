// pv24d — Interactive debugger for the COR24 P-code VM.
//
// Drives the pv24t VM step-by-step with a gdb-style REPL. Supports breakpoints,
// single-stepping, memory inspection, and disassembly.
//
// Usage:
//   pv24d <file.p24>            # Load and start debugging
//   pv24d -i <text> <file.p24>  # Provide stdin input

use pa24r::load_p24;
use pv24t::{Vm, WORD, disasm_one};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::process;

fn usage() -> ! {
    eprintln!("pv24d — Interactive P-code VM Debugger for COR24");
    eprintln!();
    eprintln!("Usage: pv24d [options] <file.p24>");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -i <text>   Provide stdin input as a string");
    eprintln!("  -h          Show this help");
    eprintln!();
    eprintln!("Commands inside the REPL:");
    eprintln!("  run, r             Start/restart program from entry");
    eprintln!("  continue, c        Resume until next breakpoint or halt");
    eprintln!("  step, s [N]        Execute N instructions (default 1)");
    eprintln!("  break <addr>, b    Set breakpoint at hex address");
    eprintln!("  info break, ib     List breakpoints");
    eprintln!("  delete <id>, d     Delete breakpoint by id");
    eprintln!("  info regs, ir      Show registers");
    eprintln!("  info stack, is     Show eval stack contents");
    eprintln!("  x/<N><fmt> <addr>  Examine memory (fmt: b,w,x,d,s,i)");
    eprintln!("  disas [<addr>]     Disassemble (default: at PC)");
    eprintln!("  print <expr>, p    Print register ($pc,$esp,$csp,$fp) or number");
    eprintln!("  help, h [cmd]      Show help");
    eprintln!("  quit, q            Exit debugger");
    process::exit(1);
}

struct Debugger {
    vm: Vm,
    breakpoints: BTreeMap<u32, usize>, // addr -> id
    next_bp_id: usize,
    entry_point: u32,
    initial_image: pa24r::LoadedImage,
    stdin_data: Vec<u8>,
}

impl Debugger {
    fn new(image: pa24r::LoadedImage, stdin_data: Vec<u8>) -> Self {
        let entry_point = image.entry_point;
        let vm = Vm::new(image.clone(), stdin_data.clone(), false, 0);
        Self {
            vm,
            breakpoints: BTreeMap::new(),
            next_bp_id: 1,
            entry_point,
            initial_image: image,
            stdin_data,
        }
    }

    fn restart(&mut self) {
        self.vm = Vm::new(
            self.initial_image.clone(),
            self.stdin_data.clone(),
            false,
            0,
        );
    }

    fn at_breakpoint(&self) -> Option<usize> {
        self.breakpoints.get(&(self.vm.pc as u32)).copied()
    }

    fn cmd_run(&mut self) {
        self.restart();
        println!("Started program at 0x{:06X}", self.entry_point);
        self.cmd_continue();
    }

    fn cmd_continue(&mut self) {
        if self.vm.status != 0 {
            println!("Program is not running. Use `run` to start.");
            return;
        }
        // Step once unconditionally to move past current breakpoint
        self.vm.step_one();
        while self.vm.status == 0 {
            if let Some(id) = self.at_breakpoint() {
                println!("Breakpoint {} hit at 0x{:06X}", id, self.vm.pc);
                return;
            }
            self.vm.step_one();
        }
        self.report_stop();
    }

    fn cmd_step(&mut self, n: u64) {
        if self.vm.status != 0 {
            println!("Program is not running. Use `run` to start.");
            return;
        }
        for _ in 0..n {
            if self.vm.status != 0 {
                break;
            }
            self.vm.step_one();
        }
        if self.vm.status == 0 {
            // Show where we landed
            let code = &self.vm.mem[..self.vm.code_size];
            let (text, _) = disasm_one(code, self.vm.pc);
            println!("0x{:06X}: {}", self.vm.pc, text);
        } else {
            self.report_stop();
        }
    }

    fn report_stop(&self) {
        match self.vm.status {
            1 => println!("Program halted normally."),
            2 => println!("Program trapped: TRAP {}", self.vm.trap_code),
            _ => {}
        }
    }

    fn cmd_break(&mut self, arg: &str) {
        let addr = match parse_addr(arg) {
            Some(a) => a,
            None => {
                println!("Invalid address: {arg}");
                return;
            }
        };
        if (addr as usize) >= self.vm.code_size {
            println!(
                "Warning: address 0x{addr:06X} is outside code segment (0..0x{:06X})",
                self.vm.code_size
            );
        }
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.breakpoints.insert(addr, id);
        println!("Breakpoint {id} at 0x{addr:06X}");
    }

    fn cmd_info_break(&self) {
        if self.breakpoints.is_empty() {
            println!("No breakpoints set.");
            return;
        }
        println!("Num  Address");
        for (addr, id) in &self.breakpoints {
            println!("{id:<4} 0x{addr:06X}");
        }
    }

    fn cmd_delete(&mut self, arg: &str) {
        let id: usize = match arg.parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid breakpoint id: {arg}");
                return;
            }
        };
        let key = self
            .breakpoints
            .iter()
            .find(|(_, v)| **v == id)
            .map(|(k, _)| *k);
        match key {
            Some(addr) => {
                self.breakpoints.remove(&addr);
                println!("Deleted breakpoint {id}");
            }
            None => println!("No breakpoint with id {id}"),
        }
    }

    fn cmd_info_regs(&self) {
        println!("pc  = 0x{:06X}", self.vm.pc);
        println!(
            "esp = 0x{:06X}  (depth {})",
            self.vm.esp,
            self.vm.eval_depth()
        );
        println!("csp = 0x{:06X}", self.vm.csp);
        println!("fp  = 0x{:06X}", self.vm.fp_vm);
        println!("gp  = 0x{:06X}", self.vm.gp);
        println!("hp  = 0x{:06X}", self.vm.hp);
        println!(
            "status = {} ({})",
            self.vm.status,
            match self.vm.status {
                0 => "running",
                1 => "halted",
                2 => "trapped",
                _ => "?",
            }
        );
        if self.vm.status == 2 {
            println!("trap_code = {}", self.vm.trap_code);
        }
    }

    fn cmd_info_stack(&self) {
        let depth = self.vm.eval_depth();
        if depth == 0 {
            println!("Eval stack is empty.");
            return;
        }
        println!("Eval stack (bottom to top, {depth} entries):");
        for i in 0..depth {
            let addr = self.vm.eval_stack_base + i * WORD;
            let val = self.vm.read_word(addr);
            let tos_marker = if i == depth - 1 { " <- TOS" } else { "" };
            println!(
                "  [{i:3}] 0x{addr:06X} = {val:>10} (0x{:06X}){}",
                (val as u32) & 0xFFFFFF,
                tos_marker
            );
        }
    }

    fn cmd_examine(&self, spec: &str, addr_arg: &str) {
        // spec is like "16bx", "8w", "s", "i", etc.
        let mut count: usize = 1;
        let mut idx = 0;
        let bytes = spec.as_bytes();
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx > 0 {
            count = spec[..idx].parse().unwrap_or(1);
        }
        let fmt_chars = &spec[idx..];

        // First char is unit (b=byte, w=word), second is display (x,d,s,i)
        let unit = fmt_chars.chars().find(|c| "bw".contains(*c)).unwrap_or('w');
        let display = fmt_chars
            .chars()
            .find(|c| "xdsi".contains(*c))
            .unwrap_or('x');

        let addr = match parse_addr(addr_arg) {
            Some(a) => a as usize,
            None => {
                println!("Invalid address: {addr_arg}");
                return;
            }
        };

        match display {
            's' => {
                // String until null (count is max length hint, default 256)
                print!("0x{addr:06X}: \"");
                let mut p = addr;
                let max = if count > 1 { count } else { 256 };
                for _ in 0..max {
                    if p >= self.vm.mem.len() {
                        break;
                    }
                    let b = self.vm.mem[p];
                    if b == 0 {
                        break;
                    }
                    if (0x20..0x7F).contains(&b) {
                        print!("{}", b as char);
                    } else {
                        print!("\\x{b:02X}");
                    }
                    p += 1;
                }
                println!("\"");
            }
            'i' => {
                // Disassemble
                let code = &self.vm.mem[..self.vm.code_size];
                let mut p = addr;
                for _ in 0..count {
                    if p >= code.len() {
                        break;
                    }
                    let (text, size) = disasm_one(code, p);
                    println!("0x{p:06X}: {text}");
                    if size == 0 {
                        break;
                    }
                    p += size;
                }
            }
            _ => {
                let size = if unit == 'b' { 1 } else { WORD };
                for i in 0..count {
                    let a = addr + i * size;
                    if a >= self.vm.mem.len() {
                        break;
                    }
                    if i % 8 == 0 {
                        if i > 0 {
                            println!();
                        }
                        print!("0x{a:06X}:");
                    }
                    if unit == 'b' {
                        let v = self.vm.mem[a];
                        match display {
                            'd' => print!(" {:4}", v),
                            _ => print!(" {:02X}", v),
                        }
                    } else {
                        let v = self.vm.read_word(a);
                        match display {
                            'd' => print!(" {:>8}", v),
                            _ => print!(" {:06X}", (v as u32) & 0xFFFFFF),
                        }
                    }
                }
                println!();
            }
        }
    }

    fn cmd_disas(&self, arg: &str) {
        let start = if arg.is_empty() {
            self.vm.pc
        } else {
            parse_addr(arg).map(|a| a as usize).unwrap_or(self.vm.pc)
        };
        let code = &self.vm.mem[..self.vm.code_size];
        let mut p = start;
        for _ in 0..10 {
            if p >= code.len() {
                break;
            }
            let (text, size) = disasm_one(code, p);
            let marker = if p == self.vm.pc { "=> " } else { "   " };
            println!("{marker}0x{p:06X}: {text}");
            if size == 0 {
                break;
            }
            p += size;
        }
    }

    fn cmd_print(&self, arg: &str) {
        match arg.trim() {
            "$pc" => println!("$pc = 0x{:06X}", self.vm.pc),
            "$esp" => println!("$esp = 0x{:06X}", self.vm.esp),
            "$csp" => println!("$csp = 0x{:06X}", self.vm.csp),
            "$fp" => println!("$fp = 0x{:06X}", self.vm.fp_vm),
            "$gp" => println!("$gp = 0x{:06X}", self.vm.gp),
            "$hp" => println!("$hp = 0x{:06X}", self.vm.hp),
            "$tos" => {
                if self.vm.eval_depth() == 0 {
                    println!("eval stack is empty");
                } else {
                    println!("$tos = {}", self.vm.peek_eval());
                }
            }
            other => match parse_addr(other) {
                Some(n) => println!("{n} (0x{n:06X})"),
                None => println!("Unknown expression: {other}"),
            },
        }
    }
}

fn parse_addr(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>()
            .ok()
            .or_else(|| u32::from_str_radix(s, 16).ok())
    }
}

fn split_first(s: &str) -> (&str, &str) {
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

fn print_help() {
    println!("Execution:  run, continue (c), step (s) [N], quit (q)");
    println!("Breakpts:   break <addr>, info break, delete <id>");
    println!("Inspect:    info regs, info stack, print <expr>");
    println!("            x/<N><fmt> <addr>  — examine memory");
    println!("              fmt: b=byte w=word x=hex d=decimal s=string i=disasm");
    println!("            disas [<addr>]     — disassemble 10 insns");
    println!("Expressions: $pc $esp $csp $fp $gp $hp $tos  0xHEX  DEC");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_file = None;
    let mut stdin_text = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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
                eprintln!("pv24d: unknown option: {other}");
                usage();
            }
        }
    }

    let path = match input_file {
        Some(p) => p,
        None => {
            eprintln!("pv24d: missing input file");
            usage();
        }
    };

    let binary = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("pv24d: cannot read {path}: {e}");
            process::exit(1);
        }
    };

    let image = match load_p24(&binary) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("pv24d: {path}: {e}");
            process::exit(1);
        }
    };

    println!(
        "pv24d: loaded {path}: code={} data={} globals={} entry=0x{:06X}",
        image.code.len(),
        image.data.len(),
        image.global_count,
        image.entry_point
    );
    println!("Type `help` for commands. Program is not yet running — use `run` to start.");

    let stdin_data = stdin_text.map(|s| s.into_bytes()).unwrap_or_default();
    let mut dbg = Debugger::new(image, stdin_data);

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut last_line = String::new();

    loop {
        print!("(pv24d) ");
        let _ = stdout.flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("pv24d: input error: {e}");
                break;
            }
        }

        let line = line.trim().to_string();
        // Empty line repeats the last command (gdb-style)
        let effective = if line.is_empty() && !last_line.is_empty() {
            last_line.clone()
        } else {
            line.clone()
        };
        if effective.is_empty() {
            continue;
        }
        if !line.is_empty() {
            last_line = line.clone();
        }

        let (cmd, rest) = split_first(&effective);
        match cmd {
            "run" | "r" => dbg.cmd_run(),
            "continue" | "c" => dbg.cmd_continue(),
            "step" | "s" => {
                let n: u64 = rest.trim().parse().unwrap_or(1);
                dbg.cmd_step(n);
            }
            "break" | "b" => dbg.cmd_break(rest.trim()),
            "delete" | "d" => dbg.cmd_delete(rest.trim()),
            "info" | "i" => {
                let (sub, _) = split_first(rest);
                match sub {
                    "break" | "b" => dbg.cmd_info_break(),
                    "regs" | "r" => dbg.cmd_info_regs(),
                    "stack" | "s" => dbg.cmd_info_stack(),
                    _ => println!("Unknown info subcommand: {sub}"),
                }
            }
            "ib" => dbg.cmd_info_break(),
            "ir" => dbg.cmd_info_regs(),
            "is" => dbg.cmd_info_stack(),
            "disas" | "disassemble" => dbg.cmd_disas(rest.trim()),
            "print" | "p" => dbg.cmd_print(rest.trim()),
            "help" | "h" | "?" => print_help(),
            "quit" | "q" | "exit" => break,
            c if c.starts_with("x/") || c == "x" => {
                // x/<N><fmt> <addr>
                let spec = c.strip_prefix("x/").unwrap_or("");
                dbg.cmd_examine(spec, rest.trim());
            }
            _ => println!("Unknown command: {cmd} (type `help`)"),
        }
    }
}
