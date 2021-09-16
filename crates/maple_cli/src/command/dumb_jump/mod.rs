//! Inspired by https://github.com/jacktasia/dumb-jump/blob/master/dumb-jump.el.
//!
//! This module requires the executable rg with `--json` and `--pcre2` is installed in the system.

use std::path::PathBuf;

use anyhow::Result;
use rayon::prelude::*;
use structopt::StructOpt;

use crate::dumb_analyzer::{
    definitions_and_references, definitions_and_references_lines, find_occurrence_matches_by_ext,
    get_comments_by_ext, get_language_by_ext, MatchKind,
};
use crate::tools::ripgrep::{Match, Word};
use crate::utils::ExactOrInverseTerms;

mod renderer;

/// All the lines as well as their match indices that can be sent to the vim side directly.
#[derive(Clone, Debug)]
pub struct Lines {
    pub lines: Vec<String>,
    pub indices: Vec<Vec<usize>>,
}

impl Lines {
    /// Constructs a new instance of [`Lines`].
    pub fn new(lines: Vec<String>, indices: Vec<Vec<usize>>) -> Self {
        Self { lines, indices }
    }

    /// Prints the lines info to stdout.
    pub fn print(&self) {
        let total = self.lines.len();
        let Self { lines, indices } = self;
        utility::println_json_with_length!(total, lines, indices);
    }
}

/// Search-based jump.
#[derive(StructOpt, Debug, Clone)]
pub struct DumbJump {
    /// Search term.
    #[structopt(index = 1, long)]
    pub word: String,

    /// File extension.
    #[structopt(index = 2, long)]
    pub extension: String,

    /// Definition kind.
    #[structopt(long)]
    pub kind: Option<String>,

    /// Classify the results in group.
    #[structopt(long)]
    pub classify: bool,

    /// Specify the working directory.
    #[structopt(long, parse(from_os_str))]
    pub cmd_dir: Option<PathBuf>,
}

impl DumbJump {
    pub async fn run(self) -> Result<()> {
        let lang = get_language_by_ext(&self.extension)?;
        let comments = get_comments_by_ext(&self.extension);

        // TODO: also take word as query?
        let word = Word::new(self.word)?;

        definitions_and_references_lines(lang, &word, &self.cmd_dir, comments, &Default::default())
            .await?
            .print();

        Ok(())
    }

    pub async fn references_or_occurrences(
        &self,
        exact_or_inverse_terms: &ExactOrInverseTerms,
    ) -> Result<Lines> {
        let word = Word::new(self.word.to_string())?;

        let lang = match get_language_by_ext(&self.extension) {
            Ok(lang) => lang,
            Err(_) => {
                return Ok(renderer::render_jump_line(
                    find_occurrence_matches_by_ext(&word, &self.extension, &self.cmd_dir).await?,
                    "refs",
                    &word,
                    &exact_or_inverse_terms,
                ));
            }
        };

        let comments = get_comments_by_ext(&self.extension);

        // render the results in group.
        if self.classify {
            let res = definitions_and_references(lang, &word, &self.cmd_dir, comments).await?;
            todo!()

            // let (lines, indices): (Vec<String>, Vec<Vec<usize>>) = res
            // .into_par_iter()
            // .flat_map(|(match_kind, matches)| renderer::render(matches, &match_kind, &word))
            // .unzip();

            // Ok(Lines::new(lines, indices))
        } else {
            definitions_and_references_lines(
                lang,
                &word,
                &self.cmd_dir,
                comments,
                exact_or_inverse_terms,
            )
            .await
        }
    }
}
