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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    #[serde(rename = "scope")]
    pub name: String,
    pub scope_kind: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TagLine {
    name: String,
    line: usize,
    kind: String,
    #[serde(flatten)]
    scope: Option<Scope>,
}

fn formatted_tags_stream(
    args: &[&str],
    dir: impl AsRef<Path>,
) -> Result<impl Iterator<Item = TagLine>> {
    let stdout_stream = subprocess::Exec::shell(args.join(" "))
        .cwd(dir)
        .stream_stdout()?;
    Ok(BufReader::new(stdout_stream).lines().filter_map(|line| {
        line.ok().and_then(|tag| {
            println!("----- line: {:?}", tag);
            println!("{:?}", serde_json::from_str::<TagLine>(&tag));
            if let Ok(tag) = serde_json::from_str::<TagLine>(&tag) {
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
        let cmd_args = cmd.split_whitespace().map(Into::into).collect::<Vec<_>>();

        // let cmd_args = cmd_args.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        for tag in formatted_tags_stream(&cmd_args, &self.dir)? {
            println!("{:?}", tag);
        }

        Ok(())
    }
}
