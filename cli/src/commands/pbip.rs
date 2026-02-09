use crate::{PbipCommands, PbipProfileArg};
use anyhow::{bail, Context, Result};
use license_client::LicenseClient;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

pub fn run(command: PbipCommands) -> Result<ExitCode> {
    match command {
        PbipCommands::Normalize {
            path,
            profile,
            header,
            fail_on_error,
        } => normalize(&path, profile, header, fail_on_error),
        PbipCommands::Diff {
            old,
            new,
            profile,
            markdown,
            docs_only,
        } => diff_dirs(&old, &new, profile, markdown, docs_only),
    }
}

fn normalize(
    path: &str,
    profile: PbipProfileArg,
    header: bool,
    fail_on_error: bool,
) -> Result<ExitCode> {
    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();

    let doc_type = match ext.as_str() {
        "pbir" => excel_diff::PbipDocType::Pbir,
        "tmdl" => excel_diff::PbipDocType::Tmdl,
        other => bail!(
            "Unsupported PBIP artifact extension '{other}'. Expected .pbir or .tmdl."
        ),
    };

    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read input: {}", path.display()))?;

    let profile = profile.to_profile();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    match excel_diff::normalize_doc_text(doc_type, &raw, profile) {
        Ok((normalized, applied)) => {
            if header {
                writeln!(out, "# Tabulensis PBIP normalize")?;
                writeln!(out, "# Path: {}", path.display())?;
                writeln!(out, "# Profile: {}", applied.profile.as_str())?;
                writeln!(out, "# Applied: {}", applied.summary)?;
                writeln!(out)?;
            }
            write!(out, "{normalized}")?;
            Ok(ExitCode::from(0))
        }
        Err(err) => {
            // For Git textconv usage, never crash hard: emit a deterministic fallback.
            // (Strict profile avoids PBIR JSON parsing and always returns a stable text form.)
            let fallback = excel_diff::normalize_doc_text(
                doc_type,
                &raw,
                excel_diff::PbipNormalizationProfile::Strict,
            )
            .map(|(text, _)| text)
            .unwrap_or_else(|_| raw.replace("\r\n", "\n").replace('\r', "\n"));

            if header {
                writeln!(out, "# Tabulensis PBIP normalize")?;
                writeln!(out, "# Path: {}", path.display())?;
                writeln!(out, "# Profile: {}", profile.as_str())?;
                writeln!(out, "# Error: {}", err.message)?;
                writeln!(out)?;
            }
            write!(out, "{fallback}")?;

            if fail_on_error {
                Ok(ExitCode::from(2))
            } else {
                Ok(ExitCode::from(0))
            }
        }
    }
}

