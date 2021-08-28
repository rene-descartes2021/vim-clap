use std::borrow::Borrow;
use std::borrow::Cow;
use structopt::clap::arg_enum;

use pattern::{file_name_only, strip_grep_filepath, tag_name_only};

/// A tuple of match text piece (matching_text, offset_of_matching_text).
pub type MatchText<'a> = (&'a str, usize);

arg_enum! {
  #[derive(Debug, Clone)]
  pub enum MatchType {
      Full,
      TagName,
      FileName,
      IgnoreFilePath,
  }
}

impl From<String> for MatchType {
    fn from(match_type: String) -> Self {
        match_type.as_str().into()
    }
}

impl From<&String> for MatchType {
    fn from(match_type: &String) -> Self {
        match_type.as_str().into()
    }
}

impl From<&str> for MatchType {
    fn from(match_type: &str) -> Self {
        match match_type.to_lowercase().as_str() {
            "full" => Self::Full,
            "tagname" => Self::TagName,
            "filename" => Self::FileName,
            "ignorefilepath" => Self::IgnoreFilePath,
            _ => Self::Full,
        }
    }
}

/// Extracts the text for running the matcher.
pub trait MatchTextFor<'a> {
    fn match_text_for(&self, match_ty: &MatchType) -> Option<MatchText>;
}

impl<'a> MatchTextFor<'a> for SourceItem<'_> {
    fn match_text_for(&self, match_type: &MatchType) -> Option<MatchText> {
        self.match_text_for(match_type)
    }
}

#[derive(Debug, Clone)]
pub struct SourceItem<'a> {
    /// Raw line content of the input stream.
    pub raw: Cow<'a, str>,
    /// Text for matching.
    pub match_text: Option<(String, usize)>,
    /// The display text can be built when creating a new source item.
    pub display_text: Option<String>,
}

impl<'a> From<Cow<'a, str>> for SourceItem<'a> {
    fn from(raw: Cow<'a, str>) -> Self {
        Self {
            raw,
            display_text: None,
            match_text: None,
        }
    }
}

impl<'a> From<&'a str> for SourceItem<'a> {
    fn from(s: &'a str) -> Self {
        Self {
            raw: s.into(),
            display_text: None,
            match_text: None,
        }
    }
}

impl From<String> for SourceItem<'_> {
    fn from(raw: String) -> Self {
        Self {
            raw: raw.into(),
            display_text: None,
            match_text: None,
        }
    }
}

impl<'a> SourceItem<'a> {
    /// Constructs `SourceItem`.
    pub fn new(
        raw: Cow<'a, str>,
        match_text: Option<(String, usize)>,
        display_text: Option<String>,
    ) -> Self {
        Self {
            raw,
            match_text,
            display_text,
        }
    }

    pub fn display_text(&self) -> Cow<'_, str> {
        if let Some(ref text) = self.display_text {
            text.into()
        } else {
            self.raw.to_owned()
        }
    }

    pub fn match_text(&self) -> Cow<'_, str> {
        if let Some((ref text, _)) = self.match_text {
            text.into()
        } else {
            self.raw.to_owned()
        }
    }

    pub fn match_text_for(&self, match_ty: &MatchType) -> Option<MatchText> {
        if let Some((ref text, offset)) = self.match_text {
            return Some((text, offset));
        }
        match match_ty {
            MatchType::Full => Some((self.raw.borrow(), 0)),
            MatchType::TagName => tag_name_only(self.raw.borrow()).map(|s| (s, 0)),
            MatchType::FileName => file_name_only(self.raw.borrow()),
            MatchType::IgnoreFilePath => strip_grep_filepath(self.raw.borrow()),
        }
    }
}

/// This struct represents the filtered result of [`SourceItem`].
#[derive(Debug, Clone)]
pub struct FilteredItem<'a, T = i64> {
    /// Tuple of (matched line text, filtering score, indices of matched elements)
    pub source_item: SourceItem<'a>,
    /// Filtering score.
    pub score: T,
    /// Indices of matched elements.
    ///
    /// The indices may be truncated when truncating the text.
    pub match_indices: Vec<usize>,
    /// The text might be truncated for fitting into the display window.
    pub display_text: Option<String>,
}

impl<'a, T> From<(SourceItem<'a>, T, Vec<usize>)> for FilteredItem<'a, T> {
    fn from((source_item, score, match_indices): (SourceItem<'a>, T, Vec<usize>)) -> Self {
        Self {
            source_item,
            score,
            match_indices,
            display_text: None,
        }
    }
}

impl<T> From<(String, T, Vec<usize>)> for FilteredItem<'_, T> {
    fn from((text, score, match_indices): (String, T, Vec<usize>)) -> Self {
        Self {
            source_item: text.into(),
            score,
            match_indices,
            display_text: None,
        }
    }
}

impl<'a, T> FilteredItem<'a, T> {
    pub fn new<I: Into<SourceItem<'a>>>(item: I, score: T, match_indices: Vec<usize>) -> Self {
        Self {
            source_item: item.into(),
            score,
            match_indices,
            display_text: None,
        }
    }

    pub fn display_text_before_truncated(&self) -> Cow<'_, str> {
        self.source_item.display_text()
    }

    pub fn display_text(&self) -> String {
        if let Some(ref text) = self.display_text {
            text.into()
        } else {
            match self.source_item.display_text() {
                Cow::Owned(s) => s,
                Cow::Borrowed(s) => s.to_string(),
            }
        }
    }

    /// Returns the match indices shifted by `offset`.
    pub fn shifted_indices(&self, offset: usize) -> Vec<usize> {
        self.match_indices.iter().map(|x| x + offset).collect()
    }

    pub fn deconstruct(self) -> (SourceItem<'a>, T, Vec<usize>) {
        let Self {
            source_item,
            score,
            match_indices,
            ..
        } = self;
        (source_item, score, match_indices)
    }
}
