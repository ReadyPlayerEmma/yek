use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

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

/// Filter out Windows drive prefixes and root directory components to get logical path components.
/// This ensures that paths like "C:\repo\src\lib.rs" become ["repo", "src", "lib.rs"]
/// instead of ["C:", "\", "repo", "src", "lib.rs"].
fn clean_path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Prefix(_) | Component::RootDir => None,
            Component::CurDir => None, // Skip "." components
            Component::ParentDir => Some("..".to_string()), // Keep ".." components
            Component::Normal(os_str) => Some(os_str.to_string_lossy().to_string()),
        })
        .collect()
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
    let components = clean_path_components(path);
    if components.is_empty() {
        return;
    }

    let mut current = root;

    // Process all components, treating intermediate ones as directories
    for (i, name) in components.iter().enumerate() {
        let is_last = i == components.len() - 1;

        if is_last {
            // Handle the final component
            match current.children.get_mut(name) {
                Some(existing_entry) => {
                    // Entry already exists - handle conflicts
                    if existing_entry.is_file && !final_is_file {
                        // Existing file, trying to make it a directory
                        // Directory wins if it will contain children
                        existing_entry.is_file = false;
                    } else if !existing_entry.is_file && final_is_file {
                        // Existing directory, trying to make it a file
                        // Keep as directory if it has children, otherwise make it a file
                        if existing_entry.children.is_empty() {
                            existing_entry.is_file = true;
                        }
                        // If it has children, directory wins and we ignore the file
                    }
                    // If both are files or both are directories, no change needed
                }
                None => {
                    // Create new entry
                    current.children.insert(
                        name.clone(),
                        TreeNode::new_with_name(name.clone(), final_is_file),
                    );
                }
            }
        } else {
            // Intermediate component - must be a directory
            let entry = current
                .children
                .entry(name.clone())
                .or_insert_with(|| TreeNode::new_with_name(name.clone(), false));

            // If this was previously marked as a file, convert to directory since we need to traverse it
            if entry.is_file {
                entry.is_file = false;
            }
            current = entry;
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

    #[test]
    fn test_windows_path_component_filtering() {
        // Test the clean_path_components function directly
        // On Unix systems, we can't easily create Windows-style paths,
        // so we test the filtering logic with relative paths that have
        // problematic components like ".." and "."

        let path = Path::new("./src/../src/lib.rs");
        let components = clean_path_components(&path);

        // Should filter out "." and keep ".." and normal components
        assert_eq!(components, vec!["src", "..", "src", "lib.rs"]);

        // Test with a simple path
        let path = Path::new("repo/src/lib.rs");
        let components = clean_path_components(&path);
        assert_eq!(components, vec!["repo", "src", "lib.rs"]);
    }

    #[test]
    fn test_path_normalization_in_tree() {
        // Test that paths with current directory components are handled correctly
        let paths = vec![PathBuf::from("./src/lib.rs"), PathBuf::from("src/main.rs")];
        let result = generate_tree(&paths);

        // Should contain proper structure without "./"
        assert!(result.contains("└── src/"));
        assert!(result.contains("    ├── lib.rs"));
        assert!(result.contains("    └── main.rs"));
        // Should not contain "./" in the output
        assert!(!result.contains("./"));
    }

    #[test]
    fn test_duplicate_file_paths() {
        // Test that duplicate file paths are handled correctly
        // The same file path added twice should still result in a single entry
        let paths = vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/lib.rs"), // Duplicate
            PathBuf::from("src/main.rs"),
        ];
        let result = generate_tree(&paths);

        // Should only show lib.rs once
        let lib_rs_count = result.matches("lib.rs").count();
        assert_eq!(
            lib_rs_count, 1,
            "lib.rs should appear only once, got: {}",
            result
        );

        // Should still show both files
        assert!(result.contains("├── lib.rs"));
        assert!(result.contains("└── main.rs"));
    }

    #[test]
    fn test_file_vs_directory_conflict() {
        // Test when the same path is used as both intermediate directory and final file
        // This tests the fix for issue where a file could be marked as directory
        let paths = vec![
            PathBuf::from("config/settings.json"), // config as directory
            PathBuf::from("config"), // config as file - should be absorbed into directory
            PathBuf::from("readme.txt"), // another file for comparison
        ];
        let result = generate_tree(&paths);

        // config should be treated as a directory containing settings.json
        // The standalone "config" file is absorbed into the directory structure
        // because directory usage takes precedence when there are children
        assert!(result.contains("├── config/"));
        assert!(result.contains("│   └── settings.json"));
        assert!(result.contains("└── readme.txt"));

        // Should not show config as both file and directory
        let config_lines: Vec<&str> = result
            .lines()
            .filter(|line| line.contains("config"))
            .collect();
        assert_eq!(
            config_lines.len(),
            1,
            "Config should appear only once as directory"
        );
    }

    #[test]
    fn test_empty_directory_becomes_file() {
        // Test that an empty directory entry can be converted to a file
        let paths = vec![
            PathBuf::from("item"), // item as file
        ];
        let result = generate_tree(&paths);

        // item should be treated as a file (no trailing slash)
        assert!(result.contains("└── item"));
        assert!(!result.contains("item/"));
    }

    #[test]
    fn test_processing_order_independence() {
        // Test that the result is the same regardless of processing order
        let paths1 = vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("src"),
        ];
        let paths2 = vec![
            PathBuf::from("src"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/main.rs"),
        ];

        let result1 = generate_tree(&paths1);
        let result2 = generate_tree(&paths2);

        // Both should produce the same tree structure
        // src should be a directory containing lib.rs and main.rs
        assert!(result1.contains("src/"));
        assert!(result1.contains("lib.rs"));
        assert!(result1.contains("main.rs"));

        assert!(result2.contains("src/"));
        assert!(result2.contains("lib.rs"));
        assert!(result2.contains("main.rs"));

        // The essential structure should be the same (ignoring exact formatting)
        let result1_lines: Vec<&str> = result1.lines().filter(|l| !l.trim().is_empty()).collect();
        let result2_lines: Vec<&str> = result2.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(result1_lines.len(), result2_lines.len());
    }
}
