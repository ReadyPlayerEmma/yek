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

/// Add a path to the tree structure.
///
/// This function processes file paths by treating:
/// - All intermediate components as directories
/// - The final component as a file (unless explicitly marked as directory)
///
/// This approach avoids filesystem checks with `Path::is_file()` which can fail
/// for relative paths or non-existent files. When processing a list of file paths
/// from a file processor, the final component should always be treated as a file.
///
/// # Arguments
/// * `root` - The root tree node to add the path to
/// * `path` - The path to add to the tree
/// * `final_is_file` - Whether to treat the final component as a file (default: true)
///
/// # Future Enhancement
/// For explicit directory support, this function could be extended to accept
/// an additional parameter or use a separate function that marks directories explicitly.
fn add_path_to_tree(root: &mut TreeNode, path: &Path) {
    add_path_to_tree_with_type(root, path, true)
}

/// Internal function to add a path to the tree with explicit control over final component type.
///
/// # Arguments
/// * `root` - The root tree node to add the path to
/// * `path` - The path to add to the tree
/// * `final_is_file` - Whether to treat the final component as a file
fn add_path_to_tree_with_type(root: &mut TreeNode, path: &Path, final_is_file: bool) {
    let mut components = path.components().peekable();
    let mut current = root;

    // Process all components, treating intermediate ones as directories
    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        let name = component.as_os_str().to_string_lossy().to_string();

        if is_last {
            // Add the final component with specified type
            // For file processing, this is always a file
            // For future directory support, this could be a directory
            current
                .children
                .entry(name.clone())
                .or_insert_with(|| TreeNode::new_with_name(name, final_is_file));
        } else {
            // All intermediate components are directories
            current = current
                .children
                .entry(name.clone())
                .or_insert_with(|| TreeNode::new_with_name(name, false));
        }
    }
}

fn render_child(
    child: &TreeNode,
    output: &mut String,
    current_prefix: &str,
    is_last: bool,
    is_root: bool,
) {
    // Add current prefix (empty for root)
    if !is_root {
        output.push_str(current_prefix);
    }

    // Add tree symbols
    let child_prefix = if is_last { "└── " } else { "├── " };
    output.push_str(child_prefix);
    output.push_str(&child.name);

    // Add '/' for directories
    if !child.is_file {
        output.push('/');
    }
    output.push('\n');

    // Calculate next prefix for children
    let next_prefix = if is_root {
        // For root children, use simple prefix
        if is_last { "    " } else { "│   " }.to_string()
    } else {
        // For non-root children, extend current prefix
        let mut next = String::with_capacity(current_prefix.len() + 4);
        next.push_str(current_prefix);
        next.push_str(if is_last { "    " } else { "│   " });
        next
    };

    // Recursively render this child's children
    render_tree(child, output, &next_prefix, false);
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

    // Render each child using the helper function
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        render_child(child, output, prefix, is_last, is_root);
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

    #[test]
    fn test_final_component_always_treated_as_file() {
        // Test that final components are always treated as files, regardless of extension
        let paths = vec![
            PathBuf::from("Makefile"),      // No extension
            PathBuf::from("Dockerfile"),    // No extension
            PathBuf::from("src/mod"),       // No extension in subdirectory
            PathBuf::from("config.toml"),   // With extension
            PathBuf::from("scripts/build"), // No extension, could look like directory
        ];
        let result = generate_tree(&paths);

        // All final components should be files (no trailing slash)
        // Directories come first, then files alphabetically
        assert!(result.contains("├── scripts/"));
        assert!(result.contains("│   └── build")); // build should be a file, not build/
        assert!(result.contains("├── src/"));
        assert!(result.contains("│   └── mod")); // mod should be a file, not mod/
        assert!(result.contains("├── Dockerfile"));
        assert!(result.contains("├── Makefile"));
        assert!(result.contains("└── config.toml")); // Last file uses └──

        // Verify no final components have trailing slashes (which would indicate directories)
        assert!(!result.contains("Dockerfile/"));
        assert!(!result.contains("Makefile/"));
        assert!(!result.contains("config.toml/"));
        assert!(!result.contains("build/"));
        assert!(!result.contains("mod/"));
    }
}
