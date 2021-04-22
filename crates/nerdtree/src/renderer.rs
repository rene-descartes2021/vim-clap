use crate::PathNode;

pub struct Renderer {
    use_utf8: bool,
    show_indent: bool,
    indent: usize,
    indent_char: char,
    icon_char: IconChar,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            use_utf8: true,
            show_indent: true,
            indent: 4,
            indent_char: ' ',
            icon_char: IconChar {
                err: ' ',
                expanded: ' ',
                collapsed: ' ',
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct IconChar {
    pub err: char,
    pub expanded: char,
    pub collapsed: char,
}

impl IconChar {
    pub fn char_for(&self, path_node: &PathNode) -> char {
        if path_node.is_err {
            self.err
        } else if path_node.is_expanded {
            self.expanded
        } else {
            self.collapsed
        }
    }
}

impl Renderer {
    pub fn new(use_utf8: bool) -> Self {
        let icon_char = if use_utf8 {
            IconChar {
                err: '⨯',
                expanded: '▼',
                collapsed: '▶',
            }
        } else {
            IconChar {
                err: 'x',
                expanded: 'v',
                collapsed: '>',
            }
        };
        Self {
            icon_char,
            ..Default::default()
        }
    }

    pub fn truncate_string(string: &str, desired_char_count: usize) -> String {
        if desired_char_count < 1 {
            return String::new();
        }

        if desired_char_count >= string.chars().count() {
            return String::from(string);
        }

        let truncated = match string.char_indices().nth(desired_char_count - 1) {
            None => string,
            Some((idx, _)) => &string[..idx],
        };

        format!("{}~", truncated)
    }

    pub fn render(&self, path_node: &PathNode) -> Vec<String> {
        let mut res = Vec::new();

        self.render_path_node_recursive(path_node, &mut res, 0);

        res
    }

    fn render_path_node_recursive(
        &self,
        path_node: &PathNode,
        texts: &mut Vec<String>,
        depth: usize,
    ) {
        for child in &path_node.children {
            let dir_prefix = self.get_dir_prefix(child);
            let dir_suffix = self.get_dir_suffix(child);
            let indent = self.get_indent(depth);

            let text = format!(
                "{}{}{}{}",
                indent,
                dir_prefix,
                child.display_text.clone(),
                dir_suffix,
            );
            texts.push(text);
            self.render_path_node_recursive(child, texts, depth + 1);
        }
    }

    fn get_dir_prefix(&self, path_node: &PathNode) -> String {
        if path_node.is_dir {
            let expanded_indicator = self.icon_char.char_for(path_node);
            format!("{} ", expanded_indicator)
        } else {
            String::from("  ")
        }
    }

    fn get_dir_suffix(&self, path_node: &PathNode) -> String {
        if path_node.is_dir {
            String::from("/")
        } else {
            String::from("")
        }
    }

    fn get_indent(&self, depth: usize) -> String {
        let indent = " ".repeat(self.indent - 1);
        format!("{}{}", self.indent_char, indent).repeat(depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_string_test() {
        let tc = Renderer::truncate_string;
        assert_eq!(tc("hello world", 5), "hell~");
        assert_eq!(tc("hello world", 1), "~");
        assert_eq!(tc("hello world", 0), "");
        assert_eq!(tc("aaa▶bbb▶ccc", 8), "aaa▶bbb~");
        assert_eq!(tc("aaa▶bbb▶ccc", 6), "aaa▶b~");
        assert_eq!(tc("aaa▶bbb▶ccc", 4), "aaa~");
    }
}
