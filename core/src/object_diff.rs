use crate::diff::DiffOp;
use crate::string_pool::StringPool;
use crate::vba::VbaModule;
use crate::workbook::{ChartObject, NamedRange, Workbook};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

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
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct ChartIdKey {
        sheet_id: u32,
        chart_name_lower: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct ChartNameKey {
        sheet_lower: String,
        chart_name_lower: String,
    }

    fn chart_ptr(chart: &ChartObject) -> *const ChartObject {
        chart as *const ChartObject
    }

    fn chart_name_lower(chart: &ChartObject, pool: &StringPool) -> String {
        pool.resolve(chart.info.name).to_lowercase()
    }

    fn sheet_name_lower(chart: &ChartObject, pool: &StringPool) -> String {
        pool.resolve(chart.sheet).to_lowercase()
    }

    fn ambiguous_sheet_ids(charts: &[ChartObject], pool: &StringPool) -> HashSet<u32> {
        let mut id_to_sheet: HashMap<u32, String> = HashMap::new();
        let mut ambiguous: HashSet<u32> = HashSet::new();
        for chart in charts {
            let Some(sheet_id) = chart.workbook_sheet_id else {
                continue;
            };
            let sheet_lower = sheet_name_lower(chart, pool);
            if let Some(existing) = id_to_sheet.get(&sheet_id) {
                if existing != &sheet_lower {
                    ambiguous.insert(sheet_id);
                }
            } else {
                id_to_sheet.insert(sheet_id, sheet_lower);
            }
        }
        ambiguous
    }

    let ambiguous_old = ambiguous_sheet_ids(&old.charts, pool);
    let ambiguous_new = ambiguous_sheet_ids(&new.charts, pool);

    let mut old_by_id: BTreeMap<ChartIdKey, &ChartObject> = BTreeMap::new();
    for chart in &old.charts {
        let Some(sheet_id) = chart.workbook_sheet_id else {
            continue;
        };
        if ambiguous_old.contains(&sheet_id) {
            continue;
        }
        let key = ChartIdKey {
            sheet_id,
            chart_name_lower: chart_name_lower(chart, pool),
        };
        old_by_id.insert(key, chart);
    }

    let mut new_by_id: BTreeMap<ChartIdKey, &ChartObject> = BTreeMap::new();
    for chart in &new.charts {
        let Some(sheet_id) = chart.workbook_sheet_id else {
            continue;
        };
        if ambiguous_new.contains(&sheet_id) {
            continue;
        }
        let key = ChartIdKey {
            sheet_id,
            chart_name_lower: chart_name_lower(chart, pool),
        };
        new_by_id.insert(key, chart);
    }

    let mut old_by_name: BTreeMap<ChartNameKey, &ChartObject> = BTreeMap::new();
    for chart in &old.charts {
        let key = ChartNameKey {
            sheet_lower: sheet_name_lower(chart, pool),
            chart_name_lower: chart_name_lower(chart, pool),
        };
        old_by_name.insert(key, chart);
    }

    let mut new_by_name: BTreeMap<ChartNameKey, &ChartObject> = BTreeMap::new();
    for chart in &new.charts {
        let key = ChartNameKey {
            sheet_lower: sheet_name_lower(chart, pool),
            chart_name_lower: chart_name_lower(chart, pool),
        };
        new_by_name.insert(key, chart);
    }

    let mut consumed_old: HashSet<*const ChartObject> = HashSet::new();
    let mut consumed_new: HashSet<*const ChartObject> = HashSet::new();

    let mut ops = Vec::new();

    let mut id_keys: BTreeSet<ChartIdKey> = BTreeSet::new();
    id_keys.extend(old_by_id.keys().cloned());
    id_keys.extend(new_by_id.keys().cloned());
    for key in id_keys {
        let old_chart = old_by_id.get(&key).copied();
        let new_chart = new_by_id.get(&key).copied();
        if let Some(old_chart) = old_chart {
            consumed_old.insert(chart_ptr(old_chart));
        }
        if let Some(new_chart) = new_chart {
            consumed_new.insert(chart_ptr(new_chart));
        }

        match (old_chart, new_chart) {
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

    let mut name_keys: BTreeSet<ChartNameKey> = BTreeSet::new();
    name_keys.extend(old_by_name.keys().cloned());
    name_keys.extend(new_by_name.keys().cloned());
    for key in name_keys {
        let old_chart = old_by_name
            .get(&key)
            .copied()
            .filter(|chart| !consumed_old.contains(&chart_ptr(chart)));
        let new_chart = new_by_name
            .get(&key)
            .copied()
            .filter(|chart| !consumed_new.contains(&chart_ptr(chart)));

        match (old_chart, new_chart) {
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

