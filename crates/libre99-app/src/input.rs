// Modified MIT License
//
// Copyright (c) 2026 Joel Odom
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, and sublicense copies of the
// Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// "Commons Clause" License Condition v1.0
//
// The Software is provided to you by the Licensor under the License, subject to
// the following condition.
//
// Without limiting other conditions in the License, the grant of rights under the
// License will not include, and the License does not grant to you, the right to
// Sell the Software.
//
// For purposes of the foregoing, "Sell" means practicing any or all of the rights
// granted to you under the License to provide to third parties, for a fee or other
// consideration (including without limitation fees for hosting or consulting/
// support services related to the Software), a product or service whose value
// derives, entirely or substantially, from the functionality of the Software. Any
// license notice or attribution required by the License must also include this
// Commons Clause License Condition notice.
//
// Software: Libre99
//
// License: Modified MIT
//
// Licensor: Joel Odom
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Translate host keyboard events into TI-99/4A keyboard-matrix keys.
//!
//! Two mappings are offered; the frontend toggles between them with `F7`
//! ([`KeyLayout`]):
//!
//! * **Character** (the default) maps by the *character your keystroke
//!   produces*. Whatever you type — on any host layout, with or without Shift —
//!   is translated into the TI keystroke that types the same character, with the
//!   TI's `SHIFT`/`FCTN` modifier synthesized as needed: `@` becomes
//!   `SHIFT`+`2`, `"` becomes `FCTN`+`P`, and so on (see [`char_to_ti_press`]).
//!   Typing feels like a modern keyboard. Hold **Left-Alt** (`FCTN`) or
//!   **Left-Ctrl** (`CTRL`) to reach the TI's function/control layer — the edit
//!   keys (`REDO`, `BACK`, …), the cursor combos, and control codes — and the
//!   **Backspace/Delete** key sends the TI's backspace, `FCTN`+`S` (cursor-left).
//! * **Positional** ([`to_ti_key`]) maps by *physical* key location: the key in
//!   the QWERTY `Q` spot is TI `Q` no matter what your OS layout is, and you press
//!   the TI modifiers yourself. Best for games and anything that reads the
//!   keyboard positionally.
//!
//! Either way the TI's modifier/function keys map to convenient host keys — Left
//! Alt = `FCTN`, Left Ctrl = `CTRL`, Left Shift = `SHIFT` — and the arrow keys
//! plus Right Alt drive joystick 1.

use libre99_core::keyboard::TiKey;
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, PhysicalKey};

/// Which mapping the frontend uses to turn host keys into TI keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyLayout {
    /// Map by the character the keystroke produces (default; natural typing,
    /// layout-independent). See [`char_to_ti_press`].
    #[default]
    Character,
    /// Map by physical key position (host layout ignored). Best for games.
    Positional,
}

impl KeyLayout {
    /// Parse a config string; anything but `positional`/`qwerty` is the default
    /// [`Character`](KeyLayout::Character).
    pub fn from_config(s: &str) -> KeyLayout {
        match s.trim().to_ascii_lowercase().as_str() {
            "positional" | "qwerty" => KeyLayout::Positional,
            _ => KeyLayout::Character,
        }
    }

    /// The config string for this layout (round-trips with [`from_config`]).
    pub fn as_config(self) -> &'static str {
        match self {
            KeyLayout::Character => "character",
            KeyLayout::Positional => "positional",
        }
    }
}

/// Host modifier state we care about while resolving keys, tracked from
/// `ModifiersChanged`: `alt`/`ctrl` drive the character layout's `FCTN`/`CTRL`
/// layer, and `cmd` (Super) is the macOS emulator-shortcut modifier. `shift`
/// isn't used for TI translation (it's folded into the produced character); the
/// frontend reads it for overlay navigation (e.g. Shift+Tab). The platform
/// "command" modifier for emulator hotkeys is [`HostMods::command`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostMods {
    pub alt: bool,
    pub ctrl: bool,
    pub cmd: bool,
    pub shift: bool,
}

impl HostMods {
    /// The platform "command" modifier for emulator shortcuts (screenshot, CPU
    /// inspector, …): **Super/Cmd** on macOS, **Ctrl** everywhere else — `Win`+key
    /// combos are stolen by the OS on Windows, so Ctrl is the portable choice.
    /// `app.rs`'s hotkey match tests this; the collision it creates on non-macOS
    /// (Ctrl is also the TI `CTRL` layer) is handled in [`resolve`].
    pub fn command(self) -> bool {
        if cfg!(target_os = "macos") {
            self.cmd
        } else {
            self.ctrl
        }
    }
}

