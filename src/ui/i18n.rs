//! Language packs. Every piece of text the GUI shows is looked up by key in a pack loaded from
//! TOML (see `assets/languages/`). Missing keys fall back to the default language, then to the
//! key itself, so a partial translation still works.

use serde::Deserialize;
use std::collections::HashMap;

pub const DEFAULT_LANGUAGE: &str = "en-US";

#[derive(Debug, Clone, Deserialize)]
pub struct Language {
    pub id: String,
    /// The language's name in that language, e.g. "Français".
    pub name: String,
    /// Short label for the language switcher, e.g. "FR".
    pub short_name: String,
    pub number: NumberFormat,
    pub notes: NoteNaming,
    pub strings: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NumberFormat {
    pub decimal_separator: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NoteNaming {
    /// The 12 pitch class names starting at C.
    pub names: Vec<String>,
    /// Octave number of MIDI note 0. -1 makes middle C "C4"; -2 makes it "Do3".
    pub octave_offset: i32,
}

/// The active language plus the fallback used for missing keys.
pub struct Strings<'a> {
    pub language: &'a Language,
    pub fallback: &'a Language,
}

impl Strings<'_> {
    pub fn text(&self, key: &str) -> String {
        self.language
            .strings
            .get(key)
            .or_else(|| self.fallback.strings.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    /// Like [`Self::text`], but uses `default` when no language pack has the key.
    pub fn text_or(&self, key: &str, default: &str) -> String {
        self.language
            .strings
            .get(key)
            .or_else(|| self.fallback.strings.get(key))
            .cloned()
            .unwrap_or_else(|| default.to_owned())
    }

    /// Looks up `key` and replaces `{name}` placeholders.
    pub fn format(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut text = self.text(key);
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    /// Formats a number with `decimals` digits using the language's decimal separator.
    pub fn number(&self, value: f64, decimals: usize) -> String {
        let text = format!("{value:.decimals$}");
        // Avoid showing "-0" for tiny negative values
        let text = match text.strip_prefix('-') {
            Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_owned(),
            _ => text,
        };
        text.replace('.', &self.language.number.decimal_separator)
    }

    pub fn note_name(&self, note: u8) -> String {
        let names = &self.language.notes.names;
        let name = names
            .get(note as usize % 12)
            .map(String::as_str)
            .unwrap_or("?");
        let octave = note as i32 / 12 + self.language.notes.octave_offset;
        format!("{name}{octave}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lang(sep: &str, names: &[&str], offset: i32, strings: &[(&str, &str)]) -> Language {
        Language {
            id: "t".into(),
            name: "T".into(),
            short_name: "T".into(),
            number: NumberFormat {
                decimal_separator: sep.into(),
            },
            notes: NoteNaming {
                names: names.iter().map(|s| s.to_string()).collect(),
                octave_offset: offset,
            },
            strings: strings
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn falls_back_and_formats() {
        let fr = lang(",", &["Do"; 12], -2, &[("a", "Bonjour {who}")]);
        let en = lang(
            ".",
            &["C"; 12],
            -1,
            &[("a", "Hello {who}"), ("b", "Only English")],
        );
        let s = Strings {
            language: &fr,
            fallback: &en,
        };
        assert_eq!(s.format("a", &[("who", "Sam")]), "Bonjour Sam");
        assert_eq!(s.text("b"), "Only English");
        assert_eq!(s.text("missing.key"), "missing.key");
        assert_eq!(s.number(5.26, 1), "5,3");
        assert_eq!(s.number(-0.2, 0), "0");
        assert_eq!(s.note_name(60), "Do3");
    }
}
