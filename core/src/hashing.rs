//! Hash utilities for row/column signature computation.
//!
//! Provides consistent hashing functions used for computing structural
//! signatures that enable efficient alignment during diffing.

use std::hash::{Hash, Hasher};
use xxhash_rust::xxh64::Xxh64;

use crate::workbook::{Cell, ColSignature, RowSignature};

pub(crate) const XXH64_SEED: u64 = 0;
const HASH_MIX_CONSTANT: u64 = 0x9e3779b97f4a7c15;

pub(crate) fn hash_cell_contribution(position: u32, cell: &Cell) -> u64 {
    let mut hasher = Xxh64::new(XXH64_SEED);
    position.hash(&mut hasher);
    cell.value.hash(&mut hasher);
    cell.formula.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn mix_hash(hash: u64) -> u64 {
    hash.rotate_left(13) ^ HASH_MIX_CONSTANT
}

pub(crate) fn combine_hashes(current: u64, contribution: u64) -> u64 {
    current.wrapping_add(mix_hash(contribution))
}

#[allow(dead_code)]
pub(crate) fn compute_row_signature<'a>(
    cells: impl Iterator<Item = (u32, &'a Cell)>,
    row: u32,
) -> RowSignature {
    let hash = cells
        .filter(|(_, cell)| cell.row == row)
        .fold(0u64, |acc, (col, cell)| {
            combine_hashes(acc, hash_cell_contribution(col, cell))
        });
    RowSignature { hash }
}

#[allow(dead_code)]
pub(crate) fn compute_col_signature<'a>(
    cells: impl Iterator<Item = (u32, &'a Cell)>,
    col: u32,
) -> ColSignature {
    let hash = cells
        .filter(|(_, cell)| cell.col == col)
        .fold(0u64, |acc, (row, cell)| {
            combine_hashes(acc, hash_cell_contribution(row, cell))
        });
    ColSignature { hash }
}
