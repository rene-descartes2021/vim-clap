use std::cmp::Ordering;

use crate::PathNode;

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

/*
#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering::Greater;
    use std::cmp::Ordering::Less;

    mod compare_dirs_bot_simple_tests {
        use super::*;

        #[test]
        fn dir_to_dir() {
            let dir_a = get_dir("dir_a");
            let dir_b = get_dir("dir_b");

            let order = PathNode::compare_dirs_bot_simple(&dir_a, &dir_b);

            assert_eq!(Less, order);
        }

        #[test]
        fn dir_to_file() {
            let dir = get_dir("something");
            let file = get_file("something");

            let order = PathNode::compare_dirs_bot_simple(&dir, &file);

            assert_eq!(Greater, order);
        }

        #[test]
        fn file_to_file() {
            let file_a = get_file("file_a");
            let file_b = get_file("file_b");

            let order = PathNode::compare_dirs_bot_simple(&file_a, &file_b);

            assert_eq!(Less, order);
        }
    }

    mod compare_dirs_top_simple_tests {
        use super::*;

        #[test]
        fn dir_to_dir() {
            let dir_a = get_dir("dir_a");
            let dir_b = get_dir("dir_b");

            let order = PathNode::compare_dirs_top_simple(&dir_a, &dir_b);

            assert_eq!(Less, order);
        }

        #[test]
        fn dir_to_file() {
            let dir = get_dir("something");
            let file = get_file("something");

            let order = PathNode::compare_dirs_top_simple(&dir, &file);

            assert_eq!(Less, order);
        }

        #[test]
        fn file_to_file() {
            let file_a = get_file("file_a");
            let file_b = get_file("file_b");

            let order = PathNode::compare_dirs_top_simple(&file_a, &file_b);

            assert_eq!(Less, order);
        }
    }

    fn get_dir(name: &str) -> PathNode {
        let mut path_node = PathNode::from(".");
        path_node.is_dir = true;
        path_node.display_text = String::from(name);
        path_node
    }

    fn get_file(name: &str) -> PathNode {
        let mut path_node = PathNode::from(".");
        path_node.is_dir = false;
        path_node.display_text = String::from(name);
        path_node
    }
}
*/
