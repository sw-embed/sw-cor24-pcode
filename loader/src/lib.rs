// p24-load — P24 multi-unit loader
//
// Reads multiple v2 .p24 files, resolves cross-unit imports against exports,
// and produces a .p24m multi-unit image for the P-code VM.

use pa24r::{LoadedImage, fnv1a_16, load_p24};

/// .p24m header constants
pub const P24M_MAGIC: [u8; 4] = [0x50, 0x32, 0x34, 0x4D]; // "P24M"
pub const P24M_VERSION: u8 = 1;
pub const P24M_HEADER_SIZE: usize = 21;

/// A loaded unit with its assigned layout.
#[derive(Debug)]
pub struct LoadedUnit {
    pub id: usize,
    pub name: String,
    pub image: LoadedImage,
    pub code_base: u32,
    pub global_base: u32,
}

/// A resolved import: absolute target address.
#[derive(Debug)]
pub struct ResolvedImport {
    pub slot: u16,
    pub target_addr: u32,
}

/// Per-unit IRT (import resolution table).
#[derive(Debug)]
pub struct UnitIrt {
    pub unit_id: usize,
    pub entries: Vec<ResolvedImport>,
}

#[derive(Debug)]
pub enum LoaderError {
    Io(String),
    Format(String),
    UnresolvedImport {
        unit: String,
        proc_name: String,
        target_unit: String,
    },
    DuplicateUnitName(String),
    HashCollision {
        unit: String,
        name_a: String,
        name_b: String,
    },
    AddressOverflow,
    NoUnits,
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::Io(s) => write!(f, "I/O error: {s}"),
            LoaderError::Format(s) => write!(f, "format error: {s}"),
            LoaderError::UnresolvedImport {
                unit,
                proc_name,
                target_unit,
            } => {
                write!(
                    f,
                    "unresolved import: {unit} wants '{proc_name}' from '{target_unit}'"
                )
            }
            LoaderError::DuplicateUnitName(n) => write!(f, "duplicate unit name: '{n}'"),
            LoaderError::HashCollision {
                unit,
                name_a,
                name_b,
            } => {
                write!(
                    f,
                    "hash collision in unit '{unit}': '{name_a}' and '{name_b}'"
                )
            }
            LoaderError::AddressOverflow => write!(f, "combined code exceeds 24-bit address space"),
            LoaderError::NoUnits => write!(f, "no input units"),
        }
    }
}

