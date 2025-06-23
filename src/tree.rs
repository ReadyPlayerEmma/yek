use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Generate a directory tree from a list of file paths
pub fn generate_tree(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return String::new();
    }

    // Pre-allocate string with estimated capacity
    let total_path_len: usize = paths.iter().map(|p| p.to_string_lossy().len()).sum();
    let mut output = String::with_capacity(total_path_len + paths.len() * 8);

    // Build a tree structure from the paths
    let mut tree = TreeNode::new();

    // Add all paths to the tree
    for path in paths {
        add_path_to_tree(&mut tree, path);
    }

    // Generate the tree output
    output.push_str("Directory structure:\n");
    render_tree(&tree, &mut output, "", true);
    output.push('\n'); // Add blank line after tree

    output
}

#[derive(Debug)]
struct TreeNode {
    name: String,
    children: HashMap<String, TreeNode>,
    is_file: bool,
}

impl TreeNode {
    fn new() -> Self {
        TreeNode {
            name: String::new(),
            children: HashMap::new(),
            is_file: false,
        }
    }

    fn new_with_name(name: String, is_file: bool) -> Self {
        TreeNode {
            name,
            children: HashMap::new(),
            is_file,
        }
    }
}

fn add_path_to_tree(root: &mut TreeNode, path: &Path) {
    let mut components = path.components().peekable();
    let mut current = root;

    // Process all components except the last one as directories
    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        let name = component.as_os_str().to_string_lossy().to_string();

        if is_last {
            // Add the final component (file or directory)
            let is_file = path.is_file() || path.extension().is_some();
            current
                .children
                .entry(name.clone())
                .or_insert_with(|| TreeNode::new_with_name(name, is_file));
        } else {
            // Process as directory
            current = current
                .children
                .entry(name.clone())
                .or_insert_with(|| TreeNode::new_with_name(name, false));
        }
    }
}

fn render_tree(node: &TreeNode, output: &mut String, prefix: &str, is_root: bool) {
    // Sort children: directories first, then files, both alphabetically
    let mut children: Vec<_> = node.children.values().collect();
    children.sort_by(|a, b| {
        // Directories before files
        match (a.is_file, b.is_file) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    if is_root {
        // For root, just render children
        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let child_prefix = if is_last { "└── " } else { "├── " };
            output.push_str(child_prefix);
            output.push_str(&child.name);
            if !child.is_file {
                output.push('/');
            }
            output.push('\n');

            // Render children with appropriate prefix
            let next_prefix = if is_last { "    " } else { "│   " };
            render_tree(child, output, next_prefix, false);
        }
    } else {
        // For non-root nodes, render children with current prefix
        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            output.push_str(prefix);
            let child_prefix = if is_last { "└── " } else { "├── " };
            output.push_str(child_prefix);
            output.push_str(&child.name);
            if !child.is_file {
                output.push('/');
            }
            output.push('\n');

            // Render children with extended prefix
            let mut next_prefix = String::with_capacity(prefix.len() + 4);
            next_prefix.push_str(prefix);
            next_prefix.push_str(if is_last { "    " } else { "│   " });
            render_tree(child, output, &next_prefix, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_generate_tree_empty() {
        let paths = vec![];
        let result = generate_tree(&paths);
        assert_eq!(result, "");
    }

    #[test]
    fn test_generate_tree_single_file() {
        let paths = vec![PathBuf::from("README.md")];
        let result = generate_tree(&paths);
        assert!(result.contains("Directory structure:"));
        assert!(result.contains("└── README.md"));
    }

    #[test]
    fn test_generate_tree_nested_structure() {
        let paths = vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("Cargo.toml"),
            PathBuf::from("README.md"),
        ];
        let result = generate_tree(&paths);

        assert!(result.contains("Directory structure:"));
        assert!(result.contains("├── src/"));
        assert!(result.contains("│   ├── lib.rs"));
        assert!(result.contains("│   └── main.rs"));
        assert!(result.contains("├── Cargo.toml"));
        assert!(result.contains("└── README.md"));
    }

    #[test]
    fn test_generate_tree_directories_before_files() {
        let paths = vec![PathBuf::from("file.txt"), PathBuf::from("dir/nested.rs")];
        let result = generate_tree(&paths);

        // Directories should come before files
        let dir_pos = result.find("├── dir/").unwrap_or(0);
        let file_pos = result.find("└── file.txt").unwrap_or(0);
        assert!(dir_pos < file_pos);
    }
}