/// The letter keys the emulator claims as command-modifier shortcuts, mirrored
/// from `app.rs`'s hotkey match: **S** = screenshot, **D** = CPU inspector. On
/// non-macOS the command modifier is Ctrl — which also drives the TI `CTRL`
/// layer — so these are the only Ctrl chords withheld from the TI; every other
/// Ctrl chord still reaches it.
fn is_command_key(code: KeyCode) -> bool {
    matches!(code, KeyCode::KeyS | KeyCode::KeyD)
}

/// The TI matrix key(s) one host key resolves to: an optional modifier
/// (`SHIFT`/`FCTN`/`CTRL`) held together with an optional base key. Character
/// mode can set both (e.g. `"` → `FCTN`+`P`); positional mode and bare modifier
/// keys set just one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TiPress {
    pub modifier: Option<TiKey>,
    pub key: Option<TiKey>,
}

impl TiPress {
    /// Nothing — the host key has no TI equivalent.
    pub const NONE: TiPress = TiPress { modifier: None, key: None };

    /// A single base key (no modifier).
    fn single(key: TiKey) -> TiPress {
        TiPress { modifier: None, key: Some(key) }
    }

    /// A single base key from the positional table (may be `None`).
    fn from_opt(key: Option<TiKey>) -> TiPress {
        TiPress { modifier: None, key }
    }

    /// A modifier held together with a base key (e.g. `SHIFT`+`2` for `@`).
    fn combo(modifier: TiKey, key: TiKey) -> TiPress {
        TiPress { modifier: Some(modifier), key: Some(key) }
    }

    /// The (at most two) matrix keys this press closes, modifier first.
    pub fn keys(self) -> impl Iterator<Item = TiKey> {
        self.modifier.into_iter().chain(self.key)
    }
}

/// Resolve a host key event to the TI key(s) it presses under `layout`.
pub fn resolve(event: &KeyEvent, layout: KeyLayout, mods: HostMods) -> TiPress {
    let PhysicalKey::Code(code) = event.physical_key else {
        return TiPress::NONE;
    };
    // Command-modifier chords are emulator shortcuts, never TI input:
    //   * Super (Cmd) is never a TI key on any platform, so swallow every Super
    //     chord (the macOS command modifier, and OS-reserved elsewhere).
    //   * On non-macOS the command modifier is Ctrl, which ALSO drives the TI
    //     CTRL layer — so swallow only the specific keys the emulator claims
    //     (`is_command_key`); every other Ctrl chord still reaches the TI. `cfg!`
    //     (not `#[cfg]`) so both branches type-check on every platform.
    if mods.cmd {
        return TiPress::NONE;
    }
    if !cfg!(target_os = "macos") && mods.ctrl && is_command_key(code) {
        return TiPress::NONE;
    }
    match layout {
        KeyLayout::Positional => TiPress::from_opt(to_ti_key(code)),
        KeyLayout::Character => resolve_character(code, event, mods),
    }
}

/// Character-layout resolution: translate the produced character into the TI
/// keystroke that types it, with the `FCTN`/`CTRL` layer and a couple of
/// convenience keys handled positionally. See the module docs.
fn resolve_character(code: KeyCode, event: &KeyEvent, mods: HostMods) -> TiPress {
    use KeyCode::*;
    use TiKey as K;

    // Bare host modifiers: SHIFT is folded into the produced character, so the
    // SHIFT key itself does nothing here; ALT and CTRL stay live as the TI
    // FCTN / CTRL layer (and Right-Alt is joystick fire, as in positional mode).
    match code {
        ShiftLeft | ShiftRight => return TiPress::NONE,
        AltLeft => return TiPress::single(K::Fctn),
        ControlLeft => return TiPress::single(K::Ctrl),
        AltRight => return TiPress::single(K::Joy1Fire),
        _ => {}
    }

    // Holding ALT or CTRL selects the TI function / control layer — the edit keys
    // (REDO, BACK, …), the cursor combos, and control codes — mapped positionally
    // so the whole TI keyboard stays reachable while typing by character.
    if mods.alt || mods.ctrl {
        return TiPress::from_opt(to_ti_key(code));
    }

    // The common case: map the character the host layout actually produced to the
    // TI keystroke (SHIFT/FCTN synthesized) that types it.
    if let Key::Character(s) = &event.logical_key {
        if let Some(press) = s.chars().next().and_then(char_to_ti_press) {
            return press;
        }
    }

    // Non-character keys. The Backspace/Delete key has no TI key of its own; the
    // TI's backspace is FCTN+S (cursor-left). Everything else (ENTER, SPACE, the
    // arrows/joystick) maps positionally.
    match code {
        Backspace => TiPress::combo(K::Fctn, K::S),
        _ => TiPress::from_opt(to_ti_key(code)),
    }
}

