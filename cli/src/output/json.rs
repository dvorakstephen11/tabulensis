use anyhow::Result;
use excel_diff::DiffReport;
use std::io::Write;

pub fn write_json_report<W: Write>(w: &mut W, report: &DiffReport) -> Result<()> {
    serde_json::to_writer_pretty(&mut *w, report)?;
    writeln!(w)?;
    Ok(())
}

