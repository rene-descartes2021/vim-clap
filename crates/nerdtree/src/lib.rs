// mod action;
mod path_node;
mod renderer;
mod tree_index;

use renderer::Renderer;

pub use self::path_node::{PathNode, PathNodeOrdering};
pub use self::tree_index::TreeIndex;

#[derive(Clone, Debug)]
pub struct TreeExplorer {
    pub root_node: PathNode,
    pub renderer: Renderer,
    pub path_node_ordering: PathNodeOrdering,
}

impl TreeExplorer {
    pub fn new(
        root_node: PathNode,
        renderer: Renderer,
        path_node_ordering: PathNodeOrdering,
    ) -> Self {
        Self {
            root_node,
            renderer,
            path_node_ordering,
        }
    }

    pub fn do_expand(&mut self, cursor_row: usize) -> Vec<String> {
        let tree_index = self.root_node.flat_index_to_tree_index(cursor_row);
        self.root_node.expand(&tree_index, &self.path_node_ordering);
        self.renderer.render(&self.root_node)
    }

    // Reload the opened directories.
    pub fn do_reload(&mut self) {}

    pub fn do_collapse(&mut self, cursor_row: usize) -> Vec<String> {
        let tree_index = self.root_node.flat_index_to_tree_index(cursor_row);

        let cursor_delta = self.get_parent_dir_cursor_delta(&tree_index, cursor_row);

        if cursor_delta == 0 {
            self.root_node.collapse(&tree_index);
        }

        self.renderer.render(&self.root_node)
    }

    fn get_parent_dir_cursor_delta(&mut self, tree_index: &TreeIndex, cursor_row: usize) -> usize {
        let child_path_node = self.root_node.get_child_path_node(tree_index);
        if child_path_node.is_dir && child_path_node.is_expanded {
            return 0;
        }

        let parent_path_node_tree_index = tree_index.get_parent();
        if parent_path_node_tree_index == TreeIndex::new() {
            return 0;
        }

        let parent_flat_index = self
            .root_node
            .tree_index_to_flat_index(&parent_path_node_tree_index);

        parent_flat_index - cursor_row
    }
}

#[test]
fn test_tree_explorer() {
    let root_node = PathNode::new("/home/xlc/.vim/plugged/vim-clap");
    let renderer = self::renderer::Renderer::new(true);
    let mut tree_explorer = TreeExplorer::new(root_node, renderer, PathNodeOrdering::Top);
    let lines = tree_explorer.do_expand(0);
    println!("");
    for line in lines {
        println!("{}", line);
    }

    let lines = tree_explorer.do_expand(7);
    for line in lines {
        println!("{}", line);
    }

    let lines = tree_explorer.do_expand(2);
    for line in lines {
        println!("{}", line);
    }
}
