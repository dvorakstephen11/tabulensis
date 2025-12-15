//! Hash utilities for row/column signature computation.
//!
//! Provides consistent hashing functions used for computing structural
//! signatures that enable efficient alignment during diffing.
//!
//! # Position Independence
//!
//! Row signatures are computed by hashing cell content in column-sorted order
//! *without* including column indices. This ensures that inserting or deleting
//! columns does not invalidate row alignment.
//!
//! Column signatures similarly hash content in row-sorted order without row indices.
//!
//! # Collision Probability
//!
//! Using 128-bit xxHash3 signatures, the collision probability is ~10^-38 per pair.
//! At 50K rows, the birthday-bound collision probability is ~10^-29, which is
//! negligible for practical purposes.

use std::hash::{Hash, Hasher};
use xxhash_rust::xxh3::Xxh3;
use xxhash_rust::xxh64::Xxh64;

use crate::workbook::{CellContent, CellValue, ColSignature, RowSignature};

#[allow(dead_code)]
pub(crate) const XXH64_SEED: u64 = 0;
const HASH_MIX_CONSTANT: u64 = 0x9e3779b97f4a7c15;
const CANONICAL_NAN_BITS: u64 = 0x7FF8_0000_0000_0000;

pub(crate) fn normalize_float_for_hash(n: f64) -> u64 {
    if n.is_nan() {
        return CANONICAL_NAN_BITS;
    }
    if n == 0.0 {
        return 0u64;
    }
    let magnitude = n.abs().log10().floor() as i32;
    let scale = 10f64.powi(14 - magnitude);
    let normalized = (n * scale).round() / scale;
    normalized.to_bits()
}

pub(crate) fn hash_cell_value<H: Hasher>(value: &Option<CellValue>, state: &mut H) {
    match value {
        None => {
            3u8.hash(state);
        }
        Some(CellValue::Blank) => {
            4u8.hash(state);
        }
        Some(CellValue::Number(n)) => {
            0u8.hash(state);
            normalize_float_for_hash(*n).hash(state);
        }
        Some(CellValue::Text(s)) => {
            1u8.hash(state);
            s.hash(state);
        }
        Some(CellValue::Bool(b)) => {
            2u8.hash(state);
            b.hash(state);
        }
        Some(CellValue::Error(id)) => {
            5u8.hash(state);
            id.hash(state);
        }
    }
}

#[allow(dead_code)]
pub(crate) fn hash_cell_content(cell: &CellContent) -> u64 {
    let mut hasher = Xxh64::new(XXH64_SEED);
    hash_cell_value(&cell.value, &mut hasher);
    cell.formula.hash(&mut hasher);
    hasher.finish()
}

#[allow(dead_code)]
pub(crate) fn hash_cell_content_128(cell: &CellContent) -> u128 {
    let mut hasher = Xxh3::new();
    hash_cell_value(&cell.value, &mut hasher);
    cell.formula.hash(&mut hasher);
    hasher.digest128()
}

pub(crate) fn hash_row_content_128(cells: &[(u32, &CellContent)]) -> u128 {
    let mut hasher = Xxh3::new();
    for (_, cell) in cells.iter() {
        hash_cell_value(&cell.value, &mut hasher);
        cell.formula.hash(&mut hasher);
    }
    hasher.digest128()
}

pub(crate) fn hash_col_content_128(cells: &[&CellContent]) -> u128 {
    let mut hasher = Xxh3::new();
    for cell in cells.iter() {
        hash_cell_value(&cell.value, &mut hasher);
        cell.formula.hash(&mut hasher);
    }
    hasher.digest128()
}

pub(crate) fn hash_col_content_unordered_128(cells: &[&CellContent]) -> u128 {
    if cells.is_empty() {
        return Xxh3::new().digest128();
    }

    let mut cell_hashes: Vec<u128> = cells
        .iter()
        .map(|cell| {
            let mut h = Xxh3::new();
            hash_cell_value(&cell.value, &mut h);
            cell.formula.hash(&mut h);
            h.digest128()
        })
        .collect();

    cell_hashes.sort_unstable();

    let mut combined = Xxh3::new();
    for h in cell_hashes {
        combined.update(&h.to_le_bytes());
    }
    combined.digest128()
}

#[allow(dead_code)]
pub(crate) fn mix_hash(hash: u64) -> u64 {
    hash.rotate_left(13) ^ HASH_MIX_CONSTANT
}

#[allow(dead_code)]
pub(crate) fn mix_hash_128(hash: u128) -> u128 {
    hash.rotate_left(47) ^ (HASH_MIX_CONSTANT as u128)
}

#[allow(dead_code)]
pub(crate) fn combine_hashes(current: u64, contribution: u64) -> u64 {
    current.wrapping_add(mix_hash(contribution))
}

#[allow(dead_code)]
pub(crate) fn combine_hashes_128(current: u128, contribution: u128) -> u128 {
    current.wrapping_add(mix_hash_128(contribution))
}

#[allow(dead_code)]
pub(crate) fn compute_row_signature<'a>(
    cells: impl Iterator<Item = ((u32, u32), &'a CellContent)>,
    row: u32,
) -> RowSignature {
    let mut row_cells: Vec<(u32, &CellContent)> = cells
        .filter_map(|((r, c), cell)| (r == row).then_some((c, cell)))
        .collect();
    row_cells.sort_by_key(|(col, _)| *col);

    let hash = hash_row_content_128(&row_cells);
    RowSignature { hash }
}

#[allow(dead_code)]
pub(crate) fn compute_col_signature<'a>(
    cells: impl Iterator<Item = ((u32, u32), &'a CellContent)>,
    col: u32,
) -> ColSignature {
    let mut col_cells: Vec<(u32, &CellContent)> = cells
        .filter_map(|((r, c), cell)| (c == col).then_some((r, cell)))
        .collect();
    col_cells.sort_by_key(|(r, _)| *r);
    let ordered: Vec<&CellContent> = col_cells.into_iter().map(|(_, cell)| cell).collect();
    let hash = hash_col_content_128(&ordered);
    ColSignature { hash }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_zero_values() {
        assert_eq!(
            normalize_float_for_hash(0.0),
            normalize_float_for_hash(-0.0)
        );
        assert_eq!(normalize_float_for_hash(0.0), 0u64);
    }

    #[test]
    fn normalize_nan_values() {
        let nan1 = f64::NAN;
        let nan2 = f64::from_bits(0x7FF8_0000_0000_0001);
        assert_eq!(
            normalize_float_for_hash(nan1),
            normalize_float_for_hash(nan2)
        );
        assert_eq!(normalize_float_for_hash(nan1), CANONICAL_NAN_BITS);
    }

    #[test]
    fn normalize_ulp_drift() {
        let a = 1.0;
        let b = 1.0000000000000002;
        assert_eq!(normalize_float_for_hash(a), normalize_float_for_hash(b));
    }

    #[test]
    fn normalize_meaningful_difference() {
        let a = 1.0;
        let b = 1.0001;
        assert_ne!(normalize_float_for_hash(a), normalize_float_for_hash(b));
    }

    #[test]
    fn normalize_preserves_large_numbers() {
        let a = 1e15;
        let b = 1e15 + 1.0;
        assert_eq!(normalize_float_for_hash(a), normalize_float_for_hash(b));
    }

    #[test]
    fn normalize_distinguishes_different_magnitudes() {
        let a = 1.0;
        let b = 2.0;
        assert_ne!(normalize_float_for_hash(a), normalize_float_for_hash(b));
    }
}
