//! Whether a key combination may be saved for an action: not a lone typing key, not one Windows
//! or common apps own, not already taken by another action. A combination that only contains, or
//! is contained in, another one is saved with a warning. Pure: compares shortcut strings and never
//! touches a backend.
//!
//! The reserved list is data and applies on every platform (ADR-0007).

use std::collections::HashMap;
use std::sync::LazyLock;

use handy_keys::{Hotkey, Key, Modifiers};
use serde::Serialize;
use specta::Type;

use super::BindingSlot;
use crate::settings::ShortcutBinding;

/// Why a combination was not saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShortcutRefusal {
    /// Another action, or the other slot of the same one, has it. `take_over` says whether
    /// "use it here" can take it: never when it would leave dictation or cancel without a key.
    InUse {
        action: String,
        slot: BindingSlot,
        take_over: bool,
    },
    /// Windows or common apps own it (Ctrl+C, Alt+Tab, Win+H…).
    Reserved,
    /// A letter, digit, punctuation or editing key alone, or one modifier alone, would get in the
    /// way of typing; Esc alone only cancels.
    LoneKey,
}

/// Holding `smaller` (modifiers only) on the way to `larger` may fire the other action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ShortcutOverlap {
    pub action: String,
    pub smaller: String,
    pub larger: String,
}

/// Windows and app shortcuts nobody should lose to a dictation app: Wispr Flow's list of 59
/// plus what Windows 11 keeps for itself (Win+H voice typing, Win+V, Win+., Win+Shift+S).
const RESERVED: &str = "\
    ctrl ctrl_right ctrl+v ctrl+c ctrl+x ctrl+z ctrl+y ctrl+r ctrl+a ctrl+f ctrl+g ctrl+o \
    ctrl+s ctrl+p ctrl+n ctrl+t ctrl+w ctrl+b ctrl+i ctrl+u ctrl+k ctrl+shift+r ctrl+shift+t \
    ctrl+shift+home ctrl+shift+end f5 ctrl+f5 shift+f3 home end ctrl+home ctrl+end shift+home \
    shift+end alt+tab alt+f4 alt+left alt+right ctrl+= ctrl+- ctrl+backspace ctrl+del f11 \
    ctrl+alt+del ctrl+shift+escape printscreen alt+printscreen super+e super+r super+l super+d \
    super+tab super+i super+s super+x super+p super+up super+down super+q \
    super+h super+v super+. super+shift+s";

static RESERVED_COMBOS: LazyLock<Vec<Combo>> =
    LazyLock::new(|| RESERVED.split_whitespace().filter_map(parse).collect());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Any,
    Left,
    Right,
}

/// A combination as a set: the order of its parts does not matter.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Combo {
    /// Ctrl, Alt, Shift, Win and Fn, in that order.
    mods: [Option<Side>; 5],
    key: Option<Key>,
}

/// Read with the handy-keys parser, so every alias it accepts (`lctrl`, `altgr`, `win`…) counts.
/// `None` for a string it rejects.
fn parse(raw: &str) -> Option<Combo> {
    let hotkey: Hotkey = raw.trim().to_lowercase().parse().ok()?;
    let held = hotkey.modifiers;
    let side = |left, right| match (held.contains(left), held.contains(right)) {
        (true, true) => Some(Side::Any),
        (true, false) => Some(Side::Left),
        (false, true) => Some(Side::Right),
        (false, false) => None,
    };
    Some(Combo {
        mods: [
            side(Modifiers::CTRL_LEFT, Modifiers::CTRL_RIGHT),
            side(Modifiers::OPT_LEFT, Modifiers::OPT_RIGHT),
            side(Modifiers::SHIFT_LEFT, Modifiers::SHIFT_RIGHT),
            side(Modifiers::CMD_LEFT, Modifiers::CMD_RIGHT),
            held.contains(Modifiers::FN).then_some(Side::Any),
        ],
        key: hotkey.key,
    })
}

fn sides_match(a: Side, b: Side) -> bool {
    a == Side::Any || b == Side::Any || a == b
}

/// Both fire on the same keys: same key, same modifier groups, compatible sides.
fn same_keys(a: &Combo, b: &Combo) -> bool {
    a.key == b.key
        && a.mods.iter().zip(&b.mods).all(|pair| match pair {
            (None, None) => true,
            (Some(x), Some(y)) => sides_match(*x, *y),
            _ => false,
        })
}

