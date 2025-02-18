use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;

use anyhow::Result;
use clap::Parser;

use utility::read_lines;

use crate::paths::AbsPathBuf;

/// Parse and display Vim helptags.
#[derive(Parser, Debug, Clone)]
pub struct Helptags {
    /// Tempfile containing the info of vim helptags.
    #[clap(index = 1, long)]
    meta_info: AbsPathBuf,
}

#[inline]
fn strip_trailing_slash(x: &str) -> Cow<str> {
    if x.ends_with('/') {
        let mut x: String = x.into();
        x.pop();
        x.into()
    } else {
        x.into()
    }
}

impl Helptags {
    pub fn run(self) -> Result<()> {
        let mut lines = read_lines(self.meta_info.as_ref())?;
        // line 1:/doc/tags,/doc/tags-cn
        // line 2:&runtimepath
        if let Some(Ok(doc_tags)) = lines.next() {
            if let Some(Ok(runtimepath)) = lines.next() {
                for dt in doc_tags.split(',') {
                    let tags_files = runtimepath
                        .split(',')
                        .map(|x| format!("{}{}", strip_trailing_slash(x), dt));
                    let mut seen = HashMap::new();
                    let mut v: Vec<String> = Vec::new();
                    for tags_file in tags_files {
                        if let Ok(lines) = read_lines(tags_file) {
                            lines.for_each(|line| {
                                if let Ok(helptag) = line {
                                    v = helptag.split('\t').map(Into::into).collect();
                                    if !seen.contains_key(&v[0]) {
                                        seen.insert(
                                            v[0].clone(),
                                            format!("{:<60}\t{}", v[0], v[1]),
                                        );
                                    }
                                }
                            });
                        }
                    }
                    let mut tag_lines = seen.values().collect::<Vec<_>>();
                    tag_lines.sort();

                    let stdout = std::io::stdout();
                    let mut lock = stdout.lock();
                    for line in tag_lines {
                        writeln!(lock, "{}", line)?;
                    }
                }
            }
        }
        Ok(())
    }
}
