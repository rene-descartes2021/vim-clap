//! This crate provides the feature of diplaying the information of filtered lines
//! by printing them to stdout in JSON format.

mod legacy;

use std::collections::HashMap;

use icon::{IconPainter, ICON_LEN};
use source_item::SourceItem;
use utility::{println_json, println_json_with_length};

const DOTS: &str = "..";
const DOTS_LEN: usize = DOTS.len();

/// Line number of Vim is 1-based.
pub type VimLineNumber = usize;

/// Map of truncated line number to original full line.
///
/// Can't use HashMap<String, String> since we can't tell the original lines in the following case:
///
/// //  ..{ version = "1.0", features = ["derive"] }
/// //  ..{ version = "1.0", features = ["derive"] }
/// //  ..{ version = "1.0", features = ["derive"] }
/// //  ..{ version = "1.0", features = ["derive"] }
///
pub type LinesTruncatedMap = HashMap<VimLineNumber, String>;

/// Tuple of (matched line text, filtering score, indices of matched elements)
pub type FilterResult = (SourceItem, i64, Vec<usize>);

/// sign column width 2
#[cfg(not(test))]
const WINWIDTH_OFFSET: usize = 4;

#[cfg(test)]
const WINWIDTH_OFFSET: usize = 0;

// https://stackoverflow.com/questions/51982999/slice-a-string-containing-unicode-chars
#[inline]
fn utf8_str_slice(line: &str, start: usize, end: usize) -> String {
    line.chars().take(end).skip(start).collect()
}

fn char_display_width(c: char) -> usize {
    match c {
        '\t' => 4,
        '\r' | '\n' => 1,
        _ => unicode_width::UnicodeWidthChar::width_cjk(c).unwrap_or(1),
    }
}

fn str_display_width(s: &str) -> usize {
    s.chars().map(|c| char_display_width(c)).sum()
}

fn display_width_with_limit(text: &str, max_width: usize) -> usize {
    let mut acc = 0usize;
    for (idx, c) in text.char_indices() {
        acc += char_display_width(c);
        if acc > max_width {
            return acc;
        }
    }
    acc
}

fn overflow(text: &str, max_width: usize) -> bool {
    display_width_with_limit(text, max_width) > max_width
}

/// foo..
fn trim_right(text: &str, max_width: usize) -> &str {
    let mut trimmed = 0usize;
    for (idx, c) in text.char_indices() {
        trimmed += char_display_width(c);
        if trimmed > max_width {
            let (left, _) = text.split_at(idx);
            return left;
        }
    }
    text
}

/// ..foo
/// Truncate `text` until the displayed content can be fitted into `max_width`.
fn trim_left(text: &str, max_width: usize) -> (&str, usize) {
    // let mut text = text.to_string();

    let mut trimmed_width = 0usize;

    let mut diff = 0usize;
    let chars_count = text.chars().count();

    if chars_count > max_width {
        diff = chars_count - max_width;
        trimmed_width = diff;
        // text = text.chars().take(chars_count).skip(diff).collect();
    }

    let text_width = str_display_width(&text);

    for (idx, c) in text.char_indices().skip(diff) {
        println!("------ idx: {}, c: {}", idx, c);
        trimmed_width += char_display_width(c);
        if text_width - trimmed_width < max_width {
            let (_, right) = text.split_at(idx);
            return (right, idx);
        }
    }

    (text, 0usize)
}

fn truncate_text(
    winwidth: usize,
    text: &str,
    indices: &[usize],
    prefix_len: usize,
) -> Option<(String, Vec<usize>)> {
    let max_width = winwidth - prefix_len;

    let display_width = display_width_with_limit(text, max_width);

    if display_width > max_width {
        let last_matched = indices.last().copied()?;

        // ..................x.x....xx........x.....
        // <                                  >
        let trailing_trimmed: String = text.chars().take(last_matched).collect();
        println!("--- trailing_trimmed: {:?}", trailing_trimmed);

        if !overflow(&trailing_trimmed, max_width - DOTS_LEN) {
            println!("-------- 1");
            let mut trimmed = trim_right(text, max_width - DOTS_LEN).to_string();
            println!("-------- 2");
            trimmed.push_str(DOTS);

            println!("-------- 3");
            Some((trimmed, indices.to_owned()))
        } else {
            println!("-------- 4");
            let text = if overflow(
                text.chars().skip(last_matched).collect::<String>().as_str(),
                DOTS_LEN,
            ) {
                println!("-------- 5");
                // Stri..
                format!("{}{}", trailing_trimmed, DOTS)
            } else {
                text.into()
            };

            println!("-------- 6");
            // ..ri..
            let (text, diff) = trim_left(text.as_str(), max_width - DOTS_LEN);

            println!("-------- 7, diff: {}, indices: {:?}", diff, indices);
            let shifted_indices = indices
                .iter()
                .map(|x| {
                    if let Some(s) = x.checked_sub(diff) {
                        s
                    } else {
                        *x
                    }
                })
                .map(|x| x + 2)
                .take_while(|x| *x < max_width)
                .collect::<Vec<_>>();

            println!("-------- 8");
            Some((format!("{}{}", DOTS, text), shifted_indices))
        }
    } else {
        None
    }
}