/// `small` is modifiers only and every one of them is held in `large`, which has more.
fn part_of(small: &Combo, large: &Combo) -> bool {
    small.key.is_none()
        && small.mods.iter().any(Option::is_some)
        && !same_keys(small, large)
        && small.mods.iter().zip(&large.mods).all(|pair| match pair {
            (None, _) => true,
            (Some(x), Some(y)) => sides_match(*x, *y),
            (Some(_), None) => false,
        })
}

/// Keys that type or edit text: alone they would fire on every keystroke. Letters, digits and
/// punctuation are the keys whose name is one character.
fn types_text(key: Key) -> bool {
    key.to_string().chars().count() == 1
        || format!("{key:?}").starts_with("Keypad")
        || matches!(
            key,
            Key::Space
                | Key::Return
                | Key::Tab
                | Key::Delete
                | Key::ForwardDelete
                | Key::LeftArrow
                | Key::RightArrow
                | Key::UpArrow
                | Key::DownArrow
                | Key::PageUp
                | Key::PageDown
                | Key::JisUnderscore
        )
}

fn lone_key_refused(combo: &Combo, action: &str) -> bool {
    let modifiers = combo.mods.iter().flatten().count();
    match combo.key {
        // One modifier alone fires on every capital letter, or opens Start.
        None => modifiers == 1,
        Some(_) if modifiers > 0 => false,
        Some(Key::Escape) => action != "cancel",
        Some(key) => types_text(key),
    }
}

/// The combinations an action holds now, with their slot.
pub(crate) fn slots(binding: &ShortcutBinding) -> impl Iterator<Item = (BindingSlot, &str)> {
    [
        (BindingSlot::Primary, binding.current_binding.as_str()),
        (BindingSlot::Alternate, binding.alternate_binding.as_str()),
    ]
    .into_iter()
    .filter(|(_, raw)| !raw.trim().is_empty())
}

