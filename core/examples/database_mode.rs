use excel_diff::{DiffConfig, WorkbookPackage};
use std::fs::File;
use std::io;

fn usage() -> ! {
    eprintln!("Usage: database_mode <OLD.xlsx> <NEW.xlsx> <SHEET_NAME> <KEYS>");
    eprintln!("  KEYS: comma-separated column letters (e.g. A,C,AA)");
    eprintln!("  Note: key columns are 0-based indices internally (A=0, B=1, ...).");
    std::process::exit(2);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let old_path = args.next().unwrap_or_else(|| usage());
    let new_path = args.next().unwrap_or_else(|| usage());
    let sheet_name = args.next().unwrap_or_else(|| usage());
    let keys = args.next().unwrap_or_else(|| usage());

    let key_columns = parse_key_columns(&keys)?;

    let old_pkg = WorkbookPackage::open(File::open(&old_path)?)?;
    let new_pkg = WorkbookPackage::open(File::open(&new_path)?)?;

    let report = old_pkg.diff_database_mode(&new_pkg, &sheet_name, &key_columns, &DiffConfig::default())?;

    for warning in &report.warnings {
        eprintln!("warning: {}", warning);
    }

    println!("complete: {}", report.complete);
    println!("ops: {}", report.ops.len());

    for (i, op) in report.ops.iter().take(25).enumerate() {
        println!("{:>4}: {:?}", i, op);
    }

    Ok(())
}

fn parse_key_columns(keys: &str) -> io::Result<Vec<u32>> {
    let mut out = Vec::new();
    for token in keys.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        out.push(col_letters_to_index(token)?);
    }

    if out.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no key columns specified",
        ));
    }

    Ok(out)
}

fn col_letters_to_index(letters: &str) -> io::Result<u32> {
    let mut col: u32 = 0;
    for ch in letters.chars() {
        let upper = ch.to_ascii_uppercase();
        if !upper.is_ascii_uppercase() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid column token: '{letters}'"),
            ));
        }
        col = col
            .checked_mul(26)
            .and_then(|c| c.checked_add((upper as u8 - b'A' + 1) as u32))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("column '{letters}' is out of range"),
                )
            })?;
    }
    Ok(col - 1)
}
