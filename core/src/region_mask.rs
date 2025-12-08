//! Region mask for tracking which cells have been accounted for during diff.
//!
//! The `RegionMask` tracks which rows and columns are "active" (still to be processed)
//! versus "excluded" (already accounted for by a move or other operation).

use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
struct RectMask {
    row_start: u32,
    row_count: u32,
    col_start: u32,
    col_count: u32,
}

#[derive(Debug, Clone)]
pub struct RegionMask {
    excluded_rows: HashSet<u32>,
    excluded_cols: HashSet<u32>,
    excluded_rects: Vec<RectMask>,
    #[allow(dead_code)]
    nrows: u32,
    #[allow(dead_code)]
    ncols: u32,
    row_shift_min: Option<u32>,
    row_shift_max: Option<u32>,
    col_shift_min: Option<u32>,
    col_shift_max: Option<u32>,
}

#[allow(dead_code)]
impl RegionMask {
    pub fn all_active(nrows: u32, ncols: u32) -> Self {
        Self {
            excluded_rows: HashSet::new(),
            excluded_cols: HashSet::new(),
            excluded_rects: Vec::new(),
            nrows,
            ncols,
            row_shift_min: None,
            row_shift_max: None,
            col_shift_min: None,
            col_shift_max: None,
        }
    }

    pub fn exclude_row(&mut self, row: u32) {
        self.excluded_rows.insert(row);
    }

    pub fn exclude_rows(&mut self, start: u32, count: u32) {
        let end = start.saturating_add(count).saturating_sub(1);
        for row in start..=end {
            self.excluded_rows.insert(row);
        }
        self.row_shift_min = Some(self.row_shift_min.map_or(start, |m| m.min(start)));
        self.row_shift_max = Some(self.row_shift_max.map_or(end, |m| m.max(end)));
    }

    pub fn exclude_col(&mut self, col: u32) {
        self.excluded_cols.insert(col);
    }

    pub fn exclude_cols(&mut self, start: u32, count: u32) {
        let end = start.saturating_add(count).saturating_sub(1);
        for col in start..=end {
            self.excluded_cols.insert(col);
        }
        self.col_shift_min = Some(self.col_shift_min.map_or(start, |m| m.min(start)));
        self.col_shift_max = Some(self.col_shift_max.map_or(end, |m| m.max(end)));
    }

    pub fn exclude_rect(&mut self, row_start: u32, row_count: u32, col_start: u32, col_count: u32) {
        self.exclude_rows(row_start, row_count);
        self.exclude_cols(col_start, col_count);
    }

    pub fn exclude_rect_cells(
        &mut self,
        row_start: u32,
        row_count: u32,
        col_start: u32,
        col_count: u32,
    ) {
        self.excluded_rects.push(RectMask {
            row_start,
            row_count,
            col_start,
            col_count,
        });
    }

    pub fn is_row_active(&self, row: u32) -> bool {
        !self.excluded_rows.contains(&row)
    }

    pub fn is_col_active(&self, col: u32) -> bool {
        !self.excluded_cols.contains(&col)
    }

    fn is_cell_excluded_by_rects(&self, row: u32, col: u32) -> bool {
        self.excluded_rects.iter().any(|rect| {
            row >= rect.row_start
                && row < rect.row_start.saturating_add(rect.row_count)
                && col >= rect.col_start
                && col < rect.col_start.saturating_add(rect.col_count)
        })
    }

    pub fn is_cell_active(&self, row: u32, col: u32) -> bool {
        self.is_row_active(row)
            && self.is_col_active(col)
            && !self.is_cell_excluded_by_rects(row, col)
    }

    pub fn active_row_count(&self) -> u32 {
        self.nrows.saturating_sub(self.excluded_rows.len() as u32)
    }

    pub fn active_col_count(&self) -> u32 {
        self.ncols.saturating_sub(self.excluded_cols.len() as u32)
    }