/// Whether `raw` may go into `action`'s `slot`. `Ok(Some)` saves with a warning. The first
/// combination that collides is reported; actions are visited by id so the answer is stable.
/// `take_over` in an `InUse` refusal is left `false`; the caller knows whether it is allowed.
pub fn check(
    raw: &str,
    action: &str,
    slot: BindingSlot,
    bindings: &HashMap<String, ShortcutBinding>,
) -> Result<Option<ShortcutOverlap>, ShortcutRefusal> {
    let Some(combo) = parse(raw) else {
        return Ok(None);
    };
    if lone_key_refused(&combo, action) {
        return Err(ShortcutRefusal::LoneKey);
    }
    if RESERVED_COMBOS.iter().any(|r| same_keys(&combo, r)) {
        return Err(ShortcutRefusal::Reserved);
    }
    let mut ids: Vec<&String> = bindings.keys().collect();
    ids.sort();
    let mut overlap = None;
    for id in ids {
        for (other_slot, other_raw) in slots(&bindings[id]) {
            if id == action && other_slot == slot {
                continue;
            }
            let Some(other) = parse(other_raw) else {
                continue;
            };
            if same_keys(&combo, &other) {
                return Err(ShortcutRefusal::InUse {
                    action: id.clone(),
                    slot: other_slot,
                    take_over: false,
                });
            }
            // Holding the action's own other combination fires the same action.
            if overlap.is_none() && id != action {
                let pair = if part_of(&combo, &other) {
                    Some((raw, other_raw))
                } else if part_of(&other, &combo) {
                    Some((other_raw, raw))
                } else {
                    None
                };
                overlap = pair.map(|(smaller, larger)| ShortcutOverlap {
                    action: id.clone(),
                    smaller: smaller.to_string(),
                    larger: larger.to_string(),
                });
            }
        }
    }
    Ok(overlap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use BindingSlot::{Alternate as Alt, Primary as Pri};

    fn bindings() -> HashMap<String, ShortcutBinding> {
        [
            ("transcribe", "ctrl+shift+space", "ctrl_left+super"),
            ("cancel", "escape", ""),
            ("paste_last", "", ""),
            ("copy_last", "alt+shift+x", ""),
            ("meeting_toggle", "", ""),
        ]
        .into_iter()
        .map(|(id, primary, alternate)| {
            let binding = ShortcutBinding {
                id: id.to_string(),
                name: String::new(),
                description: String::new(),
                default_binding: String::new(),
                current_binding: primary.to_string(),
                alternate_binding: alternate.to_string(),
            };
            (id.to_string(), binding)
        })
        .collect()
    }

    /// The verdict as one short line, so the table stays readable.
    fn verdict(raw: &str, action: &str, slot: BindingSlot) -> String {
        match check(raw, action, slot, &bindings()) {
            Ok(None) => "ok".to_string(),
            Ok(Some(o)) => format!("overlap {} {} < {}", o.action, o.smaller, o.larger),
            Err(ShortcutRefusal::InUse { action, slot, .. }) => format!("in use {action} {slot:?}"),
            Err(ShortcutRefusal::Reserved) => "reserved".to_string(),
            Err(ShortcutRefusal::LoneKey) => "lone key".to_string(),
        }
    }

    #[test]
    fn conflict_table() {
        // "<combination> <action> <slot 1|2>: <verdict>"
        let cases = [
            // Already used by another action, in any order and with either side of a modifier.
            "ctrl+shift+space paste_last 1: in use transcribe Primary",
            "space+shift+ctrl paste_last 2: in use transcribe Primary",
            "ctrl_left+shift+space paste_last 1: in use transcribe Primary",
            "shift+alt+x meeting_toggle 1: in use copy_last Primary",
            "ctrl+win copy_last 1: in use transcribe Alternate",
            // Or by the other slot of the same action; its own slot is no conflict.
            "ctrl+shift+space transcribe 2: in use transcribe Primary",
            "ctrl+shift+space transcribe 1: ok",
            // Left and right are different keys.
            "ctrl_right+super copy_last 1: ok",
            // Reserved by Windows or the apps, on either side.
            "ctrl+c paste_last 1: reserved",
            "ctrl_left+c paste_last 1: reserved",
            "super+h transcribe 1: reserved",
            "win+h transcribe 2: reserved",
            "shift+super+s copy_last 1: reserved",
            "alt+tab copy_last 1: reserved",
            // Any spelling the backend reads is checked.
            "lctrl+c paste_last 1: reserved",
            // One modifier alone fires on every capital letter or opens Start.
            "ctrl copy_last 1: lone key",
            "shift_left paste_last 1: lone key",
            "super paste_last 1: lone key",
            // A key alone that types; function and media keys alone are fine.
            "a paste_last 1: lone key",
            "7 paste_last 1: lone key",
            ". paste_last 1: lone key",
            "space transcribe 1: lone key",
            "f9 paste_last 1: ok",
            "playpause meeting_toggle 1: ok",
            // Esc alone only cancels.
            "escape paste_last 1: lone key",
            "escape cancel 1: ok",
            "ctrl+escape paste_last 1: ok",
            // What the Fala accepts today stays valid: modifiers with space, modifiers only.
            "ctrl_left+space paste_last 1: ok",
            "alt+ctrl paste_last 1: ok",
            // Holding modifiers on the way to a larger combination warns, both ways.
            "ctrl+super+alt meeting_toggle 1: overlap transcribe ctrl_left+super < ctrl+super+alt",
            "alt+shift meeting_toggle 1: overlap copy_last alt+shift < alt+shift+x",
            // The action's own other combination is no overlap.
            "ctrl+super+alt transcribe 1: ok",
            // A key with more modifiers does not fire the one with fewer.
            "ctrl+shift+alt+space meeting_toggle 1: ok",
        ];
        for case in cases {
            let (input, expected) = case.split_once(": ").unwrap();
            let [raw, action, slot] = input.split(' ').collect::<Vec<_>>()[..] else {
                panic!("bad case {case}");
            };
            let slot = if slot == "1" { Pri } else { Alt };
            assert_eq!(verdict(raw, action, slot), expected, "{input}");
        }
    }

    #[test]
    fn reserved_list_parses() {
        let reserved: Vec<&str> = RESERVED.split_whitespace().collect();
        assert_eq!(reserved.len(), 59 + 4);
        for raw in reserved {
            assert!(parse(raw).is_some(), "{raw}");
        }
    }
}
