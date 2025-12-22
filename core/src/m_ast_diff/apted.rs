use std::collections::HashMap;

use super::SimpleTree;

#[derive(Clone, Copy)]
pub(crate) struct EditCounts {
    pub(crate) cost: u32,
    pub(crate) inserted: u32,
    pub(crate) deleted: u32,
    pub(crate) updated: u32,
}

impl EditCounts {
    fn zero() -> Self {
        EditCounts {
            cost: 0,
            inserted: 0,
            deleted: 0,
            updated: 0,
        }
    }

    fn add(self, other: EditCounts) -> EditCounts {
        EditCounts {
            cost: self.cost + other.cost,
            inserted: self.inserted + other.inserted,
            deleted: self.deleted + other.deleted,
            updated: self.updated + other.updated,
        }
    }
}

fn better(a: EditCounts, b: EditCounts) -> EditCounts {
    if a.cost != b.cost {
        return if a.cost < b.cost { a } else { b };
    }
    if a.updated != b.updated {
        return if a.updated < b.updated { a } else { b };
    }
    let a_id = a.inserted + a.deleted;
    let b_id = b.inserted + b.deleted;
    if a_id != b_id {
        return if a_id < b_id { a } else { b };
    }
    a
}

pub(crate) fn tree_edit_counts(old: &SimpleTree, new: &SimpleTree) -> EditCounts {
    if old.labels.is_empty() {
        return EditCounts {
            cost: new.labels.len() as u32,
            inserted: new.labels.len() as u32,
            deleted: 0,
            updated: 0,
        };
    }
    if new.labels.is_empty() {
        return EditCounts {
            cost: old.labels.len() as u32,
            inserted: 0,
            deleted: old.labels.len() as u32,
            updated: 0,
        };
    }

    let mut memo: HashMap<(usize, usize), EditCounts> = HashMap::new();
    tree_edit_counts_at(old, new, 0, 0, &mut memo)
}

fn tree_edit_counts_at(
    old: &SimpleTree,
    new: &SimpleTree,
    oi: usize,
    ni: usize,
    memo: &mut HashMap<(usize, usize), EditCounts>,
) -> EditCounts {
    if let Some(v) = memo.get(&(oi, ni)) {
        return *v;
    }

    let mut base = if old.labels[oi] == new.labels[ni] {
        EditCounts::zero()
    } else {
        EditCounts {
            cost: 1,
            inserted: 0,
            deleted: 0,
            updated: 1,
        }
    };

    let children_cost = align_children(old, new, &old.children[oi], &new.children[ni], memo);
    base = base.add(children_cost);
    memo.insert((oi, ni), base);
    base
}

fn align_children(
    old: &SimpleTree,
    new: &SimpleTree,
    old_children: &[usize],
    new_children: &[usize],
    memo: &mut HashMap<(usize, usize), EditCounts>,
) -> EditCounts {
    let m = old_children.len();
    let n = new_children.len();
    let mut dp = vec![EditCounts::zero(); (m + 1) * (n + 1)];

    let idx = |i: usize, j: usize, n: usize| -> usize { i * (n + 1) + j };

    for i in 1..=m {
        let del = delete_cost(old, old_children[i - 1]);
        let prev = dp[idx(i - 1, 0, n)];
        dp[idx(i, 0, n)] = prev.add(del);
    }
    for j in 1..=n {
        let ins = insert_cost(new, new_children[j - 1]);
        let prev = dp[idx(0, j - 1, n)];
        dp[idx(0, j, n)] = prev.add(ins);
    }

    for i in 1..=m {
        for j in 1..=n {
            let del = dp[idx(i - 1, j, n)].add(delete_cost(old, old_children[i - 1]));
            let ins = dp[idx(i, j - 1, n)].add(insert_cost(new, new_children[j - 1]));
            let sub = dp[idx(i - 1, j - 1, n)].add(tree_edit_counts_at(
                old,
                new,
                old_children[i - 1],
                new_children[j - 1],
                memo,
            ));
            dp[idx(i, j, n)] = better(better(del, ins), sub);
        }
    }

    dp[idx(m, n, n)]
}

fn delete_cost(tree: &SimpleTree, idx: usize) -> EditCounts {
    let size = tree.subtree_size[idx];
    EditCounts {
        cost: size,
        inserted: 0,
        deleted: size,
        updated: 0,
    }
}

fn insert_cost(tree: &SimpleTree, idx: usize) -> EditCounts {
    let size = tree.subtree_size[idx];
    EditCounts {
        cost: size,
        inserted: size,
        deleted: 0,
        updated: 0,
    }
}

pub(crate) fn approximate_counts(old: &SimpleTree, new: &SimpleTree) -> EditCounts {
    let old_count = old.labels.len() as u32;
    let new_count = new.labels.len() as u32;

    let mut old_hist: HashMap<u64, u32> = HashMap::new();
    for label in &old.labels {
        *old_hist.entry(*label).or_default() += 1;
    }

    let mut common: u32 = 0;
    for label in &new.labels {
        let Some(v) = old_hist.get_mut(label) else {
            continue;
        };
        if *v > 0 {
            *v -= 1;
            common += 1;
        }
    }

    let min_nodes = old_count.min(new_count);
    let updated = min_nodes.saturating_sub(common);
    let deleted = old_count.saturating_sub(min_nodes);
    let inserted = new_count.saturating_sub(min_nodes);

    EditCounts {
        cost: inserted + deleted + updated,
        inserted,
        deleted,
        updated,
    }
}
