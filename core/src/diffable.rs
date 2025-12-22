//! Lightweight Diffable trait for component-level diffs.
//!
//! This exposes a config-aware entry point so callers can diff grids, sheets, or
//! workbooks without standing up a full engine session.

use crate::config::DiffConfig;
use crate::diff::DiffReport;
use crate::string_pool::StringPool;
use crate::workbook::{Grid, Sheet, SheetKind, Workbook};

/// Shared context for Diffable implementations.
pub struct DiffContext<'a> {
    pub config: &'a DiffConfig,
    pub pool: &'a mut StringPool,
}

impl<'a> DiffContext<'a> {
    pub fn new(pool: &'a mut StringPool, config: &'a DiffConfig) -> Self {
        Self { config, pool }
    }
}

/// Component-level diff trait.
pub trait Diffable {
    type Output;

    /// Diff `self` against `other` using the supplied context.
    fn diff(&self, other: &Self, ctx: &mut DiffContext<'_>) -> Self::Output;
}

impl Diffable for Workbook {
    type Output = DiffReport;

    fn diff(&self, other: &Self, ctx: &mut DiffContext<'_>) -> DiffReport {
        crate::engine::diff_workbooks(self, other, ctx.pool, ctx.config)
    }
}

impl Diffable for Sheet {
    type Output = DiffReport;

    fn diff(&self, other: &Self, ctx: &mut DiffContext<'_>) -> DiffReport {
        let wb_a = Workbook {
            sheets: vec![self.clone()],
            ..Default::default()
        };
        let wb_b = Workbook {
            sheets: vec![other.clone()],
            ..Default::default()
        };
        crate::engine::diff_workbooks(&wb_a, &wb_b, ctx.pool, ctx.config)
    }
}

impl Diffable for Grid {
    type Output = DiffReport;

    fn diff(&self, other: &Self, ctx: &mut DiffContext<'_>) -> DiffReport {
        let sheet_id = ctx.pool.intern("<grid>");
        let wb_a = Workbook {
            sheets: vec![Sheet {
                name: sheet_id,
                kind: SheetKind::Worksheet,
                grid: self.clone(),
            }],
            ..Default::default()
        };
        let wb_b = Workbook {
            sheets: vec![Sheet {
                name: sheet_id,
                kind: SheetKind::Worksheet,
                grid: other.clone(),
            }],
            ..Default::default()
        };
        crate::engine::diff_workbooks(&wb_a, &wb_b, ctx.pool, ctx.config)
    }
}
