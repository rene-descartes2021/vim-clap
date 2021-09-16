use super::*;

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct DisplayLine {
    pub display: String,
    pub indices: Option<Vec<usize>>,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub path: PathBuf,
    pub line_number: u64,
}

#[derive(Debug, Clone)]
pub struct DisplayLines {
    pub lines: Vec<DisplayLine>,
    pub locations: HashMap<usize, Location>,
}

#[derive(Debug, Clone)]
pub enum Results {
    Lines(Lines),
    DisplayLines(DisplayLines),
}

impl Results {
    pub fn print(&self) {
        match self {
            Self::Lines(lines) => lines.print(),
            Self::DisplayLines(display_lines) => todo!("display lines "),
        }
    }
}

// TODO: a new renderer for dumb jump
pub fn render(matches: Vec<Match>, kind: &MatchKind, word: &Word) -> Results {
    let mut group_refs = HashMap::new();

    // references are these occurrences not in the definitions.
    for line in matches.iter() {
        let group = group_refs.entry(line.path()).or_insert_with(Vec::new);
        group.push(line);
    }

    let mut title_inserted = false;

    let keys_len = group_refs.keys().len();

    let mut lnum = 1;
    let mut locations = HashMap::new();

    let lines = group_refs
        .values()
        .flat_map(|lines| {
            let mut inner_group: Vec<DisplayLine> = Vec::with_capacity(lines.len() + 3);

            if !title_inserted {
                inner_group.push(DisplayLine {
                    display: format!("{} {} in {} files", matches.len(), kind, keys_len),
                    indices: None,
                });
                title_inserted = true;
                lnum += 1;
            }

            inner_group.push(DisplayLine {
                display: format!("{} [{}]", lines[0].path(), lines.len()),
                indices: None,
            });

            inner_group.extend(lines.iter().map(|line| {
                locations.insert(
                    lnum,
                    Location {
                        path: line.path.text().to_string().into(),
                        line_number: line.line_number.unwrap_or_default(),
                    },
                );
                lnum += 1;

                let (display, indices) = line.build_jump_line_classify(word);
                DisplayLine {
                    display,
                    indices: Some(indices),
                }
            }));

            inner_group.push(DisplayLine::default());

            inner_group
        })
        .collect();

    Results::DisplayLines(DisplayLines { lines, locations })
}

pub fn render_jump_line(
    matches: Vec<Match>,
    kind: &str,
    word: &Word,
    exact_or_inverse_terms: &ExactOrInverseTerms,
) -> Results {
    let (lines, indices): (Vec<String>, Vec<Vec<usize>>) = matches
        .into_par_iter()
        .filter_map(|line| {
            exact_or_inverse_terms.check_jump_line(line.build_jump_line(kind, &word))
        })
        .unzip();

    Results::Lines(Lines::new(lines, indices))
}