// Returns true if text width is larger than the window width.
// fn overflow()

// let max_width = screen_width - prefix_len;
// maxe = min(max_width / 2, len(text))
//
// if display_width(text) > max_width - 2

/// Long matched lines can cause the matched items invisible.
///
/// # Arguments
///
/// - winwidth: width of the display window.
/// - skipped: number of skipped chars, used when need to skip the leading icons.
pub fn truncate_long_matched_lines<T>(
    lines: impl IntoIterator<Item = (SourceItem, T, Vec<usize>)>,
    winwidth: usize,
    skipped: Option<usize>,
) -> (Vec<(String, T, Vec<usize>)>, LinesTruncatedMap) {
    let mut truncated_map = HashMap::new();
    let mut lnum = 0usize;
    let winwidth = winwidth - WINWIDTH_OFFSET;
    let lines = lines
        .into_iter()
        .map(|(item, score, indices)| {
            let line = item.display_text.unwrap_or(item.raw);
            lnum += 1;

            if let Some((truncated, truncated_indices)) =
                truncate_text(winwidth, &line, &indices, skipped.unwrap_or_default())
            // truncate_line_impl(winwidth, &line, &indices, skipped)
            {
                truncated_map.insert(lnum, line);
                (truncated, score, truncated_indices)
            } else {
                (line, score, indices)
            }
        })
        .collect::<Vec<_>>();
    (lines, truncated_map)
}

pub fn truncate_grep_lines(
    lines: impl IntoIterator<Item = String>,
    indices: impl IntoIterator<Item = Vec<usize>>,
    winwidth: usize,
    skipped: Option<usize>,
) -> (Vec<String>, Vec<Vec<usize>>, LinesTruncatedMap) {
    let mut truncated_map = HashMap::new();
    let mut lnum = 0usize;
    let winwidth = winwidth - WINWIDTH_OFFSET;
    let (lines, indices): (Vec<String>, Vec<Vec<usize>>) = lines
        .into_iter()
        .zip(indices.into_iter())
        .map(|(line, indices)| {
            lnum += 1;

            if let Some((truncated, truncated_indices)) =
                crate::legacy::truncate_line_impl(winwidth, &line, &indices, skipped)
            {
                truncated_map.insert(lnum, line);
                (truncated, truncated_indices)
            } else {
                (line, indices)
            }
        })
        .unzip();
    (lines, indices, truncated_map)
}

/// Returns the info of the truncated top items ranked by the filtering score.
pub fn process_top_items<T>(
    top_list: impl IntoIterator<Item = (SourceItem, T, Vec<usize>)>,
    winwidth: usize,
    icon_painter: Option<IconPainter>,
) -> (Vec<String>, Vec<Vec<usize>>, LinesTruncatedMap) {
    let (truncated_lines, truncated_map) = truncate_long_matched_lines(top_list, winwidth, None);
    if let Some(painter) = icon_painter {
        let (lines, indices): (Vec<_>, Vec<Vec<usize>>) = truncated_lines
            .into_iter()
            .enumerate()
            .map(|(idx, (text, _, idxs))| {
                let iconized = if let Some(origin_text) = truncated_map.get(&(idx + 1)) {
                    format!("{} {}", painter.get_icon(origin_text), text)
                } else {
                    painter.paint(&text)
                };
                (iconized, idxs.iter().map(|x| x + ICON_LEN).collect())
            })
            .unzip();

        (lines, indices, truncated_map)
    } else {
        let (lines, indices): (Vec<_>, Vec<_>) = truncated_lines
            .into_iter()
            .map(|(text, _, idxs)| (text, idxs))
            .unzip();

        (lines, indices, truncated_map)
    }
}