/// Load and link multiple .p24 files into a .p24m image.
pub fn link_units(binaries: &[(&str, &[u8])]) -> Result<Vec<u8>, LoaderError> {
    if binaries.is_empty() {
        return Err(LoaderError::NoUnits);
    }

    // 1. Parse all .p24 files
    let mut units: Vec<LoadedUnit> = Vec::new();
    for (i, (name, data)) in binaries.iter().enumerate() {
        let image = load_p24(data).map_err(|e| LoaderError::Format(format!("{name}: {e}")))?;
        let unit_name = image
            .unit_info
            .as_ref()
            .map(|u| u.name.clone())
            .unwrap_or_else(|| format!("unit{i}"));
        units.push(LoadedUnit {
            id: i,
            name: unit_name,
            image,
            code_base: 0,
            global_base: 0,
        });
    }

    // 2. Check for duplicate unit names
    for i in 0..units.len() {
        for j in (i + 1)..units.len() {
            if units[i].name == units[j].name {
                return Err(LoaderError::DuplicateUnitName(units[i].name.clone()));
            }
        }
    }

    // 3. Check for hash collisions within each unit's exports
    for unit in &units {
        if let Some(ref info) = unit.image.unit_info {
            for i in 0..info.exports.len() {
                for j in (i + 1)..info.exports.len() {
                    if fnv1a_16(info.exports[i].name.as_bytes())
                        == fnv1a_16(info.exports[j].name.as_bytes())
                    {
                        return Err(LoaderError::HashCollision {
                            unit: unit.name.clone(),
                            name_a: info.exports[i].name.clone(),
                            name_b: info.exports[j].name.clone(),
                        });
                    }
                }
            }
        }
    }

    // 4. Layout code segments sequentially
    let mut code_offset: u32 = 0;
    for unit in &mut units {
        unit.code_base = code_offset;
        let size = unit.image.code.len() as u32;
        code_offset = code_offset
            .checked_add(size)
            .filter(|&v| v <= 0xFF_FFFF)
            .ok_or(LoaderError::AddressOverflow)?;
    }
    let total_code = code_offset;

    // 5. Layout global segments
    let mut global_offset: u32 = 0;
    for unit in &mut units {
        unit.global_base = global_offset;
        global_offset += unit.image.global_count;
    }
    let total_globals = global_offset;

    // 6. Resolve imports and build IRTs
    let mut irts: Vec<UnitIrt> = Vec::new();
    for unit in &units {
        let mut entries = Vec::new();
        if let Some(ref info) = unit.image.unit_info {
            for imp in &info.imports {
                let target_unit =
                    units
                        .iter()
                        .find(|u| u.name == imp.unit_name)
                        .ok_or_else(|| LoaderError::UnresolvedImport {
                            unit: unit.name.clone(),
                            proc_name: imp.proc_name.clone(),
                            target_unit: imp.unit_name.clone(),
                        })?;

                let export = target_unit
                    .image
                    .unit_info
                    .as_ref()
                    .and_then(|info| info.exports.iter().find(|e| e.name == imp.proc_name))
                    .ok_or_else(|| LoaderError::UnresolvedImport {
                        unit: unit.name.clone(),
                        proc_name: imp.proc_name.clone(),
                        target_unit: imp.unit_name.clone(),
                    })?;

                let abs_addr = target_unit.code_base + export.offset;
                entries.push(ResolvedImport {
                    slot: imp.slot,
                    target_addr: abs_addr,
                });
            }
        }
        irts.push(UnitIrt {
            unit_id: unit.id,
            entries,
        });
    }

    // 7. Patch LOADG/STOREG/ADDRG operands with global partition offsets
    let mut patched_code: Vec<Vec<u8>> = units.iter().map(|u| u.image.code.clone()).collect();
    for unit in &units {
        if unit.global_base > 0 {
            patch_global_operands(&mut patched_code[unit.id], unit.global_base);
        }
    }

    // 8. Emit .p24m image
    let mut image = Vec::new();

    // Compute layout offsets
    let unit_table_off = P24M_HEADER_SIZE as u32;
    let unit_table_size = (units.len() * 6) as u32;
    let irt_off = unit_table_off + unit_table_size;
    let mut irt_size: u32 = 0;
    for irt in &irts {
        irt_size += 2 + (irt.entries.len() as u32) * 3;
    }
    let code_off = irt_off + irt_size;
    let data_off = code_off + total_code;
    let total_data: u32 = units.iter().map(|u| u.image.data.len() as u32).sum();

    // Header
    image.extend_from_slice(&P24M_MAGIC);
    image.push(P24M_VERSION);
    emit_le24(&mut image, units[0].image.entry_point + units[0].code_base);
    image.push(units.len() as u8);
    emit_le24(&mut image, total_code);
    emit_le24(&mut image, total_globals);
    emit_le24(&mut image, unit_table_off);
    emit_le24(&mut image, irt_off);

    // Unit table (6 bytes per unit: base_addr(3) + global_base(3))
    for unit in &units {
        emit_le24(&mut image, unit.code_base);
        emit_le24(&mut image, unit.global_base);
    }

    // IRT (per unit: import_count(2) + [abs_addr(3)] * count)
    for irt in &irts {
        let count = irt.entries.len() as u16;
        image.push(count as u8);
        image.push((count >> 8) as u8);
        for entry in &irt.entries {
            emit_le24(&mut image, entry.target_addr);
        }
    }

    // Code segments (concatenated)
    for code in &patched_code {
        image.extend_from_slice(code);
    }

    // Data segments (concatenated)
    for unit in &units {
        image.extend_from_slice(&unit.image.data);
    }

    // Globals (zeroed)
    let globals_bytes = total_globals as usize * 3;
    image.resize(image.len() + globals_bytes, 0);

    let _ = data_off; // used implicitly by layout
    let _ = total_data;

    Ok(image)
}

/// Patch LOADG/STOREG/ADDRG 24-bit operands by adding a global word offset.
///
/// These opcodes use raw word indices. When multiple units share a global
/// segment, each unit's indices need to be offset by its partition start.
fn patch_global_operands(code: &mut [u8], global_word_offset: u32) {
    let mut pc = 0;
    while pc < code.len() {
        let op = code[pc];
        let size = pa24r::opcode_size(op);
        // LOADG=0x44, STOREG=0x45, ADDRG=0x47 — all IMM24 (4 bytes)
        if (op == 0x44 || op == 0x45 || op == 0x47) && pc + 3 < code.len() {
            let val = read_le24(&code[pc + 1..pc + 4]);
            let patched = val + global_word_offset;
            code[pc + 1] = patched as u8;
            code[pc + 2] = (patched >> 8) as u8;
            code[pc + 3] = (patched >> 16) as u8;
        }
        pc += size;
    }
}

