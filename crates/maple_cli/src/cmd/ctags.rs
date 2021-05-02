use std::hash::Hash;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

use filter::{
    matcher::{Bonus, MatchType},
    subprocess, FilterContext, Source,
};

use crate::app::Params;
use crate::cmd::cache::{cache_exists, send_response_from_cache, CacheEntry, SendResponse};
use crate::tools::ctags::{ensure_has_json_support, TagInfo};

/// Generate ctags recursively given the directory.
#[derive(StructOpt, Debug, Clone)]
pub struct Ctags {
    /// The directory to generate recursive ctags.
    #[structopt(short, long, parse(from_os_str))]
    dir: PathBuf,

    /// Exclude files and directories matching 'pattern'.
    ///
    /// Will be translated into ctags' option: --exclude=pattern.
    #[structopt(long, default_value = ".git,*.json,node_modules,target,_build")]
    exclude: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScopeName(String);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    #[serde(rename = "scope")]
    pub name: ScopeName,
    pub scope_kind: String,
}

/// Type parsed from the line produced by ctags.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BaseTag {
    name: String,
    #[serde(rename = "line")]
    line_number: usize,
    kind: String,
    access: Option<String>,
    signature: Option<String>,
    #[serde(flatten)]
    scope: Option<Scope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeTag {
    pub inner: BaseTag,
    pub depth: usize,
    pub parent: Option<BaseTag>,
    pub children: Vec<TreeTag>,
}

impl From<BaseTag> for TreeTag {
    fn from(inner: BaseTag) -> Self {
        Self {
            inner,
            depth: 1,
            parent: None,
            children: Vec::new(),
        }
    }
}

impl TreeTag {
    pub fn pretty_print(&self) {
        println!(
            "{}{}:{} - {:?}",
            "\t".repeat(self.depth),
            self.inner.name,
            self.inner.line_number,
            self.inner.scope
        );
    }
}

fn print_recursive(root_tree_tag: &TreeTag) {
    root_tree_tag.pretty_print();

    if root_tree_tag.children.is_empty() {
        return;
    }

    for child in root_tree_tag.children.iter() {
        print_recursive(&child);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTags {
    pub root_tags: Vec<TreeTag>,
}

fn formatted_tags_stream(
    args: &[&str],
    dir: impl AsRef<Path>,
) -> Result<impl Iterator<Item = BaseTag>> {
    let stdout_stream = subprocess::Exec::shell(args.join(" "))
        .cwd(dir)
        .stream_stdout()?;
    Ok(BufReader::new(stdout_stream).lines().filter_map(|line| {
        line.ok().and_then(|tag| {
            if let Ok(tag) = serde_json::from_str::<BaseTag>(&tag) {
                Some(tag)
            } else {
                None
            }
        })
    }))
}

impl Ctags {
    pub fn run(
        &self,
        Params {
            no_cache,
            icon_painter,
            ..
        }: Params,
    ) -> Result<()> {
        ensure_has_json_support()?;

        // In case of passing an invalid icon-painter option.
        let icon_painter = icon_painter.map(|_| icon::IconPainter::ProjTags);

        let cmd = "ctags --format=2 --excmd=pattern --fields=+nksSaf --extras=+F --sort=no --append=no --extras=  --language-force=rust --rust-kinds=cPstvfgieMnm --output-format=json --fields=-PF -f- /home/xlc/.vim/plugged/vim-clap/crates/maple_cli/src/tools/ctags.rs";

        let py_cmd = "ctags --format=2 --excmd=pattern --fields=+nksSaf --extras=+F --sort=no --append=no --extras=  --language-force=python --python-kinds=cvfim --output-format=json --fields=-PF -f- /home/xlc/.vim/plugged/vista.vim/test/data/ctags_tree_view.py";

        let cmd_args = py_cmd
            .split_whitespace()
            .map(Into::into)
            .collect::<Vec<_>>();

        let taglines = formatted_tags_stream(&cmd_args, &self.dir)?.collect::<Vec<_>>();

        let mut root_tree_tags = taglines
            .iter()
            .filter(|tagline| tagline.scope.is_none())
            .map(|t| t.clone().into())
            .collect::<Vec<TreeTag>>();

        for root_tree_tag in root_tree_tags.iter_mut() {
            for tagline in taglines.iter() {
                if let Some(ref scope) = tagline.scope {
                    if scope.name.0 == root_tree_tag.inner.name {
                        let mut child: TreeTag = tagline.clone().into();
                        child.depth = root_tree_tag.depth + 1;
                        child.parent = Some(tagline.clone());
                        root_tree_tag.children.push(child);
                    }
                }
            }
        }

        for root_tag in root_tree_tags.iter() {
            print_recursive(root_tag);
        }
        // println!("root tags: {:?}", root_tree_tags);

        Ok(())
    }
}
