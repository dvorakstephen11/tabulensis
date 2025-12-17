use rustc_hash::FxHashMap;

use crate::config::DiffConfig;
use crate::diff::FormulaDiffResult;
use crate::formula::{FormulaExpr, parse_formula, formulas_equivalent_modulo_shift};
use crate::string_pool::{StringId, StringPool};

#[derive(Debug, Default)]
pub(crate) struct FormulaParseCache {
    parsed: FxHashMap<StringId, Option<FormulaExpr>>,
    canonical: FxHashMap<StringId, Option<FormulaExpr>>,
}

impl FormulaParseCache {
    fn parsed(&mut self, pool: &StringPool, id: StringId) -> Option<&FormulaExpr> {
        if !self.parsed.contains_key(&id) {
            let s = pool.resolve(id);
            self.parsed.insert(id, parse_formula(s).ok());
        }
        self.parsed.get(&id).and_then(|x| x.as_ref())
    }

    fn canonical(&mut self, pool: &StringPool, id: StringId) -> Option<FormulaExpr> {
        if !self.canonical.contains_key(&id) {
            let canon = self.parsed(pool, id).map(|e| e.canonicalize());
            self.canonical.insert(id, canon);
        }
        self.canonical.get(&id).and_then(|x| x.clone())
    }
}

pub(crate) fn diff_cell_formulas_ids(
    pool: &StringPool,
    cache: &mut FormulaParseCache,
    old: Option<StringId>,
    new: Option<StringId>,
    row_shift: i32,
    col_shift: i32,
    config: &DiffConfig,
) -> FormulaDiffResult {
    if old == new {
        return FormulaDiffResult::Unchanged;
    }

    match (old, new) {
        (None, Some(_)) => return FormulaDiffResult::Added,
        (Some(_), None) => return FormulaDiffResult::Removed,
        (None, None) => return FormulaDiffResult::Unchanged,
        _ => {}
    }

    if !config.enable_formula_semantic_diff {
        return FormulaDiffResult::TextChange;
    }

    let (Some(old_id), Some(new_id)) = (old, new) else {
        return FormulaDiffResult::TextChange;
    };

    let old_ast = match cache.parsed(pool, old_id) {
        Some(a) => a.clone(),
        None => return FormulaDiffResult::TextChange,
    };
    let new_ast = match cache.parsed(pool, new_id) {
        Some(a) => a.clone(),
        None => return FormulaDiffResult::TextChange,
    };

    let old_c = old_ast.canonicalize();
    let new_c = match cache.canonical(pool, new_id) {
        Some(c) => c,
        None => new_ast.canonicalize(),
    };

    if old_c == new_c {
        return FormulaDiffResult::FormattingOnly;
    }

    if row_shift != 0 || col_shift != 0 {
        if formulas_equivalent_modulo_shift(&old_ast, &new_ast, row_shift, col_shift) {
            return FormulaDiffResult::Filled;
        }
    }

    FormulaDiffResult::SemanticChange
}
