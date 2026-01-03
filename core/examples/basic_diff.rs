use excel_diff::{DiffConfig, WorkbookPackage};
use std::fs::File;

fn usage() -> ! {
    eprintln!("Usage: basic_diff <OLD.xlsx> <NEW.xlsx> [N]");
    eprintln!("  N: optionally print the first N ops (debug)");
    std::process::exit(2);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let old_path = args.next().unwrap_or_else(|| usage());
    let new_path = args.next().unwrap_or_else(|| usage());
    let show_n: Option<usize> = args.next().map(|s| s.parse()).transpose()?;

    let old_pkg = WorkbookPackage::open(File::open(&old_path)?)?;
    let new_pkg = WorkbookPackage::open(File::open(&new_path)?)?;

    let report = old_pkg.diff(&new_pkg, &DiffConfig::default());

    println!("complete: {}", report.complete);
    println!("warnings: {}", report.warnings.len());
    println!("ops: {}", report.ops.len());

    if let Some(n) = show_n {
        for (i, op) in report.ops.iter().take(n).enumerate() {
            println!("{:>4}: {:?}", i, op);
        }
    }

    Ok(())
}