    pub fn active_rows(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.nrows).filter(|r| self.is_row_active(*r))
    }

    pub fn active_cols(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.ncols).filter(|c| self.is_col_active(*c))
    }

    pub fn has_excluded_rows(&self) -> bool {
        !self.excluded_rows.is_empty()
    }

    pub fn has_excluded_cols(&self) -> bool {
        !self.excluded_cols.is_empty()
    }

    pub fn has_excluded_rects(&self) -> bool {
        !self.excluded_rects.is_empty()
    }

    pub fn has_exclusions(&self) -> bool {
        self.has_excluded_rows() || self.has_excluded_cols() || self.has_excluded_rects()
    }

    pub fn has_active_cells(&self) -> bool {
        self.active_row_count() > 0 && self.active_col_count() > 0
    }

    pub fn rows_overlap_excluded(&self, start: u32, count: u32) -> bool {
        for row in start..start.saturating_add(count) {
            if self.excluded_rows.contains(&row) {
                return true;
            }
        }
        false
    }

    pub fn cols_overlap_excluded(&self, start: u32, count: u32) -> bool {
        for col in start..start.saturating_add(count) {
            if self.excluded_cols.contains(&col) {
                return true;
            }
        }
        false
    }

    pub fn rect_overlaps_excluded(
        &self,
        row_start: u32,
        row_count: u32,
        col_start: u32,
        col_count: u32,
    ) -> bool {
        self.rows_overlap_excluded(row_start, row_count)
            || self.cols_overlap_excluded(col_start, col_count)
    }

    pub fn is_row_in_shift_zone(&self, row: u32) -> bool {
        match (self.row_shift_min, self.row_shift_max) {
            (Some(min), Some(max)) => row >= min && row <= max,
            _ => false,
        }
    }

    pub fn is_col_in_shift_zone(&self, col: u32) -> bool {
        match (self.col_shift_min, self.col_shift_max) {
            (Some(min), Some(max)) => col >= min && col <= max,
            _ => false,
        }
    }

    pub fn row_shift_bounds(&self) -> Option<(u32, u32)> {
        match (self.row_shift_min, self.row_shift_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    pub fn col_shift_bounds(&self) -> Option<(u32, u32)> {
        match (self.col_shift_min, self.col_shift_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_active_initially() {
        let mask = RegionMask::all_active(10, 5);
        assert!(mask.is_row_active(0));
        assert!(mask.is_row_active(9));
        assert!(mask.is_col_active(0));
        assert!(mask.is_col_active(4));
        assert_eq!(mask.active_row_count(), 10);
        assert_eq!(mask.active_col_count(), 5);
    }

    #[test]
    fn exclude_single_row() {
        let mut mask = RegionMask::all_active(10, 5);
        mask.exclude_row(3);
        assert!(!mask.is_row_active(3));
        assert!(mask.is_row_active(2));
        assert!(mask.is_row_active(4));
        assert_eq!(mask.active_row_count(), 9);
    }

    #[test]
    fn exclude_row_range() {
        let mut mask = RegionMask::all_active(10, 5);
        mask.exclude_rows(2, 4);
        assert!(!mask.is_row_active(2));
        assert!(!mask.is_row_active(5));
        assert!(mask.is_row_active(1));
        assert!(mask.is_row_active(6));
        assert_eq!(mask.active_row_count(), 6);
    }

    #[test]
    fn exclude_rect() {
        let mut mask = RegionMask::all_active(10, 8);
        mask.exclude_rect(2, 3, 4, 2);
        assert!(!mask.is_row_active(2));
        assert!(!mask.is_row_active(4));
        assert!(mask.is_row_active(1));
        assert!(mask.is_row_active(5));
        assert!(!mask.is_col_active(4));
        assert!(!mask.is_col_active(5));
        assert!(mask.is_col_active(3));
        assert!(mask.is_col_active(6));
    }

    #[test]
    fn cell_active_based_on_row_and_col() {
        let mut mask = RegionMask::all_active(10, 10);
        mask.exclude_row(3);
        mask.exclude_col(5);
        assert!(!mask.is_cell_active(3, 5));
        assert!(!mask.is_cell_active(3, 0));
        assert!(!mask.is_cell_active(0, 5));
        assert!(mask.is_cell_active(0, 0));
        assert!(mask.is_cell_active(4, 6));
    }

    #[test]
    fn active_rows_iterator() {
        let mut mask = RegionMask::all_active(5, 3);
        mask.exclude_row(1);
        mask.exclude_row(3);
        let active: Vec<u32> = mask.active_rows().collect();
        assert_eq!(active, vec![0, 2, 4]);
    }

    #[test]
    fn rows_overlap_excluded_detects_overlap() {
        let mut mask = RegionMask::all_active(10, 5);
        mask.exclude_rows(3, 2);
        assert!(mask.rows_overlap_excluded(2, 3));
        assert!(mask.rows_overlap_excluded(4, 2));
        assert!(!mask.rows_overlap_excluded(0, 2));
        assert!(!mask.rows_overlap_excluded(5, 3));
    }

    #[test]
    fn cols_overlap_excluded_detects_overlap() {
        let mut mask = RegionMask::all_active(5, 10);
        mask.exclude_cols(4, 3);
        assert!(mask.cols_overlap_excluded(3, 2));
        assert!(mask.cols_overlap_excluded(6, 2));
        assert!(!mask.cols_overlap_excluded(0, 3));
        assert!(!mask.cols_overlap_excluded(7, 3));
    }

    #[test]
    fn rect_overlaps_excluded_detects_any_overlap() {
        let mut mask = RegionMask::all_active(10, 10);
        mask.exclude_rect(2, 3, 4, 2);
        assert!(mask.rect_overlaps_excluded(1, 2, 0, 3));
        assert!(mask.rect_overlaps_excluded(0, 2, 3, 2));
        assert!(!mask.rect_overlaps_excluded(6, 2, 7, 2));
    }

    #[test]
    fn exclude_rect_cells_masks_only_that_region() {
        let mut mask = RegionMask::all_active(6, 6);
        mask.exclude_rect_cells(2, 2, 2, 2);

        assert!(!mask.is_cell_active(2, 2));
        assert!(!mask.is_cell_active(3, 3));

        assert!(mask.is_cell_active(1, 2));
        assert!(mask.is_cell_active(2, 1));
        assert!(mask.is_cell_active(4, 4));

        assert!(mask.has_exclusions());
        assert!(mask.has_excluded_rects());
    }
}
