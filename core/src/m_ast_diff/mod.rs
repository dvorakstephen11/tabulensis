use std::hash::{Hash, Hasher};

use crate::diff::{AstDiffMode, AstDiffSummary, AstMoveHint};
use crate::m_ast::{MExpr, MModuleAst};

mod apted;
mod gumtree;

const SMALL_AST_NODE_LIMIT: usize = 240;
const REDUCED_TED_MAX_NODES: usize = 320;
const MOVE_SUBTREE_MIN_SIZE: u32 = 6;

pub(crate) fn diff_summary(old_ast: &MModuleAst, new_ast: &MModuleAst) -> AstDiffSummary {
    let mut old_tree = build_flat_tree(old_ast);
    let mut new_tree = build_flat_tree(new_ast);

    if !old_tree.nodes.is_empty() {
        compute_subtree_hashes(0, &mut old_tree.nodes);
    }
    if !new_tree.nodes.is_empty() {
        compute_subtree_hashes(0, &mut new_tree.nodes);
    }

    let old_count = old_tree.nodes.len();
    let new_count = new_tree.nodes.len();

    if old_count.max(new_count) <= SMALL_AST_NODE_LIMIT {
        let old_simple = simple_tree_from_flat(&old_tree);
        let new_simple = simple_tree_from_flat(&new_tree);
        let counts = apted::tree_edit_counts(&old_simple, &new_simple);

        return AstDiffSummary {
            mode: AstDiffMode::SmallExact,
            node_count_old: old_count as u32,
            node_count_new: new_count as u32,
            inserted: counts.inserted,
            deleted: counts.deleted,
            updated: counts.updated,
            moved: 0,
            move_hints: Vec::new(),
        };
    }

    let gum = gumtree::match_unique_subtrees(&old_tree, &new_tree, MOVE_SUBTREE_MIN_SIZE);
    let reduced_old = build_reduced_tree(&old_tree, &gum.covered_old);
    let reduced_new = build_reduced_tree(&new_tree, &gum.covered_new);

    let counts = if reduced_old.labels.len().max(reduced_new.labels.len()) <= REDUCED_TED_MAX_NODES
    {
        apted::tree_edit_counts(&reduced_old, &reduced_new)
    } else {
        apted::approximate_counts(&reduced_old, &reduced_new)
    };

    AstDiffSummary {
        mode: AstDiffMode::LargeHeuristic,
        node_count_old: old_count as u32,
        node_count_new: new_count as u32,
        inserted: counts.inserted,
        deleted: counts.deleted,
        updated: counts.updated,
        moved: gum.move_hints.len() as u32,
        move_hints: gum.move_hints,
    }
}

#[derive(Clone)]
pub(crate) struct FlatNode {
    pub(crate) label: u64,
    pub(crate) parent: Option<usize>,
    pub(crate) child_index: u32,
    pub(crate) children: Vec<usize>,
    pub(crate) subtree_hash: u64,
    pub(crate) subtree_size: u32,
}

#[derive(Clone)]
pub(crate) struct FlatTree {
    pub(crate) nodes: Vec<FlatNode>,
}

fn build_flat_tree(ast: &MModuleAst) -> FlatTree {
    let mut nodes = Vec::new();
    let root = ast.root_expr();
    build_flat_tree_inner(root, None, 0, &mut nodes);
    FlatTree { nodes }
}

fn build_flat_tree_inner(
    expr: &MExpr,
    parent: Option<usize>,
    child_index: u32,
    nodes: &mut Vec<FlatNode>,
) -> usize {
    let idx = nodes.len();
    nodes.push(FlatNode {
        label: expr.diff_label_hash(),
        parent,
        child_index,
        children: Vec::new(),
        subtree_hash: 0,
        subtree_size: 0,
    });

    let children = expr.diff_children();
    for (i, child) in children.iter().enumerate() {
        let child_idx = build_flat_tree_inner(child, Some(idx), i as u32, nodes);
        nodes[idx].children.push(child_idx);
    }

    idx
}

