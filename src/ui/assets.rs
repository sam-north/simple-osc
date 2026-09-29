//! Loads themes, language packs and keymaps. Built-in ones are compiled into the plugin; extra
//! ones can be dropped into the user folder (see [`user_dir`]) and are picked up the next time
//! the editor opens. A user file with the same `id` as a built-in one replaces it.

use nih_plug::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::i18n::{Language, DEFAULT_LANGUAGE};
use super::input::Keymap;
use super::theme::Theme;

pub const DEFAULT_THEME: &str = "grunge";

const BUILTIN_THEMES: &[(&str, &str)] = &[
    (
        "grunge.toml",
        include_str!("../../assets/themes/grunge.toml"),
    ),
    (
        "cyberpunk.toml",
        include_str!("../../assets/themes/cyberpunk.toml"),
    ),
    (
        "vintage.toml",
        include_str!("../../assets/themes/vintage.toml"),
    ),
];
const BUILTIN_LANGUAGES: &[(&str, &str)] = &[
    (
        "en-US.toml",
        include_str!("../../assets/languages/en-US.toml"),
    ),
    ("es.toml", include_str!("../../assets/languages/es.toml")),
    ("fr.toml", include_str!("../../assets/languages/fr.toml")),
];
const BUILTIN_KEYMAP: &str = include_str!("../../assets/keymaps/qwerty.toml");

/// Folder for user themes (`themes/*.toml`), language packs (`languages/*.toml`) and saved
/// preferences. `%APPDATA%\Simple Osc` on Windows.
pub fn user_dir() -> Option<PathBuf> {
    const FOLDER: &str = "Simple Osc";
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join(FOLDER))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|p| {
            PathBuf::from(p)
                .join("Library/Application Support")
                .join(FOLDER)
        })
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
            .map(|p| p.join(FOLDER))
    }
}

pub struct Library {
    pub themes: Vec<Theme>,
    pub languages: Vec<Language>,
    pub keymap: Keymap,
}

impl Library {
    pub fn load() -> Self {
        let user = user_dir();
        let themes = load_all(
            BUILTIN_THEMES,
            user.as_deref().map(|d| d.join("themes")),
            |t: &Theme| &t.id,
        );
        let languages = load_all(
            BUILTIN_LANGUAGES,
            user.as_deref().map(|d| d.join("languages")),
            |l: &Language| &l.id,
        );
        let keymap = Keymap::parse(BUILTIN_KEYMAP).expect("built-in keymap is valid");
        Self {
            themes,
            languages,
            keymap,
        }
    }

    pub fn theme_index(&self, id: &str) -> usize {
        self.themes.iter().position(|t| t.id == id).unwrap_or(0)
    }

    pub fn language_index(&self, id: &str) -> usize {
        self.languages.iter().position(|l| l.id == id).unwrap_or(0)
    }

    pub fn default_language(&self) -> &Language {
        &self.languages[self.language_index(DEFAULT_LANGUAGE)]
    }
}

/// Parses the built-in files, then overlays any valid files from `user_dir`.
fn load_all<T: for<'de> Deserialize<'de>>(
    builtin: &[(&str, &str)],
    user_dir: Option<PathBuf>,
    id: impl Fn(&T) -> &String,
) -> Vec<T> {
    let mut items: Vec<T> = builtin
        .iter()
        .map(|(name, src)| toml::from_str(src).unwrap_or_else(|e| panic!("built-in {name}: {e}")))
        .collect();

    for (path, src) in user_dir.as_deref().map(read_toml_files).unwrap_or_default() {
        match toml::from_str::<T>(&src) {
            Ok(item) => match items.iter().position(|i| id(i) == id(&item)) {
                Some(existing) => items[existing] = item,
                None => items.push(item),
            },
            Err(e) => nih_warn!("Skipping {}: {e}", path.display()),
        }
    }
    items
}

fn read_toml_files(dir: &Path) -> Vec<(PathBuf, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .filter_map(|p| std::fs::read_to_string(&p).ok().map(|s| (p, s)))
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// The last chosen theme and language, shared by every instance of the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefs {
    pub theme: String,
    pub language: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_owned(),
            language: DEFAULT_LANGUAGE.to_owned(),
        }
    }
}