fn diff_dirs(
    old: &str,
    new: &str,
    profile: PbipProfileArg,
    markdown: bool,
    docs_only: bool,
) -> Result<ExitCode> {
    let license_client =
        LicenseClient::from_env().context("Failed to initialize license client")?;
    license_client
        .ensure_valid_or_refresh()
        .context("License check failed. Run `tabulensis license activate <KEY>`.")?;

    let old_path = Path::new(old);
    let new_path = Path::new(new);
    if !old_path.is_dir() || !new_path.is_dir() {
        bail!("PBIP diff requires both OLD and NEW to be folders.");
    }

    let profile = profile.to_profile();
    let scan_cfg = excel_diff::PbipScanConfig::default();
    let old_snap = excel_diff::snapshot_pbip_project(old_path, profile, scan_cfg.clone())
        .with_context(|| format!("Failed to scan PBIP folder: {}", old_path.display()))?;
    let new_snap = excel_diff::snapshot_pbip_project(new_path, profile, scan_cfg)
        .with_context(|| format!("Failed to scan PBIP folder: {}", new_path.display()))?;

    let report = excel_diff::diff_pbip_snapshots(&old_snap, &new_snap);

    let mut doc_counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    for doc in &report.docs {
        let key = doc.change_kind.as_str();
        *doc_counts.entry(key).or_insert(0) += 1;
    }

    let mut entity_rollup: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    let mut per_doc_entity: BTreeMap<String, BTreeMap<String, (u64, u64, u64)>> = BTreeMap::new();
    if !docs_only {
        for ent in &report.entities {
            let kind = format!("{:?}", ent.entity_kind).to_ascii_lowercase();
            let entry = entity_rollup.entry(kind.clone()).or_insert((0, 0, 0));
            match ent.change_kind.as_str() {
                "added" => entry.0 = entry.0.saturating_add(1),
                "removed" => entry.1 = entry.1.saturating_add(1),
                "modified" => entry.2 = entry.2.saturating_add(1),
                _ => {}
            }

            let doc_map = per_doc_entity
                .entry(ent.doc_path.clone())
                .or_insert_with(BTreeMap::new);
            let per = doc_map.entry(kind).or_insert((0, 0, 0));
            match ent.change_kind.as_str() {
                "added" => per.0 = per.0.saturating_add(1),
                "removed" => per.1 = per.1.saturating_add(1),
                "modified" => per.2 = per.2.saturating_add(1),
                _ => {}
            }
        }
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if markdown {
        writeln!(out, "# PBIP Diff")?;
        writeln!(out)?;
        writeln!(out, "- Old: `{}`", old_path.display())?;
        writeln!(out, "- New: `{}`", new_path.display())?;
        writeln!(out, "- Profile: `{}`", profile.as_str())?;
        writeln!(out)?;
        writeln!(
            out,
            "## Document changes (added/removed/modified)\n\n- added: {}\n- removed: {}\n- modified: {}",
            doc_counts.get("added").copied().unwrap_or(0),
            doc_counts.get("removed").copied().unwrap_or(0),
            doc_counts.get("modified").copied().unwrap_or(0)
        )?;
        writeln!(out)?;
        writeln!(out, "## Changed documents")?;
        writeln!(out)?;
        for doc in &report.docs {
            writeln!(
                out,
                "- `{}` ({}, {})",
                doc.path,
                doc.doc_type.as_str(),
                doc.change_kind.as_str()
            )?;
            if let Some(err) = doc.old.as_ref().and_then(|s| s.error.as_deref()) {
                writeln!(out, "  Old error: `{}`", err.replace('`', "'"))?;
            }
            if let Some(err) = doc.new.as_ref().and_then(|s| s.error.as_deref()) {
                writeln!(out, "  New error: `{}`", err.replace('`', "'"))?;
            }
            if !docs_only {
                if let Some(kinds) = per_doc_entity.get(&doc.path) {
                    let mut parts: Vec<String> = Vec::new();
                    for (kind, (a, r, m)) in kinds.iter() {
                        parts.push(format!("{kind}: +{a} -{r} ~{m}"));
                    }
                    if !parts.is_empty() {
                        writeln!(out, "  Entities: {}", parts.join("; "))?;
                    }
                }
            }
        }

        if !docs_only && !entity_rollup.is_empty() {
            writeln!(out)?;
            writeln!(out, "## Entity rollup")?;
            writeln!(out)?;
            for (kind, (a, r, m)) in entity_rollup.iter() {
                writeln!(out, "- {kind}: +{a} -{r} ~{m}")?;
            }
        }
    } else {
        writeln!(out, "PBIP Diff")?;
        writeln!(out, "Old: {}", old_path.display())?;
        writeln!(out, "New: {}", new_path.display())?;
        writeln!(out, "Profile: {}", profile.as_str())?;
        writeln!(
            out,
            "Docs: added={} removed={} modified={}",
            doc_counts.get("added").copied().unwrap_or(0),
            doc_counts.get("removed").copied().unwrap_or(0),
            doc_counts.get("modified").copied().unwrap_or(0)
        )?;
        if !docs_only && !entity_rollup.is_empty() {
            writeln!(out)?;
            writeln!(out, "Entity rollup:")?;
            for (kind, (a, r, m)) in entity_rollup.iter() {
                writeln!(out, "  - {kind}: +{a} -{r} ~{m}")?;
            }
        }
        writeln!(out)?;
        writeln!(out, "Changed documents:")?;
        for doc in &report.docs {
            writeln!(
                out,
                "- {} ({}, {})",
                doc.path,
                doc.doc_type.as_str(),
                doc.change_kind.as_str()
            )?;
            if let Some(err) = doc.old.as_ref().and_then(|s| s.error.as_deref()) {
                writeln!(out, "  Old error: {}", err)?;
            }
            if let Some(err) = doc.new.as_ref().and_then(|s| s.error.as_deref()) {
                writeln!(out, "  New error: {}", err)?;
            }
        }
    }

    let has_doc_errors = report.docs.iter().any(|doc| {
        doc.old.as_ref().and_then(|s| s.error.as_deref()).is_some()
            || doc.new.as_ref().and_then(|s| s.error.as_deref()).is_some()
    });

    Ok(if has_doc_errors {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    })
}
