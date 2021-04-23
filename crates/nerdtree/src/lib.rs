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
    pub fn new() -> Self {
        todo!()
    }

    pub fn do_collapse(&mut self) {}

    pub fn do_expand(&mut self) {}

    // Reload the opened directories.
    pub fn do_reload(&mut self) {}
}

/*
    pub fn do_reload(&mut self) -> Option<()> {
        self.reload_openend_dirs();

        self.text_entries =
            self.composer.compose_path_node(&self.path_node_root);

        self.update_pager(0);

        Some(())
    }

    fn reload_openend_dirs(&mut self) {
        // backup the old path node structure
        let old_path_node_root = self.path_node_root.clone();

        // reset the root path node
        self.path_node_root =
            PathNode::from(self.config.setup.working_dir.clone());
        self.path_node_root
            .expand_dir(&TreeIndex::from(Vec::new()), self.path_node_compare);

        // restore the old path nodes structure for the root path node
        self.restore_expansions(&old_path_node_root, &mut TreeIndex::new());
    }

    fn restore_expansions(
        &mut self,
        path_node: &PathNode,
        tree_index: &mut TreeIndex,
    ) {
        for (c, child) in path_node.children.iter().enumerate() {
            if child.is_expanded {
                tree_index.index.push(c);

                self.path_node_root
                    .expand_dir(tree_index, self.path_node_compare);
                self.restore_expansions(child, tree_index);

                tree_index.index.pop();
            }
        }
    }

pub fn do_collapse_dir(&mut self) -> Option<()> {
    let tree_index = self
        .path_node_root
        .flat_index_to_tree_index(self.pager.cursor_row as usize);

    let cursor_delta = self.get_parent_dir_cursor_delta(&tree_index);

    if cursor_delta == 0 {
        self.path_node_root.collapse_dir(&tree_index);
    }

    self.text_entries = self.composer.compose_path_node(&self.path_node_root);

    Some(())
}

fn get_parent_dir_cursor_delta(&mut self, tree_index: &TreeIndex) -> i32 {
    let child_path_node = self.path_node_root.get_child_path_node(tree_index);
    if child_path_node.is_dir && child_path_node.is_expanded {
        return 0;
    }

    let parent_path_node_tree_index = tree_index.get_parent();
    if parent_path_node_tree_index == TreeIndex::new() {
        return 0;
    }

    let parent_flat_index = self
        .path_node_root
        .tree_index_to_flat_index(&parent_path_node_tree_index) as i32;

    parent_flat_index - cursor_row
}
*/
