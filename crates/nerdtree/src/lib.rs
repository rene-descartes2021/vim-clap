use std::path::PathBuf;

mod compare_fns;
mod renderer;
mod tree_index;

use compare_fns::PathNodeOrdering;
use tree_index::TreeIndex;

#[derive(Debug, Clone)]
pub struct PathNode {
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_err: bool,
    pub is_expanded: bool,
    pub children: Vec<PathNode>,
    pub display_text: String,
}

impl From<&str> for PathNode {
    fn from(path: &str) -> Self {
        Self {
            children: Vec::new(),
            display_text: String::from(path),
            is_dir: false,
            is_err: false,
            is_expanded: false,
            path: PathBuf::from(path),
        }
    }
}

impl From<String> for PathNode {
    fn from(path: String) -> Self {
        Self::from(path.as_str())
    }
}

impl PathNode {
    pub fn new_expanded(working_dir: &str) -> Self {
        let mut path_node = Self::from(working_dir);
        path_node.is_dir = true;
        path_node.expand(&TreeIndex::new(), &PathNodeOrdering::Top);
        path_node
    }

    /// Expands the directory.
    pub fn expand(&mut self, tree_index: &TreeIndex, node_ordering: &PathNodeOrdering) {
        let mut path_node = self;

        for i in tree_index.iter() {
            if path_node.children.len() > *i {
                path_node = &mut path_node.children[*i];
            }
        }

        if !path_node.path.is_dir() {
            return;
        }

        path_node.is_expanded = true;
        path_node.children = path_node.list_children(node_ordering);
    }

    /// Collapses the directory
    pub fn collapse(&mut self, tree_index: &TreeIndex) {
        let mut path_node = self;

        for i in tree_index.iter() {
            path_node = &mut path_node.children[*i];
        }

        path_node.is_expanded = false;
        path_node.children = Vec::new();
    }

    pub fn expand_at(&mut self, lnum: usize) -> Vec<String> {
        let tree_index = self.flat_index_to_tree_index(lnum);
        self.expand(&tree_index, &PathNodeOrdering::Top);
        let renderer = crate::renderer::Renderer::new(true);
        renderer.render(&self)
    }

    /// Returns all the child path nodes.
    fn list_children(&mut self, node_ordering: &PathNodeOrdering) -> Vec<PathNode> {
        match self.path.read_dir() {
            Ok(dirs) => {
                let mut path_nodes = dirs
                    .filter_map(|dir_entry| dir_entry.ok())
                    .map(|entry| PathNode {
                        children: Vec::new(),
                        display_text: entry.file_name().into_string().unwrap(),
                        is_dir: entry.path().is_dir(),
                        is_err: false,
                        is_expanded: false,
                        path: entry.path(),
                    })
                    .collect::<Vec<_>>();

                path_nodes.sort_unstable_by(|a, b| node_ordering.compare(a, b));

                path_nodes
            }
            Err(_) => {
                self.is_err = true;
                Vec::new()
            }
        }
    }

    fn flat_index_to_tree_index_recursive(
        &self,
        flat_index: &mut usize,
        tree_index: &mut TreeIndex,
    ) -> bool {
        if *flat_index == 0 {
            return true;
        }

        for (c, child) in self.children.iter().enumerate() {
            *flat_index -= 1;

            tree_index.index.push(c);
            if child.flat_index_to_tree_index_recursive(flat_index, tree_index) {
                return true;
            }
            tree_index.index.pop();
        }

        false
    }

    pub fn flat_index_to_tree_index(&self, flat_index: usize) -> TreeIndex {
        let mut tree_index = TreeIndex::new();

        self.flat_index_to_tree_index_recursive(&mut (flat_index + 1), &mut tree_index);

        tree_index
    }

    pub fn tree_index_to_flat_index_recursive(
        &self,
        target_tree_index: &TreeIndex,
        current_tree_index: &TreeIndex,
    ) -> usize {
        if current_tree_index >= target_tree_index {
            return 0;
        }

        if self.children.is_empty() {
            return 1;
        }

        let mut sum = 1;

        for (index, child) in self.children.iter().enumerate() {
            let mut new_current_tree_index = current_tree_index.clone();
            new_current_tree_index.index.push(index);

            sum += child
                .tree_index_to_flat_index_recursive(target_tree_index, &new_current_tree_index);
        }

        sum
    }

    pub fn tree_index_to_flat_index(&self, tree_index: &TreeIndex) -> usize {
        // We count the root directory, hence we have to subtract 1 to get the
        // proper index.
        self.tree_index_to_flat_index_recursive(tree_index, &TreeIndex::new()) - 1
    }

    pub fn get_child_path_node(&self, tree_index: &TreeIndex) -> &Self {
        let mut child_node = self;

        for i in &tree_index.index {
            child_node = &child_node.children[*i];
        }

        child_node
    }
}

#[test]
fn test_expand() {
    let mut root = PathNode::new_expanded("/home/xlc/.vim/plugged/vim-clap");
    let tree_index = root.flat_index_to_tree_index(0);
    root.expand(&tree_index, &PathNodeOrdering::Top);
    let renderer = crate::renderer::Renderer::new(true);
    let lines = renderer.render(&root);
    for line in lines {
        println!("{}", line);
    }
    let tree_index = root.flat_index_to_tree_index(7);
    root.expand(&tree_index, &PathNodeOrdering::Top);
    let renderer = crate::renderer::Renderer::new(true);
    let lines = renderer.render(&root);
    for line in lines {
        println!("{}", line);
    }
}
