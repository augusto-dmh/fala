//! Whether a key combination may be saved for an action: not a lone typing key, not one Windows
//! or common apps own, not already taken by another action. A combination that only contains, or
//! is contained in, another one is saved with a warning, except next to the meeting shortcut:
//! starting a recording must never be a side effect of another shortcut (ADR-0005), so the
//! meeting shortcut needs a key besides modifiers and may not overlap another one either way.
//! Pure: compares shortcut strings and never touches a backend.
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
    /// The meeting shortcut with modifiers only: they are held on the way to other shortcuts, and
    /// a recording only starts on purpose (ADR-0005).
    MeetingNeedsKey,
    /// The combination contains, or is contained in, `action`'s, and one of the two is the meeting
    /// shortcut: holding one on the way to the other could start or stop a recording.
    MeetingOverlap { action: String },
}

/// The action that starts and stops a meeting recording.
pub const MEETING_ACTION: &str = "meeting_toggle";

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
    if action == MEETING_ACTION && combo.key.is_none() {
        return Err(ShortcutRefusal::MeetingNeedsKey);
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
            if id != action {
                let pair = if part_of(&combo, &other) {
                    Some((raw, other_raw))
                } else if part_of(&other, &combo) {
                    Some((other_raw, raw))
                } else {
                    None
                };
                if pair.is_some() && (action == MEETING_ACTION || id == MEETING_ACTION) {
                    return Err(ShortcutRefusal::MeetingOverlap { action: id.clone() });
                }
                if overlap.is_some() {
                    continue;
                }
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

/// Whether a refusal is one of the meeting rules (ADR-0005), which also apply to what is already
/// saved: those combinations are never registered.
pub fn is_meeting_refusal(refusal: &ShortcutRefusal) -> bool {
    matches!(
        refusal,
        ShortcutRefusal::MeetingNeedsKey | ShortcutRefusal::MeetingOverlap { .. }
    )
}

/// The meeting binding without the combinations the meeting rules refuse, so a combination saved
/// before them (modifiers only, or one overlapping another shortcut) never starts a recording.
/// Any other binding comes back as it is.
pub fn registrable(
    binding: &ShortcutBinding,
    bindings: &HashMap<String, ShortcutBinding>,
) -> ShortcutBinding {
    let mut safe = binding.clone();
    if binding.id != MEETING_ACTION {
        return safe;
    }
    for (slot, raw) in slots(binding) {
        if let Err(refusal) = check(raw, &binding.id, slot, bindings) {
            if is_meeting_refusal(&refusal) {
                match slot {
                    BindingSlot::Primary => safe.current_binding.clear(),
                    BindingSlot::Alternate => safe.alternate_binding.clear(),
                }
            }
        }
    }
    safe
}

/// What the saved shortcuts already get wrong, slot by slot, in action order: the meeting rules
/// they break and the overlaps they only warn about. The shortcuts dialog shows them on opening.
pub fn standing_problems(
    bindings: &HashMap<String, ShortcutBinding>,
) -> Vec<(
    String,
    BindingSlot,
    String,
    Result<ShortcutOverlap, ShortcutRefusal>,
)> {
    let mut ids: Vec<&String> = bindings.keys().collect();
    ids.sort();
    let mut problems = Vec::new();
    for id in ids {
        for (slot, raw) in slots(&bindings[id]) {
            let found = match check(raw, id, slot, bindings) {
                Ok(Some(overlap)) => Ok(overlap),
                Err(refusal) if is_meeting_refusal(&refusal) => Err(refusal),
                _ => continue,
            };
            problems.push((id.clone(), slot, raw.to_string(), found));
        }
    }
    problems
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
            Err(ShortcutRefusal::MeetingNeedsKey) => "meeting needs key".to_string(),
            Err(ShortcutRefusal::MeetingOverlap { action }) => format!("meeting overlap {action}"),
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
            "ctrl+super+alt paste_last 1: overlap transcribe ctrl_left+super < ctrl+super+alt",
            "alt+shift paste_last 1: overlap copy_last alt+shift < alt+shift+x",
            // The action's own other combination is no overlap.
            "ctrl+super+alt transcribe 1: ok",
            // The meeting shortcut needs a key: modifiers alone are held on the way to others.
            "ctrl+shift meeting_toggle 1: meeting needs key",
            "alt+shift meeting_toggle 2: meeting needs key",
            // And it may not overlap another shortcut, either way (ADR-0005).
            "ctrl+super+m meeting_toggle 1: meeting overlap transcribe",
            "ctrl+alt+m meeting_toggle 1: ok",
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
    fn meeting_overlap_is_refused_from_either_side() {
        let mut with_meeting = bindings();
        if let Some(meeting) = with_meeting.get_mut(MEETING_ACTION) {
            meeting.current_binding = "ctrl+alt+m".to_string();
        }
        // Another action whose modifiers are held on the way to the meeting shortcut.
        assert_eq!(
            check(
                "ctrl+alt",
                "paste_last",
                BindingSlot::Primary,
                &with_meeting
            ),
            Err(ShortcutRefusal::MeetingOverlap {
                action: MEETING_ACTION.to_string()
            })
        );
        // Unrelated combinations still save, and other overlaps still only warn.
        assert_eq!(
            check(
                "ctrl+alt+p",
                "paste_last",
                BindingSlot::Primary,
                &with_meeting
            ),
            Ok(None)
        );
        assert!(matches!(
            check(
                "alt+shift",
                "paste_last",
                BindingSlot::Primary,
                &with_meeting
            ),
            Ok(Some(_))
        ));
    }

    #[test]
    fn saved_meeting_combinations_that_break_the_rules_are_not_registered() {
        let mut saved = bindings();
        if let Some(meeting) = saved.get_mut(MEETING_ACTION) {
            meeting.current_binding = "ctrl+super".to_string();
            meeting.alternate_binding = "ctrl+alt+m".to_string();
        }
        let safe = registrable(&saved[MEETING_ACTION], &saved);
        assert_eq!(safe.current_binding, "", "modifiers only");
        assert_eq!(safe.alternate_binding, "ctrl+alt+m");
        // Other actions are registered as saved.
        let transcribe = registrable(&saved["transcribe"], &saved);
        assert_eq!(
            transcribe.current_binding,
            saved["transcribe"].current_binding
        );
        assert_eq!(
            transcribe.alternate_binding,
            saved["transcribe"].alternate_binding
        );

        let problems = standing_problems(&saved);
        let meeting: Vec<_> = problems
            .iter()
            .filter(|(id, ..)| id == MEETING_ACTION)
            .map(|(_, slot, raw, found)| (*slot, raw.as_str(), found.clone()))
            .collect();
        assert_eq!(
            meeting,
            [(
                BindingSlot::Primary,
                "ctrl+super",
                Err(ShortcutRefusal::MeetingNeedsKey)
            )]
        );
        // Dictation's Ctrl+Win is the same keys as the meeting's Ctrl+Super: not an overlap, a
        // duplicate the save refused; the standing list reports only rules and overlaps.
        assert!(problems
            .iter()
            .all(|(_, _, _, found)| !matches!(found, Err(ShortcutRefusal::InUse { .. }))));
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
