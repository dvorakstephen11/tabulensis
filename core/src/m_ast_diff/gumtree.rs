use std::collections::HashMap;

use crate::diff::AstMoveHint;

use super::{FlatTree, move_hints_for_matches};

pub(crate) struct GumTreeResult {
    pub(crate) covered_old: Vec<bool>,
    pub(crate) covered_new: Vec<bool>,
    pub(crate) move_hints: Vec<AstMoveHint>,
}

#[derive(Clone, Copy)]
struct SubtreeMatch {
    old_idx: usize,
    new_idx: usize,
    size: u32,
}

pub(crate) fn match_unique_subtrees(
    old_tree: &FlatTree,
    new_tree: &FlatTree,
    min_move_size: u32,
) -> GumTreeResult {
    let candidates = unique_subtree_candidates(old_tree, new_tree);
    let mut matches: Vec<SubtreeMatch> = Vec::new();

    let mut covered_old = vec![false; old_tree.nodes.len()];
    let mut covered_new = vec![false; new_tree.nodes.len()];

    for cand in candidates {
        if range_overlaps(&covered_old, cand.old_idx, cand.size)
            || range_overlaps(&covered_new, cand.new_idx, cand.size)
        {
            continue;
        }
        mark_range(&mut covered_old, cand.old_idx, cand.size);
        mark_range(&mut covered_new, cand.new_idx, cand.size);
        matches.push(cand);
    }

    let match_pairs: Vec<(usize, usize)> =
        matches.iter().map(|m| (m.old_idx, m.new_idx)).collect();
    let move_hints = move_hints_for_matches(old_tree, new_tree, &match_pairs, min_move_size);

    GumTreeResult {
        covered_old,
        covered_new,
        move_hints,
    }
}

fn unique_subtree_candidates(old_tree: &FlatTree, new_tree: &FlatTree) -> Vec<SubtreeMatch> {
    let mut old_map: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut new_map: HashMap<u64, Vec<usize>> = HashMap::new();

    for (i, n) in old_tree.nodes.iter().enumerate() {
        old_map.entry(n.subtree_hash).or_default().push(i);
    }
    for (i, n) in new_tree.nodes.iter().enumerate() {
        new_map.entry(n.subtree_hash).or_default().push(i);
    }

    let mut out = Vec::new();
    for (h, ois) in old_map {
        let Some(nis) = new_map.get(&h) else {
            continue;
        };
        if ois.len() == 1 && nis.len() == 1 {
            let oi = ois[0];
            let ni = nis[0];
            let size = old_tree.nodes[oi].subtree_size;
            out.push(SubtreeMatch {
                old_idx: oi,
                new_idx: ni,
                size,
            });
        }
    }

    out.sort_by(|a, b| b.size.cmp(&a.size));
    out
}

fn range_overlaps(covered: &[bool], start: usize, size: u32) -> bool {
    let end = start.saturating_add(size as usize);
    covered[start..end].iter().any(|v| *v)
}

fn mark_range(covered: &mut [bool], start: usize, size: u32) {
    let end = start.saturating_add(size as usize);
    for v in covered.iter_mut().take(end).skip(start) {
        *v = true;
    }
}
