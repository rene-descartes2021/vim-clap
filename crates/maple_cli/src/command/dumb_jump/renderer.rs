use super::*;

use std::collections::HashMap;

// TODO: a new renderer for dumb jump
pub fn render(matches: Vec<Match>, kind: &MatchKind, word: &Word) -> Vec<(String, Vec<usize>)> {
    let mut group_refs = HashMap::new();

    // references are these occurrences not in the definitions.
    for line in matches.iter() {
        let group = group_refs.entry(line.path()).or_insert_with(Vec::new);
        group.push(line);
    }

    let mut kind_inserted = false;

    let keys_len = group_refs.keys().len();

    group_refs
        .values()
        .flat_map(|lines| {
            let mut inner_group: Vec<(String, Vec<usize>)> = Vec::with_capacity(lines.len() + 1);

            if !kind_inserted {
                inner_group.push((format!("{} {} in {} files", matches.len(), kind, keys_len), vec![]));
                kind_inserted = true;
            }

            inner_group.push((format!("{} [{}]", lines[0].path(), lines.len()), vec![]));

            inner_group.extend(lines.iter().map(|line| line.build_jump_line_bare(word)));

            inner_group.push(("".into(), vec![]));

            inner_group
        })
        .collect()
}

pub fn render_jump_line(
    matches: Vec<Match>,
    kind: &str,
    word: &Word,
    exact_or_inverse_terms: &ExactOrInverseTerms,
) -> Lines {
    let (lines, indices): (Vec<String>, Vec<Vec<usize>>) = matches
        .into_iter()
        .filter_map(|line| {
            exact_or_inverse_terms.check_jump_line(line.build_jump_line(kind, &word))
        })
        .unzip();

    Lines::new(lines, indices)
}
