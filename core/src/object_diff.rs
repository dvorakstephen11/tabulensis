use crate::diff::DiffOp;
use crate::package::VbaModule;
use crate::string_pool::StringPool;
use crate::workbook::{ChartObject, NamedRange, Workbook};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn diff_named_ranges(old: &Workbook, new: &Workbook, pool: &StringPool) -> Vec<DiffOp> {
    fn key(range: &NamedRange, pool: &StringPool) -> String {
        pool.resolve(range.name).to_lowercase()
    }

    let mut old_map: BTreeMap<String, &NamedRange> = BTreeMap::new();
    for range in &old.named_ranges {
        old_map.insert(key(range, pool), range);
    }

    let mut new_map: BTreeMap<String, &NamedRange> = BTreeMap::new();
    for range in &new.named_ranges {
        new_map.insert(key(range, pool), range);
    }

    let mut keys: BTreeSet<String> = BTreeSet::new();
    keys.extend(old_map.keys().cloned());
    keys.extend(new_map.keys().cloned());

    let mut ops = Vec::new();
    for k in keys {
        match (old_map.get(&k), new_map.get(&k)) {
            (None, Some(new_range)) => ops.push(DiffOp::NamedRangeAdded {
                name: new_range.name,
            }),
            (Some(old_range), None) => ops.push(DiffOp::NamedRangeRemoved {
                name: old_range.name,
            }),
            (Some(old_range), Some(new_range)) => {
                if old_range.refers_to != new_range.refers_to {
                    ops.push(DiffOp::NamedRangeChanged {
                        name: new_range.name,
                        old_ref: old_range.refers_to,
                        new_ref: new_range.refers_to,
                    });
                }
            }
            (None, None) => {}
        }
    }

    ops
}

pub(crate) fn diff_charts(old: &Workbook, new: &Workbook, pool: &StringPool) -> Vec<DiffOp> {
    fn key(chart: &ChartObject, pool: &StringPool) -> (String, String) {
        (
            pool.resolve(chart.sheet).to_lowercase(),
            pool.resolve(chart.info.name).to_lowercase(),
        )
    }

    let mut old_map: BTreeMap<(String, String), &ChartObject> = BTreeMap::new();
    for chart in &old.charts {
        old_map.insert(key(chart, pool), chart);
    }

    let mut new_map: BTreeMap<(String, String), &ChartObject> = BTreeMap::new();
    for chart in &new.charts {
        new_map.insert(key(chart, pool), chart);
    }

    let mut keys: BTreeSet<(String, String)> = BTreeSet::new();
    keys.extend(old_map.keys().cloned());
    keys.extend(new_map.keys().cloned());

    let mut ops = Vec::new();
    for k in keys {
        match (old_map.get(&k), new_map.get(&k)) {
            (None, Some(new_chart)) => ops.push(DiffOp::ChartAdded {
                sheet: new_chart.sheet,
                name: new_chart.info.name,
            }),
            (Some(old_chart), None) => ops.push(DiffOp::ChartRemoved {
                sheet: old_chart.sheet,
                name: old_chart.info.name,
            }),
            (Some(old_chart), Some(new_chart)) => {
                if old_chart.xml_hash != new_chart.xml_hash {
                    ops.push(DiffOp::ChartChanged {
                        sheet: new_chart.sheet,
                        name: new_chart.info.name,
                    });
                }
            }
            (None, None) => {}
        }
    }

    ops
}

pub(crate) fn diff_vba_modules(
    old: Option<&[VbaModule]>,
    new: Option<&[VbaModule]>,
    pool: &StringPool,
) -> Vec<DiffOp> {
    fn key(module: &VbaModule, pool: &StringPool) -> String {
        pool.resolve(module.name).to_lowercase()
    }

    fn normalize(code: &str) -> String {
        code.replace("\r\n", "\n").replace('\r', "\n")
    }

    let old = old.unwrap_or_default();
    let new = new.unwrap_or_default();

    let mut old_map: BTreeMap<String, &VbaModule> = BTreeMap::new();
    for module in old {
        old_map.insert(key(module, pool), module);
    }

    let mut new_map: BTreeMap<String, &VbaModule> = BTreeMap::new();
    for module in new {
        new_map.insert(key(module, pool), module);
    }

    let mut keys: BTreeSet<String> = BTreeSet::new();
    keys.extend(old_map.keys().cloned());
    keys.extend(new_map.keys().cloned());

    let mut ops = Vec::new();
    for k in keys {
        match (old_map.get(&k), new_map.get(&k)) {
            (None, Some(new_module)) => ops.push(DiffOp::VbaModuleAdded {
                name: new_module.name,
            }),
            (Some(old_module), None) => ops.push(DiffOp::VbaModuleRemoved {
                name: old_module.name,
            }),
            (Some(old_module), Some(new_module)) => {
                if normalize(&old_module.code) != normalize(&new_module.code) {
                    ops.push(DiffOp::VbaModuleChanged {
                        name: new_module.name,
                    });
                }
            }
            (None, None) => {}
        }
    }

    ops
}

