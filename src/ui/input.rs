//! Playing notes from the computer keyboard and the on-screen keys. The key layout comes from a
//! keymap file (see `assets/keymaps/`), so other layouts such as AZERTY can be added as data.

use nih_plug_egui::egui::{self, Context, Event, Key};
use serde::Deserialize;
use std::collections::HashSet;

use crate::shared::Shared;

#[derive(Debug, Clone, Deserialize)]
struct KeymapFile {
    #[allow(dead_code)] // Identifies the file; read once keymaps become selectable
    id: String,
    base_note: u8,
    notes: Vec<KeymapNote>,
}

#[derive(Debug, Clone, Deserialize)]
struct KeymapNote {
    key: String,
    semitone: u8,
}

pub struct Keymap {
    /// MIDI note played by semitone 0. The octave switch transposes the sound, not the keys.
    pub base_note: u8,
    /// Keys and their semitone offset from the base note.
    pub notes: Vec<(Key, u8)>,
}

impl Keymap {
    pub fn parse(source: &str) -> Result<Self, String> {
        let file: KeymapFile = toml::from_str(source).map_err(|e| e.to_string())?;
        let key = |name: &str| Key::from_name(name).ok_or_else(|| format!("unknown key '{name}'"));
        Ok(Self {
            base_note: file.base_note,
            notes: file
                .notes
                .iter()
                .map(|n| Ok((key(&n.key)?, n.semitone)))
                .collect::<Result<_, String>>()?,
        })
    }

    /// The key that plays `semitone` above the base note, if any.
    pub fn key_for(&self, semitone: u8) -> Option<Key> {
        self.notes
            .iter()
            .find(|(_, s)| *s == semitone)
            .map(|(k, _)| *k)
    }
}

pub struct NoteInput {
    /// The note played by the keymap's semitone 0.
    pub base_note: u8,
    /// Computer keys currently held. Tracked from key events rather than egui's own key state so
    /// everything can be released when the window loses focus.
    pub held_keys: HashSet<Key>,
    /// The on-screen key held with the mouse.
    pub mouse_note: Option<u8>,
}

impl NoteInput {
    pub fn new(keymap: &Keymap) -> Self {
        Self {
            base_note: keymap.base_note,
            held_keys: HashSet::new(),
            mouse_note: None,
        }
    }

    pub fn handle_events(&mut self, ctx: &Context) {
        grab_keyboard_focus_on_click(ctx);
        ctx.input(|i| {
            if i.viewport().focused == Some(false) {
                self.held_keys.clear();
                return;
            }
            for event in &i.events {
                match event {
                    Event::Key {
                        key,
                        physical_key,
                        pressed,
                        repeat: false,
                        ..
                    } => {
                        // Where the key is, not what it types, so Shift, Caps Lock and other
                        // keyboard layouts don't change which note it plays
                        let key = physical_key.unwrap_or(*key);
                        nih_plug::nih_trace!(
                            "key {key:?} {}",
                            if *pressed { "down" } else { "up" }
                        );
                        if *pressed {
                            self.held_keys.insert(key);
                        } else {
                            self.held_keys.remove(&key);
                        }
                    }
                    Event::WindowFocused(false) => self.held_keys.clear(),
                    _ => (),
                }
            }
        });
    }

    pub fn is_held(&self, note: u8, keymap: &Keymap) -> bool {
        self.mouse_note == Some(note)
            || keymap
                .notes
                .iter()
                .any(|(k, s)| self.base_note + s == note && self.held_keys.contains(k))
    }

    /// Sends the currently held notes to the audio thread.
    pub fn publish(&self, keymap: &Keymap, shared: &Shared) {
        let mut mask = [0u64; 2];
        let mut set = |note: u8| {
            if note < 128 {
                mask[note as usize / 64] |= 1 << (note % 64);
            }
        };
        for (key, semitone) in &keymap.notes {
            if self.held_keys.contains(key) {
                set(self.base_note + semitone);
            }
        }
        if let Some(note) = self.mouse_note {
            set(note);
        }
        shared.set_gui_notes(mask);
    }
}

/// On Windows the editor is a child window that doesn't take keyboard focus when clicked.
/// egui-baseview's `windows_keyboard_workaround` focuses the window whenever egui reports a UI
/// event, so report one for any click inside the editor.
fn grab_keyboard_focus_on_click(ctx: &Context) {
    if ctx.input(|i| i.pointer.any_pressed()) {
        ctx.output_mut(|o| {
            o.events
                .push(egui::output::OutputEvent::Clicked(egui::WidgetInfo::new(
                    egui::WidgetType::Other,
                )))
        });
    }
}
