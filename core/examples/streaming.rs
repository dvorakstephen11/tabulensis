use excel_diff::{DiffConfig, JsonLinesSink, WorkbookPackage};
use std::fs::File;

fn usage() -> ! {
    eprintln!("Usage: streaming <OLD.xlsx> <NEW.xlsx> > out.jsonl");
    std::process::exit(2);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let old_path = args.next().unwrap_or_else(|| usage());
    let new_path = args.next().unwrap_or_else(|| usage());

    let old_pkg = WorkbookPackage::open(File::open(&old_path)?)?;
    let new_pkg = WorkbookPackage::open(File::open(&new_path)?)?;

    let stdout = std::io::stdout();
    let handle = stdout.lock();
    let mut sink = JsonLinesSink::new(handle);

    let summary = old_pkg.diff_streaming(&new_pkg, &DiffConfig::default(), &mut sink)?;

    eprintln!(
        "complete={} ops={} warnings={}",
        summary.complete,
        summary.op_count,
        summary.warnings.len()
    );
    for warning in &summary.warnings {
        eprintln!("warning: {}", warning);
    }

    Ok(())
}

