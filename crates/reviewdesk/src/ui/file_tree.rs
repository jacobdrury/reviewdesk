//! Hierarchical file tree from Patch paths (Zed project-panel style).
//!
//! Builds ancestor dirs for each path, auto-folds single-child directory chains,
//! and flattens to a visible row model that omits children of collapsed dirs.

use std::collections::{BTreeMap, HashSet};

/// One row in the virtualized Changes file list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleTreeRow {
    pub kind: TreeRowKind,
    /// Folder collapse key, or full file path for [`TreeRowKind::File`].
    pub path: String,
    /// Label shown in the row (may include `/` after auto-fold).
    pub name: String,
    pub depth: u8,
    /// Directories only: whether children are in the visible model.
    pub expanded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeRowKind {
    Dir,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TreeNode {
    Dir {
        /// Path key after folding (joined segments).
        path: String,
        name: String,
        children: Vec<TreeNode>,
    },
    File {
        path: String,
        name: String,
    },
}

/// Build a folded tree and flatten to visible rows.
///
/// `collapsed` holds directory path keys whose children are omitted.
/// Directories not in `collapsed` are treated as expanded.
pub fn visible_file_tree_rows(
    paths: &[impl AsRef<str>],
    collapsed: &HashSet<String>,
) -> Vec<VisibleTreeRow> {
    let roots = build_folded_tree(paths);
    let mut out = Vec::new();
    flatten_visible(&roots, 0, collapsed, &mut out);
    out
}

fn build_folded_tree(paths: &[impl AsRef<str>]) -> Vec<TreeNode> {
    let mut root = DirBuilder::default();
    for path in paths {
        let path = path.as_ref();
        if path.is_empty() {
            continue;
        }
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            continue;
        }
        root.insert(&segments, path);
    }
    fold_nodes(root.into_nodes(""))
}

#[derive(Default)]
struct DirBuilder {
    dirs: BTreeMap<String, DirBuilder>,
    files: BTreeMap<String, String>,
}

impl DirBuilder {
    fn insert(&mut self, segments: &[&str], full_path: &str) {
        match segments {
            [] => {}
            [name] => {
                self.files
                    .insert((*name).to_string(), full_path.to_string());
            }
            [name, rest @ ..] => {
                self.dirs
                    .entry((*name).to_string())
                    .or_default()
                    .insert(rest, full_path);
            }
        }
    }

    fn into_nodes(self, prefix: &str) -> Vec<TreeNode> {
        let mut nodes = Vec::new();
        for (name, child) in self.dirs {
            let path = join_path(prefix, &name);
            let children = child.into_nodes(&path);
            nodes.push(TreeNode::Dir {
                path,
                name,
                children,
            });
        }
        for (name, path) in self.files {
            nodes.push(TreeNode::File { path, name });
        }
        nodes
    }
}

fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

fn fold_nodes(nodes: Vec<TreeNode>) -> Vec<TreeNode> {
    nodes.into_iter().map(fold_node).collect()
}

fn fold_node(node: TreeNode) -> TreeNode {
    match node {
        TreeNode::File { path, name } => TreeNode::File { path, name },
        TreeNode::Dir {
            mut path,
            mut name,
            mut children,
        } => {
            children = fold_nodes(children);
            while children.len() == 1 {
                let only = children.remove(0);
                match only {
                    TreeNode::Dir {
                        path: child_path,
                        name: child_name,
                        children: grandchildren,
                    } => {
                        name = format!("{name}/{child_name}");
                        path = child_path;
                        children = grandchildren;
                    }
                    file @ TreeNode::File { .. } => {
                        children.push(file);
                        break;
                    }
                }
            }
            TreeNode::Dir {
                path,
                name,
                children,
            }
        }
    }
}

fn flatten_visible(
    nodes: &[TreeNode],
    depth: u8,
    collapsed: &HashSet<String>,
    out: &mut Vec<VisibleTreeRow>,
) {
    for node in nodes {
        match node {
            TreeNode::Dir {
                path,
                name,
                children,
            } => {
                let expanded = !collapsed.contains(path);
                out.push(VisibleTreeRow {
                    kind: TreeRowKind::Dir,
                    path: path.clone(),
                    name: name.clone(),
                    depth,
                    expanded,
                });
                if !expanded {
                    continue;
                }
                let next_depth = depth.saturating_add(1);
                flatten_visible(children, next_depth, collapsed, out);
            }
            TreeNode::File { path, name } => {
                out.push(VisibleTreeRow {
                    kind: TreeRowKind::File,
                    path: path.clone(),
                    name: name.clone(),
                    depth,
                    expanded: false,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    fn names(rows: &[VisibleTreeRow]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    #[test]
    fn flat_files_at_root() {
        let rows = visible_file_tree_rows(&paths(&["a.rs", "b.rs"]), &HashSet::new());
        assert_eq!(names(&rows), ["a.rs", "b.rs"]);
        assert!(
            rows.iter()
                .all(|r| r.kind == TreeRowKind::File && r.depth == 0)
        );
    }

    #[test]
    fn nested_dirs_and_files() {
        let rows = visible_file_tree_rows(
            &paths(&["src/main.rs", "src/lib.rs", "README.md"]),
            &HashSet::new(),
        );
        // Directories before files at each level (BTreeMap name order within each).
        assert_eq!(names(&rows), ["src", "lib.rs", "main.rs", "README.md"]);
        assert_eq!(rows[0].kind, TreeRowKind::Dir);
        assert!(rows[0].expanded);
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[1].path, "src/lib.rs");
    }

    #[test]
    fn auto_folds_single_child_dirs() {
        let rows = visible_file_tree_rows(
            &paths(&["crates/reviewdesk/src/ui/files.rs"]),
            &HashSet::new(),
        );
        assert_eq!(names(&rows), ["crates/reviewdesk/src/ui", "files.rs"]);
        assert_eq!(rows[0].kind, TreeRowKind::Dir);
        assert_eq!(rows[0].path, "crates/reviewdesk/src/ui");
        assert_eq!(rows[1].depth, 1);
    }

    #[test]
    fn collapsed_dir_omits_children() {
        let mut collapsed = HashSet::new();
        collapsed.insert("src".to_string());
        let rows = visible_file_tree_rows(&paths(&["src/a.rs", "src/b.rs", "root.rs"]), &collapsed);
        assert_eq!(names(&rows), ["src", "root.rs"]);
        assert!(!rows[0].expanded);
    }

    #[test]
    fn collapsed_folded_path_key() {
        let mut collapsed = HashSet::new();
        collapsed.insert("a/b".to_string());
        let rows = visible_file_tree_rows(&paths(&["a/b/c.rs", "a/b/d.rs"]), &collapsed);
        assert_eq!(names(&rows), ["a/b"]);
        assert!(!rows[0].expanded);
    }

    #[test]
    fn sibling_dirs_not_folded() {
        let rows = visible_file_tree_rows(&paths(&["src/a.rs", "lib/b.rs"]), &HashSet::new());
        assert_eq!(names(&rows), ["lib", "b.rs", "src", "a.rs"]);
        assert_eq!(rows[0].name, "lib");
        assert_eq!(rows[2].name, "src");
    }
}