/// Load a .p24m image and extract its structure for verification.
pub fn parse_p24m(data: &[u8]) -> Result<P24mImage, LoaderError> {
    if data.len() < P24M_HEADER_SIZE {
        return Err(LoaderError::Format(
            "file too short for .p24m header".into(),
        ));
    }
    if data[0..4] != P24M_MAGIC {
        return Err(LoaderError::Format("invalid .p24m magic".into()));
    }
    if data[4] != P24M_VERSION {
        return Err(LoaderError::Format(format!(
            "unsupported .p24m version: {}",
            data[4]
        )));
    }

    let entry_point = read_le24(&data[5..8]);
    let unit_count = data[8] as usize;
    let total_code = read_le24(&data[9..12]);
    let total_globals = read_le24(&data[12..15]);
    let unit_table_off = read_le24(&data[15..18]) as usize;
    let irt_off = read_le24(&data[18..21]) as usize;

    // Parse unit table
    let mut unit_entries = Vec::new();
    let mut pos = unit_table_off;
    for _ in 0..unit_count {
        if pos + 6 > data.len() {
            return Err(LoaderError::Format("unit table truncated".into()));
        }
        let base_addr = read_le24(&data[pos..pos + 3]);
        let global_base = read_le24(&data[pos + 3..pos + 6]);
        unit_entries.push(P24mUnitEntry {
            base_addr,
            global_base,
        });
        pos += 6;
    }

    // Parse IRTs
    let mut irt_entries = Vec::new();
    pos = irt_off;
    for _ in 0..unit_count {
        if pos + 2 > data.len() {
            return Err(LoaderError::Format("IRT truncated".into()));
        }
        let count = data[pos] as u16 | ((data[pos + 1] as u16) << 8);
        pos += 2;
        let mut addrs = Vec::new();
        for _ in 0..count {
            if pos + 3 > data.len() {
                return Err(LoaderError::Format("IRT entries truncated".into()));
            }
            addrs.push(read_le24(&data[pos..pos + 3]));
            pos += 3;
        }
        irt_entries.push(addrs);
    }

    Ok(P24mImage {
        entry_point,
        unit_count,
        total_code,
        total_globals,
        units: unit_entries,
        irts: irt_entries,
    })
}

/// Parsed .p24m image structure.
#[derive(Debug)]
pub struct P24mImage {
    pub entry_point: u32,
    pub unit_count: usize,
    pub total_code: u32,
    pub total_globals: u32,
    pub units: Vec<P24mUnitEntry>,
    pub irts: Vec<Vec<u32>>,
}

#[derive(Debug)]
pub struct P24mUnitEntry {
    pub base_addr: u32,
    pub global_base: u32,
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
    use pa24r::{assemble, assemble_to_p24};

    fn make_lib_unit() -> Vec<u8> {
        let source = "\
.unit mathlib
.export double 1

.proc main 0
    halt
.end

.proc double 1
    loada 0
    dup
    add
    ret 1
.end
";
        assemble_to_p24(source).expect("lib unit should assemble")
    }

    fn make_app_unit() -> Vec<u8> {
        let source = "\
.unit app
.import mathlib
.extern double 1

.proc main 0
    push_s 21
    xcall double
    sys 1
    halt
.end
";
        assemble_to_p24(source).expect("app unit should assemble")
    }

    #[test]
    fn link_two_units() {
        let app = make_app_unit();
        let lib = make_lib_unit();
        let image = link_units(&[("app.p24", &app), ("mathlib.p24", &lib)])
            .expect("linking should succeed");

        let parsed = parse_p24m(&image).expect("parse should succeed");
        assert_eq!(parsed.unit_count, 2);
        assert_eq!(parsed.units[0].base_addr, 0); // app is first
        assert_eq!(parsed.units[1].base_addr, assemble(
            ".unit app\n.import mathlib\n.extern double 1\n.proc main 0\npush_s 21\nxcall double\nsys 1\nhalt\n.end\n"
        ).code.len() as u32); // lib starts after app code
    }

    #[test]
    fn irt_resolved_correctly() {
        let app = make_app_unit();
        let lib = make_lib_unit();
        let image = link_units(&[("app.p24", &app), ("mathlib.p24", &lib)])
            .expect("linking should succeed");

        let parsed = parse_p24m(&image).expect("parse should succeed");

        // App has 1 import (double), lib has 0 imports
        assert_eq!(parsed.irts[0].len(), 1);
        assert_eq!(parsed.irts[1].len(), 0);

        // double is the second proc in mathlib, after main
        // main: enter(2) + halt(1) + leave(1) = 4 bytes
        // double starts at offset 4 within mathlib
        let lib_base = parsed.units[1].base_addr;
        let expected_double_addr = lib_base + 4;
        assert_eq!(parsed.irts[0][0], expected_double_addr);
    }

