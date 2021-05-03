use std::cmp::Ordering;
use std::path::PathBuf;

use crate::tree_index::TreeIndex;

#[derive(Clone, Debug)]
pub enum PathNodeOrdering {
    /// Put the directory items on the top.
    Top,
    /// Put the directory items on the bottom.
    Bottom,
    /// Keep the origin order.
    None,
}

impl PathNodeOrdering {
    fn dir_top(a: &PathNode, b: &PathNode) -> Ordering {
        if a.is_dir && !b.is_dir {
            Ordering::Less
        } else if !a.is_dir && b.is_dir {
            Ordering::Greater
        } else {
            a.display_text.cmp(&b.display_text)
        }
    }

    fn dir_bottom(a: &PathNode, b: &PathNode) -> Ordering {
        if a.is_dir && !b.is_dir {
            Ordering::Greater
        } else if !a.is_dir && b.is_dir {
            Ordering::Less
        } else {
            a.display_text.cmp(&b.display_text)
        }
    }

    pub fn compare(&self, a: &PathNode, b: &PathNode) -> Ordering {
        match self {
            Self::Top => Self::dir_top(a, b),
            Self::Bottom => Self::dir_bottom(a, b),
            _ => Ordering::Equal,
        }
    }
}

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
    pub fn new(working_dir: &str) -> Self {
        let mut path_node = Self::from(working_dir);
        path_node.is_dir = true;
        path_node
    }

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

    pub fn toggle_at(&mut self, lnum: usize) -> Vec<String> {
        let tree_index = self.flat_index_to_tree_index(lnum);
        if self.is_expanded {
            let renderer = crate::renderer::Renderer::new(true);
            let mut tree_explorer =
                crate::TreeExplorer::new(self.clone(), renderer, PathNodeOrdering::Top);
            return tree_explorer.do_collapse(lnum);
        } else {
            self.expand(&tree_index, &PathNodeOrdering::Top);
        }
        self.expand_at(lnum)
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

    pub fn flat_index_to_tree_index(&self, flat_index: usize) -> TreeIndex {
        let mut tree_index = TreeIndex::new();

        self.flat_index_to_tree_index_recursive(&mut (flat_index + 1), &mut tree_index);

        tree_index
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

    pub fn is_node_expanded(&mut self, flat_index: usize) -> bool {
        let tree_index = self.flat_index_to_tree_index(flat_index);

        let mut path_node = self;

        for i in tree_index.iter() {
            if path_node.children.len() > *i {
                path_node = &mut path_node.children[*i];
            }
        }

        if !path_node.path.is_dir() {
            return false;
        }

        path_node.is_expanded
    }

    pub fn path_node_at(&mut self, flat_index: usize) -> &PathNode {
        let tree_index = self.flat_index_to_tree_index(flat_index);

        let mut path_node = self;

        for i in tree_index.iter() {
            if path_node.children.len() > *i {
                path_node = &mut path_node.children[*i];
            }
        }

        path_node
    }

    pub fn toggle(&mut self, flat_index: usize) -> Vec<String> {
        let tree_index = self.flat_index_to_tree_index(flat_index);
        log::debug!(
            "--------- is expanded: {}",
            self.is_node_expanded(flat_index)
        );
        if self.is_node_expanded(flat_index) {
            self.collapse(&tree_index);
        } else {
            self.expand(&tree_index, &PathNodeOrdering::Top);
        }

        let renderer = crate::renderer::Renderer::new(true);
        renderer.render(&self)
    }
}

#[test]
fn test_expand() {
    let mut root = PathNode::new("/home/xlc/.vim/plugged/vim-clap");

    let tree_index = root.flat_index_to_tree_index(0);
    root.expand(&tree_index, &PathNodeOrdering::Top);
    let renderer = crate::renderer::Renderer::new(true);
    let lines = renderer.render(&root);
    for line in lines {
        println!("{}", line);
    }

    let tree_index = root.flat_index_to_tree_index(7);
    root.expand(&tree_index, &PathNodeOrdering::Top);
    println!("is expanded: {}", root.is_node_expanded(7));
    // println!("{:#?}", root);
    let renderer = crate::renderer::Renderer::new(true);
    let lines = renderer.render(&root);
    // for line in lines {
    // println!("{}", line);
    // }

    let tree_index = root.flat_index_to_tree_index(2);
    root.expand(&tree_index, &PathNodeOrdering::Top);
    println!("is expanded: {}", root.is_node_expanded(2));
    let renderer = crate::renderer::Renderer::new(true);
    let lines = renderer.render(&root);
    // for line in lines {
    // println!("{}", line);
    // }

    let tree_index = root.flat_index_to_tree_index(4);
    root.expand(&tree_index, &PathNodeOrdering::Top);
    println!("is expanded: {}", root.is_node_expanded(4));
    // for child in &root.children {
    // println!("{:?}", child);
    // }
    let renderer = crate::renderer::Renderer::new(true);
    let lines = renderer.render(&root);
    for line in lines {
        println!("{}", line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_expanded_path_node() -> PathNode {
        let mut path_node = PathNode::from("./tests/test_dirs");
        path_node.expand(&TreeIndex::new(), &PathNodeOrdering::Top);
        path_node.expand(&TreeIndex::from(vec![0]), &PathNodeOrdering::Top);
        path_node.expand(&TreeIndex::from(vec![0, 0]), &PathNodeOrdering::Top);
        path_node.expand(&TreeIndex::from(vec![1]), &PathNodeOrdering::Top);
        path_node.expand(&TreeIndex::from(vec![1, 0]), &PathNodeOrdering::Top);
        path_node.expand(&TreeIndex::from(vec![1, 0, 2]), &PathNodeOrdering::Top);
        path_node
    }

    mod get_child_path_node_tests {
        use super::*;

        #[test]
        fn first_dirs() {
            let path_node = {
                let mut path_node = PathNode::from("./tests/test_dirs");
                path_node.expand(&TreeIndex::new(), &PathNodeOrdering::Top);
                path_node.expand(&TreeIndex::from(vec![0]), &PathNodeOrdering::Top);
                path_node.expand(&TreeIndex::from(vec![0, 0]), &PathNodeOrdering::Top);
                path_node
            };

            let child_path_node = path_node.get_child_path_node(&TreeIndex::from(vec![0, 0, 0]));

            assert_eq!("file4", child_path_node.display_text);
        }

        #[test]
        fn complex_dirs() {
            let path_node = get_expanded_path_node();

            let child_path_node = path_node.get_child_path_node(&TreeIndex::from(vec![1, 0, 2, 2]));

            assert_eq!("file12", child_path_node.display_text);
        }
    }

    mod tree_index_to_flat_index_tests {
        use super::*;

        #[test]
        fn complex_dirs() {
            let path_node = get_expanded_path_node();

            let flat_index = path_node.tree_index_to_flat_index(&TreeIndex::from(vec![4]));

            assert_eq!(22, flat_index);
        }

        #[test]
        fn complex_dirs2() {
            let path_node = get_expanded_path_node();

            let flat_index = path_node.tree_index_to_flat_index(&TreeIndex::from(vec![5]));

            assert_eq!(23, flat_index);
        }

        #[test]
        fn complex_dirs3() {
            let path_node = get_expanded_path_node();

            let flat_index = path_node.tree_index_to_flat_index(&TreeIndex::from(vec![1, 0, 4]));

            assert_eq!(15, flat_index);
        }

        #[test]
        fn total_count() {
            let path_node = get_expanded_path_node();

            let flat_index = path_node.tree_index_to_flat_index(&TreeIndex::from(vec![100_000]));

            assert_eq!(31, flat_index);
        }

        #[test]
        fn zero() {
            let path_node = get_expanded_path_node();

            let flat_index = path_node.tree_index_to_flat_index(&TreeIndex::from(vec![0]));

            assert_eq!(0, flat_index);
        }
    }
}