/// Map a produced character to the TI keystroke that types it: a base key plus a
/// synthesized `SHIFT`/`FCTN` modifier where the TI needs one. Covers the full
/// printable set the TI keyboard can produce. Mirrors Classic99's `keyboard.cpp`
/// table (and the on-screen reference card). Returns `None` for characters the TI
/// keyboard can't type.
pub fn char_to_ti_press(c: char) -> Option<TiPress> {
    use TiKey as K;

    // Letters: the TI types lowercase unshifted and uppercase with SHIFT.
    if c.is_ascii_alphabetic() {
        let base = char_to_ti_key(c)?;
        let modifier = c.is_ascii_uppercase().then_some(K::Shift);
        return Some(TiPress { modifier, key: Some(base) });
    }

    let base = |k: K| TiPress { modifier: None, key: Some(k) };
    let shift = |k: K| TiPress { modifier: Some(K::Shift), key: Some(k) };
    let fctn = |k: K| TiPress { modifier: Some(K::Fctn), key: Some(k) };
    Some(match c {
        // Digits and the five base punctuation keys, unshifted.
        '0' => base(K::Num0), '1' => base(K::Num1), '2' => base(K::Num2),
        '3' => base(K::Num3), '4' => base(K::Num4), '5' => base(K::Num5),
        '6' => base(K::Num6), '7' => base(K::Num7), '8' => base(K::Num8),
        '9' => base(K::Num9),
        '=' => base(K::Equals), '.' => base(K::Period), ',' => base(K::Comma),
        ';' => base(K::Semicolon), '/' => base(K::Slash), ' ' => base(K::Space),
        // SHIFT symbols: the number row and the shifted punctuation keys.
        '!' => shift(K::Num1), '@' => shift(K::Num2), '#' => shift(K::Num3),
        '$' => shift(K::Num4), '%' => shift(K::Num5), '^' => shift(K::Num6),
        '&' => shift(K::Num7), '*' => shift(K::Num8), '(' => shift(K::Num9),
        ')' => shift(K::Num0), '+' => shift(K::Equals), ':' => shift(K::Semicolon),
        '<' => shift(K::Comma), '>' => shift(K::Period), '-' => shift(K::Slash),
        // FCTN symbols.
        '?' => fctn(K::I), '_' => fctn(K::U), '\'' => fctn(K::O), '"' => fctn(K::P),
        '~' => fctn(K::W), '[' => fctn(K::R), ']' => fctn(K::T), '{' => fctn(K::F),
        '}' => fctn(K::G), '\\' => fctn(K::Z), '|' => fctn(K::A), '`' => fctn(K::C),
        _ => return None,
    })
}

/// Map a produced character (case-folded) to a TI key: the letters, digits, and
/// the five punctuation keys the TI keyboard has. The case-folded letter base for
/// [`char_to_ti_press`]; modifiers/space/enter/arrows are positional and handled
/// by [`to_ti_key`], not here.
pub fn char_to_ti_key(c: char) -> Option<TiKey> {
    use TiKey as K;
    Some(match c.to_ascii_uppercase() {
        'A' => K::A, 'B' => K::B, 'C' => K::C, 'D' => K::D, 'E' => K::E,
        'F' => K::F, 'G' => K::G, 'H' => K::H, 'I' => K::I, 'J' => K::J,
        'K' => K::K, 'L' => K::L, 'M' => K::M, 'N' => K::N, 'O' => K::O,
        'P' => K::P, 'Q' => K::Q, 'R' => K::R, 'S' => K::S, 'T' => K::T,
        'U' => K::U, 'V' => K::V, 'W' => K::W, 'X' => K::X, 'Y' => K::Y,
        'Z' => K::Z,
        '0' => K::Num0, '1' => K::Num1, '2' => K::Num2, '3' => K::Num3,
        '4' => K::Num4, '5' => K::Num5, '6' => K::Num6, '7' => K::Num7,
        '8' => K::Num8, '9' => K::Num9,
        '=' => K::Equals, '.' => K::Period, ',' => K::Comma,
        ';' => K::Semicolon, '/' => K::Slash,
        _ => return None,
    })
}