    #[test]
    fn entry_point_is_unit0() {
        let app = make_app_unit();
        let lib = make_lib_unit();
        let image = link_units(&[("app.p24", &app), ("mathlib.p24", &lib)])
            .expect("linking should succeed");

        let parsed = parse_p24m(&image).expect("parse should succeed");
        // Entry point = unit 0's entry + unit 0's base (0)
        assert_eq!(parsed.entry_point, 0);
    }

    #[test]
    fn global_partition_layout() {
        // App: 2 globals, Lib: 3 globals
        let app_src = "\
.unit app
.global g1 1
.global g2 1
.proc main 0
    halt
.end
";
        let lib_src = "\
.unit mathlib
.global ga 1
.global gb 1
.global gc 1
.proc main 0
    halt
.end
";
        let app = assemble_to_p24(app_src).unwrap();
        let lib = assemble_to_p24(lib_src).unwrap();
        let image = link_units(&[("app.p24", &app), ("mathlib.p24", &lib)]).unwrap();

        let parsed = parse_p24m(&image).unwrap();
        assert_eq!(parsed.units[0].global_base, 0); // app globals at 0
        assert_eq!(parsed.units[1].global_base, 2); // lib globals at 2
        assert_eq!(parsed.total_globals, 5); // 2 + 3
    }

    #[test]
    fn global_operand_patching() {
        // Unit with loadg 0, storeg 1 — after patching with offset 5,
        // should become loadg 5, storeg 6
        let mut code = vec![
            0x44, 0x00, 0x00, 0x00, // loadg 0
            0x45, 0x01, 0x00, 0x00, // storeg 1
            0x47, 0x02, 0x00, 0x00, // addrg 2
            0x01, 0x42, 0x00, 0x00, // push 0x42 (should NOT be patched)
            0x00, // halt
        ];
        patch_global_operands(&mut code, 5);
        // loadg: 0 + 5 = 5
        assert_eq!(read_le24(&code[1..4]), 5);
        // storeg: 1 + 5 = 6
        assert_eq!(read_le24(&code[5..8]), 6);
        // addrg: 2 + 5 = 7
        assert_eq!(read_le24(&code[9..12]), 7);
        // push should be unchanged
        assert_eq!(read_le24(&code[13..16]), 0x42);
    }

    #[test]
    fn error_unresolved_import() {
        let app_src = "\
.unit app
.import nonexistent
.extern missing_fn

.proc main 0
    xcall missing_fn
    halt
.end
";
        let app = assemble_to_p24(app_src).unwrap();
        let result = link_units(&[("app.p24", &app)]);
        assert!(matches!(result, Err(LoaderError::UnresolvedImport { .. })));
    }

    #[test]
    fn error_duplicate_unit_name() {
        let a = assemble_to_p24(".unit dup_name\n.proc main 0\nhalt\n.end\n").unwrap();
        let b = assemble_to_p24(".unit dup_name\n.proc main 0\nhalt\n.end\n").unwrap();
        let result = link_units(&[("a.p24", &a), ("b.p24", &b)]);
        assert!(matches!(result, Err(LoaderError::DuplicateUnitName(_))));
    }

    #[test]
    fn error_no_units() {
        let result = link_units(&[]);
        assert!(matches!(result, Err(LoaderError::NoUnits)));
    }

    #[test]
    fn p24m_round_trip() {
        let app = make_app_unit();
        let lib = make_lib_unit();
        let image = link_units(&[("app.p24", &app), ("mathlib.p24", &lib)]).unwrap();

        // Verify magic and version
        assert_eq!(&image[0..4], &P24M_MAGIC);
        assert_eq!(image[4], P24M_VERSION);

        // Parse and verify round-trip
        let parsed = parse_p24m(&image).unwrap();
        assert_eq!(parsed.unit_count, 2);
        assert!(parsed.total_code > 0);
    }

    #[test]
    fn single_v1_unit_loads() {
        // A v1 .p24 (no .unit directive) should load as a single unit
        let source = ".proc main 0\nhalt\n.end\n";
        let binary = assemble_to_p24(source).unwrap();
        let image = link_units(&[("app.p24", &binary)]).unwrap();
        let parsed = parse_p24m(&image).unwrap();
        assert_eq!(parsed.unit_count, 1);
        assert_eq!(parsed.units[0].base_addr, 0);
    }
}
