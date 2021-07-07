use serde::{de, Deserialize, Deserializer};
use std::str::FromStr;

/// Cheapie filename escape thing to flaten the paths
/// so we don't actually need to create the whole hierarchy
/// when generating the report.
pub(crate) fn sanitize_filename(input: &str) -> String {
    format!("{}.html", input.replace("/", "_"))
}

/// Maps a percent to a color.  Will panic on negative values.
pub(crate) fn color_for_percent<'a>(percent: f64) -> &'a str {
    match percent {
        i if i < 75.0 => "red",
        i if i >= 75.0 && i < 90.0 => "yellow",
        i if i >= 90.0 => "green",
        _ => unimplemented!(),
    }
}

/// Turns out String::insert_str will panic if we don't know where our character boundaries are e.g.
/// multibyte characters (e.g. Cyrillic) mean the byte and character boundaries are in different locations.
pub(crate) trait InsertAtCharacter {
    fn insert_at_char(&mut self, index: usize, s: &str);
}

impl InsertAtCharacter for String {
    fn insert_at_char(&mut self, index: usize, s: &str) {
        let char_indexes = self.char_indices().collect::<Vec<_>>();

        if index >= char_indexes.len() {
            self.push_str(s)
        } else {
            let index = char_indexes[index].0;
            if index >= self.len() {
                self.push_str(s)
            } else {
                if index > 0 {
                    self.insert_str(index - 1, s)
                } else {
                    self.insert_str(index, s)
                }
            }
        }
    }
}

// Ah boilerplate
// https://github.com/serde-rs/json/issues/317
pub(crate) fn deser_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}
