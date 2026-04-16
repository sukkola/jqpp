use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    Submit,
    AcceptSuggestion,
    NextPane,
    PrevPane,
    Quit,
    CopyClipboard,
    SaveOutput,
    ToggleQueryBar,
    ToggleMenu,
    HistoryUp,
    HistoryDown,
    SuggestionUp,
    SuggestionDown,
    ScrollUp,
    ScrollDown,
}

impl Action {
    pub fn all() -> &'static [Action] {
        &[
            Action::Submit,
            Action::AcceptSuggestion,
            Action::NextPane,
            Action::PrevPane,
            Action::Quit,
            Action::CopyClipboard,
            Action::SaveOutput,
            Action::ToggleQueryBar,
            Action::ToggleMenu,
            Action::HistoryUp,
            Action::HistoryDown,
            Action::SuggestionUp,
            Action::SuggestionDown,
            Action::ScrollUp,
            Action::ScrollDown,
        ]
    }

    pub fn toml_name(&self) -> &'static str {
        match self {
            Action::Submit => "submit",
            Action::AcceptSuggestion => "accept-suggestion",
            Action::NextPane => "next-pane",
            Action::PrevPane => "prev-pane",
            Action::Quit => "quit",
            Action::CopyClipboard => "copy-clipboard",
            Action::SaveOutput => "save-output",
            Action::ToggleQueryBar => "toggle-query-bar",
            Action::ToggleMenu => "toggle-menu",
            Action::HistoryUp => "history-up",
            Action::HistoryDown => "history-down",
            Action::SuggestionUp => "suggestion-up",
            Action::SuggestionDown => "suggestion-down",
            Action::ScrollUp => "scroll-up",
            Action::ScrollDown => "scroll-down",
        }
    }

    pub fn from_toml_name(s: &str) -> Option<Action> {
        match s {
            "submit" => Some(Action::Submit),
            "accept-suggestion" => Some(Action::AcceptSuggestion),
            "next-pane" => Some(Action::NextPane),
            "prev-pane" => Some(Action::PrevPane),
            "quit" => Some(Action::Quit),
            "copy-clipboard" => Some(Action::CopyClipboard),
            "save-output" => Some(Action::SaveOutput),
            "toggle-query-bar" => Some(Action::ToggleQueryBar),
            "toggle-menu" => Some(Action::ToggleMenu),
            "history-up" => Some(Action::HistoryUp),
            "history-down" => Some(Action::HistoryDown),
            "suggestion-up" => Some(Action::SuggestionUp),
            "suggestion-down" => Some(Action::SuggestionDown),
            "scroll-up" => Some(Action::ScrollUp),
            "scroll-down" => Some(Action::ScrollDown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        if self.code != event.code {
            return false;
        }
        if self.code == KeyCode::BackTab {
            // BackTab often comes with or without Shift depending on terminal.
            // We treat them as equivalent for matching purposes.
            let self_mods = self.modifiers & !KeyModifiers::SHIFT;
            let event_mods = event.modifiers & !KeyModifiers::SHIFT;
            return self_mods == event_mods;
        }
        self.modifiers == event.modifiers
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            write!(f, "Ctrl+")?;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            write!(f, "Alt+")?;
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) && !matches!(self.code, KeyCode::Char(_)) {
            write!(f, "Shift+")?;
        }

        match self.code {
            KeyCode::Char(c) => write!(f, "{}", c),
            KeyCode::Enter => write!(f, "Enter"),
            KeyCode::Tab => write!(f, "Tab"),
            KeyCode::BackTab => write!(f, "BackTab"),
            KeyCode::Esc => write!(f, "Esc"),
            KeyCode::Up => write!(f, "Up"),
            KeyCode::Down => write!(f, "Down"),
            KeyCode::Left => write!(f, "Left"),
            KeyCode::Right => write!(f, "Right"),
            KeyCode::Backspace => write!(f, "Backspace"),
            KeyCode::Delete => write!(f, "Delete"),
            KeyCode::F(n) => write!(f, "F{}", n),
            _ => write!(f, "{:?}", self.code),
        }
    }
}

pub struct Keymap(pub HashMap<Action, KeyBinding>);

