use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub depth: usize,
    pub children: Vec<FileTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleFileItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
}

#[derive(Default)]
struct InternalNode {
    is_dir: bool,
    children: BTreeMap<String, InternalNode>,
}

pub fn build_file_tree(files: &[String]) -> Vec<FileTreeNode> {
    let mut root = InternalNode {
        is_dir: true,
        children: BTreeMap::new(),
    };

    for path in files {
        let clean_path = path.trim_start_matches("./");
        if clean_path.is_empty() {
            continue;
        }

        let parts: Vec<&str> = clean_path.split('/').collect();
        let mut curr = &mut root;
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            curr = curr
                .children
                .entry((*part).to_string())
                .or_insert_with(|| InternalNode {
                    is_dir: !is_last,
                    children: BTreeMap::new(),
                });
            if !is_last {
                curr.is_dir = true;
            }
        }
    }

    convert_internal_nodes(&root.children, 0, "")
}

fn convert_internal_nodes(
    children: &BTreeMap<String, InternalNode>,
    depth: usize,
    parent_path: &str,
) -> Vec<FileTreeNode> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for (name, node) in children {
        let current_path = if parent_path.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", parent_path, name)
        };

        if node.is_dir {
            let child_nodes = convert_internal_nodes(&node.children, depth + 1, &current_path);
            dirs.push(FileTreeNode {
                name: name.clone(),
                path: current_path,
                is_dir: true,
                depth,
                children: child_nodes,
            });
        } else {
            files.push(FileTreeNode {
                name: name.clone(),
                path: current_path,
                is_dir: false,
                depth,
                children: Vec::new(),
            });
        }
    }

    dirs.extend(files);
    dirs
}

pub fn flatten_file_tree(
    nodes: &[FileTreeNode],
    expanded_folders: &HashSet<String>,
) -> Vec<VisibleFileItem> {
    let mut out = Vec::new();
    flatten_nodes_recursive(nodes, expanded_folders, &mut out);
    out
}

fn flatten_nodes_recursive(
    nodes: &[FileTreeNode],
    expanded_folders: &HashSet<String>,
    out: &mut Vec<VisibleFileItem>,
) {
    for node in nodes {
        if node.is_dir {
            let is_expanded = expanded_folders.contains(&node.path);
            out.push(VisibleFileItem {
                name: format!("{}/", node.name),
                path: node.path.clone(),
                is_dir: true,
                is_expanded,
                depth: node.depth,
            });

            if is_expanded {
                flatten_nodes_recursive(&node.children, expanded_folders, out);
            }
        } else {
            out.push(VisibleFileItem {
                name: node.name.clone(),
                path: node.path.clone(),
                is_dir: false,
                is_expanded: false,
                depth: node.depth,
            });
        }
    }
}

pub fn collect_all_dir_paths(nodes: &[FileTreeNode], out: &mut HashSet<String>) {
    for node in nodes {
        if node.is_dir {
            out.insert(node.path.clone());
            collect_all_dir_paths(&node.children, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_and_sort_file_tree() {
        let files = vec![
            "README.md".to_string(),
            "src/app/mod.rs".to_string(),
            "src/app/types.rs".to_string(),
            "src/main.rs".to_string(),
        ];

        let tree = build_file_tree(&files);
        assert_eq!(tree.len(), 2);

        // Directories sorted before files
        assert_eq!(tree[0].name, "src");
        assert!(tree[0].is_dir);

        assert_eq!(tree[1].name, "README.md");
        assert!(!tree[1].is_dir);

        let src_children = &tree[0].children;
        assert_eq!(src_children.len(), 2);
        assert_eq!(src_children[0].name, "app");
        assert!(src_children[0].is_dir);
        assert_eq!(src_children[1].name, "main.rs");
        assert!(!src_children[1].is_dir);
    }

    #[test]
    fn test_flatten_file_tree_folding() {
        let files = vec![
            "README.md".to_string(),
            "src/app/mod.rs".to_string(),
            "src/main.rs".to_string(),
        ];

        let tree = build_file_tree(&files);
        let mut expanded = HashSet::new();

        // Initially nothing expanded
        let visible_collapsed = flatten_file_tree(&tree, &expanded);
        assert_eq!(visible_collapsed.len(), 2);
        assert_eq!(visible_collapsed[0].name, "src/");
        assert_eq!(visible_collapsed[1].name, "README.md");

        // Expand "src"
        expanded.insert("src".to_string());
        let visible_src_expanded = flatten_file_tree(&tree, &expanded);
        assert_eq!(visible_src_expanded.len(), 4);
        assert_eq!(visible_src_expanded[0].name, "src/");
        assert_eq!(visible_src_expanded[1].name, "app/");
        assert_eq!(visible_src_expanded[2].name, "main.rs");
        assert_eq!(visible_src_expanded[3].name, "README.md");

        // Expand "src/app" as well
        expanded.insert("src/app".to_string());
        let visible_all_expanded = flatten_file_tree(&tree, &expanded);
        assert_eq!(visible_all_expanded.len(), 5);
        assert_eq!(visible_all_expanded[0].name, "src/");
        assert_eq!(visible_all_expanded[1].name, "app/");
        assert_eq!(visible_all_expanded[2].name, "mod.rs");
        assert_eq!(visible_all_expanded[2].depth, 2);
        assert_eq!(visible_all_expanded[3].name, "main.rs");
        assert_eq!(visible_all_expanded[4].name, "README.md");
    }
}