impl Prefs {
    fn path() -> Option<PathBuf> {
        user_dir().map(|d| d.join("prefs.toml"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        let result = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&path, toml::to_string(self).unwrap_or_default()));
        if let Err(e) = result {
            nih_warn!("Could not save {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin<T: for<'de> Deserialize<'de>>(files: &[(&str, &str)]) -> Vec<T> {
        load_all(files, None, |_: &T| unreachable!())
    }

    #[test]
    fn builtin_themes_parse() {
        let themes: Vec<Theme> = builtin(BUILTIN_THEMES);
        assert!(themes.iter().any(|t| t.id == DEFAULT_THEME));
    }

    #[test]
    fn builtin_keymap_parses() {
        Keymap::parse(BUILTIN_KEYMAP).unwrap();
    }

    /// Every language pack must translate every key the default language has.
    #[test]
    fn languages_are_complete() {
        let languages: Vec<Language> = builtin(BUILTIN_LANGUAGES);
        let default = languages.iter().find(|l| l.id == DEFAULT_LANGUAGE).unwrap();
        for lang in &languages {
            assert_eq!(lang.notes.names.len(), 12, "{}", lang.id);
            for key in default.strings.keys() {
                assert!(
                    lang.strings.contains_key(key),
                    "{} is missing '{key}'",
                    lang.id
                );
            }
        }
    }

    #[test]
    fn octave_switch_positions_are_signed() {
        let languages: Vec<Language> = builtin(BUILTIN_LANGUAGES);
        let en = languages.iter().find(|l| l.id == DEFAULT_LANGUAGE).unwrap();
        let strings = super::super::i18n::Strings {
            language: en,
            fallback: en,
        };
        let params = crate::SimpleOscParams::default();
        let sections = super::super::controls::sections(&params);
        let octave = sections
            .iter()
            .flat_map(|s| &s.slots)
            .flat_map(|s| s.controls())
            .find(|c| c.key == "octave")
            .unwrap();
        assert_eq!(
            octave.position_labels(&strings),
            ["-2", "-1", "0", "+1", "+2"]
        );
    }

    /// Every text key the GUI asks for must exist in the default language.
    #[test]
    fn default_language_covers_all_controls() {
        let languages: Vec<Language> = builtin(BUILTIN_LANGUAGES);
        let default = languages.iter().find(|l| l.id == DEFAULT_LANGUAGE).unwrap();
        let params = crate::SimpleOscParams::default();
        let mut keys = super::super::STATIC_TEXT_KEYS
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>();
        for section in super::super::controls::sections(&params) {
            keys.push(format!("section.{}", section.key));
            for control in section.slots.iter().flat_map(|s| s.controls()) {
                keys.push(control.label_key());
                if let super::super::controls::ValueFormat::Choice { group } = control.format {
                    let steps = control.binding.step_count().unwrap_or(0);
                    let ids = match group {
                        "filter" => <crate::dsp::FilterType as Enum>::ids(),
                        other => panic!("unknown choice group {other}"),
                    }
                    .expect("choice enums need #[id] attributes");
                    assert_eq!(ids.len(), steps + 1);
                    keys.extend(ids.iter().map(|id| format!("choice.{group}.{id}")));
                }
                if let super::super::controls::ValueFormat::Wave = control.format {
                    for wave in crate::waves::catalog() {
                        keys.push(format!("category.{}", wave.category.id()));
                        if let Some(role) = wave.shape {
                            keys.push(format!("shape.{}", role.id()));
                        }
                        // Sample-based waves may use their built-in names; basic ones need text
                        if wave.category == crate::waves::Category::Basic {
                            keys.push(format!("wave.{}", wave.id));
                        }
                    }
                    keys.push("control.shape".to_owned());
                }
            }
        }
        for key in keys {
            assert!(
                default.strings.contains_key(&key),
                "en-US is missing '{key}'"
            );
        }
    }
}
