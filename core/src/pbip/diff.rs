use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use xxhash_rust::xxh3::xxh3_64;

use super::types::{PbipChangeKind, PbipDocDiff, PbipDocRecord, PbipDocSnapshot, PbipDocType, PbipProjectSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PbipEntityKind {
    Report,
    Page,
    Visual,
    Theme,
    Bookmark,
    Model,
    Table,
    Column,
    Measure,
    Relationship,
    Other,
}

#[derive(Debug, Clone)]
pub struct PbipEntityDiff {
    pub entity_kind: PbipEntityKind,
    pub label: String,
    pub change_kind: PbipChangeKind,
    pub doc_path: String,
    pub pointer: Option<String>,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PbipDiffReport {
    pub docs: Vec<PbipDocDiff>,
    pub entities: Vec<PbipEntityDiff>,
}

pub fn diff_snapshots(old: &PbipProjectSnapshot, new: &PbipProjectSnapshot) -> PbipDiffReport {
    let mut report = PbipDiffReport::default();

    // Convert snapshots into path-keyed maps for diffing.
    let mut old_map: std::collections::BTreeMap<&str, &PbipDocRecord> = std::collections::BTreeMap::new();
    let mut new_map: std::collections::BTreeMap<&str, &PbipDocRecord> = std::collections::BTreeMap::new();
    for doc in &old.docs {
        old_map.insert(doc.path.as_str(), doc);
    }
    for doc in &new.docs {
        new_map.insert(doc.path.as_str(), doc);
    }

    let mut paths: Vec<&str> = old_map
        .keys()
        .copied()
        .chain(new_map.keys().copied())
        .collect();
    paths.sort();
    paths.dedup();

    for path in paths {
        let old_doc = old_map.get(path).copied();
        let new_doc = new_map.get(path).copied();

        let (doc_type, change_kind, old_snap, new_snap) = match (old_doc, new_doc) {
            (None, None) => continue,
            (Some(old_doc), None) => (
                old_doc.doc_type,
                PbipChangeKind::Removed,
                Some(old_doc.snapshot.clone()),
                None,
            ),
            (None, Some(new_doc)) => (
                new_doc.doc_type,
                PbipChangeKind::Added,
                None,
                Some(new_doc.snapshot.clone()),
            ),
            (Some(old_doc), Some(new_doc)) => {
                let doc_type = merge_doc_type(old_doc.doc_type, new_doc.doc_type);
                let same = old_doc.snapshot.hash == new_doc.snapshot.hash
                    && old_doc.snapshot.error == new_doc.snapshot.error;
                let kind = if same {
                    PbipChangeKind::Unchanged
                } else {
                    PbipChangeKind::Modified
                };
                (
                    doc_type,
                    kind,
                    Some(old_doc.snapshot.clone()),
                    Some(new_doc.snapshot.clone()),
                )
            }
        };

        if change_kind == PbipChangeKind::Unchanged {
            continue;
        }

        let impact_hint = match doc_type {
            PbipDocType::Pbir => Some("Report".to_string()),
            PbipDocType::Tmdl => Some("Model".to_string()),
            PbipDocType::Other => None,
        };

        let doc_diff = PbipDocDiff {
            path: path.to_string(),
            doc_type,
            change_kind,
            impact_hint,
            old: old_snap,
            new: new_snap,
        };

        // Best-effort entity extraction. This is deterministic and never panics; failures simply
        // yield no entities for that document. (Iteration 2 MVP starts document-first, then
        // deepens semantics incrementally.)
        report
            .entities
            .extend(diff_entities_for_doc(&doc_diff));

        report.docs.push(doc_diff);
    }

    report
}

pub fn hash_text(text: &str) -> u64 {
    xxh3_64(text.as_bytes())
}

fn merge_doc_type(old: PbipDocType, new: PbipDocType) -> PbipDocType {
    if old == new {
        return old;
    }
    if old == PbipDocType::Other {
        return new;
    }
    if new == PbipDocType::Other {
        return old;
    }
    // Prefer PBIR over TMDL only when inconsistent; treat as Other as the safe fallback.
    PbipDocType::Other
}

#[derive(Debug, Clone)]
struct EntitySnapshot {
    kind: PbipEntityKind,
    label: String,
    pointer: Option<String>,
    text: String,
    hash: u64,
}

fn diff_entities_for_doc(doc: &PbipDocDiff) -> Vec<PbipEntityDiff> {
    let old_entities = extract_entities(doc.doc_type, doc.old.as_ref());
    let new_entities = extract_entities(doc.doc_type, doc.new.as_ref());

    let mut keys: BTreeSet<(PbipEntityKind, String)> = BTreeSet::new();
    for (k, _v) in old_entities.iter() {
        keys.insert(k.clone());
    }
    for (k, _v) in new_entities.iter() {
        keys.insert(k.clone());
    }

    let mut out: Vec<PbipEntityDiff> = Vec::new();
    for (kind, label) in keys {
        let old = old_entities.get(&(kind, label.clone()));
        let new = new_entities.get(&(kind, label.clone()));

        let (change_kind, pointer, old_text, new_text) = match (old, new) {
            (None, None) => continue,
            (Some(old), None) => (
                PbipChangeKind::Removed,
                old.pointer.clone(),
                Some(old.text.clone()),
                None,
            ),
            (None, Some(new)) => (
                PbipChangeKind::Added,
                new.pointer.clone(),
                None,
                Some(new.text.clone()),
            ),
            (Some(old), Some(new)) => {
                if old.hash == new.hash {
                    continue;
                }
                (
                    PbipChangeKind::Modified,
                    new.pointer.clone().or_else(|| old.pointer.clone()),
                    Some(old.text.clone()),
                    Some(new.text.clone()),
                )
            }
        };

        out.push(PbipEntityDiff {
            entity_kind: kind,
            label,
            change_kind,
            doc_path: doc.path.clone(),
            pointer,
            old_text,
            new_text,
        });
    }
    out
}

fn extract_entities(
    doc_type: PbipDocType,
    snap: Option<&PbipDocSnapshot>,
) -> BTreeMap<(PbipEntityKind, String), EntitySnapshot> {
    let Some(snap) = snap else {
        return BTreeMap::new();
    };
    if snap.error.is_some() {
        return BTreeMap::new();
    }
    let text = snap.normalized_text.as_str();
    if text.trim().is_empty() {
        return BTreeMap::new();
    }

    let mut entities: Vec<EntitySnapshot> = match doc_type {
        PbipDocType::Pbir => extract_pbir_entities(text),
        PbipDocType::Tmdl => extract_tmdl_entities(text),
        PbipDocType::Other => Vec::new(),
    };

    // Deterministic de-dup: if multiple entities share the same (kind,label), retain the first.
    let mut out: BTreeMap<(PbipEntityKind, String), EntitySnapshot> = BTreeMap::new();
    for entity in entities.drain(..) {
        let key = (entity.kind, entity.label.clone());
        out.entry(key).or_insert(entity);
    }
    out
}

fn extract_pbir_entities(text: &str) -> Vec<EntitySnapshot> {
    let parsed: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out: Vec<EntitySnapshot> = Vec::new();
    let Value::Object(map) = parsed else {
        return out;
    };

    if let Some(Value::Array(pages)) = map.get("pages") {
        for (idx, page) in pages.iter().enumerate() {
            let label = page
                .get("displayName")
                .and_then(|v| v.as_str())
                .or_else(|| page.get("name").and_then(|v| v.as_str()))
                .unwrap_or("page")
                .to_string();
            let pointer = Some(format!("/pages/{idx}"));
            let text = serde_json::to_string_pretty(page).unwrap_or_else(|_| "{}".to_string());
            let text = ensure_trailing_newline(&text);
            out.push(EntitySnapshot {
                kind: PbipEntityKind::Page,
                label,
                pointer,
                hash: hash_text(&text),
                text,
            });
        }
    }

    if let Some(Value::Array(visuals)) = map.get("visuals") {
        for (idx, visual) in visuals.iter().enumerate() {
            let label = visual
                .get("title")
                .and_then(|v| v.as_str())
                .or_else(|| visual.get("name").and_then(|v| v.as_str()))
                .or_else(|| visual.get("id").and_then(|v| v.as_str()))
                .unwrap_or("visual")
                .to_string();
            let pointer = Some(format!("/visuals/{idx}"));
            let text = serde_json::to_string_pretty(visual).unwrap_or_else(|_| "{}".to_string());
            let text = ensure_trailing_newline(&text);
            out.push(EntitySnapshot {
                kind: PbipEntityKind::Visual,
                label,
                pointer,
                hash: hash_text(&text),
                text,
            });
        }
    }

    // Theme (best-effort): treat the root `theme` object as one entity.
    if let Some(theme) = map.get("theme") {
        let label = theme
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("theme")
            .to_string();
        let pointer = Some("/theme".to_string());
        let text = serde_json::to_string_pretty(theme).unwrap_or_else(|_| "{}".to_string());
        let text = ensure_trailing_newline(&text);
        out.push(EntitySnapshot {
            kind: PbipEntityKind::Theme,
            label,
            pointer,
            hash: hash_text(&text),
            text,
        });
    }

    // Bookmarks (best-effort): recognize a top-level `bookmarks` array.
    if let Some(Value::Array(bookmarks)) = map.get("bookmarks") {
        for (idx, bookmark) in bookmarks.iter().enumerate() {
            let label = bookmark
                .get("displayName")
                .and_then(|v| v.as_str())
                .or_else(|| bookmark.get("name").and_then(|v| v.as_str()))
                .unwrap_or("bookmark")
                .to_string();
            let pointer = Some(format!("/bookmarks/{idx}"));
            let text =
                serde_json::to_string_pretty(bookmark).unwrap_or_else(|_| "{}".to_string());
            let text = ensure_trailing_newline(&text);
            out.push(EntitySnapshot {
                kind: PbipEntityKind::Bookmark,
                label,
                pointer,
                hash: hash_text(&text),
                text,
            });
        }
    }

    out
}

fn extract_tmdl_entities(text: &str) -> Vec<EntitySnapshot> {
    let mut out: Vec<EntitySnapshot> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();

    let mut current_table: Option<String> = None;
    let mut current_table_indent: Option<usize> = None;

    let mut idx: usize = 0;
    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());
        if trimmed.trim().is_empty() {
            idx += 1;
            continue;
        }

        if let (Some(table_indent), Some(_)) = (current_table_indent, current_table.as_ref()) {
            if indent <= table_indent && !trimmed.starts_with("table ") {
                current_table = None;
                current_table_indent = None;
            }
        }

        if let Some(name) = trimmed.strip_prefix("table ").map(str::trim).filter(|v| !v.is_empty())
        {
            current_table = Some(name.to_string());
            current_table_indent = Some(indent);
            let (block, next) = capture_indent_block(&lines, idx, indent);
            let label = name.to_string();
            out.push(EntitySnapshot {
                kind: PbipEntityKind::Table,
                label,
                pointer: Some(format!("table:{name}")),
                hash: hash_text(&block),
                text: block,
            });
            // Keep scanning inside the table block so we can also extract measures, etc.
            // (Table block capture is for Details; entity extraction should remain incremental.)
            let _ = next;
            idx += 1;
            continue;
        }

        if let Some(name) = trimmed
            .strip_prefix("measure ")
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let (block, next) = capture_indent_block(&lines, idx, indent);
            let label = match current_table.as_deref() {
                Some(table) => format!("{table}.{name}"),
                None => name.to_string(),
            };
            out.push(EntitySnapshot {
                kind: PbipEntityKind::Measure,
                label,
                pointer: Some(format!("measure:{name}")),
                hash: hash_text(&block),
                text: block,
            });
            idx = next;
            continue;
        }

        if let Some(name) = trimmed
            .strip_prefix("column ")
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let (block, next) = capture_indent_block(&lines, idx, indent);
            let label = match current_table.as_deref() {
                Some(table) => format!("{table}.{name}"),
                None => name.to_string(),
            };
            out.push(EntitySnapshot {
                kind: PbipEntityKind::Column,
                label,
                pointer: Some(format!("column:{name}")),
                hash: hash_text(&block),
                text: block,
            });
            idx = next;
            continue;
        }

        if let Some(name) = trimmed
            .strip_prefix("relationship ")
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let (block, next) = capture_indent_block(&lines, idx, indent);
            out.push(EntitySnapshot {
                kind: PbipEntityKind::Relationship,
                label: name.to_string(),
                pointer: Some(format!("relationship:{name}")),
                hash: hash_text(&block),
                text: block,
            });
            idx = next;
            continue;
        }

        idx += 1;
    }

    out
}

fn capture_indent_block(lines: &[&str], start_idx: usize, start_indent: usize) -> (String, usize) {
    let mut end = start_idx + 1;
    while end < lines.len() {
        let line = lines[end];
        let trimmed = line.trim_start();
        if trimmed.trim().is_empty() {
            end += 1;
            continue;
        }
        let indent = line.len().saturating_sub(trimmed.len());
        if indent <= start_indent {
            break;
        }
        end += 1;
    }

    let mut block = lines[start_idx..end].join("\n");
    block = ensure_trailing_newline(&block);
    (block, end)
}

fn ensure_trailing_newline(text: &str) -> String {
    if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
}

#[allow(dead_code)]
fn snapshot_from_text(doc_type: PbipDocType, normalized_text: String, error: Option<String>, normalization_applied: Option<String>) -> PbipDocSnapshot {
    let hash = hash_text(&normalized_text);
    PbipDocSnapshot {
        doc_type,
        normalized_text,
        hash,
        error,
        normalization_applied,
    }
}