/// Map a host physical key to a TI key, or `None` if it isn't part of the TI
/// keyboard / joystick.
pub fn to_ti_key(code: KeyCode) -> Option<TiKey> {
    use KeyCode::*;
    use TiKey as K;
    Some(match code {
        KeyA => K::A, KeyB => K::B, KeyC => K::C, KeyD => K::D, KeyE => K::E,
        KeyF => K::F, KeyG => K::G, KeyH => K::H, KeyI => K::I, KeyJ => K::J,
        KeyK => K::K, KeyL => K::L, KeyM => K::M, KeyN => K::N, KeyO => K::O,
        KeyP => K::P, KeyQ => K::Q, KeyR => K::R, KeyS => K::S, KeyT => K::T,
        KeyU => K::U, KeyV => K::V, KeyW => K::W, KeyX => K::X, KeyY => K::Y,
        KeyZ => K::Z,

        Digit0 => K::Num0, Digit1 => K::Num1, Digit2 => K::Num2, Digit3 => K::Num3,
        Digit4 => K::Num4, Digit5 => K::Num5, Digit6 => K::Num6, Digit7 => K::Num7,
        Digit8 => K::Num8, Digit9 => K::Num9,

        Equal => K::Equals,
        Period => K::Period,
        Comma => K::Comma,
        Semicolon => K::Semicolon,
        Slash => K::Slash,
        Space => K::Space,
        Enter | NumpadEnter => K::Enter,

        AltLeft => K::Fctn,
        ShiftLeft | ShiftRight => K::Shift,
        ControlLeft => K::Ctrl,

        // Joystick 1.
        ArrowUp => K::Joy1Up,
        ArrowDown => K::Joy1Down,
        ArrowLeft => K::Joy1Left,
        ArrowRight => K::Joy1Right,
        AltRight => K::Joy1Fire,

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keys_modifiers_and_joystick() {
        assert_eq!(to_ti_key(KeyCode::KeyA), Some(TiKey::A));
        assert_eq!(to_ti_key(KeyCode::KeyZ), Some(TiKey::Z));
        assert_eq!(to_ti_key(KeyCode::Digit2), Some(TiKey::Num2));
        assert_eq!(to_ti_key(KeyCode::Enter), Some(TiKey::Enter));
        assert_eq!(to_ti_key(KeyCode::Space), Some(TiKey::Space));
        assert_eq!(to_ti_key(KeyCode::AltLeft), Some(TiKey::Fctn));
        assert_eq!(to_ti_key(KeyCode::ShiftLeft), Some(TiKey::Shift));
        assert_eq!(to_ti_key(KeyCode::ControlLeft), Some(TiKey::Ctrl));
        assert_eq!(to_ti_key(KeyCode::ArrowUp), Some(TiKey::Joy1Up));
        assert_eq!(to_ti_key(KeyCode::AltRight), Some(TiKey::Joy1Fire));
        // Keys with no TI equivalent are ignored.
        assert_eq!(to_ti_key(KeyCode::F7), None);
        assert_eq!(to_ti_key(KeyCode::Tab), None);
    }

    #[test]
    fn character_map_folds_case_and_covers_the_ti_keys() {
        assert_eq!(char_to_ti_key('a'), Some(TiKey::A));
        assert_eq!(char_to_ti_key('A'), Some(TiKey::A));
        assert_eq!(char_to_ti_key('5'), Some(TiKey::Num5));
        assert_eq!(char_to_ti_key('='), Some(TiKey::Equals));
        assert_eq!(char_to_ti_key(';'), Some(TiKey::Semicolon));
        // No TI key for these characters (they are SHIFT/FCTN combinations on the
        // TI, formed from a base key plus a modifier).
        assert_eq!(char_to_ti_key('@'), None);
        assert_eq!(char_to_ti_key('['), None);
    }

    #[test]
    fn key_layout_config_round_trips() {
        assert_eq!(KeyLayout::from_config("character"), KeyLayout::Character);
        assert_eq!(KeyLayout::from_config("Positional"), KeyLayout::Positional);
        // Character is the default, so unknown values resolve to it.
        assert_eq!(KeyLayout::from_config("nonsense"), KeyLayout::Character);
        assert_eq!(KeyLayout::default(), KeyLayout::Character);
        assert_eq!(KeyLayout::default().as_config(), "character");
        assert_eq!(
            KeyLayout::from_config(KeyLayout::Positional.as_config()),
            KeyLayout::Positional
        );
    }

    #[test]
    fn character_mode_types_symbols_via_ti_combos() {
        use TiKey as K;
        let combo = |m, k| Some(TiPress { modifier: Some(m), key: Some(k) });
        let base = |k| Some(TiPress { modifier: None, key: Some(k) });
        // The user's example: a double-quote is FCTN+P on the TI.
        assert_eq!(char_to_ti_press('"'), combo(K::Fctn, K::P));
        assert_eq!(char_to_ti_press('\''), combo(K::Fctn, K::O));
        assert_eq!(char_to_ti_press('?'), combo(K::Fctn, K::I));
        assert_eq!(char_to_ti_press('_'), combo(K::Fctn, K::U));
        // SHIFT symbols, including the unusual `-` = SHIFT+/ and `:` = SHIFT+;.
        assert_eq!(char_to_ti_press('@'), combo(K::Shift, K::Num2));
        assert_eq!(char_to_ti_press('+'), combo(K::Shift, K::Equals));
        assert_eq!(char_to_ti_press('-'), combo(K::Shift, K::Slash));
        assert_eq!(char_to_ti_press(':'), combo(K::Shift, K::Semicolon));
        // Letters: lowercase is unshifted, uppercase adds SHIFT.
        assert_eq!(char_to_ti_press('a'), base(K::A));
        assert_eq!(char_to_ti_press('A'), combo(K::Shift, K::A));
        // Plain base keys.
        assert_eq!(char_to_ti_press('5'), base(K::Num5));
        assert_eq!(char_to_ti_press('='), base(K::Equals));
        // The bracket/backslash/backtick family (formerly reserved for emulator
        // hotkeys) now types like any other key.
        assert_eq!(char_to_ti_press('['), combo(K::Fctn, K::R));
        assert_eq!(char_to_ti_press(']'), combo(K::Fctn, K::T));
        assert_eq!(char_to_ti_press('{'), combo(K::Fctn, K::F));
        assert_eq!(char_to_ti_press('}'), combo(K::Fctn, K::G));
        assert_eq!(char_to_ti_press('\\'), combo(K::Fctn, K::Z));
        assert_eq!(char_to_ti_press('|'), combo(K::Fctn, K::A));
        assert_eq!(char_to_ti_press('`'), combo(K::Fctn, K::C));
        assert_eq!(char_to_ti_press('~'), combo(K::Fctn, K::W));
        // A character the TI keyboard can't produce.
        assert_eq!(char_to_ti_press('\u{20AC}'), None); // euro sign
    }

    #[test]
    fn command_modifier_is_platform_correct_and_reserves_s_and_d() {
        // The reserved command-shortcut keys mirror app.rs's hotkey match.
        assert!(is_command_key(KeyCode::KeyS), "Cmd/Ctrl+S = screenshot");
        assert!(is_command_key(KeyCode::KeyD), "Cmd/Ctrl+D = CPU inspector");
        assert!(!is_command_key(KeyCode::KeyA));
        assert!(!is_command_key(KeyCode::KeyF));

        // The platform command modifier: Super/Cmd on macOS, Ctrl elsewhere.
        let ctrl = HostMods { ctrl: true, ..HostMods::default() };
        let cmd = HostMods { cmd: true, ..HostMods::default() };
        if cfg!(target_os = "macos") {
            assert!(cmd.command());
            assert!(!ctrl.command());
        } else {
            assert!(ctrl.command(), "Ctrl is the command modifier off macOS");
            assert!(!cmd.command(), "Super is not the command modifier off macOS");
        }
    }

    #[test]
    fn ti_press_yields_modifier_then_key() {
        let p = TiPress { modifier: Some(TiKey::Fctn), key: Some(TiKey::P) };
        assert_eq!(p.keys().collect::<Vec<_>>(), vec![TiKey::Fctn, TiKey::P]);
        let p = TiPress { modifier: None, key: Some(TiKey::A) };
        assert_eq!(p.keys().collect::<Vec<_>>(), vec![TiKey::A]);
        assert_eq!(TiPress::NONE.keys().count(), 0);
    }
}
