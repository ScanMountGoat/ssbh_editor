use egui::{Key, KeyboardShortcut, Modifiers};

pub const OPEN_FOLDER: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);

pub const ADD_FOLDER: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::O);

pub const RELOAD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::R);

pub fn format_shortcut(shortcut: &KeyboardShortcut) -> String {
    // egui has this method but doesn't format shortcut symbols.
    // TODO: This function might not be needed on newer versions.
    // TODO: Store keyboard shortcuts in a single place?
    let ctrl = if cfg!(target_os = "macos") {
        "⌘ "
    } else {
        "Ctrl+"
    };

    let ctrl_shift = if cfg!(target_os = "macos") {
        "⇧ ⌘ "
    } else {
        "Ctrl+Shift+"
    };

    let key = shortcut.logical_key.name();
    if shortcut.modifiers.command {
        if shortcut.modifiers.shift {
            format!("{ctrl_shift}{key}")
        } else {
            format!("{ctrl}{key}")
        }
    } else {
        key.to_owned()
    }
}
