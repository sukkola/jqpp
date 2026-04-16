use crate::keymap::{Action, KeyBinding, Keymap, parse_key_binding};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub keys: HashMap<String, String>,
}

pub fn resolve_config_path(override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = override_path {
        return Some(path.to_path_buf());
    }

    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg_config).join("jqpp/config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join(".config/jqpp/config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn is_allowed_conflict(a1: Action, a2: Action) -> bool {
    let (min, max) = if a1 < a2 { (a1, a2) } else { (a2, a1) };
    matches!(
        (min, max),
        (Action::AcceptSuggestion, Action::NextPane)
            | (Action::HistoryUp, Action::SuggestionUp)
            | (Action::HistoryDown, Action::SuggestionDown)
            | (Action::HistoryUp, Action::ScrollUp)
            | (Action::SuggestionUp, Action::ScrollUp)
            | (Action::HistoryDown, Action::ScrollDown)
            | (Action::SuggestionDown, Action::ScrollDown)
    )
}

pub fn load_keymap(override_path: Option<&Path>) -> (Keymap, Option<String>) {
    let config_path = resolve_config_path(override_path);

    let path = match config_path {
        Some(p) => p,
        None => return (Keymap::default(), None),
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            if path.exists() {
                return (Keymap::default(), Some(format!("Config read error: {}", e)));
            } else {
                return (Keymap::default(), None);
            }
        }
    };

    let config: Config = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            return (
                Keymap::default(),
                Some(format!("Config parse error: {}", e)),
            );
        }
    };

    let mut map = Keymap::default_map();
    let mut warnings = Vec::new();

    for (action_name, key_str) in config.keys {
        if let Some(action) = Action::from_toml_name(&action_name) {
            match parse_key_binding(&key_str) {
                Ok(binding) => {
                    map.insert(action, binding);
                }
                Err(e) => {
                    warnings.push(format!(
                        "Invalid key '{}' for '{}': {}",
                        key_str, action_name, e
                    ));
                }
            }
        } else {
            warnings.push(format!("Unknown action: {}", action_name));
        }
    }

    // Conflict detection
    let mut bindings_seen: HashMap<KeyBinding, Action> = HashMap::new();
    for (action, binding) in &map {
        match bindings_seen.get(binding) {
            Some(other_action) if !is_allowed_conflict(*action, *other_action) => {
                return (
                    Keymap::default(),
                    Some(format!(
                        "Config conflict: both '{}' and '{}' bound to '{}'",
                        other_action.toml_name(),
                        action.toml_name(),
                        binding
                    )),
                );
            }
            _ => {
                bindings_seen.insert(*binding, *action);
            }
        }
    }

    let warning_str = if warnings.is_empty() {
        None
    } else {
        Some(format!("Config warning: {}", warnings.join("; ")))
    };

    (Keymap::new(map), warning_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Action;
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_load_keymap_absent() {
        let (keymap, err) = load_keymap(Some(Path::new("nonexistent.toml")));
        assert!(err.is_none());
        assert_eq!(
            keymap.binding_for(Action::Quit),
            &KeyBinding::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        );
    }

    #[test]
    fn test_load_keymap_conflict() {
        let dir = std::env::temp_dir();
        let path = dir.join("conflict.toml");
        fs::write(&path, "[keys]\nquit = \"Ctrl+s\"\nsave-output = \"Ctrl+s\"").unwrap();

        let (keymap, err) = load_keymap(Some(&path));
        assert!(err.is_some());
        assert!(err.unwrap().contains("Config conflict"));
        // Should return defaults on conflict
        assert_eq!(
            keymap.binding_for(Action::Quit),
            &KeyBinding::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_load_keymap_partial() {
        let dir = std::env::temp_dir();
        let path = dir.join("partial.toml");
        fs::write(&path, "[keys]\nquit = \"F10\"").unwrap();

        let (keymap, err) = load_keymap(Some(&path));
        assert!(err.is_none(), "Expected no error, got: {:?}", err);
        assert_eq!(
            keymap.binding_for(Action::Quit),
            &KeyBinding::new(KeyCode::F(10), KeyModifiers::empty())
        );
        // Default preserved
        assert_eq!(
            keymap.binding_for(Action::Submit),
            &KeyBinding::new(KeyCode::Enter, KeyModifiers::empty())
        );
        let _ = fs::remove_file(path);
    }
}