/// Prints the results of filter::sync_run() to stdout.
pub fn print_sync_filter_results(
    ranked: Vec<FilterResult>,
    number: Option<usize>,
    winwidth: usize,
    icon_painter: Option<IconPainter>,
) {
    if let Some(number) = number {
        let total = ranked.len();
        let (lines, indices, truncated_map) =
            process_top_items(ranked.into_iter().take(number), winwidth, icon_painter);
        if truncated_map.is_empty() {
            println_json!(total, lines, indices);
        } else {
            println_json!(total, lines, indices, truncated_map);
        }
    } else {
        for (item, _, indices) in ranked.into_iter() {
            let text = item.display_text.unwrap_or(item.raw);
            println_json!(text, indices);
        }
    }
}

/// Prints the results of filter::dyn_run() to stdout.
pub fn print_dyn_filter_results(
    ranked: Vec<FilterResult>,
    total: usize,
    number: usize,
    winwidth: usize,
    icon_painter: Option<IconPainter>,
) {
    let (lines, indices, truncated_map) =
        process_top_items(ranked.into_iter().take(number), winwidth, icon_painter);

    if truncated_map.is_empty() {
        println_json_with_length!(total, lines, indices);
    } else {
        println_json_with_length!(total, lines, indices, truncated_map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filter::{
        matcher::{Algo, Bonus, MatchType, Matcher},
        Source,
    };
    use rayon::prelude::*;

    fn wrap_matches(line: &str, indices: &[usize]) -> String {
        let mut ret = String::new();
        let mut peekable = indices.iter().peekable();
        for (idx, ch) in line.chars().enumerate() {
            let next_id = **peekable.peek().unwrap_or(&&line.len());
            if next_id == idx {
                #[cfg(not(target_os = "windows"))]
                {
                    ret.push_str(
                        format!("{}{}{}", termion::style::Invert, ch, termion::style::Reset)
                            .as_str(),
                    );
                }

                #[cfg(target_os = "windows")]
                {
                    ret.push_str(format!("~{}~", ch).as_str());
                }

                peekable.next();
            } else {
                ret.push(ch);
            }
        }

        ret
    }

    fn run_test<I: Iterator<Item = SourceItem>>(
        source: Source<I>,
        query: &str,
        skipped: Option<usize>,
        winwidth: usize,
    ) {
        let matcher = Matcher::new(Algo::Fzy, MatchType::Full, Bonus::FileName);
        let mut ranked = source.filter(matcher, query).unwrap();
        ranked.par_sort_unstable_by(|(_, v1, _), (_, v2, _)| v2.partial_cmp(&v1).unwrap());

        let (truncated_lines, truncated_map) =
            truncate_long_matched_lines(ranked, winwidth, skipped);
        for (idx, (truncated_line, _score, truncated_indices)) in truncated_lines.iter().enumerate()
        {
            let highlighted = truncated_indices
                .iter()
                .filter_map(|i| truncated_line.chars().nth(*i))
                .collect::<String>();
            println!("\n   winwidth: {}", "─".repeat(winwidth));
            println!(
                "    display: {}",
                wrap_matches(&truncated_line, &truncated_indices)
            );
            println!("   raw_line: {}", truncated_map.get(&(idx + 1)).unwrap());
            println!("highlighted: {}", highlighted);
            // The highlighted result can be case insensitive.
            // assert!(query
            // .to_lowercase()
            // .starts_with(&highlighted.to_lowercase()));
        }
    }

    fn into_source(lines: Vec<&str>) -> Source<std::vec::IntoIter<SourceItem>> {
        Source::List(
            lines
                .into_iter()
                .map(|s| s.to_string().into())
                .collect::<Vec<SourceItem>>()
                .into_iter(),
        )
    }

    #[test]
    fn case1() {
        let source = into_source(vec![
          "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/file.scss",
          "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/another-file.scss",
          "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/file.js",
          "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/another-file.js"
        ]);
        let query = "files";
        run_test(source, query, None, 50usize);
    }

    #[test]
    fn case2() {
        let source = into_source(vec![
          "fuzzy-filter/target/debug/deps/librustversion-b273394e6c9c64f6.dylib.dSYM/Contents/Resources/DWARF/librustversion-b273394e6c9c64f6.dylib",
          "fuzzy-filter/target/debug/deps/librustversion-15764ff2535f190d.dylib.dSYM/Contents/Resources/DWARF/librustversion-15764ff2535f190d.dylib",
          "target/debug/deps/libstructopt_derive-3921fbf02d8d2ffe.dylib.dSYM/Contents/Resources/DWARF/libstructopt_derive-3921fbf02d8d2ffe.dylib",
          "target/debug/deps/libstructopt_derive-3921fbf02d8d2ffe.dylib.dSYM/Contents/Resources/DWARF/libstructopt_derive-3921fbf02d8d2ffe.dylib",
        ]);
        let query = "srlisresource";
        run_test(source, query, None, 50usize);
    }

    #[test]
    fn case3() {
        let source = into_source(vec![
          "/Users/xuliucheng/Library/Caches/Homebrew/universal-ctags--git/Units/afl-fuzz.r/github-issue-625-r.d/input.r"
        ]);
        let query = "srcggithub";
        run_test(source, query, None, 50usize);
    }

    #[test]
    fn case4() {
        let source = into_source(vec![
            "        // Wait until propagation delay period after block we plan to mine on",
        ]);
        let query = "bmine";
        run_test(source, query, None, 58usize);
    }

    #[test]
    fn test_grep_line() {
        let source = into_source(
        vec![" bin/node/cli/src/command.rs:127:1:                          let PartialComponents { client, task_manager, ..}"]
      );
        let query = "PartialComponents";
        run_test(source, query, Some(2), 64);
    }

    #[test]
    fn starting_point_should_work() {
        let source = into_source(vec![
          " crates/fuzzy_filter/target/debug/deps/librustversion-15764ff2535f190d.dylib.dSYM/Contents/Resources/DWARF/librustversion-15764ff2535f190d.dylib",
          " crates/fuzzy_filter/target/debug/deps/libstructopt_derive-5cce984f248086cc.dylib.dSYM/Contents/Resources/DWARF/libstructopt_derive-5cce984f248086cc.dylib",
        ]);
        let query = "srlisrlisrsr";
        run_test(source, query, Some(2), 50usize);

        let source  = into_source(vec![
          "crates/fuzzy_filter/target/debug/deps/librustversion-15764ff2535f190d.dylib.dSYM/Contents/Resources/DWARF/librustversion-15764ff2535f190d.dylib",
          "crates/fuzzy_filter/target/debug/deps/libstructopt_derive-5cce984f248086cc.dylib.dSYM/Contents/Resources/DWARF/libstructopt_derive-5cce984f248086cc.dylib",
        ]);
        let query = "srlisrlisrsr";
        run_test(source, query, None, 50usize);
    }

    #[test]
    fn test_print_multibyte_string_slice() {
        let multibyte_str = "README.md:23:1:Gourinath Banda. “Scalable Real-Time Kernel for Small Embedded Systems”. En- glish. PhD thesis. Denmark: University of Southern Denmark, June 2003. URL: http://citeseerx.ist.psu.edu/viewdoc/download;jsessionid=84D11348847CDC13691DFAED09883FCB?doi=10.1.1.118.1909&rep=rep1&type=pdf.";
        let start = 33;
        let end = 300;
        let expected = "Scalable Real-Time Kernel for Small Embedded Systems”. En- glish. PhD thesis. Denmark: University of Southern Denmark, June 2003. URL: http://citeseerx.ist.psu.edu/viewdoc/download;jsessionid=84D11348847CDC13691DFAED09883FCB?doi=10.1.1.118.1909&rep=rep1&type=pdf.";
        assert_eq!(expected, utf8_str_slice(multibyte_str, start, end));
    }

    #[test]
    fn test_trim_left() {
        let text = "helloworld";
        let max_width = 5;
        println!("trim_left: {:?}", trim_left(text, max_width));
        println!("trim_right: {:?}", trim_right(text, max_width));

        let text =
            "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/file.scss";
        let winwidth = 20;
        let indices = vec![2, 3, 30];
        println!(
            "truncate_text: {:?}",
            truncate_text(winwidth, text, &indices, 0)
        );
        let source = into_source(vec![
          "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/file.scss",
          "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/another-file.scss",
          "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/file.js",
          "directories/are/nested/a/lot/then/the/matched/items/will/be/invisible/another-file.js"
        ]);
        let query = "files";
        run_test(source, query, None, 50usize);
    }
}
