use excel_diff::{DiffConfig, WorkbookPackage};
use std::fs::File;

fn usage() -> ! {
    eprintln!("Usage: custom_config <OLD.xlsx> <NEW.xlsx>");
    std::process::exit(2);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let old_path = args.next().unwrap_or_else(|| usage());
    let new_path = args.next().unwrap_or_else(|| usage());

    let old_pkg = WorkbookPackage::open(File::open(&old_path)?)?;
    let new_pkg = WorkbookPackage::open(File::open(&new_path)?)?;

    let mut cfg = DiffConfig::fastest();
    cfg.max_memory_mb = Some(256);
    cfg.timeout_seconds = Some(10);

    let report = old_pkg.diff(&new_pkg, &cfg);

    for warning in &report.warnings {
        eprintln!("warning: {}", warning);
    }

    println!("complete: {}", report.complete);
    println!("ops: {}", report.ops.len());
    Ok(())
}