fn compute_subtree_hashes(root: usize, nodes: &mut [FlatNode]) -> (u64, u32) {
    use crate::hashing::XXH64_SEED;
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    nodes[root].label.hash(&mut h);

    let mut size: u32 = 1;
    let children = nodes[root].children.clone();
    for c in children {
        let (ch, cs) = compute_subtree_hashes(c, nodes);
        ch.hash(&mut h);
        size += cs;
    }
    let hash = h.finish();
    nodes[root].subtree_hash = hash;
    nodes[root].subtree_size = size;
    (hash, size)
}

#[derive(Clone)]
pub(crate) struct SimpleTree {
    pub(crate) labels: Vec<u64>,
    pub(crate) children: Vec<Vec<usize>>,
    pub(crate) subtree_size: Vec<u32>,
}

fn simple_tree_from_flat(tree: &FlatTree) -> SimpleTree {
    let mut labels = Vec::with_capacity(tree.nodes.len());
    let mut children = Vec::with_capacity(tree.nodes.len());
    for node in &tree.nodes {
        labels.push(node.label);
        children.push(node.children.clone());
    }
    let mut out = SimpleTree {
        labels,
        children,
        subtree_size: vec![0; tree.nodes.len()],
    };
    if !out.labels.is_empty() {
        compute_simple_subtree_sizes(&mut out, 0);
    }
    out
}

fn build_reduced_tree(tree: &FlatTree, covered: &[bool]) -> SimpleTree {
    let mut labels = Vec::new();
    let mut children: Vec<Vec<usize>> = Vec::new();

    fn visit(
        idx: usize,
        tree: &FlatTree,
        covered: &[bool],
        parent_covered: bool,
        labels: &mut Vec<u64>,
        children: &mut Vec<Vec<usize>>,
    ) -> Option<usize> {
        let is_covered = covered.get(idx).copied().unwrap_or(false);
        if is_covered && !parent_covered {
            let new_idx = labels.len();
            labels.push(atom_label(tree.nodes[idx].subtree_hash));
            children.push(Vec::new());
            return Some(new_idx);
        }
        if is_covered && parent_covered {
            return None;
        }

        let new_idx = labels.len();
        labels.push(tree.nodes[idx].label);
        children.push(Vec::new());
        for &child in &tree.nodes[idx].children {
            if let Some(child_idx) = visit(child, tree, covered, is_covered, labels, children) {
                children[new_idx].push(child_idx);
            }
        }
        Some(new_idx)
    }

    let _root = visit(0, tree, covered, false, &mut labels, &mut children);

    let labels_len = labels.len();
    let mut out = SimpleTree {
        labels,
        children,
        subtree_size: vec![0; labels_len],
    };
    if !out.labels.is_empty() {
        compute_simple_subtree_sizes(&mut out, 0);
    }
    out
}

fn atom_label(subtree_hash: u64) -> u64 {
    use crate::hashing::XXH64_SEED;
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    0xA7u8.hash(&mut h);
    subtree_hash.hash(&mut h);
    h.finish()
}

fn compute_simple_subtree_sizes(tree: &mut SimpleTree, idx: usize) -> u32 {
    let mut size: u32 = 1;
    let children = tree.children[idx].clone();
    for child in children {
        size += compute_simple_subtree_sizes(tree, child);
    }
    tree.subtree_size[idx] = size;
    size
}

pub(crate) fn move_hints_for_matches(
    old_tree: &FlatTree,
    new_tree: &FlatTree,
    matches: &[(usize, usize)],
    min_move_size: u32,
) -> Vec<AstMoveHint> {
    let mut move_hints = Vec::new();
    for &(old_idx, new_idx) in matches {
        let old_node = &old_tree.nodes[old_idx];
        let new_node = &new_tree.nodes[new_idx];
        if old_node.subtree_size < min_move_size {
            continue;
        }
        let (Some(op), Some(np)) = (old_node.parent, new_node.parent) else {
            continue;
        };
        let old_parent_hash = old_tree.nodes[op].subtree_hash;
        let new_parent_hash = new_tree.nodes[np].subtree_hash;
        if old_parent_hash != new_parent_hash || old_node.child_index != new_node.child_index {
            move_hints.push(AstMoveHint {
                subtree_hash: old_node.subtree_hash,
                from_preorder: old_idx as u32,
                to_preorder: new_idx as u32,
                subtree_size: old_node.subtree_size,
            });
        }
    }
    move_hints
}