impl Keymap {
    pub fn default_map() -> HashMap<Action, KeyBinding> {
        let mut m = HashMap::new();
        m.insert(
            Action::Submit,
            KeyBinding::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        m.insert(
            Action::AcceptSuggestion,
            KeyBinding::new(KeyCode::Tab, KeyModifiers::empty()),
        );
        m.insert(
            Action::NextPane,
            KeyBinding::new(KeyCode::Tab, KeyModifiers::empty()),
        );
        m.insert(
            Action::PrevPane,
            KeyBinding::new(KeyCode::BackTab, KeyModifiers::empty()),
        );
        m.insert(
            Action::Quit,
            KeyBinding::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        m.insert(
            Action::CopyClipboard,
            KeyBinding::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        );
        m.insert(
            Action::SaveOutput,
            KeyBinding::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );
        m.insert(
            Action::ToggleQueryBar,
            KeyBinding::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
        );
        m.insert(
            Action::ToggleMenu,
            KeyBinding::new(KeyCode::Char('m'), KeyModifiers::CONTROL),
        );
        m.insert(
            Action::HistoryUp,
            KeyBinding::new(KeyCode::Up, KeyModifiers::empty()),
        );
        m.insert(
            Action::HistoryDown,
            KeyBinding::new(KeyCode::Down, KeyModifiers::empty()),
        );
        m.insert(
            Action::SuggestionUp,
            KeyBinding::new(KeyCode::Up, KeyModifiers::empty()),
        );
        m.insert(
            Action::SuggestionDown,
            KeyBinding::new(KeyCode::Down, KeyModifiers::empty()),
        );
        m.insert(
            Action::ScrollUp,
            KeyBinding::new(KeyCode::Char('k'), KeyModifiers::empty()),
        );
        m.insert(
            Action::ScrollDown,
            KeyBinding::new(KeyCode::Char('j'), KeyModifiers::empty()),
        );
        m
    }

    pub fn new(map: HashMap<Action, KeyBinding>) -> Self {
        Self(map)
    }

    pub fn action_for(&self, event: &KeyEvent) -> Option<Action> {
        for (action, binding) in &self.0 {
            if binding.matches(event) {
                return Some(*action);
            }
        }
        None
    }

    pub fn is_action(&self, action: Action, event: &KeyEvent) -> bool {
        self.0
            .get(&action)
            .map(|b| b.matches(event))
            .unwrap_or(false)
    }

    pub fn binding_for(&self, action: Action) -> &KeyBinding {
        self.0.get(&action).expect("Action missing from keymap")
    }

    pub fn hint_string(&self) -> String {
        let items = [
            (Action::Submit, "submit"),
            (Action::NextPane, "nav"),
            (Action::Quit, "quit"),
            (Action::CopyClipboard, "copy"),
            (Action::ToggleQueryBar, "hide input"),
            (Action::SaveOutput, "save"),
        ];

        items
            .iter()
            .map(|(action, label)| {
                format!(
                    "{} {}",
                    self.binding_for(*action).to_string().to_lowercase(),
                    label
                )
            })
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self(Self::default_map())
    }
}

impl std::ops::Deref for Keymap {
    type Target = HashMap<Action, KeyBinding>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn parse_key_binding(s: &str) -> Result<KeyBinding, String> {
    let mut parts: Vec<&str> = s.split('+').collect();
    if parts.is_empty() || (parts.len() == 1 && parts[0].is_empty()) {
        return Err("Empty key binding".to_string());
    }

    let key_str = parts.pop().unwrap();
    let mut modifiers = KeyModifiers::empty();

    for mod_str in parts {
        match mod_str.to_lowercase().as_str() {
            "ctrl" => modifiers.insert(KeyModifiers::CONTROL),
            "alt" => modifiers.insert(KeyModifiers::ALT),
            "shift" => modifiers.insert(KeyModifiers::SHIFT),
            _ => return Err(format!("Unknown modifier: {}", mod_str)),
        }
    }

    let code = match key_str.to_lowercase().as_str() {
        "enter" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "esc" => KeyCode::Esc,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        s if s.starts_with('f') && s[1..].parse::<u8>().is_ok() => {
            KeyCode::F(s[1..].parse::<u8>().unwrap())
        }
        s if s.chars().count() == 1 => {
            let c = s.chars().next().unwrap();
            KeyCode::Char(c)
        }
        _ => return Err(format!("Unknown key: {}", key_str)),
    };

    Ok(KeyBinding { code, modifiers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_binding() {
        assert_eq!(
            parse_key_binding("Ctrl+y").unwrap(),
            KeyBinding::new(KeyCode::Char('y'), KeyModifiers::CONTROL)
        );
        assert_eq!(
            parse_key_binding("Alt+Enter").unwrap(),
            KeyBinding::new(KeyCode::Enter, KeyModifiers::ALT)
        );
        assert_eq!(
            parse_key_binding("F5").unwrap(),
            KeyBinding::new(KeyCode::F(5), KeyModifiers::empty())
        );
        assert_eq!(
            parse_key_binding("Shift+Up").unwrap(),
            KeyBinding::new(KeyCode::Up, KeyModifiers::SHIFT)
        );
        assert_eq!(
            parse_key_binding("Ctrl+Shift+s").unwrap(),
            KeyBinding::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )
        );
        assert!(parse_key_binding("Ctrl+").is_err());
        assert!(parse_key_binding("Hyperspace+Z").is_err());
    }

    #[test]
    fn test_keymap_action_for() {
        let keymap = Keymap::default();
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(keymap.action_for(&event), Some(Action::Quit));

        let event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
        assert_eq!(keymap.action_for(&event), None);
    }

    #[test]
    fn test_keymap_hint_string() {
        let keymap = Keymap::default();
        let hint = keymap.hint_string();
        assert!(hint.contains("enter submit"));
        assert!(hint.contains("ctrl+c quit"));
    }
}
