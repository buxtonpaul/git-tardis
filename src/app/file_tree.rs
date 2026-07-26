use std::collections::HashSet;

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

pub fn build_file_tree(files: &[String]) -> Vec<FileTreeNode> {
    let mut root_children: Vec<FileTreeNode> = Vec::new();

    for path in files {
        let clean_path = path.trim_start_matches("./");
        if clean_path.is_empty() {
            continue;
        }

        let parts: Vec<&str> = clean_path.split('/').collect();
        add_path_components(&mut root_children, &parts, 0, "");
    }

    sort_nodes(&mut root_children);
    root_children
}

fn add_path_components(
    nodes: &mut Vec<FileTreeNode>,
    parts: &[&str],
    depth: usize,
    parent_path: &str,
) {
    if parts.is_empty() {
        return;
    }

    let current_name = parts[0];
    let is_leaf = parts.len() == 1;

    let current_path = if parent_path.is_empty() {
        current_name.to_string()
    } else {
        format!("{}/{}", parent_path, current_name)
    };

    if is_leaf {
        if !nodes.iter().any(|n| n.name == current_name && !n.is_dir) {
            nodes.push(FileTreeNode {
                name: current_name.to_string(),
                path: current_path,
                is_dir: false,
                depth,
                children: Vec::new(),
            });
        }
    } else {
        let pos = nodes.iter().position(|n| n.name == current_name && n.is_dir);
        let dir_idx = match pos {
            Some(idx) => idx,
            None => {
                nodes.push(FileTreeNode {
                    name: current_name.to_string(),
                    path: current_path.clone(),
                    is_dir: true,
                    depth,
                    children: Vec::new(),
                });
                nodes.len() - 1
            }
        };

        add_path_components(
            &mut nodes[dir_idx].children,
            &parts[1..],
            depth + 1,
            &current_path,
        );
    }
}

fn sort_nodes(nodes: &mut Vec<FileTreeNode>) {
    nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    for node in nodes {
        if node.is_dir {
            sort_nodes(&mut node.children);
        }
    }
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
